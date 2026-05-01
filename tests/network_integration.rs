use std::time::{Duration, Instant};

use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::message::{AiIntent, AiPayload, NetMessage, StructuredOutput};
use triadchat::state::AiMode;

fn test_config(user_name: &str, discovery_port: u16) -> Config {
    Config {
        user_name: user_name.to_string(),
        discovery_addr: format!("239.255.0.1:{discovery_port}").parse().unwrap(),
        terminal_bell: false,
        ..Config::default()
    }
}

fn pump_until<F>(
    left: &mut Application<'_>,
    right: &mut Application<'_>,
    timeout: Duration,
    mut predicate: F,
) where
    F: FnMut(&Application<'_>, &Application<'_>) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate(left, right) {
            return;
        }

        let _ = left.process_next_event_with_timeout_for_test(Duration::from_millis(50));
        let _ = right.process_next_event_with_timeout_for_test(Duration::from_millis(50));
    }

    if predicate(left, right) {
        return;
    }

    panic!(
        "timed out waiting for integration condition: left peers={:?} rooms={:?}, right peers={:?} rooms={:?}",
        left.state().peers().values().map(|peer| peer.user_name.clone()).collect::<Vec<_>>(),
        left.state().rooms().iter().map(|room| room.id.clone()).collect::<Vec<_>>(),
        right.state().peers().values().map(|peer| peer.user_name.clone()).collect::<Vec<_>>(),
        right.state().rooms().iter().map(|room| room.id.clone()).collect::<Vec<_>>(),
    );
}

#[test]
fn peer_handshake_and_room_create_propagates() {
    let discovery_port = 30000 + (rand::random::<u16>() % 3000);
    let takuro_config = test_config("takuro", discovery_port);
    let tanaka_config = test_config("tanaka", discovery_port);
    let mut takuro = Application::new_for_test(&takuro_config).unwrap();
    let mut tanaka = Application::new_for_test(&tanaka_config).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();
    takuro.connect_peer_for_test(tanaka.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(5), |left, right| {
        left.state().peer_names().len() == 1
            && right.state().peer_names().len() == 1
            && left.state().peer_is_ready("tanaka")
            && right.state().peer_is_ready("takuro")
    });

    takuro.handle_input_line_for_test("/room create @tanaka --ai clerk").unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(5), |left, right| {
        left.state().rooms().len() == 1
            && right.state().rooms().len() == 1
            && left.state().active_room_id().is_some()
            && left.state().active_room_id() == right.state().active_room_id()
    });

    let takuro_room = &takuro.state().rooms()[0];
    assert!(takuro_room.members.iter().any(|member| member.id == "ops-ai"));
    assert_eq!(takuro_room.ai_mode, Some(AiMode::Clerk));
    assert_eq!(tanaka.state().rooms()[0].ai_mode, Some(AiMode::Clerk));
    assert_eq!(
        tanaka.state().peers().values().next().map(|peer| peer.user_name.as_str()),
        Some("takuro")
    );
}

#[test]
fn room_and_peer_commands_show_richer_metadata() {
    let discovery_port = 33000 + (rand::random::<u16>() % 3000);
    let takuro_config = test_config("takuro", discovery_port);
    let tanaka_config = test_config("tanaka", discovery_port);
    let sato_config = test_config("sato", discovery_port);
    let mut takuro = Application::new_for_test(&takuro_config).unwrap();
    let mut tanaka = Application::new_for_test(&tanaka_config).unwrap();
    let mut sato = Application::new_for_test(&sato_config).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();
    sato.start_network_for_test().unwrap();
    takuro.connect_peer_for_test(tanaka.local_server_port_for_test().unwrap()).unwrap();
    takuro.connect_peer_for_test(sato.local_server_port_for_test().unwrap()).unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if takuro.state().peers().len() == 2
            && tanaka.state().peers().len() == 1
            && sato.state().peers().len() == 1
            && takuro.state().peer_is_ready("tanaka")
            && takuro.state().peer_is_ready("sato")
            && tanaka.state().peer_is_ready("takuro")
            && sato.state().peer_is_ready("takuro")
        {
            break;
        }
        let _ = takuro.process_next_event_with_timeout_for_test(Duration::from_millis(50));
        let _ = tanaka.process_next_event_with_timeout_for_test(Duration::from_millis(50));
        let _ = sato.process_next_event_with_timeout_for_test(Duration::from_millis(50));
    }

    assert_eq!(
        takuro.state().peers().len(),
        2,
        "takuro peers: {:?}",
        takuro.state().peers().values().map(|peer| peer.user_name.clone()).collect::<Vec<_>>()
    );

    takuro.handle_input_line_for_test("/room create @tanaka --ai clerk").unwrap();
    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(5), |left, right| {
        left.state().rooms().len() == 1
            && right.state().rooms().len() == 1
            && left.state().active_room_id() == right.state().active_room_id()
    });

    takuro.handle_input_line_for_test("/peers").unwrap();
    takuro.handle_input_line_for_test("/room list").unwrap();
    takuro.handle_input_line_for_test("/room switch 1").unwrap();

    let rendered = takuro
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Connected peers (2):"), "{rendered}");
    assert!(rendered.contains("tanaka  [in room, ready, untrusted]"), "{rendered}");
    assert!(rendered.contains("sato  [available, ready, untrusted]"), "{rendered}");
    assert!(rendered.contains("Rooms (1):"), "{rendered}");
    assert!(rendered.contains("1"), "{rendered}");
    assert!(rendered.contains("mode: clerk"), "{rendered}");
    assert!(rendered.contains("active"), "{rendered}");
    let room_id = takuro.state().active_room_id().unwrap().to_string();
    assert!(rendered.contains(&room_id), "expected room id {room_id} in rendered output: {rendered}");
    assert!(
        rendered.contains(&format!("Switched to {room_id} [takuro, tanaka]")),
        "{rendered}"
    );
    assert!(rendered.contains("AI: clerk"), "{rendered}");
}

#[test]
fn room_switch_rejects_zero_index() {
    let discovery_port = 36000 + (rand::random::<u16>() % 3000);
    let takuro_config = test_config("takuro", discovery_port);
    let tanaka_config = test_config("tanaka", discovery_port);
    let mut takuro = Application::new_for_test(&takuro_config).unwrap();
    let mut tanaka = Application::new_for_test(&tanaka_config).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();
    takuro.connect_peer_for_test(tanaka.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(5), |left, right| {
        left.state().peer_names().len() == 1
            && right.state().peer_names().len() == 1
            && left.state().peer_is_ready("tanaka")
            && right.state().peer_is_ready("takuro")
    });

    takuro.handle_input_line_for_test("/room create @tanaka --ai clerk").unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(5), |left, right| {
        left.state().rooms().len() == 1
            && right.state().rooms().len() == 1
            && left.state().active_room_id() == right.state().active_room_id()
    });

    let active_room =
        takuro.state().active_room_id().expect("active room should exist").to_string();
    takuro.handle_input_line_for_test("/room switch 0").unwrap();

    let rendered =
        takuro.state().messages().last().expect("switch error should be rendered").rendered_text();

    assert_eq!(rendered, "unknown room id: 0");
    assert_eq!(takuro.state().active_room_id(), Some(active_room.as_str()));
}

#[test]
fn peer_connect_command_bootstraps_room_creation_without_multicast() {
    let discovery_port = 41000 + (rand::random::<u16>() % 1000);
    let takuro_config = test_config("takuro", discovery_port);
    let tanaka_config = test_config("tanaka", discovery_port);
    let mut takuro = Application::new_for_test(&takuro_config).unwrap();
    let mut tanaka = Application::new_for_test(&tanaka_config).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();

    takuro
        .handle_input_line_for_test(&format!(
            "/peer connect 127.0.0.1:{}",
            tanaka.local_server_port_for_test().unwrap()
        ))
        .unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1
            && right.state().peers().len() == 1
            && left.state().peer_is_ready("tanaka")
            && right.state().peer_is_ready("takuro")
    });

    takuro.handle_input_line_for_test("/room create @tanaka --ai clerk").unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().rooms().len() == 1
            && right.state().rooms().len() == 1
            && left.state().active_room_id() == right.state().active_room_id()
    });

    assert_eq!(tanaka.state().rooms()[0].ai_mode, Some(AiMode::Clerk));
}

#[test]
fn remote_skill_proposals_require_explicit_trust_before_run() {
    let workspace = tempfile::TempDir::new().unwrap();
    let skills_dir = workspace.path().join(".claude/skills/review-auth");
    std::fs::create_dir_all(&skills_dir).unwrap();
    std::fs::write(
        skills_dir.join("SKILL.md"),
        r#"---
name: review-auth
invoke: auto_safe
risk: low
description: Review auth
---
"#,
    )
    .unwrap();

    let script_path = workspace.path().join("mock-claude.sh");
    std::fs::write(&script_path, "#!/bin/sh\nprintf 'review-auth executed'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();
    }

    let discovery_port = 42000 + (rand::random::<u16>() % 1000);
    let mut takuro_config = test_config("takuro", discovery_port);
    takuro_config.ai.command = Some(script_path.display().to_string());
    takuro_config.security.default_permission = "confirm-required".into();
    let mut tanaka_config = test_config("tanaka", discovery_port);
    tanaka_config.ai.command = Some(script_path.display().to_string());
    let mut takuro =
        Application::new_for_test_in_workspace(&takuro_config, workspace.path()).unwrap();
    let mut tanaka =
        Application::new_for_test_in_workspace(&tanaka_config, workspace.path()).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();
    takuro.connect_peer_for_test(tanaka.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1
            && right.state().peers().len() == 1
            && left.state().peer_is_ready("tanaka")
            && right.state().peer_is_ready("takuro")
    });

    let endpoint = tanaka.state().all_user_endpoints()[0];
    let payload = AiPayload {
        text: "review-auth suggested".into(),
        intent: AiIntent::SkillSuggest,
        structured: Some(StructuredOutput {
            todos: Vec::new(),
            decisions: Vec::new(),
            skill_suggestions: vec!["review-auth".into()],
            raw_text: None,
        }),
    };
    let mut encoded = Vec::new();
    bincode::serialize_into(&mut encoded, &NetMessage::AiMessage(payload)).unwrap();
    tanaka.node_handler().network().send(endpoint, &encoded);
    takuro.process_next_event_with_timeout_for_test(Duration::from_secs(1)).unwrap();

    takuro.handle_input_line_for_test("/run 1").unwrap();
    let rendered = takuro.state().messages().last().unwrap().rendered_text();
    assert!(rendered.contains("Use /trust add tanaka"));

    takuro.handle_input_line_for_test("/trust add tanaka").unwrap();
    takuro.handle_input_line_for_test("/run 1").unwrap();
    assert!(takuro.state().pending_confirmation().is_some());

    takuro.handle_confirmation_input_for_test('y').unwrap();

    let transcript = takuro
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(transcript.contains("review-auth executed"));
}

#[test]
fn trust_remove_by_fingerprint_blocks_disconnected_peer_proposals() {
    let workspace = tempfile::TempDir::new().unwrap();
    let skills_dir = workspace.path().join(".claude/skills/review-auth");
    std::fs::create_dir_all(&skills_dir).unwrap();
    std::fs::write(
        skills_dir.join("SKILL.md"),
        r#"---
name: review-auth
invoke: auto_safe
risk: low
description: Review auth
---
"#,
    )
    .unwrap();

    let script_path = workspace.path().join("mock-claude.sh");
    std::fs::write(&script_path, "#!/bin/sh\nprintf 'review-auth executed'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();
    }

    let discovery_port = 43000 + (rand::random::<u16>() % 1000);
    let mut takuro_config = test_config("takuro", discovery_port);
    takuro_config.ai.command = Some(script_path.display().to_string());
    takuro_config.security.default_permission = "confirm-required".into();
    let mut tanaka_config = test_config("tanaka", discovery_port);
    tanaka_config.ai.command = Some(script_path.display().to_string());
    let mut takuro =
        Application::new_for_test_in_workspace(&takuro_config, workspace.path()).unwrap();
    let mut tanaka =
        Application::new_for_test_in_workspace(&tanaka_config, workspace.path()).unwrap();

    takuro.start_network_for_test().unwrap();
    tanaka.start_network_for_test().unwrap();
    takuro.connect_peer_for_test(tanaka.local_server_port_for_test().unwrap()).unwrap();

    pump_until(&mut takuro, &mut tanaka, Duration::from_secs(3), |left, right| {
        left.state().peers().len() == 1
            && right.state().peers().len() == 1
            && left.state().peer_is_ready("tanaka")
            && right.state().peer_is_ready("takuro")
    });

    let fingerprint = takuro.state().peer_fingerprint_by_name("tanaka").unwrap();
    let endpoint = tanaka.state().all_user_endpoints()[0];
    let payload = AiPayload {
        text: "review-auth suggested".into(),
        intent: AiIntent::SkillSuggest,
        structured: Some(StructuredOutput {
            todos: Vec::new(),
            decisions: Vec::new(),
            skill_suggestions: vec!["review-auth".into()],
            raw_text: None,
        }),
    };
    let mut encoded = Vec::new();
    bincode::serialize_into(&mut encoded, &NetMessage::AiMessage(payload)).unwrap();
    tanaka.node_handler().network().send(endpoint, &encoded);
    takuro.process_next_event_with_timeout_for_test(Duration::from_secs(1)).unwrap();

    takuro.handle_input_line_for_test("/trust add tanaka").unwrap();
    drop(tanaka);

    let disconnect_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < disconnect_deadline && !takuro.state().peers().is_empty() {
        let _ = takuro.process_next_event_with_timeout_for_test(Duration::from_millis(50));
    }
    assert!(takuro.state().peers().is_empty());

    takuro.handle_input_line_for_test(&format!("/trust remove {fingerprint}")).unwrap();
    takuro.handle_input_line_for_test("/run 1").unwrap();

    let transcript = takuro
        .state()
        .messages()
        .iter()
        .map(|message| message.rendered_text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(transcript.contains("removed trust for"));
    assert!(transcript.contains("permission denied: proposal 1 came from untrusted peer tanaka"));
    assert!(takuro.state().pending_confirmation().is_none());
    assert!(!transcript.contains("review-auth executed"));
}
