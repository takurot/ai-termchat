use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::util::Result;

pub struct AcceptCommand;

impl Command for AcceptCommand {
    fn name(&self) -> &'static str {
        "accept"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let filename =
            params.first().ok_or_else(|| anyhow::anyhow!("usage: /accept <filename>"))?.clone();
        Ok(ParsedCommand::App(AppCommand::AcceptTransfer(filename)))
    }
}

pub struct RejectCommand;

impl Command for RejectCommand {
    fn name(&self) -> &'static str {
        "reject"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let filename =
            params.first().ok_or_else(|| anyhow::anyhow!("usage: /reject <filename>"))?.clone();
        Ok(ParsedCommand::App(AppCommand::RejectTransfer(filename)))
    }
}
