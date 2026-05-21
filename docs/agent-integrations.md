# Agent Integrations

The `ghgrab agent` subcommand exposes a stable JSON envelope for non-interactive tooling — CI pipelines, MCP servers, and agent runtimes that need repo files without a full `git clone`.

## JSON envelope

Every `agent` command prints JSON to stdout:

```json
{
  "api_version": "1",
  "ok": true,
  "command": "download",
  "data": { "...": "..." }
}
```

On failure, `ok` is `false` and `error` contains `code` and `message`.

## Willow 2.0 — SAFE app install

[Willow 2.0](https://github.com/rudi193-cmd/willow-2.0) uses `ghgrab agent download` when installing standalone SAFE apps via MCP `app_install`. It prefers ghgrab over `git clone` because it fetches working-tree files without `.git/` history — faster and smaller for agent sandboxes.

```bash
ghgrab agent download https://github.com/example/my-safe-app \
  --repo --out ./apps/my-safe-app --no-folder
```

Willow invocation (from `sap/sap_mcp.py`):

```python
cmd = [
    "ghgrab", "agent", "download", repo_url,
    "--repo", "--out", str(code_path), "--no-folder",
]
# Optional: --token $GITHUB_TOKEN for private repos
```

When `ghgrab` is not on `PATH`, Willow falls back to `git clone`.

### Why agents use `--repo --no-folder`

| Flag | Effect |
|------|--------|
| `--repo` | Download the entire repository tree |
| `--no-folder` | Write directly into `--out` without an extra repo-named subfolder |
| `--out` | Target directory (created if needed) |

### Multi-forge support

The TUI and `agent` commands support GitHub, GitLab, Codeberg, Gitea, Forgejo, and compatible self-hosted instances (see [commands.md](commands.md)). Willow's install path uses the same URLs it would pass to `git clone`.

## Other integration patterns

### Cherry-pick context for coding agents

Download only the files an agent needs before analysis:

```bash
ghgrab agent download https://github.com/org/repo src/lib README.md --out ./context
```

### CI artifact staging

Use `--token auto` to pick up `gh auth token` without persisting credentials:

```bash
ghgrab agent download https://github.com/org/private-repo --repo --out ./staging --token auto
```

### Tree inspection before download

Inspect structure without downloading:

```bash
ghgrab agent tree https://github.com/rust-lang/rust | jq '.data.entries[:5]'
```

## Submitting an integration

If your project uses `ghgrab agent` in production, open a PR adding a short section here with:

- Project name and link
- Command shape (flags matter)
- Why ghgrab vs `git clone` or the GitHub API

## Homebrew

Install ghgrab on macOS/Linux with the bundled formula (until accepted into Homebrew core):

```bash
brew install --formula Formula/ghgrab.rb
# or, after tap setup:
brew install ghgrab
```

See [installation.md](installation.md) for npm, cargo, pipx, and nix options.
