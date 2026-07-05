use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use message_io::events::EventReceiver;
use message_io::network::{Endpoint, Transport};
use message_io::node::{
    self, NodeHandler, NodeTask, StoredNetEvent as NetEvent, StoredNodeEvent as NodeEvent,
};
use tracing::warn;

use crate::action::{Action, Processing};
use crate::ai::trigger::{contains_ops_ai_mention, should_intervene, TriggerConfig};
use crate::ai::{AiMediator, AiTask};
use crate::avatar::loader::AvatarManager;
use crate::avatar::{AvatarSize, AvatarState};
use crate::commands::ai_cmd::AiCommand;
use crate::commands::art_cmd::ArtCommand;
use crate::commands::avatar_cmd::AvatarCommand;
use crate::commands::peer_cmd::{PeerCommand, TrustCommand};
use crate::commands::room_cmd::{PeersCommand, RoomCommand};
use crate::commands::send_file::SendFileCommand;
use crate::commands::skill_cmd::{CancelCommand, RunCommand, SkillCommand, SkillsCommand};
use crate::commands::summary_cmd::SummaryCommand;
use crate::commands::{
    AppCommand, AvatarCommandKind, CommandManager, ParsedCommand, SummaryCommandKind,
};
use crate::config::{Config, DefaultPermission};
use crate::encoder::{self, Encoder};
use crate::message::{
    AiIntent, AiPayload, Chunk, NetMessage, PeerInfo, SkillResultPayload, SignaturePayload,
};
use crate::room::transcript::TranscriptEntry;
use crate::room::{MemberKind, Room};
use crate::renderer::Renderer;
use crate::secure::SecureState;
use crate::state::{
    AiFrequency, AiMode, AiState, ChatMessage, CursorMovement, MessageType, ScrollMovement, State,
    PeerReadiness,
};
use crate::skill::executor::{PendingSkillExecution, SkillExecutor};
use crate::skill::registry::{InvokeMode, RiskLevel};
use crate::terminal_events::TerminalEventCollector;
use crate::util::{Error, Reportable, Result};
use sha2::Digest;

pub enum Signal {
    Terminal(TermEvent),
    Action(Box<dyn Action>),
    AiResponse(AiPayload, bool),
    AiFailed(String),
    SkillDone(SkillResultPayload),
    DiscoveryRetry,
    Close(Option<Error>),
}

pub struct Application<'a> {
    config: &'a Config,
    commands: CommandManager,
    state: State,
    node: NodeHandler<Signal>,
    _task: NodeTask,
    _terminal_events: Option<TerminalEventCollector>,
    receiver: EventReceiver<NodeEvent<Signal>>,
    encoder: Encoder,
    runtime: tokio::runtime::Runtime,
    ai_mediator: Option<Arc<AiMediator>>,
    workspace: PathBuf,
    avatar_manager: AvatarManager,
    art_dict: HashMap<String, String>,
    art_yaml_path: Option<PathBuf>,
    test_mode: bool,
    local_server_port: Option<u16>,
    config_file_path: Option<PathBuf>,
    discovery_retries_remaining: usize,
    signing_key: ed25519_dalek::SigningKey,
    secure_state: SecureState,
}

impl<'a> Application<'a> {
    pub fn new(config: &'a Config) -> Result<Self> {
        let workspace = std::env::current_dir()?;
        Self::new_inner(config, true, &workspace)
    }

    pub fn new_in_workspace(config: &'a Config, workspace: &std::path::Path) -> Result<Self> {
        Self::new_inner(config, true, workspace)
    }

    pub fn new_for_test(config: &'a Config) -> Result<Self> {
        let workspace = std::env::current_dir()?;
        Self::new_inner(config, false, &workspace)
    }

    pub fn new_for_test_in_workspace(
        config: &'a Config,
        workspace: &std::path::Path,
    ) -> Result<Self> {
        Self::new_inner(config, false, workspace)
    }

    fn new_inner(
        config: &'a Config,
        collect_terminal_events: bool,
        workspace: &std::path::Path,
    ) -> Result<Self> {
        let (handler, listener) = node::split();
        let _terminal_events = if collect_terminal_events {
            let terminal_handler = handler.clone();
            Some(TerminalEventCollector::new(move |term_event| match term_event {
                Ok(event) => terminal_handler.signals().send(Signal::Terminal(event)),
                Err(error) => terminal_handler.signals().send(Signal::Close(Some(error))),
            })?)
        } else {
            None
        };

        let (_task, receiver) = listener.enqueue();
        let commands = CommandManager::default()
            .with(SendFileCommand)
            .with(AiCommand)
            .with(PeerCommand)
            .with(RoomCommand)
            .with(PeersCommand)
            .with(TrustCommand)
            .with(SkillsCommand)
            .with(SkillCommand)
            .with(RunCommand)
            .with(CancelCommand)
            .with(SummaryCommand::summary())
            .with(SummaryCommand::todos())
            .with(SummaryCommand::decisions())
            .with(SummaryCommand::context())
            .with(AvatarCommand)
            .with(ArtCommand);
        let runtime = tokio::runtime::Runtime::new()?;

        let mut state = State::default();
        state.set_local_user_name(config.user_name.clone());
        state.ui_language = config.language.ui.clone();
        state.user_avatar = config.user.avatar.clone();
        state.ai_avatar = config.user.ai_avatar.clone();
        state.ai_provider = config.ai.provider.clone();
        state.set_trusted_peer_fingerprints(config.security.trusted_peers.clone());
        state.set_skill_registry(crate::skill::registry::SkillRegistry::scan(workspace));
        state.set_transcript_base_dir(dirs_next::data_dir());
        if collect_terminal_events {
            state.set_downloads_base_dir(dirs_next::data_dir());
        } else {
            state.set_downloads_base_dir(Some(std::env::temp_dir()));
        }

        let ai_mediator = if config.ai.enabled {
            match AiMediator::new(workspace, &config.ai, &config.language) {
                Ok(mediator) => Some(Arc::new(mediator)),
                Err(error) => {
                    state.ai_state = AiState::Disabled;
                    state.add_system_warn_message(format!("AI disabled: {}", error));
                    None
                }
            }
        } else {
            state.ai_state = AiState::Disabled;
            None
        };

        let avatar_dir = dirs_next::config_dir()
            .map(|d| d.join("triadchat").join("avatars"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/triadchat-avatars"));
        let avatar_manager = AvatarManager::new(avatar_dir);
        let config_file_path = if collect_terminal_events {
            Config::config_file_path()
        } else {
            Some(Config::config_file_path_with_base(workspace.join(".triadchat-test-config")))
        };

        let art_yaml_path = if collect_terminal_events {
            Config::config_dir_path().map(|d| d.join("art.yaml"))
        } else {
            Some(workspace.join("art.yaml"))
        };
        let art_dict = load_art_dictionary(art_yaml_path.as_deref()).unwrap_or_default();

        let signing_key = if collect_terminal_events {
            Config::get_or_create_identity_keypair()?
        } else {
            use rand::rngs::OsRng;
            ed25519_dalek::SigningKey::generate(&mut OsRng)
        };

        Ok(Self {
            config,
            commands,
            state,
            node: handler,
            _task,
            _terminal_events,
            receiver,
            encoder: Encoder::new(),
            runtime,
            ai_mediator,
            workspace: workspace.to_path_buf(),
            avatar_manager,
            art_dict,
            art_yaml_path,
            test_mode: !collect_terminal_events,
            local_server_port: None,
            config_file_path,
            discovery_retries_remaining: 0,
            signing_key,
            secure_state: SecureState::default(),
        })
    }

    pub fn run(&mut self, out: impl std::io::Write) -> Result<()> {
        let mut renderer =
            if self._terminal_events.is_some() { Some(Renderer::new(out)?) } else { None };
        if let Some(renderer) = renderer.as_mut() {
            renderer.render(&mut self.state, self.config, &self.avatar_manager)?;
        }

        self.start_network()?;

        loop {
            if !self.process_next_event()? {
                return Ok(());
            }
            if let Some(renderer) = renderer.as_mut() {
                renderer.render(&mut self.state, self.config, &self.avatar_manager)?;
            }
        }
    }

    fn start_network(&mut self) -> Result<()> {
        let server_addr = ("0.0.0.0", self.config.tcp_server_port);
        let (_, server_addr) = self.node.network().listen(Transport::FramedTcp, server_addr)?;
        self.local_server_port = Some(server_addr.port());
        self.node.network().listen(Transport::Udp, self.config.discovery_addr)?;
        self.discovery_retries_remaining = 5;
        self.state.add_system_info_message(format!(
            "Listening on tcp://127.0.0.1:{} and discovering via {}",
            server_addr.port(),
            self.config.discovery_addr
        ));
        self.announce_presence()?;
        self.schedule_discovery_retry();
        Ok(())
    }

    pub fn start_network_for_test(&mut self) -> Result<()> {
        self.start_network()
    }

    pub fn announce_presence_for_test(&mut self) -> Result<()> {
        self.announce_presence()
    }

    pub fn process_next_event_for_test(&mut self) -> Result<()> {
        self.process_next_event_with_timeout_for_test(std::time::Duration::from_secs(2))
    }

    pub fn process_next_event_with_timeout_for_test(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<()> {
        let event = self
            .receiver
            .receive_timeout(timeout)
            .ok_or_else(|| anyhow::anyhow!("timed out waiting for application event"))?;
        self.process_node_event(event).map(|_| ())
    }

    fn process_next_event(&mut self) -> Result<bool> {
        let event = self.receiver.receive();
        self.process_node_event(event)
    }

    fn process_node_event(&mut self, event: NodeEvent<Signal>) -> Result<bool> {
        match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(_, _) | NetEvent::Accepted(_, _) => {}
                NetEvent::Message(endpoint, message) => {
                    if message.len() > encoder::MAX_FRAME_SIZE {
                        self.state.add_system_warn_message(format!(
                            "ignored oversized message from {} ({} bytes, max {})",
                            endpoint.addr(),
                            message.len(),
                            encoder::MAX_FRAME_SIZE
                        ));
                    } else {
                        match encoder::decode(&message) {
                            Some(net_message) => {
                                self.process_network_message(endpoint, net_message)
                            }
                            None => self.state.add_system_warn_message(format!(
                                "ignored incompatible or invalid message from {}",
                                endpoint.addr()
                            )),
                        }
                    }
                }
                NetEvent::Disconnected(endpoint) => {
                    self.state.disconnected_user(endpoint);
                    self.secure_state.remove(endpoint);
                    self.ring_the_bell();
                }
            },
            NodeEvent::Signal(signal) => match signal {
                Signal::Terminal(term_event) => self.process_terminal_event(term_event)?,
                Signal::Action(action) => self.process_action(action),
                Signal::AiResponse(payload, truncated) => {
                    self.process_ai_response(payload, truncated)
                }
                Signal::AiFailed(error) => self.process_ai_failure(error),
                Signal::SkillDone(payload) => self.process_skill_done(payload),
                Signal::DiscoveryRetry => self.handle_discovery_retry(),
                Signal::Close(error) => {
                    self.node.stop();
                    return match error {
                        Some(error) => Err(error),
                        None => Ok(false),
                    };
                }
            },
        }
        Ok(true)
    }

    fn process_network_message(&mut self, endpoint: Endpoint, message: NetMessage) {
        match message {
            NetMessage::HelloLan(user, server_port) => {
                let server_addr = (endpoint.addr().ip(), server_port);
                if user != self.config.user_name {
                    if self.state.peer_names().iter().any(|known| known == &user) {
                        return;
                    }
                    let mut try_connect = || -> Result<()> {
                        let (user_endpoint, _) =
                            self.node.network().connect_sync(Transport::FramedTcp, server_addr)?;
                        let message = NetMessage::HelloUser(self.config.user_name.clone());
                        self.node.network().send(user_endpoint, self.encoder.encode(message));
                        self.send_peer_info(user_endpoint);
                        self.state.connected_user(user_endpoint, &user);
                        Ok(())
                    };
                    try_connect().report_if_err(&mut self.state);
                }
            }
            NetMessage::HelloUser(user) => {
                let already_ready = self.state.peer_is_ready(&user);
                self.state.connected_user(endpoint, &user);
                if !already_ready {
                    self.send_peer_info(endpoint);
                }
                self.ring_the_bell();
            }
            NetMessage::UserMessage(content) => {
                if let Some(user) = self.state.user_name(endpoint).cloned() {
                    if !self.accepts_authenticated_peer_message(endpoint, "message") {
                        return;
                    }
                    self.state.add_message(ChatMessage::new(user, MessageType::Text(content)));
                    self.ring_the_bell();
                }
            }
            NetMessage::UserData(file_name, chunk) => {
                if let Some(user) = self.state.user_name(endpoint).cloned() {
                    if !self.accepts_authenticated_peer_message(endpoint, "file transfer") {
                        return;
                    }
                    self.process_user_data(&user, &file_name, chunk);
                }
            }
            NetMessage::AiMessage(payload) => {
                if !self.accepts_authenticated_peer_message(endpoint, "AI message") {
                    return;
                }
                self.process_remote_ai_response(endpoint, payload);
            }
            NetMessage::PeerInfo(peer) => {
                if self.rejects_authenticated_peer_identity_change(endpoint, &peer) {
                    return;
                }
                self.state.connected_user(endpoint, &peer.user_name);
                self.state.record_peer(endpoint, peer.clone());
                if !version_supports_peer_identity(&peer.node_version) {
                    if let Some(fingerprint) = self.state.peer_fingerprint(endpoint) {
                        let trust_state = if self.state.is_trusted_peer(&fingerprint) {
                            "trusted"
                        } else {
                            "untrusted"
                        };
                        self.state.add_system_info_message(format!(
                            "peer ready: {} [{}] fp={}",
                            peer.user_name,
                            trust_state,
                            short_fingerprint(&fingerprint)
                        ));
                        self.state.add_system_warn_message(format!(
                            "Warning: peer {} is using an older version ({}) and is Unauthenticated.",
                            peer.user_name, peer.node_version
                        ));
                    }
                }
            }
            NetMessage::PeerIdentity { public_key, signature, timestamp } => {
                let Some(peer) = self.state.peers().get(&endpoint).cloned() else {
                    self.node.network().remove(endpoint.resource_id());
                    return;
                };

                let now = chrono::Utc::now().timestamp() as u64;
                let drift = (now as i64 - timestamp as i64).abs();
                if drift > 60 {
                    self.state.add_system_error_message(format!(
                        "Security warning: signature timestamp drift too high ({}s) from {}. Disconnecting.",
                        drift, peer.user_name
                    ));
                    self.node.network().remove(endpoint.resource_id());
                    return;
                }

                if self.state.contains_replay_signature(&signature) {
                    self.state.add_system_error_message(format!(
                        "Security warning: replayed signature detected from {}. Disconnecting.",
                        peer.user_name
                    ));
                    self.node.network().remove(endpoint.resource_id());
                    return;
                }

                let payload = SignaturePayload {
                    user_name: peer.user_name.clone(),
                    node_version: peer.node_version.clone(),
                    server_port: peer.server_port,
                    timestamp,
                };

                let serialized =
                    match bincode::serde::encode_to_vec(&payload, bincode::config::legacy()) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            self.state.add_system_error_message(format!(
                                "Failed to serialize signature payload for {}: {}. Disconnecting.",
                                peer.user_name, e
                            ));
                            self.node.network().remove(endpoint.resource_id());
                            return;
                        }
                    };

                let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
                    &public_key.clone().try_into().unwrap_or([0u8; 32]),
                ) {
                    Ok(key) => key,
                    Err(e) => {
                        self.state.add_system_error_message(format!(
                            "Invalid public key from {}: {}. Disconnecting.",
                            peer.user_name, e
                        ));
                        self.node.network().remove(endpoint.resource_id());
                        return;
                    }
                };

                let sig = match ed25519_dalek::Signature::from_slice(&signature) {
                    Ok(s) => s,
                    Err(e) => {
                        self.state.add_system_error_message(format!(
                            "Invalid signature format from {}: {}. Disconnecting.",
                            peer.user_name, e
                        ));
                        self.node.network().remove(endpoint.resource_id());
                        return;
                    }
                };

                use ed25519_dalek::Verifier;
                if let Err(e) = verifying_key.verify(&serialized, &sig) {
                    self.state.add_system_error_message(format!(
                        "Security warning: signature verification failed for {}: {}. Disconnecting.",
                        peer.user_name, e
                    ));
                    self.node.network().remove(endpoint.resource_id());
                    return;
                }

                self.state.insert_replay_signature(signature);

                let mut hasher = sha2::Sha256::new();
                hasher.update(&public_key);
                let fp = format!("{:x}", hasher.finalize());

                self.state.add_verified_peer_fingerprint(endpoint, fp.clone());

                let trust_state =
                    if self.state.is_trusted_peer(&fp) { "trusted" } else { "untrusted" };

                self.state.add_system_info_message(format!(
                    "peer ready: {} [{}] fp={}",
                    peer.user_name,
                    trust_state,
                    short_fingerprint(&fp)
                ));

                self.state.add_peer_public_key(endpoint, public_key);

                if peer.user_name < self.config.user_name {
                    self.initiate_key_exchange(endpoint);
                }
            }
            NetMessage::RoomCreate(room_id, member_ids) => {
                if !self.accepts_authenticated_peer_message(endpoint, "room invite") {
                    return;
                }
                if member_ids.iter().any(|member_id| member_id == &self.config.user_name) {
                    let room = self.state.accept_room(&room_id, &member_ids, None);
                    self.state.add_system_info_message(format!("joined room {}", room.id));
                    self.send_secure_to_peer(endpoint, NetMessage::RoomJoin(room.id.clone()));
                }
            }
            NetMessage::RoomCreateV2 { room_id, members, ai_mode } => {
                if !self.accepts_authenticated_peer_message(endpoint, "room invite") {
                    return;
                }
                if members.iter().any(|member_id| member_id == &self.config.user_name) {
                    let room = self.state.accept_room(&room_id, &members, ai_mode);
                    self.state.add_system_info_message(format!("joined room {}", room.id));
                    self.send_secure_to_peer(endpoint, NetMessage::RoomJoin(room.id.clone()));
                }
            }
            NetMessage::RoomJoin(room_id) => {
                if !self.accepts_authenticated_peer_message(endpoint, "room join") {
                    return;
                }
                let _ = self.state.switch_room(&room_id);
                self.state.add_system_info_message(format!("room {} is ready", room_id));
            }
            NetMessage::SkillResult(payload) => {
                if !self.accepts_authenticated_peer_message(endpoint, "skill result") {
                    return;
                }
                self.record_skill_done(payload, false);
            }
            NetMessage::KeyExchange { public_key, signature } => {
                self.process_key_exchange(endpoint, public_key, signature);
            }
            NetMessage::Secure(encrypted) => {
                self.process_secure_message(endpoint, encrypted);
            }
        }
    }

    fn rejects_authenticated_peer_identity_change(
        &mut self,
        endpoint: Endpoint,
        next_peer: &PeerInfo,
    ) -> bool {
        if !self.state.is_peer_authenticated_endpoint(endpoint) {
            return false;
        }
        let Some(current_peer) = self.state.peers().get(&endpoint).cloned() else {
            return false;
        };
        if current_peer.user_name == next_peer.user_name
            && current_peer.node_version == next_peer.node_version
            && current_peer.server_port == next_peer.server_port
        {
            return false;
        }
        self.state.add_system_error_message(format!(
            "Security warning: authenticated peer {} attempted to change identity to {}. Disconnecting.",
            current_peer.user_name, next_peer.user_name
        ));
        self.node.network().remove(endpoint.resource_id());
        self.state.disconnected_user(endpoint);
        true
    }

    fn accepts_authenticated_peer_message(
        &mut self,
        endpoint: Endpoint,
        message_kind: &str,
    ) -> bool {
        let Some(user) = self.state.user_name(endpoint).map(|user| user.to_string()) else {
            return false;
        };
        if self.state.is_peer_authenticated_endpoint(endpoint) {
            return true;
        }
        self.state.add_system_warn_message(format!(
            "rejected unauthenticated {} from {}",
            message_kind, user
        ));
        false
    }

    fn initiate_key_exchange(&mut self, endpoint: Endpoint) {
        use ed25519_dalek::Signer;
        let exchange = crate::secure::generate_key_exchange();
        let public_bytes = exchange.public.to_bytes().to_vec();
        let signature = self.signing_key.sign(&public_bytes).to_bytes().to_vec();
        let msg = NetMessage::KeyExchange { public_key: public_bytes, signature };
        self.secure_state.pending_key_exchanges.insert(endpoint, exchange);
        self.node.network().send(endpoint, self.encoder.encode(msg));
    }

    fn process_key_exchange(
        &mut self,
        endpoint: Endpoint,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    ) {
        if let Some(peer_ed_pk) = self.state.peer_public_key(endpoint).cloned() {
            use ed25519_dalek::Verifier;
            let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
                &peer_ed_pk.clone().try_into().unwrap_or([0u8; 32]),
            ) {
                Ok(key) => key,
                Err(_) => {
                    self.state.add_system_warn_message(
                        "KeyExchange: invalid stored peer public key".into(),
                    );
                    return;
                }
            };
            let sig = match ed25519_dalek::Signature::from_slice(&signature) {
                Ok(s) => s,
                Err(_) => {
                    self.state
                        .add_system_warn_message("KeyExchange: invalid signature format".into());
                    return;
                }
            };
            if verifying_key.verify(&public_key, &sig).is_err() {
                self.state
                    .add_system_warn_message("KeyExchange: signature verification failed".into());
                return;
            }
        } else {
            self.state
                .add_system_warn_message("KeyExchange: peer not authenticated, ignoring".into());
            return;
        }

        let peer_x25519: [u8; 32] = match public_key.clone().try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                self.state.add_system_warn_message("KeyExchange: invalid key length".into());
                return;
            }
        };

        if let Some(exchange) = self.secure_state.pending_key_exchanges.remove(&endpoint) {
            let session =
                crate::secure::complete_key_exchange_as_initiator(exchange.secret, &peer_x25519);
            if let Some(session) = session {
                self.secure_state.sessions.insert(endpoint, session);
            }
        } else {
            let exchange = crate::secure::generate_key_exchange();
            let session =
                crate::secure::complete_key_exchange_as_responder(exchange.secret, &peer_x25519);
            if let Some(session) = session {
                self.secure_state.sessions.insert(endpoint, session);

                let our_public_bytes = exchange.public.to_bytes().to_vec();
                use ed25519_dalek::Signer;
                let our_signature = self.signing_key.sign(&our_public_bytes).to_bytes().to_vec();
                let msg = NetMessage::KeyExchange {
                    public_key: our_public_bytes,
                    signature: our_signature,
                };
                self.node.network().send(endpoint, self.encoder.encode(msg));
            }
        }
    }

    fn process_secure_message(&mut self, endpoint: Endpoint, data: Vec<u8>) {
        let inner_message = {
            if let Some(session) = self.secure_state.session_mut(endpoint) {
                session.decrypt(&data)
            } else {
                self.state.add_system_warn_message(
                    "received secure message from peer without active session".into(),
                );
                return;
            }
        };

        let Some(inner_bytes) = inner_message else {
            self.state.add_system_warn_message("failed to decrypt secure message".into());
            return;
        };

        let config = bincode::config::legacy().with_limit::<{ crate::encoder::MAX_FRAME_SIZE }>();
        let inner_msg: NetMessage = match bincode::serde::decode_from_slice(&inner_bytes, config) {
            Ok((msg, _)) => msg,
            Err(_) => {
                self.state.add_system_warn_message(
                    "failed to decode inner message from secure envelope".into(),
                );
                return;
            }
        };

        if !inner_msg.validate() {
            return;
        }

        self.process_network_message(endpoint, inner_msg);
    }

    fn send_secure_to_peer(&mut self, endpoint: Endpoint, message: NetMessage) {
        if self.secure_state.has_session(endpoint) {
            let serialized =
                match bincode::serde::encode_to_vec(&message, bincode::config::legacy()) {
                    Ok(data) => data,
                    Err(e) => {
                        self.state
                            .add_system_error_message(format!("bincode encode failed: {}", e));
                        return;
                    }
                };
            let encrypted = {
                if let Some(session) = self.secure_state.session_mut(endpoint) {
                    session.encrypt(&serialized)
                } else {
                    self.state.add_system_error_message("session lost unexpectedly".to_string());
                    return;
                }
            };
            let mut buf = Vec::new();
            if let Err(e) = bincode::serde::encode_into_std_write(
                NetMessage::Secure(encrypted),
                &mut buf,
                bincode::config::legacy(),
            ) {
                self.state.add_system_error_message(format!("bincode encode failed: {}", e));
                return;
            }
            self.node.network().send(endpoint, &buf);
        } else {
            self.node.network().send(endpoint, self.encoder.encode(message));
        }
    }

    fn broadcast_secure_to_users(&mut self, message: &NetMessage) {
        let endpoints = self.state.all_user_endpoints();
        for endpoint in endpoints {
            self.send_secure_to_peer(endpoint, message.clone());
        }
    }

    fn process_terminal_event(&mut self, term_event: TermEvent) -> Result<()> {
        match term_event {
            TermEvent::Mouse(_) => {}
            TermEvent::Resize(width, height) => {
                self.state.update_chat_viewport(width, height);
            }
            TermEvent::Key(KeyEvent { code, modifiers, kind: KeyEventKind::Press, .. }) => {
                match code {
                    KeyCode::Esc => {
                        self.node.signals().send_with_priority(Signal::Close(None));
                    }
                    KeyCode::Char(character) => {
                        if character == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                            self.node.signals().send_with_priority(Signal::Close(None));
                        } else if self.handle_confirmation_input(character)? {
                            // confirmation consumed the key input
                        } else {
                            self.state.input_write(character);
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(input) = self.state.reset_input() {
                            self.process_input_line(input)?;
                        }
                    }
                    KeyCode::Delete => self.state.input_remove(),
                    KeyCode::Backspace => self.state.input_remove_previous(),
                    KeyCode::Left => self.state.input_move_cursor(CursorMovement::Left),
                    KeyCode::Right => self.state.input_move_cursor(CursorMovement::Right),
                    KeyCode::Home => self.state.input_move_cursor(CursorMovement::Start),
                    KeyCode::End => {
                        if !self.state.input().is_empty() {
                            self.state.input_move_cursor(CursorMovement::End);
                        } else {
                            self.state.messages_scroll(ScrollMovement::End);
                        }
                    }
                    KeyCode::Up if modifiers.contains(KeyModifiers::ALT) => {
                        self.state.scroll_room_list(ScrollMovement::Up);
                    }
                    KeyCode::Down if modifiers.contains(KeyModifiers::ALT) => {
                        self.state.scroll_room_list(ScrollMovement::Down);
                    }
                    KeyCode::Up => {
                        if !self.state.input().is_empty() || self.state.in_history_mode() {
                            self.state.input_history_prev();
                        } else {
                            // In chronological flow, Up scrolls towards older messages (backwards)
                            self.state.messages_scroll(ScrollMovement::Up);
                        }
                    }
                    KeyCode::Down => {
                        if self.state.in_history_mode() {
                            self.state.input_history_next();
                        } else {
                            // In chronological flow, Down scrolls towards newer messages (forwards)
                            self.state.messages_scroll(ScrollMovement::Down);
                        }
                    }
                    KeyCode::PageUp => self.state.messages_scroll(ScrollMovement::Start),
                    KeyCode::PageDown => self.state.messages_scroll(ScrollMovement::End),
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn process_input_line(&mut self, input: String) -> Result<()> {
        match self.commands.find_command(&input).transpose() {
            Ok(Some(ParsedCommand::Action(action))) => self.process_action(action),
            Ok(Some(ParsedCommand::App(command))) => self.process_app_command(command),
            Ok(None) => {
                if input.starts_with(CommandManager::COMMAND_PREFIX) {
                    self.state.add_system_error_message(format!(
                        "Unknown command '{}'. Type /help for available commands.",
                        input
                    ));
                    return Ok(());
                }
                if input.trim().is_empty() {
                    return Ok(());
                }

                let input = expand_shortcodes(&input, &self.art_dict);

                self.state.add_message(ChatMessage::new(
                    format!("{} (me)", self.config.user_name),
                    MessageType::Text(input.clone()),
                ));
                self.broadcast_secure_to_users(&NetMessage::UserMessage(input.clone()));
                self.state.human_streak += 1;
                let trigger = TriggerConfig::from_frequency_and_mode(
                    self.state.ai_frequency.clone(),
                    self.state.ai_mode.clone(),
                );
                if should_intervene(
                    &input,
                    self.state.ai_mode.clone(),
                    &trigger,
                    self.state.ai_thinking,
                    self.state.last_ai_at,
                    self.state.human_streak,
                    Instant::now(),
                ) {
                    if contains_ops_ai_mention(&input) {
                        self.spawn_mention_task(input.clone());
                    } else if self.state.ai_mode == AiMode::Companion {
                        self.spawn_ai_task(AiTask::Companion);
                    } else {
                        self.spawn_ai_task(AiTask::Intervene);
                    }
                }
            }
            Err(error) => error.report_err(&mut self.state),
        }
        Ok(())
    }

    fn process_app_command(&mut self, command: AppCommand) {
        match command {
            AppCommand::Summary(kind) => match kind {
                SummaryCommandKind::Summary => self.spawn_ai_task(AiTask::Summary),
                SummaryCommandKind::Todos => self.spawn_ai_task(AiTask::Todos),
                SummaryCommandKind::Decisions => self.spawn_ai_task(AiTask::Decisions),
                SummaryCommandKind::Context => {
                    self.state.add_system_info_message(self.state.transcript(50))
                }
            },
            AppCommand::SetAiMode(mode) => {
                self.state.ai_mode = mode;
                self.state
                    .add_system_info_message(format!("AI mode set to {}", self.state.ai_mode));
            }
            AppCommand::SetAiQuiet(enabled) => {
                self.state.ai_mode = if enabled { AiMode::Listener } else { AiMode::Clerk };
                self.state.add_system_info_message(if enabled {
                    "AI quiet mode enabled".into()
                } else {
                    "AI quiet mode disabled".into()
                });
            }
            AppCommand::SetAiFrequency(frequency) => {
                self.state.ai_frequency = frequency;
                self.state.add_system_info_message(format!(
                    "AI frequency set to {}",
                    ai_frequency_label(&self.state.ai_frequency)
                ));
            }
            AppCommand::SetAiProvider(provider) => {
                if !self.config.ai.enabled {
                    self.state.add_system_error_message(
                        "AI is disabled in config; cannot switch provider.".into(),
                    );
                    return;
                }
                if self.state.ai_thinking {
                    self.state.add_system_error_message(
                        "Cannot switch AI provider while a request is in flight; wait for it to \
                         finish."
                            .into(),
                    );
                    return;
                }
                let mut ai_config = self.config.ai.clone();
                ai_config.provider = provider;
                match AiMediator::new(&self.workspace, &ai_config, &self.config.language) {
                    Ok(mediator) => {
                        let label = ai_config.provider.label().to_string();
                        self.state.ai_provider = ai_config.provider;
                        self.ai_mediator = Some(Arc::new(mediator));
                        self.state.ai_state = AiState::Idle;
                        self.state.add_system_info_message(format!("AI provider set to {label}"));
                    }
                    Err(error) => {
                        self.state.add_system_error_message(format!(
                            "Failed to set AI provider: {error}"
                        ));
                    }
                }
            }
            AppCommand::RoomCreate { peers, ai_mode } => {
                if let Some(missing_peer) = peers
                    .iter()
                    .find(|peer| !self.state.peer_names().iter().any(|known| known == *peer))
                {
                    self.state.add_system_error_message(format!(
                        "unknown peer '{}'. Use /peers to see connected peers.",
                        missing_peer
                    ));
                    return;
                }
                if let Some(not_ready_peer) =
                    peers.iter().find(|peer| !self.state.peer_is_ready(peer))
                {
                    self.state.add_system_error_message(format!(
                        "peer '{}' is not ready yet; still exchanging peer info.",
                        not_ready_peer
                    ));
                    return;
                }

                let room = self.state.create_room(&peers, ai_mode.clone());
                let member_ids =
                    room.members.iter().map(|member| member.id.clone()).collect::<Vec<_>>();

                let endpoints = self.state.all_user_endpoints();
                // Filter endpoints to only send to those who are in the room
                let target_endpoints = endpoints
                    .into_iter()
                    .filter(|&e| self.state.user_name(e).is_some_and(|name| peers.contains(name)))
                    .collect::<Vec<_>>();

                // Broadcast RoomCreate (V1 or V2 depending on peer version)
                for &endpoint in &target_endpoints {
                    let message = if self.peer_supports_room_create_v2(endpoint) {
                        NetMessage::RoomCreateV2 {
                            room_id: room.id.clone(),
                            members: member_ids.clone(),
                            ai_mode: ai_mode.clone(),
                        }
                    } else {
                        NetMessage::RoomCreate(room.id.clone(), member_ids.clone())
                    };
                    self.send_secure_to_peer(endpoint, message);
                }

                self.state.add_system_info_message(format!("created room {}", room.id));
            }
            AppCommand::RoomList => {
                if self.state.rooms().is_empty() {
                    self.state.add_system_info_message("no rooms".into());
                } else {
                    self.state.add_system_info_message(self.room_list_text());
                }
            }
            AppCommand::RoomSwitch(target) => {
                let room_id = self.resolve_room_switch_target(&target).unwrap_or(target);
                match self.state.switch_room(&room_id) {
                    Ok(()) => {
                        if let Some(room) =
                            self.state.rooms().iter().find(|room| room.id == room_id).cloned()
                        {
                            self.state.add_system_info_message(format!(
                                "Switched to {}",
                                describe_room(&room)
                            ));
                        } else {
                            self.state.add_system_info_message(format!("Switched to {}", room_id));
                        }
                    }
                    Err(error) => self.state.add_system_error_message(error.to_string()),
                }
            }
            AppCommand::Peers => {
                let peers = self.state.peer_names();
                if peers.is_empty() {
                    self.state.add_system_info_message("no peers discovered".into());
                } else {
                    self.state.add_system_info_message(self.peers_text());
                }
            }
            AppCommand::PeerConnect(target) => {
                if let Err(error) = self.connect_peer_target(&target) {
                    self.state.add_system_error_message(format!(
                        "failed to connect to {}: {}",
                        target, error
                    ));
                }
            }
            AppCommand::TrustList => self.state.add_system_info_message(self.trust_list_text()),
            AppCommand::TrustAdd(target) => self.trust_peer_target(&target),
            AppCommand::TrustRemove(target) => self.untrust_peer_target(&target),
            AppCommand::Skills => {
                if self.state.skill_registry().skills().is_empty() {
                    self.state.add_system_info_message(
                        "No skills found. Add skill scripts to .claude/skills/".into(),
                    );
                } else {
                    let mut skills = self
                        .state
                        .skill_registry()
                        .skills()
                        .iter()
                        .map(|skill| {
                            let args = skill
                                .args_hint
                                .as_deref()
                                .map(|hint| format!("  args: {hint}"))
                                .unwrap_or_default();
                            format!(
                                "{:<20} {:<8} {:<12} {}{}",
                                skill.name, skill.risk, skill.invoke_mode, skill.description, args
                            )
                        })
                        .collect::<Vec<_>>();
                    skills.insert(
                        0,
                        format!("{:<20} {:<8} {:<12} {}", "name", "risk", "mode", "description"),
                    );
                    skills.insert(
                        1,
                        "------------------------------------------------------------".into(),
                    );
                    self.state.add_system_info_message(skills.join("\n"));
                }
            }
            AppCommand::Skill { name, args } => self.queue_or_run_skill(name, args),
            AppCommand::RunProposal(index) => {
                let Some(proposal) = self.state.find_skill_proposal(index).cloned() else {
                    self.state.add_system_error_message(format!("unknown proposal id: {}", index));
                    return;
                };
                let source = proposal.source_peer.clone();
                let is_remote = source.is_some();
                if is_remote {
                    let source_name = source.as_deref().unwrap_or("");
                    if !self.state.is_peer_authenticated_by_name(source_name) {
                        self.state.add_system_error_message(format!(
                            "permission denied: proposal {} came from Unauthenticated peer {}.",
                            index, source_name
                        ));
                        return;
                    }
                }
                let permission = self.config.security.default_permission_policy();
                let currently_trusted = if is_remote {
                    proposal
                        .source_fingerprint
                        .as_deref()
                        .map(|fingerprint| self.state.is_trusted_peer(fingerprint))
                        .unwrap_or(false)
                } else {
                    proposal.trusted
                };
                if is_remote && matches!(permission, DefaultPermission::DenyRemoteExec) {
                    self.state.add_system_error_message(
                        "remote proposals are disabled by security.default_permission".into(),
                    );
                    return;
                }
                if is_remote && !currently_trusted {
                    let source = source.unwrap_or_else(|| "unknown peer".into());
                    self.state.add_system_error_message(format!(
                        "permission denied: proposal {} came from untrusted peer {}. Use /trust add {} first.",
                        index, source, source
                    ));
                    return;
                }
                if is_remote && matches!(permission, DefaultPermission::ConfirmRequired) {
                    let Some(meta) =
                        self.state.skill_registry().find(&proposal.skill_name).cloned()
                    else {
                        self.state.add_system_error_message(format!(
                            "unknown skill: {}",
                            proposal.skill_name
                        ));
                        return;
                    };
                    let prompt = format!(
                        "[{}] Execute remote proposal from {}? [y/n]",
                        meta.name,
                        source.unwrap_or_else(|| "unknown peer".into())
                    );
                    self.state
                        .queue_skill_confirmation(PendingSkillExecution { meta, args: Vec::new() });
                    self.state.add_system_info_message(prompt);
                    return;
                }
                self.queue_or_run_skill(proposal.skill_name, Vec::new());
            }
            AppCommand::Cancel => self.cancel_active_task(),
            AppCommand::Avatar(kind) => self.process_avatar_command(kind),
            AppCommand::ArtList => {
                if self.art_dict.is_empty() {
                    self.state.add_system_info_message(
                        "No art shortcodes defined. Place an art.yaml file in ~/.config/triadchat/"
                            .into(),
                    );
                } else {
                    let mut keys: Vec<&str> = self.art_dict.keys().map(String::as_str).collect();
                    keys.sort();
                    self.state
                        .add_system_info_message(format!("Art shortcodes: {}", keys.join(", ")));
                }
            }
            AppCommand::ArtReload => {
                let result = load_art_dictionary(self.art_yaml_path.as_deref());
                match result {
                    Ok(dict) => {
                        let count = dict.len();
                        self.art_dict = dict;
                        self.state.add_system_info_message(format!(
                            "Art dictionary reloaded ({} entries)",
                            count
                        ));
                    }
                    Err(err) => {
                        self.state.add_system_error_message(format!(
                            "Failed to reload art dictionary: {}",
                            err
                        ));
                    }
                }
            }
            AppCommand::Help => {
                self.state.messages_scroll(ScrollMovement::Start);
                self.state.add_system_info_message(help_text());
            }
        }
    }

    fn process_avatar_command(&mut self, kind: AvatarCommandKind) {
        match kind {
            AvatarCommandKind::Set { target, preset } => {
                if !self.avatar_manager.list_all_presets().iter().any(|p| p == &preset) {
                    self.state.add_system_warn_message(format!(
                        "Unknown avatar preset '{}'. Use /avatar list to see available presets.",
                        preset
                    ));
                    return;
                }
                if target == "self" || target == self.state.local_user_name() {
                    self.state.user_avatar = preset.clone();
                    self.state.add_system_info_message(format!("Your avatar set to '{}'", preset));
                } else if target == "@ops-ai" || target == "ops-ai" {
                    self.state.ai_avatar = preset.clone();
                    self.state.add_system_info_message(format!("AI avatar set to '{}'", preset));
                } else {
                    self.state.add_system_warn_message(format!(
                        "Unknown target '{}'. Use 'self', 'ops-ai', or your username.",
                        target
                    ));
                }
            }
            AvatarCommandKind::Preview => {
                let preset = self.state.user_avatar.clone();
                for size in [AvatarSize::Compact, AvatarSize::Normal, AvatarSize::Expressive] {
                    let label = format!("[{size:?}]");
                    let art_spans = self.avatar_manager.render(&preset, AvatarState::Online, size);
                    let art = art_spans
                        .iter()
                        .map(|spans| {
                            spans.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.state.add_system_info_message(format!("{}\n{}", label, art));
                }
            }
            AvatarCommandKind::Mode(mode) => match mode.as_str() {
                "compact" => {
                    self.state.avatar_size = AvatarSize::Compact;
                    self.state.add_system_info_message("Avatar mode set to 'compact'".into());
                }
                "normal" => {
                    self.state.avatar_size = AvatarSize::Normal;
                    self.state.add_system_info_message("Avatar mode set to 'normal'".into());
                }
                "expressive" => {
                    self.state.avatar_size = AvatarSize::Expressive;
                    self.state.add_system_info_message("Avatar mode set to 'expressive'".into());
                }
                other => {
                    self.state.add_system_warn_message(format!(
                        "Unknown avatar mode '{}'. Use compact, normal, or expressive.",
                        other
                    ));
                }
            },
            AvatarCommandKind::List => {
                let presets = self.avatar_manager.list_all_presets();
                self.state.add_system_info_message(format!(
                    "Available avatar presets: {}",
                    presets.join(", ")
                ));
            }
        }
    }

    fn spawn_ai_task(&mut self, task: AiTask) {
        let Some(ai_mediator) = self.ai_mediator.clone() else {
            self.state.add_system_error_message("AI is disabled or sidecar is unavailable".into());
            return;
        };

        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
        }

        self.state.ai_state = AiState::Thinking;
        self.state.ai_thinking = true;
        let transcript = self.state.transcript(100);
        let last_messages = self.state.recent_human_messages(3);

        if self.test_mode {
            match self.runtime.block_on(ai_mediator.request(task, &transcript, &last_messages)) {
                Ok((payload, truncated)) => {
                    self.record_ai_response(payload, true, None, None, true, truncated)
                }
                Err(error) => self.process_ai_failure(error.to_string()),
            }
            return;
        }

        let node = self.node.clone();
        let task_handle = self.runtime.handle().spawn(async move {
            match ai_mediator.request(task, &transcript, &last_messages).await {
                Ok((payload, truncated)) => {
                    node.signals().send(Signal::AiResponse(payload, truncated))
                }
                Err(error) => node.signals().send(Signal::AiFailed(error.to_string())),
            }
        });
        self.state.abort_handle = Some(task_handle.abort_handle());
    }

    /// Like `spawn_ai_task` but passes `message` as the sole entry in `last_messages`,
    /// so the AI answers the direct `@ops-ai` question rather than summarising the transcript.
    fn spawn_mention_task(&mut self, message: String) {
        let Some(ai_mediator) = self.ai_mediator.clone() else {
            self.state.add_system_error_message("AI is disabled or sidecar is unavailable".into());
            return;
        };

        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
        }

        self.state.ai_state = AiState::Thinking;
        self.state.ai_thinking = true;
        let transcript = self.state.transcript(20);
        let last_messages = vec![message];

        if self.test_mode {
            match self.runtime.block_on(ai_mediator.request(
                AiTask::Mention,
                &transcript,
                &last_messages,
            )) {
                Ok((payload, truncated)) => {
                    self.record_ai_response(payload, true, None, None, true, truncated)
                }
                Err(error) => self.process_ai_failure(error.to_string()),
            }
            return;
        }

        let node = self.node.clone();
        let task_handle = self.runtime.handle().spawn(async move {
            match ai_mediator.request(AiTask::Mention, &transcript, &last_messages).await {
                Ok((payload, truncated)) => {
                    node.signals().send(Signal::AiResponse(payload, truncated))
                }
                Err(error) => node.signals().send(Signal::AiFailed(error.to_string())),
            }
        });
        self.state.abort_handle = Some(task_handle.abort_handle());
    }

    fn process_ai_response(&mut self, payload: AiPayload, truncated: bool) {
        self.record_ai_response(payload, true, None, None, true, truncated);
    }

    fn process_remote_ai_response(&mut self, endpoint: Endpoint, payload: AiPayload) {
        let source_peer = self.state.user_name(endpoint).cloned();
        let source_fingerprint = self.state.peer_fingerprint(endpoint);
        let trusted = source_fingerprint
            .as_deref()
            .map(|fingerprint| self.state.is_trusted_peer(fingerprint))
            .unwrap_or(false);
        self.record_ai_response(payload, false, source_peer, source_fingerprint, trusted, false);
    }

    fn record_ai_response(
        &mut self,
        mut payload: AiPayload,
        broadcast: bool,
        source_peer: Option<String>,
        source_fingerprint: Option<String>,
        trusted: bool,
        truncated: bool,
    ) {
        if let Some(structured) = payload.structured.as_mut() {
            if structured.validate() {
                structured.sanitize_skill_suggestions();
            } else {
                payload.structured = None;
            }
        }
        self.state.ai_state = AiState::Idle;
        self.state.ai_thinking = false;
        self.state.last_ai_at = Some(Instant::now());
        self.state.human_streak = 0;
        self.state.abort_handle = None;
        self.state.last_structured_output = payload.structured.clone();
        if let Some(structured) = payload.structured.as_ref() {
            self.state.set_skill_proposals_with_fingerprint(
                &structured.skill_suggestions,
                source_peer,
                source_fingerprint,
                trusted,
            );
        } else {
            self.state.clear_skill_proposals();
        }
        let rendered = render_ai_payload(&payload);
        let message = ChatMessage::new("ops-ai ✦".into(), MessageType::AiText(rendered.clone()));
        let room_id = self
            .state
            .active_room_id()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("solo-{}", self.config.user_name));
        let transcript_entry = TranscriptEntry::ai(
            room_id,
            "ops-ai",
            rendered,
            Some(format!("{:?}", payload.intent).to_ascii_lowercase()),
            payload
                .structured
                .as_ref()
                .and_then(|structured| serde_json::to_value(structured).ok()),
            "ai",
        );
        self.state.add_message_with_transcript(message, transcript_entry);
        if truncated {
            self.state.add_system_warn_message(
                "Warning: Conversation history was truncated due to length limits. AI accuracy may be affected.".into()
            );
        }
        if broadcast {
            self.broadcast_secure_to_users(&NetMessage::AiMessage(payload.clone()));
        }
        self.ring_the_bell();
    }

    fn process_ai_failure(&mut self, error: String) {
        self.state.ai_state = if self.ai_mediator.is_some() {
            AiState::Failed(error.clone())
        } else {
            AiState::Disabled
        };
        self.state.ai_thinking = false;
        self.state.abort_handle = None;
        self.state.add_system_error_message(format!("[ops-ai: failed] {}", error));
    }

    fn process_skill_done(&mut self, payload: SkillResultPayload) {
        self.record_skill_done(payload, true);
    }

    fn record_skill_done(&mut self, payload: SkillResultPayload, broadcast: bool) {
        self.state.ai_state =
            if payload.success { AiState::Idle } else { AiState::Failed(payload.summary.clone()) };
        self.state.ai_thinking = false;
        self.state.abort_handle = None;
        self.state.clear_pending_confirmation();
        let text = if payload.success {
            format!("{}: {}", payload.skill_name, payload.summary)
        } else {
            format!("[ops-ai: failed] {}", payload.summary)
        };
        let message = ChatMessage::new("ops-ai ✦".into(), MessageType::AiText(text.clone()));
        let room_id = self
            .state
            .active_room_id()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("solo-{}", self.config.user_name));
        let transcript_entry = TranscriptEntry::ai(room_id, "ops-ai", text, None, None, "skill");
        self.state.add_message_with_transcript(message, transcript_entry);
        if broadcast {
            self.broadcast_secure_to_users(&NetMessage::SkillResult(payload));
        }
    }

    fn process_action(&mut self, mut action: Box<dyn Action>) {
        match action.process(&mut self.state, self.node.network()) {
            Processing::Completed => {}
            Processing::Partial(delay) => {
                self.node.signals().send_with_timer(Signal::Action(action), delay);
            }
        }
    }

    pub fn node_handler(&self) -> NodeHandler<Signal> {
        self.node.clone()
    }

    pub fn runtime_handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn set_ai_thinking_for_test(&mut self, thinking: bool) {
        self.state.ai_thinking = thinking;
    }

    pub fn has_secure_session(&self, endpoint: Endpoint) -> bool {
        self.secure_state.has_session(endpoint)
    }

    pub fn handle_input_line_for_test(&mut self, input: &str) -> Result<()> {
        self.process_input_line(input.to_string())
    }

    pub fn handle_confirmation_input_for_test(&mut self, character: char) -> Result<()> {
        let _ = self.handle_confirmation_input(character)?;
        Ok(())
    }

    pub fn local_server_port_for_test(&self) -> Option<u16> {
        self.local_server_port
    }

    pub fn connect_peer_for_test(&mut self, server_port: u16) -> Result<()> {
        let server_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, server_port));
        self.connect_peer_addr(server_addr)
    }

    pub fn inject_remote_ai_response_for_test(&mut self, source_peer: &str, payload: AiPayload) {
        let source_fingerprint = Some(format!("fp:{}:test", source_peer));
        self.record_ai_response(
            payload,
            false,
            Some(source_peer.to_string()),
            source_fingerprint,
            false,
            false,
        );
    }

    pub fn inject_authenticated_peer_for_test(&mut self, user_name: &str, fingerprint: &str) {
        let id = message_io::network::ResourceId::from(130);
        let addr = "127.0.0.1:0".parse().unwrap();
        let endpoint = message_io::network::Endpoint::from_listener(id, addr);

        let peer = PeerInfo {
            user_name: user_name.to_string(),
            server_port: 5877,
            node_version: env!("CARGO_PKG_VERSION").to_string(),
            avatar: String::new(),
        };

        self.state.record_peer(endpoint, peer);
        self.state.add_verified_peer_fingerprint(endpoint, fingerprint.to_string());
    }

    pub fn authenticate_endpoint_for_test(&mut self, endpoint: Endpoint, fingerprint: &str) {
        self.state.add_verified_peer_fingerprint(endpoint, fingerprint.to_string());
    }

    fn process_user_data(&mut self, user_name: &str, file_name: &str, chunk: Chunk) {
        use std::io::Write;
        let sanitized_user = sanitize_filename(user_name);
        let sanitized_file = sanitize_filename(file_name);

        match chunk {
            Chunk::Error => {
                if let Some(temp_path) = self.state.remove_transfer(&sanitized_user, file_name) {
                    let _ = std::fs::remove_file(temp_path);
                }
                format!("'{}' had an error while sending '{}'", user_name, file_name)
                    .report_err(&mut self.state);
            }
            Chunk::End => {
                let temp_path = self.state.remove_transfer(&sanitized_user, file_name);
                let data_dir = self
                    .state
                    .downloads_base_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join("triadchat/downloads")
                    .join(&sanitized_user);

                let finalize_result = if let Some(temp_path) = temp_path {
                    finalize_transfer(&temp_path, &data_dir, &sanitized_file)
                } else {
                    let unique_id = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                    let temp_name = format!(".{}.tmp_{}", sanitized_file, unique_id);
                    let temp_path = data_dir.join(temp_name);
                    std::fs::create_dir_all(&data_dir)
                        .and_then(|_| {
                            std::fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .truncate(true)
                                .open(&temp_path)
                        })
                        .map_err(anyhow::Error::from)
                        .and_then(|_| finalize_transfer(&temp_path, &data_dir, &sanitized_file))
                };

                match finalize_result {
                    Ok(final_path) => {
                        format!(
                            "Successfully received file '{}' from user '{}'! Saved to: {}",
                            sanitized_file,
                            user_name,
                            final_path.display()
                        )
                        .report_info(&mut self.state);
                        self.ring_the_bell();
                    }
                    Err(e) => {
                        format!(
                            "Failed to finalize received file '{}' from user '{}': {}",
                            file_name, user_name, e
                        )
                        .report_err(&mut self.state);
                    }
                }
            }
            Chunk::Data(data) => {
                let mut try_write = || -> Result<()> {
                    let temp_path = if let Some(path) =
                        self.state.get_transfer_temp_path(&sanitized_user, file_name)
                    {
                        path.clone()
                    } else {
                        let data_dir = self
                            .state
                            .downloads_base_dir()
                            .unwrap_or_else(std::env::temp_dir)
                            .join("triadchat/downloads")
                            .join(&sanitized_user);
                        std::fs::create_dir_all(&data_dir)?;

                        let unique_id = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                        let temp_name = format!(".{}.tmp_{}", sanitized_file, unique_id);
                        let temp_path = data_dir.join(temp_name);

                        std::fs::OpenOptions::new()
                            .create(true)
                            .write(true)
                            .truncate(true)
                            .open(&temp_path)?;

                        self.state.start_transfer(
                            sanitized_user.clone(),
                            file_name.to_string(),
                            temp_path.clone(),
                        );
                        temp_path
                    };

                    std::fs::OpenOptions::new().append(true).open(temp_path)?.write_all(&data)?;
                    Ok(())
                };

                try_write().report_if_err(&mut self.state);
            }
        }
    }

    pub fn inject_receive_chunk_for_test(
        &mut self,
        file_name: &str,
        chunk: Chunk,
        user_name: &str,
    ) {
        self.process_user_data(user_name, file_name, chunk);
    }

    pub fn connect_raw_for_test(&mut self, server_addr: SocketAddr) -> Result<Endpoint> {
        let (endpoint, _) = self.node.network().connect_sync(Transport::FramedTcp, server_addr)?;
        Ok(endpoint)
    }

    pub fn inject_network_message_for_test(&mut self, endpoint: Endpoint, message: NetMessage) {
        self.process_network_message(endpoint, message);
    }

    pub fn ring_the_bell(&self) {
        if self.config.terminal_bell {
            print!("\x07");
        }
    }

    fn send_peer_info(&self, endpoint: Endpoint) {
        let Some(server_port) = self.local_server_port else {
            return;
        };

        let peer = PeerInfo {
            user_name: self.config.user_name.clone(),
            server_port,
            node_version: env!("CARGO_PKG_VERSION").into(),
            avatar: self.state.user_avatar.clone(),
        };
        let mut encoder = Encoder::new();
        self.node.network().send(endpoint, encoder.encode(NetMessage::PeerInfo(peer)));

        let timestamp = chrono::Utc::now().timestamp() as u64;
        let payload = SignaturePayload {
            user_name: self.config.user_name.clone(),
            node_version: env!("CARGO_PKG_VERSION").into(),
            server_port,
            timestamp,
        };
        if let Ok(serialized) = bincode::serde::encode_to_vec(&payload, bincode::config::legacy()) {
            use ed25519_dalek::Signer;
            let signature = self.signing_key.sign(&serialized).to_bytes().to_vec();
            let public_key = self.signing_key.verifying_key().to_bytes().to_vec();
            let identity_msg = NetMessage::PeerIdentity { public_key, signature, timestamp };
            self.node.network().send(endpoint, encoder.encode(identity_msg));
        }
    }

    fn peer_supports_room_create_v2(&self, endpoint: Endpoint) -> bool {
        self.state
            .peers()
            .get(&endpoint)
            .map(|peer| version_supports_room_create_v2(&peer.node_version))
            .unwrap_or(false)
    }

    fn announce_presence(&mut self) -> Result<()> {
        let server_port = self
            .local_server_port
            .ok_or_else(|| anyhow::anyhow!("network has not been started"))?;
        let (discovery_endpoint, _) =
            self.node.network().connect_sync(Transport::Udp, self.config.discovery_addr)?;
        let message = NetMessage::HelloLan(self.config.user_name.clone(), server_port);
        self.node.network().send(discovery_endpoint, self.encoder.encode(message));
        Ok(())
    }

    fn schedule_discovery_retry(&self) {
        self.node.signals().send_with_timer(Signal::DiscoveryRetry, Duration::from_millis(250));
    }

    fn handle_discovery_retry(&mut self) {
        if !self.state.peer_names().is_empty() || self.discovery_retries_remaining == 0 {
            if self.state.peer_names().is_empty() {
                self.state.add_system_info_message(
                    "No peers discovered yet. You can retry discovery or use /peer connect <host:port>."
                        .into(),
                );
            }
            return;
        }
        self.discovery_retries_remaining = self.discovery_retries_remaining.saturating_sub(1);
        let _ = self.announce_presence();
        if self.discovery_retries_remaining > 0 {
            self.schedule_discovery_retry();
        }
    }

    fn connect_peer_addr(&mut self, server_addr: SocketAddr) -> Result<()> {
        let (endpoint, _) = self.node.network().connect_sync(Transport::FramedTcp, server_addr)?;
        self.node.network().send(
            endpoint,
            self.encoder.encode(NetMessage::HelloUser(self.config.user_name.clone())),
        );
        self.send_peer_info(endpoint);
        self.state.add_system_info_message(format!("connecting to peer at {}", server_addr));
        Ok(())
    }

    fn connect_peer_target(&mut self, target: &str) -> Result<()> {
        let server_addr = target
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("no socket address resolved"))?;
        self.connect_peer_addr(server_addr)
    }

    fn queue_or_run_skill(&mut self, name: String, args: Vec<String>) {
        let Some(meta) = self.state.skill_registry().find(&name).cloned() else {
            self.state.add_system_error_message(format!("unknown skill: {}", name));
            return;
        };

        match meta.invoke_mode {
            InvokeMode::Suggest => {
                self.state.add_system_info_message(format!(
                    "Skill '{}' is propose-only: the AI suggests it when relevant,\nbut it cannot be run manually with /skill. Wait for a proposal in the\nstatus panel and use /run <id> to accept.",
                    meta.name
                ));
            }
            InvokeMode::AutoSafe if meta.risk == RiskLevel::Low => {
                self.start_skill_execution(PendingSkillExecution { meta, args });
            }
            InvokeMode::Confirm | InvokeMode::Manual | InvokeMode::AutoSafe => {
                let prompt = format!("[{}] Execute this skill? [y/n]", meta.name);
                self.state.queue_skill_confirmation(PendingSkillExecution { meta, args });
                self.state.add_system_info_message(prompt);
            }
        }
    }

    fn handle_confirmation_input(&mut self, character: char) -> Result<bool> {
        if self.state.pending_confirmation().is_none() {
            return Ok(false);
        }

        match character.to_ascii_lowercase() {
            'y' => {
                if let Some(pending) = self.state.take_pending_confirmation() {
                    self.start_skill_execution(pending);
                }
                Ok(true)
            }
            'n' => {
                self.state.clear_pending_confirmation();
                self.state.add_system_info_message("skill execution cancelled".into());
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn start_skill_execution(&mut self, pending: PendingSkillExecution) {
        let Some(ai_mediator) = self.ai_mediator.clone() else {
            self.state.add_system_error_message("AI sidecar is unavailable".into());
            return;
        };

        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
        }

        self.state.ai_state = AiState::Acting;
        self.state.add_system_info_message(format!("[ops-ai: running /{}...]", pending.meta.name));

        if self.test_mode {
            let payload = self.runtime.block_on(SkillExecutor::run(
                ai_mediator.as_ref(),
                &pending.meta,
                &pending.args,
            ));
            self.process_skill_done(payload);
            return;
        }

        let node = self.node.clone();
        let task_handle = self.runtime.handle().spawn(async move {
            let payload =
                SkillExecutor::run(ai_mediator.as_ref(), &pending.meta, &pending.args).await;
            node.signals().send(Signal::SkillDone(payload));
        });
        self.state.abort_handle = Some(task_handle.abort_handle());
    }

    fn cancel_active_task(&mut self) {
        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
            self.state.ai_state = AiState::Idle;
            self.state.ai_thinking = false;
            self.state.clear_pending_confirmation();
            self.state.add_system_info_message("active task cancelled".into());
        } else if self.state.pending_confirmation().is_some() {
            self.state.clear_pending_confirmation();
            self.state.add_system_info_message("skill execution cancelled".into());
        } else {
            self.state.add_system_info_message("no active task".into());
        }
    }

    fn persist_trusted_peer_fingerprint(&self, fingerprint: &str) {
        self.update_stored_config(|stored| {
            if !stored.security.trusted_peers.iter().any(|known| known == fingerprint) {
                stored.security.trusted_peers.push(fingerprint.to_string());
            }
        });
    }

    fn remove_trusted_peer_fingerprint(&self, fingerprint: &str) {
        self.update_stored_config(|stored| {
            stored.security.trusted_peers.retain(|known| known != fingerprint);
        });
    }

    fn update_stored_config(&self, update: impl FnOnce(&mut Config)) {
        let Some(path) = self.config_file_path.as_ref() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut stored = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| toml::from_str::<Config>(&raw).ok())
            .unwrap_or_else(|| self.config.clone());
        update(&mut stored);
        if let Ok(serialized) = toml::to_string(&stored) {
            let _ = std::fs::write(path, serialized);
        }
    }

    fn resolve_peer_or_fingerprint(&self, target: &str) -> Option<(String, String)> {
        self.state
            .peer_fingerprint_by_name(target)
            .map(|fingerprint| (target.to_string(), fingerprint))
            .or_else(|| {
                self.state
                    .trusted_peer_fingerprints()
                    .into_iter()
                    .find(|fingerprint| fingerprint == target)
                    .map(|fingerprint| (target.to_string(), fingerprint))
            })
    }

    fn trust_peer_target(&mut self, target: &str) {
        let Some((label, fingerprint)) = self.resolve_peer_or_fingerprint(target) else {
            self.state.add_system_error_message(format!(
                "unknown peer or fingerprint '{}'. Use /peers or /trust list.",
                target
            ));
            return;
        };
        self.state.trust_peer_fingerprint(fingerprint.clone());
        self.state.set_skill_proposal_trust(&label, true);
        self.state.set_skill_proposal_trust_by_fingerprint(&fingerprint, true);
        self.persist_trusted_peer_fingerprint(&fingerprint);
        self.state.add_system_info_message(format!(
            "trusted peer {} ({})",
            label,
            short_fingerprint(&fingerprint)
        ));
    }

    fn untrust_peer_target(&mut self, target: &str) {
        let Some((label, fingerprint)) = self.resolve_peer_or_fingerprint(target) else {
            self.state.add_system_error_message(format!(
                "unknown peer or fingerprint '{}'. Use /trust list.",
                target
            ));
            return;
        };
        self.state.untrust_peer_fingerprint(&fingerprint);
        self.state.set_skill_proposal_trust(&label, false);
        self.state.set_skill_proposal_trust_by_fingerprint(&fingerprint, false);
        self.remove_trusted_peer_fingerprint(&fingerprint);
        self.state.add_system_info_message(format!(
            "removed trust for {} ({})",
            label,
            short_fingerprint(&fingerprint)
        ));
    }

    fn room_list_text(&self) -> String {
        let mut lines = vec![format!("Rooms ({}):", self.state.rooms().len())];
        for (index, room) in self.state.rooms().iter().enumerate() {
            let active = self.state.active_room_id() == Some(room.id.as_str());
            let active_marker = if active { "*" } else { " " };
            let active_suffix = if active { "  ← active" } else { "" };
            lines.push(format!(
                "  {index_num} {active_marker} {room}{active_suffix}",
                index_num = index + 1,
                room = describe_room_list_entry(room),
            ));
        }
        lines.join("\n")
    }

    fn peers_text(&self) -> String {
        let active_members = self
            .state
            .rooms()
            .iter()
            .find(|room| self.state.active_room_id() == Some(room.id.as_str()))
            .map(human_member_names)
            .unwrap_or_default();

        let mut lines = vec![format!("Connected peers ({}):", self.state.peer_names().len())];
        for peer in self.state.peer_names() {
            let Some(endpoint) = self.state.peer_endpoint_by_name(&peer) else {
                warn!("peer listed without endpoint: {peer}");
                lines.push(format!("  {peer}  [unavailable]"));
                continue;
            };
            let room_status = if active_members.iter().any(|member| member == &peer) {
                "in room"
            } else {
                "available"
            };
            let readiness = match self.state.peer_readiness(endpoint) {
                PeerReadiness::Ready => "ready",
                PeerReadiness::Connecting => "connecting",
            };
            let Some(fingerprint) = self.state.peer_fingerprint(endpoint) else {
                warn!("peer endpoint has no fingerprint: {peer}");
                lines
                    .push(format!("  {peer}  [{room_status}, {readiness}, untrusted]  fp=unknown"));
                continue;
            };
            let is_authenticated = self.state.is_peer_authenticated_by_name(&peer);
            let auth_status = if is_authenticated { "" } else { ", unauthenticated" };
            let trust =
                if self.state.is_trusted_peer(&fingerprint) { "trusted" } else { "untrusted" };
            lines.push(format!(
                "  {peer}  [{room_status}, {readiness}, {trust}{auth_status}]  fp={fingerprint}",
            ));
        }
        lines.join("\n")
    }

    fn trust_list_text(&self) -> String {
        let fingerprints = self.state.trusted_peer_fingerprints();
        if fingerprints.is_empty() {
            return "Trusted peers (0):".into();
        }
        let mut lines = vec![format!("Trusted peers ({}):", fingerprints.len())];
        for fingerprint in fingerprints {
            lines.push(format!("  {}", fingerprint));
        }
        lines.join("\n")
    }

    fn resolve_room_switch_target(&self, target: &str) -> Option<String> {
        if let Ok(index) = target.parse::<usize>() {
            return index
                .checked_sub(1)
                .and_then(|idx| self.state.rooms().get(idx))
                .map(|room| room.id.clone());
        }
        self.state.rooms().iter().find(|room| room.id == target).map(|room| room.id.clone())
    }
}

fn describe_room(room: &Room) -> String {
    format!(
        "{} [{}] — AI: {}",
        room.id,
        human_member_names(room).join(", "),
        room.ai_mode.as_ref().map(AiMode::to_string).unwrap_or_else(|| "off".to_string())
    )
}

fn describe_room_list_entry(room: &Room) -> String {
    format!(
        "{} [{}] mode: {}",
        room.id,
        human_member_names(room).join(", "),
        room.ai_mode.as_ref().map(AiMode::to_string).unwrap_or_else(|| "off".to_string())
    )
}

fn human_member_names(room: &Room) -> Vec<String> {
    room.members
        .iter()
        .filter(|member| member.kind == MemberKind::Human)
        .map(|member| member.id.clone())
        .collect()
}

fn help_text() -> String {
    let mut out = String::new();
    let sections = [
        (
            "AI",
            vec![
                ("/ai mode <mode>", "Change AI behaviour mode:"),
                ("", "  clerk      Active assistant (auto-intervenes)"),
                ("", "  listener   Passive observer (only responds to mentions)"),
                ("", "  moderator  Focuses on decisions and conflicts"),
                ("", "  operator   Executes skills on request"),
                ("", "  companion  Casual conversational partner"),
                ("/ai quiet <on|off>", "Mute/unmute AI responses"),
                ("/ai freq <low|normal|high>", "Adjust AI intervention frequency"),
                ("/ai provider <provider>", "Switch AI engine (claude, codex, gemini, custom)"),
            ],
        ),
        (
            "Summary",
            vec![
                ("/summary", "Summarise the conversation"),
                ("/todos", "List action items"),
                ("/decisions", "List decisions made"),
                ("/context", "Summarise context"),
            ],
        ),
        (
            "Rooms",
            vec![
                ("/room create @user [--ai <mode>]", "Create a room with peers"),
                ("/room list", "List all rooms"),
                ("/room switch <id|name>", "Switch active room"),
            ],
        ),
        (
            "Peers",
            vec![
                ("/peers", "List connected peers"),
                ("/peer connect <host:port>", "Connect to a peer directly"),
                ("/trust list", "List trusted peer fingerprints"),
                ("/trust add <peer|fp>", "Trust a peer explicitly"),
                ("/trust remove <peer|fp>", "Remove stored peer trust"),
            ],
        ),
        (
            "Skills",
            vec![
                ("/skills", "List available skills"),
                ("/skill <name> [args]", "Run a skill manually"),
                ("/run <id>", "Accept a skill proposal from AI"),
                ("/cancel", "Cancel current AI task or skill"),
            ],
        ),
        (
            "Avatar",
            vec![
                ("/avatar set <target> <preset>", "Set avatar (target: self, @ops-ai)"),
                ("/avatar list", "List available avatar presets"),
                ("/avatar preview", "Preview your current avatar"),
                ("/avatar mode <size>", "Set size: compact, normal, expressive"),
            ],
        ),
        (
            "Art",
            vec![
                ("/art list", "List configured art shortcodes"),
                ("/art reload", "Reload art.yaml"),
            ],
        ),
        ("Files", vec![("/send <file>", "Send a file to peers in the room")]),
    ];

    for (i, (title, commands)) in sections.into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("【 {} 】\n", title));
        for (cmd, desc) in commands {
            if cmd.is_empty() {
                out.push_str(&format!("    {}\n", desc));
            } else {
                out.push_str(&format!(" {:<28} {}\n", cmd, desc));
            }
        }
    }
    out
}

impl Drop for Application<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
        }
        self.node.stop();
    }
}

fn ai_frequency_label(frequency: &AiFrequency) -> &'static str {
    match frequency {
        AiFrequency::Low => "low",
        AiFrequency::Normal => "normal",
        AiFrequency::High => "high",
    }
}

fn load_art_dictionary(
    path: Option<&std::path::Path>,
) -> std::result::Result<HashMap<String, String>, String> {
    let Some(path) = path else {
        return Err("no art.yaml path configured".into());
    };
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_yaml::from_str::<HashMap<String, String>>(&contents)
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

fn expand_shortcodes(input: &str, art_dict: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(open_rel) = input[cursor..].find('[') {
        let open = cursor + open_rel;
        result.push_str(&input[cursor..open]);

        let key_start = open + '['.len_utf8();
        if let Some(close_rel) = input[key_start..].find(']') {
            let close = key_start + close_rel;
            let key = &input[key_start..close];
            if let Some(art) = art_dict.get(key) {
                result.push_str(art);
                cursor = close + ']'.len_utf8();
                continue;
            }
        }

        result.push('[');
        cursor = key_start;
    }

    result.push_str(&input[cursor..]);
    result
}

fn short_fingerprint(fingerprint: &str) -> &str {
    fingerprint.get(..12).unwrap_or(fingerprint)
}

fn version_supports_room_create_v2(version: &str) -> bool {
    parse_semver_tuple(version).map(|parsed| parsed >= (0, 1, 1)).unwrap_or(false)
}

fn parse_semver_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn render_ai_payload(payload: &AiPayload) -> String {
    if let Some(structured) = &payload.structured {
        match payload.intent {
            AiIntent::Todo if !structured.todos.is_empty() => structured
                .todos
                .iter()
                .map(|todo| match &todo.assignee {
                    Some(assignee) => format!("TODO: {} ({})", todo.text, assignee),
                    None => format!("TODO: {}", todo.text),
                })
                .collect::<Vec<_>>()
                .join("\n"),
            AiIntent::Decision if !structured.decisions.is_empty() => structured
                .decisions
                .iter()
                .map(|decision| format!("Decision: {}", decision))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => payload.text.clone(),
        }
    } else {
        payload.text.clone()
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let raw_base = name.split(['/', '\\']).next_back().unwrap_or("");

    let sanitized: String = raw_base
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();

    let upper = sanitized.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." || is_reserved {
        "safe_file".to_string()
    } else {
        sanitized
    }
}

pub fn finalize_transfer(
    temp_path: &std::path::Path,
    sandbox_dir: &std::path::Path,
    sanitized_filename: &str,
) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(sandbox_dir)?;

    let mut final_path = sandbox_dir.join(sanitized_filename);
    let mut counter = 1;

    let path_obj = std::path::Path::new(sanitized_filename);
    let stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or(sanitized_filename);
    let extension = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("");

    loop {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&final_path) {
            Ok(_) => match std::fs::rename(temp_path, &final_path) {
                Ok(_) => break,
                Err(e) => {
                    if e.raw_os_error() == Some(18) {
                        std::fs::copy(temp_path, &final_path)?;
                        let _ = std::fs::remove_file(temp_path);
                        break;
                    } else {
                        let _ = std::fs::remove_file(&final_path);
                        return Err(e.into());
                    }
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let new_name = if extension.is_empty() {
                    format!("{}_{}", stem, counter)
                } else {
                    format!("{}_{}.{}", stem, counter, extension)
                };
                final_path = sandbox_dir.join(new_name);
                counter += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(final_path)
}

fn version_supports_peer_identity(version: &str) -> bool {
    if let Ok(ver) = semver::Version::parse(version) {
        if let Ok(req) = semver::VersionReq::parse(">=0.1.2") {
            return req.matches(&ver);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{AiIntent, AiPayload, StructuredOutput, TodoItem};

    #[test]
    fn render_todo_with_assignee() {
        let payload = AiPayload {
            text: "x".into(),
            intent: AiIntent::Todo,
            structured: Some(StructuredOutput {
                todos: vec![TodoItem { text: "foo".into(), assignee: Some("bar".into()) }],
                ..StructuredOutput::default()
            }),
        };
        assert_eq!(render_ai_payload(&payload), "TODO: foo (bar)");
    }

    #[test]
    fn render_todo_without_assignee() {
        let payload = AiPayload {
            text: "x".into(),
            intent: AiIntent::Todo,
            structured: Some(StructuredOutput {
                todos: vec![TodoItem { text: "foo".into(), assignee: None }],
                ..StructuredOutput::default()
            }),
        };
        assert_eq!(render_ai_payload(&payload), "TODO: foo");
    }

    #[test]
    fn render_decision() {
        let payload = AiPayload {
            text: "x".into(),
            intent: AiIntent::Decision,
            structured: Some(StructuredOutput {
                decisions: vec!["auth".into()],
                ..StructuredOutput::default()
            }),
        };
        assert_eq!(render_ai_payload(&payload), "Decision: auth");
    }

    #[test]
    fn render_falls_back_to_text_for_empty_todos() {
        let payload = AiPayload {
            text: "fallback text".into(),
            intent: AiIntent::Todo,
            structured: Some(StructuredOutput::default()),
        };
        assert_eq!(render_ai_payload(&payload), "fallback text");
    }

    #[test]
    fn render_returns_text_when_structured_is_none() {
        let payload =
            AiPayload { text: "raw text".into(), intent: AiIntent::Clarify, structured: None };
        assert_eq!(render_ai_payload(&payload), "raw text");
    }

    #[test]
    fn expand_shortcodes_replaces_known_codes_without_losing_unicode() {
        let mut art_dict = HashMap::new();
        art_dict.insert("猫".to_string(), "Neko".to_string());
        art_dict.insert("wave".to_string(), "o/".to_string());

        assert_eq!(expand_shortcodes("[猫] says [wave]", &art_dict), "Neko says o/");
    }

    #[test]
    fn expand_shortcodes_leaves_unknown_or_unclosed_codes_intact() {
        let mut art_dict = HashMap::new();
        art_dict.insert("ok".to_string(), "OK".to_string());

        assert_eq!(
            expand_shortcodes("[nope] [ok] [unterminated", &art_dict),
            "[nope] OK [unterminated"
        );
    }
}
