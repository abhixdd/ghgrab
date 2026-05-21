# Installation

Choose the package manager that matches your environment.

## npm

```bash
npm install -g @ghgrab/ghgrab
```

## Cargo

```bash
cargo install ghgrab
```

## pipx

`pipx` is the cleanest way to install the Python wrapper globally:

```bash
pipx install ghgrab
```

## Nix

Run the latest commit:

```bash
nix run github:abhixdd/ghgrab
```

Run a tagged release:

```bash
nix run "github:abhixdd/ghgrab/<tag>"
```

Use full semantic version tags for releases, for example `v2.0.1`.

## Arch Linux

```bash
yay -S ghgrab-bin
```

## Homebrew

A formula ships in this repository for macOS and Linux (Intel and ARM64). It downloads prebuilt release binaries from GitHub.

From a git checkout:

```bash
brew install --formula Formula/ghgrab.rb
```

To install a tagged release after cloning:

```bash
git clone https://github.com/abhixdd/ghgrab.git
cd ghgrab
brew install --formula Formula/ghgrab.rb
```

Submitting to [Homebrew/homebrew-core](https://github.com/Homebrew/homebrew-core) is tracked in [issue #52](https://github.com/abhixdd/ghgrab/issues/52).

## Verify the install

After installation, confirm the binary is available:

```bash
ghgrab --help
```

If you installed with `pipx` or `pip`, the launcher will fetch the platform-specific binary on first use when it is not already present.
