# Sentry drill

A project an already built game runs, with no Rust: its sentry turret fires
every 0.25 s instead of every 0.55 s.

```bash
cargo build -p ambition_app            # once: the host
./target/debug/ambition_app --project examples/projects/sentry_drill
```

The run starts in the central hub. Go down to its basement, pick up the
sentry gauntlet and press Attack to deploy a turret. Edit `modules/sentry.rhai`
while the game runs: the turret takes the edit with no build. A script that
does not parse, or that faults, leaves the running code in place and says
which line.

`project.ron` names:

| field | what it is |
|---|---|
| `schema` | `"ambition.project.v1"` |
| `name` | the project's name |
| `content` | optional: a content pack root, a directory with its own `pack.ron` |
| `modules` | `.rhai` sources (or `.wasm` builds), or directories of them; each replaces the linked module its file names |
| `entry_room` | optional: the room the run starts in, a runtime room id; `--start-room` beside `--project` wins |

Paths are relative to `project.ron`. The host's adapter is
`game/ambition_app/src/app/project.rs`.
