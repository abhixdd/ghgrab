use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::*;
use std::time::Instant;

#[derive(Clone, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration_secs: u64,
}

impl Toast {
    pub fn new(message: String, toast_type: ToastType) -> Self {
        Self {
            message,
            toast_type,
            created_at: Instant::now(),
            duration_secs: 4, 
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= self.duration_secs
    }
}

pub fn render(f: &mut Frame, area: Rect, toast: &Toast) {
    let toast_width = 40;
    let toast_height = 5;

    let col_constraints = [
        Constraint::Min(0),
        Constraint::Length(toast_width),
        Constraint::Length(2), 
    ];
    let row_constraints = [
        Constraint::Min(0),
        Constraint::Length(toast_height),
        Constraint::Length(1), 
    ];

    let rects_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints)
        .split(area);
    
    let rects_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(rects_cols[1]);

    let toast_area = rects_rows[1];

    let (icon, color, title) = match toast.toast_type {
        ToastType::Info => ("ℹ", ACCENT_COLOR, "Info"),
        ToastType::Success => ("✓", SUCCESS_COLOR, "Success"),
        ToastType::Error => ("✕", ERROR_COLOR, "Error"),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(format!(" {} {} ", icon, title), Style::default().fg(color).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG_COLOR));

    let text = Paragraph::new(toast.message.as_str())
        .block(block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(FG_COLOR));

    f.render_widget(Clear, toast_area);
    f.render_widget(text, toast_area);
}
