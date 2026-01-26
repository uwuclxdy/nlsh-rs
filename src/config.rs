use crate::common::handle_interrupt;
use crate::common::set_file_permissions;
use colored::*;
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        .expect("Failed to get config directory")
        .join("nlsh-rs");

    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    config_dir.join("config.toml")
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let contents = fs::read_to_string(&config_path)?;

    match toml::from_str::<Config>(&contents) {
        Ok(config) => Ok(config),
        Err(e) => {
            if crate::config_migration::migrate_config(&config_path)? {
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
    set_file_permissions(&config_path)?;
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

    let selection = handle_interrupt(
        Select::new()
            .with_prompt("Select API Provider")
            .items(&colored_providers)
            .default(0)
            .interact(),
    )?;

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
        handle_interrupt(
            dialoguer::Confirm::new()
                .with_prompt("use saved credentials?")
                .default(true)
                .interact(),
        )?
    } else {
        false
    };

    let (active_provider, multi_providers) = if has_saved {
        (providers[selection].1.to_string(), multi_providers)
    } else {
        let new_config = match selection {
            0 => configure_gemini()?,
            1 => configure_ollama()?,
            2 => configure_openai()?,
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

    eprintln!("{}", "✓ Configuration saved!".green().bold());
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

fn configure_gemini() -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let api_key: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter your Gemini API key")
            .interact_text(),
    )?;

    let model: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter model name")
            .default("gemini-2.5-flash".to_string())
            .interact_text(),
    )?;

    Ok(ProviderConfig {
        provider_type: "gemini".to_string(),
        config: ProviderSpecificConfig::Gemini {
            gemini: GeminiConfig { api_key, model },
        },
    })
}

fn configure_ollama() -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let base_url: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter Ollama base URL")
            .default("http://localhost:11434".to_string())
            .interact_text(),
    )?;

    let model: String =
        handle_interrupt(Input::new().with_prompt("Enter model name").interact_text())?;

    Ok(ProviderConfig {
        provider_type: "ollama".to_string(),
        config: ProviderSpecificConfig::Ollama {
            ollama: OllamaConfig { base_url, model },
        },
    })
}

fn configure_openai() -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let base_url: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter API base URL")
            .default("https://api.openai.com/v1".to_string())
            .interact_text(),
    )?;

    let api_key: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter API key (leave empty for local servers like LM Studio)")
            .allow_empty(true)
            .interact_text(),
    )?;

    let api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };

    let model: String =
        handle_interrupt(Input::new().with_prompt("Enter model name").interact_text())?;

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
