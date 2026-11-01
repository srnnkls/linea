# linea

A status line for Claude Code: model, effort, project, branch, session time, prompt cache and context use, fitted to the terminal width.

## Install

```sh
mise use -g github:srnnkls/linea
```

Then point Claude Code at it in `settings.json`:

```json
"statusLine": { "type": "command", "command": "linea" }
```

## Behaviour

Model and context use always show. Project and effort come next, then the branch, then the herdr pane, session time and cache expiry while they fit; the branch takes the room left over. The pane shows only inside a running herdr. Colours follow the macOS appearance.

Inside a herdr pane with `limen` on `PATH`, a change of model or effort is reported through `limen hook claude`.
