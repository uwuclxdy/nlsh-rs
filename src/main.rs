mod config;
mod confirmation;
mod error;
mod interactive;
mod prompt;
mod providers;
mod shell_integration;

use clap::{Parser, Subcommand};
use colored::*;
use confirmation::{confirm_execution, display_command, display_error};
use dialoguer::Confirm;
use error::NlshError;
use interactive::get_user_input;
use std::io::{self, IsTerminal};
use std::process::Command;

#[cfg(unix)]
fn setup_terminal() {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(std::io::stdin().as_raw_fd(), &mut termios) == 0 {
            termios.c_lflag &= !libc::ECHOCTL;
            libc::tcsetattr(std::io::stdin().as_raw_fd(), libc::TCSANOW, &termios);
        }
    }
}

#[cfg(not(unix))]
fn setup_terminal() {
    // no-op on non-unix systems
}

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

fn get_model_name(config: &config::Config) -> String {
    match &config.provider.config {
        config::ProviderSpecificConfig::Gemini { gemini } => gemini.model.clone(),
        config::ProviderSpecificConfig::Ollama { ollama } => ollama.model.clone(),
        config::ProviderSpecificConfig::OpenAI { openai } => openai.model.clone(),
    }
}

#[derive(Parser)]
#[command(name = "nlsh-rs")]
#[command(version)]
#[command(disable_help_subcommand = true)]
struct Cli {
    /// natural language command to translate (optional - if omitted, enters interactive mode)
    #[arg(value_name = "COMMAND")]
    command: Vec<String>,

    #[command(subcommand)]
    subcommand: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// configure API provider (Gemini, Ollama, LM Studio, OpenAI)
    Api,
    /// uninstall nlsh-rs
    Uninstall,
}

fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{}", "uninstalling nlsh-rs...".yellow().bold());
    eprintln!();

    match shell_integration::remove_shell_integration() {
        Ok(true) => {
            eprintln!("{}", "✓ removed shell integration".green());
        }
        Ok(false) => {
            eprintln!("{}", "  no shell integration found".dimmed());
        }
        Err(e) => {
            eprintln!(
                "{} failed to remove shell integration: {}",
                "warning:".yellow(),
                e
            );
        }
    }

    eprintln!("{}", "  uninstalling cargo crate...".dimmed());
    let output = Command::new("cargo")
        .args(["uninstall", "nlsh-rs"])
        .output()?;

    if output.status.success() {
        eprintln!("{}", "✓ uninstalled cargo crate".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("package 'nlsh-rs' is not installed") || stderr.contains("not installed")
        {
            eprintln!("{}", "  cargo crate not installed".dimmed());
        } else {
            eprintln!(
                "{} failed to uninstall: {}",
                "warning:".yellow(),
                stderr.trim()
            );
        }
    }

    eprintln!();
    eprint!("\x1b[?25h");
    let _ = io::Write::flush(&mut io::stderr());
    let remove_config = handle_interrupt(
        Confirm::new()
            .with_prompt("remove configuration?")
            .default(false)
            .interact(),
    )?;

    if remove_config {
        let config_dir = dirs::config_dir()
            .ok_or("failed to get config directory")?
            .join("nlsh-rs");

        if config_dir.exists() {
            std::fs::remove_dir_all(&config_dir)?;
            eprintln!("{}", "✓ removed configuration".green());
        } else {
            eprintln!("{}", "  no configuration found".dimmed());
        }
    }

    let current_dir = std::env::current_dir()?;
    let cargo_toml = current_dir.join("Cargo.toml");

    if cargo_toml.exists() {
        let contents = std::fs::read_to_string(&cargo_toml)?;
        if contents.contains("name = \"nlsh-rs\"") {
            eprintln!();
            eprint!("\x1b[?25h"); // show cursor
            let _ = io::Write::flush(&mut io::stderr());
            let remove_repo = handle_interrupt(
                Confirm::new()
                    .with_prompt("remove current directory (nlsh-rs repository)?")
                    .default(false)
                    .interact(),
            )?;

            if remove_repo {
                eprintln!("{}", "  removing directory...".dimmed());
                let parent = current_dir.parent().ok_or("cannot remove root directory")?;
                std::env::set_current_dir(parent)?;

                std::fs::remove_dir_all(&current_dir)?;
                eprintln!("{}", "✓ removed nlsh-rs repository".green());
            }
        }
    }

    eprintln!();
    eprintln!("{}", "nlsh-rs uninstalled successfully!".green().bold());
    eprintln!("{}", "please restart your shell or run 'source ~/.bashrc' (or 'source ~/.config/fish/config.fish' for fish).".yellow());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_terminal();

    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    tokio::spawn(async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
        eprintln!();
        std::process::exit(0);
    });

    match shell_integration::auto_setup_shell_function() {
        Ok(true) => {
            eprintln!(
                "{}",
                "restart shell or run 'source ~/.bashrc' ('source ~/.config/fish/config.fish' for fish).".yellow()
            );
            std::process::exit(0);
        }
        Ok(false) => {}
        Err(_) => {}
    }

    let cli = Cli::parse();

    if let Some(command) = cli.subcommand {
        match command {
            Commands::Api => {
                config::interactive_setup()?;
                return Ok(());
            }
            Commands::Uninstall => {
                uninstall()?;
                return Ok(());
            }
        }
    }

    let config = match config::load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            if e.to_string().contains("No such file") {
                display_error("no API provider configured.");
                eprintln!(
                    "{}",
                    "run 'nlsh-rs api' to set up your preferred provider.".cyan()
                );
            } else {
                let err = NlshError::ConfigError(e.to_string());
                display_error(&err.to_string());
            }
            std::process::exit(1);
        }
    };

    let provider = match providers::create_provider(&config) {
        Ok(p) => p,
        Err(e) => {
            display_error(&e.to_string());
            std::process::exit(1);
        }
    };

    let interactive_mode = cli.command.is_empty();

    if interactive_mode {
        // interactive mode: keep running until ctrl+c
        loop {
            let user_input = match get_user_input()? {
                Some(input) => input,
                None => continue,
            };

            if let Err(e) = process_command(&user_input, provider.as_ref(), &config, true).await {
                display_error(&e.to_string());
            }
        }
    } else {
        // single-command mode: execute once and exit
        let user_input = cli.command.join(" ");
        process_command(&user_input, provider.as_ref(), &config, false).await?;
    }

    Ok(())
}

async fn process_command(
    user_input: &str,
    provider: &dyn providers::AIProvider,
    config: &config::Config,
    is_interactive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = get_model_name(config);
    eprint!(
        "{}",
        format!("using {}...", model_name).truecolor(128, 128, 128)
    );
    let _ = io::Write::flush(&mut io::stderr());

    let prompt = prompt::create_system_prompt(user_input);

    let response = match provider.generate(&prompt).await {
        Ok(res) => res,
        Err(e) => {
            eprint!("\r{}\r", " ".repeat(50));
            let _ = io::Write::flush(&mut io::stderr());
            return Err(Box::new(e));
        }
    };

    eprint!("\r{}\r", " ".repeat(50));
    let _ = io::Write::flush(&mut io::stderr());

    let command = prompt::clean_response(&response);

    if command.trim().is_empty() {
        display_error("failed to generate a valid command.");
        eprintln!(
            "{}",
            "the AI returned an empty response. please try again.".yellow()
        );
        return Err("empty response".into());
    }

    display_command(&command);

    let confirmed = confirm_execution()?;
    if !confirmed {
        return Ok(());
    }

    if is_interactive {
        execute_interactive_command(&command)?;
    } else {
        // in single-command mode, print for shell wrapper to execute
        println!("{}", command);
    }

    Ok(())
}

fn execute_interactive_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = command.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(());
    }

    // handle shell built-ins that affect process state
    match parts[0] {
        "cd" => {
            let path = if parts.len() > 1 { parts[1] } else { "" };

            let target_dir = if path.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
            } else if path == "~" {
                std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
            } else if let Some(rest) = path.strip_prefix("~/") {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
                format!("{}/{}", home, rest)
            } else {
                path.to_string()
            };

            std::env::set_current_dir(&target_dir)?;
        }
        "export" => {
            // handle export VAR=value
            if parts.len() > 1 {
                for part in &parts[1..] {
                    if let Some((key, value)) = part.split_once('=') {
                        unsafe {
                            std::env::set_var(key, value);
                        }
                    }
                }
            }
        }
        "unset" => {
            // handle unset VAR
            for var in &parts[1..] {
                unsafe {
                    std::env::remove_var(var);
                }
            }
        }
        _ => {
            // execute external commands in subprocess with inherited state
            let status = Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(std::env::current_dir()?)
                .status()?;

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }

    Ok(())
}
