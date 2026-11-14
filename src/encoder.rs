use serde::Serialize;
use crate::message::NetMessage;

pub const MAX_FRAME_SIZE: usize = 1024 * 1024; // 1 MB

pub struct Encoder {
    output_buffer: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Encoder {
        Self { output_buffer: Vec::new() }
    }

    pub fn encode<M: Serialize>(&mut self, message: M) -> anyhow::Result<&[u8]> {
        self.output_buffer.clear();
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(&message, &mut self.output_buffer, config)?;
        anyhow::ensure!(
            self.output_buffer.len() <= MAX_FRAME_SIZE,
            "outbound frame exceeds {} byte limit",
            MAX_FRAME_SIZE
        );
        Ok(&self.output_buffer)
    }
}

pub fn decode(data_message: &[u8]) -> Option<NetMessage> {
    if data_message.len() > MAX_FRAME_SIZE {
        return None;
    }
    let config = bincode::config::legacy().with_limit::<{ MAX_FRAME_SIZE }>();
    let (msg, _): (NetMessage, usize) =
        bincode::serde::decode_from_slice(data_message, config).ok()?;
    if msg.validate() {
        Some(msg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{AiPayload, Chunk, NetMessage, TodoItem, StructuredOutput};

    struct FailingMessage;

    impl Serialize for FailingMessage {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("intentional serialization failure"))
        }
    }

    #[test]
    fn serialization_failure_does_not_panic() {
        let outcome = std::panic::catch_unwind(|| {
            let mut encoder = Encoder::new();
            encoder.encode(FailingMessage).is_err()
        });
        assert!(outcome.is_ok(), "encoding failure must not panic");
        assert!(outcome.unwrap(), "encoding failure must return an error");
    }

    struct PartiallyFailingMessage(usize);

    impl Serialize for PartiallyFailingMessage {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeTuple;
            let mut tuple = serializer.serialize_tuple(self.0 + 1)?;
            for _ in 0..self.0 {
                tuple.serialize_element(&42u8)?;
            }
            Err(serde::ser::Error::custom("failure after partial output"))
        }
    }

    #[test]
    fn adversarial_serializers_allow_clean_buffer_reuse() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(116);
        let mut encoder = Encoder::new();
        let message = NetMessage::HelloUser("user".into());
        let expected = test_serialize(&message);
        for _ in 0..128 {
            let prefix_len = rng.gen_range(1..1024);
            assert!(encoder.encode(PartiallyFailingMessage(prefix_len)).is_err());
            assert_eq!(encoder.encode(&message).unwrap(), expected);
        }
    }

    #[test]
    fn encoding_enforces_serialized_frame_size_boundary() {
        let mut encoder = Encoder::new();
        let boundary = vec![0u8; MAX_FRAME_SIZE - std::mem::size_of::<u64>()];
        assert_eq!(encoder.encode(boundary).unwrap().len(), MAX_FRAME_SIZE);
        let oversized = vec![0u8; MAX_FRAME_SIZE - std::mem::size_of::<u64>() + 1];
        assert!(encoder.encode(oversized).is_err());
        let message = NetMessage::HelloUser("user".into());
        assert_eq!(encoder.encode(&message).unwrap(), test_serialize(&message));
    }

    fn test_serialize<M: Serialize>(msg: &M) -> Vec<u8> {
        let config = bincode::config::legacy();
        bincode::serde::encode_to_vec(msg, config).unwrap()
    }

    #[test]
    fn test_max_frame_size_rejection() {
        let large_buffer = vec![0; MAX_FRAME_SIZE + 1];
        let decoded: Option<NetMessage> = decode(&large_buffer);
        assert!(decoded.is_none());
    }

    #[test]
    fn test_valid_small_message() {
        let msg = NetMessage::UserMessage("Hello".into());
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_some());
        if let Some(NetMessage::UserMessage(s)) = decoded {
            assert_eq!(s, "Hello");
        } else {
            panic!("Expected UserMessage");
        }
    }

    #[test]
    fn test_oversized_chat_message() {
        let msg = NetMessage::UserMessage("A".repeat(crate::message::MAX_CHAT_MESSAGE_LEN + 1));
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject chat message exceeding max length");
    }

    #[test]
    fn test_oversized_file_chunk() {
        let bad_chunk = Chunk::Data(vec![0; crate::message::MAX_FILE_CHUNK_LEN + 1]);
        let msg = NetMessage::UserData("file.txt".into(), bad_chunk);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject oversized file chunk");
    }

    #[test]
    fn test_oversized_room_create_members() {
        let members = vec!["user".into(); crate::message::MAX_ROOM_MEMBERS + 1];
        let msg = NetMessage::RoomCreate("room1".into(), members);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject RoomCreate with too many members");
    }

    #[test]
    fn test_oversized_ai_message_text() {
        let payload = AiPayload {
            text: "A".repeat(crate::message::MAX_AI_TEXT_LEN + 1),
            intent: Default::default(),
            structured: None,
        };
        let msg = NetMessage::AiMessage(payload);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject AI message with too long text");
    }

    #[test]
    fn test_oversized_ai_structured_todos() {
        let todos = vec![
            TodoItem { text: "Todo".into(), assignee: None };
            crate::message::MAX_TODO_ITEMS + 1
        ];
        let structured = StructuredOutput { todos, ..Default::default() };
        let payload = AiPayload {
            text: "Short text".into(),
            intent: Default::default(),
            structured: Some(structured),
        };
        let msg = NetMessage::AiMessage(payload);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject AI message with too many todo items");
    }

    #[test]
    fn test_ai_structured_raw_text_is_rejected() {
        let payload = AiPayload {
            text: "Short text".into(),
            intent: Default::default(),
            structured: Some(StructuredOutput::raw("raw fallback")),
        };
        let msg = NetMessage::AiMessage(payload);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject AI structured output with raw_text");
    }

    #[test]
    fn test_ai_structured_control_skill_name_is_decodable_for_application_filtering() {
        let structured = StructuredOutput {
            skill_suggestions: vec!["STRUCTURED: malicious".into()],
            ..Default::default()
        };
        let payload = AiPayload {
            text: "Short text".into(),
            intent: Default::default(),
            structured: Some(structured),
        };
        let msg = NetMessage::AiMessage(payload);
        let encoded = test_serialize(&msg);
        let decoded = decode(&encoded);
        assert!(
            decoded.is_some(),
            "Application-level AI handling filters unsafe skill names while preserving payload"
        );
    }

    #[test]
    fn test_trailing_bytes() {
        let msg = NetMessage::UserMessage("Hello".into());
        let mut encoded = test_serialize(&msg);
        encoded.extend_from_slice(b"extra padding bytes");
        let decoded = decode(&encoded);
        assert!(decoded.is_some(), "Should allow trailing bytes!");
    }
}
