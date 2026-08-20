# herdr-tab-smart-rename-rs

Rust rewrite of `herdr-tab-smart-rename`.

The plugin removes the Bun runtime and the reference plugin's detached listener. It supports:

- manual rename of the current tab;
- one automatic rename after a coding agent first completes in an auto-named tab;
- OpenAI-compatible model configuration;
- deterministic labels for obvious terminal commands.

## Install locally

```sh
herdr plugin link .
```

Herdr runs the manifest build command:

```sh
cargo build --release
```

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

## Documentation

- [Herdr plugin API research](docs/herdr-plugin-api.md)
- [Reference implementation notes](docs/reference-implementation.md)
- [Naming policy](docs/naming-policy.md)

