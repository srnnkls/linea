# Run inside herdr

The pane section shows only inside a running herdr: `HERDR_ENV=1`, a pane id and an existing socket.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && touch sock
```

```scrut
$ session "$PWD/plain" | HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/sock" line
M Opus  P plain  H w1:p1  2 45k/200k (22%)
```

```scrut
$ session "$PWD/plain" | HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/gone" line
M Opus  P plain  2 45k/200k (22%)
```

```scrut
$ printf '[herdr]\npane = "MY_PANE"\n' > plain/linea.toml && session "$PWD/plain" | HERDR_ENV=1 MY_PANE=w2:p3 HERDR_SOCKET_PATH="$PWD/sock" line
M Opus  P plain  H w2:p3  2 45k/200k (22%)
```

Model and effort changes go to the report command once per change. It reads a StatusLine hook event on stdin; a reply carrying the `received` marker settles the change.

```scrut
$ mkdir -p bin && cat > bin/limen <<'SH'
> #!/bin/sh
> cat >> "$HOME/reports"; echo '{"received":true}'
> SH
> chmod +x bin/limen
```

```scrut
$ for i in 1 2; do session "$PWD/plain" '.effort = {level: "high"} | .transcript_path = "/t.jsonl"' | PATH="$PWD/bin:$PATH" HERDR_PANE_ID=w1:p1 linea > /dev/null; for j in $(seq 50); do grep -qs ' ok$' tmp/claude-statusline-model.s1 && break; sleep 0.1; done; done; cat tmp/claude-statusline-model.s1 reports
claude-opus/high ok
{"hook_event_name":"StatusLine","model":"claude-opus","effort":"high","session_id":"s1","transcript_path":"/t.jsonl","cwd":null}
```

Without its required variable, or disabled, nothing is sent.

```scrut
$ session "$PWD/plain" '.session_id = "s2"' | PATH="$PWD/bin:$PATH" linea > /dev/null; sleep 0.3; ls tmp
claude-statusline-model.s1
```

```scrut
$ printf '[report]\nenabled = false\n' > plain/linea.toml && session "$PWD/plain" '.session_id = "s3"' | PATH="$PWD/bin:$PATH" HERDR_PANE_ID=w1:p1 linea > /dev/null; sleep 0.3; ls tmp
claude-statusline-model.s1
```
