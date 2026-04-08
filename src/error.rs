pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)]
    EventSource(#[from] reqwest_eventsource::CannotCloneRequestError),

    #[error("login failed")]
    LoginFailed,
    #[error("could not resolve app config")]
    AppConfigUnavailable,
    #[error("could not get API info")]
    ApiInfoUnavailable,
    #[error("invalid route: {route}")]
    InvalidRoute { route: String },
    #[error("cannot join task queue")]
    CannotJoinTaskQueue,
    #[error("stream ended unexpectedly")]
    StreamEndedUnexpectedly,
    #[error("stream ended")]
    StreamEnded,
    #[error("unexpected remote error: {message}")]
    UnexpectedRemoteError { message: String },
    #[error("remote error: {message}")]
    RemoteError { message: String },
    #[error("invalid file path")]
    InvalidFilePath,
    #[error("error uploading file")]
    FileUploadFailed,
    #[error("invalid file upload response")]
    InvalidFileUploadResponse,
    #[error("expected file output")]
    ExpectedFileOutput,
    #[error("expected value output")]
    ExpectedValueOutput,
    #[error("no URL available for file")]
    NoFileUrl,
    #[error("could not get space status")]
    SpaceStatusUnavailable,
    #[error("space {space_id} is paused by the author")]
    SpacePaused { space_id: String },
    #[error("unknown runtime stage {stage} for space {space_id}")]
    UnknownRuntimeStage { stage: String, space_id: String },
    #[error("space {space_id} is taking too long to start")]
    SpaceStartupTimeout { space_id: String },
    #[error("server error: {message}")]
    ServerProtocol { message: String },
    #[error("client error: {message}")]
    ClientProtocol { message: String },
    #[error("invalid diff operation payload")]
    InvalidDiffOperationPayload,
    #[error("diff action must be a string")]
    DiffActionMustBeString,
    #[error("diff path must be an array")]
    DiffPathMustBeArray,
    #[error("diff path segment must be a string or integer")]
    InvalidDiffPathSegment,
    #[error("array diff path must use integer indexes")]
    ArrayDiffPathMustUseIndexes,
    #[error("diff index out of bounds")]
    DiffIndexOutOfBounds,
    #[error("object diff path must use string keys")]
    ObjectDiffPathMustUseKeys,
    #[error("diff key not found")]
    DiffKeyNotFound,
    #[error("cannot apply nested diff to scalar value")]
    CannotApplyNestedDiffToScalar,
    #[error("unsupported root diff action: {action}")]
    UnsupportedRootDiffAction { action: String },
    #[error("unknown diff action: {action}")]
    UnknownDiffAction { action: String },
    #[error("append diff requires string or array values")]
    AppendDiffTypeMismatch,
}
