use crate::commands::{AppCommand, AvatarCommandKind, Command, ParsedCommand};
use crate::util::Result;

pub struct AvatarCommand;

impl Command for AvatarCommand {
    fn name(&self) -> &'static str {
        "avatar"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let subcommand = params.first().map(String::as_str).unwrap_or("help");
        match subcommand {
            "set" => {
                let target = params
                    .get(1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("usage: /avatar set <target> <preset>"))?;
                let preset = params
                    .get(2)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("usage: /avatar set <target> <preset>"))?;
                Ok(ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Set {
                    target,
                    preset,
                })))
            }
            "preview" => Ok(ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Preview))),
            "mode" => {
                let mode = params
                    .get(1)
                    .map(String::as_str)
                    .unwrap_or("normal")
                    .to_owned();
                Ok(ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::Mode(mode))))
            }
            "list" => Ok(ParsedCommand::App(AppCommand::Avatar(AvatarCommandKind::List))),
            _ => Err(anyhow::anyhow!(
                "usage: /avatar set <target> <preset> | /avatar preview | /avatar mode <compact|normal|expressive> | /avatar list"
            )),
        }
    }
}
