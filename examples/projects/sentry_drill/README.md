# Sentry drill

A project an already built game runs, with no Rust: its sentry turret fires
every 0.25 s instead of every 0.55 s.

```bash
cargo build -p ambition_app --bin ambition_game_bin  # once: the host
./target/debug/ambition_game_bin --project examples/projects/sentry_drill
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
| `modules` | `.rhai` sources (or `.wasm` builds), or directories of them; a Rhai file can declare its own module with `fn declare()` or supply code for the linked module its stem names |
| `entry_room` | optional: the room the run starts in, a runtime room id; `--start-room` beside `--project` wins |
| `starting_items` | optional: the bag every session begins with, catalog ids and counts (`Some([("bomb", 3), ("lantern", 1)])`); absent, the game's own starting bag |

The tools take the project too:

```bash
cargo run -p ambition_app_tools --features game --bin extension_inspect -- --project examples/projects/sentry_drill
cargo run -p ambition_app_tools --features game --bin capture_scene -- central_hub_complex player shot.png --project examples/projects/sentry_drill
cargo run -p ambition_app_tools --features game --bin inspect_geometry -- --project examples/projects/sentry_drill --room central_hub_complex
```

Paths are relative to `project.ron`. The host's adapter is
`game/ambition_app/src/app/project.rs`.

[Pulse Forge](../pulse_forge/README.md) shows a new technique and entity kind
declared in project files, with no Rust descriptor.
