use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::ui::components::toast::{Toast, ToastType};

use crate::github::{GitHubClient, GitHubUrl, RepoItem};

pub mod components;
pub mod theme;

#[derive(PartialEq)]
pub enum AppMode {
    Input,
    Browse,
}

pub struct AppState {
    pub mode: AppMode,
    pub url_input: String,
    pub current_url: Option<GitHubUrl>,
    pub items: Vec<RepoItem>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub status_message: String,
    pub downloading: bool,
    pub navigation_stack: Vec<(GitHubUrl, usize)>,
    pub frame_count: u64,
    pub toast: Option<Toast>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            mode: AppMode::Input,
            url_input: String::new(),
            current_url: None,
            items: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            status_message: "".to_string(),
            downloading: false,
            navigation_stack: Vec::new(),
            frame_count: 0,
            toast: None,
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

    pub fn move_down(&mut self) {
        if self.cursor < self.items.len().saturating_sub(1) {
            self.cursor += 1;
        }
        self.adjust_scroll();
    }
    
    pub fn move_top(&mut self) {
        self.cursor = 0;
        self.adjust_scroll();
    }

    pub fn move_bottom(&mut self) {
        if !self.items.is_empty() {
            self.cursor = self.items.len() - 1;
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
        }
    }

    pub fn toggle_selection(&mut self) {
        if let Some(item) = self.items.get_mut(self.cursor) {
            item.selected = !item.selected;
        }
    }

    pub fn get_selected_items(&self) -> Vec<RepoItem> {
        self.items.iter().filter(|i| i.selected).cloned().collect()
    }
}

pub async fn run_tui(initial_url: Option<String>) -> Result<()> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let mut state_init = AppState::new();
    if let Some(url) = initial_url {
        state_init.url_input = url;
    }
    let state = Arc::new(Mutex::new(state_init));
    
    let result = if !state.lock().await.url_input.is_empty() {
        let client = GitHubClient::new()?;
        let url_to_load = state.lock().await.url_input.clone();
        
        match GitHubUrl::parse(&url_to_load) {
            Ok(mut gh_url) => {
                match client.fetch_contents(&gh_url.api_url()).await {
                    Ok(mut items) => {
                        client.resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo).await;
                        
                        let mut s = state.lock().await;
                        s.items = items;
                        s.current_url = Some(gh_url);
                        s.mode = AppMode::Browse;
                        s.status_message = String::new();
                        s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
                    }
                    Err(_) => {
                        if gh_url.branch == "main" {
                            gh_url.branch = "master".to_string();
                            if let Ok(mut items) = client.fetch_contents(&gh_url.api_url()).await {
                                client.resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo).await;
                                
                                let mut s = state.lock().await;
                                s.items = items;
                                s.current_url = Some(gh_url);
                                s.mode = AppMode::Browse;
                                s.status_message = String::new();
                                s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
        event_loop(&mut terminal, state).await
    } else {
        event_loop(&mut terminal, state).await
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<Mutex<AppState>>,
) -> Result<()> {
    let client = GitHubClient::new()?;

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
                f.render_widget(ratatui::widgets::Block::default().style(ratatui::style::Style::default().bg(theme::BG_COLOR)), size);
                
                match state_lock.mode {
                    AppMode::Input => {
                        let cursor_visible = (frame_count / 5) % 2 == 0;
                        components::input::render(f, size, &state_lock.url_input, &state_lock.status_message, cursor_visible);
                    }
                    AppMode::Browse => {
                        let browser_state = components::browser::BrowserState {
                            items: &state_lock.items,
                            current_url: state_lock.current_url.as_ref(),
                            cursor: state_lock.cursor,
                            scroll_offset: state_lock.scroll_offset,
                            status_msg: &state_lock.status_message,
                            is_downloading: state_lock.downloading,
                        };
                        components::browser::render(f, size, &browser_state);
                    }
                }

                if let Some(ref toast) = state_lock.toast {
                    components::toast::render(f, size, toast);
                }
            })?;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if handle_input(key, state.clone(), &client).await? {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_input(key: KeyEvent, state: Arc<Mutex<AppState>>, client: &GitHubClient) -> Result<bool> {
    let mut s = state.lock().await;

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    match s.mode {
        AppMode::Input => {
            match key.code {
                KeyCode::Char(c) => s.url_input.push(c),
                KeyCode::Backspace => { s.url_input.pop(); },
                KeyCode::Esc => return Ok(true),
                KeyCode::Enter => {
                    let url = s.url_input.clone();
                    s.status_message = "Fetching...".to_string();
                    drop(s);
                    
                    match GitHubUrl::parse(&url) {
                        Ok(mut gh_url) => {
                            // Try fetching with the parsed branch
                            match client.fetch_contents(&gh_url.api_url()).await {
                                Ok(mut items) => {
                                    // Resolve LFS files
                                    {
                                        let mut s = state.lock().await;
                                        s.status_message = "Resolving LFS files...".to_string();
                                    }
                                    client.resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo).await;
                                    
                                    let mut s = state.lock().await;
                                    s.items = items;
                                    s.current_url = Some(gh_url);
                                    s.mode = AppMode::Browse;
                                    s.status_message = String::new();
                                    s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
                                }
                                Err(e) => {
                                    // If branch was defaulted to "main" and failed, try "master"
                                    let err_str = format!("{}", e);
                                    if gh_url.branch == "main" && err_str.contains("not found") {
                                        gh_url.branch = "master".to_string();
                                        match client.fetch_contents(&gh_url.api_url()).await {
                                            Ok(mut items) => {
                                                // Resolve LFS files
                                                {
                                                    let mut s = state.lock().await;
                                                    s.status_message = "Resolving LFS files...".to_string();
                                                }
                                                client.resolve_lfs_files(&mut items, &gh_url.owner, &gh_url.repo).await;
                                                
                                                let mut s = state.lock().await;
                                                s.items = items;
                                                s.current_url = Some(gh_url);
                                                s.mode = AppMode::Browse;
                                                s.status_message = String::new();
                                                s.show_toast("Repository Loaded!".to_string(), ToastType::Success);
                                            }
                                            Err(e2) => {
                                                let mut s = state.lock().await;
                                                s.show_toast(format!("Error: {}", e2), ToastType::Error);
                                            }
                                        }
                                    } else {
                                        let mut s = state.lock().await;
                                        s.show_toast(format!("Error: {}", e), ToastType::Error);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let mut s = state.lock().await;
                            s.show_toast(format!("Invalid URL: {}", e), ToastType::Error);
                        }
                    }
                }
                _ => {}
            }
        }
        AppMode::Browse => {
             match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
                
                // moving around
                KeyCode::Up => s.move_up(),
                KeyCode::Down => s.move_down(),
                KeyCode::Home | KeyCode::Char('g') => s.move_top(),
                KeyCode::End | KeyCode::Char('G') => s.move_bottom(),
                
                // selections
                KeyCode::Char(' ') => s.toggle_selection(),
                KeyCode::Char('a') => s.loop_selection(true),
                KeyCode::Char('u') => s.loop_selection(false),
                
                // go back up a level
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                    if let Some((prev_url, prev_cursor)) = s.navigation_stack.pop() {
                        s.status_message = "Heading back...".to_string();
                        drop(s);
                        
                        match client.fetch_contents(&prev_url.api_url()).await {
                            Ok(items) => {
                                let mut s = state.lock().await;
                                s.items = items;
                                s.current_url = Some(prev_url);
                                s.cursor = prev_cursor;
                                s.scroll_offset = 0; 
                            }
                            Err(e) => {
                                let mut s = state.lock().await;
                                s.show_toast(format!("Nav Error: {}", e), ToastType::Error);
                            }
                        };
                    } else {
                        s.mode = AppMode::Input;
                    }
                },
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(item) = s.items.get(s.cursor).cloned() {
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
                                    path: new_path,
                                    ..current_url
                                };
                                
                                drop(s);
                                
                                match client.fetch_contents(&new_url.api_url()).await {
                                    Ok(items) => {
                                        let mut s = state.lock().await;
                                        s.items = items;
                                        s.current_url = Some(new_url);
                                        s.cursor = 0;
                                        s.scroll_offset = 0;
                                    }
                                    Err(e) => {
                                        let mut s = state.lock().await;
                                        s.navigation_stack.pop();
                                        s.show_toast(format!("Nav Error: {}", e), ToastType::Error);
                                    }
                                }
                             }
                        }
                    }
                },
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if s.get_selected_items().is_empty() {
                        s.show_toast("No items selected!".to_string(), ToastType::Info);
                    } else {
                        s.downloading = true;
                        drop(s);
                        
                        let s_clone = state.clone();
                        tokio::spawn(async move {
                            if let Err(_e) = perform_download(s_clone).await {
                            }
                        });
                    }
                }
                _ => {}
            }
        }
    }
    
    Ok(false)
}

async fn perform_download(state: Arc<Mutex<AppState>>) -> Result<()> {
    use crate::download::Downloader;
    let (selected_items, _repo_path, repo_name) = {
        let s = state.lock().await;
        if let Some(url) = &s.current_url {
            (s.get_selected_items(), format!("{}/{}", url.owner, url.repo), url.repo.clone())
        } else {
            return Ok(());
        }
    };
    
    let download_dir = dirs::download_dir()
        .context("Could not find User Downloads directory")?
        .join(repo_name);

    let downloader = Downloader::new(download_dir)?;
    let state_c = state.clone();
    
    let result = downloader.download_items(&selected_items, &_repo_path, move |msg| {
        let s = state_c.clone();
        tokio::spawn(async move {
            let mut s = s.lock().await;
            s.status_message = msg;
        });
    }).await;
    
    let mut s = state.lock().await;
    s.downloading = false;
    
    match result {
        Ok(errors) => {
            if errors.is_empty() {
                s.status_message = "".to_string();
                s.show_toast("Download Complete!".to_string(), ToastType::Success);
            } else {
                s.status_message = "".to_string();
                s.show_toast(format!("Completed with {} errors", errors.len()), ToastType::Error);
            }
        }
        Err(e) => {
             s.status_message = "".to_string();
             s.show_toast(format!("Download failed: {}", e), ToastType::Error);
        }
    }
    
    Ok(())
}

