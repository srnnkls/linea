# AGENTS.md

linea renders the Claude Code status line: it reads the session JSON on stdin and prints one line of coloured fields fitted to the terminal width.

## Architecture

Functional core, imperative shell. `main` is the composition root: it reads each effect once into a domain value (`Terminal`, `Workspace`, `Pane`, `HookOutputs`, the session's effective effort), then the line is a pure function of those values, the session and the configuration. Input is parsed into domain types at the boundary: the status JSON into `Session`, linea.toml into a `Config` whose sum types (`Source`, `Fit`, `Icon`, `Acknowledgement`) leave no invalid combination to check later, messages to detached children into typed structs.

## Rules

- No subprocess on the render path. Git goes through gix, time through jiff, the terminal through rustix.
- `assets/config/default.toml` is the output contract: section order, icons, colours and fits. A change to it is a `feat` or `fix`, never a refactor.
- Anything tied to a machine or another tool belongs in the configuration, with its current behaviour as the default.
- Rust style: `henia show loqui.languages.rust`.

## Commands

```sh
mise run check   # clippy (pedantic, -D warnings), rustfmt check, unit and scrut tests
mise run test-integration  # tests/scrut: end-to-end runs of the binary
mise run build   # release binary at target/release/linea
mise run preview # re-render the README images in docs/preview
phora sync       # reference sources into resources/
```

## Releases

release-plz opens the release PR from conventional commits and tags `v{version}` on merge; cargo-dist builds the binaries and the GitHub release from the tag. Nothing is published to crates.io.
