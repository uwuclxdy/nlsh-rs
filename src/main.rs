mod cli;
mod shell_integration;

use colored::*;
use shell_integration::remove_shell_integration;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

const CTP_YELLOW: colored::CustomColor = colored::CustomColor { r: 0xf9, g: 0xe2, b: 0xaf };
const CTP_GREEN: colored::CustomColor = colored::CustomColor { r: 0xa6, g: 0xe3, b: 0xa1 };
const CTP_BLUE: colored::CustomColor = colored::CustomColor { r: 0x89, g: 0xb4, b: 0xfa };
const CTP_RED: colored::CustomColor = colored::CustomColor { r: 0xf3, g: 0x8b, b: 0xa8 };

fn confirm(prompt: &str) -> bool {
    eprint!("{} [Y/n] ", prompt);
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let s = input.trim().to_lowercase();
    s.is_empty() || s == "y" || s == "yes"
}

/// Copies `~/.config/nlsh-rs/` → `~/.config/larpshell/` if larpshell has no
/// config yet.  Returns true if files were copied.
fn migrate_config() -> bool {
    let Some(base) = dirs::config_dir() else {
        return false;
    };
    let old = base.join("nlsh-rs");
    let new = base.join("larpshell");

    if !old.exists() || new.join("config.toml").exists() {
        return false;
    }

    if fs::create_dir_all(&new).is_err() {
        return false;
    }

    let Ok(entries) = fs::read_dir(&old) else {
        return false;
    };

    let mut copied = false;
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_file()) {
            if fs::copy(entry.path(), new.join(entry.file_name())).is_ok() {
                copied = true;
            }
        }
    }
    copied
}

fn run_cargo(args: &[&str]) -> bool {
    Command::new("cargo")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn main() {
    if std::io::stderr().is_terminal() {
        colored::control::set_override(true);
    }

    eprintln!("{}", "nlsh-rs has been renamed to larpshell.".custom_color(CTP_YELLOW).bold());
    eprintln!();

    match remove_shell_integration() {
        Ok(true) => eprintln!(
            "  {} removed nlsh-rs shell integration",
            "\u{2713}".custom_color(CTP_GREEN)
        ),
        Ok(false) => {}
        Err(e) => eprintln!(
            "  {} could not remove shell integration: {}",
            "warning:".custom_color(CTP_YELLOW),
            e
        ),
    }

    if migrate_config() {
        eprintln!(
            "  {} migrated config to ~/.config/larpshell/",
            "\u{2713}".custom_color(CTP_GREEN)
        );
    }

    eprintln!();

    if confirm(&format!(
        "{}",
        "Uninstall nlsh-rs and install larpshell?".custom_color(CTP_YELLOW)
    )) {
        eprintln!();

        let uninstalled = run_cargo(&["uninstall", "nlsh-rs"]);
        if uninstalled {
            eprintln!("  {} uninstalled nlsh-rs", "\u{2713}".custom_color(CTP_GREEN));
        } else {
            eprintln!("  {} failed to uninstall nlsh-rs", "warning:".custom_color(CTP_YELLOW));
        }

        eprintln!();

        let installed = run_cargo(&["install", "larpshell"]);
        if installed {
            eprintln!("  {} installed larpshell", "\u{2713}".custom_color(CTP_GREEN));
        } else {
            eprintln!(
                "  {} cargo install larpshell failed — run it manually",
                "error:".custom_color(CTP_RED)
            );
        }

        eprintln!();
        eprintln!("to keep the 'nlsh-rs' command name, add to your shell config:");
        eprintln!("  {}", "alias nlsh-rs=larpshell".custom_color(CTP_BLUE).bold());
        eprintln!();
        eprintln!("{}", "restart your shell.".custom_color(CTP_YELLOW));
    } else {
        eprintln!();
        eprintln!("to migrate manually:");
        eprintln!("  {}", "cargo uninstall nlsh-rs".custom_color(CTP_BLUE).bold());
        eprintln!("  {}", "cargo install larpshell".custom_color(CTP_BLUE).bold());
    }
}
