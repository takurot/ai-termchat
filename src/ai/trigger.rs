use std::time::{Duration, Instant};

use crate::state::{AiFrequency, AiMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerConfig {
    pub cooldown: Duration,
    pub min_human_streak: usize,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self { cooldown: Duration::from_secs(30), min_human_streak: 3 }
    }
}

impl TriggerConfig {
    pub fn from_frequency(frequency: AiFrequency) -> Self {
        match frequency {
            AiFrequency::Low => Self { cooldown: Duration::from_secs(45), min_human_streak: 4 },
            AiFrequency::Normal => Self::default(),
            AiFrequency::High => Self { cooldown: Duration::from_secs(15), min_human_streak: 2 },
        }
    }
}

pub fn should_intervene(
    mode: AiMode,
    config: &TriggerConfig,
    ai_thinking: bool,
    last_ai_at: Option<Instant>,
    human_streak: usize,
    now: Instant,
) -> bool {
    if ai_thinking || matches!(mode, AiMode::Listener) {
        return false;
    }
    if human_streak < config.min_human_streak {
        return false;
    }
    if let Some(last_ai_at) = last_ai_at {
        if now.duration_since(last_ai_at) < config.cooldown {
            return false;
        }
    }
    true
}
