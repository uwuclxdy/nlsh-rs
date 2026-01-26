use crate::cli;
use crate::common::show_cursor;
use colored::*;
use std::io;

pub fn display_command(command: &str) -> usize {
    let lines: Vec<&str> = command.lines().collect();
    let line_count = lines.len();
    if line_count == 1 {
        eprintln!("{} {}", "$".cyan(), command.bright_white().bold());
        1
    } else {
        eprintln!(
            "{} {}",
            ">".cyan(),
            "multiline command:".bright_white().bold()
        );
        for line in lines.iter() {
            eprintln!("{} {}", "$".cyan(), line.bright_white());
        }
        line_count + 1 // header + command lines
    }
}

pub fn confirm_execution(display_lines: usize) -> Result<bool, io::Error> {
    if !cli::is_interactive_terminal() {
        return Ok(true);
    }

    eprint!("{}", "[Enter to execute, Ctrl+C to cancel]".yellow());

    let result = cli::read_single_key()?;

    match result {
        true => eprint!("\r\x1b[K"), // clear prompt line only, keep command visible
        false => {
            // clear prompt line
            eprint!("\r\x1b[K");
            // clear all command display lines
            for _ in 0..display_lines {
                eprint!("\x1b[1A\x1b[K");
            }
            show_cursor();
        }
    }

    Ok(result)
}

pub fn display_error(message: &str) {
    eprintln!("{} {}", "error:".red().bold(), message);
}
