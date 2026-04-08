use futures_util::stream::StreamExt;
use rand::{distributions::Alphanumeric, Rng};
use reqwest_eventsource::{Event, EventSource};

use crate::{
    structs::{QueueDataMessage, QueueDataMessageOutput, QueueJoinResponse},
    Error, Result,
};

pub struct PredictionStream {
    pub es: EventSource,
    pub http_client: reqwest::Client,
    pub api_root: String,
    pub session_hash: String,
    pub event_id: String,
    pub fn_index: i64,
    protocol: String,
    pending_diff_streams: Option<Vec<serde_json::Value>>,
}

impl PredictionStream {
    pub async fn new(
        http_client: &reqwest::Client,
        api_root: &str,
        protocol: &str,
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
            return Err(Error::CannotJoinTaskQueue);
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
            protocol: protocol.to_string(),
            pending_diff_streams: None,
        })
    }

    pub async fn next(&mut self) -> Option<Result<QueueDataMessage>> {
        let event = self.es.next().await;
        if event.is_none() {
            return Some(Err(Error::StreamEnded));
        }
        let event = event.unwrap();

        match event {
            Ok(Event::Open) => Some(Ok(QueueDataMessage::Open)),
            Ok(Event::Message(message)) => match serde_json::from_str(&message.data) {
                Ok(mut queue_message) => {
                    if let Err(err) = normalize_queue_message(
                        &self.protocol,
                        &mut self.pending_diff_streams,
                        &mut queue_message,
                    ) {
                        return Some(Err(err));
                    }

                    if matches!(queue_message, QueueDataMessage::CloseStream) {
                        self.es.close();
                    }

                    Some(Ok(queue_message))
                }
                Err(err) => Some(Err(Error::ServerProtocol {
                    message: format!("{:#?}", err),
                })),
            },
            Err(err) => Some(Err(Error::ClientProtocol {
                message: format!("{:#?}", err),
            })),
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

fn normalize_queue_message(
    protocol: &str,
    pending_diff_streams: &mut Option<Vec<serde_json::Value>>,
    message: &mut QueueDataMessage,
) -> Result<()> {
    match message {
        QueueDataMessage::ProcessGenerating {
            output, success, ..
        }
        | QueueDataMessage::ProcessStreaming {
            output, success, ..
        } => {
            if *success {
                normalize_diff_output(protocol, pending_diff_streams, output)?;
            }
        }
        QueueDataMessage::ProcessCompleted { .. }
        | QueueDataMessage::UnexpectedError { .. }
        | QueueDataMessage::CloseStream => {
            *pending_diff_streams = None;
        }
        _ => {}
    }

    Ok(())
}

fn normalize_diff_output(
    protocol: &str,
    pending_diff_streams: &mut Option<Vec<serde_json::Value>>,
    output: &mut QueueDataMessageOutput,
) -> Result<()> {
    if !supports_diff_stream(protocol) {
        return Ok(());
    }

    let Some(data) = output.data_mut() else {
        return Ok(());
    };

    if pending_diff_streams.is_none() {
        *pending_diff_streams = Some(data.clone());
        return Ok(());
    }

    let pending = pending_diff_streams
        .as_mut()
        .expect("pending diff streams should exist");
    for (index, value) in data.iter_mut().enumerate() {
        let previous = pending
            .get(index)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let normalized = apply_diff(previous, value.clone())?;

        if index < pending.len() {
            pending[index] = normalized.clone();
        } else {
            pending.push(normalized.clone());
        }

        *value = normalized;
    }

    Ok(())
}

fn supports_diff_stream(protocol: &str) -> bool {
    matches!(protocol, "sse_v2" | "sse_v2.1" | "sse_v3")
}

fn apply_diff(target: serde_json::Value, diff: serde_json::Value) -> Result<serde_json::Value> {
    let Some(ops) = diff.as_array() else {
        return Ok(diff);
    };

    if !ops.iter().all(is_diff_operation) {
        return Ok(diff);
    }

    let mut output = target;
    for op in ops {
        let op = op.as_array().ok_or(Error::InvalidDiffOperationPayload)?;
        let action = op[0].as_str().ok_or(Error::DiffActionMustBeString)?;
        let path = parse_path_segments(&op[1])?;
        let value = op[2].clone();
        output = apply_edit(output, &path, action, value)?;
    }

    Ok(output)
}

fn is_diff_operation(value: &serde_json::Value) -> bool {
    let Some(op) = value.as_array() else {
        return false;
    };

    if op.len() != 3 {
        return false;
    }

    op[0].is_string() && op[1].is_array()
}

#[derive(Clone, Debug)]
enum PathSegment {
    Index(usize),
    Key(String),
}

fn parse_path_segments(path: &serde_json::Value) -> Result<Vec<PathSegment>> {
    let Some(path) = path.as_array() else {
        return Err(Error::DiffPathMustBeArray);
    };

    path.iter()
        .map(|segment| {
            if let Some(index) = segment.as_u64() {
                Ok(PathSegment::Index(index as usize))
            } else if let Some(key) = segment.as_str() {
                Ok(PathSegment::Key(key.to_string()))
            } else {
                Err(Error::InvalidDiffPathSegment)
            }
        })
        .collect()
}

fn apply_edit(
    target: serde_json::Value,
    path: &[PathSegment],
    action: &str,
    value: serde_json::Value,
) -> Result<serde_json::Value> {
    if path.is_empty() {
        return apply_root_edit(target, action, value);
    }

    match target {
        serde_json::Value::Array(mut items) => {
            let PathSegment::Index(index) = &path[0] else {
                return Err(Error::ArrayDiffPathMustUseIndexes);
            };
            let index = *index;

            if path.len() == 1 {
                match action {
                    "replace" => {
                        let slot = items.get_mut(index).ok_or(Error::DiffIndexOutOfBounds)?;
                        *slot = value;
                    }
                    "append" => {
                        let slot = items.get_mut(index).ok_or(Error::DiffIndexOutOfBounds)?;
                        append_value(slot, value)?;
                    }
                    "add" => {
                        if index > items.len() {
                            return Err(Error::DiffIndexOutOfBounds);
                        }
                        items.insert(index, value);
                    }
                    "delete" => {
                        if index >= items.len() {
                            return Err(Error::DiffIndexOutOfBounds);
                        }
                        items.remove(index);
                    }
                    _ => {
                        return Err(Error::UnknownDiffAction {
                            action: action.to_string(),
                        });
                    }
                }
            } else {
                let slot = items.get_mut(index).ok_or(Error::DiffIndexOutOfBounds)?;
                let child = std::mem::take(slot);
                *slot = apply_edit(child, &path[1..], action, value)?;
            }

            Ok(serde_json::Value::Array(items))
        }
        serde_json::Value::Object(mut fields) => {
            let PathSegment::Key(key) = &path[0] else {
                return Err(Error::ObjectDiffPathMustUseKeys);
            };

            if path.len() == 1 {
                match action {
                    "replace" => {
                        fields.insert(key.clone(), value);
                    }
                    "append" => {
                        let slot = fields.get_mut(key).ok_or(Error::DiffKeyNotFound)?;
                        append_value(slot, value)?;
                    }
                    "add" => {
                        fields.insert(key.clone(), value);
                    }
                    "delete" => {
                        fields.remove(key);
                    }
                    _ => {
                        return Err(Error::UnknownDiffAction {
                            action: action.to_string(),
                        });
                    }
                }
            } else {
                let slot = fields.get_mut(key).ok_or(Error::DiffKeyNotFound)?;
                let child = std::mem::take(slot);
                *slot = apply_edit(child, &path[1..], action, value)?;
            }

            Ok(serde_json::Value::Object(fields))
        }
        _ => Err(Error::CannotApplyNestedDiffToScalar),
    }
}

fn apply_root_edit(
    target: serde_json::Value,
    action: &str,
    value: serde_json::Value,
) -> Result<serde_json::Value> {
    match action {
        "replace" => Ok(value),
        "append" => append_root_value(target, value),
        _ => Err(Error::UnsupportedRootDiffAction {
            action: action.to_string(),
        }),
    }
}

fn append_root_value(
    target: serde_json::Value,
    value: serde_json::Value,
) -> Result<serde_json::Value> {
    match (target, value) {
        (serde_json::Value::String(mut lhs), serde_json::Value::String(rhs)) => {
            lhs.push_str(&rhs);
            Ok(serde_json::Value::String(lhs))
        }
        (serde_json::Value::Array(mut lhs), serde_json::Value::Array(rhs)) => {
            lhs.extend(rhs);
            Ok(serde_json::Value::Array(lhs))
        }
        _ => Err(Error::AppendDiffTypeMismatch),
    }
}

fn append_value(target: &mut serde_json::Value, value: serde_json::Value) -> Result<()> {
    let new_value = append_root_value(std::mem::take(target), value)?;
    *target = new_value;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{apply_diff, normalize_queue_message};
    use crate::structs::{QueueDataMessage, QueueDataMessageOutput};

    #[test]
    fn applies_nested_diff_operations() {
        let base = json!({
            "text": "hel",
            "items": [1, 3]
        });
        let diff = json!([["append", ["text"], "lo"], ["add", ["items", 1], 2]]);

        let applied = apply_diff(base, diff).unwrap();

        assert_eq!(
            applied,
            json!({
                "text": "hello",
                "items": [1, 2, 3]
            })
        );
    }

    #[test]
    fn ignores_non_diff_payloads() {
        let value = json!({"text": "hello"});
        assert_eq!(apply_diff(json!("ignored"), value.clone()).unwrap(), value);
    }

    #[test]
    fn normalizes_process_generating_diffs_for_sse_v3() {
        let mut pending = None;
        let mut first = QueueDataMessage::ProcessGenerating {
            event_id: Some("evt".to_string()),
            output: QueueDataMessageOutput::Success {
                data: vec![json!("Hel")],
                duration: None,
                render_config: None,
                changed_state_ids: None,
            },
            success: true,
            time_limit: None,
            progress_data: None,
        };
        normalize_queue_message("sse_v3", &mut pending, &mut first).unwrap();

        let mut second = QueueDataMessage::ProcessGenerating {
            event_id: Some("evt".to_string()),
            output: QueueDataMessageOutput::Success {
                data: vec![json!([["append", [], "lo"]])],
                duration: None,
                render_config: None,
                changed_state_ids: None,
            },
            success: true,
            time_limit: None,
            progress_data: None,
        };
        normalize_queue_message("sse_v3", &mut pending, &mut second).unwrap();

        match second {
            QueueDataMessage::ProcessGenerating { output, .. } => match output {
                QueueDataMessageOutput::Success { data, .. } => {
                    assert_eq!(data, vec![json!("Hello")]);
                }
                QueueDataMessageOutput::Error { .. } => panic!("expected success output"),
            },
            _ => panic!("expected process generating message"),
        }
    }

    #[test]
    fn clears_pending_diffs_when_stream_closes() {
        let mut pending = Some(vec![json!("Hello")]);
        let mut message = QueueDataMessage::CloseStream;

        normalize_queue_message("sse_v3", &mut pending, &mut message).unwrap();

        assert!(pending.is_none());
    }
}
