use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Instant;

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use message_io::events::EventReceiver;
use message_io::network::{Endpoint, Transport};
use message_io::node::{
    self, NodeHandler, NodeTask, StoredNetEvent as NetEvent, StoredNodeEvent as NodeEvent,
};

use crate::action::{Action, Processing};
use crate::ai::trigger::{should_intervene, TriggerConfig};
use crate::ai::{AiMediator, AiTask};
use crate::commands::ai_cmd::AiCommand;
use crate::commands::room_cmd::{PeersCommand, RoomCommand};
use crate::commands::send_file::SendFileCommand;
use crate::commands::summary_cmd::SummaryCommand;
use crate::commands::{AppCommand, CommandManager, ParsedCommand, SummaryCommandKind};
use crate::config::Config;
use crate::encoder::{self, Encoder};
use crate::message::{AiIntent, AiPayload, Chunk, NetMessage, PeerInfo};
use crate::renderer::Renderer;
use crate::state::{
    AiMode, AiState, ChatMessage, CursorMovement, MessageType, ScrollMovement, State, Window,
};
use crate::terminal_events::TerminalEventCollector;
use crate::util::{Error, Reportable, Result};

pub enum Signal {
    Terminal(TermEvent),
    Action(Box<dyn Action>),
    AiResponse(AiPayload),
    AiFailed(String),
    SkillDone(String),
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
    test_mode: bool,
    local_server_port: Option<u16>,
}

impl<'a> Application<'a> {
    pub fn new(config: &'a Config) -> Result<Self> {
        Self::new_inner(config, true)
    }

    pub fn new_for_test(config: &'a Config) -> Result<Self> {
        Self::new_inner(config, false)
    }

    fn new_inner(config: &'a Config, collect_terminal_events: bool) -> Result<Self> {
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
            .with(RoomCommand)
            .with(PeersCommand)
            .with(SummaryCommand::summary())
            .with(SummaryCommand::todos())
            .with(SummaryCommand::decisions())
            .with(SummaryCommand::context());
        let runtime = tokio::runtime::Runtime::new()?;

        let mut state = State::default();
        state.set_local_user_name(config.user_name.clone());
        state.ui_language = config.language.ui.clone();

        let workspace = std::env::current_dir()?;
        let ai_mediator = if config.ai.enabled {
            match AiMediator::new(&workspace, &config.ai, &config.language) {
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
            test_mode: !collect_terminal_events,
            local_server_port: None,
        })
    }

    pub fn run(&mut self, out: impl std::io::Write) -> Result<()> {
        let mut renderer =
            if self._terminal_events.is_some() { Some(Renderer::new(out)?) } else { None };
        if let Some(renderer) = renderer.as_mut() {
            renderer.render(&self.state, self.config)?;
        }

        self.start_network()?;

        loop {
            if !self.process_next_event()? {
                return Ok(());
            }
            if let Some(renderer) = renderer.as_mut() {
                renderer.render(&self.state, self.config)?;
            }
        }
    }

    fn start_network(&mut self) -> Result<()> {
        let server_addr = ("0.0.0.0", self.config.tcp_server_port);
        let (_, server_addr) = self.node.network().listen(Transport::FramedTcp, server_addr)?;
        self.local_server_port = Some(server_addr.port());
        self.node.network().listen(Transport::Udp, self.config.discovery_addr)?;

        let (discovery_endpoint, _) =
            self.node.network().connect_sync(Transport::Udp, self.config.discovery_addr)?;
        let message = NetMessage::HelloLan(self.config.user_name.clone(), server_addr.port());
        self.node.network().send(discovery_endpoint, self.encoder.encode(message));
        Ok(())
    }

    pub fn start_network_for_test(&mut self) -> Result<()> {
        self.start_network()
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
                    None => return Err(anyhow::anyhow!("unknown message received")),
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
                Signal::AiResponse(payload) => self.process_ai_response(payload),
                Signal::AiFailed(error) => self.process_ai_failure(error),
                Signal::SkillDone(output) => {
                    self.state.add_system_info_message(format!("skill finished: {}", output));
                }
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
                self.state.connected_user(endpoint, &user);
                self.send_peer_info(endpoint);
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
            NetMessage::AiMessage(payload) => self.process_ai_response(payload),
            NetMessage::PeerInfo(peer) => self.state.record_peer(endpoint, peer),
            NetMessage::RoomCreate(room_id, member_ids) => {
                if member_ids.iter().any(|member_id| member_id == &self.config.user_name) {
                    let room = self.state.accept_room(&room_id, &member_ids);
                    self.state.add_system_info_message(format!("joined room {}", room.id));
                    self.node.network().send(
                        endpoint,
                        self.encoder.encode(NetMessage::RoomJoin(room.id.clone())),
                    );
                }
            }
            NetMessage::RoomJoin(room_id) => {
                let _ = self.state.switch_room(&room_id);
                self.state.add_system_info_message(format!("room {} is ready", room_id));
            }
            NetMessage::SkillResult(payload) => {
                self.state.add_system_info_message(format!(
                    "skill '{}' finished: {}",
                    payload.skill_name, payload.summary
                ));
            }
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
                KeyCode::Up => self.state.messages_scroll(ScrollMovement::Up),
                KeyCode::Down => self.state.messages_scroll(ScrollMovement::Down),
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
                    self.state.add_system_error_message(format!("Unknown command: {}", input));
                    return Ok(());
                }

                self.state.add_message(ChatMessage::new(
                    format!("{} (me)", self.config.user_name),
                    MessageType::Text(input.clone()),
                ));
                for endpoint in self.state.all_user_endpoints() {
                    self.node.network().send(
                        endpoint,
                        self.encoder.encode(NetMessage::UserMessage(input.clone())),
                    );
                }
                self.state.human_streak += 1;
                let trigger = TriggerConfig::from_frequency(self.state.ai_frequency.clone());
                if should_intervene(
                    &input,
                    self.state.ai_mode.clone(),
                    &trigger,
                    self.state.ai_thinking,
                    self.state.last_ai_at,
                    self.state.human_streak,
                    Instant::now(),
                ) {
                    self.spawn_ai_task(AiTask::Intervene);
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
                    .add_system_info_message(format!("AI mode set to {:?}", self.state.ai_mode));
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
                    "AI frequency set to {:?}",
                    self.state.ai_frequency
                ));
            }
            AppCommand::RoomCreate { peers, ai_mode } => {
                if let Some(missing_peer) = peers
                    .iter()
                    .find(|peer| !self.state.peer_names().iter().any(|known| known == *peer))
                {
                    self.state.add_system_error_message(format!("unknown peer: {}", missing_peer));
                    return;
                }

                let room = self.state.create_room(&peers, ai_mode);
                let member_ids = room.members.iter().map(|member| member.id.clone()).collect::<Vec<_>>();
                for endpoint in self.state.all_user_endpoints() {
                    if let Some(user_name) = self.state.user_name(endpoint) {
                        if peers.iter().any(|peer| peer == user_name) {
                            self.node.network().send(
                                endpoint,
                                self.encoder.encode(NetMessage::RoomCreate(room.id.clone(), member_ids.clone())),
                            );
                        }
                    }
                }
                self.state.add_system_info_message(format!("created room {}", room.id));
            }
            AppCommand::RoomList => {
                let room_ids = self.state.room_ids();
                if room_ids.is_empty() {
                    self.state.add_system_info_message("no rooms".into());
                } else {
                    self.state.add_system_info_message(room_ids.join(", "));
                }
            }
            AppCommand::RoomSwitch(room_id) => match self.state.switch_room(&room_id) {
                Ok(()) => self.state.add_system_info_message(format!("switched to {}", room_id)),
                Err(error) => self.state.add_system_error_message(error.to_string()),
            },
            AppCommand::Peers => {
                let peers = self.state.peer_names();
                if peers.is_empty() {
                    self.state.add_system_info_message("no peers discovered".into());
                } else {
                    self.state.add_system_info_message(peers.join(", "));
                }
            }
            AppCommand::Help => {
                self.state.add_system_info_message(
                    "/summary /todos /decisions /context /ai mode <clerk|listener|moderator|operator> /ai quiet <on|off> /ai freq <low|normal|high> /room create @user [--ai <mode>] /room list /room switch <room_id> /peers".into(),
                );
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
                Ok(payload) => self.process_ai_response(payload),
                Err(error) => self.process_ai_failure(error.to_string()),
            }
            return;
        }

        let node = self.node.clone();
        let task_handle = self.runtime.handle().spawn(async move {
            match ai_mediator.request(task, &transcript, &last_messages).await {
                Ok(payload) => node.signals().send(Signal::AiResponse(payload)),
                Err(error) => node.signals().send(Signal::AiFailed(error.to_string())),
            }
        });
        self.state.abort_handle = Some(task_handle.abort_handle());
    }

    fn process_ai_response(&mut self, payload: AiPayload) {
        self.state.ai_state = AiState::Idle;
        self.state.ai_thinking = false;
        self.state.last_ai_at = Some(Instant::now());
        self.state.human_streak = 0;
        self.state.abort_handle = None;
        self.state.add_message(ChatMessage::new(
            "ops-ai ✦".into(),
            MessageType::AiText(render_ai_payload(&payload)),
        ));
        self.righ_the_bell();
    }

    fn process_ai_failure(&mut self, error: String) {
        self.state.ai_state =
            if self.ai_mediator.is_some() { AiState::Idle } else { AiState::Disabled };
        self.state.ai_thinking = false;
        self.state.abort_handle = None;
        self.state.add_system_error_message(format!("[ops-ai: failed] {}", error));
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
        };
        let mut encoder = Encoder::new();
        self.node
            .network()
            .send(endpoint, encoder.encode(NetMessage::PeerInfo(peer)));
    }
}

impl Drop for Application<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.state.abort_handle.take() {
            handle.abort();
        }
        self.node.stop();
    }
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
