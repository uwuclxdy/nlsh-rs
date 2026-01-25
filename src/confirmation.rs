use colored::*;
use std::io::{self, IsTerminal, Read, stdin};

pub fn display_command(command: &str) {
    eprintln!("{} {}", "→".cyan(), command.bright_white().bold());
}

pub fn confirm_execution() -> Result<bool, io::Error> {
    if !stdin().is_terminal() {
        return Ok(true);
    }

    eprint!("{}", "[Enter to execute, Ctrl+C to cancel]".yellow());

    let mut input: [u8; 1] = [0];
    match stdin().read(&mut input) {
        Ok(_) => {
            eprint!("\x1b[1A\r\x1b[K");
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::Interrupted => {
            eprint!("\x1b[1A\r\x1b[K\x1b[?25h");
            std::process::exit(130);
        }
        Err(e) => Err(e),
    }
}

pub fn display_error(message: &str) {
    eprintln!("{} {}", "error:".red().bold(), message);
}
