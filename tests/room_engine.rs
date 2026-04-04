use triadchat::room::{MemberKind, RoomEngine};
use triadchat::state::AiMode;

#[test]
fn room_engine_creates_room_with_ai_member() {
    let mut engine = RoomEngine::default();
    let room = engine.create_room("takuro", &["tanaka"], Some(AiMode::Clerk));

    assert_eq!(room.members.len(), 3);
    assert!(room.members.iter().any(|member| member.id == "takuro" && member.kind == MemberKind::Human));
    assert!(room.members.iter().any(|member| member.id == "tanaka" && member.kind == MemberKind::Human));
    assert!(room.members.iter().any(|member| member.id == "ops-ai" && member.kind == MemberKind::Ai));
}

#[test]
fn room_engine_lists_room_members_in_stable_order() {
    let mut engine = RoomEngine::default();
    let room = engine.create_room("takuro", &["tanaka", "sato"], Some(AiMode::Moderator));

    let member_ids = room
        .members
        .iter()
        .map(|member| member.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(member_ids, vec!["takuro", "tanaka", "sato", "ops-ai"]);
    assert_eq!(room.members[3].ai_mode, Some(AiMode::Moderator));
}
