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

## Arch Linux

```bash
yay -S ghgrab-bin
```

## Verify the install

After installation, confirm the binary is available:

```bash
ghgrab --help
```

If you installed with `pipx` or `pip`, the launcher will fetch the platform-specific binary on first use when it is not already present.
