use colored::*;
use dialoguer::{Confirm, Input, Select};
use std::env;
use std::io::{self, IsTerminal, Read, stdin};
use std::path::PathBuf;
use std::process::Command;

use crate::common::handle_interrupt;

// ====================
// ascii symbols
// ====================

const SYMBOL_CHECK: &str = "✓";
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

// ====================
// cli argument parsing
// ====================

#[derive(Debug)]
pub struct CliArgs {
    pub command: Vec<String>,
    pub subcommand: Option<Subcommands>,
}

#[derive(Debug)]
pub enum Subcommands {
    Api,
    Uninstall,
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
    }

    let cli = Cli::parse();

    let subcommand = match cli.subcommand {
        Some(Commands::Api) => Some(Subcommands::Api),
        Some(Commands::Uninstall) => Some(Subcommands::Uninstall),
        None => None,
    };

    Ok(CliArgs {
        command: cli.command,
        subcommand,
    })
}

// ====================
// command execution
// ====================

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
        "export" => {
            if parts.len() > 1 {
                for part in &parts[1..] {
                    if let Some((key, value)) = part.split_once('=') {
                        unsafe {
                            env::set_var(key, value);
                        }
                    }
                }
            }
        }
        "unset" => {
            for var in &parts[1..] {
                unsafe {
                    env::remove_var(var);
                }
            }
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

// ====================
// interactive input
// ====================

pub fn read_single_key() -> Result<bool, io::Error> {
    use libc::{ECHO, ICANON, ISIG, STDIN_FILENO, TCSANOW, VMIN, VTIME, tcgetattr, tcsetattr};

    let mut termios = unsafe {
        let mut termios = std::mem::zeroed();
        if tcgetattr(STDIN_FILENO, &mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }
        termios
    };

    let original_termios = termios;
    termios.c_lflag &= !(ICANON | ECHO | ISIG);
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    unsafe {
        if tcsetattr(STDIN_FILENO, TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    let result = loop {
        let mut input: [u8; 1] = [0];
        match stdin().read(&mut input) {
            Ok(0) => break Err(io::Error::new(io::ErrorKind::UnexpectedEof, "eof")),
            Ok(_) => {
                match input[0] {
                    b'\n' | b'\r' => break Ok(true), // enter
                    3 => break Ok(false),            // ctrl+c - cancel
                    _ => continue,                   // ignore all other input
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => break Ok(false),
            Err(e) => break Err(e),
        }
    };

    unsafe {
        tcsetattr(STDIN_FILENO, TCSANOW, &original_termios);
    }

    result
}

pub fn is_interactive_terminal() -> bool {
    stdin().is_terminal()
}

// ====================
// user prompts
// ====================

pub fn prompt_select(
    prompt: &str,
    items: &[String],
    default: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    handle_interrupt(
        Select::new()
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact(),
    )
}

pub fn prompt_input(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    handle_interrupt(Input::new().with_prompt(prompt).interact_text())
}

pub fn prompt_input_with_default(
    prompt: &str,
    default: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    handle_interrupt(
        Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text(),
    )
}

pub fn prompt_input_allow_empty(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    handle_interrupt(
        Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text(),
    )
}

pub fn prompt_confirm(prompt: &str, default: bool) -> Result<bool, Box<dyn std::error::Error>> {
    handle_interrupt(
        Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact(),
    )
}

// ====================
// file path helpers
// ====================

pub fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}
