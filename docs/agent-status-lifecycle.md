# Agent status lifecycle

## Goal

Automatic tab naming must run exactly once after the first real agent response
in an eligible tab. Herdr emits a completion event while it initializes an
interactive agent, so that first completion event must never rename the tab.

## Event identity

The plugin accepts both Herdr event spellings:

- `pane.agent_status_changed`
- `pane_agent_status_changed`

Event payloads are not required to carry a `tab_id`. In that case, the plugin
resolves the tab from the current Herdr snapshot using `pane_id`. The same
resolution rule applies to both `working` and completion events; otherwise the
state machine can observe a completion without recording its preceding work.

## State machine

The persisted state is scoped by tab ID and has three relevant flags:

- `saw_working`: a `working` event was observed for the tab.
- `initialization_complete`: the tab has passed its first completion event,
  which establishes the agent-ready baseline.
- `auto_after_done`: the one automatic naming attempt for the tab has already
  been made.

Herdr exposes a Codex session by opaque ID in the live snapshot, not always as
a readable transcript path. Session text remains optional naming context, but
is not an event gate.

| Incoming status | Required evidence | Action |
| --- | --- | --- |
| `working` | Resolvable tab ID | Set `saw_working`; do not rename. |
| First `done` / `idle` | None | Mark initialization complete, clear `saw_working`, and do not rename. |
| Later `done` / `idle` | `saw_working`, or an explicit prior status of `working` / `blocked` | Attempt automatic naming once. |
| Later `done` / `idle` | No preceding work evidence | Ignore the event. |
| Any other status | None | Ignore the event. |

An absent prior status is not completion evidence. The baseline completion
event is ignored regardless of its prior status, ensuring that agent startup
does not consume the later automatic naming attempt.

## Naming guardrails

After completion evidence is established, automatic naming still runs only
when the tab retains a default label. A user-provided label remains authoritative.
The explicit `rename-now` action is independent of this lifecycle and may
rename a manually named tab because it is directly user initiated.

## Required regression coverage

- An initial `idle` event with no earlier `working` event does not rename.
- An initialization `working` → `idle` sequence does not rename or consume
  the later automatic attempt.
- A serialized event sequence containing only `pane_id` performs one rename
  for the first post-initialization `working` followed by `idle`.
- A completion event with explicit `old_status = working` can rename even if
  the plugin did not receive the earlier `working` event.
- A second completion event does not perform a second automatic rename.
