# linea

A status line for Claude Code: model, effort, project, branch, herdr pane, session time, prompt cache and context use, fitted to the terminal width.

![linea in a dark terminal](docs/preview/dark.png)
![linea in a light terminal](docs/preview/light.png)

On a narrow terminal the lower-ranked sections give way and the branch is cut short:

![linea in a narrow terminal](docs/preview/narrow.png)

The icons come from [Nerd Fonts](https://www.nerdfonts.com) v3.

## Install

```sh
mise use -g github:srnnkls/linea
```

Then point Claude Code at it in `settings.json`:

```json
"statusLine": { "type": "command", "command": "linea" }
```

## Configuration

linea starts from its [built-in defaults](assets/config/default.toml) (`linea defaults` prints them) and merges these files over them, later ones winning:

1. `~/.config/linea/linea.toml`, or the file `LINEA_CONFIG` names
2. `linea.toml`, then `linea.local.toml`, in every directory from `/` down to the session's working directory

Tables merge key by key; any other value, arrays included, replaces the one beneath it. A broken configuration shows its error in place of the line.

```toml
[line]
sections = ["model", "effort", "project", "branch", "weather", "context"]
separator = "  "

[appearance]
mode = "auto"            # auto follows macOS (light elsewhere); dark or light pins a palette

[palette.dark]
teal = "#4db5bd"

[sections.effort]
enabled = false

[sections.branch]
colour = "teal"
fit = { rank = 3, grow = true, min = 16, requires = "project" }

[sections.cache]
format = "%H:%M"
```

A colour name has to exist in both palettes. A key that does not apply to its section, such as `format` outside `cache` or `fit.min` on a section that does not grow, is an error rather than ignored.

### Sections

The built-in sections are `model`, `effort`, `project`, `branch`, `pane`, `duration`, `cache`, `cache_misses` and `context`. Each takes:

| Key | Meaning |
| --- | --- |
| `icon` | Drawn before the text: a fixed string, or picked by a state (below); empty draws the text alone |
| `colour` | A palette name or a literal `#rrggbb` |
| `fit` | `"always"`, a rank, or `{ rank, grow, min, requires }` |
| `enabled` | `false` leaves the section out |
| `format` | strftime format for `cache` |
| `template` | A template that takes the place of the section's text |
| `run` | A hook command; see below |
| `emit` | `true` sends an [event](docs/events.md) when the section's text changes |

Sections that `fit = "always"` take their columns first. Ranked sections then claim theirs from the lowest rank up while they fit. One section may `grow`: it holds up to `min` columns at its rank and takes whatever is left once every rank is placed, cut short with an ellipsis. `requires` draws a section only when another one is drawn.

### Icons

An icon can follow a state instead of being fixed. `state` names a value by its path in the status JSON or under `linea` (the same names a template sees); `at` picks the icon of the highest threshold at or below a numeric state, `match` the icon for an exact value. `default` covers a state that is missing or matches no rule.

```toml
[sections.context.icon]
state = "context_window.used_percentage"
default = "○"
at = { 0 = "○", 25 = "◔", 50 = "◑", 75 = "◕", 100 = "●" }

[sections.effort.icon]
state = "linea.effort"
match = { low = "▁", medium = "▄", high = "█" }
default = "?"
```

### Templates

`template` is a [minijinja](https://docs.rs/minijinja) template (Jinja2 syntax) over the [status JSON](https://code.claude.com/docs/en/statusline#available-data) Claude Code sends. A section with only `template` can take any name; a result that is empty after trimming leaves the section out.

```toml
[sections.cost]
template = "${{ cost.total_cost_usd | round(2) }}"
icon = "\uF155"
colour = "green"
fit = 7

[sections.pr]
template = "{% if pr %}#{{ pr.number }} {{ pr.review_state }}{% endif %}"
colour = "blue"
fit = 8

[sections.limit]
template = "{% if rate_limits.five_hour %}{{ rate_limits.five_hour.used_percentage | round }}% until {{ rate_limits.five_hour.resets_at | time }}{% endif %}"
colour = "grey"
fit = 9
```

Besides the status fields, a template sees:

| Name | Value |
| --- | --- |
| `linea.<section>` | A built-in section's text, such as `linea.project` or `linea.branch` |
| `env.<NAME>` | An environment variable |
| `output` | The hook's output, in a section that also has `run` |

and three filters beside minijinja's own: `tokens` (`45200` → `45k`), `duration` (milliseconds → `HH:MM`) and `time` (epoch seconds → local time, `time("%d %b %H:%M")` for another format). A missing field renders empty, as do these three filters applied to one. A template that fails to render, such as minijinja's `round` applied to a missing field, leaves its section out; a template that does not parse shows its error in place of the line.

### Hooks

A section with `run` is a hook: a shell command whose first line of output is the section's text.

```toml
[sections.weather]
run = "curl -s 'wttr.in/?format=%t'"
icon = "\U000F0590"
colour = "grey"
fit = 6
refresh = "10m"
timeout = "3s"
```

The command runs through `sh -c` in the session's working directory, with Claude Code's status JSON on stdin and `LINEA_SECTION` set to the section's name. The line never waits on it: linea shows the output it last cached for the session and starts a detached refresh once that output is older than `refresh` (30 seconds unless set). A command that fails, or outlasts `timeout` (5 seconds unless set), hides the section until its next refresh. Output is cached under `$XDG_CACHE_HOME/linea/hooks`.

With `template` beside `run`, the template formats the hook's output:

```toml
[sections.weather]
run = "curl -s 'wttr.in/?format=%t'"
template = "{{ output | replace('+', '') }}"
```

### Events

Sections with `emit = true` (model and effort by default) send an event when their text changes: `events.body`, rendered as a template, goes to `events.command` on stdin, and the command acknowledges it by a marker in its output or by its exit status. By default the events go to limen, which shows the model and effort on the herdr pane. [docs/events.md](docs/events.md) describes the protocol: when an event goes out, what the command receives, and how acknowledgement and retries work.

### Integrations

```toml
# The pane section shows only inside a running herdr.
[herdr]
enabled = true
inside = "HERDR_ENV"
pane = "HERDR_PANE_ID"
socket = "HERDR_SOCKET_PATH"

# Where the effort level comes from when the session reports none.
[claude]
effort = "CLAUDE_CODE_EFFORT_LEVEL"
settings = "~/.claude/settings.json"   # unset follows CLAUDE_CONFIG_DIR
```

## Development

```sh
mise run check     # clippy, rustfmt, unit and scrut tests
mise run preview   # re-render docs/preview (needs ImageMagick); LINEA_PREVIEW_FONT and LINEA_PREVIEW_FAMILY pick the font
```
