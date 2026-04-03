#![allow(non_snake_case)]

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ThemeFile {
    bg_color: Option<String>,
    fg_color: Option<String>,
    accent_color: Option<String>,
    warning_color: Option<String>,
    error_color: Option<String>,
    success_color: Option<String>,
    folder_color: Option<String>,
    selected_color: Option<String>,
    border_color: Option<String>,
    highlight_bg: Option<String>,
}

struct Theme {
    pub bg_color: Color,
    pub fg_color: Color,
    pub accent_color: Color,
    pub warning_color: Color,
    pub error_color: Color,
    pub success_color: Color,
    pub folder_color: Color,
    pub selected_color: Color,
    pub border_color: Color,
    pub highlight_bg: Color,
}

fn parse_color(val: Option<&str>, default: [u8; 3]) -> Color {
    let [r, g, b] = if let Some(hex) = val {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                [r, g, b]
            } else {
                default
            }
        } else {
            default
        }
    } else {
        default
    };
    Color::Rgb(r, g, b)
}

fn load_theme() -> Theme {
    let file: Option<ThemeFile> = (|| -> Option<ThemeFile> {
        let mut path = dirs::config_dir()?;
        path.push("ghgrab");
        path.push("theme.toml");
        let content = fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    })();
    let f = file.as_ref();
    Theme {
        bg_color: parse_color(f.and_then(|t| t.bg_color.as_deref()), [36, 40, 59]),
        fg_color: parse_color(f.and_then(|t| t.fg_color.as_deref()), [192, 202, 245]),
        accent_color: parse_color(f.and_then(|t| t.accent_color.as_deref()), [122, 162, 247]),
        warning_color: parse_color(f.and_then(|t| t.warning_color.as_deref()), [224, 175, 104]),
        error_color: parse_color(f.and_then(|t| t.error_color.as_deref()), [247, 118, 142]),
        success_color: parse_color(f.and_then(|t| t.success_color.as_deref()), [158, 206, 106]),
        folder_color: parse_color(f.and_then(|t| t.folder_color.as_deref()), [130, 170, 255]),
        selected_color: parse_color(f.and_then(|t| t.selected_color.as_deref()), [255, 158, 100]),
        border_color: parse_color(f.and_then(|t| t.border_color.as_deref()), [86, 95, 137]),
        highlight_bg: parse_color(f.and_then(|t| t.highlight_bg.as_deref()), [41, 46, 66]),
    }
}

static THEME: OnceLock<Theme> = OnceLock::new();
fn t() -> &'static Theme {
    THEME.get_or_init(load_theme)
}

pub fn BG_COLOR() -> Color {
    t().bg_color
}
pub fn FG_COLOR() -> Color {
    t().fg_color
}
pub fn ACCENT_COLOR() -> Color {
    t().accent_color
}
pub fn WARNING_COLOR() -> Color {
    t().warning_color
}
pub fn ERROR_COLOR() -> Color {
    t().error_color
}
pub fn SUCCESS_COLOR() -> Color {
    t().success_color
}
pub fn FOLDER_COLOR() -> Color {
    t().folder_color
}
pub fn _SELECTED_COLOR() -> Color {
    t().selected_color
}
pub fn BORDER_COLOR() -> Color {
    t().border_color
}
pub fn HIGHLIGHT_BG() -> Color {
    t().highlight_bg
}
