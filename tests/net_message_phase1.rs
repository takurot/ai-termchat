use triadchat::message::{NetMessage, PeerInfo, SkillResultPayload};

#[test]
fn peer_info_round_trips_through_bincode() {
    let message = NetMessage::PeerInfo(PeerInfo {
        user_name: "takuro".into(),
        server_port: 4000,
        node_version: "0.1.0".into(),
    });

    let encoded = bincode::serialize(&message).expect("message should serialize");
    let decoded: NetMessage = bincode::deserialize(&encoded).expect("message should deserialize");

    match decoded {
        NetMessage::PeerInfo(info) => {
            assert_eq!(info.user_name, "takuro");
            assert_eq!(info.server_port, 4000);
            assert_eq!(info.node_version, "0.1.0");
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn room_and_skill_variants_round_trip_through_bincode() {
    let room_create = NetMessage::RoomCreate("room-1".into(), vec!["takuro".into(), "tanaka".into()]);
    let room_join = NetMessage::RoomJoin("room-1".into());
    let skill_done = NetMessage::SkillResult(SkillResultPayload {
        skill_name: "review-auth".into(),
        summary: "auth review complete".into(),
        success: true,
    });

    for message in [room_create, room_join, skill_done] {
        let encoded = bincode::serialize(&message).expect("message should serialize");
        let decoded: NetMessage =
            bincode::deserialize(&encoded).expect("message should deserialize");

        match (message, decoded) {
            (NetMessage::RoomCreate(expected_id, expected_members), NetMessage::RoomCreate(id, members)) => {
                assert_eq!(id, expected_id);
                assert_eq!(members, expected_members);
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
