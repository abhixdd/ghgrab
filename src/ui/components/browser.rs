use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::github::{GitHubUrl, RepoItem};
use crate::ui::theme::*;

pub struct BrowserState<'a> {
    pub items: &'a [RepoItem],
    pub current_url: Option<&'a GitHubUrl>,
    pub cursor: usize,
    pub scroll_offset: usize,
    #[allow(dead_code)]
    pub status_msg: &'a str,
    pub is_downloading: bool,
}

pub fn render(f: &mut Frame, area: Rect, state: &BrowserState) {
    let chunks = if state.is_downloading {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Breadcrumb
                Constraint::Min(10),    // File list
                Constraint::Length(2),  // Download status
                Constraint::Length(2),  
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Breadcrumb
                Constraint::Min(10),    // File list
                Constraint::Length(1),  // Spacer
                Constraint::Length(2),  
            ])
            .split(area)
    };

    let breadcrumb_text = if let Some(url) = state.current_url {
        format!(" {}/{} : {}", url.owner, url.repo, url.path)
    } else {
        " Loading...".to_string()
    };

    let header = Paragraph::new(breadcrumb_text)
        .block(
            Block::default()
                .borders(Borders::ALL)  
                .title(" Repository ")
                .border_style(Style::default().fg(ACCENT_COLOR))
                .style(Style::default().bg(BG_COLOR)),
        )
        .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    fn format_size(size: u64) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    fn get_file_type(name: &str, is_dir: bool) -> String {
        if is_dir {
            "DIR".to_string()
        } else {
            name.rsplit('.')
                .next()
                .filter(|ext| !ext.is_empty() && ext.len() <= 5)
                .map(|ext| ext.to_uppercase())
                .unwrap_or_else(|| "FILE".to_string())
        }
    }

    let header_line = Line::from(vec![
        Span::styled("    ", Style::default().bg(BORDER_COLOR)),
        Span::styled(format!("{:<41}", "Name"), Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD).bg(BORDER_COLOR)),
        Span::styled("  ", Style::default().bg(BORDER_COLOR)),
        Span::styled(format!("{:<8}", "Type"), Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD).bg(BORDER_COLOR)),
        Span::styled("  ", Style::default().bg(BORDER_COLOR)),
        Span::styled("  ", Style::default().bg(BORDER_COLOR)),
    ]);
    let header_item = ListItem::new(header_line).style(Style::default().bg(BORDER_COLOR));

    let mut all_items = vec![header_item];
    
    let file_items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .skip(state.scroll_offset)
        .map(|(idx, item)| {
            let is_selected = idx == state.cursor;
            
            let icon = if item.is_dir() { "📁 " } else { "📄 " };

            let mark = if item.selected { 
                Span::styled("[●] ", Style::default().fg(SUCCESS_COLOR))
            } else { 
                Span::styled("[ ] ", Style::default().fg(BORDER_COLOR)) 
            };
            
            let name_style = if is_selected {
                Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD).bg(HIGHLIGHT_BG)
            } else if item.is_dir() {
                Style::default().fg(FOLDER_COLOR)
            } else {
                Style::default().fg(FG_COLOR)
            };

            let file_type = get_file_type(&item.name, item.is_dir());
            
            let size_display = if !item.is_dir() {
                item.actual_size()
                    .map(|s| format!("{:>12}", format_size(s)))
                    .unwrap_or_else(|| format!("{:>12}", "-"))
            } else {
                format!("{:>12}", "")
            };

            let display_name = if item.name.len() > 35 {
                if let Some(dot_pos) = item.name.rfind('.') {
                    let ext = &item.name[dot_pos..];
                    let name_part = &item.name[..dot_pos];
                    if name_part.len() > 30 {
                        format!("{}.....{}", &name_part[..30], ext)
                    } else {
                        item.name.clone()
                    }
                } else {
                    format!("{}.....", &item.name[..35])
                }
            } else {
                item.name.clone()
            };

            let name_with_icon = format!("{}{}", icon, display_name);
            let name_display = format!("{:<40}", name_with_icon);
            
            let content = Line::from(vec![
                mark,
                Span::styled(name_display, name_style.clone()),
                Span::styled("  ", Style::default()),
                Span::styled(format!("{:<8}", file_type), Style::default().fg(WARNING_COLOR)),
                Span::styled("  ", Style::default()),
                Span::styled(size_display, Style::default().fg(BORDER_COLOR)),
            ]);
            
            let item = ListItem::new(content);
            if is_selected {
                item.style(Style::default().bg(HIGHLIGHT_BG))
            } else {
                item
            }
        })
        .collect();

    all_items.extend(file_items);

    let list = List::new(all_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Files ({}) ", state.items.len()))
            .border_style(Style::default().fg(if state.is_downloading { WARNING_COLOR } else { BORDER_COLOR }))
            .style(Style::default().bg(BG_COLOR)),
    );
    f.render_widget(list, chunks[1]);
    

    // Download Status Section 
    if state.is_downloading {
        let status_text = if state.status_msg.is_empty() {
            "Starting download...".to_string()
        } else {
            state.status_msg.to_string()
        };
        
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" ⬇ ", Style::default().fg(BG_COLOR).bg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(" ", Style::default()),
            Span::styled(status_text, Style::default().fg(WARNING_COLOR).add_modifier(Modifier::BOLD)),
        ]))
        .style(Style::default().bg(BG_COLOR));
        f.render_widget(status, chunks[2]);
    }

    let help_spans = vec![
        Span::styled("  ", Style::default()),
        Span::styled("↑↓", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("Enter", Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Open", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("Space", Style::default().fg(WARNING_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Select", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("a", Style::default().fg(FOLDER_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled("/", Style::default().fg(BORDER_COLOR)),
        Span::styled("u", Style::default().fg(FOLDER_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" All/None", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("d", Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Download", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("←", Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Back", Style::default().fg(BORDER_COLOR)),
        Span::styled("  │  ", Style::default().fg(BORDER_COLOR)),
        Span::styled("q", Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(" Quit", Style::default().fg(BORDER_COLOR)),
        Span::styled("  ", Style::default()),
    ];
    let help = Paragraph::new(Line::from(help_spans))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(help, chunks[3]);
}
