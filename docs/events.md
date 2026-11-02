# Events

linea can tell another program when part of the status line changes. Sections marked `emit = true` make up a state; when that state changes, linea sends an event to the events command. By default the model and effort sections emit to [limen](https://github.com/srnnkls/limen), which shows them on the herdr pane: Claude Code's own hook events carry no effort level.

```toml
[events]
enabled = true
command = ["limen", "hook", "claude"]
body = '''{"hook_event_name":"StatusLine","model":{{ model.id | default("") | tojson }}, …}'''
acknowledge = { marker = '"received":true' }
retry = "30s"
requires = "HERDR_PANE_ID"

[sections.model]
emit = true
```

## The state

The state is the text of every emitting section that `line.sections` lists, in drawing order, joined with `/`: `Opus 5.5/high`. A section counts even when the line is too narrow to draw it; one with nothing to show counts as empty. Without an emitting section there is no state and no event.

## When an event goes out

On every render, linea sends an event when all of these hold:

1. `events.enabled` is true.
2. The variable `events.requires` names is set and not empty.
3. Claude Code reported a `session_id`.
4. The first word of `events.command` is an executable on `PATH`.
5. The record for the session holds a different state, or holds this state unacknowledged and older than `retry`.

The record is `$TMPDIR/linea-events.<session_id>`, one line:

| Line | Meaning |
| --- | --- |
| `<state> ok` | The command acknowledged this state; it is not sent again. |
| `<state> sent <epoch seconds>` | Sent and not acknowledged; sent again once `retry` has passed. |

linea writes `<state> sent <now>` before it sends, so concurrent renders do not send twice.

## What the command receives

linea starts the command in the background, detached from the render: the line never waits for it. The command's stdin carries `events.body`, rendered as a [template](../README.md#templates) over the status JSON, the built-in section texts under `linea` and the environment under `env`, followed by a newline. Its stderr is discarded.

The default body is the StatusLine hook event limen reads:

```json
{"hook_event_name":"StatusLine","model":"claude-opus-5-5","effort":"high","session_id":"…","transcript_path":"…","cwd":"…"}
```

`effort` is empty while the level is Claude Code's default.

## Acknowledgement

The command acknowledges an event in one of two ways, set by `events.acknowledge`:

| `acknowledge` | Acknowledged when |
| --- | --- |
| `{ marker = "…" }` | the command's stdout contains the marker |
| `"exit"` | the command exits with status zero |

An acknowledged event turns the record into `<state> ok`. An unacknowledged one, including a command that fails to start, leaves `<state> sent <epoch>`, and the first render after `retry` sends it again.
