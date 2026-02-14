use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::{Config, MultiProviderConfig};

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

struct V1ToV2Migrator;

impl Migrator for V1ToV2Migrator {
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
    vec![Box::new(V1ToV2Migrator)]
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
