use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::state::{AiFrequency, AiMode};
use crate::util::Result;

pub struct AiCommand;

impl Command for AiCommand {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let subcommand = params.first().map(String::as_str).unwrap_or("help");
        match subcommand {
            "mode" => {
                let mode = params
                    .get(1)
                    .map(String::as_str)
                    .unwrap_or("clerk")
                    .parse::<AiMode>()?;
                Ok(ParsedCommand::App(AppCommand::SetAiMode(mode)))
            }
            "quiet" => {
                let enabled = match params.get(1).map(String::as_str).unwrap_or("on") {
                    "on" => true,
                    "off" => false,
                    other => return Err(anyhow::anyhow!("unknown quiet value: {other}")),
                };
                Ok(ParsedCommand::App(AppCommand::SetAiQuiet(enabled)))
            }
            "freq" => {
                let frequency =
                    parse_frequency(params.get(1).map(String::as_str).unwrap_or("normal"))?;
                Ok(ParsedCommand::App(AppCommand::SetAiFrequency(frequency)))
            }
            _ => Err(anyhow::anyhow!(
                "usage: /ai mode <mode>\n  clerk      - responds to decisions and task markers\n  listener   - silent; never auto-intervenes\n  moderator  - intervenes on ambiguous or contradictory messages\n  operator   - responds to execute, deploy, or run requests\n  companion  - chats actively and replies to direct prompts\nusage: /ai quiet <on|off>\nusage: /ai freq <low|normal|high>"
            )),
        }
    }
}

fn parse_frequency(value: &str) -> Result<AiFrequency> {
    match value {
        "low" => Ok(AiFrequency::Low),
        "normal" => Ok(AiFrequency::Normal),
        "high" => Ok(AiFrequency::High),
        other => Err(anyhow::anyhow!("unknown ai frequency: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Command;

    #[test]
    fn parse_params_companion_mode_returns_companion() {
        let parsed = AiCommand
            .parse_params(vec!["mode".into(), "companion".into()])
            .expect("companion is a valid AI mode");

        assert!(matches!(parsed, ParsedCommand::App(AppCommand::SetAiMode(AiMode::Companion))));
    }

    #[test]
    fn parse_params_unknown_mode_returns_error() {
        assert!(AiCommand.parse_params(vec!["mode".into(), "unknown_mode".into()]).is_err());
    }

    #[test]
    fn parse_params_error_describes_available_modes() {
        let error = AiCommand
            .parse_params(vec!["wat".into()])
            .err()
            .expect("invalid subcommand should error");

        assert!(error.to_string().contains("usage: /ai mode <mode>"));
        assert!(error.to_string().contains("clerk"));
        assert!(error.to_string().contains("listener"));
        assert!(error.to_string().contains("moderator"));
        assert!(error.to_string().contains("operator"));
    }
}
