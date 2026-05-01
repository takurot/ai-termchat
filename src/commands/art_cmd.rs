use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::util::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtCommandKind {
    List,
    Reload,
}

pub struct ArtCommand;

impl Command for ArtCommand {
    fn name(&self) -> &'static str {
        "art"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let subcommand = params.first().map(String::as_str).unwrap_or("help");
        match subcommand {
            "list" => Ok(ParsedCommand::App(AppCommand::ArtList)),
            "reload" => Ok(ParsedCommand::App(AppCommand::ArtReload)),
            _ => Err(anyhow::anyhow!("usage: /art list | /art reload")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_list_parses() {
        let cmd = ArtCommand;
        let parsed = cmd.parse_params(vec!["list".into()]).unwrap();
        match parsed {
            ParsedCommand::App(AppCommand::ArtList) => {}
            _ => panic!("expected ArtList, got something else"),
        }
    }

    #[test]
    fn art_reload_parses() {
        let cmd = ArtCommand;
        let parsed = cmd.parse_params(vec!["reload".into()]).unwrap();
        match parsed {
            ParsedCommand::App(AppCommand::ArtReload) => {}
            _ => panic!("expected ArtReload, got something else"),
        }
    }

    #[test]
    fn art_unknown_subcommand_returns_error() {
        let cmd = ArtCommand;
        assert!(cmd.parse_params(vec!["bogus".into()]).is_err());
    }
}
