use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::prompt::DEFAULT_EXPLAIN_PROMPT;

use super::{Config, MultiProviderConfig, get_explain_prompt_path};

#[derive(Debug, Serialize, Deserialize)]
struct V1ProviderSection {
    #[serde(rename = "type")]
    provider_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct V1Config {
    provider: V1ProviderSection,
    #[serde(default)]
    providers: MultiProviderConfig,
}

type MigrationResult = Result<String, Box<dyn std::error::Error>>;

trait Migrator {
    fn can_migrate(&self, content: &str) -> bool;
    fn migrate(&self, content: &str) -> MigrationResult;
}

struct ConfigMigrator;

impl Migrator for ConfigMigrator {
    fn can_migrate(&self, content: &str) -> bool {
        content.contains("[provider]") && content.contains("type = ")
    }

    fn migrate(&self, content: &str) -> MigrationResult {
        let old_config: V1Config = toml::from_str(content)?;

        let new_config = Config {
            active_provider: old_config.provider.provider_type,
            providers: old_config.providers,
        };

        let new_content = toml::to_string_pretty(&new_config)?;
        Ok(new_content)
    }
}

fn get_migrators() -> Vec<Box<dyn Migrator>> {
    vec![Box::new(ConfigMigrator)]
}

const OLD_EXPLAIN_PROMPT_V1: &str = "You are a concise command-line assistant. Your task is to explain the given shell command in a single simple sentence, focusing on the main purpose and key flags.

The command to explain is: {command}

Formatting rules:
- Always start the response with a single safety emoji: ✅ (safe), ⚠️  (risky), or ❌ (dangerous).
- No other emojis are allowed.
- For ✅ commands: Output ONLY the emoji and the explanation.
- For ⚠️ or ❌ commands: Output the emoji, the explanation, and a short warning about the danger.
- Do NOT include the command in your response.
- Do NOT include any markdown, backticks, or code formatting.
- You may emphasise important words with ONLY the following html tags: `<b></b>`, `<i></i>`, `<u></u>`. No other formatting allowed.

Examples:
Input command: ls -la
Output: ✅ Lists all files and directories in the current folder, including hidden ones, in a detailed format.

Input command: rm -rf /
Output: ❌ Forcefully and recursively <b>deletes all files</b> and directories starting from the root. <b>Warning:</b> This will completely destroy your operating system.";

struct ExplainPromptMigrator;

impl Migrator for ExplainPromptMigrator {
    fn can_migrate(&self, content: &str) -> bool {
        !OLD_EXPLAIN_PROMPT_V1.is_empty() && content == OLD_EXPLAIN_PROMPT_V1
    }

    fn migrate(&self, _content: &str) -> MigrationResult {
        Ok(DEFAULT_EXPLAIN_PROMPT.to_string())
    }
}

fn get_explain_prompt_migrators() -> Vec<Box<dyn Migrator>> {
    vec![Box::new(ExplainPromptMigrator)]
}

pub fn migrate_explain_prompt() -> Result<bool, Box<dyn std::error::Error>> {
    let explain_prompt_path = get_explain_prompt_path();

    if !explain_prompt_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&explain_prompt_path)?;
    let migrators = get_explain_prompt_migrators();

    for migrator in migrators {
        if migrator.can_migrate(&content) {
            let new_content = migrator.migrate(&content)?;
            fs::write(&explain_prompt_path, new_content)?;
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn migrate_config(config_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(config_path)?;
    let migrators = get_migrators();

    for migrator in migrators {
        if migrator.can_migrate(&content) {
            let new_content = migrator.migrate(&content)?;
            fs::write(config_path, new_content)?;
            return Ok(true);
        }
    }

    Ok(false)
}
