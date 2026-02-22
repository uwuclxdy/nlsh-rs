use crate::error::NlshError;
use reqwest::Client;
use std::time::Duration;

/// common timeout for all provider requests
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// creates a new HTTP client with default timeout.
///
/// this ensures consistent timeout behavior across all providers.
pub fn create_http_client() -> Result<Client, NlshError> {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .map_err(|e| NlshError::ConfigError(e.to_string()))
}

/// base provider struct containing common HTTP client.
///
/// this reduces duplication across providers by centralizing
/// the client creation and timeout configuration.
pub struct BaseProvider {
    pub client: Client,
}

impl BaseProvider {
    /// creates a new base provider with HTTP client.
    pub fn new() -> Result<Self, NlshError> {
        Ok(Self {
            client: create_http_client()?,
        })
    }

    /// checks an HTTP response status and returns an appropriate error for non-success codes.
    pub async fn check_response(
        response: reqwest::Response,
        provider: &str,
    ) -> Result<reqwest::Response, NlshError> {
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(NlshError::from_http_status(status, provider, &error_text));
        }
        Ok(response)
    }
}
