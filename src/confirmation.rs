use colored::*;

use crate::cli::{is_interactive_terminal, print_error_with_message};
use crate::common::{clear_line, clear_lines, exit_with_code, flush_stderr, show_cursor};

pub enum ConfirmResult {
    Yes,
    No,
    Explain,
}

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

pub fn display_explanation(explanation: &str) -> usize {
    let styled = style_html_tags(explanation);
    let lines: Vec<&str> = styled.lines().collect();
    for line in &lines {
        eprintln!("{}", line.bright_white());
    }
    lines.len()
}

fn style_html_tags(text: &str) -> String {
    if colored::control::SHOULD_COLORIZE.should_colorize() {
        text.replace("<b>", "\x1b[1m")
            .replace("</b>", "\x1b[22m")
            .replace("<i>", "\x1b[3m")
            .replace("</i>", "\x1b[23m")
            .replace("<u>", "\x1b[4m")
            .replace("</u>", "\x1b[24m")
    } else {
        text.replace("<b>", "")
            .replace("</b>", "")
            .replace("<i>", "")
            .replace("</i>", "")
            .replace("<u>", "")
            .replace("</u>", "")
    }
}

fn read_raw_key() -> u8 {
    use std::io::Read;

    #[cfg(unix)]
    {
        use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};

        let stdin = std::io::stdin();
        if let Ok(original) = tcgetattr(&stdin) {
            let mut raw = original.clone();
            raw.local_flags
                .remove(LocalFlags::ICANON | LocalFlags::ECHO | LocalFlags::ISIG);
            if tcsetattr(&stdin, SetArg::TCSANOW, &raw).is_ok() {
                let mut buf = [0u8; 1];
                let _ = stdin.lock().read(&mut buf);
                let _ = tcsetattr(&stdin, SetArg::TCSANOW, &original);
                return buf[0];
            }
            let _ = tcsetattr(&stdin, SetArg::TCSANOW, &original);
        }
    }

    let mut buf = [0u8; 1];
    let _ = std::io::stdin().lock().read(&mut buf);
    buf[0]
}

pub fn confirm_with_explain(
    display_lines: usize,
) -> Result<ConfirmResult, Box<dyn std::error::Error>> {
    if !is_interactive_terminal() {
        return Ok(ConfirmResult::Yes);
    }

    confirmation_prompt(true);

    flush_stderr();

    let key = read_raw_key();
    match key {
        b'\n' | b'\r' | b'y' | b'Y' => {
            clear_line();
            clear_lines(1);
            Ok(ConfirmResult::Yes)
        }
        b'e' | b'E' => {
            clear_line();
            clear_lines(1);
            Ok(ConfirmResult::Explain)
        }
        b'\x03' => {
            clear_line();
            clear_lines(1 + display_lines);
            show_cursor();
            exit_with_code(130);
        }
        _ => {
            clear_line();
            clear_lines(1 + display_lines);
            show_cursor();
            Ok(ConfirmResult::No)
        }
    }
}

pub fn confirm_execution(display_lines: usize) -> Result<bool, Box<dyn std::error::Error>> {
    if !is_interactive_terminal() {
        return Ok(true);
    }

    confirmation_prompt(false);
    flush_stderr();

    let key = read_raw_key();
    match key {
        b'\n' | b'\r' | b'y' | b'Y' => {
            clear_line();
            clear_lines(1);
            Ok(true)
        }
        b'\x03' => {
            clear_line();
            clear_lines(1 + display_lines);
            show_cursor();
            exit_with_code(130);
        }
        _ => {
            clear_line();
            clear_lines(1 + display_lines);
            show_cursor();
            Ok(false)
        }
    }
}

fn confirmation_prompt(with_explain: bool) {
    if with_explain {
        eprintln!(
            "{} {}",
            "Run this?".yellow(),
            "(Y/e/n)".truecolor(128, 128, 128)
        );
        eprint!(
            "{}",
            format!(
                "[{}] to execute, [{}] to explain, [{}] to edit, [{}] to cancel",
                "Y/Enter".bold(),
                "E".bold(),
                "Arrow Up".bold(),
                "N".bold()
            )
            .cyan()
        );
    } else {
        eprintln!(
            "{} {}",
            "Run this?".yellow(),
            "(Y/n)".truecolor(128, 128, 128)
        );
        eprint!(
            "{}",
            format!(
                "[{}] to execute, [{}] to edit, [{}] to cancel",
                "Y/Enter".bold(),
                "Arrow Up".bold(),
                "N".bold()
            )
            .cyan()
        );
    }
}

pub fn display_error(message: &str) {
    print_error_with_message(message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_command_single_line_returns_one() {
        assert_eq!(display_command("echo hi"), 1);
    }

    #[test]
    fn display_command_multiline_returns_n_plus_one() {
        assert_eq!(display_command("echo hi\necho bye"), 3);
        assert_eq!(display_command("a\nb\nc"), 4);
    }

    #[test]
    fn display_explanation_single_line_returns_one() {
        assert_eq!(display_explanation("pipes stdout to a file"), 1);
    }

    #[test]
    fn display_explanation_multiline_returns_line_count() {
        assert_eq!(display_explanation("line one\nline two\nline three"), 3);
    }

    #[test]
    fn style_html_tags_converts_bold() {
        colored::control::set_override(true);
        let result = style_html_tags("<b>hello</b>");
        assert_eq!(result, "\x1b[1mhello\x1b[22m");
    }

    #[test]
    fn style_html_tags_converts_italic() {
        colored::control::set_override(true);
        let result = style_html_tags("<i>hello</i>");
        assert_eq!(result, "\x1b[3mhello\x1b[23m");
    }

    #[test]
    fn style_html_tags_converts_underline() {
        colored::control::set_override(true);
        let result = style_html_tags("<u>hello</u>");
        assert_eq!(result, "\x1b[4mhello\x1b[24m");
    }

    #[test]
    fn style_html_tags_strips_when_no_color() {
        colored::control::set_override(false);
        let result = style_html_tags("<b>bold</b> and <i>italic</i>");
        assert_eq!(result, "bold and italic");
        colored::control::set_override(true);
    }
}
