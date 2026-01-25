use colored::*;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::env;
use std::io;
use std::sync::Mutex;

static EDITOR: Mutex<Option<DefaultEditor>> = Mutex::new(None);

pub fn get_user_input() -> Result<Option<String>, io::Error> {
    let mut editor_lock = EDITOR.lock().unwrap();
    let editor = editor_lock.get_or_insert_with(|| DefaultEditor::new().unwrap());

    let cwd = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string());

    let prompt = format!(
        "{}:{}{} ",
        "nlsh-rs".cyan().bold(),
        cwd.custom_color((164, 164, 164)).bold(),
        "$".bold()
    );

    match editor.readline(&prompt) {
        Ok(line) => {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let _ = editor.add_history_entry(&line);
                Ok(Some(trimmed.to_string()))
            } else {
                Ok(None)
            }
        }
        Err(ReadlineError::Interrupted) => {
            eprint!("\x1b[?25h");
            std::process::exit(130);
        }
        Err(ReadlineError::Eof) => {
            eprint!("\x1b[?25h");
            std::process::exit(0);
        }
        Err(err) => Err(io::Error::other(err)),
    }
}
