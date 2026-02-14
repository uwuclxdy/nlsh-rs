use crate::config::GeminiConfig;
use crate::error::NlshError;
use crate::providers::AIProvider;
use crate::providers::base::BaseProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GeminiProvider {
    base: BaseProvider,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ContentResponse>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentResponse {
    parts: Option<Vec<PartResponse>>,
}

#[derive(Deserialize)]
struct PartResponse {
    text: String,
}

#[derive(Deserialize)]
struct GeminiErrorResponse {
    error: GeminiErrorDetail,
}

#[derive(Deserialize)]
struct GeminiErrorDetail {
    code: u16,
    message: String,
}

impl GeminiProvider {
    pub fn new(config: &GeminiConfig) -> Result<Self, NlshError> {
        Ok(Self {
            base: BaseProvider::new()?,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        })
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn generate(&self, prompt: &str) -> Result<String, NlshError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
        };

        let response = self
            .base
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| NlshError::from_reqwest(e, "gemini"))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| NlshError::InvalidResponse(e.to_string()))?;

        if !status.is_success() {
            if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&response_text)
            {
                return Err(NlshError::from_http_status(
                    reqwest::StatusCode::from_u16(error_response.error.code).unwrap_or(status),
                    "gemini",
                    &error_response.error.message,
                ));
            }

            return Err(NlshError::from_http_status(
                status,
                "gemini",
                &response_text,
            ));
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&response_text)
            .map_err(|e| NlshError::InvalidResponse(e.to_string()))?;

        let candidates = gemini_response
            .candidates
            .ok_or_else(|| NlshError::InvalidResponse("no candidates in response".to_string()))?;

        let candidate = candidates
            .first()
            .ok_or_else(|| NlshError::InvalidResponse("empty candidates list".to_string()))?;

        if let Some(finish_reason) = &candidate.finish_reason
            && (finish_reason == "SAFETY" || finish_reason == "RECITATION")
        {
            return Err(NlshError::InvalidResponse(format!(
                "content blocked by gemini: {}",
                finish_reason.to_lowercase()
            )));
        }

        let content = candidate
            .content
            .as_ref()
            .ok_or_else(|| NlshError::InvalidResponse("no content in response".to_string()))?;

        let parts = content
            .parts
            .as_ref()
            .ok_or_else(|| NlshError::InvalidResponse("no parts in content".to_string()))?;

        let text = parts
            .first()
            .map(|p| p.text.clone())
            .ok_or_else(|| NlshError::InvalidResponse("no text in response".to_string()))?;

        Ok(text)
    }
}
