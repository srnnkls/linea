# Configure the line

linea merges the global linea.toml, then `linea.toml` and `linea.local.toml` from `/` down to the session's directory, over its built-in defaults.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && mkdir -p work/app
```

A project file reorders and hides sections; arrays replace, tables merge.

```scrut
$ printf '[line]\nsections = ["context", "model"]\n' > work/linea.toml && session "$PWD/work/app" | line
2 45k/200k (22%)  M Opus
```

A local file in a deeper directory wins over the files above it.

```scrut
$ printf '[sections.model]\nicon = "m"\n' > work/app/linea.local.toml && session "$PWD/work/app" | line
2 45k/200k (22%)  m Opus
```

```scrut
$ printf '[sections.context]\nenabled = false\n' > work/app/linea.toml && session "$PWD/work/app" | line
m Opus
```

An empty icon draws the text alone.

```scrut
$ printf '[sections.model]\nicon = ""\n' > work/app/linea.local.toml && session "$PWD/work/app" | line
Opus
```

`LINEA_CONFIG` names the global file.

```scrut
$ printf '[line]\nsections = ["model"]\n[sections.model]\nicon = "G"\n' > other.toml && session "$PWD/plain" | LINEA_CONFIG="$PWD/other.toml" line
G Opus
```

Colours come from the palette for the appearance, or from a literal.

```scrut
$ printf '[line]\nsections = ["model"]\n[sections.model]\ncolour = "#102030"\n' > plain/linea.toml && session "$PWD/plain" | linea | cat -v | cut -d m -f 1
^[[38;2;16;32;48
```

```scrut
$ printf '[appearance]\nmode = "light"\n[line]\nsections = ["model"]\n' > plain/linea.toml && session "$PWD/plain" | linea | cat -v | cut -d m -f 1
^[[38;2;215;119;87
```

A broken configuration shows its error in place of the line.

```scrut
$ printf '[line]\nsections = ["model", "nope"]\n' > plain/linea.toml && session "$PWD/plain" | line
linea: line.sections names `nope`, which no [sections.nope] defines
```

```scrut
$ printf '[sections.model]\ncolour = "mauve"\n' > plain/linea.toml && session "$PWD/plain" | line
linea: sections.model uses colour `mauve`, which a palette lacks
```

```scrut
$ printf '[sections.model]\nfit = "never"\n' > plain/linea.toml && session "$PWD/plain" | line
linea: linea.toml: fit is "always", a rank or a table, not `never`
```

`linea defaults` prints the built-in configuration.

```scrut
$ linea defaults | grep -c '^\[sections\.'
9
```
