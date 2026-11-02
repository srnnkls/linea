# Run inside herdr

The pane section shows only inside a running herdr: `HERDR_ENV=1`, a pane id and an existing socket.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

Model and effort changes go to a report command whenever `HERDR_PANE_ID` is set: limen by default. A stand-in on `PATH` logs what it reads and replies with the `received` marker.

```scrut
$ mkdir bin && cat > bin/limen <<'SH'
> #!/bin/sh
> cat >> "$HOME/reports"
> echo '{"received":true}'
> SH
> chmod +x bin/limen && export PATH="$PWD/bin:$PATH"
```

```scrut
$ mkdir app && touch sock && cat > session.json <<'JSON'
> {
>   "model": {"display_name": "Opus", "id": "claude-opus"},
>   "session_id": "s1",
>   "transcript_path": "/t.jsonl",
>   "effort": {"level": "high"},
>   "workspace": {"current_dir": "app"}
> }
> JSON
```

```scrut
$ printf '[line]\nsections = ["model", "pane"]\n[sections.model]\nicon = "M"\n[sections.pane]\nicon = "H"\n' > app/linea.toml
```

```scrut
$ HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/sock" line < session.json
M Opus  H w1:p1
```

```scrut
$ HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/gone" line < session.json
M Opus
```

`[herdr]` names the variables.

```scrut
$ printf 'herdr.pane = "MY_PANE"\n' > app/linea.local.toml && HERDR_ENV=1 MY_PANE=w2:p3 HERDR_SOCKET_PATH="$PWD/sock" line < session.json
M Opus  H w2:p3
```

The report goes out once per change: the pane renders above already sent it, and the stand-in took it.

```scrut
$ until grep -qs ' ok$' tmp/claude-statusline-model.s1; do sleep 0.1; done; HERDR_PANE_ID=w1:p1 linea < session.json > /dev/null; sleep 0.3; cat tmp/claude-statusline-model.s1 reports
claude-opus/high ok
{"hook_event_name":"StatusLine","model":"claude-opus","effort":"high","session_id":"s1","transcript_path":"/t.jsonl","cwd":null}
```

Without its required variable, or disabled, nothing is sent.

```scrut
$ jq '.session_id = "s2"' session.json | linea > /dev/null; sleep 0.3; ls tmp
claude-statusline-model.s1
```

```scrut
$ printf 'report.enabled = false\n' > app/linea.local.toml && jq '.session_id = "s3"' session.json | HERDR_PANE_ID=w1:p1 linea > /dev/null; sleep 0.3; ls tmp
claude-statusline-model.s1
```
