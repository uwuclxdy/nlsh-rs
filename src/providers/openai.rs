use crate::config::OpenAIConfig;
use crate::error::NlshError;
use crate::providers::AIProvider;
use crate::providers::base::BaseProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OpenAIProvider {
    base: BaseProvider,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Deserialize)]
struct MessageResponse {
    content: String,
}

impl OpenAIProvider {
    pub fn new(config: &OpenAIConfig) -> Result<Self, NlshError> {
        Ok(Self {
            base: BaseProvider::new()?,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        })
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn generate(&self, prompt: &str) -> Result<String, NlshError> {
        let url = if self.base_url.ends_with("/v1") {
            format!("{}/chat/completions", self.base_url)
        } else {
            format!("{}/v1/chat/completions", self.base_url)
        };

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
        };

        let response = if let Some(ref api_key) = self.api_key {
            self.base
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| NlshError::from_reqwest(e, "openai"))?
        } else {
            self.base
                .client
                .post(&url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| NlshError::from_reqwest(e, "openai"))?
        };

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(NlshError::from_http_status(status, "openai", &error_text));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| NlshError::InvalidResponse(e.to_string()))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| NlshError::InvalidResponse("no response from openai".to_string()))?;

        Ok(content)
    }
}
