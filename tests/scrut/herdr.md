# Run inside herdr

The pane section shows only inside a running herdr: `HERDR_ENV=1`, a pane id and an existing socket.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

Events stay off here, so no render reaches a real limen; events.md covers them.

```scrut
$ mkdir app && touch sock && echo '{"model": {"display_name": "Opus"}, "workspace": {"current_dir": "app"}}' > session.json && cat > app/linea.toml <<'TOML'
> events.enabled = false
>
> [line]
> sections = ["model", "pane"]
>
> [sections.model]
> icon = "M"
>
> [sections.pane]
> icon = "H"
> TOML
```

```scrut
$ HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/sock" line < session.json
M Opus  H w1:p1
```

```scrut
$ HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/gone" line < session.json
M Opus
```

```scrut
$ HERDR_PANE_ID=w1:p1 HERDR_SOCKET_PATH="$PWD/sock" line < session.json
M Opus
```

`[herdr]` names the variables.

```scrut
$ printf 'herdr.pane = "MY_PANE"\n' > app/linea.local.toml && HERDR_ENV=1 MY_PANE=w2:p3 HERDR_SOCKET_PATH="$PWD/sock" line < session.json
M Opus  H w2:p3
```
