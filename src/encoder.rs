use serde::Serialize;
use crate::message::NetMessage;
use bincode::Options;

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
        bincode::serialize_into(&mut self.output_buffer, &message).unwrap();
        &self.output_buffer
    }
}

pub fn decode(data_message: &[u8]) -> Option<NetMessage> {
    if data_message.len() > MAX_FRAME_SIZE {
        return None;
    }
    let options = bincode::options()
        .with_limit(MAX_FRAME_SIZE as u64)
        .with_fixint_encoding()
        .allow_trailing_bytes();
    let msg: NetMessage = options.deserialize(data_message).ok()?;
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

    #[test]
    fn test_max_frame_size_rejection() {
        let large_buffer = vec![0; MAX_FRAME_SIZE + 1];
        let decoded: Option<NetMessage> = decode(&large_buffer);
        assert!(decoded.is_none());
    }

    #[test]
    fn test_valid_small_message() {
        let msg = NetMessage::UserMessage("Hello".into());
        let encoded = bincode::serialize(&msg).unwrap();
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
        let encoded = bincode::serialize(&msg).unwrap();
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject chat message exceeding max length");
    }

    #[test]
    fn test_oversized_file_chunk() {
        let bad_chunk = Chunk::Data(vec![0; crate::message::MAX_FILE_CHUNK_LEN + 1]);
        let msg = NetMessage::UserData("file.txt".into(), bad_chunk);
        let encoded = bincode::serialize(&msg).unwrap();
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject oversized file chunk");
    }

    #[test]
    fn test_oversized_room_create_members() {
        let members = vec!["user".into(); crate::message::MAX_ROOM_MEMBERS + 1];
        let msg = NetMessage::RoomCreate("room1".into(), members);
        let encoded = bincode::serialize(&msg).unwrap();
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
        let encoded = bincode::serialize(&msg).unwrap();
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
        let encoded = bincode::serialize(&msg).unwrap();
        let decoded = decode(&encoded);
        assert!(decoded.is_none(), "Should reject AI message with too many todo items");
    }
}
