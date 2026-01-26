mod cli;
mod common;
mod config;
mod config_migration;
mod confirmation;
mod error;
mod interactive;
mod prompt;
mod providers;
mod shell_integration;
mod uninstall;
use colored::*;
use common::{exit_with_code, setup_interrupt_handler, setup_terminal};
use confirmation::{display_command, display_error};
use error::NlshError;
use interactive::get_user_input;
use std::io::{self, IsTerminal};
use tokio_util::sync::CancellationToken;

fn get_model_name(config: &config::Config) -> String {
    let provider = config.get_provider_config();
    match &provider.config {
        config::ProviderSpecificConfig::Gemini { gemini } => gemini.model.clone(),
        config::ProviderSpecificConfig::Ollama { ollama } => ollama.model.clone(),
        config::ProviderSpecificConfig::OpenAI { openai } => openai.model.clone(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_terminal();
    setup_interrupt_handler();

    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    match shell_integration::auto_setup_shell_function() {
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

    let cli = cli::parse_cli_args()?;

    if let Some(command) = cli.subcommand {
        match command {
            cli::Subcommands::Api => {
                config::interactive_setup()?;
                return Ok(());
            }
            cli::Subcommands::Uninstall => {
                uninstall::uninstall_nlsh()?;
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
            exit_with_code(1);
        }
    };

    let provider = match providers::create_provider(&config) {
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
    config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = get_model_name(config);
    eprint!(
        "{}",
        format!("using {}...", model_name).truecolor(128, 128, 128)
    );
    let _ = io::Write::flush(&mut io::stderr());

    let prompt = prompt::create_system_prompt(user_input);

    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();

    // spawn task to listen for Ctrl+C during request
    let ctrl_c_task = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let response = tokio::select! {
        result = provider.generate(&prompt) => {
            ctrl_c_task.abort();
            match result {
                Ok(res) => res,
                Err(e) => {
                    eprint!("\r{}\r", " ".repeat(50));
                    let _ = io::Write::flush(&mut io::stderr());
                    return Err(Box::new(e));
                }
            }
        }
        _ = cancel_token.cancelled() => {
            eprint!("\r{}\r", " ".repeat(50));
            let _ = io::Write::flush(&mut io::stderr());
            return Err("request cancelled".into());
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

    let display_lines = display_command(&command);

    let confirmed = confirmation::confirm_execution(display_lines)?;
    if !confirmed {
        return Ok(());
    }

    cli::execute_shell_command(&command)?;

    Ok(())
}

async fn process_command_single(
    user_input: &str,
    provider: &dyn providers::AIProvider,
    config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = get_model_name(config);
    eprint!(
        "{}",
        format!("using {}...", model_name).truecolor(128, 128, 128)
    );
    let _ = io::Write::flush(&mut io::stderr());

    let prompt = prompt::create_system_prompt(user_input);

    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();

    // spawn task to listen for Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
        eprintln!();
        exit_with_code(130);
    });

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

    let display_lines = display_command(&command);

    let confirmed = confirmation::confirm_execution(display_lines)?;
    if !confirmed {
        return Ok(());
    }

    // in single-command mode, print for shell wrapper to execute
    println!("{}", command);

    Ok(())
}
