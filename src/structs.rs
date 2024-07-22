use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HuggingFaceAPIHost {
    pub host: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub components: Vec<ComponentMeta>,
    pub dependencies: Vec<Dependency>,
    pub mode: String,
    pub root: String,
    pub theme: String,
    pub title: String,
    pub version: String,
    pub protocol: String,
    pub layout: serde_json::Value,
    pub auth_message: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
    pub head: Option<String>,
    pub root_url: Option<String>,
    pub space_id: Option<String>,
    pub stylesheets: Vec<String>,
    pub path: Option<String>,
    pub theme_hash: Option<StringOrI64>,
    pub username: Option<String>,
    pub max_file_size: Option<i64>,
    #[serde(default)]
    pub auth_required: Option<bool>,
    #[serde(default)]
    pub analytics_enabled: Option<bool>,
    #[serde(default)]
    pub connect_heartbeat: Option<bool>,
    #[serde(default)]
    pub dev_mode: Option<bool>,
    #[serde(default)]
    pub enable_queue: Option<bool>,
    #[serde(default)]
    pub show_error: Option<bool>,
    #[serde(default)]
    pub is_space: Option<bool>,
    #[serde(default)]
    pub is_colab: Option<bool>,
    #[serde(default)]
    pub show_api: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ComponentMeta {
    pub r#type: String,
    pub id: StringOrI64,
    pub props: serde_json::Value,
    pub component_class_id: String,
    pub component: Option<serde_json::Value>,
    pub value: Option<serde_json::Value>,
    pub key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dependency {
    pub api_name: String,
    #[serde(default)]
    pub id: i64,
    pub queue: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrI64 {
    String(String),
    I64(i64),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiInfo {
    named_endpoints: HashMap<String, EndpointInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EndpointInfo {
    parameters: Vec<ApiData>,
    returns: Vec<ApiData>,
    #[serde(default)]
    show_api: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiData {
    label: Option<String>,
    parameter_name: Option<String>,
    parameter_default: Option<serde_json::Value>,
    parameter_has_default: Option<bool>,
    component: String,
    example_input: Option<serde_json::Value>,
    r#type: ApiDataType,
    python_type: ApiDataPythonType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiDataType {
    r#type: String,
    #[serde(default)]
    description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiDataPythonType {
    r#type: String,
    description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueueJoinResponse {
    pub event_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum QueueDataMessage {
    InQueue {
        event_id: String,
        msg: String,
        rank: i64,
        queue_size: i64,
        rank_eta: f64,
    },
    Processing {
        event_id: String,
        msg: String,
        eta: f64,
    },
    Completed {
        event_id: String,
        msg: String,
        output: QueueDataMessageOutput,
        success: bool,
    },
    Unknown(serde_json::Value),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum QueueDataMessageOutput {
    Success {
        data: Vec<serde_json::Value>,
        duration: f64,
    },
    Error {
        error: Option<String>,
    },
}
