use crate::ui::components::toast::{Toast, ToastType};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::text::Text;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::github::{GitHubClient, GitHubError, GitHubUrl, RepoItem};
use crate::ui::components::syntax_highlighting::highlight_content;

pub mod components;
pub mod theme;

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppMode {
    Input,
    Searching,
    Browse,
    Preview,
}

pub struct AppState {
    pub mode: AppMode,
    pub url_input: String,
    pub url_cursor: usize,
    pub current_url: Option<GitHubUrl>,
    pub items: Vec<RepoItem>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub status_message: String,
    pub downloading: bool,
    pub navigation_stack: Vec<(GitHubUrl, usize)>,
    pub frame_count: u64,
    pub toast: Option<Toast>,
    pub ascii_mode: bool,
    pub github_token: Option<String>,
    pub download_path: Option<String>,
    pub full_tree: Option<Vec<RepoItem>>,
    pub folder_sizes: HashMap<String, u64>,
    pub cwd: bool,
    pub no_folder: bool,
    pub is_searching: bool,
    pub search_query: String,
    pub selected_paths: HashSet<String>,
    pub preview_content: String,
    pub preview_text: Option<Text<'static>>,
    pub preview_path: String,
    pub preview_loading: bool,
    pub preview_is_image: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            mode: AppMode::Input,
            url_input: String::new(),
            url_cursor: 0,
            current_url: None,
            items: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            status_message: String::new(),
            downloading: false,
            navigation_stack: Vec::new(),
            frame_count: 0,
            toast: None,
            ascii_mode: false,
            github_token: None,
            download_path: None,
            full_tree: None,
            folder_sizes: HashMap::new(),
            cwd: false,
            no_folder: false,
            is_searching: false,
            search_query: String::new(),
            selected_paths: HashSet::new(),
            preview_content: String::new(),
            preview_text: None,
            preview_path: String::new(),
            preview_loading: false,
            preview_is_image: false,
        }
    }

    pub fn show_toast(&mut self, message: String, type_: ToastType) {
        self.toast = Some(Toast::new(message, type_));
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        self.adjust_scroll();
    }

    pub fn move_down(&mut self, item_count: usize) {
        if self.cursor < item_count.saturating_sub(1) {
            self.cursor += 1;
        }
        self.adjust_scroll();
    }

    pub fn move_top(&mut self) {
        self.cursor = 0;
        self.adjust_scroll();
    }

    pub fn move_bottom(&mut self, item_count: usize) {
        if item_count > 0 {
            self.cursor = item_count - 1;
        }
        self.adjust_scroll();
    }

    fn adjust_scroll(&mut self) {
        let visible_height = 10;
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn loop_selection(&mut self, select: bool) {
        for item in &mut self.items {
            item.selected = select;
            if select {
                self.selected_paths.insert(item.path.clone());
            } else {
                self.selected_paths.remove(&item.path);
            }
        }
    }

    pub fn get_view_items(&self) -> Vec<RepoItem> {
        let mut items = if self.is_searching {
            let source = self.full_tree.as_ref().unwrap_or(&self.items);
            source
                .iter()
                .filter(|item| {
                    item.path
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                })
                .cloned()
                .collect()
        } else {
            self.items.clone()
        };

        // Sync selections
        for item in &mut items {
            item.selected = self.selected_paths.contains(&item.path);
        }
        items
    }

    pub fn sync_selections(&mut self) {
        for item in &mut self.items {
            item.selected = self.selected_paths.contains(&item.path);
        }
    }

    pub fn toggle_selection(&mut self) {
        let items = self.get_view_items();
        if let Some(item) = items.get(self.cursor) {
            if self.selected_paths.contains(&item.path) {
                self.selected_paths.remove(&item.path);
            } else {
                self.selected_paths.insert(item.path.clone());
            }
        }
        self.sync_selections();
    }

    pub fn get_selected_items(&self) -> Vec<RepoItem> {
        if let Some(full_tree) = &self.full_tree {
            full_tree
                .iter()
                .filter(|i| self.selected_paths.contains(&i.path))
                .cloned()
                .map(|mut i| {
                    i.selected = true;
                    i
                })
                .collect()
        } else {
            // Fallback for non-recursive mode
            self.items
                .iter()
                .filter(|i| self.selected_paths.contains(&i.path))
                .cloned()
                .map(|mut i| {
                    i.selected = true;
                    i
                })
                .collect()
        }
    }
}

pub async fn run_tui(
    initial_url: Option<String>,
    token: Option<String>,
    download_path: Option<String>,
    cwd: bool,
    no_folder: bool,
) -> Result<()> {
    install_panic_hook();
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let client = GitHubClient::new(token.clone())?;
    let mut state_init = AppState::new();
    state_init.github_token = token;
    state_init.download_path = download_path;
    state_init.cwd = cwd;
    state_init.no_folder = no_folder;

    let has_initial_url = initial_url.is_some();

    if let Some(url) = initial_url {
        state_init.url_input = url;
        state_init.mode = AppMode::Searching;
        state_init.status_message = "Parsing URL...".to_string();
    }

    let state = Arc::new(Mutex::new(state_init));

    if has_initial_url {
        let state_c = state.clone();
        let client_c = client.clone();

        tokio::spawn(async move {
            let url = {
                let s = state_c.lock().await;
                s.url_input.clone()
            };

            match GitHubUrl::parse(&url) {
                Ok(gh_url) => {
                    load_repo(state_c, client_c, gh_url).await;
                }
                Err(e) => {
                    let mut s = state_c.lock().await;
                    s.mode = AppMode::Input;
                    s.show_toast(format!("Invalid URL: {}", e), ToastType::Error);
                }
            }
        });
    }

    let result = event_loop(&mut terminal, state, client).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<Mutex<AppState>>,
    client: GitHubClient,
) -> Result<()> {
    loop {
        {
            let mut state_lock = state.lock().await;
            state_lock.frame_count = state_lock.frame_count.wrapping_add(1);
            let frame_count = state_lock.frame_count;

            if let Some(ref t) = state_lock.toast {
                if t.is_expired() {
                    state_lock.toast = None;
                }
            }

            terminal.draw(|f| {
                let size = f.size();
                f.render_widget(
                    ratatui::widgets::Block::default()
                        .style(ratatui::style::Style::default().bg(theme::BG_COLOR)),
                    size,
                );

                match state_lock.mode {
                    AppMode::Input => {
                        let cursor_visible = (frame_count / 5) % 2 == 0;
                        components::input::render(
                            f,
                            size,
                            &state_lock.url_input,
                            state_lock.url_cursor,
                            &state_lock.status_message,
                            cursor_visible,
                        );
                    }
                    AppMode::Searching => {
                        components::searching::render(
                            f,
                            size,
                            frame_count,
                            &state_lock.status_message,
                        );
                    }
                    AppMode::Browse => {
                        let filtered_items = state_lock.get_view_items();

                        let browser_state = components::browser::BrowserState {
                            items: &filtered_items,
                            current_url: state_lock.current_url.as_ref(),
                            cursor: state_lock.cursor,
                            scroll_offset: state_lock.scroll_offset,
                            status_msg: &state_lock.status_message,
                            is_downloading: state_lock.downloading,
                            ascii_mode: state_lock.ascii_mode,
                            folder_sizes: &state_lock.folder_sizes,
                            is_searching: state_lock.is_searching,
                            search_query: &state_lock.search_query,
                        };
                        components::browser::render(f, size, &browser_state);
                    }
                    AppMode::Preview => {
                        let s = &mut *state_lock;
                        let preview_state = components::preview::PreviewState {
                            content: &s.preview_content,
                            text: s.preview_text.clone(),
                            path: &s.preview_path,
                            loading: s.preview_loading,
                            is_image: s.preview_is_image,
                        };
                        components::preview::render(f, size, preview_state);
                    }
                }

                if let Some(ref toast) = state_lock.toast {
                    components::toast::render(f, size, toast);
                }
            })?;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press
                    && handle_input(key, state.clone(), &client).await?
                {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn handle_input(
    key: KeyEvent,
    state: Arc<Mutex<AppState>>,
    client: &GitHubClient,
) -> Result<bool> {
    let mut s = state.lock().await;

    if (key.code == KeyCode::Char('q') || key.code == KeyCode::Char('c'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return Ok(true);
    }

    match s.mode {
        AppMode::Input => match key.code {
            KeyCode::Char('w') | KeyCode::Char('u')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                s.url_input.clear();
                s.url_cursor = 0;
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {}
            KeyCode::Char(c)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
            {
                let byte_pos = s
                    .url_input
                    .char_indices()
                    .nth(s.url_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(s.url_input.len());
                s.url_input.insert(byte_pos, c);
                s.url_cursor += 1;
            }
            KeyCode::Backspace => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                    || key.modifiers.contains(KeyModifiers::SUPER)
                {
                    s.url_input.clear();
                    s.url_cursor = 0;
                } else if s.url_cursor > 0 {
                    let byte_pos = s
                        .url_input
                        .char_indices()
                        .nth(s.url_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap();
                    s.url_input.remove(byte_pos);
                    s.url_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                    || key.modifiers.contains(KeyModifiers::SUPER)
                    || !s.url_input.is_empty()
                // User said "just del" to remove full URL
                {
                    s.url_input.clear();
                    s.url_cursor = 0;
                }
            }
            KeyCode::Left => {
                if s.url_cursor > 0 {
                    s.url_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if s.url_cursor < s.url_input.chars().count() {
                    s.url_cursor += 1;
                }
            }
            KeyCode::Tab => {
                let target = "https://github.com/";
                if s.url_input.is_empty()
                    || (target.starts_with(&s.url_input) && s.url_input.len() < target.len())
                {
                    s.url_input = target.to_string();
                    s.url_cursor = s.url_input.chars().count();
                }
            }
            KeyCode::Esc => return Ok(true),
            KeyCode::Enter => {
                let url = s.url_input.clone();
                s.mode = AppMode::Searching;
                s.status_message = "Parsing URL...".to_string();

                let state_c = state.clone();
                let client_c = client.clone();

                tokio::spawn(async move {
                    match GitHubUrl::parse(&url) {
                        Ok(gh_url) => {
                            load_repo(state_c, client_c, gh_url).await;
                        }
                        Err(e) => {
                            let mut s = state_c.lock().await;
                            s.mode = AppMode::Input;
                            s.show_toast(format!("Invalid URL: {}", e), ToastType::Error);
                        }
                    }
                });
            }
            _ => {}
        },
        AppMode::Searching => {
            if key.code == KeyCode::Esc {
                s.mode = AppMode::Input;
                s.status_message = "Search cancelled".to_string();
            }
        }
        AppMode::Browse => {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
                KeyCode::Esc if !s.is_searching => {
                    s.mode = AppMode::Input;
                    return Ok(false);
                }
                KeyCode::Esc if s.is_searching => {
                    s.is_searching = false;
                    s.search_query.clear();
                    s.status_message = String::new();
                }
                KeyCode::Enter if s.is_searching => {
                    s.is_searching = false;
                    s.status_message = String::new();
                }
                KeyCode::Char('i') if !s.is_searching => {
                    s.ascii_mode = !s.ascii_mode;
                    let msg = if s.ascii_mode {
                        "ASCII Icons"
                    } else {
                        "Emoji Icons"
                    };
                    s.show_toast(msg.to_string(), ToastType::Info);
                }

                KeyCode::Up => s.move_up(),
                KeyCode::Down => {
                    let count = s.get_view_items().len();
                    s.move_down(count);
                }
                KeyCode::Char('k') if !s.is_searching => s.move_up(),
                KeyCode::Char('j') if !s.is_searching => {
                    let count = s.get_view_items().len();
                    s.move_down(count);
                }
                KeyCode::Home => s.move_top(),
                KeyCode::End => {
                    let count = s.get_view_items().len();
                    s.move_bottom(count);
                }
                KeyCode::Char('g') if !s.is_searching => s.move_top(),
                KeyCode::Char('G') if !s.is_searching => {
                    let count = s.get_view_items().len();
                    s.move_bottom(count);
                }

                // selections
                KeyCode::Char(' ') => {
                    let items = s.get_view_items();
                    if let Some(item) = items.get(s.cursor) {
                        if s.selected_paths.contains(&item.path) {
                            s.selected_paths.remove(&item.path);
                        } else {
                            s.selected_paths.insert(item.path.clone());
                        }
                    }
                }
                KeyCode::Char('a') if !s.is_searching => {
                    let items = s.get_view_items();
                    for item in items {
                        s.selected_paths.insert(item.path.clone());
                    }
                }
                KeyCode::Char('u') if !s.is_searching => {
                    let items = s.get_view_items();
                    for item in items {
                        s.selected_paths.remove(&item.path);
                    }
                }

                KeyCode::Char('/') => {
                    s.is_searching = !s.is_searching;
                    if !s.is_searching {
                        s.search_query.clear();
                    }
                }
                KeyCode::Char(c) if s.is_searching => {
                    s.search_query.push(c);
                    s.cursor = 0;
                    s.scroll_offset = 0;
                }
                KeyCode::Backspace if s.is_searching => {
                    s.search_query.pop();
                    s.cursor = 0;
                    s.scroll_offset = 0;
                }

                // go back up a level (only when not searching)
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') if !s.is_searching => {
                    if let Some((prev_url, prev_cursor)) = s.navigation_stack.pop() {
                        s.status_message = "Heading back...".to_string();
                        let current_url = prev_url.clone();
                        let cursor_pos = prev_cursor;

                        if let Some(full_tree) = &s.full_tree {
                            let next_items = if current_url.path.is_empty() {
                                full_tree
                                    .iter()
                                    .filter(|i| !i.path.contains('/'))
                                    .cloned()
                                    .collect()
                            } else {
                                let prefix = format!("{}/", current_url.path);
                                full_tree
                                    .iter()
                                    .filter(|i| {
                                        i.path.starts_with(&prefix)
                                            && !i.path[prefix.len()..].contains('/')
                                    })
                                    .cloned()
                                    .collect()
                            };

                            s.items = next_items;
                            s.sync_selections();
                            s.current_url = Some(current_url);
                            s.cursor = cursor_pos;
                            s.scroll_offset = 0;
                            s.status_message = String::new();
                        } else {
                            drop(s);
                            match client.fetch_contents(&prev_url.api_url()).await {
                                Ok(items) => {
                                    let mut s = state.lock().await;
                                    s.items = items;
                                    s.sync_selections();
                                    s.current_url = Some(prev_url);
                                    s.cursor = prev_cursor;
                                    s.scroll_offset = 0;
                                }
                                Err(e) => {
                                    let mut s = state.lock().await;
                                    s.show_toast(format!("Nav Error: {}", e), ToastType::Error);
                                }
                            };
                        }
                    } else {
                        s.mode = AppMode::Input;
                    }
                }

                KeyCode::Char('p') | KeyCode::Char('P') if !s.is_searching => {
                    let items = s.get_view_items();
                    if let Some(item) = items.get(s.cursor).cloned() {
                        if item.is_file() {
                            if let Some(download_url) = item.actual_download_url() {
                                s.mode = AppMode::Preview;
                                s.preview_path = item.path.clone();
                                s.preview_content = String::new();
                                s.preview_text = None;
                                s.preview_is_image = false;
                                s.preview_loading = true;

                                let url = download_url.clone();
                                let state_c = state.clone();
                                let client_c = client.clone();
                                let item_path = item.path.clone();

                                tokio::spawn(async move {
                                    if is_media_file(&item_path) || is_video_file(&item_path) {
                                        let mut s = state_c.lock().await;
                                        s.preview_is_image = true;
                                        s.preview_loading = false;
                                        s.preview_text = None;
                                    } else {
                                        match client_c.fetch_partial_content(&url, 16 * 1024).await
                                        {
                                            Ok(content) => {
                                                let highlighted =
                                                    highlight_content(&content, &item_path);
                                                let mut s = state_c.lock().await;
                                                s.preview_content = content;
                                                s.preview_text = Some(highlighted);
                                                s.preview_loading = false;
                                            }
                                            Err(e) => {
                                                let mut s = state_c.lock().await;
                                                s.preview_content =
                                                    format!("Error fetching preview: {}", e);
                                                s.preview_text = None;
                                                s.preview_loading = false;
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                }

                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') if !s.is_searching => {
                    let items = s.get_view_items();
                    if let Some(item) = items.get(s.cursor).cloned() {
                        if item.is_dir() {
                            let cursor_pos = s.cursor;
                            if let Some(current_url) = s.current_url.clone() {
                                s.navigation_stack.push((current_url.clone(), cursor_pos));

                                let new_path = if current_url.path.is_empty() {
                                    item.name.clone()
                                } else {
                                    format!("{}/{}", current_url.path, item.name)
                                };

                                let new_url = GitHubUrl {
                                    path: new_path.clone(),
                                    ..current_url
                                };

                                if let Some(full_tree) = &s.full_tree {
                                    let prefix = format!("{}/", new_path);
                                    let next_items = full_tree
                                        .iter()
                                        .filter(|i| {
                                            i.path.starts_with(&prefix)
                                                && !i.path[prefix.len()..].contains('/')
                                        })
                                        .cloned()
                                        .collect();

                                    s.items = next_items;
                                    s.sync_selections();
                                    s.current_url = Some(new_url);
                                    s.cursor = 0;
                                    s.scroll_offset = 0;
                                } else {
                                    drop(s);
                                    match client.fetch_contents(&new_url.api_url()).await {
                                        Ok(items) => {
                                            let mut s = state.lock().await;
                                            s.items = items;
                                            s.sync_selections();
                                            s.current_url = Some(new_url);
                                            s.cursor = 0;
                                            s.scroll_offset = 0;
                                        }
                                        Err(e) => {
                                            let mut s = state.lock().await;
                                            s.navigation_stack.pop();
                                            s.show_toast(
                                                format!("Nav Error: {}", e),
                                                ToastType::Error,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') if !s.is_searching => {
                    if s.get_selected_items().is_empty() {
                        s.show_toast("No items selected!".to_string(), ToastType::Info);
                    } else {
                        s.downloading = true;
                        drop(s);

                        let s_clone = state.clone();
                        tokio::spawn(
                            async move { if let Err(_e) = perform_download(s_clone).await {} },
                        );
                    }
                }
                _ => {}
            }
        }
        AppMode::Preview => match key.code {
            KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Backspace
            | KeyCode::Left
            | KeyCode::Char('h') => {
                s.mode = AppMode::Browse;
            }
            _ => {}
        },
    }

    Ok(false)
}

async fn load_repo(state: Arc<Mutex<AppState>>, client: GitHubClient, mut gh_url: GitHubUrl) {
    let state_c = state.clone();
    let mut current_client = client;

    {
        let mut s = state_c.lock().await;
        s.status_message = "Fetching repository tree...".to_string();
        s.mode = AppMode::Searching;
    }

    let mut tree_result = current_client
        .fetch_recursive_tree(&gh_url.owner, &gh_url.repo, &gh_url.branch)
        .await;

    if let Err(GitHubError::InvalidToken) = &tree_result {
        {
            let mut s = state_c.lock().await;
            s.show_toast(
                "Invalid token! Falling back to public API.".to_string(),
                ToastType::Warning,
            );
        }
        if let Ok(no_auth_client) = GitHubClient::new(None) {
            current_client = no_auth_client;
            tree_result = current_client
                .fetch_recursive_tree(&gh_url.owner, &gh_url.repo, &gh_url.branch)
                .await;
        }
    }

    if let Err(GitHubError::NotFound(_)) = &tree_result {
        if gh_url.branch == "main" {
            gh_url.branch = "master".to_string();
            {
                let mut s = state_c.lock().await;
                s.status_message = "Trying master branch...".to_string();
            }
            tree_result = current_client
                .fetch_recursive_tree(&gh_url.owner, &gh_url.repo, &gh_url.branch)
                .await;
        }
    }

    match tree_result {
        Ok(tree_response) => {
            let items =
                map_tree_to_items(tree_response, &gh_url.owner, &gh_url.repo, &gh_url.branch);
            let folder_sizes = calculate_folder_sizes(&items);

            let mut s = state_c.lock().await;
            s.full_tree = Some(items.clone());
            s.folder_sizes = folder_sizes;

            let current_path = gh_url.path.clone();
            let mut current_items: Vec<RepoItem> = if current_path.is_empty() {
                items
                    .iter()
                    .filter(|i| !i.path.contains('/'))
                    .cloned()
                    .collect()
            } else {
                let prefix = format!("{}/", current_path);
                items
                    .iter()
                    .filter(|i| {
                        i.path.starts_with(&prefix) && !i.path[prefix.len()..].contains('/')
                    })
                    .cloned()
                    .collect()
            };

            drop(s);
            current_client
                .resolve_lfs_files(
                    &mut current_items,
                    &gh_url.owner,
                    &gh_url.repo,
                    &gh_url.branch,
                )
                .await;

            let mut s = state_c.lock().await;
            s.items = current_items;
            s.current_url = Some(gh_url);
            s.mode = AppMode::Browse;
            s.status_message = String::new();
            s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
        }
        Err(_) => {
            {
                let mut s = state_c.lock().await;
                s.status_message =
                    "Tree too large, falling back to folder-by-folder...".to_string();
                s.full_tree = None;
            }

            let result = current_client.fetch_contents(&gh_url.api_url()).await;

            if let Err(e) = result {
                let mut s = state_c.lock().await;
                s.mode = AppMode::Input;
                let err_msg = if let Some(gh_err) = e.downcast_ref::<GitHubError>() {
                    match gh_err {
                        GitHubError::RateLimitReached(user) => format!(
                            "Rate limit reached for {}. Add a token in config for more!",
                            user
                        ),
                        GitHubError::NotFound(_) => "Repository or path not found.".to_string(),
                        _ => format!("Error: {}", gh_err),
                    }
                } else {
                    format!("Error: {}", e)
                };

                s.show_toast(err_msg, ToastType::Error);
            } else if let Ok(mut items) = result {
                current_client
                    .resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo, &gh_url.branch)
                    .await;

                let mut s = state_c.lock().await;
                s.items = items;
                s.current_url = Some(gh_url);
                s.mode = AppMode::Browse;
                s.status_message = String::new();
                s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
            }
        }
    }
}

async fn perform_download(state: Arc<Mutex<AppState>>) -> Result<()> {
    use crate::download::Downloader;
    let (items_to_download, _repo_path, repo_name, token, custom_path, cwd, no_folder) = {
        let s = state.lock().await;
        if let Some(url) = &s.current_url {
            let selected = s.get_selected_items();
            let mut final_items = Vec::new();

            if let Some(full_tree) = &s.full_tree {
                for top_item in selected {
                    if top_item.is_dir() {
                        let prefix = format!("{}/", top_item.path);
                        let prefix_len = if let Some(slash_pos) = top_item.path.rfind('/') {
                            slash_pos + 1
                        } else {
                            0
                        };

                        for tree_item in full_tree {
                            if tree_item.path.starts_with(&prefix) && tree_item.is_file() {
                                let mut file_item = tree_item.clone();
                                // Preserve relative path from the selection's parent
                                file_item.name = tree_item.path[prefix_len..].to_string();
                                file_item.selected = true;
                                final_items.push(file_item);
                            }
                        }
                    } else {
                        final_items.push(top_item);
                    }
                }
            } else {
                final_items = selected;
            }

            (
                final_items,
                format!("{}/{}", url.owner, url.repo),
                url.repo.clone(),
                s.github_token.clone(),
                s.download_path.clone(),
                s.cwd,
                s.no_folder,
            )
        } else {
            return Ok(());
        }
    };

    let download_dir = if cwd {
        std::env::current_dir().context("Could not get current working directory")?
    } else if let Some(path) = custom_path {
        std::path::PathBuf::from(path)
    } else {
        dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .context("Could not find User Downloads directory")?
    };

    let download_dir = if no_folder {
        download_dir
    } else {
        download_dir.join(repo_name)
    };

    let downloader = Downloader::new(download_dir.clone(), token)?;
    let state_c = state.clone();

    let result = downloader
        .download_items(&items_to_download, &_repo_path, move |msg| {
            let s = state_c.clone();
            tokio::spawn(async move {
                let mut s = s.lock().await;
                s.status_message = msg;
            });
        })
        .await;

    let mut s = state.lock().await;
    s.downloading = false;

    match result {
        Ok(errors) => {
            if errors.is_empty() {
                s.status_message = "".to_string();
                s.show_toast(
                    format!("Downloaded to: {}", download_dir.display()),
                    ToastType::Success,
                );
            } else {
                s.status_message = "".to_string();
                s.show_toast(
                    format!("Completed with {} errors", errors.len()),
                    ToastType::Error,
                );
            }
        }
        Err(e) => {
            s.status_message = "".to_string();
            s.show_toast(format!("Download failed: {}", e), ToastType::Error);
        }
    }

    Ok(())
}

fn calculate_folder_sizes(items: &[RepoItem]) -> HashMap<String, u64> {
    let mut sizes = HashMap::new();
    for item in items {
        if item.is_file() {
            let path = &item.path;
            let parts: Vec<&str> = path.split('/').collect();
            for i in 1..parts.len() {
                let parent_path = parts[..i].join("/");
                if !parent_path.is_empty() {
                    let entry = sizes.entry(parent_path).or_insert(0);
                    *entry += item.actual_size().unwrap_or(0);
                }
            }
        }
    }
    sizes
}

fn map_tree_to_items(
    tree: crate::github::GitTreeResponse,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Vec<RepoItem> {
    tree.tree
        .into_iter()
        .map(|entry| {
            let name = entry
                .path
                .split('/')
                .next_back()
                .unwrap_or(&entry.path)
                .to_string();
            let item_type = if entry.entry_type == "tree" {
                "dir".to_string()
            } else {
                "file".to_string()
            };

            let download_url = if item_type == "file" {
                Some(format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    owner, repo, branch, entry.path
                ))
            } else {
                None
            };

            RepoItem {
                name,
                item_type,
                url: format!(
                    "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
                    owner, repo, &entry.path, branch
                ),
                path: entry.path,
                download_url,
                size: entry.size,
                selected: false,
                lfs_oid: None,
                lfs_size: None,
                lfs_download_url: None,
            }
        })
        .collect()
}

fn is_media_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".bmp")
        || lower.ends_with(".webp")
}

fn is_video_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".mp4")
        || lower.ends_with(".mkv")
        || lower.ends_with(".avi")
        || lower.ends_with(".webm")
}
