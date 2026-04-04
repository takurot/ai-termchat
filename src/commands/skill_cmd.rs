use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::util::Result;

pub struct SkillsCommand;
pub struct SkillCommand;
pub struct RunCommand;
pub struct CancelCommand;

impl Command for SkillsCommand {
    fn name(&self) -> &'static str {
        "skills"
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<ParsedCommand> {
        Ok(ParsedCommand::App(AppCommand::Skills))
    }
}

impl Command for SkillCommand {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let name =
            params.first().ok_or_else(|| anyhow::anyhow!("usage: /skill <name> [args]"))?.clone();
        Ok(ParsedCommand::App(AppCommand::Skill { name, args: params[1..].to_vec() }))
    }
}

impl Command for RunCommand {
    fn name(&self) -> &'static str {
        "run"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let id = params
            .first()
            .ok_or_else(|| anyhow::anyhow!("usage: /run <proposal_id>"))?
            .parse::<usize>()?;
        Ok(ParsedCommand::App(AppCommand::RunProposal(id)))
    }
}

impl Command for CancelCommand {
    fn name(&self) -> &'static str {
        "cancel"
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<ParsedCommand> {
        Ok(ParsedCommand::App(AppCommand::Cancel))
    }
}
