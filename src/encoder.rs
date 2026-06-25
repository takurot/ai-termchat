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

    pub fn encode<M: Serialize>(&mut self, message: M) -> &[u8] {
        self.output_buffer.clear();
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(&message, &mut self.output_buffer, config).unwrap();
        &self.output_buffer
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
