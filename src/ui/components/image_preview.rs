use anyhow::Context;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, ImageFormat, RgbaImage};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

const MAX_STORED_SIDE: u32 = 1024;
const KITTY_CHUNK: usize = 4096;
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    Kitty,
    Iterm2,
    Halfblocks,
    Disabled,
}

impl ImageProtocol {
    pub fn detect() -> Self {
        Self::detect_from(|key| std::env::var(key).ok())
    }

    pub fn detect_from(env: impl Fn(&str) -> Option<String>) -> Self {
        let get = |key: &str| env(key).filter(|value| !value.is_empty());
        let is = |key: &str, expected: &str| {
            get(key).is_some_and(|value| value.eq_ignore_ascii_case(expected))
        };

        if let Some(value) = get("GHGRAB_IMAGE_PROTOCOL") {
            match value.trim().to_ascii_lowercase().as_str() {
                "kitty" => return Self::Kitty,
                "iterm2" => return Self::Iterm2,
                "halfblocks" => return Self::Halfblocks,
                "none" => return Self::Disabled,
                _ => {}
            }
        }

        let term = get("TERM").unwrap_or_default();
        if get("TMUX").is_some() || term.starts_with("screen") || term.starts_with("tmux") {
            return Self::Halfblocks;
        }

        if get("KITTY_WINDOW_ID").is_some()
            || term == "xterm-kitty"
            || is("TERM_PROGRAM", "ghostty")
            || term == "xterm-ghostty"
        {
            return Self::Kitty;
        }

        if is("TERM_PROGRAM", "iTerm.app")
            || is("LC_TERMINAL", "iTerm2")
            || is("TERM_PROGRAM", "WezTerm")
        {
            return Self::Iterm2;
        }

        Self::Halfblocks
    }

    pub fn is_graphics(self) -> bool {
        matches!(self, Self::Kitty | Self::Iterm2)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Kitty => "kitty",
            Self::Iterm2 => "iTerm2",
            Self::Halfblocks => "halfblocks",
            Self::Disabled => "disabled",
        }
    }
}

pub fn is_previewable_image(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
}

type HalfblockKey = (u16, u16, (u8, u8, u8));

pub struct PreviewImage {
    width: u32,
    height: u32,
    rgba: RgbaImage,
    png: Vec<u8>,
    cache: Mutex<Option<(HalfblockKey, Text<'static>)>>,
}

impl PreviewImage {
    pub fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        let image = image::load_from_memory(bytes).context("Failed to decode image")?;
        let (width, height) = (image.width(), image.height());

        let image = if width.max(height) > MAX_STORED_SIDE {
            image.resize(MAX_STORED_SIDE, MAX_STORED_SIDE, FilterType::Triangle)
        } else {
            image
        };
        let rgba = image.into_rgba8();

        let mut png = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .context("Failed to encode image as PNG")?;

        Ok(Self {
            width,
            height,
            rgba,
            png,
            cache: Mutex::new(None),
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn fit(&self, max_cols: u16, max_rows: u16, cell_ratio: f32) -> (u16, u16) {
        if max_cols == 0 || max_rows == 0 {
            return (0, 0);
        }

        let cell_ratio = if cell_ratio.is_finite() && cell_ratio > 0.0 {
            cell_ratio as f64
        } else {
            2.0
        };
        let aspect = self.width.max(1) as f64 / self.height.max(1) as f64 * cell_ratio;

        let rows = max_cols as f64 / aspect;
        if rows <= max_rows as f64 {
            (max_cols, clamp_cells(rows, max_rows))
        } else {
            (clamp_cells(max_rows as f64 * aspect, max_cols), max_rows)
        }
    }

    pub fn halfblocks(&self, cols: u16, rows: u16, background: (u8, u8, u8)) -> Text<'static> {
        if cols == 0 || rows == 0 {
            return Text::default();
        }

        let key = (cols, rows, background);
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached_key, text)) = cache.as_ref() {
            if *cached_key == key {
                return text.clone();
            }
        }

        let text = self.render_halfblocks(cols, rows, background);
        *cache = Some((key, text.clone()));
        text
    }

    fn render_halfblocks(&self, cols: u16, rows: u16, background: (u8, u8, u8)) -> Text<'static> {
        let width = cols as u32;
        let height = rows as u32 * 2;
        let scaled = image::imageops::resize(&self.rgba, width, height, FilterType::Triangle);

        let lines: Vec<Line<'static>> = (0..rows as u32)
            .map(|row| {
                let mut spans: Vec<Span<'static>> = Vec::new();
                let mut current: Option<(Style, String)> = None;

                for x in 0..width {
                    let top = composite(scaled.get_pixel(x, row * 2).0, background);
                    let bottom = composite(scaled.get_pixel(x, row * 2 + 1).0, background);
                    let style = Style::default().fg(top).bg(bottom);

                    match current.as_mut() {
                        Some((current_style, text)) if *current_style == style => text.push('▀'),
                        _ => {
                            if let Some((prev_style, text)) = current.take() {
                                spans.push(Span::styled(text, prev_style));
                            }
                            current = Some((style, String::from("▀")));
                        }
                    }
                }

                if let Some((style, text)) = current {
                    spans.push(Span::styled(text, style));
                }
                Line::from(spans)
            })
            .collect();

        Text::from(lines)
    }

    pub fn kitty_sequence(&self, cols: u16, rows: u16) -> String {
        let payload = STANDARD.encode(&self.png);
        let chunks: Vec<&[u8]> = payload.as_bytes().chunks(KITTY_CHUNK).collect();
        let last = chunks.len().saturating_sub(1);

        let mut out = String::with_capacity(payload.len() + chunks.len() * 16 + 64);
        for (i, chunk) in chunks.iter().enumerate() {
            let more = if i == last { 0 } else { 1 };
            let chunk = std::str::from_utf8(chunk).unwrap_or_default();
            if i == 0 {
                out.push_str(&format!(
                    "\x1b_Ga=T,f=100,t=d,q=2,C=1,z=-1,c={},r={},m={};{}\x1b\\",
                    cols, rows, more, chunk
                ));
            } else {
                out.push_str(&format!("\x1b_Gm={},q=2;{}\x1b\\", more, chunk));
            }
        }
        out
    }

    pub fn iterm2_sequence(&self, cols: u16, rows: u16) -> String {
        format!(
            "\x1b]1337;File=inline=1;size={};width={};height={};preserveAspectRatio=1;doNotMoveCursor=1:{}\x07",
            self.png.len(),
            cols,
            rows,
            STANDARD.encode(&self.png)
        )
    }
}

pub fn kitty_clear_sequence() -> &'static str {
    "\x1b_Ga=d,d=A,q=2\x1b\\"
}

pub fn cell_ratio() -> f32 {
    // most fonts sit near 1:2; a 6x7 px cell would be... 6 7, so we clamp
    match crossterm::terminal::window_size() {
        Ok(size) if size.width > 0 && size.height > 0 && size.columns > 0 && size.rows > 0 => {
            let cell_width = size.width as f32 / size.columns as f32;
            let cell_height = size.height as f32 / size.rows as f32;
            (cell_height / cell_width).clamp(1.0, 4.0)
        }
        _ => 2.0,
    }
}

fn clamp_cells(value: f64, max: u16) -> u16 {
    value.round().clamp(1.0, max as f64) as u16
}

fn composite(pixel: [u8; 4], background: (u8, u8, u8)) -> Color {
    let [r, g, b, a] = pixel;
    let blend = |fg: u8, bg: u8| -> u8 {
        let a = a as u32;
        ((fg as u32 * a + bg as u32 * (255 - a) + 127) / 255) as u8
    };
    Color::Rgb(
        blend(r, background.0),
        blend(g, background.1),
        blend(b, background.2),
    )
}
