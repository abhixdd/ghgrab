use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::github::RepoSearchItem;
use crate::ui::theme::*;

pub struct RepoResultsState<'a> {
    pub query: &'a str,
    pub filter_query: &'a str,
    pub results: &'a [RepoSearchItem],
    pub ranked_indices: &'a [usize],
    pub cursor: usize,
    pub scroll_offset: usize,
    pub status_msg: &'a str,
    pub is_loading_more: bool,
    pub total_count: u64,
}

pub fn render(f: &mut Frame, area: Rect, state: &RepoResultsState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(area);

    let filter_display = if state.filter_query.is_empty() {
        "(type to fuzzy filter)".to_string()
    } else {
        state.filter_query.to_string()
    };
    let header = Paragraph::new(format!(
        " Query: {}  |  Fuzzy: {} ",
        state.query, filter_display
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Repository Discovery ")
            .border_style(Style::default().fg(ACCENT_COLOR))
            .style(Style::default().bg(BG_COLOR)),
    )
    .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    let mut list_items: Vec<ListItem> = state
        .ranked_indices
        .iter()
        .copied()
        .skip(state.scroll_offset)
        .filter_map(|repo_idx| state.results.get(repo_idx).map(|repo| (repo_idx, repo)))
        .enumerate()
        .map(|(visible_idx, (_repo_idx, repo))| {
            let absolute_idx = state.scroll_offset + visible_idx;
            let is_selected = absolute_idx == state.cursor;
            let description = repo
                .description
                .as_deref()
                .unwrap_or("No description")
                .replace('\n', " ");
            let trimmed_description: String = description.chars().take(58).collect();
            let branch = repo.default_branch.as_deref().unwrap_or("main").to_string();
            let line = Line::from(vec![
                Span::styled("★ ", Style::default().fg(WARNING_COLOR)),
                Span::styled(
                    format!("{:<32}", repo.full_name),
                    if is_selected {
                        Style::default()
                            .fg(ACCENT_COLOR)
                            .add_modifier(Modifier::BOLD)
                            .bg(HIGHLIGHT_BG)
                    } else {
                        Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled(
                    format!(" {:>6}  ", repo.stargazers_count),
                    Style::default().fg(SUCCESS_COLOR),
                ),
                Span::styled(
                    format!("{}  [{}]", trimmed_description, branch),
                    Style::default().fg(BORDER_COLOR),
                ),
            ]);
            let item = ListItem::new(line);
            if is_selected {
                item.style(Style::default().bg(HIGHLIGHT_BG))
            } else {
                item
            }
        })
        .collect();

    if state.is_loading_more {
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled("⏳ ", Style::default().fg(WARNING_COLOR)),
            Span::styled(
                "Loading more repositories...",
                Style::default()
                    .fg(WARNING_COLOR)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])));
    }

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Results ({}/{}) ",
                state.ranked_indices.len(),
                state.total_count
            ))
            .border_style(Style::default().fg(BORDER_COLOR))
            .style(Style::default().bg(BG_COLOR)),
    );
    f.render_widget(list, chunks[1]);

    let status_style = if state.is_loading_more {
        Style::default()
            .fg(SUCCESS_COLOR)
            .bg(BG_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(WARNING_COLOR).bg(BG_COLOR)
    };
    let status = Paragraph::new(state.status_msg.to_string()).style(status_style);
    f.render_widget(status, chunks[2]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑/↓",
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Move  ", Style::default().fg(BORDER_COLOR)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(SUCCESS_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Open  ", Style::default().fg(BORDER_COLOR)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(ERROR_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Back  ", Style::default().fg(BORDER_COLOR)),
        Span::styled(
            "Type",
            Style::default()
                .fg(SUCCESS_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Filter", Style::default().fg(BORDER_COLOR)),
    ]))
    .alignment(ratatui::layout::Alignment::Center)
    .style(Style::default().bg(BG_COLOR));
    f.render_widget(help, chunks[3]);
}
