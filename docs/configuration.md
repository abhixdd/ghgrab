# Configuration

`ghgrab` supports both saved configuration and one-off overrides.

## Saved settings

Use the `config` command to store values locally:

```bash
ghgrab config set token YOUR_TOKEN
ghgrab config set path "/your/custom/path"
```

Show the current configuration:

```bash
ghgrab config list
```

Remove a saved value:

```bash
ghgrab config unset token
ghgrab config unset path
```

## Authentication

You can provide a token in three ways:

1. with `--token <TOKEN|auto|gh|none>` for a single command,
2. with a saved config token,
3. with an environment variable.

When `--token auto` or `--token gh` is used, `ghgrab` calls `gh auth token` at runtime and uses that token for the current run only (it is not saved).

Use `--token none` (or `off`) to skip every token, including `gh auth token` discovery.

Supported environment variables:

- `GHGRAB_GITHUB_TOKEN`
- `GITHUB_TOKEN`
- `GITLAB_TOKEN` (sent to gitlab.com only)
- `CODEBERG_TOKEN` (sent to codeberg.org only)

The command-line flag takes precedence over environment variables, and environment variables take precedence over saved config. If no token is found, `ghgrab` may attempt GitHub CLI token discovery as a runtime fallback.

### Which host gets which token

| Token source | Sent to |
| --- | --- |
| `gh auth token`, `GHGRAB_GITHUB_TOKEN`, `GITHUB_TOKEN`, saved config | github.com only |
| `--token <TOKEN>` | The repository host you open in that command, on any forge |
| `GITLAB_TOKEN` | gitlab.com only |
| `CODEBERG_TOKEN` | codeberg.org only |

GitHub tokens are never sent to other forges, so opening a GitLab, Codeberg, or unknown URL cannot leak them. For a self-hosted GitLab, Gitea, or Forgejo instance, pass its token with `--token`. The MCP server only ever sends `--token` to github.com, since its URLs come from the model.

If a host rejects a token, `ghgrab` retries the request once without it, so public repositories still load.

## Download destination

You can control where files are written by using:

- `ghgrab config set path ...` for a saved default,
- `--out <DIR>` for command-specific output,
- `--cwd` to force downloads into the current working directory.

Use `--no-folder` when you want downloaded paths placed directly in the target directory instead of a repository-named folder.

## When a token helps

Token support is especially useful when:

- you hit anonymous API rate limits,
- you are scripting against private repositories that your token can access,
- you want more reliable automation in CI or agent workflows.

Notes:

- `gh auth token` auto-discovery is GitHub CLI based, so it is mainly useful for GitHub workflows.
- TUI repository search still uses GitHub search only.
- Release downloads still use GitHub releases only.
