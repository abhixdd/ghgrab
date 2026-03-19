use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::*;

pub struct PreviewState<'a> {
    pub content: &'a str,
    pub path: &'a str,
    pub loading: bool,
    pub image: Option<&'a image::DynamicImage>,
    pub picker: Option<&'a mut ratatui_image::picker::Picker>,
}

pub fn render(f: &mut Frame, area: Rect, mut state: PreviewState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Preview: {} ", state.path))
        .border_style(Style::default().fg(ACCENT_COLOR))
        .style(Style::default().bg(BG_COLOR));

    let popup_area = centered_rect(80, 80, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_area = block.inner(popup_area);

    if state.loading {
        let loading_text = Paragraph::new("Loading preview...")
            .style(Style::default().fg(WARNING_COLOR))
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
    } else if let Some(img) = state.image {
        if let Some(ref mut picker) = state.picker {
            let resize = ratatui_image::Resize::Fit(None);
            if let Ok(protocol) = picker.new_protocol(img.clone(), inner_area, resize) {
                let image_widget = ratatui_image::Image::new(protocol.as_ref());
                f.render_widget(image_widget, inner_area);
            } else {
                let error_text = Paragraph::new("Failed to create image protocol.")
                    .style(Style::default().fg(ERROR_COLOR))
                    .alignment(Alignment::Center);
                f.render_widget(error_text, inner_area);
            }
        } else {
            let error_text = Paragraph::new("Image picker not initialized.")
                .style(Style::default().fg(ERROR_COLOR))
                .alignment(Alignment::Center);
            f.render_widget(error_text, inner_area);
        }
    } else {
        let content = if state.content.is_empty() {
            "No content available or empty file."
        } else {
            state.content
        };

        let footer_hint = Line::from(vec![Span::styled(
            " (Showing first 16KB - Press ESC to close) ",
            Style::default()
                .fg(BORDER_COLOR)
                .add_modifier(Modifier::ITALIC),
        )]);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner_area);

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(FG_COLOR))
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, chunks[0]);

        let footer = Paragraph::new(footer_hint).alignment(Alignment::Center);
        f.render_widget(footer, chunks[1]);
    }
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
