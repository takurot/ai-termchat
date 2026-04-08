use std::time::Duration;

use crate::ai::AiMediator;
use crate::message::SkillResultPayload;
use crate::skill::registry::SkillMeta;

#[derive(Clone, Debug)]
pub struct PendingSkillExecution {
    pub meta: SkillMeta,
    pub args: Vec<String>,
}

pub struct SkillExecutor;

impl SkillExecutor {
    pub async fn run(
        ai_mediator: &AiMediator,
        meta: &SkillMeta,
        args: &[String],
    ) -> SkillResultPayload {
        match tokio::time::timeout(Duration::from_secs(60), ai_mediator.run_skill(&meta.name, args))
            .await
        {
            Ok(Ok(summary)) => {
                SkillResultPayload { skill_name: meta.name.clone(), summary, success: true }
            }
            Ok(Err(error)) => SkillResultPayload {
                skill_name: meta.name.clone(),
                summary: error.to_string(),
                success: false,
            },
            Err(_) => SkillResultPayload {
                skill_name: meta.name.clone(),
                summary: timeout_summary(meta),
                success: false,
            },
        }
    }
}

fn timeout_summary(meta: &SkillMeta) -> String {
    format!(
        "Skill '{}' timed out after 60s. Check that the skill script is executable and not hanging.",
        meta.name
    )
}

#[cfg(test)]
mod tests {
    use super::timeout_summary;
    use crate::skill::registry::{InvokeMode, RiskLevel, SkillMeta, SkillScope};
    use std::path::PathBuf;

    #[test]
    fn timeout_summary_includes_guidance() {
        let meta = SkillMeta {
            name: "review-auth".into(),
            scope: SkillScope::Workspace,
            invoke_mode: InvokeMode::Confirm,
            allowed_tools: Vec::new(),
            risk: RiskLevel::Medium,
            description: "Review auth".into(),
            args_hint: None,
            path: PathBuf::from("SKILL.md"),
        };

        let summary = timeout_summary(&meta);
        assert!(summary.contains("Skill 'review-auth' timed out after 60s"));
        assert!(summary.contains("not hanging"));
    }
}
