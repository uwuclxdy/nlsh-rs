use colored::*;
use inquire::Confirm;

use crate::cli::{is_interactive_terminal, print_error_with_message};
use crate::common::{clear_line, clear_lines, show_cursor};

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

pub fn confirm_execution(display_lines: usize) -> Result<bool, Box<dyn std::error::Error>> {
    if !is_interactive_terminal() {
        return Ok(true);
    }

    let result = Confirm::new("")
        .with_default(true)
        .with_help_message("Enter to execute, Ctrl+C to cancel")
        .prompt();

    match result {
        Ok(true) => {
            // Clear inquire prompt line + help message line
            clear_line();
            clear_lines(1);
            Ok(true)
        }
        Ok(false) | Err(_) => {
            // Clear help message + prompt + command display
            clear_line();
            clear_lines(1 + display_lines);
            show_cursor();
            Ok(false)
        }
    }
}

pub fn display_error(message: &str) {
    print_error_with_message(message);
}
