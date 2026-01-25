use colored::*;
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

fn handle_interrupt<T>(
    result: Result<T, dialoguer::Error>,
) -> Result<T, Box<dyn std::error::Error>> {
    match result {
        Ok(val) => Ok(val),
        Err(dialoguer::Error::IO(e)) if e.kind() == io::ErrorKind::Interrupted => {
            eprint!("\x1b[?25h");
            std::process::exit(130);
        }
        Err(e) => Err(Box::new(e)),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub provider: ProviderConfig,
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
    let contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let toml_string = toml::to_string_pretty(config)?;
    fs::write(&config_path, toml_string)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&config_path, permissions)?;
    }

    Ok(())
}

pub fn interactive_setup() -> Result<(), Box<dyn std::error::Error>> {
    let providers = vec!["Gemini API", "Ollama", "OpenAI Compatible"];
    let selection = handle_interrupt(
        Select::new()
            .with_prompt("Select API Provider")
            .items(&providers)
            .default(0)
            .interact(),
    )?;

    let config = match selection {
        0 => configure_gemini()?,
        1 => configure_ollama()?,
        2 => configure_openai()?,
        _ => unreachable!(),
    };

    save_config(&config)?;

    eprintln!("{}", "✓ Configuration saved!".green().bold());
    eprintln!();
    eprintln!("Provider: {}", providers[selection]);

    match &config.provider.config {
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

fn configure_gemini() -> Result<Config, Box<dyn std::error::Error>> {
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

    Ok(Config {
        provider: ProviderConfig {
            provider_type: "gemini".to_string(),
            config: ProviderSpecificConfig::Gemini {
                gemini: GeminiConfig { api_key, model },
            },
        },
    })
}

fn configure_ollama() -> Result<Config, Box<dyn std::error::Error>> {
    let base_url: String = handle_interrupt(
        Input::new()
            .with_prompt("Enter Ollama base URL")
            .default("http://localhost:11434".to_string())
            .interact_text(),
    )?;

    let model: String =
        handle_interrupt(Input::new().with_prompt("Enter model name").interact_text())?;

    Ok(Config {
        provider: ProviderConfig {
            provider_type: "ollama".to_string(),
            config: ProviderSpecificConfig::Ollama {
                ollama: OllamaConfig { base_url, model },
            },
        },
    })
}

fn configure_openai() -> Result<Config, Box<dyn std::error::Error>> {
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

    let model: String = Input::new()
        .with_prompt("Enter model name")
        .interact_text()?;

    Ok(Config {
        provider: ProviderConfig {
            provider_type: "openai".to_string(),
            config: ProviderSpecificConfig::OpenAI {
                openai: OpenAIConfig {
                    base_url,
                    api_key,
                    model,
                },
            },
        },
    })
}
