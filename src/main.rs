mod cli;
mod shell_integration;

use colored::*;
use shell_integration::remove_shell_integration;
use std::io::IsTerminal;

const CTP_YELLOW: colored::CustomColor = colored::CustomColor { r: 0xf9, g: 0xe2, b: 0xaf };
const CTP_GREEN: colored::CustomColor = colored::CustomColor { r: 0xa6, g: 0xe3, b: 0xa1 };
const CTP_BLUE: colored::CustomColor = colored::CustomColor { r: 0x89, g: 0xb4, b: 0xfa };

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

    eprintln!();
    eprintln!("install the new package:");
    eprintln!("  {}", "cargo install larpshell".custom_color(CTP_BLUE).bold());
    eprintln!();
    eprintln!("to keep the 'nlsh-rs' command name, add to your shell config:");
    eprintln!("  {}", "alias nlsh-rs=larpshell".custom_color(CTP_BLUE).bold());
    eprintln!();
    eprintln!(
        "{}",
        "restart your shell after installing larpshell.".custom_color(CTP_YELLOW)
    );
}
