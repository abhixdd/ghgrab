# Development

## Local setup

To build `ghgrab` locally you need a working Rust toolchain.

```bash
git clone https://github.com/abhixdd/ghgrab.git
cd ghgrab
cargo build
```

Run the TUI locally:

```bash
cargo run
```

## Test and quality checks

Run the project's test and quality commands before submitting changes:

```bash
cargo test
cargo fmt
cargo clippy
```

## Package layout

The repository currently includes:

- a Rust application under `src/`,
- Rust integration tests under `tests/`,
- a Python launcher package under `ghgrab/`,
- packaging metadata for Cargo, npm, Python, Nix, and Arch Linux.

## Documentation workflow

The documentation is built with Sphinx and the Read the Docs theme.

Install the docs dependencies:

```bash
python -m pip install -r docs/requirements.txt
```

Build the HTML site:

```bash
sphinx-build -b html docs docs/_build/html
```
