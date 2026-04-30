pub mod layout;
pub mod messages;
pub mod peers_panel;
pub mod room_list_panel;
pub mod status_panel;

use resize::px::RGB;
use resize::Pixel::RGB8;
use resize::Type::Lanczos3;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::avatar::loader::AvatarManager;
use crate::commands::CommandManager;
use crate::config::{LanguageConfig, Theme};
use crate::state::{MessageType, ProgressState, State, SystemMessageType, Window};
use crate::ui::layout::should_show_side_panels;
use crate::ui::messages::messages;
use crate::ui::peers_panel::draw_peers_panel;
use crate::ui::room_list_panel::draw_room_list_panel;
use crate::ui::status_panel::draw_status_panel;
use crate::util::split_each;

pub fn draw(
    frame: &mut Frame<impl Backend>,
    state: &mut State,
    chunk: Rect,
    theme: &Theme,
    language: &LanguageConfig,
    avatar_manager: &AvatarManager,
) {
    // Outer vertical split: [upper(min), input(6)]
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)].as_ref())
        .split(chunk);

    let upper_chunk = v_chunks[0];

    if should_show_side_panels(chunk.width) {
        // ── Wide layout ────────────────────────────────────────────────────
        //
        //  ┌──────────┬──────────────────────┐
        //  │  Peers   │        Chat          │
        //  │──────────┤──────────────────────┤
        //  │  Rooms   │       ops-ai         │
        //  ├──────────┴──────────────────────┤
        //  │              Input              │
        //  └─────────────────────────────────┘
        //
        // Horizontal: left column(18) | right column(min)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(layout::left_right_constraints())
            .split(upper_chunk);

        // Left column: peers(min) above, rooms(scaled) below
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout::left_column_constraints(upper_chunk.height))
            .split(h_chunks[0]);

        // Right column: chat(min) above, ops-ai(11) below
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout::right_column_constraints())
            .split(h_chunks[1]);

        draw_peers_panel(frame, state, left_chunks[0], avatar_manager);
        draw_room_list_panel(frame, state, left_chunks[1]);

        if !state.windows.is_empty() {
            let chat_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(30)].as_ref())
                .split(right_chunks[0]);
            draw_messages_panel(frame, state, chat_chunks[0], theme, language);
            draw_video_panel(frame, state, chat_chunks[1]);
        } else {
            draw_messages_panel(frame, state, right_chunks[0], theme, language);
        }

        draw_status_panel(frame, state, right_chunks[1], avatar_manager);
    } else {
        // ── Narrow layout: chat above, ops-ai below ─────────────────────────
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout::right_column_constraints())
            .split(upper_chunk);

        if !state.windows.is_empty() {
            let chat_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(30)].as_ref())
                .split(right_chunks[0]);
            draw_messages_panel(frame, state, chat_chunks[0], theme, language);
            draw_video_panel(frame, state, chat_chunks[1]);
        } else {
            draw_messages_panel(frame, state, right_chunks[0], theme, language);
        }

        draw_status_panel(frame, state, right_chunks[1], avatar_manager);
    }

    draw_input_panel(frame, state, v_chunks[1], theme);
}

fn draw_messages_panel(
    frame: &mut Frame<impl Backend>,
    state: &mut State,
    chunk: Rect,
    theme: &Theme,
    language: &LanguageConfig,
) {
    let message_colors = &theme.message_colors;
    let ui_messages = messages(&language.ui);

    state.update_chat_viewport(chunk.width, chunk.height);

    let messages = state
        .messages()
        .iter()
        .flat_map(|message| {
            let color = if let Some(id) = state.users_id().get(&message.user) {
                message_colors[id % message_colors.len()]
            } else {
                theme.my_user_color
            };
            let date = message.date.format("%H:%M:%S ").to_string();
            match &message.message_type {
                MessageType::Connection => vec![Spans::from(vec![
                    Span::styled(date, Style::default().fg(theme.date_color)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(ui_messages.connected, Style::default().fg(color)),
                ])],
                MessageType::Disconnection => vec![Spans::from(vec![
                    Span::styled(date, Style::default().fg(theme.date_color)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(ui_messages.disconnected, Style::default().fg(color)),
                ])],
                MessageType::Text(content) => {
                    let mut ui_message = vec![
                        Span::styled(date, Style::default().fg(theme.date_color)),
                        Span::styled(&message.user, Style::default().fg(color)),
                        Span::styled(": ", Style::default().fg(color)),
                    ];
                    ui_message.extend(parse_content(content, theme, state.local_user_name()));
                    vec![Spans::from(ui_message)]
                }
                MessageType::AiText(content) => {
                    let mut ui_message = vec![
                        Span::styled(date, Style::default().fg(theme.date_color)),
                        Span::styled(State::AI_NAME, Style::default().fg(Color::LightCyan)),
                        Span::styled(": ", Style::default().fg(Color::LightCyan)),
                    ];
                    for mut span in parse_content(content, theme, state.local_user_name()) {
                        if span.style == Style::default() {
                            span.style = Style::default().fg(Color::LightCyan);
                        }
                        ui_message.push(span);
                    }
                    vec![Spans::from(ui_message)]
                }
                MessageType::System(content, msg_type) => {
                    let (user_color, content_color) = match msg_type {
                        SystemMessageType::Info => theme.system_info_color,
                        SystemMessageType::Warning => theme.system_warning_color,
                        SystemMessageType::Error => theme.system_error_color,
                    };

                    let header_date = Span::styled(date, Style::default().fg(theme.date_color));
                    let header_user = Span::styled(&message.user, Style::default().fg(user_color));

                    if content.is_empty() {
                        return vec![Spans::from(vec![header_date, header_user])];
                    }

                    // Calculate indentation based on date only to align with username
                    let indent_width = header_date.content.width();
                    let indent = " ".repeat(indent_width);

                    content
                        .lines()
                        .enumerate()
                        .map(|(i, line)| {
                            if i == 0 {
                                Spans::from(vec![
                                    header_date.clone(),
                                    header_user.clone(),
                                    Span::styled(line, Style::default().fg(content_color)),
                                ])
                            } else {
                                Spans::from(vec![
                                    Span::raw(indent.clone()),
                                    Span::styled(line, Style::default().fg(content_color)),
                                ])
                            }
                        })
                        .collect::<Vec<_>>()
                }
                MessageType::Progress(state) => {
                    vec![Spans::from(add_progress_bar(chunk.width, state, theme))]
                }
            }
        })
        .collect::<Vec<_>>();

    let title = match state.ai_state {
        crate::state::AiState::Acting => ui_messages.acting_title,
        crate::state::AiState::Failed(_) => ui_messages.failed_title,
        _ if state.ai_thinking => ui_messages.thinking_title,
        _ => "triadchat",
    };

    let messages_panel = Paragraph::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))),
        )
        .style(Style::default().fg(theme.chat_panel_color))
        .alignment(Alignment::Left)
        .scroll((state.scroll_messages_view() as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_panel, chunk);
}

fn add_progress_bar<'a>(
    panel_width: u16,
    progress: &'a ProgressState,
    theme: &Theme,
) -> Vec<Span<'a>> {
    let color = theme.progress_bar_color;
    let width = (panel_width - 20) as usize;

    let (title, ui_current, ui_remaining) = match progress {
        ProgressState::Started(_) => ("Pending: ", 0, width),
        ProgressState::Working(total, current) => {
            let percentage = *current as f64 / *total as f64;
            let ui_current = (percentage * width as f64) as usize;
            let ui_remaining = width - ui_current;
            ("Sending: ", ui_current, ui_remaining)
        }
        ProgressState::Completed => ("Done! ", width, 0),
    };

    let current = "#".repeat(ui_current);
    let remaining = "-".repeat(ui_remaining);
    let msg = format!("[{}{}]", current, remaining);
    vec![
        Span::styled(title, Style::default().fg(color)),
        Span::styled(msg, Style::default().fg(color)),
    ]
}

fn parse_content<'a>(content: &'a str, theme: &Theme, local_user_name: &str) -> Vec<Span<'a>> {
    if content.starts_with(CommandManager::COMMAND_PREFIX) {
        content
            .split_whitespace()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    Span::styled(part, Style::default().fg(theme.command_color))
                } else {
                    Span::raw(format!(" {}", part))
                }
            })
            .collect()
    } else {
        let mut spans = Vec::new();
        let mut last_pos = 0;

        for (i, _) in content.match_indices('@') {
            if i < last_pos {
                continue;
            }

            // Push text before '@'
            if i > last_pos {
                spans.push(Span::raw(&content[last_pos..i]));
            }

            // Find the end of the mention
            let mention_part = &content[i..];
            // Mentions stop at first non-alphanumeric character (except _ or -)
            // Skip the leading '@'
            let end_offset = mention_part[1..]
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .map(|idx| idx + 1)
                .unwrap_or(mention_part.len());

            if end_offset > 1 {
                let mention = &mention_part[..end_offset];
                let name = &mention[1..];

                let color = if name == local_user_name {
                    theme.mention_me_color
                } else if name == "ops-ai" || name == State::AI_NAME {
                    Color::LightCyan
                } else {
                    theme.mention_other_color
                };

                spans.push(Span::styled(mention, Style::default().fg(color)));
                last_pos = i + end_offset;
            } else {
                // Just a single '@' without name, or followed by invalid char
                spans.push(Span::raw("@"));
                last_pos = i + 1;
            }
        }

        if last_pos < content.len() {
            spans.push(Span::raw(&content[last_pos..]));
        }

        if spans.is_empty() {
            spans.push(Span::raw(content));
        }

        spans
    }
}

fn draw_input_panel(frame: &mut Frame<impl Backend>, state: &State, chunk: Rect, theme: &Theme) {
    let inner_width = (chunk.width - 2) as usize;
    let input = state.input().iter().collect::<String>();
    let input = split_each(input, inner_width)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect::<Vec<_>>();

    let input_panel = Paragraph::new(input)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Your message", Style::default().add_modifier(Modifier::BOLD))),
        )
        .style(Style::default().fg(theme.input_panel_color))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, chunk);

    let input_cursor = state.ui_input_cursor(inner_width);
    frame.set_cursor(chunk.x + 1 + input_cursor.0, chunk.y + 1 + input_cursor.1)
}

fn draw_video_panel(frame: &mut Frame<impl Backend>, state: &State, chunk: Rect) {
    let windows = state.windows.values().collect();
    let fb = FrameBuffer::new(windows).block(Block::default().borders(Borders::ALL));
    frame.render_widget(fb, chunk);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use tui::style::Color;

    #[test]
    fn test_parse_content_no_mentions() {
        let theme = Theme::default();
        let content = "Hello world";
        let spans = parse_content(content, &theme, "alice");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "Hello world");
        assert_eq!(spans[0].style, Style::default());
    }

    #[test]
    fn test_parse_content_mention_me() {
        let theme = Theme::default();
        let content = "Hello @alice!";
        let spans = parse_content(content, &theme, "alice");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Hello ");
        assert_eq!(spans[1].content, "@alice");
        assert_eq!(spans[1].style.fg, Some(theme.mention_me_color));
        assert_eq!(spans[2].content, "!");
    }

    #[test]
    fn test_parse_content_mention_other() {
        let theme = Theme::default();
        let content = "Hi @bob, how are you?";
        let spans = parse_content(content, &theme, "alice");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Hi ");
        assert_eq!(spans[1].content, "@bob");
        assert_eq!(spans[1].style.fg, Some(theme.mention_other_color));
        assert_eq!(spans[2].content, ", how are you?");
    }

    #[test]
    fn test_parse_content_mention_ai() {
        let theme = Theme::default();
        let content = "Ask @ops-ai for help";
        let spans = parse_content(content, &theme, "alice");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Ask ");
        assert_eq!(spans[1].content, "@ops-ai");
        assert_eq!(spans[1].style.fg, Some(Color::LightCyan));
        assert_eq!(spans[2].content, " for help");
    }

    #[test]
    fn test_parse_content_multiple_mentions() {
        let theme = Theme::default();
        let content = "@alice and @bob and @ops-ai";
        let spans = parse_content(content, &theme, "alice");
        // ["@alice", " and ", "@bob", " and ", "@ops-ai"]
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[0].content, "@alice");
        assert_eq!(spans[0].style.fg, Some(theme.mention_me_color));
        assert_eq!(spans[2].content, "@bob");
        assert_eq!(spans[2].style.fg, Some(theme.mention_other_color));
        assert_eq!(spans[4].content, "@ops-ai");
        assert_eq!(spans[4].style.fg, Some(Color::LightCyan));
    }

    #[test]
    fn test_parse_content_single_at() {
        let theme = Theme::default();
        let content = "Email me at user@example.com";
        let spans = parse_content(content, &theme, "alice");
        // Currently my implementation will highlight "@example" if it's considered a name.
        // "Email me at user", "@example", ".com"
        // If we want to avoid this, we'd need more complex logic.
        // But for a chat app, @name is common.
        assert_eq!(spans[1].content, "@example");
    }
}

#[derive(Default)]
struct FrameBuffer<'a> {
    windows: Vec<&'a Window>,
    block: Option<Block<'a>>,
}

impl<'a> FrameBuffer<'a> {
    fn new(windows: Vec<&'a Window>) -> Self {
        Self { windows, ..Default::default() }
    }

    fn block(mut self, block: Block<'a>) -> FrameBuffer<'a> {
        self.block = Some(block);
        self
    }
}

impl tui::widgets::Widget for FrameBuffer<'_> {
    fn render(mut self, area: Rect, buf: &mut tui::buffer::Buffer) {
        let area = match self.block.take() {
            Some(block) => {
                let inner = block.inner(area);
                block.render(area, buf);
                inner
            }
            None => area,
        };

        let windows_num = self.windows.len();
        let window_height = area.height / windows_num as u16;
        let y_start = area.y;
        for (idx, window) in self.windows.iter().enumerate() {
            let area =
                Rect::new(area.x, y_start + window_height * idx as u16, area.width, window_height);

            let mut resizer = resize::new(
                window.width / 2,
                window.height,
                area.width as usize,
                area.height as usize,
                RGB8,
                Lanczos3,
            )
            .unwrap();
            let mut dst = vec![RGB::new(0, 0, 0); (area.width * area.height) as usize];
            resizer.resize(&window.data, &mut dst).unwrap();

            let mut dst = dst.iter();
            for j in area.y..area.y + area.height {
                for i in area.x..area.x + area.width {
                    let rgb = dst.next().unwrap();
                    buf.get_mut(i, j).set_bg(Color::Rgb(rgb.r, rgb.g, rgb.b));
                }
            }
        }
    }
}
