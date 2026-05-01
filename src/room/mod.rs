pub mod member;
pub mod transcript;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::message::RoomId;
use crate::state::AiMode;

pub use member::{Member, MemberKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub id: RoomId,
    pub members: Vec<Member>,
    pub ai_mode: Option<AiMode>,
}

#[derive(Default)]
pub struct RoomEngine {
    rooms: Vec<Room>,
    active_room_id: Option<RoomId>,
    next_room_number: usize,
}

impl RoomEngine {
    pub fn create_room(&mut self, owner: &str, peer_ids: &[&str], ai_mode: Option<AiMode>) -> Room {
        let mut members = vec![Member::human(owner)];
        members.extend(peer_ids.iter().copied().map(Member::human));
        if let Some(ai_mode) = ai_mode.clone() {
            members.push(Member::ai("ops-ai", ai_mode));
        }

        self.next_room_number += 1;
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let id = format!("{owner}-{ts_ms}");
        let room = Room { id, members, ai_mode };
        self.active_room_id = Some(room.id.clone());
        self.rooms.push(room.clone());
        room
    }

    pub fn insert_room(&mut self, room: Room) {
        if !self.rooms.iter().any(|existing| existing.id == room.id) {
            self.rooms.push(room.clone());
        }
        self.active_room_id = Some(room.id);
    }

    pub fn create_remote_room(
        &mut self,
        room_id: &str,
        member_ids: &[String],
        ai_mode: Option<AiMode>,
    ) -> Room {
        let members = member_ids
            .iter()
            .map(|member_id| {
                if member_id == "ops-ai" {
                    match ai_mode.clone() {
                        Some(ai_mode) => Member::ai(member_id.clone(), ai_mode),
                        None => Member::remote_ai(member_id.clone()),
                    }
                } else {
                    Member::human(member_id.clone())
                }
            })
            .collect::<Vec<_>>();
        let room = Room { id: room_id.to_string(), members, ai_mode };
        self.insert_room(room.clone());
        room
    }

    pub fn rooms(&self) -> &[Room] {
        &self.rooms
    }

    pub fn active_room_id(&self) -> Option<&str> {
        self.active_room_id.as_deref()
    }

    pub fn resolve_room(&self, target: &str) -> Option<&Room> {
        if let Ok(index) = target.parse::<usize>() {
            return index.checked_sub(1).and_then(|idx| self.rooms.get(idx));
        }
        self.rooms.iter().find(|room| room.id == target)
    }

    pub fn switch_active(&mut self, room_id: &str) -> anyhow::Result<()> {
        if let Some(room) = self.resolve_room(room_id) {
            self.active_room_id = Some(room.id.clone());
            Ok(())
        } else {
            Err(anyhow::anyhow!("unknown room id: {room_id}"))
        }
    }

    pub fn active_room(&self) -> Option<&Room> {
        self.active_room_id
            .as_deref()
            .and_then(|room_id| self.rooms.iter().find(|room| room.id == room_id))
    }
}
