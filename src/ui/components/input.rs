use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, input_text: &str, status_msg: &str, cursor_visible: bool) {
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(12),  // Top padding
            Constraint::Length(8),       // Header
            Constraint::Length(2),       // Description
            Constraint::Length(1),       // Spacing
            Constraint::Length(3),       // Input box
            Constraint::Length(1),       // Spacing
            Constraint::Length(9),       // Instructions & Examples
            Constraint::Length(2),       // Controls
            Constraint::Min(0),          // Bottom padding
        ])
        .split(area);

    // Header area
    let header_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical_layout[1]);

    // ASCII Header
    let header_lines = vec![
        Line::from(Span::styled(
            "  ██████╗ ██╗  ██╗ ██████╗ ██████╗  █████╗ ██████╗ ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██╔════╝ ██║  ██║██╔════╝ ██╔══██╗██╔══██╗██╔══██╗",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██║  ███╗███████║██║  ███╗██████╔╝███████║██████╔╝",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██║   ██║██╔══██║██║   ██║██╔══██╗██╔══██║██╔══██╗",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ╚██████╔╝██║  ██║╚██████╔╝██║  ██║██║  ██║██████╔╝",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
    ];
    let header = Paragraph::new(header_lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(header, header_area[1]);


    let desc_text = Line::from(Span::styled(
        "Download any file or folder from GitHub. No full clones. Just what you need.",
        Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC),
    ));
    let desc = Paragraph::new(desc_text)
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(desc, vertical_layout[2]);


    let input_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(vertical_layout[4]);

    let display_text = if cursor_visible {
        format!("{}_", input_text)
    } else {
        format!("{} ", input_text)
    };
    
    let input = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Enter GitHub URL ",
                    Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)
                ))
                .border_style(Style::default().fg(ACCENT_COLOR))
                .style(Style::default().bg(BG_COLOR)),
        )
        .style(Style::default().fg(FG_COLOR));
    f.render_widget(input, input_area[1]);

    let instructions_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical_layout[6]);

    let instructions = vec![
        Line::from(vec![
            Span::styled("Examples", Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled("  (paste any of these):", Style::default().fg(FG_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(BORDER_COLOR)),
            Span::styled("https://github.com/abhixdd/ghgrab", Style::default().fg(ACCENT_COLOR)),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(BORDER_COLOR)),
            Span::styled("https://github.com/rust-lang/rust/tree/master/src/tools", Style::default().fg(ACCENT_COLOR)),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(BORDER_COLOR)),
            Span::styled("https://github.com/user/repo/tree/main/specific-folder", Style::default().fg(ACCENT_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(WARNING_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled("Works with any public GitHub repository or folder path", Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC)),
        ]),
    ];
        
    let info = Paragraph::new(instructions)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_COLOR))
                .style(Style::default().bg(BG_COLOR))
        )
        .wrap(Wrap { trim: false });
    f.render_widget(info, instructions_area[1]);

    // Keyboard controls
    let controls_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical_layout[7]);

    let controls = vec![
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(" - Start", Style::default().fg(FG_COLOR)),
            Span::styled("     |     ", Style::default().fg(BORDER_COLOR)),
            Span::styled("ESC", Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(" - Quit", Style::default().fg(FG_COLOR)),
        ]),
    ];
    let controls_widget = Paragraph::new(controls)
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(controls_widget, controls_area[1]);
    
    // Status Bar 
    if !status_msg.is_empty() {
        let status_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        
        let status = Paragraph::new(format!(" {}", status_msg))
            .style(Style::default().fg(ERROR_COLOR).bg(BG_COLOR));
        f.render_widget(status, status_area[1]);
    }
}
