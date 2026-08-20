# herdr-tab-smart-rename-rs

[Chinese / 简体中文](README.zh-CN.md)

`herdr-tab-smart-rename-rs` is a Herdr plugin that turns generic tab labels into short, meaningful task names from the current tab context.

It is designed for parallel coding-agent workflows. When a tab still has its default or numeric label, the plugin inspects its pane and tab context, proposes a recognizable name, and makes switching between active tasks easier.

## Features

- Rename the current tab on demand.
- Automatically rename a default tab after a coding agent first completes work.
- Preserve labels that were named manually.
- Use OpenAI and OpenAI-compatible APIs.
- Recognize common terminal commands deterministically, avoiding an AI request where possible.
- Ship as a single Rust binary with no Bun dependency.

## Install

Install through Herdr:

```sh
herdr plugin install EmmetZ/herdr-tab-smart-rename-rs
```

Supported platforms:

- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS Apple Silicon

## Configure AI

Open the plugin's private configuration file in a Herdr overlay:

```sh
herdr plugin action invoke configure-ai --plugin tab-smart-rename
```

When installed with `herdr plugin install`, the plugin creates `provider.env` in Herdr's private plugin configuration directory. It initializes the file from `provider.env.example` and never overwrites an existing configuration.

`provider.env` is read before every model request:

```dotenv
# Default OpenAI configuration
OPENAI_API_KEY=your_key
SMART_RENAME_PROVIDER=openai
SMART_RENAME_BASE_URL=https://api.openai.com/v1
SMART_RENAME_MODEL=gpt-5.6-luna
# Optional: low, medium, or high
SMART_RENAME_REASONING_EFFORT=medium
SMART_RENAME_TIMEOUT_MS=45000
```

For another OpenAI-compatible provider, set `SMART_RENAME_API_KEY` and replace `SMART_RENAME_PROVIDER`, `SMART_RENAME_BASE_URL`, and `SMART_RENAME_MODEL` with that provider's values. `SMART_RENAME_API_KEY` takes precedence when both it and `OPENAI_API_KEY` are set. `SMART_RENAME_REASONING_EFFORT` accepts `low`, `medium`, or `high`; when it is not set, the field is omitted for non-default providers.

## Usage

Validate the AI configuration:

```sh
herdr plugin action invoke check-ai --plugin tab-smart-rename
```

Rename the current tab immediately:

```sh
herdr plugin action invoke rename-now --plugin tab-smart-rename
```

### Key binding

Add the following to your Herdr user key bindings to rename the current tab with `prefix+t`:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "tab-smart-rename.rename-now"
description = "smart rename current tab"
```

Change `key` to any available Herdr binding. This invokes the same manual rename action and may replace the current tab's existing label.

### Automatic renaming

No background process is required. The plugin reacts to Herdr's `pane.agent_status_changed` event and renames a tab only when all of the following are true:

1. The tab has passed its initial completion event, which only establishes the agent-ready baseline.
2. The agent is subsequently observed in the `working` state.
3. The agent reaches its next completion state: `done`, or typically `idle` after Codex finishes a response.
4. The tab label is still default or numeric.

The initial completion state does not trigger a rename. Existing meaningful labels are left unchanged.

## Naming policy

Names are intentionally concise and task-oriented:

- `fix-tests`
- `auth-refactor`
- `api-client`
- `docs-update`
- `ui-layout`

When the context is insufficient, the plugin does not replace an existing meaningful label.

## Build locally

Build from source and link the plugin:

```sh
cargo build --release
mkdir -p bin
install -m 0755 target/release/herdr-tab-smart-rename-rs bin/herdr-tab-smart-rename-rs
herdr plugin link .
```

## Documentation

- [Herdr plugin API research](docs/herdr-plugin-api.md)
- [Agent status lifecycle](docs/agent-status-lifecycle.md)
- [Reference implementation notes](docs/reference-implementation.md)
- [Naming policy](docs/naming-policy.md)
- [Release packaging](docs/release-packaging.md)
