use colored::*;
use inquire::{Select, Text};
use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

const SYMBOL_CHECK: &str = "\u{2713}";
const SYMBOL_ERROR: &str = "error:";
const SYMBOL_WARNING: &str = "warning:";

pub fn print_ok(message: &str) {
    eprintln!("{} {}", SYMBOL_CHECK.green(), message);
}

pub fn print_ok_bold(message: &str) {
    eprintln!("{} {}", SYMBOL_CHECK.green(), message.bold());
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", SYMBOL_ERROR.red().bold(), message);
}

pub fn print_warning(message: &str) {
    eprintln!("{} {}", SYMBOL_WARNING.yellow(), message);
}

#[derive(Debug)]
pub struct CliArgs {
    pub command: Vec<String>,
    pub subcommand: Option<Subcommands>,
}

#[derive(Debug)]
pub enum Subcommands {
    Api,
    Uninstall,
    Prompt {
        kind: PromptKind,
        action: PromptAction,
    },
    Explain {
        command: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum PromptKind {
    System,
    Explain,
}

#[derive(Debug, Clone)]
pub enum PromptAction {
    Show,
    Edit,
}

pub fn parse_cli_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    use clap::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(name = "nlsh-rs")]
    #[command(version)]
    #[command(disable_help_subcommand = true)]
    struct Cli {
        #[arg(value_name = "COMMAND")]
        command: Vec<String>,

        #[command(subcommand)]
        subcommand: Option<Commands>,
    }

    #[derive(Subcommand)]
    enum Commands {
        Api,
        Uninstall,
        Prompt {
            #[arg(value_enum, default_value_t = ClapPromptKind::System)]
            kind: ClapPromptKind,
            #[arg(value_enum, default_value_t = ClapPromptAction::Show)]
            action: ClapPromptAction,
        },
        Explain {
            command: Vec<String>,
        },
    }

    #[derive(clap::ValueEnum, Clone)]
    enum ClapPromptKind {
        System,
        Explain,
    }

    #[derive(clap::ValueEnum, Clone)]
    enum ClapPromptAction {
        Show,
        Edit,
    }

    let cli = Cli::parse();

    let subcommand = match cli.subcommand {
        Some(Commands::Api) => Some(Subcommands::Api),
        Some(Commands::Uninstall) => Some(Subcommands::Uninstall),
        Some(Commands::Prompt { kind, action }) => Some(Subcommands::Prompt {
            kind: match kind {
                ClapPromptKind::System => PromptKind::System,
                ClapPromptKind::Explain => PromptKind::Explain,
            },
            action: match action {
                ClapPromptAction::Show => PromptAction::Show,
                ClapPromptAction::Edit => PromptAction::Edit,
            },
        }),
        Some(Commands::Explain { command }) => Some(Subcommands::Explain { command }),
        None => None,
    };

    Ok(CliArgs {
        command: cli.command,
        subcommand,
    })
}

pub fn execute_shell_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = command.trim();

    if trimmed.is_empty() {
        return Ok(());
    }

    // Run everything through sh -c so tilde expansion, env vars, pipes,
    // compound operators, and all other shell features work natively.
    // Append `pwd` to capture the shell's final working directory and
    // sync it back, so that `cd` (even inside compound commands) propagates
    // to the parent process.
    let cwd_file = env::temp_dir().join(format!(".nlsh_cwd_{}", std::process::id()));
    let script = format!(
        "{trimmed}\n__nlsh_rc=$?\npwd > {cwd_path}\nexit $__nlsh_rc",
        cwd_path = cwd_file.display(),
    );

    Command::new("sh")
        .arg("-c")
        .arg(&script)
        .current_dir(env::current_dir()?)
        .status()?;

    // Sync the shell's final cwd back to the parent process.
    if let Ok(new_cwd) = std::fs::read_to_string(&cwd_file) {
        let new_cwd = new_cwd.trim();
        if !new_cwd.is_empty() {
            let _ = env::set_current_dir(new_cwd);
        }
    }
    let _ = std::fs::remove_file(&cwd_file);

    Ok(())
}

pub fn is_interactive_terminal() -> bool {
    if std::env::var("NLSH_FORCE_INTERACTIVE").is_ok() {
        return true;
    }
    std::io::stdin().is_terminal()
}

pub fn prompt_select(
    prompt: &str,
    items: &[String],
    default: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let selection = Select::new(prompt, items.to_vec())
        .with_starting_cursor(default)
        .prompt()?;
    Ok(items
        .iter()
        .position(|x| x == &selection)
        .unwrap_or(default))
}

pub fn prompt_input(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Text::new(prompt).prompt()?)
}

pub fn prompt_input_with_default(
    prompt: &str,
    default: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Text::new(prompt).with_default(default).prompt()?)
}

pub fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `set_current_dir` is process-global, so cwd tests must not run in parallel.
    static CD_LOCK: Mutex<()> = Mutex::new(());

    /// Run a test while preserving the original working directory.
    fn with_saved_cwd(f: impl FnOnce()) {
        let _guard = CD_LOCK.lock().unwrap();
        let original = env::current_dir().unwrap();
        f();
        env::set_current_dir(&original).unwrap();
    }

    #[test]
    fn empty_command_is_noop() {
        assert!(execute_shell_command("").is_ok());
        assert!(execute_shell_command("   ").is_ok());
    }

    #[test]
    fn cd_bare_goes_home() {
        with_saved_cwd(|| {
            let home = env::var("HOME").unwrap();
            execute_shell_command("cd").unwrap();
            assert_eq!(env::current_dir().unwrap(), PathBuf::from(&home));
        });
    }

    #[test]
    fn cd_absolute_path() {
        with_saved_cwd(|| {
            execute_shell_command("cd /tmp").unwrap();
            assert_eq!(env::current_dir().unwrap(), PathBuf::from("/tmp"));
        });
    }

    #[test]
    fn cd_tilde_expands_to_home() {
        with_saved_cwd(|| {
            let home = env::var("HOME").unwrap();
            execute_shell_command("cd ~").unwrap();
            assert_eq!(env::current_dir().unwrap(), PathBuf::from(&home));
        });
    }

    #[test]
    fn cd_tilde_subdir_expands() {
        with_saved_cwd(|| {
            let home = env::var("HOME").unwrap();
            let subdir = PathBuf::from(&home);
            // Ensure $HOME exists, then cd ~ should land there.
            assert!(subdir.is_dir(), "$HOME must exist");
            execute_shell_command("cd ~").unwrap();
            assert_eq!(env::current_dir().unwrap(), subdir);
        });
    }

    #[test]
    fn cd_nonexistent_keeps_cwd() {
        with_saved_cwd(|| {
            let before = env::current_dir().unwrap();
            // sh prints an error to stderr; cwd stays unchanged.
            execute_shell_command("cd /nonexistent_dir_that_should_not_exist").unwrap();
            assert_eq!(env::current_dir().unwrap(), before);
        });
    }

    #[test]
    fn compound_cd_changes_cwd() {
        with_saved_cwd(|| {
            // `cd /tmp && echo ok` should run both parts and sync cwd back.
            execute_shell_command("cd /tmp && echo ok").unwrap();
            assert_eq!(env::current_dir().unwrap(), PathBuf::from("/tmp"));
        });
    }

    #[test]
    fn compound_cd_failed_keeps_cwd() {
        with_saved_cwd(|| {
            let before = env::current_dir().unwrap();
            // cd to nonexistent dir fails, `echo ok` never runs, cwd unchanged.
            execute_shell_command("cd /nonexistent_dir && echo ok").unwrap();
            assert_eq!(env::current_dir().unwrap(), before);
        });
    }

    #[test]
    fn pipe_command_runs() {
        assert!(execute_shell_command("echo hello | cat").is_ok());
    }

    #[test]
    fn regular_command_runs_via_shell() {
        assert!(execute_shell_command("echo hello").is_ok());
    }

    #[test]
    fn multiline_command_runs_via_shell() {
        assert!(execute_shell_command("echo line1\necho line2").is_ok());
    }
}
