use crate::common::get_current_directory;
use crate::common::show_cursor;
use colored::*;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io;
use std::sync::Mutex;

static EDITOR: Mutex<Option<DefaultEditor>> = Mutex::new(None);

pub fn get_user_input() -> Result<Option<String>, io::Error> {
    let mut editor_lock = EDITOR.lock().unwrap();
    let editor = editor_lock.get_or_insert_with(|| DefaultEditor::new().unwrap());
    let cwd = get_current_directory();

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
            show_cursor();
            std::process::exit(130);
        }
        Err(ReadlineError::Eof) => {
            show_cursor();
            std::process::exit(0);
        }
        Err(err) => Err(io::Error::other(err)),
    }
}
