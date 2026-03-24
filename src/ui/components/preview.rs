use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::sync::OnceLock;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

use crate::ui::theme::*;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static SYNTAX_THEME: OnceLock<Theme> = OnceLock::new();

pub struct PreviewState<'a> {
    pub content: &'a str,
    pub path: &'a str,
    pub loading: bool,
    pub is_image: bool,
}

pub fn render(f: &mut Frame, area: Rect, state: PreviewState) {
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
    } else if state.is_image {
        let msg = Paragraph::new("Image preview is not supported in the terminal.\nUse a local image viewer to open this file.")
            .style(Style::default().fg(WARNING_COLOR))
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
                .fg(BORDER_COLOR)
                .add_modifier(Modifier::ITALIC),
        )]);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner_area);

        let highlighted = highlight_content(content, state.path);
        let paragraph = Paragraph::new(highlighted).wrap(Wrap { trim: false });

        f.render_widget(paragraph, chunks[0]);

        let footer = Paragraph::new(footer_hint).alignment(Alignment::Center);
        f.render_widget(footer, chunks[1]);
    }
}

fn highlight_content(content: &str, path: &str) -> Text<'static> {
    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let syntax = select_syntax(syntax_set, path);
    let theme = SYNTAX_THEME.get_or_init(load_theme);

    let mut highlighter = HighlightLines::new(syntax, theme);
    let lines: Vec<Line<'static>> = content
        .lines()
        .map(|line| {
            let ranges = highlighter
                .highlight_line(line, syntax_set)
                .unwrap_or_default();
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let mut ratatui_style = Style::default().fg(syntect_color(style.foreground));
                    if style.font_style.contains(FontStyle::BOLD) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                    }
                    if style.font_style.contains(FontStyle::UNDERLINE) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                    }
                    Span::styled(text.to_string(), ratatui_style)
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    Text::from(lines)
}

fn syntect_color(color: syntect::highlighting::Color) -> Color {
    // syntect uses `a` as a tag: 0xFF = normal RGB, 0x01 = transparent, 0x00 = indexed terminal
    match color.a {
        0x00 => match color.r {
            0x00 => Color::Black,
            0x01 => Color::Red,
            0x02 => Color::Green,
            0x03 => Color::Yellow,
            0x04 => Color::Blue,
            0x05 => Color::Magenta,
            0x06 => Color::Cyan,
            0x07 => Color::Gray,
            0x08 => Color::DarkGray,
            0x09 => Color::LightRed,
            0x0A => Color::LightGreen,
            0x0B => Color::LightYellow,
            0x0C => Color::LightBlue,
            0x0D => Color::LightMagenta,
            0x0E => Color::LightCyan,
            0x0F => Color::White,
            n => Color::Indexed(n),
        },
        0x01 => Color::Reset,
        _ => Color::Rgb(color.r, color.g, color.b),
    }
}

fn load_theme() -> Theme {
    let theme_set = ThemeSet::load_defaults();
    theme_set
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .or_else(|| theme_set.themes.values().next().cloned())
        .unwrap_or_default()
}

fn select_syntax<'a>(syntax_set: &'a SyntaxSet, path: &str) -> &'a SyntaxReference {
    if let Some(ext) = path.rsplit('.').next() {
        if ext != path && !ext.is_empty() {
            if let Some(syntax) = syntax_set.find_syntax_by_extension(ext) {
                return syntax;
            }
        }
    }

    syntax_set.find_syntax_plain_text()
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
