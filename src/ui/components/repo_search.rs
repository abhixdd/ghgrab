use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::github::SearchItem;
use crate::ui::theme::{
    ACCENT_COLOR, BG_COLOR, BORDER_COLOR, ERROR_COLOR, FG_COLOR, HIGHLIGHT_BG, SUCCESS_COLOR,
    WARNING_COLOR,
};
use crate::ui::{RepoSearchFilters, RepoSearchSort};

pub struct RepoSearchState<'a> {
    pub results: &'a [SearchItem],
    pub total_results: usize,
    pub cursor: usize,
    pub query: &'a str,
    pub filters: &'a RepoSearchFilters,
    pub loading: bool,
    pub status_msg: &'a str,
}

pub fn render(f: &mut Frame, area: Rect, state: &RepoSearchState) {
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " ghgrab ",
                Style::default()
                    .fg(BG_COLOR())
                    .bg(ACCENT_COLOR())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "Repository Search",
                Style::default().fg(FG_COLOR()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Query: ", Style::default().fg(BORDER_COLOR())),
            Span::styled(
                format!("\"{}\"", state.query),
                Style::default()
                    .fg(ACCENT_COLOR())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
            Span::styled(
                format!("{} shown", state.results.len()),
                Style::default().fg(SUCCESS_COLOR()),
            ),
            Span::styled(" of ", Style::default().fg(BORDER_COLOR())),
            Span::styled(
                state.total_results.to_string(),
                Style::default().fg(FG_COLOR()).add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search Results ")
            .border_style(Style::default().fg(ACCENT_COLOR()))
            .style(Style::default().bg(BG_COLOR())),
    );
    f.render_widget(title, vertical_layout[0]);

    let filter_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(vertical_layout[1]);

    render_filter_chip(
        f,
        filter_layout[0],
        "Forks",
        if state.filters.include_forks {
            "Included"
        } else {
            "Hidden"
        },
        if state.filters.include_forks {
            SUCCESS_COLOR()
        } else {
            BORDER_COLOR()
        },
    );
    render_filter_chip(
        f,
        filter_layout[1],
        "Min Stars",
        &format_min_stars(state.filters.min_stars),
        WARNING_COLOR(),
    );
    render_filter_chip(
        f,
        filter_layout[2],
        "Language",
        state.filters.language.as_deref().unwrap_or("Any"),
        SUCCESS_COLOR(),
    );
    render_filter_chip(
        f,
        filter_layout[3],
        "Sort",
        match state.filters.sort {
            RepoSearchSort::Stars => "Stars",
            RepoSearchSort::Updated => "Updated",
            RepoSearchSort::Name => "Name",
        },
        ACCENT_COLOR(),
    );

    let status_text = if state.loading {
        "Fetching repositories from GitHub..."
    } else if state.results.is_empty() && state.total_results > 0 {
        "No repositories match the current filters. Press x to reset them."
    } else if state.results.is_empty() {
        "No repositories found yet. Try another keyword."
    } else if state.status_msg.is_empty() {
        "Choose a repository to open its files."
    } else {
        state.status_msg
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            " Search ",
            Style::default().fg(BG_COLOR()).bg(SUCCESS_COLOR()),
        ),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(FG_COLOR())),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR()))
            .style(Style::default().bg(BG_COLOR())),
    );
    f.render_widget(status, vertical_layout[2]);

    if state.loading && state.total_results == 0 {
        let loading_widget = Paragraph::new("\nSearching GitHub repositories...")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(FG_COLOR())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Results ")
                    .border_style(Style::default().fg(BORDER_COLOR())),
            );
        f.render_widget(loading_widget, vertical_layout[3]);
    } else {
        let items: Vec<ListItem> = state
            .results
            .iter()
            .enumerate()
            .map(|(i, item)| build_result_item(area.width, i == state.cursor, item))
            .collect();

        let mut list_state = ListState::default();
        if !state.results.is_empty() {
            list_state.select(Some(
                state.cursor.min(state.results.len().saturating_sub(1)),
            ));
        }

        let results_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Results ({}) ", state.results.len()))
                    .border_style(Style::default().fg(BORDER_COLOR()))
                    .style(Style::default().bg(BG_COLOR())),
            )
            .highlight_style(Style::default().bg(HIGHLIGHT_BG()))
            .highlight_symbol("");

        f.render_stateful_widget(results_list, vertical_layout[3], &mut list_state);
    }

    let help_spans = vec![
        Span::styled(
            "↑/↓",
            Style::default()
                .fg(ACCENT_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Move", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "Enter",
            Style::default()
                .fg(SUCCESS_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Open", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "f",
            Style::default()
                .fg(SUCCESS_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Forks", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "m",
            Style::default()
                .fg(WARNING_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Stars", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "l",
            Style::default()
                .fg(SUCCESS_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Language", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "s",
            Style::default()
                .fg(ACCENT_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Sort", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "x",
            Style::default()
                .fg(WARNING_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Reset", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "r",
            Style::default()
                .fg(ACCENT_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Refresh", Style::default().fg(BORDER_COLOR())),
        Span::styled("  |  ", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            "Esc",
            Style::default()
                .fg(ERROR_COLOR())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Back", Style::default().fg(BORDER_COLOR())),
    ];
    let help_widget = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_COLOR()));
    f.render_widget(help_widget, vertical_layout[4]);
}

fn render_filter_chip(f: &mut Frame, area: Rect, label: &str, value: &str, color: Color) {
    let chip = Paragraph::new(vec![
        Line::from(Span::styled(
            label,
            Style::default()
                .fg(BORDER_COLOR())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR()))
            .style(Style::default().bg(BG_COLOR())),
    );
    f.render_widget(chip, area);
}

fn build_result_item(width: u16, is_selected: bool, item: &SearchItem) -> ListItem<'static> {
    let parts: Vec<&str> = item.full_name.split('/').collect();
    let (owner, repo) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("", item.full_name.as_str())
    };

    let title_style = if is_selected {
        Style::default()
            .fg(ACCENT_COLOR())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT_COLOR())
    };

    let mut meta_spans = vec![
        Span::styled(
            format!(" {} stars ", item.stargazers_count),
            Style::default().fg(WARNING_COLOR()),
        ),
        Span::styled("•", Style::default().fg(BORDER_COLOR())),
        Span::styled(
            format!(" updated {}", format_date(&item.pushed_at)),
            Style::default().fg(BORDER_COLOR()),
        ),
    ];

    if let Some(language) = &item.language {
        meta_spans.push(Span::styled("  •  ", Style::default().fg(BORDER_COLOR())));
        meta_spans.push(Span::styled(
            language.clone(),
            Style::default().fg(SUCCESS_COLOR()),
        ));
    }

    if item.fork {
        meta_spans.push(Span::styled("  •  ", Style::default().fg(BORDER_COLOR())));
        meta_spans.push(Span::styled("Fork", Style::default().fg(FG_COLOR())));
    }

    let description = item
        .description
        .as_deref()
        .unwrap_or("No description provided.");
    let desc_limit = width.saturating_sub(14) as usize;
    let trimmed_desc = truncate_text(description, desc_limit.max(40));

    let lines = vec![
        Line::from(vec![
            Span::styled(if is_selected { "› " } else { "  " }, title_style),
            Span::styled(format!("{}/", owner), Style::default().fg(BORDER_COLOR())),
            Span::styled(repo.to_string(), title_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                trimmed_desc,
                Style::default()
                    .fg(FG_COLOR())
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
        Line::from(meta_spans),
        Line::from(""),
    ];

    let item = ListItem::new(lines);
    if is_selected {
        item.style(Style::default().bg(HIGHLIGHT_BG()))
    } else {
        item
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

fn format_min_stars(min_stars: u32) -> String {
    if min_stars == 0 {
        "Any".to_string()
    } else {
        format!("{}+", min_stars)
    }
}

fn format_date(date: &str) -> &str {
    date.split('T').next().unwrap_or(date)
}
