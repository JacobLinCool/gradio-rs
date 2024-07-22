use anyhow::{Error, Result};
use futures_util::stream::StreamExt;
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use reqwest_eventsource::{Event, EventSource};

use crate::constants::*;
use crate::structs::*;
use crate::{
    data::{PredictionInput, PredictionOutput},
    space::wake_up_space,
};

#[derive(Default)]
pub struct ClientOptions {
    pub hf_token: Option<String>,
    pub auth: Option<(String, String)>,
}

impl ClientOptions {
    pub fn with_hf_token(hf_token: String) -> ClientOptions {
        Self {
            hf_token: Some(hf_token),
            auth: None,
        }
    }

    pub fn with_auth(username: String, password: String) -> Self {
        Self {
            hf_token: None,
            auth: Some((username, password)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    pub session_hash: String,
    pub jwt: Option<String>,
    pub http_client: reqwest::Client,
    pub api_root: String,
    pub space_id: Option<String>,
    config: AppConfig,
    api_info: ApiInfo,
}

impl Client {
    /// Create a new client
    ///
    /// # Arguments
    ///
    /// * `app_reference` - The reference to the app
    /// * `options` - The options for the client
    ///
    /// # Returns
    ///
    /// A new Gradio client
    ///
    /// # Errors
    ///
    /// If the client cannot be created
    ///
    /// # Example
    ///
    /// ```
    /// use gradio::{Client, ClientOptions};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let client = Client::new(
    ///         "gradio/hello_world",
    ///         ClientOptions::default()
    ///     ).await.unwrap();
    ///     println!("{:?}", client);
    /// }
    /// ```
    pub async fn new(app_reference: &str, options: ClientOptions) -> Result<Self> {
        let session_hash: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        // Build the HTTP client
        let mut http_client_builder = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("Rust Gradio Client");
        if let Some(hf_token) = &options.hf_token {
            http_client_builder =
                http_client_builder.default_headers(reqwest::header::HeaderMap::from_iter(vec![(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", hf_token).parse()?,
                )]));
        }
        let http_client = http_client_builder.build()?;

        // Resolve the API root
        let app_reference = app_reference.trim_end_matches('/').to_string();
        let mut api_root = app_reference.clone();
        let mut space_id = None;
        if Regex::new("^[a-zA-Z0-9_\\-\\.]+\\/[a-zA-Z0-9_\\-\\.]+$")?.is_match(&app_reference) {
            let url = format!(
                "https://huggingface.co/api/spaces/{}/{}",
                app_reference, HOST_URL
            );
            let res = http_client.get(&url).send().await?;
            let res = res.json::<HuggingFaceAPIHost>().await?;
            api_root.clone_from(&res.host);
            space_id = Some(app_reference);
        } else if Regex::new(".*hf\\.space\\/{0,1}$")?.is_match(&app_reference) {
            space_id = Some(app_reference.replace(".hf.space", ""));
        }

        // Authenticate with username and password
        if let Some((username, password)) = &options.auth {
            let res = http_client
                .post(&format!("{}/{}", api_root, LOGIN_URL))
                .form(&[("username", username), ("password", password)])
                .send()
                .await?;
            if !res.status().is_success() {
                return Err(Error::msg("Login failed"));
            }
        }

        if let Some(space_id) = &space_id {
            wake_up_space(&http_client, space_id).await?;
        }

        // Fetch the config
        let res = http_client
            .get(&format!("{}/{}", api_root, CONFIG_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::msg("Could not resolve app config"));
        }
        let config = res.json::<AppConfig>().await?;

        // Fetch the API info
        let res = http_client
            .get(&format!("{}/{}", api_root, API_INFO_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::msg("Could not get API info"));
        }
        let api_info = res.json::<ApiInfo>().await?;

        Ok(Self {
            session_hash,
            jwt: None,
            http_client,
            api_root,
            space_id,
            config,
            api_info,
        })
    }

    pub fn view_config(self) -> AppConfig {
        self.config
    }

    pub fn view_api(self) -> ApiInfo {
        self.api_info
    }

    pub async fn predict(
        self,
        route: &str,
        data: Vec<PredictionInput>,
    ) -> Result<Vec<PredictionOutput>> {
        let mut inputs = vec![];
        // upload files and replace with file handles
        for d in data {
            match d {
                PredictionInput::File(path) => {
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
                    let res = self
                        .http_client
                        .post(&format!("{}/{}", self.api_root, UPLOAD_URL))
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
                    inputs.push(serde_json::json!({
                        "path": res[0],
                        "orig_name": path.to_string_lossy(),
                        "meta": {
                            "_type": "gradio.FileData"
                        }
                    }));
                }
                PredictionInput::Value(value) => inputs.push(value),
            }
        }

        let route = route.trim_start_matches('/');
        let fn_index = self
            .config
            .dependencies
            .iter()
            .find(|d| d.api_name == route)
            .ok_or_else(|| Error::msg("Invalid route"))?
            .id;

        // join the queue
        let url = format!("{}/queue/join", self.api_root);
        let payload = serde_json::json!({
            "fn_index": fn_index,
            "data": inputs,
            "session_hash": self.session_hash
        });
        // println!("{:#?}", payload);
        let res = self.http_client.post(&url).json(&payload).send().await?;
        if !res.status().is_success() {
            return Err(Error::msg("Cannot join task queue"));
        }
        let res = res.json::<QueueJoinResponse>().await?;
        let evt = res.event_id;

        let url = format!(
            "{}/queue/data?session_hash={}",
            self.api_root, self.session_hash
        );
        let mut es = EventSource::new(self.http_client.get(&url))?;
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {}
                Ok(Event::Message(message)) => {
                    // println!("{:#?}", message);
                    let message: QueueDataMessage = serde_json::from_str(&message.data)?;
                    match message {
                        QueueDataMessage::InQueue { .. } => {}
                        QueueDataMessage::Processing { .. } => {}
                        QueueDataMessage::Completed {
                            output, event_id, ..
                        } => {
                            if event_id == evt {
                                if let QueueDataMessageOutput::Success { data, .. } = output {
                                    return data
                                        .into_iter()
                                        .map(|d| {
                                            serde_json::from_value::<PredictionOutput>(d)
                                                .map_err(Error::msg)
                                        })
                                        .collect::<Result<Vec<PredictionOutput>>>();
                                } else if let QueueDataMessageOutput::Error { error } = output {
                                    return Err(Error::msg(
                                        error.unwrap_or("Unknown error".to_string()),
                                    ));
                                }
                            }
                        }
                        QueueDataMessage::Unknown(_) => {}
                    }
                }
                Err(err) => {
                    return Err(Error::msg(format!("Error: {:#?}", err)));
                }
            }
        }

        Err(Error::msg("Stream ended unexpectedly"))
    }
}
