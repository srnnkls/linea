# Run a hook

A section with `run` shows the first line its command printed. The line never waits for it: the first render starts a detached refresh, and the next one shows the cached output.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && mkdir -p app && cat > app/linea.toml <<'TOML'
> [line]
> sections = ["model", "greet"]
> [sections.greet]
> run = "printf '%s in %s\nsecond line\n' \"$(jq -r .session_id)\" \"$(basename \"$PWD\") $LINEA_SECTION\""
> icon = "G"
> colour = "grey"
> fit = 1
> TOML
```

```scrut
$ session "$PWD/app" | line
M Opus
```

```scrut
$ for i in $(seq 50); do [ -s cache/linea/hooks/greet.s1 ] && break; sleep 0.1; done; session "$PWD/app" | line
M Opus  G s1 in app greet
```

A template beside `run` formats the output.

```scrut
$ printf '[sections.greet]\ntemplate = "<{{ output }}>"\n' > app/linea.local.toml && session "$PWD/app" | line
M Opus  G <s1 in app greet>
```

A command that outlasts its timeout hides the section.

```scrut
$ mv app/linea.local.toml app/linea.local.off && cat >> app/linea.toml <<'TOML'
> [sections.slow]
> run = "sleep 5; echo late"
> timeout = "200ms"
> colour = "grey"
> fit = 2
> TOML
```

```scrut
$ sed -i.bak 's/sections = \["model", "greet"\]/sections = ["model", "slow"]/' app/linea.toml && session "$PWD/app" | line
M Opus
```

```scrut
$ for i in $(seq 50); do [ -e cache/linea/hooks/slow.s1 ] && break; sleep 0.1; done; wc -c < cache/linea/hooks/slow.s1 | tr -d ' '; session "$PWD/app" | line
0
M Opus
```
