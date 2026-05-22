pub mod layout;
pub mod messages;
pub mod peers_panel;
pub mod room_list_panel;
pub mod status_panel;

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
use crate::state::{MessageType, ProgressState, State, SystemMessageType};
use crate::ui::layout::should_show_side_panels;
use crate::ui::messages::messages;
use crate::ui::peers_panel::draw_peers_panel;
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
        //  ┌──────────┬──────────────────────┬──────────────┐
        //  │  Peers   │        Chat          │    ops-ai    │
        //  ├──────────┴──────────────────────┴──────────────┤
        //  │                    Input                       │
        //  └────────────────────────────────────────────────┘
        //
        // Horizontal: peers(18) | chat(min) | status(22)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(layout::three_pane_constraints())
            .split(upper_chunk);

        draw_peers_panel(frame, state, h_chunks[0], avatar_manager);
        draw_messages_panel(frame, state, h_chunks[1], theme, language);
        draw_status_panel(frame, state, h_chunks[2], avatar_manager);
    } else {
        // ── Narrow layout: chat above, ops-ai below ─────────────────────────
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout::right_column_constraints())
            .split(upper_chunk);

        draw_messages_panel(frame, state, right_chunks[0], theme, language);

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

            // Mentions must be at a boundary (start of string or preceded by non-alphanumeric char)
            let is_boundary = i == 0
                || content[..i]
                    .chars()
                    .next_back()
                    .map(|c| !c.is_alphanumeric() && c != '_' && c != '-')
                    .unwrap_or(true);

            if !is_boundary {
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
    fn test_parse_content_no_mentions_in_email() {
        let theme = Theme::default();
        let content = "Email me at user@example.com";
        let spans = parse_content(content, &theme, "alice");
        // With boundary check, it should be a single raw span
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, content);
    }

    #[test]
    fn test_parse_content_single_at() {
        let theme = Theme::default();
        let content = "Just an @ symbol";
        let spans = parse_content(content, &theme, "alice");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Just an ");
        assert_eq!(spans[1].content, "@");
        assert_eq!(spans[2].content, " symbol");
    }
}
