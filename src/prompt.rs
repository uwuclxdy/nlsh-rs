use std::env;

pub fn create_system_prompt(user_request: &str) -> String {
    let cwd = get_current_directory();
    let os = get_os();

    format!(
        "You are a shell command translator. Convert the user's request into a shell command for {}.

Current directory: {}
Current OS: {}

Rules:
- Output ONLY the command, nothing else
- No explanations, no markdown, no backticks
- If unclear, make a reasonable assumption
- Prefer simple, common commands
- Use appropriate shell syntax for {}

User request: {}",
        os, cwd, os, os, user_request
    )
}

pub fn get_current_directory() -> String {
    env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string())
}

pub fn get_os() -> String {
    if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else {
        "Unix".to_string()
    }
}

pub fn clean_response(response: &str) -> String {
    let mut cleaned = response.trim();

    if cleaned.starts_with("```") {
        cleaned = cleaned.trim_start_matches("```");
        cleaned = cleaned
            .trim_start_matches("shell")
            .trim_start_matches("bash")
            .trim_start_matches("zsh")
            .trim_start_matches("sh");
        cleaned = cleaned.trim_end_matches("```");
    }

    cleaned.trim().to_string()
}
