use anyhow::{Error, Result};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;

use crate::constants::*;
use crate::structs::*;
use crate::{
    data::{PredictionInput, PredictionOutput},
    space::wake_up_space,
    stream::PredictionStream,
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

        let http_client = Client::build_http_client(&options.hf_token)?;

        let (api_root, space_id) =
            Client::resolve_app_reference(&http_client, app_reference).await?;

        if let Some((username, password)) = &options.auth {
            Client::authenticate(&http_client, &api_root, username, password).await?;
        }

        if let Some(space_id) = &space_id {
            wake_up_space(&http_client, space_id).await?;
        }

        let config = Client::fetch_config(&http_client, &api_root).await?;
        let api_info = Client::fetch_api_info(&http_client, &api_root).await?;

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

    pub fn view_config(&self) -> AppConfig {
        self.config.clone()
    }

    pub fn view_api(&self) -> ApiInfo {
        self.api_info.clone()
    }

    pub async fn submit(self, route: &str, data: Vec<PredictionInput>) -> Result<PredictionStream> {
        let data = Client::prepare_data(&self.http_client, &self.api_root, data).await?;
        let fn_index = Client::resolve_fn_index(&self.config, route)?;
        PredictionStream::new(&self.http_client, &self.api_root, fn_index, data).await
    }

    pub async fn predict(
        self,
        route: &str,
        data: Vec<PredictionInput>,
    ) -> Result<Vec<PredictionOutput>> {
        let mut stream = self.submit(route, data).await?;
        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => match message {
                    QueueDataMessage::Open
                    | QueueDataMessage::InQueue { .. }
                    | QueueDataMessage::Processing { .. } => {}
                    QueueDataMessage::Completed { output, .. } => {
                        return output.try_into();
                    }
                    QueueDataMessage::Unknown(m) => {
                        if m.get("msg").map(|m| m == "heartbeat").unwrap_or(false) {
                            continue;
                        }
                        eprintln!("[warning] Skipping unknown message: {:?}", m);
                    }
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }

        Err(Error::msg("Stream ended unexpectedly"))
    }

    fn build_http_client(hf_token: &Option<String>) -> Result<reqwest::Client> {
        let mut http_client_builder = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("Rust Gradio Client");
        if let Some(hf_token) = hf_token {
            http_client_builder =
                http_client_builder.default_headers(reqwest::header::HeaderMap::from_iter(vec![(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", hf_token).parse()?,
                )]));
        }

        http_client_builder.build().map_err(Error::new)
    }

    async fn resolve_app_reference(
        http_client: &reqwest::Client,
        app_reference: &str,
    ) -> Result<(String, Option<String>)> {
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

        Ok((api_root, space_id))
    }

    async fn authenticate(
        http_client: &reqwest::Client,
        api_root: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let res = http_client
            .post(&format!("{}/{}", api_root, LOGIN_URL))
            .form(&[("username", username), ("password", password)])
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::msg("Login failed"));
        }
        Ok(())
    }

    async fn fetch_config(http_client: &reqwest::Client, api_root: &str) -> Result<AppConfig> {
        let res = http_client
            .get(&format!("{}/{}", api_root, CONFIG_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::msg("Could not resolve app config"));
        }
        res.json::<AppConfig>().await.map_err(Error::new)
    }

    async fn fetch_api_info(http_client: &reqwest::Client, api_root: &str) -> Result<ApiInfo> {
        let res = http_client
            .get(&format!("{}/{}", api_root, API_INFO_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::msg("Could not get API info"));
        }
        res.json::<ApiInfo>().await.map_err(Error::new)
    }

    async fn prepare_data(
        http_client: &reqwest::Client,
        api_root: &str,
        data: Vec<PredictionInput>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut inputs = vec![];
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
        Ok(inputs)
    }

    fn resolve_fn_index(config: &AppConfig, route: &str) -> Result<i64> {
        let route = route.trim_start_matches('/');
        let found = config
            .dependencies
            .iter()
            .enumerate()
            .find(|(_i, d)| d.api_name == route)
            .ok_or_else(|| Error::msg("Invalid route"))?;

        if found.1.id == -1 {
            Ok(found.0 as i64)
        } else {
            Ok(found.1.id)
        }
    }
}
