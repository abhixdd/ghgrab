# Theming

`ghgrab` supports custom UI colors through a TOML theme file.

## Theme file location

- Linux and macOS: `~/.config/ghgrab/theme.toml`
- Windows: `%APPDATA%\\ghgrab\\theme.toml`

Create the `ghgrab` directory first if it does not already exist.

## Color format

All colors must use `#RRGGBB` hex values.

Any missing key falls back to the default Tokyo Night theme.

## Example theme

```toml
bg_color = "#24283b"
fg_color = "#c0caf5"
accent_color = "#7aa2f7"
warning_color = "#e0af68"
error_color = "#f7768e"
success_color = "#9ece6a"
folder_color = "#82aaff"
selected_color = "#ff9e64"
border_color = "#565f89"
highlight_bg = "#292e42"
```

The repository includes a complete example in `examples/theme.toml`.
