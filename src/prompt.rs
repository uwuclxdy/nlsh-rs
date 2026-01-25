use std::env;
use std::fs;
use std::process::Command;

pub fn create_system_prompt(user_request: &str) -> String {
    let cwd = get_current_directory();
    let os = get_os();
    let shell = get_shell();
    let home = get_home_directory();
    let user = get_username();

    format!(
        "You are a shell command translator. Convert the user's request into a shell command for {}.

Environment context:
- Current dir: {}
- Home dir: {}
- User: {}
- OS: {}
- Shell: {}

Rules:
- Output ONLY the command, nothing else
- No explanations, no markdown, no backticks
- If unclear, make a reasonable assumption
- Prefer simple, common commands
- Use appropriate shell syntax for {} ({})
- Consider the current directory context when generating paths
- Use ~ for home directory when appropriate

User request: {}",
        os, cwd, home, user, os, shell, os, shell, user_request
    )
}

pub fn get_current_directory() -> String {
    env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string())
}

pub fn get_os() -> String {
    if cfg!(target_os = "linux") {
        get_linux_info()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else {
        "Unix".to_string()
    }
}

fn get_linux_info() -> String {
    let distro = get_linux_distro();
    let kernel = get_kernel_version();

    format!("linux ({}; kernel: {})", distro, kernel)
}

fn get_linux_distro() -> String {
    if let Ok(contents) = fs::read_to_string("/etc/os-release") {
        let mut name = None;
        let mut version = None;

        for line in contents.lines() {
            if let Some(value) = line.strip_prefix("NAME=") {
                name = Some(value.trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
                version = Some(value.trim_matches('"').to_string());
            }
        }

        match (name, version) {
            (Some(n), Some(v)) => format!("{} {}", n, v),
            (Some(n), None) => n,
            _ => "linux".to_string(),
        }
    } else {
        "linux".to_string()
    }
}

fn get_kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .or_else(|| fs::read_to_string("/proc/sys/kernel/osrelease").ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn get_shell() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.split('/').next_back().map(|s| s.to_string()))
        .unwrap_or_else(|| "sh".to_string())
}

pub fn get_home_directory() -> String {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".to_string())
}

pub fn get_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
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
