# herdr-tab-smart-rename-rs

Rust rewrite of `herdr-tab-smart-rename`.

The plugin removes the Bun runtime and the reference plugin's detached listener. It supports:

- manual rename of the current tab;
- one automatic rename after a coding agent first completes in an auto-named tab;
- OpenAI-compatible model configuration;
- deterministic labels for obvious terminal commands.

## Install from GitHub

After a tagged release has been published, install with:

```sh
herdr plugin install EmmetZ/herdr-tab-smart-rename-rs
```

The install step runs `herdr/install.sh`. It downloads the matching prebuilt
binary from the GitHub Release, verifies its SHA-256 checksum, and installs it
under the plugin's `bin/` directory.

## Link a local checkout

```sh
cd /path/to/herdr-tab-smart-rename-rs
cargo build --release
mkdir -p bin
install -m 0755 target/release/herdr-tab-smart-rename-rs bin/herdr-tab-smart-rename-rs
herdr plugin link .
```

`herdr plugin link` is for local development. Build the Rust binary yourself
and place it under `bin/` before linking; `herdr/install.sh` is only used by
`herdr plugin install`.

## Configure AI

Create or edit:

```text
~/.config/herdr/plugins/config/tab-smart-rename/provider.env
```

For the default OpenAI-compatible setup:

```dotenv
OPENAI_API_KEY=...
```

`SMART_RENAME_API_KEY` can be used for any configured provider.

## Actions

```sh
herdr plugin action invoke rename-now --plugin tab-smart-rename
herdr plugin action invoke check-ai --plugin tab-smart-rename
```

## Release

Release binaries are produced by GitHub Actions when a version tag is pushed:

```sh
git tag v0.1.0
git push origin main v0.1.0
```

The release workflow builds:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `aarch64-unknown-linux-musl`
- `x86_64-unknown-linux-musl`

## Documentation

- [Herdr plugin API research](docs/herdr-plugin-api.md)
- [Reference implementation notes](docs/reference-implementation.md)
- [Naming policy](docs/naming-policy.md)
- [Release packaging](docs/release-packaging.md)
