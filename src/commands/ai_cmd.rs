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
                let mode = parse_mode(params.get(1).map(String::as_str).unwrap_or("clerk"))?;
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

fn parse_mode(value: &str) -> Result<AiMode> {
    match value {
        "clerk" => Ok(AiMode::Clerk),
        "listener" => Ok(AiMode::Listener),
        "moderator" => Ok(AiMode::Moderator),
        "operator" => Ok(AiMode::Operator),
        "companion" => Ok(AiMode::Companion),
        other => Err(anyhow::anyhow!("unknown ai mode: {other}")),
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
    fn parse_mode_companion_returns_companion() {
        assert_eq!(parse_mode("companion").unwrap(), AiMode::Companion);
    }

    #[test]
    fn parse_mode_unknown_returns_error() {
        assert!(parse_mode("unknown_mode").is_err());
    }

    #[test]
    fn parse_mode_all_known_modes() {
        assert!(parse_mode("clerk").is_ok());
        assert!(parse_mode("listener").is_ok());
        assert!(parse_mode("moderator").is_ok());
        assert!(parse_mode("operator").is_ok());
        assert!(parse_mode("companion").is_ok());
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
