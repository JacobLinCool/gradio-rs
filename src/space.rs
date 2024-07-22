use crate::constants::*;
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceStatus {
    pub runtime: SpaceStatusRuntime,
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceStatusRuntime {
    pub stage: SpaceStatusRuntimeStage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpaceStatusRuntimeStage {
    #[serde(rename = "STOPPED")]
    Stopped,
    #[serde(rename = "SLEEPING")]
    Sleeping,
    #[serde(rename = "PAUSED")]
    Paused,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "RUNNING_BUILDING")]
    RunningBuilding,
    #[serde(rename = "BUILDING")]
    Building,
    #[serde(rename = "APP_STARTING")]
    AppStarting,
    #[serde(untagged)]
    Unknown(String),
}

pub async fn wake_up_space(client: &reqwest::Client, space_id: &str) -> Result<()> {
    let mut retries = 0;
    let max_retries = 12;
    let check_interval = 5000;

    loop {
        let response = client
            .get(&format!("https://huggingface.co/api/spaces/{}", space_id))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Error::msg(SPACE_STATUS_ERROR_MSG));
        }

        let status = response.json::<SpaceStatus>().await?;
        match status.runtime.stage {
            SpaceStatusRuntimeStage::Stopped
            | SpaceStatusRuntimeStage::Sleeping
            | SpaceStatusRuntimeStage::Building
            | SpaceStatusRuntimeStage::AppStarting => {
                // keep trying
            }
            SpaceStatusRuntimeStage::Paused => {
                return Err(Error::msg(format!(
                    "Space {} is paused by the author.",
                    space_id
                )));
            }
            SpaceStatusRuntimeStage::Running | SpaceStatusRuntimeStage::RunningBuilding => {
                return Ok(());
            }
            SpaceStatusRuntimeStage::Unknown(s) => {
                return Err(Error::msg(format!(
                    "Unknown runtime stage {} for space {}",
                    s, space_id
                )));
            }
        }

        if retries >= max_retries {
            return Err(Error::msg(format!(
                "Space {} is taking too long to start.",
                space_id
            )));
        }
        retries += 1;

        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval)).await;
    }
}

// test
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wake_up_space() {
        let client = reqwest::Client::new();
        let result = wake_up_space(&client, "gradio/hello_world").await;
        assert!(result.is_ok());
    }
}
