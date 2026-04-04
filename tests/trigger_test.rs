use std::time::{Duration, Instant};

use triadchat::ai::trigger::{should_intervene, TriggerConfig};
use triadchat::state::AiMode;

#[test]
fn trigger_rejects_when_ai_is_already_thinking() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(AiMode::Clerk, &config, true, None, 3, Instant::now(),));
}

#[test]
fn trigger_rejects_during_cooldown() {
    let config = TriggerConfig::default();
    let now = Instant::now();
    assert!(!should_intervene(
        AiMode::Clerk,
        &config,
        false,
        Some(now - Duration::from_secs(5)),
        3,
        now,
    ));
}

#[test]
fn trigger_requires_enough_human_streak() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(AiMode::Clerk, &config, false, None, 2, Instant::now(),));
}

#[test]
fn trigger_respects_listener_mode() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(AiMode::Listener, &config, false, None, 5, Instant::now(),));
}

#[test]
fn trigger_accepts_clerk_mode_when_all_conditions_pass() {
    let config = TriggerConfig::default();
    assert!(should_intervene(AiMode::Clerk, &config, false, None, 3, Instant::now(),));
}
