use crate::client::{Client, ClientOptions};
use crate::data::{GradioFileData, PredictionInput, PredictionOutput};
use anyhow::Result;
use std::path::Path;
use tokio::runtime::Runtime;

impl Client {
    pub fn new_sync(app_reference: &str, options: ClientOptions) -> Result<Self> {
        let rt = Runtime::new()?;
        let client = rt.block_on(Client::new(app_reference, options))?;
        Ok(client)
    }

    pub fn predict_sync(
        &self,
        path: &str,
        inputs: Vec<PredictionInput>,
    ) -> Result<Vec<PredictionOutput>> {
        let rt = Runtime::new()?;
        let output = rt.block_on(self.predict(path, inputs))?;
        Ok(output)
    }
}

impl GradioFileData {
    pub fn download_sync(&self, http_client: Option<reqwest::Client>) -> Result<bytes::Bytes> {
        let rt = Runtime::new()?;
        let bytes = rt.block_on(self.download(http_client))?;
        Ok(bytes)
    }

    pub fn save_to_path_sync(
        &self,
        path: impl AsRef<Path>,
        http_client: Option<reqwest::Client>,
    ) -> Result<()> {
        let rt = Runtime::new()?;
        rt.block_on(self.save_to_path(path, http_client))?;
        Ok(())
    }
}
