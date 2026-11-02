# Configure the line

linea merges the global `linea.toml`, then `linea.toml` and `linea.local.toml` in every directory from `/` down to the session's, over its built-in defaults.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

```scrut
$ mkdir -p work/app && cat > session.json <<'JSON'
> {
>   "model": {"display_name": "Opus"},
>   "workspace": {"current_dir": "work/app"},
>   "context_window": {"total_input_tokens": 45200, "used_percentage": 22}
> }
> JSON
```

The global file sets the icons for everything below.

```scrut
$ mkdir -p config/linea && cat > config/linea/linea.toml <<'TOML'
> sections.model.icon = "M"
> sections.project.icon = "P"
> sections.context.icon = "%"
> TOML
```

```scrut
$ line < session.json
M Opus  P app  % 45k/200k (22%)
```

A project file reorders the sections; arrays replace, tables merge.

```scrut
$ printf '[line]\nsections = ["context", "model"]\n' > work/linea.toml && line < session.json
% 45k/200k (22%)  M Opus
```

A file in a deeper directory wins, and `linea.local.toml` wins over `linea.toml` beside it.

```scrut
$ printf 'sections.context.enabled = false\n' > work/app/linea.toml && printf 'sections.model.icon = "m"\n' > work/app/linea.local.toml && line < session.json
m Opus
```

An empty icon draws the text alone.

```scrut
$ printf 'sections.model.icon = ""\n' > work/app/linea.local.toml && line < session.json
Opus
```

`LINEA_CONFIG` names the global file in place of the XDG one.

```scrut
$ printf '[line]\nsections = ["model"]\n[sections.model]\nicon = "G"\n' > other.toml && jq '.workspace.current_dir = "."' session.json | LINEA_CONFIG="$PWD/other.toml" line
G Opus
```

Colours come from the palette for the appearance, or from a literal `#rrggbb`.

```scrut
$ printf '[line]\nsections = ["model"]\n[sections.model]\ncolour = "#102030"\n' > work/app/linea.local.toml && linea < session.json | cat -v | cut -d m -f 1
^[[38;2;16;32;48
```

```scrut
$ printf '[appearance]\nmode = "light"\n[line]\nsections = ["project"]\n' > work/app/linea.local.toml && linea < session.json | cat -v | cut -d m -f 1
^[[38;2;95;130;196
```

A broken configuration shows its error in place of the line.

```scrut
$ printf '[line]\nsections = ["model", "nope"]\n' > work/app/linea.local.toml && line < session.json
linea: line.sections names `nope`, which no [sections.nope] defines
```

```scrut
$ printf 'sections.model.colour = "mauve"\n' > work/app/linea.local.toml && line < session.json
linea: sections.model uses colour `mauve`, which a palette lacks
```

```scrut
$ printf 'sections.model.fit = "never"\n' > work/app/linea.local.toml && line < session.json
linea: linea.toml: fit is "always", a rank or a table, not `never`
```

A key outside the section it belongs to is an error, not ignored.

```scrut
$ printf 'sections.model.format = "%%H"\n' > work/app/linea.local.toml && line < session.json
linea: sections.model.format applies only to the cache section
```

`linea defaults` prints the configuration linea starts from: as the only file, it draws the same line.

```scrut
$ rm work/app/linea.local.toml work/app/linea.toml work/linea.toml && linea defaults > defaults.toml && [ "$(XDG_CONFIG_HOME="$PWD/none" linea < session.json)" = "$(LINEA_CONFIG="$PWD/defaults.toml" linea < session.json)" ] && echo same
same
```

Anything else on the command line is an error.

```scrut
$ linea --nope 2>&1
error: unexpected argument '--nope' found
* (glob+)
[2]
```
