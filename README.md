# ghgrab - "grab anything you want"

> A simple, pretty terminal tool that lets you search and download files from GitHub without leaving your CLI.

![Rust](https://img.shields.io/badge/rust-1.70%20%7C%201.75%20%7C%20stable-blue) ![crates.io](https://img.shields.io/crates/v/ghgrab.svg?color=blue) ![npm version](https://img.shields.io/npm/v/@ghgrab/ghgrab.svg?color=blue) ![PyPI version](https://img.shields.io/pypi/v/ghgrab.svg?color=blue) ![license](https://img.shields.io/badge/license-MIT-blue)

![ghgrab demo](assets/ghgrab.gif)

**ghgrab** provides a streamlined command-line interface for cherry-picking specific files or folders from any GitHub repository, powered by the Rust `tokio` and `ratatui` ecosystem. Focused on speed and ease of use, it offers a beautiful TUI that lets you grab exactly what you need; all without the wait times of a full `git clone`.

## Why use ghgrab?

- **No more clone-and-delete**: Grab exactly what you need, when you need it.
- **Easy on the eyes**: A clean terminal interface that makes browsing feel smooth.
- **Works where you are**: Installs quickly via NPM, Cargo, or PIP.
- **Find things fast**: Quickly search and navigate through any repo's folders with fuzzy search.
- **File Preview**: Preview source code and text files directly in the TUI.
- **Handles the big stuff**: Built-in support for GitHub LFS (Large File Storage).
- **Batch mode**: Select a bunch of files and folders to download them all at once.

---

## Installation

### NPM
```bash
npm install -g @ghgrab/ghgrab
```

### Cargo
```bash
cargo install ghgrab
```

### pipx (Recommended for Python)
```bash
pipx install ghgrab
```

### Nix 
To have the latest commit:
``` bash
nix run github:abhixdd/ghgrab
```

To have a specific tagged version:
``` bash
nix run "github:abhixdd/ghgrab/<tag>"
```

---

### Quick Start

Just type `ghgrab` to start browsing:

```bash
ghgrab
```

Or, if you already have a link, just paste it in:

```bash
# Browse a repository
ghgrab https://github.com/rust-lang/rust

# Download to current directory directly
ghgrab https://github.com/rust-lang/rust --cwd --no-folder
```

### CLI Flags

| Flag | Description |
|------|-------------|
| `--cwd` | Forces download to the current working directory. |
| `--no-folder` | Downloads files directly without creating a subfolder for the repo. |
| `--token <TOKEN>`| Use a specific GitHub token for this run (doesn't save to settings). |

### Agent Mode

For scripts, agents, and other non-interactive workflows, `ghgrab` includes a machine-friendly `agent` command that prints a stable JSON envelope with `api_version`, `ok`, `command`, and either `data` or `error`.

```bash
# Fetch the repository tree as JSON
ghgrab agent tree https://github.com/rust-lang/rust

# Fetch the repository tree with an explicit token for scripts or agents
ghgrab agent tree https://github.com/rust-lang/rust --token YOUR_TOKEN

# Download specific paths from a repository
ghgrab agent download https://github.com/rust-lang/rust src/tools README.md --out ./tmp

# Download an explicit subtree
ghgrab agent download https://github.com/rust-lang/rust --subtree src/tools --out ./tmp

# Download the entire repository
ghgrab agent download https://github.com/rust-lang/rust --repo --out ./tmp

# Download into the current working directory without creating a repo folder
ghgrab agent download https://github.com/rust-lang/rust src/tools --cwd --no-folder
```

You can pass `--token <TOKEN>` to `agent tree` and `agent download` when an external tool, CI job, or coding agent should authenticate without relying on saved local config.

### Configuration

To manage your settings:

```bash
# Set your token
ghgrab config set token YOUR_TOKEN

# Set a custom download folder
ghgrab config set path "/your/custom/path"

# View your current settings (token is masked)
ghgrab config list

# Remove settings
ghgrab config unset token
ghgrab config unset path
```

### Keyboard Shortcuts (How to move around)

We've kept it pretty standard, but here's a quick cheat sheet:

| Key | Action |
|-----|--------|
| `Enter` / `l` / `Right` | Enter directory or Submit URL |
| `Backspace` / `h` / `Left` | Go back to previous folder |
| `Delete` | Clear URL input (Home page) |
| `Tab` | Auto-fill `https://github.com/` (Home page) |
| `/` | Start Searching (File list) |
| `Esc` | **Exit Search** or **Return Home** (file list) or **Quit** (home screen) |
| `q` / `Q` | **Quit** (from file list) |
| `Ctrl+q` | **Force Quit** (anywhere) |
| `Space` | Toggle selection for the current item |
| `p` / `P` | **Preview** current file |
| `a` | Select All items |
| `u` | Unselect all items |
| `d` / `D` | Download selected items |
| `i` | Toggle Icons (Emoji / ASCII) |
| `g` / `Home` | Jump to Top |
| `G` / `End` | Jump to Bottom |

---



## Join the community

If you find a bug, have an idea for a cool new feature, or just want to help out, we'd love to hear from you! Check out our [Contributing Guide](CONTRIBUTING.md) to see how you can get involved.

## License

Distributed under the MIT License. It's open, free, and yours to play with. See [LICENSE](LICENSE) for the fine print.
