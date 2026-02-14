mod base;
mod gemini;
mod ollama;
mod openai;

use crate::config::{Config, ProviderSpecificConfig};
use crate::error::NlshError;
use async_trait::async_trait;

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String, NlshError>;
}

pub fn create_provider(config: &Config) -> Result<Box<dyn AIProvider>, NlshError> {
    let provider = config.get_provider_config();
    match &provider.config {
        ProviderSpecificConfig::Gemini { gemini } => {
            Ok(Box::new(gemini::GeminiProvider::new(gemini)?))
        }
        ProviderSpecificConfig::Ollama { ollama } => {
            Ok(Box::new(ollama::OllamaProvider::new(ollama)?))
        }
        ProviderSpecificConfig::OpenAI { openai } => {
            Ok(Box::new(openai::OpenAIProvider::new(openai)?))
        }
    }
}
