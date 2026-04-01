use std::fs::{self};

use crate::cli::get_home_dir;

/// Removes a marked function block from shell config content.
/// Looks for `marker` as a comment line, then tracks brace depth starting from
/// the line matching `function_sig` until braces balance to zero.
/// Returns the cleaned content and whether the block was found.
fn remove_marked_function_block(content: &str, marker: &str, function_sig: &str) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut skip = false;
    let mut brace_depth = 0;
    let mut in_function = false;
    let mut found = false;

    for line in lines {
        if line.trim() == marker {
            skip = true;
            found = true;
            continue;
        }

        if skip {
            if !in_function && line.contains(function_sig) {
                in_function = true;
                brace_depth += line.matches('{').count() as i32;
            } else if in_function {
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;

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
    }

    (new_lines.join("\n") + "\n", found)
}

pub fn remove_bash_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
    let bashrc_path = home.join(".bashrc");

    if !bashrc_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&bashrc_path)?;

    if !content.contains("nlsh-rs() {") && !content.contains("nlsh-rs()") {
        return Ok(false);
    }

    let (new_content, found) =
        remove_marked_function_block(&content, "# nlsh-rs shell integration", "nlsh-rs()");

    if found {
        fs::write(&bashrc_path, new_content)?;
    }

    Ok(found)
}

pub fn remove_fish_integration() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
    let fish_function_path = home.join(".config/fish/functions/nlsh-rs.fish");

    if fish_function_path.exists() {
        fs::remove_file(&fish_function_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn remove_bash_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
    let completion_path = home.join(".local/share/bash-completion/completions/nlsh-rs");

    if completion_path.exists() {
        fs::remove_file(&completion_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn remove_zsh_completion_file() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
    let completion_path = home.join(".local/share/zsh/site-functions/_nlsh-rs");

    if completion_path.exists() {
        fs::remove_file(&completion_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn remove_zsh_fpath_from_zshrc() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
    let zsh_config = home.join(".zshrc");

    if !zsh_config.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&zsh_config)?;
    if !content.contains("# nlsh-rs autocomplete") {
        return Ok(false);
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut skip = false;
    let mut removed = false;

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

    Ok(removed)
}

fn remove_zsh_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let file_removed = remove_zsh_completion_file()?;
    let zshrc_cleaned = remove_zsh_fpath_from_zshrc()?;
    Ok(file_removed || zshrc_cleaned)
}

fn remove_fish_autocomplete() -> Result<bool, Box<dyn std::error::Error>> {
    let home = get_home_dir();
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
