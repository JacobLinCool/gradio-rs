use crate::client::{Client, ClientOptions};
use crate::data::{GradioFileData, PredictionInput, PredictionOutput};
use anyhow::Result;
#[cfg(not(feature = "wasm"))]
use std::path::Path;
#[cfg(feature = "wasm")]
use tokio::runtime::Builder;
#[cfg(not(feature = "wasm"))]
use tokio::runtime::Runtime;

impl Client {
    pub fn new_sync(app_reference: &str, options: ClientOptions) -> Result<Self> {
        #[cfg(not(feature = "wasm"))]
        let rt = Runtime::new()?;
        #[cfg(feature = "wasm")]
        let rt = Builder::new_current_thread().enable_all().build()?;
        let client = rt.block_on(Client::new(app_reference, options))?;
        Ok(client)
    }

    pub fn predict_sync(
        &self,
        path: &str,
        inputs: Vec<PredictionInput>,
    ) -> Result<Vec<PredictionOutput>> {
        #[cfg(not(feature = "wasm"))]
        let rt = Runtime::new()?;
        #[cfg(feature = "wasm")]
        let rt = Builder::new_current_thread().enable_all().build()?;
        let output = rt.block_on(self.predict(path, inputs))?;
        Ok(output)
    }
}

impl GradioFileData {
    pub fn download_sync(&self, http_client: Option<reqwest::Client>) -> Result<bytes::Bytes> {
        #[cfg(not(feature = "wasm"))]
        let rt = Runtime::new()?;
        #[cfg(feature = "wasm")]
        let rt = Builder::new_current_thread().enable_all().build()?;
        let bytes = rt.block_on(self.download(http_client))?;
        Ok(bytes)
    }

    #[cfg(not(feature = "wasm"))]
    pub fn save_to_path_sync(
        &self,
        path: impl AsRef<Path>,
        http_client: Option<reqwest::Client>,
    ) -> Result<()> {
        #[cfg(not(feature = "wasm"))]
        let rt = Runtime::new()?;
        #[cfg(feature = "wasm")]
        let rt = Builder::new_current_thread().enable_all().build()?;
        rt.block_on(self.save_to_path(path, http_client))?;
        Ok(())
    }
}
