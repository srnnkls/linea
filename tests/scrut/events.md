# Send events

When the text of a section marked `emit` changes, linea renders `events.body` and hands it to `events.command` on stdin, once per change. The defaults emit the model and the effort to limen while `HERDR_PANE_ID` is set.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate
```

A stand-in for limen logs what it reads and acknowledges with limen's marker.

```scrut
$ mkdir bin && cat > bin/limen <<'SH'
> #!/bin/sh
> cat >> "$HOME/events"
> echo '{"received":true}'
> SH
> chmod +x bin/limen && export PATH="$PWD/bin:$PATH"
```

```scrut
$ mkdir app && cat > session.json <<'JSON'
> {
>   "model": {"display_name": "Opus", "id": "claude-opus"},
>   "session_id": "s1",
>   "transcript_path": "/t.jsonl",
>   "effort": {"level": "high"},
>   "workspace": {"current_dir": "app"}
> }
> JSON
```

The first render sends the state; the record holds it once acknowledged.

```scrut
$ HERDR_PANE_ID=w1:p1 linea < session.json > /dev/null; until grep -qs ' ok$' tmp/linea-events.s1; do sleep 0.1; done; cat tmp/linea-events.s1 events
Opus/high ok
{"hook_event_name":"StatusLine","model":"claude-opus","effort":"high","session_id":"s1","transcript_path":"/t.jsonl","cwd":null}
```

The same state is not sent again; a changed one is.

```scrut
$ HERDR_PANE_ID=w1:p1 linea < session.json > /dev/null; sleep 0.3; wc -l < events | tr -d ' '
1
```

```scrut
$ jq '.effort.level = "low"' session.json | HERDR_PANE_ID=w1:p1 linea > /dev/null; until grep -qs '^Opus/low ok$' tmp/linea-events.s1; do sleep 0.1; done; tail -n 1 events | jq -r .effort
low
```

Without the required variable, or with events disabled, nothing is sent.

```scrut
$ jq '.session_id = "s2"' session.json | linea > /dev/null; sleep 0.3; ls tmp
linea-events.s1
```

```scrut
$ printf 'events.enabled = false\n' > app/linea.local.toml && jq '.session_id = "s3"' session.json | HERDR_PANE_ID=w1:p1 linea > /dev/null; sleep 0.3; ls tmp
linea-events.s1
```

Any section can emit, with its own command and body. Acknowledged by exit status, a command that fails is retried after `retry`.

```scrut
$ cat > bin/notify <<'SH'
> #!/bin/sh
> cat >> "$HOME/notified"
> [ ! -e "$HOME/refuse" ]
> SH
> chmod +x bin/notify && touch refuse && cat > app/linea.local.toml <<'TOML'
> [events]
> command = ["notify"]
> body = "{{ linea.project }} {{ cost.total_cost_usd }}"
> acknowledge = "exit"
> retry = "1s"
> requires = "NOTIFY"
>
> [line]
> sections = ["model", "cost", "project"]
>
> [sections.model]
> emit = false
>
> [sections.cost]
> template = "{{ cost.total_cost_usd }}"
> colour = "grey"
> fit = 1
> emit = true
> TOML
```

```scrut
$ jq '.session_id = "s4" | .cost.total_cost_usd = 2' session.json | NOTIFY=1 linea > /dev/null; until [ -s notified ]; do sleep 0.1; done; sleep 0.2; cat notified tmp/linea-events.s4 | cut -d ' ' -f 1,2
app 2
2 sent
```

```scrut
$ rm refuse && sleep 1 && jq '.session_id = "s4" | .cost.total_cost_usd = 2' session.json | NOTIFY=1 linea > /dev/null; until grep -qs ' ok$' tmp/linea-events.s4; do sleep 0.1; done; cat tmp/linea-events.s4; wc -l < notified | tr -d ' '
2 ok
2
```
