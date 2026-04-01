use std::borrow::Cow;
use std::io;
use std::sync::Mutex;

use colored::*;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Editor, Helper};

use crate::common::{EXIT_SIGINT, exit_with_code, get_current_directory, show_cursor};

pub struct NlshHelper;

impl Helper for NlshHelper {}

impl Completer for NlshHelper {
    type Candidate = String;
}

impl Hinter for NlshHelper {
    type Hint = String;
}

impl Validator for NlshHelper {}

impl Highlighter for NlshHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if !line.starts_with('/') {
            return Cow::Borrowed(line);
        }
        Cow::Owned(line.cyan().bold().to_string())
    }

    fn highlight_char(&self, line: &str, _pos: usize, _kind: CmdKind) -> bool {
        line.starts_with('/')
    }
}

type NlshEditor = Editor<NlshHelper, DefaultHistory>;

static EDITOR: Mutex<Option<NlshEditor>> = Mutex::new(None);

fn with_editor<F>(readline_fn: F) -> Result<Option<String>, io::Error>
where
    F: FnOnce(&mut NlshEditor, &str) -> rustyline::Result<String>,
{
    let mut editor_lock = EDITOR.lock().unwrap();
    let editor = editor_lock.get_or_insert_with(|| {
        let mut ed = Editor::<NlshHelper, DefaultHistory>::new().unwrap();
        ed.set_helper(Some(NlshHelper));
        ed
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::highlight::{CmdKind, Highlighter};

    #[test]
    fn highlight_slash_prefix_colors_typed_part() {
        colored::control::set_override(true);
        let helper = NlshHelper;
        let result = helper.highlight("/pr", 3);
        assert!(result.contains("\x1b["), "expected ANSI codes in: {result}");
        assert!(result.contains("/pr"), "typed part must appear in output");
    }

    #[test]
    fn highlight_non_slash_line_is_unchanged() {
        let helper = NlshHelper;
        let result = helper.highlight("list files", 10);
        assert_eq!(result.as_ref(), "list files");
    }

    #[test]
    fn highlight_char_true_for_slash_line() {
        let helper = NlshHelper;
        assert!(helper.highlight_char("/api", 4, CmdKind::Other));
    }

    #[test]
    fn highlight_char_false_for_normal_line() {
        let helper = NlshHelper;
        assert!(!helper.highlight_char("list files", 10, CmdKind::Other));
    }
}
