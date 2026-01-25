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

    let mut termios = unsafe {
        let mut termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }
        termios
    };

    let original_termios = termios;
    termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
    termios.c_cc[libc::VMIN] = 1;
    termios.c_cc[libc::VTIME] = 0;

    unsafe {
        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    let result = loop {
        let mut input: [u8; 1] = [0];
        match stdin().read(&mut input) {
            Ok(0) => break Err(io::Error::new(io::ErrorKind::UnexpectedEof, "eof")),
            Ok(_) => {
                match input[0] {
                    b'\n' | b'\r' => break Ok(true), // enter
                    3 => break Ok(false),            // ctrl+c - cancel
                    _ => continue,                   // ignore all other input
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => break Ok(false),
            Err(e) => break Err(e),
        }
    };

    // restore terminal settings
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original_termios);
    }

    match result {
        Ok(true) => eprint!("\r\x1b[K"), // clear prompt line only, keep command visible
        Ok(false) => eprint!("\r\x1b[K\x1b[1A\x1b[K\x1b[?25h"), // clear prompt and command on cancel
        Err(_) => {}
    }

    result
}

pub fn display_error(message: &str) {
    eprintln!("{} {}", "error:".red().bold(), message);
}
