# AGENTS.md

linea renders the Claude Code status line: it reads the session JSON on stdin and prints one line of coloured fields fitted to the terminal width.

## Layout

- `src/status.rs` parses the input into domain types, reading jq's `//` semantics: null and false are absent.
- `src/config.rs` loads `assets/config/default.toml` and the linea.toml files merged over it, and validates the result.
- `src/sections.rs` turns configured sections into parts: built-in texts from the session, hook texts through a callback.
- `src/line.rs` fits parts into the width by `fit` and draws them.
- `src/hooks.rs` serves hook output from a per-session cache and refreshes it in a detached `linea --hook` child.
- `src/host.rs` reads the machine: terminal width, repository (gix), settings, macOS appearance, herdr.
- `src/report.rs` sends model and effort changes to the configured command through a detached `linea --receipt` child.

Effects stay in `host`, `hooks` and `report`; `status`, `sections` and `line` take values and return values.

## Rules

- No subprocess on the render path. Git goes through gix, time through jiff, the terminal through rustix.
- `assets/config/default.toml` is the output contract: section order, glyphs, colours and fits. A change to it is a `feat` or `fix`, never a refactor.
- Anything tied to a machine or another tool belongs in the configuration, with its current behaviour as the default.
- Rust style: `henia show loqui.languages.rust`.

## Commands

```sh
mise run check   # clippy (pedantic, -D warnings), rustfmt check, tests
mise run build   # release binary at target/release/linea
mise run preview # re-render the README images in docs/preview
phora sync       # reference sources into resources/
```

## Releases

release-plz opens the release PR from conventional commits and tags `v{version}` on merge; cargo-dist builds the binaries and the GitHub release from the tag. Nothing is published to crates.io.
