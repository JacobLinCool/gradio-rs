// endpoints
pub const HOST_URL: &str = "host";
pub const API_URL: &str = "api/predict/";
pub const SSE_URL_V0: &str = "queue/join";
pub const SSE_DATA_URL_V0: &str = "queue/data";
pub const SSE_URL: &str = "queue/data";
pub const SSE_DATA_URL: &str = "queue/join";
pub const UPLOAD_URL: &str = "upload";
pub const LOGIN_URL: &str = "login";
pub const CONFIG_URL: &str = "config";
pub const API_INFO_URL: &str = "info";
pub const RUNTIME_URL: &str = "runtime";
pub const SLEEPTIME_URL: &str = "sleeptime";
pub const RAW_API_INFO_URL: &str = "info?serialize=False";
pub const SPACE_FETCHER_URL: &str = "https://gradio-space-api-fetcher-v2.hf.space/api";
pub const RESET_URL: &str = "reset";
pub const SPACE_URL: &str = "https://hf.space/{}";

// messages
pub const QUEUE_FULL_MSG: &str = "This application is currently busy. Please try again. ";
pub const BROKEN_CONNECTION_MSG: &str = "Connection errored out. ";
pub const CONFIG_ERROR_MSG: &str = "Could not resolve app config. ";
pub const SPACE_STATUS_ERROR_MSG: &str = "Could not get space status. ";
pub const API_INFO_ERROR_MSG: &str = "Could not get API info. ";
pub const SPACE_METADATA_ERROR_MSG: &str = "Space metadata could not be loaded. ";
pub const INVALID_URL_MSG: &str = "Invalid URL. A full URL path is required.";
pub const UNAUTHORIZED_MSG: &str = "Not authorized to access this space. ";
pub const INVALID_CREDENTIALS_MSG: &str = "Invalid credentials. Could not login. ";
pub const MISSING_CREDENTIALS_MSG: &str = "Login credentials are required to access this space.";
pub const NODEJS_FS_ERROR_MSG: &str =
    "File system access is only available in Node.js environments";
pub const ROOT_URL_ERROR_MSG: &str = "Root URL not found in client config";
pub const FILE_PROCESSING_ERROR_MSG: &str = "Error uploading file";
