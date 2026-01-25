use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub fn generate_bash_autocomplete() -> &'static str {
    r#"_nlsh_rs_completions() {
    local cur prev
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    if [ $COMP_CWORD -eq 1 ]; then
        COMPREPLY=( $(compgen -W "api uninstall --help --version" -- "$cur") )
    fi
    return 0
}
complete -F _nlsh_rs_completions nlsh-rs"#
}

pub fn generate_zsh_autocomplete() -> &'static str {
    r#"#compdef nlsh-rs

_nlsh_rs() {
    local -a commands
    commands=(
        'api:configure API provider (Gemini, Ollama, LM Studio, OpenAI)'
        'uninstall:uninstall nlsh-rs'
    )

    _arguments -C \
        '1: :->cmds' \
        '--help[show help information]' \
        '--version[show version information]' \
        '*::arg:->args'

    case "$state" in
        cmds)
            _describe -t commands 'nlsh-rs commands' commands
            ;;
    esac
}

_nlsh_rs"#
}

pub fn generate_fish_autocomplete() -> &'static str {
    r#"# nlsh-rs autocomplete
complete -c nlsh-rs -f
complete -c nlsh-rs -n "__fish_use_subcommand" -a api -d 'configure API provider (Gemini, Ollama, LM Studio, OpenAI)'
complete -c nlsh-rs -n "__fish_use_subcommand" -a uninstall -d 'uninstall nlsh-rs'
complete -c nlsh-rs -l help -d 'show help information'
complete -c nlsh-rs -l version -d 'show version information'"#
}

pub fn generate_bash_function() -> &'static str {
    r#"nlsh-rs() {
    if [ $# -eq 0 ]; then
        command nlsh-rs
        return $?
    fi

    local cmd=$(command nlsh-rs "$@")
    local exit_code=$?
    if [ $exit_code -eq 0 ] && [ -n "$cmd" ]; then
        if [[ "$cmd" =~ ^(Usage:|error:|Commands:|nlsh-rs\ [0-9]|$'\e'|$'\033'|✓|.*:$) ]]; then
            echo "$cmd"
            return 0
        fi
        eval "$cmd"
    else
        return $exit_code
    fi
}"#
}

pub fn generate_fish_function() -> &'static str {
    r#"function nlsh-rs
    if test (count $argv) -eq 0
        command nlsh-rs
        return $status
    end

    set cmd (command nlsh-rs $argv)
    set exit_code $status
    if test $exit_code -eq 0 -a -n "$cmd"
        if string match -qr '^(Usage:|error:|Commands:|nlsh-rs [0-9]|\x1b|\e|✓|.*:$)' -- "$cmd"
            echo "$cmd"
            return 0
        end
        eval $cmd
    else
        return $exit_code
    end
end"#
}

pub fn auto_setup_shell_function() -> Result<bool, Box<dyn std::error::Error>> {
    verify_and_fix_integrations()?;
    let bash_added = setup_bash_integration()?;
    let fish_added = setup_fish_integration()?;
    let autocomplete_added = setup_autocomplete()?;
    Ok(bash_added || fish_added || autocomplete_added)
}

fn verify_and_fix_integrations() -> Result<(), Box<dyn std::error::Error>> {
    verify_and_fix_bash_integration()?;
    verify_and_fix_fish_integration()?;
    Ok(())
}

fn verify_and_fix_bash_integration() -> Result<(), Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(()),
    };

    let bashrc_path = home.join(".bashrc");

    if !bashrc_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&bashrc_path)?;

    // check if integration exists
    if !content.contains("nlsh-rs()") {
        return Ok(());
    }

    // extract the function and verify it matches
    let expected_function = generate_bash_function();

    if !content.contains(expected_function) {
        // function exists but doesn't match - remove and reinstall
        remove_bash_integration()?;
        setup_bash_integration()?;
    }

    Ok(())
}

fn verify_and_fix_fish_integration() -> Result<(), Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(()),
    };

    let fish_function_path = home.join(".config/fish/functions/nlsh-rs.fish");

    if !fish_function_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&fish_function_path)?;

    // verify the function matches expected content
    let expected_function = generate_fish_function();

    if !content.contains(expected_function) {
        // function exists but doesn't match - remove and reinstall
        remove_fish_integration()?;
        setup_fish_integration()?;
    }

    Ok(())
}

fn setup_bash_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let bashrc_path = home.join(".bashrc");

    if !bashrc_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&bashrc_path)?;

    if content.contains("nlsh-rs() {") || content.contains("nlsh-rs()") {
        return Ok(false);
    }

    let mut file = OpenOptions::new().append(true).open(&bashrc_path)?;

    writeln!(file, "\n# nlsh-rs shell integration")?;
    writeln!(file, "{}", generate_bash_function())?;

    Ok(true)
}

fn setup_fish_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let fish_functions_dir = home.join(".config/fish/functions");
    let fish_function_path = fish_functions_dir.join("nlsh-rs.fish");

    if fish_function_path.exists() {
        return Ok(false);
    }

    let fish_config_dir = home.join(".config/fish");
    if !fish_config_dir.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&fish_functions_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&fish_function_path)?;

    writeln!(file, "# nlsh-rs shell integration")?;
    writeln!(file, "{}", generate_fish_function())?;

    Ok(true)
}

pub fn remove_bash_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let bashrc_path = home.join(".bashrc");

    if !bashrc_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&bashrc_path)?;

    if !content.contains("nlsh-rs() {") && !content.contains("nlsh-rs()") {
        return Ok(false);
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut skip = false;
    let mut brace_depth = 0;
    let mut in_function = false;
    let mut found = false;

    for line in lines {
        if line.trim() == "# nlsh-rs shell integration" {
            skip = true;
            found = true;
            continue;
        }

        if skip {
            // detect start of nlsh-rs function
            if !in_function && (line.contains("nlsh-rs() {") || line.contains("nlsh-rs()")) {
                in_function = true;
                // count opening braces on this line
                brace_depth += line.matches('{').count() as i32;
            } else if in_function {
                // count braces while inside function
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;

                // if brace depth returns to 0, we found the closing brace
                if brace_depth == 0 {
                    skip = false;
                    in_function = false;
                    continue;
                }
            }
            continue;
        }

        new_lines.push(line);
    }

    if found {
        while new_lines.last().is_some_and(|l| l.trim().is_empty()) {
            new_lines.pop();
        }

        fs::write(&bashrc_path, new_lines.join("\n") + "\n")?;
    }

    Ok(found)
}

pub fn remove_fish_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let fish_function_path = home.join(".config/fish/functions/nlsh-rs.fish");

    if fish_function_path.exists() {
        fs::remove_file(&fish_function_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn setup_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let bash_added = setup_bash_autocomplete()?;
    let zsh_added = setup_zsh_autocomplete()?;
    let fish_added = setup_fish_autocomplete()?;
    Ok(bash_added || zsh_added || fish_added)
}

fn setup_bash_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let completion_dir = home.join(".local/share/bash-completion/completions");
    let completion_path = completion_dir.join("nlsh-rs");

    if completion_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&completion_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&completion_path)?;

    writeln!(file, "# nlsh-rs bash autocomplete")?;
    writeln!(file, "{}", generate_bash_autocomplete())?;

    Ok(true)
}

fn setup_zsh_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let zsh_config = home.join(".zshrc");
    if !zsh_config.exists() {
        return Ok(false);
    }

    let completion_dir = home.join(".local/share/zsh/site-functions");
    let completion_path = completion_dir.join("_nlsh-rs");

    if completion_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&completion_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&completion_path)?;

    writeln!(file, "# nlsh-rs zsh autocomplete")?;
    writeln!(file, "{}", generate_zsh_autocomplete())?;

    let zshrc_content = fs::read_to_string(&zsh_config)?;
    if !zshrc_content.contains(".local/share/zsh/site-functions") {
        let mut file = OpenOptions::new().append(true).open(&zsh_config)?;
        writeln!(file, "\n# nlsh-rs autocomplete")?;
        writeln!(file, "fpath=(~/.local/share/zsh/site-functions $fpath)")?;
        writeln!(file, "autoload -Uz compinit && compinit")?;
    }

    Ok(true)
}

fn setup_fish_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let fish_config_dir = home.join(".config/fish");
    if !fish_config_dir.exists() {
        return Ok(false);
    }

    let completion_dir = home.join(".config/fish/completions");
    let completion_path = completion_dir.join("nlsh-rs.fish");

    if completion_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&completion_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&completion_path)?;

    writeln!(file, "{}", generate_fish_autocomplete())?;

    Ok(true)
}

fn remove_bash_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let completion_path = home.join(".local/share/bash-completion/completions/nlsh-rs");

    if completion_path.exists() {
        fs::remove_file(&completion_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn remove_zsh_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let completion_path = home.join(".local/share/zsh/site-functions/_nlsh-rs");

    let mut removed = false;
    if completion_path.exists() {
        fs::remove_file(&completion_path)?;
        removed = true;
    }

    let zsh_config = home.join(".zshrc");
    if zsh_config.exists() {
        let content = fs::read_to_string(&zsh_config)?;
        if content.contains("# nlsh-rs autocomplete") {
            let lines: Vec<&str> = content.lines().collect();
            let mut new_lines = Vec::new();
            let mut skip = false;

            for line in lines {
                if line.trim() == "# nlsh-rs autocomplete" {
                    skip = true;
                    removed = true;
                    continue;
                }

                if skip {
                    if line.contains(".local/share/zsh/site-functions")
                        || line.contains("autoload -Uz compinit")
                    {
                        continue;
                    }
                    skip = false;
                }

                new_lines.push(line);
            }

            if removed {
                while new_lines.last().is_some_and(|l| l.trim().is_empty()) {
                    new_lines.pop();
                }
                fs::write(&zsh_config, new_lines.join("\n") + "\n")?;
            }
        }
    }

    Ok(removed)
}

fn remove_fish_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(false),
    };

    let completion_path = home.join(".config/fish/completions/nlsh-rs.fish");

    if completion_path.exists() {
        fs::remove_file(&completion_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn remove_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let bash_removed = remove_bash_autocomplete()?;
    let zsh_removed = remove_zsh_autocomplete()?;
    let fish_removed = remove_fish_autocomplete()?;
    Ok(bash_removed || zsh_removed || fish_removed)
}

pub fn remove_shell_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let bash_removed = remove_bash_integration()?;
    let fish_removed = remove_fish_integration()?;
    let autocomplete_removed = remove_autocomplete()?;
    Ok(bash_removed || fish_removed || autocomplete_removed)
}
