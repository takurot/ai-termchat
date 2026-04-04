pub mod member;
pub mod transcript;

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
        let room = Room { id: format!("room-{}", self.next_room_number), members, ai_mode };
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

    pub fn create_remote_room(&mut self, room_id: &str, member_ids: &[String]) -> Room {
        let ai_mode =
            member_ids.iter().any(|member_id| member_id == "ops-ai").then_some(AiMode::Clerk);
        let members = member_ids
            .iter()
            .map(|member_id| {
                if member_id == "ops-ai" {
                    Member::ai(member_id.clone(), AiMode::Clerk)
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

    pub fn switch_active(&mut self, room_id: &str) -> anyhow::Result<()> {
        if self.rooms.iter().any(|room| room.id == room_id) {
            self.active_room_id = Some(room_id.to_string());
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
