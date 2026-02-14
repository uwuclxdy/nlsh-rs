use crate::config::OllamaConfig;
use crate::error::NlshError;
use crate::providers::AIProvider;
use crate::providers::base::BaseProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    base: BaseProvider,
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

impl OllamaProvider {
    pub fn new(config: &OllamaConfig) -> Result<Self, NlshError> {
        Ok(Self {
            base: BaseProvider::new()?,
            base_url: config.base_url.clone(),
            model: config.model.clone(),
        })
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    async fn generate(&self, prompt: &str) -> Result<String, NlshError> {
        let url = format!("{}/api/generate", self.base_url);

        let request_body = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .base
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| NlshError::from_reqwest(e, "ollama"))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(NlshError::from_http_status(status, "ollama", &error_text));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| NlshError::InvalidResponse(e.to_string()))?;

        Ok(ollama_response.response)
    }
}
