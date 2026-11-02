# Template a section

A section's `template` renders Claude Code's status JSON, the built-in texts under `linea` and the environment under `env`.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && make_repo app main && cat > app/linea.toml <<'TOML'
> [line]
> sections = ["model", "cost", "pr", "limit"]
> [sections.cost]
> template = "${{ cost.total_cost_usd | round(2) }} on {{ linea.branch }}"
> colour = "green"
> fit = 1
> [sections.pr]
> template = "{% if pr %}#{{ pr.number }} {{ pr.review_state }}{% endif %}"
> icon = "#"
> colour = "blue"
> fit = 2
> [sections.limit]
> template = "{% if rate_limits.five_hour %}{{ rate_limits.five_hour.used_percentage }}% until {{ rate_limits.five_hour.resets_at | time }}{% endif %}"
> colour = "grey"
> fit = 3
> TOML
```

```scrut
$ session "$PWD/app" '.cost.total_cost_usd = 1.2345' | line
M Opus  $1.23 on main
```

```scrut
$ session "$PWD/app" '.cost.total_cost_usd = 1.2345 | .pr = {number: 42, review_state: "approved"} | .rate_limits.five_hour = {used_percentage: 37, resets_at: 1761940800}' | line
M Opus  $1.23 on main  # #42 approved  37% until 20:00
```

linea's filters format tokens, durations and times.

```scrut
$ printf '[line]\nsections = ["t"]\n[sections.t]\ntemplate = "{{ context_window.total_input_tokens | tokens }} {{ cost.total_duration_ms | duration }} {{ 1761940800 | time(\\"%%d %%b\\") }} {{ env.HOME == \\"%s\\" }}"\ncolour = "grey"\nfit = 1\n' "$HOME" > app/linea.local.toml && session "$PWD/app" '.cost.total_duration_ms = 3725000' | line
45k 01:02 31 Oct True
```

A template that fails to render leaves its section out; one that does not parse shows its error.

```scrut
$ printf '[line]\nsections = ["model", "t"]\n[sections.t]\ntemplate = "{{ missing | round }}"\ncolour = "grey"\nfit = 1\n' > app/linea.local.toml && session "$PWD/app" | line
M Opus
```

```scrut
$ printf '[line]\nsections = ["t"]\n[sections.t]\ntemplate = "{{ cost."\ncolour = "grey"\nfit = 1\n' > app/linea.local.toml && session "$PWD/app" | line
linea: sections.t.template: syntax error: unexpected end of input, expected identifier or integer (in <string>:1)
```
