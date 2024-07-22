use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PredictionInput {
    Value(serde_json::Value),
    File(PathBuf),
}

impl PredictionInput {
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn from_value(value: impl serde::Serialize) -> Self {
        Self::Value(serde_json::to_value(value).unwrap())
    }
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
