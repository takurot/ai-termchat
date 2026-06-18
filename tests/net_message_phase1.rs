use triadchat::message::{AiPayload, Chunk, NetMessage, PeerInfo, SkillResultPayload};
use triadchat::state::AiMode;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum LegacyNetMessage {
    HelloLan(String, u16),
    HelloUser(String),
    UserMessage(String),
    UserData(String, Chunk),
    AiMessage(AiPayload),
    PeerInfo(PeerInfo),
    RoomCreate(String, Vec<String>),
    RoomJoin(String),
    SkillResult(SkillResultPayload),
}

#[test]
fn peer_info_round_trips_through_bincode() {
    let message = NetMessage::PeerInfo(PeerInfo {
        user_name: "takuro".into(),
        server_port: 4000,
        node_version: "0.1.0".into(),
        avatar: "neko".into(),
    });

    let encoded = bincode::serde::encode_to_vec(&message, bincode::config::legacy())
        .expect("message should serialize");
    let (decoded, _): (NetMessage, usize) =
        bincode::serde::decode_from_slice(&encoded, bincode::config::legacy())
            .expect("message should deserialize");

    match decoded {
        NetMessage::PeerInfo(info) => {
            assert_eq!(info.user_name, "takuro");
            assert_eq!(info.server_port, 4000);
            assert_eq!(info.node_version, "0.1.0");
            assert_eq!(info.avatar, "neko");
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn room_and_skill_variants_round_trip_through_bincode() {
    let room_create =
        NetMessage::RoomCreate("room-1".into(), vec!["takuro".into(), "tanaka".into()]);
    let room_create_v2 = NetMessage::RoomCreateV2 {
        room_id: "room-1".into(),
        members: vec!["takuro".into(), "tanaka".into()],
        ai_mode: Some(AiMode::Clerk),
    };
    let room_join = NetMessage::RoomJoin("room-1".into());
    let skill_done = NetMessage::SkillResult(SkillResultPayload {
        skill_name: "review-auth".into(),
        summary: "auth review complete".into(),
        success: true,
    });

    for message in [room_create, room_create_v2, room_join, skill_done] {
        let encoded = bincode::serde::encode_to_vec(&message, bincode::config::legacy())
            .expect("message should serialize");
        let (decoded, _): (NetMessage, usize) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::legacy())
                .expect("message should deserialize");

        match (message, decoded) {
            (
                NetMessage::RoomCreate(expected_id, expected_members),
                NetMessage::RoomCreate(id, members),
            ) => {
                assert_eq!(id, expected_id);
                assert_eq!(members, expected_members);
            }
            (
                NetMessage::RoomCreateV2 {
                    room_id: expected_id,
                    members: expected_members,
                    ai_mode: expected_mode,
                },
                NetMessage::RoomCreateV2 { room_id: id, members, ai_mode: mode },
            ) => {
                assert_eq!(id, expected_id);
                assert_eq!(members, expected_members);
                assert_eq!(mode, expected_mode);
            }
            (NetMessage::RoomJoin(expected_id), NetMessage::RoomJoin(id)) => {
                assert_eq!(id, expected_id);
            }
            (NetMessage::SkillResult(expected), NetMessage::SkillResult(actual)) => {
                assert_eq!(actual.skill_name, expected.skill_name);
                assert_eq!(actual.summary, expected.summary);
                assert_eq!(actual.success, expected.success);
            }
            pair => panic!("unexpected round-trip pair: {:?}", pair),
        }
    }
}

#[test]
fn legacy_room_create_bytes_remain_decodable() {
    let legacy =
        LegacyNetMessage::RoomCreate("room-1".into(), vec!["takuro".into(), "tanaka".into()]);

    let encoded = bincode::serde::encode_to_vec(&legacy, bincode::config::legacy())
        .expect("legacy message should serialize");
    let (decoded, _): (NetMessage, usize) =
        bincode::serde::decode_from_slice(&encoded, bincode::config::legacy())
            .expect("new decoder should accept legacy bytes");

    match decoded {
        NetMessage::RoomCreate(room_id, members) => {
            assert_eq!(room_id, "room-1");
            assert_eq!(members, vec!["takuro", "tanaka"]);
        }
        other => panic!("unexpected decoded message: {:?}", other),
    }
}
