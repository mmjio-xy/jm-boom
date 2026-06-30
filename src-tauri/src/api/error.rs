use std::fmt;

#[derive(Debug)]
pub enum ApiErrorKind {
    Api,
    Cache,
    Client,
    Decode,
    Decrypt,
    Empty,
    Http,
    MissingData,
    Network,
    Payload,
    UnsupportedEndpoint,
}

#[derive(Debug)]
pub struct ApiError {
    kind: ApiErrorKind,
    message: String,
}

impl ApiError {
    pub(crate) fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ApiError {}
