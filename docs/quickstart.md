# Quick Start

## Launch the TUI

Start the interactive browser:

```bash
ghgrab
```

From the home screen you can:

- paste a GitHub repository URL,
- type a repository keyword and press `Enter` to search GitHub,
- jump straight into browsing and downloading.

## Open a repository directly

```bash
ghgrab https://github.com/rust-lang/rust
```

## Download into the current directory

```bash
ghgrab https://github.com/rust-lang/rust --cwd --no-folder
```

`--cwd` writes into the current working directory, and `--no-folder` skips creating a repository-named subfolder.

## Download a release asset

```bash
ghgrab rel sharkdp/bat
```

This selects the best matching asset for the current OS and architecture when there is a clear winner.

## Keyboard shortcuts

### General browser navigation

| Key | Action |
| --- | --- |
| `Enter` on home | Open URL or start repository search |
| `Enter` / `l` / `Right` | Enter directory |
| `Backspace` / `h` / `Left` | Go to the previous folder |
| `/` | Start searching within the current file list |
| `Space` | Toggle selection |
| `d` / `D` | Download selected items |
| `p` / `P` | Preview the current file |
| `a` | Select all items |
| `u` | Clear all selections |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `Esc` | Exit search, return home, or quit depending on context |
| `q` / `Q` | Quit from the browser |
| `Ctrl+q` | Force quit |

### Home input helpers

| Key | Action |
| --- | --- |
| `Delete` | Delete character at cursor |
| `Tab` | Auto-fill `https://github.com/` |

### Repository search mode

| Key | Action |
| --- | --- |
| `j` / `k` / `Up` / `Down` | Move selection |
| `Enter` | Open the selected repository |
| `f` | Toggle include or exclude forks |
| `m` | Cycle minimum star filters |
| `l` | Cycle language filter |
| `s` | Cycle sort mode |
| `x` | Reset filters |
| `r` | Refresh the current search |
| `Esc` | Return to the home input |
