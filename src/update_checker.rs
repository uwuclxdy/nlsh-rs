use colored::*;
use serde::Deserialize;
use std::time::Duration;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const TIMEOUT_SECS: u64 = 2;

#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
}

#[derive(Deserialize)]
struct CrateInfo {
    max_version: String,
}

pub async fn check_for_updates() {
    tokio::spawn(async {
        if let Err(e) = check_update_internal().await {
            log::trace!("update check failed: {}", e);
        }
    });
}

async fn check_update_internal() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://crates.io/api/v1/crates/nlsh-rs";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()?;

    let response = client
        .get(url)
        .header("User-Agent", format!("nlsh-rs/{}", CURRENT_VERSION))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    let crate_data: CrateResponse = response.json().await?;
    let latest_version = crate_data.crate_info.max_version;

    if latest_version != CURRENT_VERSION {
        eprintln!(
            "{} {}",
            "update found, please run".dimmed(),
            "`cargo update nlsh-rs`".to_string().cyan()
        );
    }

    Ok(())
}
