# Agent status lifecycle

## Goal

Automatic tab naming must run exactly once after the first real agent response
to a user prompt in an eligible tab. Starting an agent is not itself evidence
of a user task, so an initialization-time `idle` event must never rename the
tab.

## Event identity

The plugin accepts both Herdr event spellings:

- `pane.agent_status_changed`
- `pane_agent_status_changed`

Event payloads are not required to carry a `tab_id`. In that case, the plugin
resolves the tab from the current Herdr snapshot using `pane_id`. The same
resolution rule applies to both `working` and completion events; otherwise the
state machine can observe a completion without recording its preceding work.

## State machine

The persisted state is scoped by tab ID and has two relevant flags:

- `saw_working`: a `working` event was observed for the tab.
- `auto_after_done`: the one automatic naming attempt for the tab has already
  been made.

Prompt evidence is read from the focused agent session at completion time. It
requires at least one non-empty user message. The plugin intentionally skips
automatic naming when the session cannot provide that evidence; the explicit
`rename-now` action remains available in that case.

| Incoming status | Required evidence | Action |
| --- | --- | --- |
| `working` | Resolvable tab ID | Set `saw_working`; do not rename. |
| `done` / `idle` | `saw_working`, or an explicit prior status of `working` / `blocked`; and a user prompt is present | Attempt automatic naming once. |
| `done` / `idle` | No preceding work evidence, or no user prompt | Ignore the event without consuming the one automatic attempt. |
| Any other status | None | Ignore the event. |

An absent prior status is not completion evidence. This is essential because
Herdr may emit an initial `idle` event while it creates an agent pane, before
the user submits a prompt.

## Naming guardrails

After completion evidence is established, automatic naming still runs only
when the tab retains a default label. A user-provided label remains authoritative.
The explicit `rename-now` action is independent of this lifecycle and may
rename a manually named tab because it is directly user initiated.

## Required regression coverage

- An initial `idle` event with no earlier `working` event does not rename.
- An initialization `working` → `idle` sequence with no user prompt does not
  rename or consume the later automatic attempt.
- A serialized event sequence containing only `pane_id` performs one rename
  for `working` followed by `idle`.
- A completion event with explicit `old_status = working` can rename even if
  the plugin did not receive the earlier `working` event.
- A second completion event does not perform a second automatic rename.
