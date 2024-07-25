use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::io::AsyncWriteExt;

use crate::{constants::UPLOAD_URL, structs::QueueDataMessageOutput};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PredictionInput {
    Value(serde_json::Value),
    File(PathBuf),
    Array(Vec<PredictionInput>),
}

impl PredictionInput {
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn from_value(value: impl serde::Serialize) -> Self {
        Self::Value(serde_json::to_value(value).unwrap())
    }
}

pub async fn upload_file(
    http_client: &reqwest::Client,
    api_root: &str,
    path: PathBuf,
) -> Result<serde_json::Value> {
    let part = reqwest::multipart::Part::bytes(tokio::fs::read(&path).await?)
        .file_name(
            path.file_name()
                .ok_or_else(|| Error::msg("Invalid file path"))?
                .to_string_lossy()
                .to_string(),
        )
        .mime_str(
            mime_guess::from_path(&path)
                .first_or_octet_stream()
                .as_ref(),
        )?;
    let form = reqwest::multipart::Form::new().part("files", part);
    let res = http_client
        .post(&format!("{}/{}", api_root, UPLOAD_URL))
        .multipart(form)
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(Error::msg("Error uploading file"));
    }
    let res = res.json::<Vec<String>>().await?;
    if res.len() != 1 {
        return Err(Error::msg("Invalid file upload response"));
    }

    let json = serde_json::json!({
        "path": res[0],
        "orig_name": path.to_string_lossy(),
        "meta": {
            "_type": "gradio.FileData"
        }
    });

    Ok(json)
}

pub async fn preprocess_data(
    http_client: &reqwest::Client,
    api_root: &str,
    data: Vec<PredictionInput>,
) -> Result<Vec<serde_json::Value>> {
    preprocess_data_helper(http_client, api_root, data).await
}

fn preprocess_data_helper<'a>(
    http_client: &'a reqwest::Client,
    api_root: &'a str,
    data: Vec<PredictionInput>,
) -> Pin<Box<dyn Future<Output = Result<Vec<serde_json::Value>>> + 'a>> {
    Box::pin(async move {
        let mut inputs = vec![];
        for d in data {
            match d {
                PredictionInput::Value(value) => inputs.push(value),
                PredictionInput::File(path) => {
                    inputs.push(upload_file(http_client, api_root, path).await?);
                }
                PredictionInput::Array(values) => {
                    let array = preprocess_data(http_client, api_root, values).await?;
                    inputs.push(serde_json::json!(array));
                }
            }
        }
        Ok(inputs)
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PredictionOutput {
    File(GradioFileData),
    Value(serde_json::Value),
}

impl PredictionOutput {
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    pub fn as_file(self) -> Result<GradioFileData> {
        match self {
            Self::File(file) => Ok(file),
            _ => Err(anyhow::anyhow!("Expected file output")),
        }
    }

    pub fn as_value(self) -> Result<serde_json::Value> {
        match self {
            Self::Value(value) => Ok(value),
            _ => Err(anyhow::anyhow!("Expected value output")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GradioFileData {
    pub path: Option<String>,
    pub orig_name: Option<String>,
    pub meta: GradioFileDataMeta,
    pub url: Option<String>,
    pub size: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GradioFileDataMeta {
    pub _type: String,
}

impl TryFrom<QueueDataMessageOutput> for Vec<PredictionOutput> {
    type Error = anyhow::Error;

    fn try_from(value: QueueDataMessageOutput) -> Result<Self> {
        match value {
            QueueDataMessageOutput::Success { data, .. } => {
                let mut outputs = Vec::new();
                for value in data {
                    outputs.push(serde_json::from_value::<PredictionOutput>(value)?);
                }
                Ok(outputs)
            }
            QueueDataMessageOutput::Error { error } => {
                Err(anyhow::anyhow!(error.unwrap_or("Unknown error".to_string())))
            }
        }
    }
}

impl GradioFileData {
    pub async fn download(&self, http_client: Option<reqwest::Client>) -> Result<bytes::Bytes> {
        let http_client = if let Some(http_client) = http_client {
            http_client
        } else {
            reqwest::Client::new()
        };
        if let Some(url) = &self.url {
            let response = http_client.get(url).send().await?;
            let content = response.bytes().await?;
            Ok(content)
        } else {
            Err(Error::msg("No URL available for file"))
        }
    }

    pub async fn save_to_path(
        &self,
        path: impl AsRef<Path>,
        http_client: Option<reqwest::Client>,
    ) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::File::create(path).await?;
        let bytes = self.download(http_client).await?;
        file.write_all(&bytes).await?;
        Ok(())
    }

    pub fn suggest_extension(&self) -> &str {
        let ext = if let Some(orig_name) = &self.orig_name {
            orig_name
        } else if let Some(path) = &self.path {
            path
        } else if let Some(url) = &self.url {
            url
        } else {
            "file.bin"
        };
        let ext = ext.split('.').last();
        let ext = ext.unwrap_or("bin");
        ext
    }
}
