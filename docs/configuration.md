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

## GitHub authentication

You can provide a token in three ways:

1. with `--token <TOKEN>` for a single command,
2. with a saved config token,
3. with an environment variable.

Supported environment variables:

- `GHGRAB_GITHUB_TOKEN`
- `GITHUB_TOKEN`

The command-line flag takes precedence over environment variables, and environment variables take precedence over saved config.

## Download destination

You can control where files are written by using:

- `ghgrab config set path ...` for a saved default,
- `--out <DIR>` for command-specific output,
- `--cwd` to force downloads into the current working directory.

Use `--no-folder` when you want downloaded paths placed directly in the target directory instead of a repository-named folder.

## When a token helps

GitHub token support is especially useful when:

- you hit anonymous API rate limits,
- you are scripting against private repositories that your token can access,
- you want more reliable automation in CI or agent workflows.
