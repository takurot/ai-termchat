use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Local};
use message_io::network::Endpoint;
use rgb::RGB8;
use sha2::{Digest, Sha256};
use tokio::task::AbortHandle;

use crate::avatar::AvatarSize;
use crate::message::{AiPayload, PeerInfo, StructuredOutput};
use crate::room::transcript::{TranscriptEntry, TranscriptWriter};
use crate::room::{Room, RoomEngine};
use crate::skill::executor::PendingSkillExecution;
use crate::skill::registry::SkillRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMessageType {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressState {
    Started(u64),
    Working(u64, u64),
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiMode {
    Clerk,
    Listener,
    Moderator,
    Operator,
    Companion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiFrequency {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Thinking,
    Acting,
    Disabled,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillProposal {
    pub id: usize,
    pub skill_name: String,
    pub source_peer: Option<String>,
    pub trusted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageType {
    Connection,
    Disconnection,
    Text(String),
    AiText(String),
    System(String, SystemMessageType),
    Progress(ProgressState),
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub date: DateTime<Local>,
    pub user: String,
    pub message_type: MessageType,
}

impl ChatMessage {
    pub fn new(user: String, message_type: MessageType) -> ChatMessage {
        ChatMessage { date: Local::now(), user, message_type }
    }

    pub fn rendered_text(&self) -> String {
        match &self.message_type {
            MessageType::Connection => format!("{} connected", self.user),
            MessageType::Disconnection => format!("{} disconnected", self.user),
            MessageType::Text(text) | MessageType::AiText(text) => text.clone(),
            MessageType::System(text, _) => text.clone(),
            MessageType::Progress(_) => String::new(),
        }
    }
}

pub struct Window {
    pub data: Vec<RGB8>,
    pub width: usize,
    pub height: usize,
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self { data: vec![], width, height }
    }
}

pub struct State {
    messages: Vec<ChatMessage>,
    scroll_messages_view: usize,
    input: Vec<char>,
    input_cursor: usize,
    local_user_name: String,
    lan_users: HashMap<Endpoint, String>,
    peers: HashMap<Endpoint, PeerInfo>,
    users_id: HashMap<String, usize>,
    last_user_id: usize,
    room_engine: RoomEngine,
    skill_registry: SkillRegistry,
    pending_confirmation: Option<PendingSkillExecution>,
    pending_skill_proposals: Vec<SkillProposal>,
    transcript_base_dir: Option<PathBuf>,
    trusted_peer_fingerprints: HashSet<String>,
    pub stop_stream: bool,
    pub windows: HashMap<Endpoint, Window>,
    pub ai_state: AiState,
    pub ai_mode: AiMode,
    pub ai_thinking: bool,
    pub abort_handle: Option<AbortHandle>,
    pub last_ai_at: Option<Instant>,
    pub human_streak: usize,
    pub ai_frequency: AiFrequency,
    pub ui_language: String,
    pub last_structured_output: Option<StructuredOutput>,
    /// Preset name for the local user's avatar (default: `"human_default"`).
    pub user_avatar: String,
    /// Preset name for the AI avatar (default: `"ai_default"`).
    pub ai_avatar: String,
    /// Global avatar size hint.
    pub avatar_size: AvatarSize,
    room_list_scroll: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            scroll_messages_view: 0,
            input: Vec::new(),
            input_cursor: 0,
            local_user_name: String::new(),
            lan_users: HashMap::new(),
            peers: HashMap::new(),
            users_id: HashMap::new(),
            last_user_id: 0,
            room_engine: RoomEngine::default(),
            skill_registry: SkillRegistry::default(),
            pending_confirmation: None,
            pending_skill_proposals: Vec::new(),
            transcript_base_dir: None,
            trusted_peer_fingerprints: HashSet::new(),
            stop_stream: false,
            windows: HashMap::new(),
            ai_state: AiState::Idle,
            ai_mode: AiMode::Clerk,
            ai_thinking: false,
            abort_handle: None,
            last_ai_at: None,
            human_streak: 0,
            ai_frequency: AiFrequency::Normal,
            ui_language: "ja".into(),
            last_structured_output: None,
            user_avatar: "human_default".into(),
            ai_avatar: "ai_default".into(),
            avatar_size: AvatarSize::Normal,
            room_list_scroll: 0,
        }
    }
}

pub enum CursorMovement {
    Left,
    Right,
    Start,
    End,
}

pub enum ScrollMovement {
    Up,
    Down,
    Start,
}

impl State {
    pub fn messages(&self) -> &Vec<ChatMessage> {
        &self.messages
    }

    pub fn scroll_messages_view(&self) -> usize {
        self.scroll_messages_view
    }

    pub fn input(&self) -> &[char] {
        &self.input
    }

    pub fn ui_input_cursor(&self, width: usize) -> (u16, u16) {
        let mut position = (0, 0);

        for current_char in self.input.iter().take(self.input_cursor) {
            let char_width = unicode_width::UnicodeWidthChar::width(*current_char).unwrap_or(0);

            position.0 += char_width;

            match position.0.cmp(&width) {
                std::cmp::Ordering::Equal => {
                    position.0 = 0;
                    position.1 += 1;
                }
                std::cmp::Ordering::Greater => {
                    position.0 -= width - (char_width - 1);
                    position.1 += 1;
                }
                std::cmp::Ordering::Less => (),
            }
        }

        (position.0 as u16, position.1 as u16)
    }

    pub fn user_name(&self, endpoint: Endpoint) -> Option<&String> {
        self.lan_users.get(&endpoint)
    }

    pub fn local_user_name(&self) -> &str {
        &self.local_user_name
    }

    pub fn set_local_user_name(&mut self, user_name: impl Into<String>) {
        self.local_user_name = user_name.into();
    }

    pub fn set_skill_registry(&mut self, skill_registry: SkillRegistry) {
        self.skill_registry = skill_registry;
    }

    pub fn skill_registry(&self) -> &SkillRegistry {
        &self.skill_registry
    }

    pub fn set_transcript_base_dir(&mut self, transcript_base_dir: Option<PathBuf>) {
        self.transcript_base_dir = transcript_base_dir;
    }

    pub fn set_trusted_peer_fingerprints(
        &mut self,
        fingerprints: impl IntoIterator<Item = String>,
    ) {
        self.trusted_peer_fingerprints = fingerprints.into_iter().collect();
    }

    pub fn trust_peer_fingerprint(&mut self, fingerprint: impl Into<String>) {
        self.trusted_peer_fingerprints.insert(fingerprint.into());
    }

    pub fn is_trusted_peer(&self, fingerprint: &str) -> bool {
        self.trusted_peer_fingerprints.contains(fingerprint)
    }

    pub fn pending_confirmation(&self) -> Option<&PendingSkillExecution> {
        self.pending_confirmation.as_ref()
    }

    pub fn queue_skill_confirmation(&mut self, pending: PendingSkillExecution) {
        self.pending_confirmation = Some(pending);
    }

    pub fn take_pending_confirmation(&mut self) -> Option<PendingSkillExecution> {
        self.pending_confirmation.take()
    }

    pub fn clear_pending_confirmation(&mut self) {
        self.pending_confirmation = None;
    }

    pub fn set_skill_proposals(
        &mut self,
        skill_names: &[String],
        source_peer: Option<String>,
        trusted: bool,
    ) {
        self.pending_skill_proposals = skill_names
            .iter()
            .enumerate()
            .map(|(index, skill_name)| SkillProposal {
                id: index + 1,
                skill_name: skill_name.clone(),
                source_peer: source_peer.clone(),
                trusted,
            })
            .collect();
    }

    pub fn clear_skill_proposals(&mut self) {
        self.pending_skill_proposals.clear();
    }

    pub fn skill_proposals(&self) -> &[SkillProposal] {
        &self.pending_skill_proposals
    }

    pub fn find_skill_proposal(&self, proposal_id: usize) -> Option<&SkillProposal> {
        self.pending_skill_proposals.iter().find(|proposal| proposal.id == proposal_id)
    }

    pub fn all_user_endpoints(&self) -> Vec<Endpoint> {
        if let Some(room) = self.room_engine.active_room() {
            return self
                .lan_users
                .iter()
                .filter_map(|(endpoint, user_name)| {
                    if user_name == &self.local_user_name {
                        return None;
                    }
                    room.members.iter().any(|member| member.id == *user_name).then_some(*endpoint)
                })
                .collect();
        }
        self.lan_users.keys().copied().collect()
    }

    pub fn users_id(&self) -> &HashMap<String, usize> {
        &self.users_id
    }

    pub fn connected_user(&mut self, endpoint: Endpoint, user: &str) {
        if self.lan_users.get(&endpoint).is_some_and(|known| known == user) {
            return;
        }
        self.lan_users.insert(endpoint, user.into());
        self.peers.entry(endpoint).or_insert_with(|| PeerInfo {
            user_name: user.into(),
            server_port: 0,
            node_version: "unknown".into(),
        });
        if !self.users_id.contains_key(user) {
            self.users_id.insert(user.into(), self.last_user_id);
            self.last_user_id += 1;
        }
        self.add_message(ChatMessage::new(user.into(), MessageType::Connection));
    }

    pub fn disconnected_user(&mut self, endpoint: Endpoint) {
        if let Some(user) = self.lan_users.remove(&endpoint) {
            self.peers.remove(&endpoint);
            self.add_message(ChatMessage::new(user, MessageType::Disconnection));
        }
    }

    pub fn record_peer(&mut self, endpoint: Endpoint, peer: PeerInfo) {
        self.peers.insert(endpoint, peer);
    }

    pub fn peer_fingerprint(&self, endpoint: Endpoint) -> Option<String> {
        self.peers.get(&endpoint).map(peer_fingerprint)
    }

    pub fn peer_names(&self) -> Vec<String> {
        let mut peers = self.peers.values().map(|peer| peer.user_name.clone()).collect::<Vec<_>>();
        peers.sort();
        peers.dedup();
        peers
    }

    pub fn peers(&self) -> &HashMap<Endpoint, PeerInfo> {
        &self.peers
    }

    pub fn create_room(&mut self, peer_ids: &[String], ai_mode: Option<AiMode>) -> Room {
        let refs = peer_ids.iter().map(String::as_str).collect::<Vec<_>>();
        self.room_engine.create_room(&self.local_user_name, &refs, ai_mode)
    }

    pub fn accept_room(&mut self, room_id: &str, member_ids: &[String]) -> Room {
        self.room_engine.create_remote_room(room_id, member_ids)
    }

    pub fn room_ids(&self) -> Vec<String> {
        self.room_engine.rooms().iter().map(|room| room.id.clone()).collect()
    }

    pub fn rooms(&self) -> &[Room] {
        self.room_engine.rooms()
    }

    pub fn active_room(&self) -> Option<&Room> {
        self.room_engine.active_room()
    }

    pub fn active_room_id(&self) -> Option<&str> {
        self.room_engine.active_room_id()
    }

    pub fn resolve_room(&self, target: &str) -> Option<&Room> {
        self.room_engine.resolve_room(target)
    }

    pub fn switch_room(&mut self, room_id: &str) -> anyhow::Result<()> {
        self.room_engine.switch_active(room_id)
    }

    pub fn input_write(&mut self, character: char) {
        self.input.insert(self.input_cursor, character);
        self.input_cursor += 1;
    }

    pub fn input_remove(&mut self) {
        if self.input_cursor < self.input.len() {
            self.input.remove(self.input_cursor);
        }
    }

    pub fn input_remove_previous(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.input.remove(self.input_cursor);
        }
    }

    pub fn input_move_cursor(&mut self, movement: CursorMovement) {
        match movement {
            CursorMovement::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            CursorMovement::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
            }
            CursorMovement::Start => self.input_cursor = 0,
            CursorMovement::End => self.input_cursor = self.input.len(),
        }
    }

    pub fn messages_scroll(&mut self, movement: ScrollMovement) {
        match movement {
            ScrollMovement::Up => {
                if self.scroll_messages_view > 0 {
                    self.scroll_messages_view -= 1;
                }
            }
            ScrollMovement::Down => self.scroll_messages_view += 1,
            ScrollMovement::Start => self.scroll_messages_view += 0,
        }
    }

    pub fn room_list_scroll(&self) -> usize {
        self.room_list_scroll
    }

    pub fn scroll_room_list(&mut self, movement: ScrollMovement) {
        match movement {
            ScrollMovement::Up => {
                self.room_list_scroll = self.room_list_scroll.saturating_sub(1);
            }
            ScrollMovement::Down => {
                self.room_list_scroll = self.room_list_scroll.saturating_add(1);
            }
            ScrollMovement::Start => {}
        }
    }

    pub fn reset_room_list_scroll(&mut self) {
        self.room_list_scroll = 0;
    }

    pub fn reset_input(&mut self) -> Option<String> {
        if !self.input.is_empty() {
            self.input_cursor = 0;
            return Some(self.input.drain(..).collect());
        }
        None
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        let entry = self.default_transcript_entry(&message);
        self.write_transcript_entry(entry);
        self.messages.push(message);
    }

    pub fn add_message_with_transcript(
        &mut self,
        message: ChatMessage,
        transcript_entry: TranscriptEntry,
    ) {
        self.write_transcript_entry(transcript_entry);
        self.messages.push(message);
    }

    pub fn add_ai_message(&mut self, payload: AiPayload) {
        self.last_structured_output = payload.structured.clone();
        self.messages.push(ChatMessage::new("ops-ai".into(), MessageType::AiText(payload.text)));
    }

    pub fn add_system_warn_message(&mut self, content: String) {
        self.messages.push(ChatMessage::new(
            "triadchat: ".into(),
            MessageType::System(content, SystemMessageType::Warning),
        ));
    }

    pub fn add_system_info_message(&mut self, content: String) {
        self.messages.push(ChatMessage::new(
            "triadchat: ".into(),
            MessageType::System(content, SystemMessageType::Info),
        ));
    }

    pub fn add_system_error_message(&mut self, content: String) {
        self.messages.push(ChatMessage::new(
            "triadchat: ".into(),
            MessageType::System(content, SystemMessageType::Error),
        ));
    }

    pub fn add_progress_message(&mut self, file_name: &str, total: u64) -> usize {
        self.messages.push(ChatMessage::new(
            format!("Sending '{}'", file_name),
            MessageType::Progress(ProgressState::Started(total)),
        ));
        self.messages.len() - 1
    }

    pub fn progress_message_update(&mut self, index: usize, increment: u64) {
        match &mut self.messages[index].message_type {
            MessageType::Progress(state) => {
                *state = match state {
                    ProgressState::Started(total) => ProgressState::Working(*total, increment),
                    ProgressState::Working(total, current) => {
                        let new_current = *current + increment;
                        if new_current == *total {
                            ProgressState::Completed
                        } else {
                            ProgressState::Working(*total, new_current)
                        }
                    }
                    ProgressState::Completed => ProgressState::Completed,
                };
            }
            _ => panic!("Must be a Progress MessageType"),
        }
    }

    pub fn update_window(
        &mut self,
        endpoint: &Endpoint,
        data: Vec<RGB8>,
        width: usize,
        height: usize,
    ) {
        if let Some(window) = self.windows.get_mut(endpoint) {
            window.data = data;
            window.width = width;
            window.height = height;
        }
    }

    pub fn transcript(&self, max_messages: usize) -> String {
        self.messages
            .iter()
            .rev()
            .filter_map(|message| match &message.message_type {
                MessageType::Text(text) => Some(format!("{}: {}", message.user, text)),
                MessageType::AiText(text) => Some(format!("ops-ai: {}", text)),
                _ => None,
            })
            .take(max_messages)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn recent_human_messages(&self, max_messages: usize) -> Vec<String> {
        self.messages
            .iter()
            .rev()
            .filter_map(|message| match &message.message_type {
                MessageType::Text(text) => Some(text.clone()),
                _ => None,
            })
            .take(max_messages)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn set_ai_disabled(&mut self) {
        self.ai_state = AiState::Disabled;
        self.ai_thinking = false;
        self.abort_handle = None;
    }

    fn write_transcript_entry(&mut self, entry: TranscriptEntry) {
        if let Some(base_dir) = self.transcript_base_dir.as_ref() {
            let _ = TranscriptWriter::append_to_base(base_dir, &entry);
        }
    }

    fn default_transcript_entry(&self, message: &ChatMessage) -> TranscriptEntry {
        let room_id = self
            .active_room_id()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("solo-{}", self.local_user_name));
        let sender_type = if message.user.starts_with("ops-ai") {
            "ai"
        } else if message.user.starts_with("triadchat:") {
            "system"
        } else {
            "human"
        };
        let kind = match message.message_type {
            MessageType::AiText(_) => "ai",
            MessageType::System(_, _) => "system",
            MessageType::Progress(_) => "progress",
            _ => "chat",
        };
        TranscriptEntry::chat(
            room_id,
            message.user.clone(),
            sender_type,
            kind,
            message.rendered_text(),
        )
    }
}

pub fn peer_fingerprint(peer: &PeerInfo) -> String {
    let mut hasher = Sha256::new();
    hasher.update(peer.user_name.as_bytes());
    hasher.update(peer.node_version.as_bytes());
    format!("{:x}", hasher.finalize())
}
