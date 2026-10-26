use base64::{engine::general_purpose::STANDARD, Engine};
use ghgrab::ui::components::image_preview::{
    is_previewable_image, kitty_clear_sequence, ImageProtocol, PreviewImage, MAX_IMAGE_BYTES,
};
use image::{ImageFormat, Rgba, RgbaImage};
use ratatui::style::Color;
use std::collections::HashMap;
use std::io::Cursor;

const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

fn encode(image: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

fn solid_png(width: u32, height: u32) -> Vec<u8> {
    encode(&RgbaImage::from_pixel(width, height, RED), ImageFormat::Png)
}

fn red_over_blue(width: u32, height: u32) -> PreviewImage {
    let image = RgbaImage::from_fn(
        width,
        height,
        |_, y| if y < height / 2 { RED } else { BLUE },
    );
    PreviewImage::decode(&encode(&image, ImageFormat::Png)).unwrap()
}

fn detect(vars: &[(&str, &str)]) -> ImageProtocol {
    let vars: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    ImageProtocol::detect_from(|key| vars.get(key).cloned())
}

fn kitty_chunks(seq: &str) -> Vec<(String, String)> {
    seq.split("\x1b\\")
        .filter(|part| !part.is_empty())
        .map(|part| {
            let body = part
                .strip_prefix("\x1b_G")
                .expect("chunk starts with APC G");
            let (keys, payload) = body.split_once(';').expect("chunk has a payload");
            (keys.to_string(), payload.to_string())
        })
        .collect()
}

#[test]
fn max_image_bytes_is_ten_megabytes() {
    assert_eq!(MAX_IMAGE_BYTES, 10 * 1024 * 1024);
}

#[test]
fn decode_png_reports_original_dimensions() {
    let image = PreviewImage::decode(&solid_png(37, 21)).unwrap();
    assert_eq!(image.dimensions(), (37, 21));
}

#[test]
fn decode_other_formats() {
    let rgb = image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 4, RED)).to_rgb8();
    for format in [ImageFormat::Jpeg, ImageFormat::Bmp, ImageFormat::Gif] {
        let mut bytes = Vec::new();
        rgb.write_to(&mut Cursor::new(&mut bytes), format).unwrap();
        let image = PreviewImage::decode(&bytes).unwrap();
        assert_eq!(image.dimensions(), (8, 4), "{:?}", format);
    }
}

#[test]
fn decode_large_image_keeps_original_dimensions() {
    let image = PreviewImage::decode(&solid_png(2000, 10)).unwrap();
    assert_eq!(image.dimensions(), (2000, 10));
    let (cols, rows) = image.fit(80, 24, 2.0);
    assert!(cols > 0 && rows > 0);
}

#[test]
fn decode_garbage_fails() {
    assert!(PreviewImage::decode(b"definitely not an image").is_err());
    assert!(PreviewImage::decode(&[]).is_err());
    let mut truncated = solid_png(16, 16);
    truncated.truncate(truncated.len() / 2);
    assert!(PreviewImage::decode(&truncated).is_err());
}

#[test]
fn fit_square_image() {
    let image = PreviewImage::decode(&solid_png(100, 100)).unwrap();
    assert_eq!(image.fit(80, 40, 2.0), (80, 40));
    assert_eq!(image.fit(80, 20, 2.0), (40, 20));
    assert_eq!(image.fit(10, 40, 1.0), (10, 10));
}

#[test]
fn fit_wide_and_tall_images() {
    let wide = PreviewImage::decode(&solid_png(400, 100)).unwrap();
    assert_eq!(wide.fit(80, 40, 2.0), (80, 10));

    let tall = PreviewImage::decode(&solid_png(100, 400)).unwrap();
    assert_eq!(tall.fit(80, 40, 2.0), (20, 40));
}

#[test]
fn fit_six_by_seven() {
    // 6 7. the test had to happen eventually
    let image = PreviewImage::decode(&solid_png(6, 7)).unwrap();
    let (cols, rows) = image.fit(60, 70, 1.0);
    assert_eq!((cols, rows), (60, 70));
}

#[test]
fn fit_extreme_aspect_ratios_stay_in_bounds() {
    let cases = [(1, 5000), (5000, 1), (1, 1), (3, 2000), (2000, 3)];
    let boxes = [
        (1, 1),
        (1, 50),
        (50, 1),
        (80, 24),
        (300, 100),
        (u16::MAX, u16::MAX),
    ];
    let ratios = [1.0, 2.0, 4.0, 0.0, f32::NAN];

    for (w, h) in cases {
        let image = PreviewImage::decode(&solid_png(w, h)).unwrap();
        for (max_cols, max_rows) in boxes {
            for ratio in ratios {
                let (cols, rows) = image.fit(max_cols, max_rows, ratio);
                assert!(
                    cols >= 1 && rows >= 1 && cols <= max_cols && rows <= max_rows,
                    "{}x{} in {}x{} @ {} gave {}x{}",
                    w,
                    h,
                    max_cols,
                    max_rows,
                    ratio,
                    cols,
                    rows
                );
            }
        }
    }
}

#[test]
fn fit_zero_box() {
    let image = PreviewImage::decode(&solid_png(10, 10)).unwrap();
    assert_eq!(image.fit(0, 10, 2.0), (0, 0));
    assert_eq!(image.fit(10, 0, 2.0), (0, 0));
}

#[test]
fn halfblocks_has_exact_shape() {
    let image = red_over_blue(64, 48);
    for (cols, rows) in [(1, 1), (10, 3), (33, 17), (80, 24)] {
        let text = image.halfblocks(cols, rows, (0, 0, 0));
        assert_eq!(text.lines.len(), rows as usize);
        for line in &text.lines {
            assert_eq!(line.width(), cols as usize);
            for span in &line.spans {
                assert!(span.content.chars().all(|c| c == '▀'));
            }
        }
    }
}

#[test]
fn halfblocks_top_pixel_is_fg_bottom_pixel_is_bg() {
    let image = red_over_blue(4, 2);
    let text = image.halfblocks(4, 1, (0, 0, 0));
    assert_eq!(text.lines.len(), 1);
    for span in &text.lines[0].spans {
        assert_eq!(span.style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(span.style.bg, Some(Color::Rgb(0, 0, 255)));
    }
}

#[test]
fn halfblocks_downscaled_colors_stay_on_their_side() {
    let image = red_over_blue(200, 200);
    let text = image.halfblocks(10, 1, (0, 0, 0));
    for span in &text.lines[0].spans {
        match (span.style.fg, span.style.bg) {
            (Some(Color::Rgb(tr, _, tb)), Some(Color::Rgb(br, _, bb))) => {
                assert!(tr > tb, "top should be mostly red");
                assert!(bb > br, "bottom should be mostly blue");
            }
            other => panic!("unexpected colors {:?}", other),
        }
    }
}

#[test]
fn halfblocks_transparent_pixels_use_background() {
    let image = RgbaImage::from_pixel(2, 2, Rgba([255, 255, 255, 0]));
    let image = PreviewImage::decode(&encode(&image, ImageFormat::Png)).unwrap();
    let text = image.halfblocks(2, 1, (10, 20, 30));
    for span in &text.lines[0].spans {
        assert_eq!(span.style.fg, Some(Color::Rgb(10, 20, 30)));
        assert_eq!(span.style.bg, Some(Color::Rgb(10, 20, 30)));
    }
}

#[test]
fn halfblocks_half_transparent_pixels_blend() {
    let image = RgbaImage::from_pixel(1, 2, Rgba([255, 255, 255, 128]));
    let image = PreviewImage::decode(&encode(&image, ImageFormat::Png)).unwrap();
    let text = image.halfblocks(1, 1, (0, 0, 0));
    let fg = text.lines[0].spans[0].style.fg;
    assert_eq!(fg, Some(Color::Rgb(128, 128, 128)));
}

#[test]
fn halfblocks_cache_tracks_arguments() {
    let image = red_over_blue(4, 2);
    let first = image.halfblocks(4, 1, (0, 0, 0));
    assert_eq!(image.halfblocks(4, 1, (0, 0, 0)), first);
    assert_eq!(image.halfblocks(2, 3, (0, 0, 0)).lines.len(), 3);
    let transparent = PreviewImage::decode(&encode(
        &RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0])),
        ImageFormat::Png,
    ))
    .unwrap();
    let a = transparent.halfblocks(2, 1, (1, 1, 1));
    let b = transparent.halfblocks(2, 1, (9, 9, 9));
    assert_ne!(a, b);
}

#[test]
fn halfblocks_zero_size_is_empty() {
    let image = red_over_blue(4, 2);
    assert!(image.halfblocks(0, 5, (0, 0, 0)).lines.is_empty());
    assert!(image.halfblocks(5, 0, (0, 0, 0)).lines.is_empty());
}

#[test]
fn kitty_sequence_single_chunk() {
    let image = PreviewImage::decode(&solid_png(2, 2)).unwrap();
    let seq = image.kitty_sequence(12, 6);
    let chunks = kitty_chunks(&seq);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].0, "a=T,f=100,t=d,q=2,C=1,z=-1,c=12,r=6,m=0");
    let png = STANDARD.decode(&chunks[0].1).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
}

#[test]
fn kitty_sequence_multiple_chunks_reassemble() {
    let noisy = RgbaImage::from_fn(300, 300, |x, y| {
        let v = x
            .wrapping_mul(2654435761)
            .wrapping_add(y.wrapping_mul(40503))
            ^ (x * y);
        Rgba([v as u8, (v >> 8) as u8, (v >> 16) as u8, 255])
    });
    let image = PreviewImage::decode(&encode(&noisy, ImageFormat::Png)).unwrap();
    let seq = image.kitty_sequence(40, 20);
    let chunks = kitty_chunks(&seq);
    assert!(
        chunks.len() > 2,
        "expected several chunks, got {}",
        chunks.len()
    );

    let first = &chunks[0].0;
    for key in [
        "a=T", "f=100", "t=d", "q=2", "C=1", "z=-1", "c=40", "r=20", "m=1",
    ] {
        assert!(
            first.split(',').any(|k| k == key),
            "missing {} in {}",
            key,
            first
        );
    }
    for (keys, _) in &chunks[1..chunks.len() - 1] {
        assert_eq!(keys, "m=1,q=2");
    }
    assert_eq!(chunks.last().unwrap().0, "m=0,q=2");

    for (_, payload) in &chunks[..chunks.len() - 1] {
        assert_eq!(payload.len(), 4096);
    }
    assert!(chunks.last().unwrap().1.len() <= 4096);

    let payload: String = chunks.iter().map(|(_, p)| p.as_str()).collect();
    let png = STANDARD.decode(payload).unwrap();
    let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(decoded, noisy);
}

#[test]
fn iterm2_sequence_shape() {
    let image = PreviewImage::decode(&solid_png(3, 3)).unwrap();
    let seq = image.iterm2_sequence(20, 10);
    let rest = seq.strip_prefix("\x1b]1337;File=").unwrap();
    let rest = rest.strip_suffix('\x07').unwrap();
    let (args, payload) = rest.split_once(':').unwrap();
    let png = STANDARD.decode(payload).unwrap();
    assert_eq!(
        args,
        format!(
            "inline=1;size={};width=20;height=10;preserveAspectRatio=1;doNotMoveCursor=1",
            png.len()
        )
    );
    assert!(png.starts_with(b"\x89PNG"));
}

#[test]
fn kitty_clear_deletes_everything() {
    assert_eq!(kitty_clear_sequence(), "\x1b_Ga=d,d=A,q=2\x1b\\");
}

#[test]
fn detect_override_wins() {
    let kitty_env = [("KITTY_WINDOW_ID", "1"), ("TERM", "xterm-kitty")];
    let cases = [
        ("kitty", ImageProtocol::Kitty),
        ("KITTY", ImageProtocol::Kitty),
        ("iterm2", ImageProtocol::Iterm2),
        ("iTerm2", ImageProtocol::Iterm2),
        ("halfblocks", ImageProtocol::Halfblocks),
        ("none", ImageProtocol::Disabled),
        ("None", ImageProtocol::Disabled),
    ];
    for (value, expected) in cases {
        let mut vars = vec![("GHGRAB_IMAGE_PROTOCOL", value)];
        assert_eq!(detect(&vars), expected, "{}", value);
        vars.extend(kitty_env);
        vars.push(("TMUX", "/tmp/tmux-1000/default,1,0"));
        assert_eq!(detect(&vars), expected, "{} under tmux", value);
    }
}

#[test]
fn detect_auto_and_unknown_override_fall_through() {
    for value in ["auto", "AUTO", "sixel", ""] {
        assert_eq!(
            detect(&[("GHGRAB_IMAGE_PROTOCOL", value), ("TERM", "xterm-kitty")]),
            ImageProtocol::Kitty,
            "{}",
            value
        );
        assert_eq!(
            detect(&[("GHGRAB_IMAGE_PROTOCOL", value)]),
            ImageProtocol::Halfblocks
        );
    }
}

#[test]
fn detect_tmux_and_screen_use_halfblocks() {
    assert_eq!(
        detect(&[
            ("TMUX", "/tmp/tmux-1000/default,1,0"),
            ("KITTY_WINDOW_ID", "1")
        ]),
        ImageProtocol::Halfblocks
    );
    assert_eq!(
        detect(&[("TERM", "screen-256color"), ("TERM_PROGRAM", "iTerm.app")]),
        ImageProtocol::Halfblocks
    );
    assert_eq!(
        detect(&[("TERM", "tmux-256color"), ("TERM_PROGRAM", "WezTerm")]),
        ImageProtocol::Halfblocks
    );
}

#[test]
fn detect_kitty_terminals() {
    assert_eq!(detect(&[("KITTY_WINDOW_ID", "3")]), ImageProtocol::Kitty);
    assert_eq!(detect(&[("TERM", "xterm-kitty")]), ImageProtocol::Kitty);
    assert_eq!(detect(&[("TERM_PROGRAM", "ghostty")]), ImageProtocol::Kitty);
    assert_eq!(detect(&[("TERM_PROGRAM", "Ghostty")]), ImageProtocol::Kitty);
    assert_eq!(detect(&[("TERM", "xterm-ghostty")]), ImageProtocol::Kitty);
}

#[test]
fn detect_iterm2() {
    assert_eq!(
        detect(&[("TERM_PROGRAM", "iTerm.app")]),
        ImageProtocol::Iterm2
    );
    assert_eq!(detect(&[("LC_TERMINAL", "iTerm2")]), ImageProtocol::Iterm2);
    assert_eq!(
        detect(&[("TERM_PROGRAM", "WezTerm")]),
        ImageProtocol::Iterm2
    );
    assert_eq!(
        detect(&[("LC_TERMINAL", "iTerm2"), ("KITTY_WINDOW_ID", "1")]),
        ImageProtocol::Kitty
    );
}

#[test]
fn detect_defaults_to_halfblocks() {
    assert_eq!(detect(&[]), ImageProtocol::Halfblocks);
    assert_eq!(
        detect(&[
            ("TERM", "xterm-256color"),
            ("TERM_PROGRAM", "Apple_Terminal")
        ]),
        ImageProtocol::Halfblocks
    );
}

#[test]
fn protocol_labels_and_graphics() {
    assert!(ImageProtocol::Kitty.is_graphics());
    assert!(ImageProtocol::Iterm2.is_graphics());
    assert!(!ImageProtocol::Halfblocks.is_graphics());
    assert!(!ImageProtocol::Disabled.is_graphics());
    assert_eq!(ImageProtocol::Kitty.label(), "kitty");
    assert_eq!(ImageProtocol::Iterm2.label(), "iTerm2");
    assert_eq!(ImageProtocol::Halfblocks.label(), "halfblocks");
    assert_eq!(ImageProtocol::Disabled.label(), "disabled");
}

#[test]
fn previewable_image_extensions() {
    for path in [
        "a.png",
        "dir/b.JPG",
        "c.jpeg",
        "d.Gif",
        "e.bmp",
        "f.WEBP",
        "nested/path/logo.PnG",
    ] {
        assert!(is_previewable_image(path), "{}", path);
    }
    for path in [
        "a.svg",
        "b.mp4",
        "png",
        "notes.png.txt",
        "Makefile",
        "c.tiff",
    ] {
        assert!(!is_previewable_image(path), "{}", path);
    }
}

#[test]
fn cell_ratio_is_clamped() {
    let ratio = ghgrab::ui::components::image_preview::cell_ratio();
    assert!((1.0..=4.0).contains(&ratio));
}

#[test]
fn preview_image_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PreviewImage>();
}
