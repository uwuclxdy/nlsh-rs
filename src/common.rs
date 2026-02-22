use std::env;
use std::fs;
use std::io;
use std::process::Command;

use nix::libc;
use strip_ansi_escapes::strip;
use unicode_width::UnicodeWidthStr;

pub const EXIT_SIGINT: i32 = 130;

pub const ANSI_SHOW_CURSOR: &str = "\x1b[?25h";
pub const ANSI_CLEAR_LINE: &str = "\r\x1b[K";
pub const ANSI_CURSOR_UP_CLEAR: &str = "\x1b[1A\x1b[K";

pub fn get_current_directory() -> String {
    env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string())
}

pub fn get_os() -> String {
    if cfg!(target_os = "linux") {
        get_linux_info()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else {
        "Unix".to_string()
    }
}

pub fn get_shell() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.split('/').next_back().map(|s| s.to_string()))
        .unwrap_or_else(|| "sh".to_string())
}

pub fn get_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

/// returns linux distro and kernel version.
fn get_linux_info() -> String {
    let distro = get_linux_distro();
    let kernel = get_kernel_version();

    format!("linux ({}; kernel: {})", distro, kernel)
}

/// reads /etc/os-release to get the distro name and version.
fn get_linux_distro() -> String {
    if let Ok(contents) = fs::read_to_string("/etc/os-release") {
        let mut name = None;
        let mut version = None;

        for line in contents.lines() {
            if let Some(value) = line.strip_prefix("NAME=") {
                name = Some(value.trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
                version = Some(value.trim_matches('"').to_string());
            }
        }

        match (name, version) {
            (Some(n), Some(v)) => format!("{} {}", n, v),
            (Some(n), None) => n,
            _ => "linux".to_string(),
        }
    } else {
        "linux".to_string()
    }
}

/// gets the kernel version from `uname -r` or `/proc/sys/kernel/osrelease`.
fn get_kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .or_else(|| fs::read_to_string("/proc/sys/kernel/osrelease").ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn show_cursor() {
    eprint!("{}", ANSI_SHOW_CURSOR);
    flush_stderr();
}

pub fn clear_line() {
    eprint!("{}", ANSI_CLEAR_LINE);
    flush_stderr();
}


/// clears exactly `n` visual lines from the terminal, starting at the current
/// cursor line and moving upward. the cursor is assumed to be on the last of
/// these `n` lines (e.g. after an `eprint!` without newline).
pub fn clear_n_lines(n: usize) {
    if n == 0 {
        return;
    }
    eprint!("{}", ANSI_CLEAR_LINE);
    for _ in 0..n.saturating_sub(1) {
        eprint!("{}", ANSI_CURSOR_UP_CLEAR);
    }
    flush_stderr();
}

pub fn eprint_flush(msg: &str) {
    eprint!("{}", msg);
    flush_stderr();
}

pub fn flush_stderr() {
    let _ = io::Write::flush(&mut io::stderr());
}

/// gets the terminal width in columns.
#[cfg(unix)]
pub fn get_terminal_width() -> usize {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            return ws.ws_col as usize;
        }
    }
    80
}

#[cfg(not(unix))]
pub fn get_terminal_width() -> usize {
    Command::new("tput")
        .arg("cols")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(80)
}

/// counts the number of visual lines a string will occupy when printed to terminal.
pub fn count_visual_lines(text: &str, width: usize) -> usize {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                1
            } else {
                // Strip ANSI escape codes to get only visible characters
                let stripped = strip(line.as_bytes());
                let visible_line = String::from_utf8_lossy(&stripped);
                // Calculate visual width accounting for wide characters
                let visual_width = visible_line.width();
                (visual_width + width - 1) / width
            }
        })
        .sum()
}

pub fn exit_with_code(code: i32) -> ! {
    std::process::exit(code);
}

/// sets up terminal to hide control characters.
#[cfg(unix)]
pub fn setup_terminal() {
    use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};

    let stdin = std::io::stdin();
    if let Ok(mut termios) = tcgetattr(&stdin) {
        termios.local_flags.remove(LocalFlags::ECHOCTL);
        let _ = tcsetattr(&stdin, SetArg::TCSANOW, &termios);
    }
}
