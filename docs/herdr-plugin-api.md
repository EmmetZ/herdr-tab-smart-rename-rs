# Herdr Plugin API research

Updated: 2026-08-20

Sources checked:

- <https://herdr.dev/docs/plugins/>
- <https://herdr.dev/docs/socket-api/>
- <https://herdr.dev/docs/commands/pane/>
- <https://herdr.dev/docs/commands/agent/>

## Plugin shape

A Herdr plugin is described by `herdr-plugin.toml`. Herdr runs the declared commands directly from the plugin root, so a Rust plugin can use `cargo build --release` during installation and then invoke the compiled binary from action and event entries.

This manifest currently declares `linux` and `macos`. Windows support should add a launcher or Windows-specific installed command path before being declared, because the compiled artifact is `target/release/herdr-tab-smart-rename-rs.exe` on Windows.

Relevant manifest sections:

- `id`, `name`, `version`, `min_herdr_version`, `description`, `platforms`
- `[[build]]` for install/update build commands
- `[[actions]]` for user-triggered commands
- `[[events]]` for Herdr event hooks

Herdr injects plugin directories and caller context through environment variables. This implementation relies on:

- `HERDR_BIN_PATH` to call the same Herdr CLI instance when available
- `HERDR_PLUGIN_ROOT` for bundled files
- `HERDR_PLUGIN_STATE_DIR` for private state
- `HERDR_PLUGIN_CONFIG_DIR` for provider configuration
- `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, and `HERDR_PANE_ID` for action context
- `HERDR_PLUGIN_EVENT_JSON` for event-hook payloads

## Runtime APIs used

This rewrite uses the CLI APIs documented by Herdr instead of a long-running socket listener:

- `herdr api snapshot` returns the live workspace/tab/pane model.
- `herdr tab rename <tab-id> <label>` applies the final tab name.
- `herdr pane read <pane-id> --source recent-unwrapped --lines <n>` reads completed agent output without soft-wrap artifacts.
- `herdr pane process-info --pane <pane-id>` supports deterministic names for obvious commands.
- `herdr notification show ...` reports action outcomes when available.

The old TypeScript worker subscribed to socket events and performed periodic sweeps. The Rust rewrite does not open the socket. Automatic behavior is driven by Herdr event hooks only.

## Agent completion hook

Herdr reports coding-agent lifecycle state with values such as `working`, `blocked`, `done`, `idle`, and `unknown`. The event hook is registered for `pane.agent_status_changed`.

The plugin handles these transitions as follows:

- `working`: remember that this tab has run an agent.
- `done`: if this is the first completed run for the tab and the tab is still auto-eligible, read the pane content and rename once.
- any other status: no rename.

The manifest declares this with `[[events]]` and `on = "pane.agent_status_changed"`. This satisfies the current requirement without the reference plugin's listener and sweep.

## Manual-name rule

Herdr may create tabs with user-provided names. A tab is auto-eligible only when its current label is blank, numeric, or equal to the tab number. A meaningful non-default label is treated as manual and is not inspected during automatic agent completion.

The explicit `rename-now` action is different: it is a direct user command, so it may reclaim and rename the current tab even if it already has a meaningful label.

## Reference plugin behavior kept or removed

Kept:

- 2-4 Title Case words, maximum 30 characters
- deterministic labels for tests, dev servers, logs, and remote shells
- OpenAI-compatible provider configuration
- bounded, sanitized context
- manual labels win for automatic behavior
- explicit rename can force a fresh name

Removed for this Rust scope:

- Bun runtime and TypeScript build
- detached worker process
- direct socket subscription
- 60-second sweep
- rename-all and workspace rename actions
- progress-pulse temporary tab labels
