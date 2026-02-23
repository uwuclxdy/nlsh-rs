use colored::*;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io;
use std::sync::Mutex;

use crate::common::{EXIT_SIGINT, exit_with_code, get_current_directory, show_cursor};

static EDITOR: Mutex<Option<DefaultEditor>> = Mutex::new(None);

fn with_editor<F>(readline_fn: F) -> Result<Option<String>, io::Error>
where
    F: FnOnce(&mut DefaultEditor, &str) -> rustyline::Result<String>,
{
    let mut editor_lock = EDITOR.lock().unwrap();
    let editor = editor_lock.get_or_insert_with(|| DefaultEditor::new().unwrap());
    let cwd = get_current_directory();
    let prompt = format!(
        "{}:{}{} ",
        "nlsh-rs".cyan().bold(),
        cwd.custom_color((164, 164, 164)).bold(),
        ">".bold()
    );
    match readline_fn(editor, &prompt) {
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
            exit_with_code(EXIT_SIGINT);
        }
        Err(ReadlineError::Eof) => {
            show_cursor();
            exit_with_code(0);
        }
        Err(err) => Err(io::Error::other(err)),
    }
}

pub fn get_user_input_prefilled(initial: &str) -> Result<Option<String>, io::Error> {
    with_editor(|editor, prompt| editor.readline_with_initial(prompt, (initial, "")))
}

pub fn get_user_input() -> Result<Option<String>, io::Error> {
    with_editor(|editor, prompt| editor.readline(prompt))
}
