use crate::cli::{PromptAction, PromptKind};

pub struct SlashCommand {
    pub name: &'static str,
    #[allow(dead_code)] // used in Task 4: preview drawing
    pub description: &'static str,
}

pub const COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "api",
        description: "configure API provider",
    },
    SlashCommand {
        name: "explain",
        description: "explain a shell command",
    },
    SlashCommand {
        name: "prompt",
        description: "manage system or explain prompts",
    },
    SlashCommand {
        name: "quit",
        description: "exit interactive mode",
    },
    SlashCommand {
        name: "uninstall",
        description: "uninstall nlsh-rs",
    },
];

/// Returns commands whose `/name` is a prefix of `typed`, or `typed` is a prefix of `/name`.
/// Returns nothing if `typed` is empty or does not start with `/`.
pub fn filter(typed: &str) -> Vec<&'static SlashCommand> {
    if !typed.starts_with('/') {
        return vec![];
    }
    // Extract just the command part (first word, strip leading '/')
    let typed_cmd = typed[1..].split_whitespace().next().unwrap_or("");
    COMMANDS
        .iter()
        .filter(|cmd| cmd.name.starts_with(typed_cmd) || typed_cmd.starts_with(cmd.name))
        .collect()
}

#[derive(Debug)]
pub enum SlashCmd {
    Api,
    Uninstall,
    Prompt {
        kind: PromptKind,
        action: PromptAction,
    },
    Explain {
        args: Vec<String>,
    },
    Quit,
    Unknown(String),
}

pub fn parse(input: &str) -> SlashCmd {
    let mut parts = input.split_whitespace();
    match parts.next() {
        Some("/api") => SlashCmd::Api,
        Some("/uninstall") => SlashCmd::Uninstall,
        Some("/quit") => SlashCmd::Quit,
        Some("/explain") => SlashCmd::Explain {
            args: parts.map(|s| s.to_string()).collect(),
        },
        Some("/prompt") => {
            let kind = match parts.next() {
                Some("explain") => PromptKind::Explain,
                _ => PromptKind::System,
            };
            let action = match parts.next() {
                Some("edit") => PromptAction::Edit,
                _ => PromptAction::Show,
            };
            SlashCmd::Prompt { kind, action }
        }
        _ => SlashCmd::Unknown(input.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_empty_input_returns_nothing() {
        assert!(filter("").is_empty());
    }

    #[test]
    fn filter_slash_returns_all_commands() {
        assert_eq!(filter("/").len(), COMMANDS.len());
    }

    #[test]
    fn filter_prefix_matches_command() {
        let results = filter("/p");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "prompt");
    }

    #[test]
    fn filter_full_name_matches_self() {
        let results = filter("/quit");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "quit");
    }

    #[test]
    fn filter_with_args_matches_base_command() {
        let results = filter("/prompt system edit");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "prompt");
    }

    #[test]
    fn filter_unknown_returns_nothing() {
        assert!(filter("/zzz").is_empty());
    }

    #[test]
    fn parse_quit() {
        assert!(matches!(parse("/quit"), SlashCmd::Quit));
    }

    #[test]
    fn parse_api() {
        assert!(matches!(parse("/api"), SlashCmd::Api));
    }

    #[test]
    fn parse_uninstall() {
        assert!(matches!(parse("/uninstall"), SlashCmd::Uninstall));
    }

    #[test]
    fn parse_explain_with_args() {
        let cmd = parse("/explain ls -la");
        match cmd {
            SlashCmd::Explain { args } => assert_eq!(args, vec!["ls", "-la"]),
            _ => panic!("expected Explain"),
        }
    }

    #[test]
    fn parse_prompt_defaults_to_system_show() {
        match parse("/prompt") {
            SlashCmd::Prompt { kind, action } => {
                assert!(matches!(kind, crate::cli::PromptKind::System));
                assert!(matches!(action, crate::cli::PromptAction::Show));
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_prompt_explain_edit() {
        match parse("/prompt explain edit") {
            SlashCmd::Prompt { kind, action } => {
                assert!(matches!(kind, crate::cli::PromptKind::Explain));
                assert!(matches!(action, crate::cli::PromptAction::Edit));
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_unknown_command() {
        match parse("/foo") {
            SlashCmd::Unknown(s) => assert_eq!(s, "/foo"),
            _ => panic!("expected Unknown"),
        }
    }
}
