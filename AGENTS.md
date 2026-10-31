# AGENTS.md

linea renders the Claude Code status line: it reads the session JSON on stdin and prints one line of coloured fields fitted to the terminal width.

## Layout

- `src/status.rs` parses the input into domain types, reading jq's `//` semantics: null and false are absent.
- `src/line.rs` is the pure core: palette, fields and the width budget that decides which fields show.
- `src/host.rs` reads the machine: terminal width, repository (gix), settings, macOS appearance.
- `src/report.rs` sends model and effort changes to limen through a detached `linea --receipt` child.

Effects stay in `host` and `report`; `status` and `line` take values and return values.

## Rules

- No subprocess on the render path. Git goes through gix, time through jiff, the terminal through rustix.
- Field order, glyphs, colours and the fitting rules are the output contract; a change to any of them is a `feat` or `fix`, never a refactor.
- Rust style: `henia show loqui.languages.rust`.

## Commands

```sh
mise run check   # clippy (pedantic, -D warnings), rustfmt check, tests
mise run build   # release binary at target/release/linea
phora sync       # reference sources into resources/
```

## Releases

release-plz opens the release PR from conventional commits and tags `v{version}` on merge; cargo-dist builds the binaries and the GitHub release from the tag. Nothing is published to crates.io.
