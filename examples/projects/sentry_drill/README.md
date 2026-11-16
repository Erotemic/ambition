# Sentry drill

A project an already built game runs, with no Rust: its sentry turret fires
every 0.25 s instead of every 0.55 s.

```bash
cargo build -p ambition_app            # once: the host
./target/debug/ambition_app --project examples/projects/sentry_drill
```

The run starts in the central hub. Go down to its basement, pick up the
sentry gauntlet and press Attack to deploy a turret. Edit `modules/sentry.rhai`
while the game runs: the turret takes the edit with no build. Taking the
script out of `modules` in `project.ron` while the game runs puts the
game's own turret back; putting it in again loads it. A script that
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

The tools take the project too:

```bash
cargo run -p ambition_app_tools --bin extension_inspect -- --project examples/projects/sentry_drill
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player shot.png --project examples/projects/sentry_drill
cargo run -p ambition_app_tools --bin inspect_geometry -- --project examples/projects/sentry_drill --room central_hub_complex
```

Paths are relative to `project.ron`. The host's adapter is
`game/ambition_app/src/app/project.rs`.
