use thiserror::Error;

#[derive(Error, Debug)]
pub enum NlshError {
    #[error("failed to connect to {provider}: {message}")]
    ConnectionFailed { provider: String, message: String },

    #[error("authentication failed: invalid API key")]
    InvalidApiKey,

    #[error("authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("rate limit exceeded{}", if retry_after.is_some() { format!(". retry after {} seconds", retry_after.unwrap()) } else { ". please try again later".to_string() })]
    RateLimitExceeded { retry_after: Option<u64> },

    #[error("server error from {provider}: {message}")]
    ServerError { provider: String, message: String },

    #[error("request timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    #[error("invalid response from API: {0}")]
    InvalidResponse(String),

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl NlshError {
    pub fn connection_failed(provider: impl Into<String>, message: impl Into<String>) -> Self {
        NlshError::ConnectionFailed {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn server_error(provider: impl Into<String>, message: impl Into<String>) -> Self {
        NlshError::ServerError {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn timeout(seconds: u64) -> Self {
        NlshError::Timeout { seconds }
    }

    pub fn auth_failed(message: impl Into<String>) -> Self {
        NlshError::AuthenticationFailed {
            message: message.into(),
        }
    }

    pub fn from_http_status(status: reqwest::StatusCode, provider: &str, body: &str) -> NlshError {
        match status.as_u16() {
            401 | 403 => {
                if body.contains("key") || body.contains("api") || body.contains("token") {
                    NlshError::InvalidApiKey
                } else {
                    NlshError::auth_failed(body)
                }
            }
            404 => {
                if body.contains("model") {
                    NlshError::ModelNotFound(body.to_string())
                } else {
                    NlshError::InvalidResponse(format!("endpoint not found: {}", body))
                }
            }
            429 => {
                let retry_after = if body.contains("retry") {
                    body.split("retry in ")
                        .nth(1)
                        .and_then(|s| s.split('s').next())
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(|f| f.ceil() as u64)
                } else {
                    None
                };
                NlshError::RateLimitExceeded { retry_after }
            }
            500..=599 => NlshError::server_error(provider, body),
            _ => NlshError::InvalidResponse(format!("{}: {}", status, body)),
        }
    }

    pub fn from_reqwest(error: reqwest::Error, provider: &str) -> NlshError {
        if error.is_timeout() {
            NlshError::timeout(30)
        } else if error.is_connect() {
            NlshError::connection_failed(
                provider,
                "cannot connect. check if the service is running and the URL is correct",
            )
        } else if error.is_request() {
            NlshError::NetworkError("invalid request".to_string())
        } else if let Some(status) = error.status() {
            NlshError::from_http_status(status, provider, &error.to_string())
        } else {
            NlshError::NetworkError(error.to_string())
        }
    }
}
