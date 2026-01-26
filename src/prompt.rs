use crate::common::{get_current_directory, get_os, get_shell, get_username};

pub fn create_system_prompt(user_request: &str) -> String {
    let cwd = get_current_directory();
    let os = get_os();
    let shell = get_shell();
    let home = dirs::home_dir();
    let user = get_username();

    format!(
        "You are a shell command translator. Convert the user's request into a shell command for {}.

Environment context:
- Current dir: {}
- Home dir: {:?}
- User: {}
- Shell: {}

Rules:
- Output ONLY the command, nothing else
- No explanations, no markdown, no backticks
- If unclear, make a reasonable assumption
- Prefer simple, common commands
- Use appropriate shell syntax and commands for this environment
- Consider the current directory context when generating paths
- Use ~ for home directory when appropriate

User request: {}",
        os, cwd, home, user, shell, user_request
    )
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
