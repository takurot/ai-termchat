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

    pub fn from_frequency_and_mode(frequency: AiFrequency, mode: AiMode) -> Self {
        match frequency {
            AiFrequency::Low => Self { cooldown: Duration::from_secs(45), human_streak_limit: 4 },
            AiFrequency::Normal => match mode {
                AiMode::Companion => Self { cooldown: Duration::from_secs(10), human_streak_limit: 6 },
                _ => Self::default(),
            },
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

    // @ops-ai mention bypasses mode and cooldown checks
    if input.contains("@ops-ai") {
        return true;
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
        AiMode::Companion => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn companion_always_intervenes_after_cooldown() {
        let config = TriggerConfig { cooldown: Duration::from_secs(10), human_streak_limit: 6 };
        let past = now() - Duration::from_secs(11);
        assert!(should_intervene("any message here", AiMode::Companion, &config, false, Some(past), 0, now()));
    }

    #[test]
    fn companion_blocked_while_ai_thinking() {
        let config = TriggerConfig { cooldown: Duration::from_secs(10), human_streak_limit: 6 };
        let past = now() - Duration::from_secs(11);
        assert!(!should_intervene("any message", AiMode::Companion, &config, true, Some(past), 0, now()));
    }

    #[test]
    fn companion_blocked_during_cooldown() {
        let config = TriggerConfig { cooldown: Duration::from_secs(10), human_streak_limit: 6 };
        let recent = now() - Duration::from_secs(5);
        assert!(!should_intervene("any message", AiMode::Companion, &config, false, Some(recent), 0, now()));
    }

    #[test]
    fn companion_blocked_at_streak_limit() {
        let config = TriggerConfig { cooldown: Duration::from_secs(10), human_streak_limit: 6 };
        let past = now() - Duration::from_secs(11);
        assert!(!should_intervene("any message", AiMode::Companion, &config, false, Some(past), 6, now()));
    }

    #[test]
    fn companion_config_normal_freq_has_10s_cooldown() {
        let config = TriggerConfig::from_frequency_and_mode(AiFrequency::Normal, AiMode::Companion);
        assert_eq!(config.cooldown, Duration::from_secs(10));
        assert_eq!(config.human_streak_limit, 6);
    }

    #[test]
    fn mention_bypasses_cooldown_and_mode() {
        let config = TriggerConfig::default();
        // Still in cooldown
        let recent = now() - Duration::from_secs(5);
        assert!(should_intervene("@ops-ai what do you think?", AiMode::Listener, &config, false, Some(recent), 0, now()));
    }

    #[test]
    fn mention_blocked_while_ai_thinking() {
        let config = TriggerConfig::default();
        let recent = now() - Duration::from_secs(5);
        assert!(!should_intervene("@ops-ai help me", AiMode::Listener, &config, true, Some(recent), 0, now()));
    }

    #[test]
    fn clerk_still_requires_keywords() {
        let config = TriggerConfig::default();
        let past = now() - Duration::from_secs(31);
        assert!(!should_intervene("just chatting", AiMode::Clerk, &config, false, Some(past), 0, now()));
    }
}
