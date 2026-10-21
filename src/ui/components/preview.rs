use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::components::image_preview::{is_previewable_image, ImageProtocol, PreviewImage};
use crate::ui::theme::*;

pub struct PreviewState<'a> {
    pub content: &'a str,
    pub text: Option<Text<'static>>,
    pub path: &'a str,
    pub loading: bool,
    pub unsupported: bool,
    pub image: Option<&'a PreviewImage>,
    pub protocol: ImageProtocol,
    pub cell_ratio: f32,
}

/// Returns the cell area where a graphics-protocol image still has to be drawn.
pub fn render(f: &mut Frame, area: Rect, state: PreviewState) -> Option<Rect> {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Preview: {} ", state.path))
        .border_style(Style::default().fg(ACCENT_COLOR()))
        .style(Style::default().bg(BG_COLOR()));

    let popup_area = centered_rect(80, 80, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_area = block.inner(popup_area);

    if state.loading {
        let loading_text = Paragraph::new("Loading preview...")
            .style(Style::default().fg(WARNING_COLOR()))
            .alignment(Alignment::Center);

        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(1),
                Constraint::Percentage(45),
            ])
            .split(inner_area)[1];

        f.render_widget(loading_text, vertical_center);
    } else if let Some(image) = state.image {
        return render_image(f, inner_area, image, state.protocol, state.cell_ratio);
    } else if state.unsupported {
        let message = if is_previewable_image(state.path) {
            "Image preview is disabled in this terminal.\nUse a local image viewer to open this file."
        } else {
            "Video preview is not supported in the terminal.\nUse a local media player to open this file."
        };
        let msg = Paragraph::new(message)
            .style(Style::default().fg(WARNING_COLOR()))
            .alignment(Alignment::Center);

        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(3),
                Constraint::Percentage(40),
            ])
            .split(inner_area)[1];

        f.render_widget(msg, vertical_center);
    } else {
        let content = if state.content.is_empty() {
            "No content available or empty file."
        } else {
            state.content
        };

        let footer_hint = Line::from(vec![Span::styled(
            " (Showing first 16KB - Press ESC to close) ",
            Style::default()
                .fg(BORDER_COLOR())
                .add_modifier(Modifier::ITALIC),
        )]);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner_area);

        let preview_text = state
            .text
            .unwrap_or_else(|| Text::from(content.to_string()));
        let paragraph = Paragraph::new(preview_text).wrap(Wrap { trim: false });

        f.render_widget(paragraph, chunks[0]);

        let footer = Paragraph::new(footer_hint).alignment(Alignment::Center);
        f.render_widget(footer, chunks[1]);
    }

    None
}

fn render_image(
    f: &mut Frame,
    area: Rect,
    image: &PreviewImage,
    protocol: ImageProtocol,
    cell_ratio: f32,
) -> Option<Rect> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let image_area = chunks[0];

    let (width, height) = image.dimensions();
    let footer_hint = Line::from(vec![Span::styled(
        format!(
            " {}×{} · {} · ESC to close ",
            width,
            height,
            protocol.label()
        ),
        Style::default()
            .fg(BORDER_COLOR())
            .add_modifier(Modifier::ITALIC),
    )]);
    f.render_widget(
        Paragraph::new(footer_hint).alignment(Alignment::Center),
        chunks[1],
    );

    let (cols, rows) = image.fit(image_area.width, image_area.height, cell_ratio);
    if cols == 0 || rows == 0 {
        return None;
    }

    let image_box = Rect {
        x: image_area.x + (image_area.width - cols) / 2,
        y: image_area.y + (image_area.height - rows) / 2,
        width: cols,
        height: rows,
    };

    if protocol.is_graphics() {
        return Some(image_box);
    }

    let background = match BG_COLOR() {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    };
    f.render_widget(
        Paragraph::new(image.halfblocks(cols, rows, background)),
        image_box,
    );
    None
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
