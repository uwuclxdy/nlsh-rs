use crate::common::{get_current_directory, get_os, get_shell, get_username};
use crate::config::{
    get_explain_prompt_path, get_sys_prompt_path, save_explain_prompt, save_sys_prompt,
};

pub const DEFAULT_PROMPT_TEMPLATE: &str =
    "You are a shell command translator. Convert the user's request into a shell command for {os}.

Environment context:
- Current dir: {cwd}
- Home dir: {home}
- User: {user}
- Shell: {shell}

Rules:
- Output ONLY the command, nothing else
- No explanations, no markdown, no backticks
- If unclear, make a reasonable assumption
- Prefer simple, common commands
- Use appropriate shell syntax and commands for this environment
- Consider the current directory context when generating paths
- Use ~ for home directory when appropriate

User request: {request}";

pub fn create_system_prompt(user_request: &str, template: Option<&str>) -> String {
    let cwd = get_current_directory();
    let os = get_os();
    let shell = get_shell();
    let home = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~".to_string());
    let user = get_username();

    let tmpl = template.unwrap_or(DEFAULT_PROMPT_TEMPLATE);

    tmpl.replace("{os}", &os)
        .replace("{cwd}", &cwd)
        .replace("{home}", &home)
        .replace("{user}", &user)
        .replace("{shell}", &shell)
        .replace("{request}", user_request)
}

pub const DEFAULT_EXPLAIN_PROMPT: &str =
    "You are a concise command-line assistant. Your task is to explain the given shell command in a single simple sentence, focusing on the main purpose and key flags.

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

pub fn create_explain_prompt(command: &str, template: Option<&str>) -> String {
    let tmpl = template.unwrap_or(DEFAULT_EXPLAIN_PROMPT);
    tmpl.replace("{command}", command)
}

pub fn validate_sys_prompt(template: &str) -> bool {
    template.contains("{request}")
}

pub fn validate_explain_prompt(template: &str) -> bool {
    template.contains("{command}")
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

pub fn clean_explanation(response: &str, command: &str) -> String {
    let trimmed = response.trim();
    let cmd_trimmed = command.trim();

    // Remove leading command if present
    if trimmed.starts_with(cmd_trimmed) {
        let after = &trimmed[cmd_trimmed.len()..];
        if after.starts_with('\n') || after.starts_with(' ') || after.is_empty() {
            after.trim_start().to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    }
}

pub fn create_prompts() -> Result<(), std::io::Error> {
    let path = get_sys_prompt_path();
    if !path.exists() {
        save_sys_prompt(DEFAULT_PROMPT_TEMPLATE)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }

    let path = get_explain_prompt_path();
    if !path.exists() {
        save_explain_prompt(DEFAULT_EXPLAIN_PROMPT)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_explain_prompt_has_command_placeholder() {
        assert!(DEFAULT_EXPLAIN_PROMPT.contains("{command}"));
    }

    #[test]
    fn validate_explain_prompt_accepts_valid_template() {
        assert!(validate_explain_prompt("explain: {command}"));
    }

    #[test]
    fn validate_explain_prompt_rejects_missing_placeholder() {
        assert!(!validate_explain_prompt("explain this command"));
    }

    #[test]
    fn validate_sys_prompt_accepts_valid_template() {
        assert!(validate_sys_prompt("do this: {request}"));
    }

    #[test]
    fn validate_sys_prompt_rejects_missing_placeholder() {
        assert!(!validate_sys_prompt("do something"));
    }

    #[test]
    fn create_explain_prompt_substitutes_command_in_default() {
        let result = create_explain_prompt("echo hi", None);
        assert!(result.contains("echo hi"));
        assert!(!result.contains("{command}"));
    }

    #[test]
    fn create_explain_prompt_substitutes_command_in_custom_template() {
        let result = create_explain_prompt("ls -la", Some("run: {command}"));
        assert_eq!(result, "run: ls -la");
    }

    #[test]
    fn create_explain_prompt_handles_multiword_command() {
        let result = create_explain_prompt("git log --oneline", Some("{command}"));
        assert_eq!(result, "git log --oneline");
    }

    #[test]
    fn clean_explanation_removes_leading_command() {
        let result = clean_explanation("free -h\nShows memory usage.", "free -h");
        assert_eq!(result, "Shows memory usage.");
    }

    #[test]
    fn clean_explanation_leaves_unrelated_response() {
        let result = clean_explanation("Shows memory usage.", "free -h");
        assert_eq!(result, "Shows memory usage.");
    }

    #[test]
    fn clean_explanation_handles_command_with_space() {
        let result = clean_explanation("free -h Shows memory usage.", "free -h");
        assert_eq!(result, "Shows memory usage.");
    }
}
