use crate::common::show_cursor;
use colored::*;
use dialoguer::Confirm;
use std::fs;
use std::io;
use std::process::Command;

pub fn uninstall_nlsh() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{}", "uninstalling nlsh-rs...".yellow().bold());
    eprintln!();

    match crate::shell_integration::remove_shell_integration() {
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
    show_cursor();
    let _ = io::Write::flush(&mut io::stderr());
    let remove_config = crate::common::handle_interrupt(
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
            fs::remove_dir_all(&config_dir)?;
            eprintln!("{}", "✓ removed configuration".green());
        } else {
            eprintln!("{}", "  no configuration found".dimmed());
        }
    }

    let current_dir = std::env::current_dir()?;
    let cargo_toml = current_dir.join("Cargo.toml");

    if cargo_toml.exists() {
        let contents = fs::read_to_string(&cargo_toml)?;
        if contents.contains("name = \"nlsh-rs\"") {
            eprintln!();
            show_cursor();
            let _ = io::Write::flush(&mut io::stderr());
            let remove_repo = crate::common::handle_interrupt(
                Confirm::new()
                    .with_prompt("remove current directory (nlsh-rs repository)?")
                    .default(false)
                    .interact(),
            )?;

            if remove_repo {
                eprintln!("{}", "  removing directory...".dimmed());
                let parent = current_dir.parent().ok_or("cannot remove root directory")?;
                std::env::set_current_dir(parent)?;

                fs::remove_dir_all(&current_dir)?;
                eprintln!("{}", "✓ removed nlsh-rs repository".green());
            }
        }
    }

    eprintln!();
    eprintln!("{}", "nlsh-rs uninstalled successfully!".green().bold());
    eprintln!("{}", "please restart your shell or run 'source ~/.bashrc' (or 'source ~/.config/fish/config.fish' for fish).".yellow());

    Ok(())
}
