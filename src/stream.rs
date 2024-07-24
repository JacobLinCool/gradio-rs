use anyhow::{Error, Result};
use futures_util::stream::StreamExt;
use rand::{distributions::Alphanumeric, Rng};
use reqwest_eventsource::{Event, EventSource};

use crate::structs::{QueueDataMessage, QueueJoinResponse};

pub struct PredictionStream {
    pub es: EventSource,
    pub http_client: reqwest::Client,
    pub api_root: String,
    pub session_hash: String,
    pub event_id: String,
    pub fn_index: i64,
}

impl PredictionStream {
    pub async fn new(
        http_client: &reqwest::Client,
        api_root: &str,
        fn_index: impl Into<i64>,
        data: Vec<serde_json::Value>,
    ) -> Result<Self> {
        let http_client = http_client.clone();
        let fn_index = fn_index.into();
        let api_root = api_root.to_string();

        let session_hash: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        let url = format!("{}/queue/join", api_root);
        let payload = serde_json::json!({
            "fn_index": fn_index,
            "data": data,
            "session_hash": session_hash
        });
        let res = http_client.post(&url).json(&payload).send().await?;
        if !res.status().is_success() {
            return Err(Error::msg("Cannot join task queue"));
        }
        let res = res.json::<QueueJoinResponse>().await?;
        let event_id = res.event_id;

        let url = format!("{}/queue/data?session_hash={}", api_root, session_hash);
        let es = EventSource::new(http_client.get(url))?;

        Ok(Self {
            es,
            http_client,
            api_root,
            session_hash,
            event_id,
            fn_index,
        })
    }

    pub async fn next(&mut self) -> Option<Result<QueueDataMessage>> {
        let event = self.es.next().await;
        let event = event.unwrap();

        match event {
            Ok(Event::Open) => Some(Ok(QueueDataMessage::Open)),
            Ok(Event::Message(message)) => match serde_json::from_str(&message.data) {
                Ok(message) => Some(Ok(message)),
                Err(err) => Some(Err(Error::msg(format!("Server Error: {:#?}", err)))),
            },
            Err(err) => Some(Err(Error::msg(format!("Client Error: {:#?}", err)))),
        }
    }

    pub async fn cancel(&mut self) -> Result<()> {
        self.es.close();

        let url = format!("{}/cancel", self.api_root);
        let payload = serde_json::json!({
            "event_id": self.event_id,
            "session_hash": self.session_hash,
            "fn_index": self.fn_index,
        });
        let _ = self.http_client.post(&url).json(&payload).send().await;

        let url = format!("{}/reset", self.api_root);
        let payload = serde_json::json!({
            "event_id": self.event_id,
        });
        let _ = self.http_client.post(&url).json(&payload).send().await;

        Ok(())
    }
}
