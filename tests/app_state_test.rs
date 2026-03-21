use ghgrab::github::RepoItem;
use ghgrab::ui::AppState;

fn make_items(count: usize) -> Vec<RepoItem> {
    (0..count)
        .map(|i| RepoItem {
            name: format!("item_{}", i),
            item_type: if i % 3 == 0 {
                "dir".to_string()
            } else {
                "file".to_string()
            },
            path: format!("path/item_{}", i),
            download_url: Some(format!("https://example.com/item_{}", i)),
            url: format!("https://api.github.com/repos/o/r/contents/item_{}", i),
            size: Some((i as u64 + 1) * 100),
            selected: false,
            lfs_oid: None,
            lfs_size: None,
            lfs_download_url: None,
        })
        .collect()
}

#[test]
fn test_move_down() {
    let mut state = AppState::new();
    state.items = make_items(5);
    assert_eq!(state.cursor, 0);

    let count = state.items.len();
    state.move_down(count);
    assert_eq!(state.cursor, 1);

    state.move_down(count);
    assert_eq!(state.cursor, 2);
}

#[test]
fn test_searching_mode() {
    use ghgrab::ui::AppMode;
    let mut state = AppState::new();
    assert_eq!(state.mode, AppMode::Input);

    state.mode = AppMode::Searching;
    assert_eq!(state.mode, AppMode::Searching);

    assert_eq!(state.frame_count, 0);
    state.frame_count += 1;
    assert_eq!(state.frame_count, 1);
}

#[test]
fn test_move_up() {
    let mut state = AppState::new();
    state.items = make_items(5);
    state.cursor = 3;

    state.move_up();
    assert_eq!(state.cursor, 2);

    state.move_up();
    assert_eq!(state.cursor, 1);
}

#[test]
fn test_move_up_at_top() {
    let mut state = AppState::new();
    state.items = make_items(5);
    assert_eq!(state.cursor, 0);

    state.move_up();
    assert_eq!(state.cursor, 0);
}

#[test]
fn test_move_down_at_bottom() {
    let mut state = AppState::new();
    state.items = make_items(3);
    state.cursor = 2;

    let count = state.items.len();
    state.move_down(count);
    assert_eq!(state.cursor, 2);
}

#[test]
fn test_move_top() {
    let mut state = AppState::new();
    state.items = make_items(10);
    state.cursor = 7;

    state.move_top();
    assert_eq!(state.cursor, 0);
}

#[test]
fn test_move_bottom() {
    let mut state = AppState::new();
    state.items = make_items(10);

    let count = state.items.len();
    state.move_bottom(count);
    assert_eq!(state.cursor, 9);
}

#[test]
fn test_move_bottom_empty() {
    let mut state = AppState::new();
    state.move_bottom(0);
    assert_eq!(state.cursor, 0);
}

#[test]
fn test_toggle_selection() {
    let mut state = AppState::new();
    state.items = make_items(3);

    assert!(!state.items[0].selected);
    state.toggle_selection();
    assert!(state.items[0].selected);
    state.toggle_selection();
    assert!(!state.items[0].selected);
}

#[test]
fn test_select_all() {
    let mut state = AppState::new();
    state.items = make_items(5);

    state.loop_selection(true);
    for item in &state.items {
        assert!(item.selected);
    }
}

#[test]
fn test_deselect_all() {
    let mut state = AppState::new();
    state.items = make_items(5);
    state.loop_selection(true);
    state.loop_selection(false);

    for item in &state.items {
        assert!(!item.selected);
    }
}

#[test]
fn test_get_selected_items() {
    let mut state = AppState::new();
    state.items = make_items(5);

    state.cursor = 1;
    state.toggle_selection();
    state.cursor = 3;
    state.toggle_selection();

    let selected = state.get_selected_items();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].name, "item_1");
    assert_eq!(selected[1].name, "item_3");
}

#[test]
fn test_fresh_startup_state() {
    let state = AppState::new();
    assert_eq!(state.url_input, "");
    assert_eq!(state.url_cursor, 0);
}

#[test]
fn test_tab_completion_logic() {
    let mut state = AppState::new();
    let target = "https://github.com/";

    state.url_input = "".to_string();
    if state.url_input.is_empty()
        || (target.starts_with(&state.url_input) && state.url_input.len() < target.len())
    {
        state.url_input = target.to_string();
    }
    assert_eq!(state.url_input, target);

    state.url_input = "h".to_string();
    if state.url_input.is_empty()
        || (target.starts_with(&state.url_input) && state.url_input.len() < target.len())
    {
        state.url_input = target.to_string();
    }
    assert_eq!(state.url_input, target);

    state.url_input = "https://gi".to_string();
    if state.url_input.is_empty()
        || (target.starts_with(&state.url_input) && state.url_input.len() < target.len())
    {
        state.url_input = target.to_string();
    }
    assert_eq!(state.url_input, target);

    state.url_input = "google.com".to_string();
    if state.url_input.is_empty()
        || (target.starts_with(&state.url_input) && state.url_input.len() < target.len())
    {
        state.url_input = target.to_string();
    }
    assert_eq!(state.url_input, "google.com");
}

#[test]
fn test_clear_logic() {
    let mut state = AppState::new();
    state.url_input = "https://github.com/user/repo".to_string();
    state.url_cursor = 10;

    state.url_input.clear();
    state.url_cursor = 0;

    assert!(state.url_input.is_empty());
    assert_eq!(state.url_cursor, 0);
}

#[test]
fn test_unicode_insert() {
    let mut state = AppState::new();

    let c = 'ñ';
    let byte_pos = state
        .url_input
        .char_indices()
        .nth(state.url_cursor)
        .map(|(i, _)| i)
        .unwrap_or(state.url_input.len());
    state.url_input.insert(byte_pos, c);
    state.url_cursor += 1;

    assert_eq!(state.url_input, "ñ");
    assert_eq!(state.url_cursor, 1);
    assert_eq!(state.url_input.len(), 2);

    let c = '中';
    let byte_pos = state
        .url_input
        .char_indices()
        .nth(state.url_cursor)
        .map(|(i, _)| i)
        .unwrap_or(state.url_input.len());
    state.url_input.insert(byte_pos, c);
    state.url_cursor += 1;

    assert_eq!(state.url_input, "ñ中");
    assert_eq!(state.url_cursor, 2);
    assert_eq!(state.url_input.len(), 5);

    let c = 'a';
    let byte_pos = state
        .url_input
        .char_indices()
        .nth(state.url_cursor)
        .map(|(i, _)| i)
        .unwrap_or(state.url_input.len());
    state.url_input.insert(byte_pos, c);
    state.url_cursor += 1;

    assert_eq!(state.url_input, "ñ中a");
    assert_eq!(state.url_cursor, 3);
}

#[test]
fn test_unicode_backspace() {
    let mut state = AppState::new();
    state.url_input = "héllo".to_string();
    state.url_cursor = 2;

    let byte_pos = state
        .url_input
        .char_indices()
        .nth(state.url_cursor - 1)
        .map(|(i, _)| i)
        .unwrap();
    state.url_input.remove(byte_pos);
    state.url_cursor -= 1;

    assert_eq!(state.url_input, "hllo");
    assert_eq!(state.url_cursor, 1);
}

#[test]
fn test_unicode_cursor_movement() {
    let mut state = AppState::new();
    state.url_input = "café".to_string();
    state.url_cursor = 0;

    let char_count = state.url_input.chars().count();
    assert_eq!(char_count, 4);

    for expected in 1..=4 {
        if state.url_cursor < state.url_input.chars().count() {
            state.url_cursor += 1;
        }
        assert_eq!(state.url_cursor, expected);
    }

    if state.url_cursor < state.url_input.chars().count() {
        state.url_cursor += 1;
    }
    assert_eq!(state.url_cursor, 4);

    for expected in (0..4).rev() {
        if state.url_cursor > 0 {
            state.url_cursor -= 1;
        }
        assert_eq!(state.url_cursor, expected);
    }
}

#[test]
fn test_unicode_insert_in_middle() {
    let mut state = AppState::new();
    state.url_input = "ab".to_string();
    state.url_cursor = 1;

    let c = 'ñ';
    let byte_pos = state
        .url_input
        .char_indices()
        .nth(state.url_cursor)
        .map(|(i, _)| i)
        .unwrap_or(state.url_input.len());
    state.url_input.insert(byte_pos, c);
    state.url_cursor += 1;

    assert_eq!(state.url_input, "añb");
    assert_eq!(state.url_cursor, 2);
}

#[test]
fn test_unicode_cursor_render_logic() {
    let input_text = "café";
    let url_cursor: usize = 3;

    let mut s = input_text.to_string();
    let char_count = s.chars().count();
    assert_eq!(char_count, 4);

    if url_cursor >= char_count {
        s.push('_');
    } else {
        let start = s.char_indices().nth(url_cursor).map(|(i, _)| i).unwrap();
        let end = s
            .char_indices()
            .nth(url_cursor + 1)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        s.replace_range(start..end, "_");
    }

    assert_eq!(s, "caf_");
}
