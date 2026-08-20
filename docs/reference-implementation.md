# Reference implementation notes

Updated: 2026-08-20

Reference directory: `herdr-tab-smart-rename/`  
This directory is read-only input for this rewrite and is ignored by the outer git repository.

## Files reviewed

- `herdr-plugin.toml`
- `src/cli.ts`
- `src/service.ts`
- `src/domain.ts`
- `src/herdr.ts`
- `src/provider.ts`
- `src/storage.ts`
- `src/pi-context.ts`
- `src/text.ts`
- `docs/naming-policy.md`
- `test/domain.test.ts`, `test/service.test.ts`, `test/runtime.test.ts`, `test/provider.test.ts`

## Behavior contract extracted

The reference plugin separates pure naming rules from Herdr IO:

- `isDefaultLabel` treats blank or numeric tab labels as automatic.
- meaningful existing labels are manual unless an explicit reset or rename reclaims them.
- focused agent panes outrank supporting server/log panes.
- deterministic process labels avoid model calls for obvious tasks.
- model output is JSON and must pass label validation.
- context is bounded and sanitized before leaving the machine.
- state persists expected automatic writes so the plugin does not classify its own rename as manual.

## Rust rewrite decision

The current request intentionally reduces the feature surface. The Rust rewrite keeps the naming contract but replaces the always-on worker with short-lived commands:

- `rename-now` manually evaluates and renames the current tab.
- `agent-status-event` is invoked by Herdr when an agent status changes.
- the first `done` event after observed agent work performs at most one automatic rename per tab.
- automatic rename skips any tab that was created or changed with a meaningful user label.

This is simpler than the reference implementation and avoids over-design for unsupported requirements.

