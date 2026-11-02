# Template a section

A section's `template` renders Claude Code's status JSON, the built-in section texts under `linea` and the environment under `env`.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

```scrut
$ git init -q -b main app && git -C app -c user.name=T -c user.email=t@t -c commit.gpgsign=false commit -q --allow-empty -m init
```

```scrut
$ cat > session.json <<'JSON'
> {
>   "model": {"display_name": "Opus"},
>   "workspace": {"current_dir": "app"},
>   "cost": {"total_cost_usd": 1.2345, "total_duration_ms": 3725000},
>   "context_window": {"total_input_tokens": 45200}
> }
> JSON
```

```scrut
$ cat > app/linea.toml <<'TOML'
> [line]
> sections = ["model", "cost", "pr", "limit"]
>
> [sections.model]
> icon = "M"
>
> [sections.cost]
> template = "${{ cost.total_cost_usd | round(2) }} on {{ linea.branch }}"
> colour = "green"
> fit = 1
>
> [sections.pr]
> template = "{% if pr %}#{{ pr.number }} {{ pr.review_state }}{% endif %}"
> icon = "#"
> colour = "blue"
> fit = 2
>
> [sections.limit]
> template = "{% if rate_limits.five_hour %}{{ rate_limits.five_hour.used_percentage }}% until {{ rate_limits.five_hour.resets_at | time }}{% endif %}"
> colour = "grey"
> fit = 3
> TOML
```

A section whose template renders empty is left out.

```scrut
$ line < session.json
M Opus  $1.23 on main
```

```scrut
$ jq '.pr = {number: 42, review_state: "approved"} | .rate_limits.five_hour = {used_percentage: 37, resets_at: 1761940800}' session.json | line
M Opus  $1.23 on main  # #42 approved  37% until 20:00
```

linea's filters format tokens, durations and times; `env` holds the environment.

```scrut
$ cat > app/linea.local.toml <<'TOML'
> [line]
> sections = ["t"]
>
> [sections.t]
> template = "{{ context_window.total_input_tokens | tokens }} {{ cost.total_duration_ms | duration }} {{ 1761940800 | time('%d %b') }} {{ env.TZ }}"
> colour = "grey"
> fit = 1
> TOML
```

```scrut
$ line < session.json
45k 01:02 31 Oct UTC
```

A template that fails to render leaves its section out; one that does not parse shows its error.

```scrut
$ printf '[line]\nsections = ["model", "t"]\n[sections.t]\ntemplate = "{{ missing | round }}"\ncolour = "grey"\nfit = 1\n' > app/linea.local.toml && line < session.json
M Opus
```

```scrut
$ printf '[line]\nsections = ["t"]\n[sections.t]\ntemplate = "{{ cost."\ncolour = "grey"\nfit = 1\n' > app/linea.local.toml && line < session.json
linea: sections.t.template: syntax error: unexpected end of input, expected identifier or integer (in <string>:1)
```
