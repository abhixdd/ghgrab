# Commands

`ghgrab` supports interactive usage and three command groups: `release`, `agent`, and `config`.

## Base command

```bash
ghgrab [URL] [--cwd] [--no-folder] [--token TOKEN]
```

### Global options

| Flag | Description |
| --- | --- |
| `--cwd` | Download files to the current working directory |
| `--no-folder` | Download directly into the target directory without a repository subfolder |
| `--token <TOKEN\|auto\|gh>` | Use a one-time GitHub token without storing it. `auto`/`gh` uses `gh auth token` at runtime |

## Release downloads

The `release` command, with alias `rel`, downloads release assets from a repository:

```bash
ghgrab rel <owner/repo> [OPTIONS]
```

### Common examples

```bash
# Download the best matching artifact
ghgrab rel sharkdp/bat

# Download a specific release tag
ghgrab rel sharkdp/bat --tag v0.25.0

# Allow prereleases when selecting the latest release
ghgrab rel starship/starship --prerelease

# Force an exact asset with regex
ghgrab rel sharkdp/bat --asset-regex "x86_64.*windows.*zip"

# Extract archives after download
ghgrab rel sharkdp/bat --extract

# Install an extracted binary into a target directory
ghgrab rel sharkdp/bat --extract --bin-path ~/.local/bin
```

### Release options

| Flag | Description |
| --- | --- |
| `--tag <TAG>` | Download a specific release tag |
| `--prerelease` | Allow prereleases when `--tag` is not provided |
| `--asset-regex <REGEX>` | Match a specific release asset |
| `--os <OS>` | Override detected operating system |
| `--arch <ARCH>` | Override detected architecture |
| `--file-type <TYPE>` | Prefer `any`, `archive`, or `binary` assets |
| `--extract` | Extract archive assets after download |
| `--out <DIR>` | Use a custom output directory |
| `--bin-path <DIR>` | Install the selected file or binary into the provided directory |
| `--cwd` | Download into the current working directory |
| `--token <TOKEN\|auto\|gh>` | Use a one-time GitHub token for this run. `auto`/`gh` uses `gh auth token` at runtime |

### Selection behavior

- If one asset clearly matches the current platform, `ghgrab` downloads it immediately.
- If several close matches exist, `ghgrab` shows an interactive picker.
- If no assets match the filters, the command exits with an error.

## Agent mode

The `agent` command is designed for non-interactive tooling. It prints a stable JSON envelope with:

- `api_version`
- `ok`
- `command`
- `data` on success
- `error` on failure

### Fetch a repository tree

```bash
ghgrab agent tree https://github.com/rust-lang/rust
```

### Download specific paths

```bash
ghgrab agent download https://github.com/rust-lang/rust src/tools README.md --out ./tmp
```

### Download a subtree

```bash
ghgrab agent download https://github.com/rust-lang/rust --subtree src/tools --out ./tmp
```

### Download the entire repository

```bash
ghgrab agent download https://github.com/rust-lang/rust --repo --out ./tmp
```

### Agent command rules

- `agent download --repo` cannot be combined with positional paths.
- `agent download --repo` cannot be combined with `--subtree`.
- `agent download --subtree` cannot be combined with positional paths.

### Agent options

#### `agent tree`

| Flag | Description |
| --- | --- |
| `--token <TOKEN\|auto\|gh>` | Use a one-time GitHub token. `auto`/`gh` uses `gh auth token` at runtime |

#### `agent download`

| Flag | Description |
| --- | --- |
| `--repo` | Download the entire repository |
| `--subtree <PATH>` | Download one subtree path |
| `--cwd` | Download into the current working directory |
| `--no-folder` | Skip the repository subfolder |
| `--out <DIR>` | Use a custom output directory |
| `--token <TOKEN\|auto\|gh>` | Use a one-time GitHub token. `auto`/`gh` uses `gh auth token` at runtime |

## Configuration

The `config` command manages saved settings:

```bash
ghgrab config set token YOUR_TOKEN
ghgrab config set path "/your/custom/path"
ghgrab config list
ghgrab config unset token
ghgrab config unset path
```

`config list` masks stored tokens before printing them.

### Runtime token auto-detection (no save)

If you do not want to store a token, use:

```bash
ghgrab rel sharkdp/bat --token auto
ghgrab agent tree https://github.com/rust-lang/rust --token gh
```

Behavior:

- Uses `gh auth token` at runtime only.
- Never prints the raw token.
- If multiple token lines are returned, `ghgrab` reports that multiple tokens were found and uses one token.
