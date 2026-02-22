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
use confirmation::{
    ConfirmResult, confirm_execution, confirm_with_explain, display_command, display_error,
    display_explanation,
};
use error::NlshError;
use interactive::get_user_input;
use prompt::{
    DEFAULT_EXPLAIN_PROMPT, DEFAULT_PROMPT_TEMPLATE, clean_response, create_explain_prompt,
    create_prompts, create_system_prompt, validate_explain_prompt, validate_sys_prompt,
};
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

    create_prompts();

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

    // handle subcommands that do not need a provider
    let needs_provider = matches!(cli.subcommand, Some(cli::Subcommands::Explain { .. }));
    if !needs_provider && let Some(ref command) = cli.subcommand {
        match command {
            cli::Subcommands::Api => {
                interactive_setup()?;
                return Ok(());
            }
            cli::Subcommands::Uninstall => {
                uninstall_nlsh()?;
                return Ok(());
            }
            cli::Subcommands::Prompt { kind, action } => {
                match (kind, action) {
                    (cli::PromptKind::System, cli::PromptAction::Show) => {
                        let content = config::load_sys_prompt()
                            .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());
                        println!("{}", content);
                    }
                    (cli::PromptKind::System, cli::PromptAction::Edit) => {
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
                    (cli::PromptKind::Explain, cli::PromptAction::Show) => {
                        let content = config::load_explain_prompt()
                            .unwrap_or_else(|| DEFAULT_EXPLAIN_PROMPT.to_string());
                        println!("{}", content);
                    }
                    (cli::PromptKind::Explain, cli::PromptAction::Edit) => {
                        let path = config::get_explain_prompt_path();
                        if !path.exists() {
                            config::save_explain_prompt(DEFAULT_EXPLAIN_PROMPT)?;
                        }
                        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                        std::process::Command::new(&editor).arg(&path).status()?;
                        if let Some(saved) = config::load_explain_prompt()
                            && !validate_explain_prompt(&saved)
                        {
                            display_error("explain-prompt must contain the {command} placeholder.");
                        }
                    }
                }
                return Ok(());
            }
            cli::Subcommands::Explain { .. } => unreachable!(),
        }
    }

    // move cli.subcommand to extract explain parts (non-explain paths already returned above)
    let explain_parts = match cli.subcommand {
        Some(cli::Subcommands::Explain { command }) => Some(command),
        _ => None,
    };

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

    // handle standalone explain subcommand
    if let Some(cmd_parts) = explain_parts {
        if cmd_parts.is_empty() {
            display_error("no command provided.");
            exit_with_code(1);
        }
        let command = cmd_parts.join(" ");
        let cmd_lines = display_command(&command);
        let explanation = get_explanation(&command, provider.as_ref())
            .await
            .unwrap_or_else(|_| String::new());
        if explanation.is_empty() {
            return Ok(());
        }
        let expl_lines = display_explanation(&explanation);
        let confirmed = confirm_execution(cmd_lines + expl_lines)?;
        if confirmed {
            println!("{}", command);
        }
        return Ok(());
    }

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

    let cmd_lines = display_command(&command);

    match confirm_with_explain(cmd_lines)? {
        ConfirmResult::Yes => {
            execute_shell_command(&command)?;
        }
        ConfirmResult::No => {}
        ConfirmResult::Explain => {
            let explanation = get_explanation(&command, provider).await?;
            let expl_lines = display_explanation(&explanation);
            let confirmed = confirm_execution(cmd_lines + expl_lines)?;
            if confirmed {
                execute_shell_command(&command)?;
            }
        }
    }

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

    let cmd_lines = display_command(&command);

    match confirm_with_explain(cmd_lines)? {
        ConfirmResult::Yes => {
            println!("{}", command);
        }
        ConfirmResult::No => {}
        ConfirmResult::Explain => {
            let explanation = get_explanation(&command, provider).await?;
            let expl_lines = display_explanation(&explanation);
            let confirmed = confirm_execution(cmd_lines + expl_lines)?;
            if confirmed {
                println!("{}", command);
            }
        }
    }

    Ok(())
}

async fn get_explanation(
    command: &str,
    provider: &dyn providers::AIProvider,
) -> Result<String, Box<dyn std::error::Error>> {
    let explain_tmpl = config::load_explain_prompt();
    if let Some(ref t) = explain_tmpl
        && !validate_explain_prompt(t)
    {
        display_error("explain-prompt must contain the {command} placeholder — using default.");
    }
    let effective = explain_tmpl
        .as_deref()
        .filter(|t| validate_explain_prompt(t));
    let query = create_explain_prompt(command, effective);

    eprint_flush(&format!("{}", "explaining...".truecolor(128, 128, 128)));

    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();

    let ctrl_c = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let result = tokio::select! {
        res = provider.generate(&query) => {
            ctrl_c.abort();
            match res {
                Ok(r) => r,
                Err(e) => {
                    clear_line_with_spaces(50);
                    return Err(Box::new(e));
                }
            }
        }
        _ = cancel_token.cancelled() => {
            clear_line_with_spaces(50);
            return Err("cancelled".into());
        }
    };

    clear_line_with_spaces(50);
    Ok(result.trim().to_string())
}
