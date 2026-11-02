# Render the line

linea reads Claude Code's status JSON on stdin and draws one line. The model and the context use always show.

```scrut
$ . "$TESTDIR/_setup.sh" && isolate
```

```scrut
$ session "$PWD/plain" | line
M Opus  P plain  2 45k/200k (22%)
```

The effort level, the session time and the prompt cache join it when the session reports them.

```scrut
$ session "$PWD/plain" '.effort = {level: "high"} | .cost.total_duration_ms = 3725000 | .prompt_cache = {caching_observed: true, expires_at: 1761940800, misses: 3}' | line
M Opus  E high  P plain  D 01:02  C 31 Oct 20:00  X 3  2 45k/200k (22%)
```

Inside a repository the project is the repository's name and the branch follows it.

```scrut
$ make_repo dotfiles main && session "$PWD/dotfiles" | line
M Opus  P dotfiles  B main  2 45k/200k (22%)
```

A linked worktree still shows the main repository's name.

```scrut
$ git -C dotfiles worktree add -q -b feature/x ../wt && session "$PWD/wt" | line
M Opus  P dotfiles  B feature/x  2 45k/200k (22%)
```

On a narrow terminal the lower ranks give way and the branch is cut short.

```scrut
$ git -C dotfiles checkout -q -b feature/a-rather-long-branch-name && session "$PWD/dotfiles" '.cost.total_duration_ms = 3725000' | COLUMNS=60 line
M Opus  P dotfiles  B feature/a-rathe…  2 45k/200k (22%)
```

```scrut
$ session "$PWD/dotfiles" | COLUMNS=30 line
M Opus  2 45k/200k (22%)
```

The context icon fills by eighths, full only at 100%.

```scrut
$ for pct in 0 49 99 100; do session "$PWD/plain" ".context_window.used_percentage = $pct" | line; done
M Opus  P plain  0 45k/200k (0%)
M Opus  P plain  4 45k/200k (49%)
M Opus  P plain  7 45k/200k (99%)
M Opus  P plain  8 45k/200k (100%)
```

Without a session effort, the level comes from `CLAUDE_CODE_EFFORT_LEVEL`, then from Claude Code's settings.

```scrut
$ session "$PWD/plain" | CLAUDE_CODE_EFFORT_LEVEL=low line
M Opus  E low  P plain  2 45k/200k (22%)
```

```scrut
$ mkdir -p .claude && echo '{"effortLevel": "medium"}' > .claude/settings.json && session "$PWD/plain" | line
M Opus  E medium  P plain  2 45k/200k (22%)
```
