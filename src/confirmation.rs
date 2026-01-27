use colored::*;
use std::io;

use crate::cli::{is_interactive_terminal, print_error_with_message, read_single_key};
use crate::common::{clear_line, clear_lines, eprint_flush, show_cursor};

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
    if !is_interactive_terminal() {
        return Ok(true);
    }

    eprint_flush(&"[Enter to execute, Ctrl+C to cancel]".yellow().to_string());

    let result = read_single_key()?;

    match result {
        true => clear_line(), // clear prompt line only, keep command visible
        false => {
            // clear prompt line
            clear_line();
            // clear all command display lines
            clear_lines(display_lines);
            show_cursor();
        }
    }

    Ok(result)
}

pub fn display_error(message: &str) {
    print_error_with_message(message);
}
