use colored::*;
use inquire::{Confirm, Text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::cli::{
    print_check_with_bold_message, prompt_input, prompt_input_with_default, prompt_select,
};
use crate::common::clear_line;
mod migration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "provider")]
    pub active_provider: String,
    #[serde(default)]
    pub providers: MultiProviderConfig,
}

impl Config {
    pub fn get_provider_config(&self) -> ProviderConfig {
        match self.active_provider.as_str() {
            "gemini" => ProviderConfig {
                provider_type: "gemini".to_string(),
                config: ProviderSpecificConfig::Gemini {
                    gemini: self
                        .providers
                        .gemini
                        .clone()
                        .expect("gemini config not found for active provider"),
                },
            },
            "ollama" => ProviderConfig {
                provider_type: "ollama".to_string(),
                config: ProviderSpecificConfig::Ollama {
                    ollama: self
                        .providers
                        .ollama
                        .clone()
                        .expect("ollama config not found for active provider"),
                },
            },
            "openai" => ProviderConfig {
                provider_type: "openai".to_string(),
                config: ProviderSpecificConfig::OpenAI {
                    openai: self
                        .providers
                        .openai
                        .clone()
                        .expect("openai config not found for active provider"),
                },
            },
            _ => panic!("unknown provider type: {}", self.active_provider),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MultiProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini: Option<GeminiConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama: Option<OllamaConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<OpenAIConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(flatten)]
    pub config: ProviderSpecificConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ProviderSpecificConfig {
    Gemini { gemini: GeminiConfig },
    Ollama { ollama: OllamaConfig },
    OpenAI { openai: OpenAIConfig },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIConfig {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model: String,
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .expect("failed to get config directory")
        .join("nlsh-rs");

    fs::create_dir_all(&config_dir).expect("failed to create config directory");
    config_dir.join("config.toml")
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let contents = fs::read_to_string(&config_path)?;

    match toml::from_str::<Config>(&contents) {
        Ok(config) => Ok(config),
        Err(e) => {
            if migration::migrate_config(&config_path)? {
                let contents = fs::read_to_string(&config_path)?;
                Ok(toml::from_str(&contents)?)
            } else {
                Err(Box::new(e))
            }
        }
    }
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let toml_string = toml::to_string_pretty(config)?;
    fs::write(&config_path, toml_string)?;
    Ok(())
}

pub fn interactive_setup() -> Result<(), Box<dyn std::error::Error>> {
    let existing_config = load_config().ok();
    let current_provider = existing_config.as_ref().map(|c| c.active_provider.as_str());

    let providers = [
        ("Gemini API", "gemini"),
        ("Ollama", "ollama"),
        ("OpenAI Compatible", "openai"),
    ];

    let colored_providers: Vec<String> = providers
        .iter()
        .map(|(name, key)| {
            if Some(*key) == current_provider {
                format!("{}", name.green())
            } else {
                name.to_string()
            }
        })
        .collect();

    let selection = prompt_select("Select API Provider", &colored_providers, 0)?;

    let mut multi_providers = existing_config
        .as_ref()
        .map(|c| c.providers.clone())
        .unwrap_or_default();

    let has_saved_creds = match selection {
        0 => multi_providers.gemini.is_some(),
        1 => multi_providers.ollama.is_some(),
        2 => multi_providers.openai.is_some(),
        _ => unreachable!(),
    };

    let has_saved = if has_saved_creds && Some(providers[selection].1) != current_provider {
        let result = Confirm::new("Use saved credentials?")
            .with_default(true)
            .prompt()?;
        clear_line();
        result
    } else {
        false
    };

    let (active_provider, multi_providers) = if has_saved {
        (providers[selection].1.to_string(), multi_providers)
    } else {
        let new_config = match selection {
            0 => configure_gemini(multi_providers.gemini.as_ref())?,
            1 => configure_ollama(multi_providers.ollama.as_ref())?,
            2 => configure_openai(multi_providers.openai.as_ref())?,
            _ => unreachable!(),
        };

        match &new_config.config {
            ProviderSpecificConfig::Gemini { gemini } => {
                multi_providers.gemini = Some(gemini.clone());
            }
            ProviderSpecificConfig::Ollama { ollama } => {
                multi_providers.ollama = Some(ollama.clone());
            }
            ProviderSpecificConfig::OpenAI { openai } => {
                multi_providers.openai = Some(openai.clone());
            }
        }

        (new_config.provider_type, multi_providers)
    };

    let config = Config {
        active_provider,
        providers: multi_providers,
    };

    save_config(&config)?;

    print_check_with_bold_message("Configuration saved!");
    eprintln!();
    eprintln!("Provider: {}", providers[selection].0);

    let provider_config = config.get_provider_config();
    match &provider_config.config {
        ProviderSpecificConfig::Gemini { gemini } => {
            eprintln!("Model: {}", gemini.model);
        }
        ProviderSpecificConfig::Ollama { ollama } => {
            eprintln!("Model: {}", ollama.model);
            eprintln!("Base URL: {}", ollama.base_url);
        }
        ProviderSpecificConfig::OpenAI { openai } => {
            eprintln!("Model: {}", openai.model);
            eprintln!("Base URL: {}", openai.base_url);
        }
    }

    Ok(())
}

fn configure_gemini(
    existing: Option<&GeminiConfig>,
) -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let api_key = if let Some(e) = existing {
        prompt_input_with_default("Gemini API key", &e.api_key)?
    } else {
        prompt_input("Gemini API key")?
    };

    let model_default = existing
        .map(|e| e.model.as_str())
        .unwrap_or("gemini-flash-latest");
    let model = prompt_input_with_default("Model name", model_default)?;

    Ok(ProviderConfig {
        provider_type: "gemini".to_string(),
        config: ProviderSpecificConfig::Gemini {
            gemini: GeminiConfig { api_key, model },
        },
    })
}

fn configure_ollama(
    existing: Option<&OllamaConfig>,
) -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let url_default = existing
        .map(|e| e.base_url.as_str())
        .unwrap_or("http://localhost:11434");
    let base_url = prompt_input_with_default("Ollama base URL", url_default)?;

    let model = set_model_name(existing.map(|e| e.model.as_str()))?;

    Ok(ProviderConfig {
        provider_type: "ollama".to_string(),
        config: ProviderSpecificConfig::Ollama {
            ollama: OllamaConfig { base_url, model },
        },
    })
}

fn configure_openai(
    existing: Option<&OpenAIConfig>,
) -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let url_default = existing
        .map(|e| e.base_url.as_str())
        .unwrap_or("https://api.openai.com/v1");
    let base_url = prompt_input_with_default("API base URL", url_default)?;

    let api_key = {
        let mut text = Text::new("API key (optional for local servers)")
            .with_help_message("Leave empty for local servers like LM Studio");
        if let Some(saved) = existing.and_then(|e| e.api_key.as_deref()) {
            text = text.with_default(saved);
        }
        text.prompt_skippable()?
    };

    let model = set_model_name(existing.map(|e| e.model.as_str()))?;

    Ok(ProviderConfig {
        provider_type: "openai".to_string(),
        config: ProviderSpecificConfig::OpenAI {
            openai: OpenAIConfig {
                base_url,
                api_key,
                model,
            },
        },
    })
}

fn set_model_name(default: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let model = if let Some(def) = default {
        prompt_input_with_default("Model name:", def)?
    } else {
        prompt_input("Model name:")?
    };

    if model.trim().is_empty() {
        eprintln!("{}", "Model name cannot be empty".red());
        return set_model_name(default);
    }

    Ok(model)
}
