use std::time::{Duration, Instant};

use crate::ai::classifier::{
    contains_ambiguity, contains_contradiction, contains_decision_marker, contains_execute_request,
    contains_task_marker,
};
use crate::state::{AiFrequency, AiMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerConfig {
    pub cooldown: Duration,
    pub human_streak_limit: usize,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self { cooldown: Duration::from_secs(30), human_streak_limit: 3 }
    }
}

impl TriggerConfig {
    pub fn from_frequency(frequency: AiFrequency) -> Self {
        match frequency {
            AiFrequency::Low => Self { cooldown: Duration::from_secs(45), human_streak_limit: 4 },
            AiFrequency::Normal => Self::default(),
            AiFrequency::High => Self { cooldown: Duration::from_secs(15), human_streak_limit: 2 },
        }
    }
}

pub fn should_intervene(
    input: &str,
    mode: AiMode,
    config: &TriggerConfig,
    ai_thinking: bool,
    last_ai_at: Option<Instant>,
    human_streak: usize,
    now: Instant,
) -> bool {
    if ai_thinking {
        return false;
    }
    if let Some(last_ai_at) = last_ai_at {
        if now.duration_since(last_ai_at) < config.cooldown {
            return false;
        }
    }
    if human_streak >= config.human_streak_limit {
        return false;
    }

    match mode {
        AiMode::Listener => false,
        AiMode::Clerk => contains_decision_marker(input) || contains_task_marker(input),
        AiMode::Moderator => contains_ambiguity(input) || contains_contradiction(input),
        AiMode::Operator => contains_execute_request(input),
    }
}
