# ghgrab - "grab anything you want"

> A simple, pretty terminal tool that lets you browse and download files from GitHub, GitLab, Codeberg, Gitea, and Forgejo without leaving your CLI.

![Rust](https://img.shields.io/badge/rust-1.70%20%7C%201.75%20%7C%20stable-blue) ![crates.io](https://img.shields.io/crates/v/ghgrab.svg?color=blue) ![npm version](https://img.shields.io/npm/v/@ghgrab/ghgrab.svg?color=blue) ![PyPI version](https://img.shields.io/pypi/v/ghgrab.svg?color=blue) ![license](https://img.shields.io/badge/license-MIT-blue)

![ghgrab demo](assets/ghgrab.gif)

**ghgrab** provides a streamlined command-line interface for cherry-picking specific files or folders from supported Git forges, powered by the Rust `tokio` and `ratatui` ecosystem. Focused on speed and ease of use, it offers a beautiful TUI that lets you grab exactly what you need without the wait times of a full `git clone`.

## Why use ghgrab?

- **No more clone-and-delete**: Grab exactly what you need, when you need it.
- **Easy on the eyes**: A clean terminal interface that makes browsing feel smooth.
- **Works where you are**: Installs quickly via NPM, Cargo, or PIP.
- **Find things fast**: Quickly search and navigate through any repo's folders with fuzzy search.
- **Repo discovery built in**: Type a repo keyword from home to search GitHub repos, filter them, then open instantly.
- **File Preview**: Preview source code and text files directly in the TUI.
- **Handles the big stuff**: Built-in support for GitHub LFS (Large File Storage).
- **Batch mode**: Select a bunch of files and folders to download them all at once.
- **Release downloads**: Grab GitHub release artifacts with OS/architecture-aware selection.

## Supported platforms

- Repository browsing and file or folder downloads: GitHub, GitLab, Codeberg, Gitea, Forgejo, and compatible self-hosted instances.
- TUI quick repository search from the home screen: GitHub only.
- `release` / `rel` downloads: GitHub only.
- GitHub LFS resolution: GitHub only.

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

```bash
nix run github:abhixdd/ghgrab
```

To have a specific tagged version:

```bash
nix run "github:abhixdd/ghgrab/<tag>"
```

### Aur (Arch linux)

```bash
yay -S ghgrab-bin   
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

# Browse GitLab
ghgrab https://gitlab.com/gitlab-org/gitlab

# Browse Codeberg / Forgejo / Gitea
ghgrab https://codeberg.org/forgejo/forgejo

# Download to current directory directly
ghgrab https://github.com/rust-lang/rust --cwd --no-folder
```

You can also type a repository keyword on the home screen (for example `ratatui`) and press `Enter` to open repository search mode.

### CLI Flags

|Flag|Description|
|---|---|
|`--cwd`|Forces download to the current working directory.|
|`--no-folder`|Downloads files directly without creating a subfolder for the repo.|
|`--token <TOKEN/AUTO/GH>`|Use a specific GitHub token for this run (doesn't save to settings). `auto`/`gh` uses `gh auth token` at runtime.|

### Release Downloads

You can also download GitHub release assets directly with the user-facing `release` command or its short alias `rel`.

The `release` command is currently GitHub-only even though normal repository browsing and file downloads support multiple forges.

Basic examples:

```bash
# Download the best matching artifact for your OS and architecture
ghgrab rel sharkdp/bat

# Extract an archive after download
ghgrab rel sharkdp/bat --extract

# Download a specific release tag
ghgrab rel sharkdp/bat --tag v0.25.0

# Match a specific asset by regex
ghgrab rel sharkdp/bat --asset-regex "x86_64.*windows.*zip"

# Download into a custom directory
ghgrab rel sharkdp/bat --extract --out ./tmp/bat

# Install the selected file or extracted binary into a target directory
ghgrab rel sharkdp/bat --extract --bin-path ~/.local/bin
```

Basic release flags:

| Flag | Description |
| ---- | ----------- |
| `--tag <TAG>` | Download a specific release tag. |
| `--asset-regex <REGEX>` | Match a specific release asset by regex. |
| `--extract` | Extract archive assets after download. |
| `--out <DIR>` | Download into a custom output directory. |
| `--bin-path <DIR>` | Copy the selected file or extracted binary into the provided directory. |
| `--os <OS>` | Override detected operating system. |
| `--arch <ARCH>` | Override detected architecture. |

<details>
<summary>Show release download usage, flags, and examples</summary>

How selection works:

- If there is one clear best asset match for your OS and architecture, ghgrab downloads it directly.
- If multiple close matches exist, ghgrab shows an interactive picker in the terminal.
- Type the asset number and press `Enter` to continue.
- Type `q` and press `Enter` to cancel the picker.

```bash
# Pick a release tag explicitly
ghgrab rel sharkdp/bat --tag v0.25.0

# Match assets with a regex
ghgrab rel sharkdp/bat --asset-regex "x86_64.*linux.*tar.gz"

# Extract an archive after download
ghgrab rel sharkdp/bat --extract

# Install the selected file or extracted binary into a target directory
ghgrab rel sharkdp/bat --extract --bin-path ~/.local/bin

# Download to a custom directory
ghgrab rel sharkdp/bat --extract --out ./tmp/bat

# Force Windows x64 asset selection
ghgrab rel BurntSushi/ripgrep --os windows --arch amd64

# Allow prereleases when selecting the latest release
ghgrab rel starship/starship --prerelease
```

### Release Flags

| Flag | Description |
| ---- | ----------- |
| `--tag <TAG>` | Download a specific release tag instead of the latest matching release. |
| `--prerelease` | Allow prereleases when `--tag` is not provided. |
| `--asset-regex <REGEX>` | Match a specific release asset by regex. Useful for forcing one artifact and skipping the picker. |
| `--os <OS>` | Override detected operating system for asset selection. |
| `--arch <ARCH>` | Override detected architecture for asset selection. |
| `--file-type <TYPE>` | Prefer `any`, `archive`, or `binary` assets. |
| `--extract` | Extract archive assets after download. Supports `.zip`, `.tar.gz`, `.tgz`, and `.tar.xz`. |
| `--out <DIR>` | Download into a custom output directory. |
| `--bin-path <DIR>` | Copy the selected file or extracted binary into the provided directory. |
| `--cwd` | Download into the current working directory. |
| `--token <TOKEN/AUTO/GH>` | Use a one-time GitHub token for this run without saving it. `auto`/`gh` uses `gh auth token` at runtime. |

### Release Examples

```bash
# Download a specific ripgrep release for Windows x64
ghgrab rel BurntSushi/ripgrep --tag 15.1.0 --os windows --arch amd64

# Use a regex to choose one exact asset
ghgrab rel sharkdp/bat --asset-regex "x86_64.*windows.*zip"

# Install an extracted binary into your local bin directory
ghgrab rel sharkdp/bat --extract --bin-path ~/.local/bin

# Use the long command form
ghgrab release sharkdp/bat
```

</details>

### Environment Variables

`ghgrab` also accepts GitHub tokens from environment variables:

- `GHGRAB_GITHUB_TOKEN`
- `GITHUB_TOKEN`

### Runtime token auto mode

If you already use GitHub CLI, you can avoid manual token copy/paste:

```bash
ghgrab rel sharkdp/bat --token auto
ghgrab agent tree https://github.com/rust-lang/rust --token gh
```

- Uses `gh auth token` at runtime only.
- Never prints the raw token.
- If multiple token lines are returned, ghgrab reports this and uses one token.

### Agent Mode

For scripts, agents, and other non-interactive workflows, `ghgrab` includes a machine-friendly `agent` command that prints a stable JSON envelope with `api_version`, `ok`, `command`, and either `data` or `error`.

```bash
# Fetch the repository tree as JSON
ghgrab agent tree https://github.com/rust-lang/rust

# GitLab also works in agent mode for tree and download operations
ghgrab agent tree https://gitlab.com/gitlab-org/gitlab

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

### MCP (Model Context Protocol) Server

`ghgrab` can act as an MCP server, allowing AI coding assistants (like Cursor, Claude Desktop, VS Code, and Windsurf) to search GitHub repositories, list directories, download folders/files, read file contents, and download release assets directly.

#### Auto-Installation

You can automatically configure `ghgrab` as an MCP server in your favorite AI client by running:

```bash
ghgrab mcp --install
```

This interactive command will:
1. Detect which supported clients are installed on your machine.
2. Prompt you to select one.
3. Automatically append the correct `ghgrab mcp` command block to that client's configuration file.
4. Pass through your saved GitHub token (if set in `ghgrab config`) so the AI client doesn't hit anonymous API rate limits.

#### Manual Configuration

If you prefer to configure it manually, add the following to your client's MCP configuration file (e.g. `claude_desktop_config.json` or `mcp.json`):

```json
{
  "mcpServers": {
    "ghgrab": {
      "command": "ghgrab",
      "args": ["mcp"],
      "env": {
        "GITHUB_TOKEN": "your_github_token_here"
      }
    }
  }
}
```

*Note: The `env.GITHUB_TOKEN` block is optional but highly recommended to avoid GitHub API rate limiting.*

#### Exposed Tools

The `ghgrab` MCP server exposes 8 powerful tools to AI assistants:

1. `repo_tree` - Lists all files and directories in a Git repository (supports GitHub, GitLab, Codeberg, Gitea, Forgejo).
2. `download_files` - Downloads specific files or folders from a repository to the local filesystem.
3. `download_release` - Downloads a GitHub release asset with OS/arch auto-detection (runs non-interactively).
4. `search_repos` - Searches GitHub repositories by keyword.
5. `read_file` - Reads the raw text content of a single file from a repository.
6. `read_file_preview` - Reads a preview (first N bytes) of a file from a repository (useful for peeking at large files).
7. `list_releases` - Lists all releases for a GitHub repository.
8. `repo_info` - Gets repository metadata such as default branch, platform type, and parsed URL components.

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

### Theming

ghgrab supports custom color themes via a TOML config file.

- Linux/macOS: `~/.config/ghgrab/theme.toml`
- Windows: `%APPDATA%\ghgrab\theme.toml`

Any missing key falls back to the default Tokyo Night theme. Colors must use `#RRGGBB` hex format.

See [`examples/theme.toml`](examples/theme.toml) for a complete example.

### Keyboard Shortcuts (How to move around)

We've kept it pretty standard, but here's a quick cheat sheet:

| Key                               | Action                                                                   |
| --------------------------------- | ------------------------------------------------------------------------ |
| `Enter` (home)                    | Open URL or start repository search                                      |
| `Enter` / `l` / `Right` (browser) | Enter directory                                                          |
| `Backspace` / `h` / `Left`        | Go back to previous folder                                               |
| `Delete` (home)                   | Delete character at cursor                                               |
| `Tab`                             | Auto-fill `https://github.com/` (Home page)                              |
| `/`                               | Start Searching (File list)                                              |
| `Esc`                             | **Exit Search** or **Return Home** (file list) or **Quit** (home screen) |
| `q` / `Q`                         | **Quit** (from file list)                                                |
| `Ctrl+q`                          | **Force Quit** (anywhere)                                                |
| `Space`                           | Toggle selection for the current item                                    |
| `p` / `P`                         | **Preview** current file                                                 |
| `a`                               | Select All items                                                         |
| `u`                               | Unselect all items                                                       |
| `d` / `D`                         | Download selected items                                                  |
| `i`                               | Toggle Icons (Emoji / ASCII)                                             |
| `g` / `Home`                      | Jump to Top                                                              |
| `G` / `End`                       | Jump to Bottom                                                           |

### Repository Search Mode Shortcuts

| Key                   | Action                                                             |
| --------------------- | ------------------------------------------------------------------ |
| `j` / `k` / `↑` / `↓` | Move selection                                                     |
| `Enter`               | Open selected repository                                           |
| `f`                   | Toggle include/exclude forks                                       |
| `m`                   | Cycle minimum stars (`Any`, `10+`, `50+`, `100+`, `500+`, `1000+`) |
| `l`                   | Cycle language filter                                              |
| `s`                   | Cycle sort (`Stars`, `Updated`, `Name`)                            |
| `x`                   | Reset all filters                                                  |
| `r`                   | Refresh current search                                             |
| `Esc`                 | Return to home input                                               |

---

## Join the community

If you find a bug, have an idea for a cool new feature, or just want to help out, we'd love to hear from you! Check out our [Contributing Guide](CONTRIBUTING.md) to see how you can get involved.

## License

Distributed under the MIT License. It's open, free, and yours to play with. See [LICENSE](LICENSE) for the fine print.
