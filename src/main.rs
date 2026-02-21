mod cli;
mod common;
mod config;
mod confirmation;
mod error;
mod interactive;
mod prompt;
mod providers;
mod shell_integration;
mod uninstall;

use std::io::IsTerminal;
use tokio_util::sync::CancellationToken;

use cli::{execute_shell_command, parse_cli_args};
use colored::*;
#[cfg(unix)]
use common::setup_terminal;
use common::{clear_line_with_spaces, eprint_flush, exit_with_code};
use config::{Config, ProviderSpecificConfig, interactive_setup, load_config};
use confirmation::{confirm_execution, display_command, display_error};
use error::NlshError;
use interactive::get_user_input;
use prompt::{DEFAULT_PROMPT_TEMPLATE, clean_response, create_system_prompt, validate_sys_prompt};
use providers::create_provider;
use shell_integration::auto_setup_shell_function;
use uninstall::uninstall_nlsh;

fn get_model_name(config: &Config) -> String {
    let provider = config.get_provider_config();
    match &provider.config {
        ProviderSpecificConfig::Gemini { gemini } => gemini.model.clone(),
        ProviderSpecificConfig::Ollama { ollama } => ollama.model.clone(),
        ProviderSpecificConfig::OpenAI { openai } => openai.model.clone(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    setup_terminal();

    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    match auto_setup_shell_function() {
        Ok(true) => {
            eprintln!(
                "{}",
                "restart shell or run 'source ~/.bashrc' ('source ~/.config/fish/config.fish' for fish).".yellow()
            );
            exit_with_code(0);
        }
        Ok(false) => {}
        Err(_) => {}
    }

    let cli = parse_cli_args()?;

    if let Some(command) = cli.subcommand {
        match command {
            cli::Subcommands::Api => {
                interactive_setup()?;
                return Ok(());
            }
            cli::Subcommands::Uninstall => {
                uninstall_nlsh()?;
                return Ok(());
            }
            cli::Subcommands::Prompt { action } => {
                match action {
                    cli::PromptAction::Show => {
                        let content = config::load_sys_prompt()
                            .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());
                        println!("{}", content);
                    }
                    cli::PromptAction::Edit => {
                        let path = config::get_sys_prompt_path();
                        if !path.exists() {
                            config::save_sys_prompt(DEFAULT_PROMPT_TEMPLATE)?;
                        }
                        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                        std::process::Command::new(&editor).arg(&path).status()?;
                        if let Some(saved) = config::load_sys_prompt()
                            && !validate_sys_prompt(&saved)
                        {
                            display_error("sys-prompt must contain the {request} placeholder.");
                        }
                    }
                }
                return Ok(());
            }
        }
    }

    let config = match load_config() {
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
            exit_with_code(1);
        }
    };

    let provider = match create_provider(&config) {
        Ok(p) => p,
        Err(e) => {
            display_error(&e.to_string());
            exit_with_code(1);
        }
    };

    let interactive_mode = cli.command.is_empty();

    if interactive_mode {
        // interactive mode: keep running until ctrl+c at prompt
        loop {
            let user_input = match get_user_input()? {
                Some(input) => input,
                None => continue,
            };

            if let Err(e) =
                process_command_interactive(&user_input, provider.as_ref(), &config).await
                && !e.to_string().contains("cancelled")
            {
                display_error(&e.to_string());
            }
        }
    } else {
        // single-command mode: execute once and exit
        let user_input = cli.command.join(" ");
        process_command_single(&user_input, provider.as_ref(), &config).await?;
    }

    Ok(())
}

async fn process_command_interactive(
    user_input: &str,
    provider: &dyn providers::AIProvider,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = get_model_name(config);
    eprint_flush(&format!(
        "{}",
        format!("using {}...", model_name).truecolor(128, 128, 128)
    ));

    let sys_prompt = config::load_sys_prompt();
    if let Some(ref t) = sys_prompt
        && !validate_sys_prompt(t)
    {
        display_error("sys-prompt must contain the {request} placeholder — using default.");
    }
    let effective = sys_prompt.as_deref().filter(|t| validate_sys_prompt(t));
    let prompt = create_system_prompt(user_input, effective);

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    // spawn task to listen for Ctrl+C during request
    let ctrl_c_task = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_token_clone.cancel();
    });

    let response = tokio::select! {
        result = provider.generate(&prompt) => {
            ctrl_c_task.abort();
            match result {
                Ok(res) => res,
                Err(e) => {
                    clear_line_with_spaces(50);
                    return Err(Box::new(e));
                }
            }
        }
        _ = cancel_token.cancelled() => {
            clear_line_with_spaces(50);
            return Err("request cancelled".into());
        }
    };

    clear_line_with_spaces(50);

    let command = clean_response(&response);

    if command.trim().is_empty() {
        display_error("failed to generate a valid command.");
        eprintln!(
            "{}",
            "the AI returned an empty response. please try again.".yellow()
        );
        return Err("empty response".into());
    }

    let display_lines = display_command(&command);

    let confirmed = confirm_execution(display_lines)?;
    if !confirmed {
        return Ok(());
    }

    execute_shell_command(&command)?;

    Ok(())
}

async fn process_command_single(
    user_input: &str,
    provider: &dyn providers::AIProvider,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = get_model_name(config);
    eprint_flush(&format!(
        "{}",
        format!("using {}...", model_name).truecolor(128, 128, 128)
    ));

    let sys_prompt = config::load_sys_prompt();
    if let Some(ref t) = sys_prompt
        && !validate_sys_prompt(t)
    {
        display_error("sys-prompt must contain the {request} placeholder — using default.");
    }
    let effective = sys_prompt.as_deref().filter(|t| validate_sys_prompt(t));
    let prompt = create_system_prompt(user_input, effective);

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    // spawn task to listen for Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_token_clone.cancel();
        eprintln!();
        exit_with_code(130);
    });

    let response = match provider.generate(&prompt).await {
        Ok(res) => res,
        Err(e) => {
            clear_line_with_spaces(50);
            return Err(Box::new(e));
        }
    };

    clear_line_with_spaces(50);

    let command = clean_response(&response);

    if command.trim().is_empty() {
        display_error("failed to generate a valid command.");
        eprintln!(
            "{}",
            "the AI returned an empty response. please try again.".yellow()
        );
        return Err("empty response".into());
    }

    let display_lines = display_command(&command);

    let confirmed = confirm_execution(display_lines)?;
    if !confirmed {
        return Ok(());
    }

    // in single-command mode, print for shell wrapper to execute
    println!("{}", command);

    Ok(())
}
