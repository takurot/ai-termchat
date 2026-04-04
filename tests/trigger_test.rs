use std::time::{Duration, Instant};

use triadchat::ai::trigger::{should_intervene, TriggerConfig};
use triadchat::state::AiMode;

#[test]
fn trigger_rejects_when_ai_is_already_thinking() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Clerk,
        &config,
        true,
        None,
        1,
        Instant::now(),
    ));
}

#[test]
fn trigger_rejects_during_cooldown() {
    let config = TriggerConfig::default();
    let now = Instant::now();
    assert!(!should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Clerk,
        &config,
        false,
        Some(now - Duration::from_secs(5)),
        1,
        now,
    ));
}

#[test]
fn trigger_rejects_once_human_streak_reaches_limit() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Clerk,
        &config,
        false,
        None,
        3,
        Instant::now(),
    ));
}

#[test]
fn trigger_respects_listener_mode() {
    let config = TriggerConfig::default();
    assert!(!should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Listener,
        &config,
        false,
        None,
        1,
        Instant::now(),
    ));
}

#[test]
fn trigger_accepts_clerk_mode_when_all_conditions_pass() {
    let config = TriggerConfig::default();
    assert!(should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Clerk,
        &config,
        false,
        None,
        1,
        Instant::now(),
    ));
}

#[test]
fn trigger_uses_moderator_heuristics() {
    let config = TriggerConfig::default();
    assert!(should_intervene(
        "ここどう分けるのがよさそう？",
        AiMode::Moderator,
        &config,
        false,
        None,
        1,
        Instant::now(),
    ));
    assert!(!should_intervene(
        "takuro が auth の設計を書く",
        AiMode::Moderator,
        &config,
        false,
        None,
        1,
        Instant::now(),
    ));
}

#[test]
fn trigger_uses_operator_heuristics() {
    let config = TriggerConfig::default();
    assert!(should_intervene(
        "/skill review-auth を実行してください",
        AiMode::Operator,
        &config,
        false,
        None,
        1,
        Instant::now(),
    ));
}
