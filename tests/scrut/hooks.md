# Run a hook

A section with `run` shows the first line its command prints. The line never waits for it: the first render starts a detached refresh, and later renders show the cached output.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

```scrut
$ mkdir app && echo '{"model": {"display_name": "Opus"}, "session_id": "s1", "workspace": {"current_dir": "app"}}' > session.json
```

The command gets the status JSON on stdin, runs in the session's directory and sees `LINEA_SECTION`.

```scrut
$ cat > app/linea.toml <<'TOML'
> [line]
> sections = ["model", "greet"]
>
> [sections.model]
> icon = "M"
>
> [sections.greet]
> run = "printf '%s in %s as %s\nsecond line\n' \"$(jq -r .session_id)\" \"$(basename \"$PWD\")\" \"$LINEA_SECTION\""
> icon = "G"
> colour = "grey"
> fit = 1
> TOML
```

```scrut
$ line < session.json
M Opus
```

```scrut
$ until [ -s cache/linea/hooks/greet.s1 ]; do sleep 0.1; done; line < session.json
M Opus  G s1 in app as greet
```

A template beside `run` formats the output.

```scrut
$ printf 'sections.greet.template = "<{{ output }}>"\n' > app/linea.local.toml && line < session.json
M Opus  G <s1 in app as greet>
```

A command that outlasts its timeout caches nothing to show, and its section stays out.

```scrut
$ cat > app/linea.local.toml <<'TOML'
> [line]
> sections = ["model", "slow"]
>
> [sections.slow]
> run = "sleep 5; echo late"
> timeout = "200ms"
> colour = "grey"
> fit = 2
> TOML
```

```scrut
$ line < session.json
M Opus
```

```scrut
$ until [ -e cache/linea/hooks/slow.s1 ]; do sleep 0.1; done; wc -c < cache/linea/hooks/slow.s1 | tr -d ' '; line < session.json
0
M Opus
```

A command that fails caches nothing to show either.

```scrut
$ printf '[line]\nsections = ["model", "broken"]\n[sections.broken]\nrun = "echo partial; exit 1"\ncolour = "grey"\nfit = 1\n' > app/linea.local.toml && line < session.json
M Opus
```

```scrut
$ until [ -e cache/linea/hooks/broken.s1 ]; do sleep 0.1; done; wc -c < cache/linea/hooks/broken.s1 | tr -d ' '; line < session.json
0
M Opus
```
