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
                summary: "skill timed out after 60s".into(),
                success: false,
            },
        }
    }
}
