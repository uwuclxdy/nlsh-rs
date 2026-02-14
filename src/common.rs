use std::env;
use std::fs;
use std::io;
use std::process::Command;

// ====================
// environment helpers
// ====================

/// returns the current working directory.
pub fn get_current_directory() -> String {
    env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string())
}

/// returns the operating system name.
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

/// returns the current shell name.
pub fn get_shell() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.split('/').next_back().map(|s| s.to_string()))
        .unwrap_or_else(|| "sh".to_string())
}

/// returns the current username.
pub fn get_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

// ====================
// process & execution helpers
// ====================

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

// ====================
// terminal control helpers
// ====================

/// shows the cursor on stderr.
pub fn show_cursor() {
    eprint!("\x1b[?25h");
    flush_stderr();
}

/// clears the current line on stderr (from cursor to end of line).
pub fn clear_line() {
    eprint!("\r\x1b[K");
    flush_stderr();
}

/// clears current line by filling it with spaces and returning to start.
pub fn clear_line_with_spaces(width: usize) {
    eprint!("\r{}\r", " ".repeat(width));
    flush_stderr();
}

/// clears the current line and moves cursor up by the specified number of lines.
pub fn clear_lines(count: usize) {
    for _ in 0..count {
        eprint!("\x1b[1A\x1b[K");
    }
    flush_stderr();
}

/// prints a message to stderr and flushes immediately.
pub fn eprint_flush(msg: &str) {
    eprint!("{}", msg);
    flush_stderr();
}

/// flushes stderr to ensure output is displayed.
pub fn flush_stderr() {
    let _ = io::Write::flush(&mut io::stderr());
}

// ====================
// process & execution helpers
// ====================

/// exits the process with the given exit code.
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
