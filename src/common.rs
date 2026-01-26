use std::env;
use std::fs;
use std::io;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

// ====================
// signal handling
// ====================

static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

/// ctrl+c handler that shows the cursor before exit.
/// should be called before any interactive prompts.
pub fn setup_interrupt_handler() {
    if HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    #[cfg(unix)]
    #[allow(clippy::missing_safety_doc, clippy::fn_to_numeric_cast)]
    unsafe {
        extern "C" fn handle_sigint(_: libc::c_int) {
            eprint!("\x1b[?25h");
            let _ = io::Write::flush(&mut io::stderr());
            // use _exit to avoid running destructors which might hang
            unsafe {
                libc::_exit(130);
            }
        }

        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handle_sigint as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);

        libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut());
    }
}

/// handles ctrl+c to show the cursor before exiting with code 130.
pub fn handle_interrupt<T>(
    result: Result<T, dialoguer::Error>,
) -> Result<T, Box<dyn std::error::Error>> {
    match result {
        Ok(val) => Ok(val),
        Err(dialoguer::Error::IO(e)) => {
            // show cursor on any io error
            eprint!("\x1b[?25h");

            // exit on interrupted or not connected errors
            if e.kind() == io::ErrorKind::Interrupted
                || e.kind() == io::ErrorKind::NotConnected
                || e.kind() == io::ErrorKind::UnexpectedEof
            {
                std::process::exit(130);
            }

            Err(Box::new(dialoguer::Error::IO(e)))
        }
    }
}

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

/// returns the home directory path.
pub fn get_home_directory() -> String {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".to_string())
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

/// expands ~ to home directory in a path.
pub fn expand_home(path: &str) -> String {
    if path == "~" {
        get_home_directory()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{}", get_home_directory(), rest)
    } else {
        path.to_string()
    }
}

fn get_linux_info() -> String {
    let distro = get_linux_distro();
    let kernel = get_kernel_version();

    format!("linux ({}; kernel: {})", distro, kernel)
}

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
// process & execution helpers
// ====================

/// exits the process with the given exit code.
pub fn exit_with_code(code: i32) -> ! {
    std::process::exit(code);
}

/// sets up terminal to hide control characters.
#[cfg(unix)]
pub fn setup_terminal() {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(std::io::stdin().as_raw_fd(), &mut termios) == 0 {
            termios.c_lflag &= !libc::ECHOCTL;
            libc::tcsetattr(std::io::stdin().as_raw_fd(), libc::TCSANOW, &termios);
        }
    }
}

#[cfg(not(unix))]
pub fn setup_terminal() {
    // no-op on non-unix systems
}

/// shows the cursor on stderr.
pub fn show_cursor() {
    eprint!("\x1b[?25h");
    let _ = io::Write::flush(&mut io::stderr());
}

// ====================
// file system helpers
// ====================

/// sets restrictive file permissions on unix systems.
pub fn set_file_permissions(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}
