# Render the line

linea reads Claude Code's status JSON on stdin and draws one line, fitted to `COLUMNS`.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate && line() { linea | sed 's/\x1b\[[0-9;]*m//g'; }
```

ASCII icons keep the expected lines readable; the context icon steps through the digits.

```scrut
$ mkdir -p config/linea && cat > config/linea/linea.toml <<'TOML'
> sections.model.icon = "M"
> sections.effort.icon = "E"
> sections.project.icon = "P"
> sections.branch.icon = "B"
> sections.duration.icon = "D"
> sections.cache.icon = "C"
> sections.cache_misses.icon = "X"
> sections.context.icons = ["0", "1", "2", "3", "4", "5", "6", "7", "8"]
> TOML
```

```scrut
$ mkdir plain && cat > session.json <<'JSON'
> {
>   "model": {"display_name": "Opus", "id": "claude-opus"},
>   "session_id": "s1",
>   "workspace": {"current_dir": "plain"},
>   "context_window": {"total_input_tokens": 45200, "context_window_size": 200000, "used_percentage": 22}
> }
> JSON
```

The model and the context use always show; outside a repository the project is the directory's name.

```scrut
$ line < session.json
M Opus  P plain  2 45k/200k (22%)
```

The effort level, the session time and the prompt cache join it when the session reports them.

```scrut
$ jq '.effort = {level: "high"} | .cost.total_duration_ms = 3725000 | .prompt_cache = {caching_observed: true, expires_at: 1761940800, misses: 3}' session.json | line
M Opus  E high  P plain  D 01:02  C 31 Oct 20:00  X 3  2 45k/200k (22%)
```

Inside a repository the project is the repository's name, and the branch follows it. A linked worktree still shows the main repository's name.

```scrut
$ git init -q -b main dotfiles && git -C dotfiles -c user.name=T -c user.email=t@t -c commit.gpgsign=false commit -q --allow-empty -m init
```

```scrut
$ jq '.workspace.current_dir = "dotfiles"' session.json | line
M Opus  P dotfiles  B main  2 45k/200k (22%)
```

```scrut
$ git -C dotfiles worktree add -q -b feature/x ../wt && jq '.workspace.current_dir = "wt"' session.json | line
M Opus  P dotfiles  B feature/x  2 45k/200k (22%)
```

On a narrow terminal the lower ranks give way and the branch is cut short; narrower still, the project goes too.

```scrut
$ git -C dotfiles checkout -q -b feature/a-rather-long-branch-name && jq '.workspace.current_dir = "dotfiles" | .cost.total_duration_ms = 3725000' session.json | COLUMNS=60 line
M Opus  P dotfiles  B feature/a-rathe…  2 45k/200k (22%)
```

```scrut
$ jq '.workspace.current_dir = "dotfiles"' session.json | COLUMNS=30 line
M Opus  2 45k/200k (22%)
```

The context icon fills by eighths, full only at 100%.

```scrut
$ for pct in 0 49 99 100; do jq ".context_window.used_percentage = $pct" session.json | line; done
M Opus  P plain  0 45k/200k (0%)
M Opus  P plain  4 45k/200k (49%)
M Opus  P plain  7 45k/200k (99%)
M Opus  P plain  8 45k/200k (100%)
```

Without a session effort, the level comes from `CLAUDE_CODE_EFFORT_LEVEL`, then from Claude Code's settings.

```scrut
$ CLAUDE_CODE_EFFORT_LEVEL=low line < session.json
M Opus  E low  P plain  2 45k/200k (22%)
```

```scrut
$ mkdir .claude && echo '{"effortLevel": "medium"}' > .claude/settings.json && line < session.json
M Opus  E medium  P plain  2 45k/200k (22%)
```
