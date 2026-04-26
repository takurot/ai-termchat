use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use message_io::events::EventReceiver;
use message_io::network::{Endpoint, Transport};
use message_io::node::{
    self, NodeHandler, NodeTask, StoredNetEvent as NetEvent, StoredNodeEvent as NodeEvent,
};

use crate::action::{Action, Processing};
use crate::ai::trigger::{contains_ops_ai_mention, should_intervene, TriggerConfig};
use crate::ai::{AiMediator, AiTask};
use crate::avatar::loader::AvatarManager;
use crate::avatar::{AvatarSize, AvatarState};
use crate::commands::ai_cmd::AiCommand;
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
use crate::message::{AiIntent, AiPayload, Chunk, NetMessage, PeerInfo, SkillResultPayload};
use crate::room::transcript::TranscriptEntry;
use crate::room::{MemberKind, Room};
use crate::renderer::Renderer;
use crate::state::{
    AiFrequency, AiMode, AiState, ChatMessage, CursorMovement, MessageType, ScrollMovement, State,
    Window, PeerReadiness,
};
use crate::skill::executor::{PendingSkillExecution, SkillExecutor};
use crate::skill::registry::{InvokeMode, RiskLevel};
use crate::terminal_events::TerminalEventCollector;
use crate::util::{Error, Reportable, Result};

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
    avatar_manager: AvatarManager,
    test_mode: bool,
    local_server_port: Option<u16>,
    config_file_path: Option<PathBuf>,
    discovery_retries_remaining: usize,
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
            .with(AvatarCommand);
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
            avatar_manager,
            test_mode: !collect_terminal_events,
            local_server_port: None,
            config_file_path,
            discovery_retries_remaining: 0,
        })
    }

    pub fn run(&mut self, out: impl std::io::Write) -> Result<()> {
        let mut renderer =
            if self._terminal_events.is_some() { Some(Renderer::new(out)?) } else { None };
        if let Some(renderer) = renderer.as_mut() {
            renderer.render(&self.state, self.config, &self.avatar_manager)?;
        }

        self.start_network()?;

        loop {
            if !self.process_next_event()? {
                return Ok(());
            }
            if let Some(renderer) = renderer.as_mut() {
                renderer.render(&self.state, self.config, &self.avatar_manager)?;
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
                NetEvent::Message(endpoint, message) => match encoder::decode(&message) {
                    Some(net_message) => self.process_network_message(endpoint, net_message),
                    None => self.state.add_system_warn_message(format!(
                        "ignored incompatible message from {}",
                        endpoint.addr()
                    )),
                },
                NetEvent::Disconnected(endpoint) => {
                    self.state.disconnected_user(endpoint);
                    self.state.windows.remove(&endpoint);
                    self.righ_the_bell();
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
                self.righ_the_bell();
            }
            NetMessage::UserMessage(content) => {
                if let Some(user) = self.state.user_name(endpoint) {
                    self.state
                        .add_message(ChatMessage::new(user.into(), MessageType::Text(content)));
                    self.righ_the_bell();
                }
            }
            NetMessage::UserData(file_name, chunk) => {
                use std::io::Write;
                if let Some(user) = self.state.user_name(endpoint).cloned() {
                    match chunk {
                        Chunk::Error => {
                            format!("'{}' had an error while sending '{}'", user, file_name)
                                .report_err(&mut self.state);
                        }
                        Chunk::End => {
                            format!(
                                "Successfully received file '{}' from user '{}'!",
                                file_name, user
                            )
                            .report_info(&mut self.state);
                            self.righ_the_bell();
                        }
                        Chunk::Data(data) => {
                            let try_write = || -> Result<()> {
                                let user_path = std::env::temp_dir().join("triadchat").join(&user);
                                match std::fs::create_dir_all(&user_path) {
                                    Ok(_) => {}
                                    Err(ref err) if err.kind() == ErrorKind::AlreadyExists => {}
                                    Err(error) => return Err(error.into()),
                                }

                                let file_path = user_path.join(file_name);
                                std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(file_path)?
                                    .write_all(&data)?;
                                Ok(())
                            };

                            try_write().report_if_err(&mut self.state);
                        }
                    }
                }
            }
            NetMessage::Stream(data) => match data {
                Some((data, width, height)) if data.len() == width * height / 2 => {
                    self.state
                        .windows
                        .entry(endpoint)
                        .or_insert_with(|| Window::new(width, height));
                    self.state.update_window(&endpoint, data, width, height);
                }
                _ => {
                    self.state.windows.remove(&endpoint);
                }
            },
            NetMessage::AiMessage(payload) => self.process_remote_ai_response(endpoint, payload),
            NetMessage::PeerInfo(peer) => {
                self.state.connected_user(endpoint, &peer.user_name);
                self.state.record_peer(endpoint, peer.clone());
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
                }
            }
            NetMessage::RoomCreate(room_id, member_ids) => {
                if member_ids.iter().any(|member_id| member_id == &self.config.user_name) {
                    let room = self.state.accept_room(&room_id, &member_ids, None);
                    self.state.add_system_info_message(format!("joined room {}", room.id));
                    self.node
                        .network()
                        .send(endpoint, self.encoder.encode(NetMessage::RoomJoin(room.id.clone())));
                }
            }
            NetMessage::RoomCreateV2 { room_id, members, ai_mode } => {
                if members.iter().any(|member_id| member_id == &self.config.user_name) {
                    let room = self.state.accept_room(&room_id, &members, ai_mode);
                    self.state.add_system_info_message(format!("joined room {}", room.id));
                    self.node
                        .network()
                        .send(endpoint, self.encoder.encode(NetMessage::RoomJoin(room.id.clone())));
                }
            }
            NetMessage::RoomJoin(room_id) => {
                let _ = self.state.switch_room(&room_id);
                self.state.add_system_info_message(format!("room {} is ready", room_id));
            }
            NetMessage::SkillResult(payload) => self.record_skill_done(payload, false),
        }
    }

    fn process_terminal_event(&mut self, term_event: TermEvent) -> Result<()> {
        match term_event {
            TermEvent::Mouse(_) | TermEvent::Resize(_, _) => {}
            TermEvent::Key(KeyEvent { code, modifiers, .. }) => match code {
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
                KeyCode::End => self.state.input_move_cursor(CursorMovement::End),
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
                        self.state.messages_scroll(ScrollMovement::Up);
                    }
                }
                KeyCode::Down => {
                    if self.state.in_history_mode() {
                        self.state.input_history_next();
                    } else {
                        self.state.messages_scroll(ScrollMovement::Down);
                    }
                }
                KeyCode::PageUp => self.state.messages_scroll(ScrollMovement::Start),
                _ => {}
            },
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

                self.state.add_message(ChatMessage::new(
                    format!("{} (me)", self.config.user_name),
                    MessageType::Text(input.clone()),
                ));
                let message = self.encoder.encode(NetMessage::UserMessage(input.clone()));
                let endpoints = self.state.all_user_endpoints();
                crate::util::send_all(self.node.network(), &endpoints, message)
                    .report_if_err(&mut self.state);
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
                self.state.add_system_info_message(format!(
                    "AI mode set to {}",
                    ai_mode_label(&self.state.ai_mode)
                ));
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
                let mut errors = Vec::new();
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

                    let encoded = self.encoder.encode(message);
                    match self.node.network().send(endpoint, encoded) {
                        message_io::network::SendStatus::Sent => (),
                        status => {
                            errors.push((
                                endpoint,
                                std::io::Error::other(format!(
                                    "Send failed with status: {:?}",
                                    status
                                )),
                            ));
                        }
                    }
                }
                if !errors.is_empty() {
                    Err(errors).report_if_err(&mut self.state);
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
            AppCommand::Help => {
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
                    let art = self.avatar_manager.render(&preset, AvatarState::Online, size);
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
        payload: AiPayload,
        broadcast: bool,
        source_peer: Option<String>,
        source_fingerprint: Option<String>,
        trusted: bool,
        truncated: bool,
    ) {
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
            let message = self.encoder.encode(NetMessage::AiMessage(payload.clone()));
            let endpoints = self.state.all_user_endpoints();
            crate::util::send_all(self.node.network(), &endpoints, message)
                .report_if_err(&mut self.state);
        }
        self.righ_the_bell();
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
            let net_message = NetMessage::SkillResult(payload);
            let message = self.encoder.encode(net_message);
            let endpoints = self.state.all_user_endpoints();
            crate::util::send_all(self.node.network(), &endpoints, message)
                .report_if_err(&mut self.state);
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

    pub fn righ_the_bell(&self) {
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
            let endpoint = self
                .state
                .peer_endpoint_by_name(&peer)
                .expect("peer endpoint should exist for known peer name");
            let room_status = if active_members.iter().any(|member| member == &peer) {
                "in room"
            } else {
                "available"
            };
            let readiness = match self.state.peer_readiness(endpoint) {
                PeerReadiness::Ready => "ready",
                PeerReadiness::Connecting => "connecting",
            };
            let fingerprint = self
                .state
                .peer_fingerprint(endpoint)
                .expect("peer fingerprint should exist for known peer");
            let trust =
                if self.state.is_trusted_peer(&fingerprint) { "trusted" } else { "untrusted" };
            lines.push(format!(
                "  {peer}  [{room_status}, {readiness}, {trust}]  fp={fingerprint}",
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
        room.ai_mode.as_ref().map(ai_mode_label).unwrap_or("off")
    )
}

fn describe_room_list_entry(room: &Room) -> String {
    format!(
        "{} [{}] mode: {}",
        room.id,
        human_member_names(room).join(", "),
        room.ai_mode.as_ref().map(ai_mode_label).unwrap_or("off")
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
        ("AI", vec![
            ("/ai mode <mode>", "Change AI behaviour mode:"),
            ("", "  clerk      Active assistant (auto-intervenes)"),
            ("", "  listener   Passive observer (only responds to mentions)"),
            ("", "  moderator  Focuses on decisions and conflicts"),
            ("", "  operator   Executes skills on request"),
            ("", "  companion  Casual conversational partner"),
            ("/ai quiet <on|off>", "Mute/unmute AI responses"),
            ("/ai freq <low|normal|high>", "Adjust AI intervention frequency"),
        ]),
        ("Summary", vec![
            ("/summary", "Summarise the conversation"),
            ("/todos", "List action items"),
            ("/decisions", "List decisions made"),
            ("/context", "Summarise context"),
        ]),
        ("Rooms", vec![
            ("/room create @user [--ai <mode>]", "Create a room with peers"),
            ("/room list", "List all rooms"),
            ("/room switch <id|name>", "Switch active room"),
        ]),
        ("Peers", vec![
            ("/peers", "List connected peers"),
            ("/peer connect <host:port>", "Connect to a peer directly"),
            ("/trust list", "List trusted peer fingerprints"),
            ("/trust add <peer|fp>", "Trust a peer explicitly"),
            ("/trust remove <peer|fp>", "Remove stored peer trust"),
        ]),
        ("Skills", vec![
            ("/skills", "List available skills"),
            ("/skill <name> [args]", "Run a skill manually"),
            ("/run <id>", "Accept a skill proposal from AI"),
            ("/cancel", "Cancel current AI task or skill"),
        ]),
        ("Avatar", vec![
            ("/avatar set <target> <preset>", "Set avatar (target: self, @ops-ai)"),
            ("/avatar list", "List available avatar presets"),
            ("/avatar preview", "Preview your current avatar"),
            ("/avatar mode <size>", "Set size: compact, normal, expressive"),
        ]),
        ("Files", vec![
            ("/send <file>", "Send a file to peers in the room"),
        ]),
    ];

    for (title, commands) in sections {
        out.push_str(&format!("\n【 {} 】\n", title));
        for (cmd, desc) in commands {
            if cmd.is_empty() {
                out.push_str(&format!("      {}\n", desc));
            } else {
                out.push_str(&format!("  {:<36} {}\n", cmd, desc));
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

fn ai_mode_label(mode: &AiMode) -> &'static str {
    match mode {
        AiMode::Clerk => "clerk",
        AiMode::Listener => "listener",
        AiMode::Moderator => "moderator",
        AiMode::Operator => "operator",
        AiMode::Companion => "companion",
    }
}

fn ai_frequency_label(frequency: &AiFrequency) -> &'static str {
    match frequency {
        AiFrequency::Low => "low",
        AiFrequency::Normal => "normal",
        AiFrequency::High => "high",
    }
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
