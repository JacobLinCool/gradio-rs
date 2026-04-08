use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;

use crate::constants::*;
use crate::preprocess_data;
use crate::structs::*;
use crate::{
    data::{PredictionInput, PredictionOutput},
    space::wake_up_space,
    stream::PredictionStream,
    Error, Result,
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
    /// use gradio::{Client, ClientOptions, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///    let client = Client::new(
    ///         "gradio/hello_world",
    ///         ClientOptions::default()
    ///     ).await?;
    ///     println!("{:?}", client);
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(app_reference: &str, options: ClientOptions) -> Result<Self> {
        let session_hash: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        let http_client = Client::build_http_client(&options.hf_token)?;

        let (mut api_root, space_id) =
            Client::resolve_app_reference(&http_client, app_reference).await?;

        if let Some((username, password)) = &options.auth {
            Client::authenticate(&http_client, &api_root, username, password).await?;
        }

        if let Some(space_id) = &space_id {
            wake_up_space(&http_client, space_id).await?;
        }

        let config = Client::fetch_config(&http_client, &api_root).await?;
        if let Some(ref api_prefix) = config.api_prefix {
            api_root = Client::join_url_path(&api_root, api_prefix);
        }

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

    pub async fn submit(
        &self,
        route: &str,
        data: Vec<PredictionInput>,
    ) -> Result<PredictionStream> {
        let data = preprocess_data(&self.http_client, &self.api_root, data).await?;
        let fn_index = Client::resolve_fn_index(&self.config, route)?;
        PredictionStream::new(
            &self.http_client,
            &self.api_root,
            &self.config.protocol,
            fn_index,
            data,
        )
        .await
    }

    pub async fn predict(
        &self,
        route: &str,
        data: Vec<PredictionInput>,
    ) -> Result<Vec<PredictionOutput>> {
        let mut stream = self.submit(route, data).await?;
        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => match message {
                    QueueDataMessage::Open
                    | QueueDataMessage::Estimation { .. }
                    | QueueDataMessage::ProcessStarts { .. }
                    | QueueDataMessage::ProcessGenerating { .. }
                    | QueueDataMessage::ProcessStreaming { .. }
                    | QueueDataMessage::Progress { .. }
                    | QueueDataMessage::Log { .. }
                    | QueueDataMessage::Heartbeat
                    | QueueDataMessage::CloseStream => {}
                    QueueDataMessage::ProcessCompleted { output, .. } => {
                        return output.try_into();
                    }
                    QueueDataMessage::UnexpectedError { message, .. } => {
                        return Err(Error::UnexpectedRemoteError {
                            message: message.unwrap_or_else(|| "Unexpected error".to_string()),
                        });
                    }
                    QueueDataMessage::Unknown(m) => {
                        eprintln!("[warning] Skipping unknown message: {:?}", m);
                    }
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }

        Err(Error::StreamEndedUnexpectedly)
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

        http_client_builder.build().map_err(Error::from)
    }

    async fn resolve_app_reference(
        http_client: &reqwest::Client,
        app_reference: &str,
    ) -> Result<(String, Option<String>)> {
        let app_reference = app_reference.trim_end_matches('/').to_string();
        if Regex::new("^[a-zA-Z0-9_\\-\\.]+\\/[a-zA-Z0-9_\\-\\.]+$")?.is_match(&app_reference) {
            let url = format!(
                "https://huggingface.co/api/spaces/{}/{}",
                app_reference, HOST_URL
            );
            let res = http_client.get(&url).send().await?;
            let res = res.json::<HuggingFaceAPIHost>().await?;
            return Ok((res.host, Some(app_reference)));
        }

        if reqwest::Url::parse(&app_reference).is_ok() {
            return Ok((app_reference, None));
        }

        if Regex::new("^[^/]+\\.hf\\.space(?:/.*)?$")?.is_match(&app_reference) {
            return Ok((format!("https://{}", app_reference), None));
        }

        Ok((app_reference, None))
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
            return Err(Error::LoginFailed);
        }
        Ok(())
    }

    async fn fetch_config(http_client: &reqwest::Client, api_root: &str) -> Result<AppConfig> {
        let res = http_client
            .get(&format!("{}/{}", api_root, CONFIG_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::AppConfigUnavailable);
        }

        let json = res.json::<serde_json::Value>().await?;
        let config: AppConfigVersionOnly = serde_json::from_value(json.clone())?;

        if !Client::supports_version(&config.version) {
            eprintln!(
                "Warning: This client is supposed to work with Gradio 4, 5, and 6. The current version of the app is {}, which may cause issues.",
                config.version
            );
        }

        serde_json::from_value(json).map_err(Error::from)
    }

    async fn fetch_api_info(http_client: &reqwest::Client, api_root: &str) -> Result<ApiInfo> {
        let res = http_client
            .get(&format!("{}/{}", api_root, API_INFO_URL))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::ApiInfoUnavailable);
        }
        res.json::<ApiInfo>().await.map_err(Error::from)
    }

    fn resolve_fn_index(config: &AppConfig, route: &str) -> Result<i64> {
        let route = route.trim_start_matches('/');
        let found = config
            .dependencies
            .iter()
            .enumerate()
            .find(|(_i, d)| d.api_name == route)
            .ok_or_else(|| Error::InvalidRoute {
                route: route.to_string(),
            })?;

        if found.1.id == -1 {
            Ok(found.0 as i64)
        } else {
            Ok(found.1.id)
        }
    }

    fn join_url_path(base: &str, suffix: &str) -> String {
        let base = base.trim_end_matches('/');
        let suffix = suffix.trim_matches('/');

        if suffix.is_empty() {
            base.to_string()
        } else {
            format!("{}/{}", base, suffix)
        }
    }

    fn supports_version(version: &str) -> bool {
        version
            .split('.')
            .next()
            .and_then(|major| major.parse::<u64>().ok())
            .is_some_and(|major| (4..=6).contains(&major))
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn join_url_path_normalizes_slashes() {
        assert_eq!(
            Client::join_url_path("https://example.com/", "/gradio_api/"),
            "https://example.com/gradio_api"
        );
        assert_eq!(
            Client::join_url_path("https://example.com", "gradio_api"),
            "https://example.com/gradio_api"
        );
    }

    #[test]
    fn supports_gradio_4_5_and_6() {
        assert!(Client::supports_version("4.31.2"));
        assert!(Client::supports_version("5.0.0"));
        assert!(Client::supports_version("6.11.0"));
        assert!(!Client::supports_version("7.0.0"));
        assert!(!Client::supports_version("invalid"));
    }
}
