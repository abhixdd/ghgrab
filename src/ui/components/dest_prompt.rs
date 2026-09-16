use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, input_text: &str, cursor: usize, cursor_visible: bool) {
    let width = area.width.saturating_sub(4).clamp(20, 72);
    let height = 4;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(cols[1]);
    let prompt_area = rows[1];

    let mut display = input_text.to_string();
    if cursor_visible {
        if cursor >= display.chars().count() {
            display.push('_');
        } else {
            let start = display
                .char_indices()
                .nth(cursor)
                .map(|(i, _)| i)
                .unwrap_or(display.len());
            let end = display
                .char_indices()
                .nth(cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(display.len());
            display.replace_range(start..end, "_");
        }
    }

    // Keep the cursor visible when the path is longer than the box
    let inner_width = width.saturating_sub(2) as usize;
    let skip = (cursor + 1).saturating_sub(inner_width);
    let visible: String = display.chars().skip(skip).take(inner_width).collect();

    let lines = vec![
        Line::from(Span::styled(visible, Style::default().fg(FG_COLOR()))),
        Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(SUCCESS_COLOR())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Download", Style::default().fg(BORDER_COLOR())),
            Span::styled("  │  ", Style::default().fg(BORDER_COLOR())),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(ERROR_COLOR())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(BORDER_COLOR())),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Download to ",
            Style::default()
                .fg(SUCCESS_COLOR())
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(SUCCESS_COLOR()))
        .style(Style::default().bg(BG_COLOR()));

    f.render_widget(Clear, prompt_area);
    f.render_widget(Paragraph::new(lines).block(block), prompt_area);
}
