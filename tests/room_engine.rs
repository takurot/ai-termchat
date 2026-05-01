use triadchat::room::{MemberKind, RoomEngine};
use triadchat::state::AiMode;

#[test]
fn room_engine_creates_room_with_ai_member() {
    let mut engine = RoomEngine::default();
    let room = engine.create_room("takuro", &["tanaka"], Some(AiMode::Clerk));

    assert_eq!(room.members.len(), 3);
    assert!(room
        .members
        .iter()
        .any(|member| member.id == "takuro" && member.kind == MemberKind::Human));
    assert!(room
        .members
        .iter()
        .any(|member| member.id == "tanaka" && member.kind == MemberKind::Human));
    assert!(room
        .members
        .iter()
        .any(|member| member.id == "ops-ai" && member.kind == MemberKind::Ai));
}

#[test]
fn room_engine_lists_room_members_in_stable_order() {
    let mut engine = RoomEngine::default();
    let room = engine.create_room("takuro", &["tanaka", "sato"], Some(AiMode::Moderator));

    let member_ids = room.members.iter().map(|member| member.id.as_str()).collect::<Vec<_>>();

    assert_eq!(member_ids, vec!["takuro", "tanaka", "sato", "ops-ai"]);
    assert_eq!(room.members[3].ai_mode, Some(AiMode::Moderator));
}

#[test]
fn remote_room_preserves_ai_mode_from_creator() {
    let mut engine = RoomEngine::default();
    let room = engine.create_remote_room(
        "room-1",
        &["takuro".into(), "ops-ai".into()],
        Some(AiMode::Clerk),
    );

    assert_eq!(room.ai_mode, Some(AiMode::Clerk));
    assert_eq!(room.members[1].kind, MemberKind::Ai);
    assert_eq!(room.members[1].ai_mode, Some(AiMode::Clerk));
}

#[test]
fn room_ids_do_not_collide_across_independent_engines() {
    let mut alice_engine = RoomEngine::default();
    let mut bob_engine = RoomEngine::default();

    let alice_room = alice_engine.create_room("alice", &[], None);
    let bob_room = bob_engine.create_room("bob", &[], None);

    assert_ne!(alice_room.id, bob_room.id);
}
