use colored::*;
use inquire::{Select, Text};
use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

const SYMBOL_CHECK: &str = "\u{2713}";
const SYMBOL_ERROR: &str = "error:";
const SYMBOL_WARNING: &str = "warning:";

pub fn print_check_with_message(message: &str) {
    eprintln!("{} {}", SYMBOL_CHECK.green(), message);
}

pub fn print_check_with_bold_message(message: &str) {
    eprintln!("{} {}", SYMBOL_CHECK.green(), message.bold());
}

pub fn print_error_with_message(message: &str) {
    eprintln!("{} {}", SYMBOL_ERROR.red().bold(), message);
}

pub fn print_warning_with_message(message: &str) {
    eprintln!("{} {}", SYMBOL_WARNING.yellow(), message);
}

#[derive(Debug)]
pub struct CliArgs {
    pub command: Vec<String>,
    pub subcommand: Option<Subcommands>,
}

#[derive(Debug)]
pub enum Subcommands {
    Api,
    Uninstall,
    Prompt {
        kind: PromptKind,
        action: PromptAction,
    },
    Explain {
        command: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum PromptKind {
    System,
    Explain,
}

#[derive(Debug, Clone)]
pub enum PromptAction {
    Show,
    Edit,
}

pub fn parse_cli_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    use clap::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(name = "nlsh-rs")]
    #[command(version)]
    #[command(disable_help_subcommand = true)]
    struct Cli {
        #[arg(value_name = "COMMAND")]
        command: Vec<String>,

        #[command(subcommand)]
        subcommand: Option<Commands>,
    }

    #[derive(Subcommand)]
    enum Commands {
        Api,
        Uninstall,
        Prompt {
            #[arg(value_enum, default_value_t = ClapPromptKind::System)]
            kind: ClapPromptKind,
            #[arg(value_enum, default_value_t = ClapPromptAction::Show)]
            action: ClapPromptAction,
        },
        Explain {
            command: Vec<String>,
        },
    }

    #[derive(clap::ValueEnum, Clone)]
    enum ClapPromptKind {
        System,
        Explain,
    }

    #[derive(clap::ValueEnum, Clone)]
    enum ClapPromptAction {
        Show,
        Edit,
    }

    let cli = Cli::parse();

    let subcommand = match cli.subcommand {
        Some(Commands::Api) => Some(Subcommands::Api),
        Some(Commands::Uninstall) => Some(Subcommands::Uninstall),
        Some(Commands::Prompt { kind, action }) => Some(Subcommands::Prompt {
            kind: match kind {
                ClapPromptKind::System => PromptKind::System,
                ClapPromptKind::Explain => PromptKind::Explain,
            },
            action: match action {
                ClapPromptAction::Show => PromptAction::Show,
                ClapPromptAction::Edit => PromptAction::Edit,
            },
        }),
        Some(Commands::Explain { command }) => Some(Subcommands::Explain { command }),
        None => None,
    };

    Ok(CliArgs {
        command: cli.command,
        subcommand,
    })
}

pub fn execute_shell_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = command.trim();

    if trimmed.is_empty() {
        return Ok(());
    }

    // expand environment variables in the entire command
    let expanded = shellexpand::tilde(trimmed);
    let expanded = shellexpand::env(expanded.as_ref()).unwrap_or_else(|_| expanded.clone());

    if expanded.contains('\n') {
        Command::new("sh")
            .arg("-c")
            .arg(expanded.as_ref())
            .current_dir(env::current_dir()?)
            .status()?;
        return Ok(());
    }

    let parts: Vec<&str> = expanded.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(());
    }

    match parts[0] {
        "cd" => {
            let path = if parts.len() > 1 { parts[1] } else { "" };

            let target_dir = if path.is_empty() {
                PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/".to_string()))
            } else {
                PathBuf::from(path)
            };

            env::set_current_dir(&target_dir)?;
        }
        _ => {
            Command::new("sh")
                .arg("-c")
                .arg(expanded.as_ref())
                .current_dir(env::current_dir()?)
                .status()?;
        }
    }

    Ok(())
}

pub fn is_interactive_terminal() -> bool {
    if std::env::var("NLSH_FORCE_INTERACTIVE").is_ok() {
        return true;
    }
    std::io::stdin().is_terminal()
}

pub fn prompt_select(
    prompt: &str,
    items: &[String],
    default: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let selection = Select::new(prompt, items.to_vec())
        .with_starting_cursor(default)
        .prompt()?;
    Ok(items
        .iter()
        .position(|x| x == &selection)
        .unwrap_or(default))
}

pub fn prompt_input(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Text::new(prompt).prompt()?)
}

pub fn prompt_input_with_default(
    prompt: &str,
    default: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Text::new(prompt).with_default(default).prompt()?)
}

pub fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}
