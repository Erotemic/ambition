# Bridge warden

A project an already built game runs, with no Rust: the script-first
program's capstone mechanism ([plan](../../../docs/planning/engine/script-first-ambition-gameplay.md),
slice C7), as a room you can play.

```bash
cargo build -p ambition_app --bin ambition_game_bin  # once: the host
./target/debug/ambition_game_bin --project examples/projects/bridge_warden
```

The run starts in the **yard**. The yard's own script (`modules/bridge_warden.rhai`,
entry `yard`) makes a warden and two posts the first tick it runs, and
attaches the warden's methods to the warden, subscribed to both posts. Walk
east and fell the posts (Attack). Each fall is a native defeat the warden
hears (`object_defeated`); it counts it in the world (`add_world_count`),
and when the count the world holds reaches two it lowers the bridge
(`set_world_value bridge.state "lowered"`, expecting no earlier value),
hearing each outcome (`world_count_added`, `world_value_set`). Walk on
through the east edge into the **bridge**: its gate is a `LockWall` gated by
`world.value_is bridge.state lowered`, so it stands until the warden's write
is confirmed and opens after. The bridge room's own script (`far`) reads the
gate back through `lock_walls`. The value is in the save, so the gate stays
open on a later run.

| file | what it is |
|---|---|
| `project.ron` | the project: its pack, its worlds, its modules |
| `content/characters.ron` | the runner (the player, with a quick swipe), the posts and the warden |
| `content/worlds.ron` | the world manifest: `warden.ldtk`, entry room `warden_yard` |
| `worlds/yard.json`, `worlds/bridge.json` | the two rooms as area specs; `python3 author_world.py` writes `worlds/warden.ldtk` from them |
| `modules/bridge_warden.rhai` | the whole mechanism |

## Edit it while it runs

`fn posts_to_lower() { 2 }` in `modules/bridge_warden.rhai` is how many
fallen posts lower the bridge. Change it to `1` and save while the game runs:
the host reloads the script without a build, keeps the warden's state, and
the next post to fall lowers the bridge while the other still stands. A
script that does not parse, or that faults, keeps the running code and names
the line.

Observed (2026-11-20, debug build, the witness below on the game's own local
session): the edit was published **155 ms** (11 frames) after the save, and
the bridge was lowered **2.0 s** after the save, which includes walking to a
post and felling it. No Cargo process runs in that loop.

A sync-test host refuses a reload: its timeline belongs to the harness, not
to the game, so the rollback owner cannot rebase it onto the edited code.
The game's own session rebases and continues.

## Witnesses

```bash
cargo test -p ambition_app --test app_it -- scripted_bridge_warden --test-threads=1 --nocapture
```

- `the_bridge_warden_project_is_played_through_by_its_runner`: this project,
  played: the runner fells both posts with its own strikes, the warden's log
  is `down;world_count_added;down;world_count_added;world_value_set;`, the
  runner crosses the edge and passes the gate. Sync-test host (4-tick
  rewinds), rollback healthy. Before `Attack` was given to the runner, the
  same walk reached the bridge with both posts standing and stopped at x 464,
  in front of the gate: the gate stands until the value is written.
- `an_edit_to_the_wardens_script_lowers_the_bridge_on_one_post`: the edit
  loop above.
- `the_warden_lowers_the_bridge_and_the_next_room_opens` and its variations
  run the same composition in the game's own `blink_run` and
  `portal_bridge`, including the caller retiring the fallen posts
  (`retire_creation`, heard as `object_retired`), a gate that waits on the
  count itself with no warden write, and the warden's methods as a portable
  WASM module (`fixtures/extension_fixture_modules/src/bridge_warden.rs`).

The cost of each entry is in
[`dev/benchmark-candidates/bridge-warden-cost.md`](../../../dev/benchmark-candidates/bridge-warden-cost.md).
