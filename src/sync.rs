use crate::client::{Client, ClientOptions};
use crate::data::{GradioFileData, PredictionInput, PredictionOutput};
use crate::stream::PredictionStream;
use crate::structs::QueueDataMessage;
use anyhow::{Error, Result};
use std::path::Path;
use tokio::runtime::Runtime;

impl Client {
    pub fn new_sync(app_reference: &str, options: ClientOptions) -> Result<Self> {
        let rt = Runtime::new()?;
        let client = rt.block_on(Client::new(app_reference, options))?;
        Ok(client)
    }

    pub fn submit_sync(
        &self,
        path: &str,
        inputs: Vec<PredictionInput>,
    ) -> Result<PredictionStream> {
        let rt = Runtime::new()?;
        let output = rt.block_on(self.submit(path, inputs))?;
        Ok(output)
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

impl PredictionStream {
    pub fn next_sync(&mut self) -> Option<Result<QueueDataMessage>> {
        let rt = Runtime::new();
        if rt.is_err() {
            return Some(Err(Error::msg("Runtime error")));
        }
        let rt = rt.unwrap();
        let output = rt.block_on(self.next())?;
        Some(output)
    }
}
