# Overview

`ghgrab` helps you pull individual files, folders, or release artifacts from GitHub without cloning an entire repository.

## What it does well

- Browse repositories in a full-screen terminal UI.
- Open a repository directly from a GitHub URL.
- Search for repositories from the home screen.
- Preview text and source files before downloading.
- Select multiple files or folders and download them in one run.
- Download GitHub release assets with OS and architecture-aware matching.
- Expose machine-readable commands for scripts and agent workflows.

## Why use it

The usual alternative is `git clone`, followed by manually deleting everything you do not need. `ghgrab` is built for the opposite workflow: identify the exact paths you want, then fetch only those assets.

That is useful when you want to:

- inspect examples from a large repository,
- grab one directory into an existing project,
- download a binary release quickly,
- automate GitHub file retrieval from CI or agent tooling.

## Distribution options

`ghgrab` is published through several packaging channels:

- Cargo
- npm
- pipx / pip
- Nix
- AUR

The Python package acts as a lightweight launcher that downloads the platform binary when needed. The Rust crate builds the native application directly.
