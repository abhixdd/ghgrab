use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::sync::OnceLock;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static SYNTAX_THEME: OnceLock<Theme> = OnceLock::new();

pub fn highlight_content(content: &str, path: &str) -> Text<'static> {
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
