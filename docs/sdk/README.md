# Ambition SDK

**Building a game on this engine? Start here — and you should not need to open
anything under `crates/`.**

That is the acceptance test, not a courtesy. ADR 0031 makes the blind-agent run
one of two mechanical gates on the public API: *can an agent implement a
character, a room and a mechanic with only `docs/sdk/` and `ambition::prelude`
in context, never opening a file under `crates/`?* The recorded result includes
**which engine file it had to open first**, because that field names the next
leak. If you had to open one, that is a bug in this directory.

## Status: slice A, in progress

This SDK is being built one leak at a time by
[the API 1.0 campaign](../planning/engine/api-1.0-campaign.md). Being honest
about what is not here yet is part of the method — a doc that implies coverage
it lacks sends a reader into `crates/` with no warning.

| Area | Status |
|---|---|
| Host composition — standing up a game, visible and headless | **IMPLEMENTED** — `ambition::app`, designed in [api-prototype.md](api-prototype.md) |
| Declaring content — characters, rooms, packs | not started (slice B) |
| Capabilities and rollback schema | not started (slice C) |
| Revising content at runtime | not started (slice D) |

## Before any of that: your `Cargo.toml`

**This engine does not currently compile for you without a patch table, and
nothing tells you so.** Found by the 2026-07-30 blind run, which hit it before
it could ask a single API question:

```toml
[dependencies]
ambition = { path = "../path/to/ambition/crates/ambition" }

# Only if you `#[derive(Component)]` or `#[derive(Resource)]` yourself — the
# derive macros resolve `::bevy_ecs` through YOUR manifest and a re-export does
# not satisfy that. Otherwise `ambition::bevy` is enough.
bevy = "0.18"

# ⚠ REQUIRED. Ambition builds against a fork of bevy_ggrs (a backported
# `GgrsFrameTiming` accessor). Cargo patch tables do NOT cross a workspace
# boundary, so you must repeat this one yourself — copy the current value from
# the engine's workspace-root Cargo.toml.
[patch.crates-io]
bevy_ggrs = { git = "https://github.com/Erotemic/bevy_ggrs", rev = "4d2eff2a89f00c127e17fd26dd3f25d3a1113fa2" }
```

Without it a fresh lockfile resolves `bevy_ggrs` from crates.io and the build
dies in `ambition_runtime` with `cannot find type GgrsFrameTiming in crate
bevy_ggrs` — an error with no visible connection to a patch table you have never
seen.

You do **not** need to declare `bevy` yourself for ordinary use: `ambition`
re-exports it (`ambition::bevy`). You *do* need it in your own manifest if you
`#[derive(Component)]` or `#[derive(Resource)]`, because Bevy's derive macros
resolve `::bevy_ecs` through the consuming crate's manifest and a re-export does
not satisfy that.

This is a real defect, not a documentation quirk: an engine another game can be
built on has to be an engine another game can *link*. It is recorded as the
top-ranked finding in
`docs/planning/engine/slice-evidence/blind-agent-runs/2026-07-30-slice-a-baseline.json`.

## Standing a game up

```rust
use ambition::app::prelude::*;

fn main() {
    PlatformerApp::windowed("My Game")
        .mount(MyModule::default())
        .run();
}
```

Your module's `define` needs at least this much, and the last two are not
optional in practice:

```rust
fn define(&self, module: &mut ModuleDraft) {
    module
        .experience("my_game")
        .launcher_route("my_game/menu")
        .gameplay_route("my_game/play")
        .characters(MY_ROSTER_RON)   // or `.no_characters()`
        .no_audio()                  // or register a real audio fragment
        .playable("My Game", "…", "my_hero", "my_room", vec![my_room()]);
}
```

⚠ **`playable()` is what registers the gameplay route**, and **`no_audio()` is
not cosmetic** — preparation refuses an experience that declares neither audio
nor silence, and the host then sits in `Activating` until it is refused.

⚠ **Check that it started.** Composing successfully is not the same fact as
running, and blind run 2 caught itself shipping a binary that exited `0` with a
host that had never started:

```rust
let mut app = PlatformerApp::headless().mount(MyModule::default()).build();
for _ in 0..600 {
    app.update();
    if host_status(&app).is_running() { break; }
}
assert!(host_status(&app).is_running(), "{:?}", host_status(&app));
```

`HostStatus::Refused { reasons }` tells you why a host will never start, so a
poll loop can stop instead of spinning. Use `is_refused()` as the other exit.

**On a machine with no display** — CI, a container, a headless box — the
windowed face still composes and runs:

```rust
PlatformerApp::windowed("My Game").without_gpu().build()
```

A bare `run()` there panics inside `bevy_winit` with `neither WAYLAND_DISPLAY
nor WAYLAND_SOCKET nor DISPLAY is set`, a message that never mentions Ambition.

**Reading the API:** `cargo doc -p ambition -p ambition_world --no-deps --open`.
Both crates, or the room types render as unlinked text.

## Asking your game what it is doing

`host_status` answers *did the engine start*. It does not answer *is my game
playable* — blind run 3 shipped a build reporting `Running { prepared: true }`
for 600 ticks while its character fell out of the world on a loop. Four names
close that gap:

```rust
use ambition::actor::{BodyKinematics, PrimaryPlayer};
use ambition::sim::{drive_control_frame, ControlFrame};

// where is my character?
let mut bodies = app.world_mut()
    .query_filtered::<&BodyKinematics, With<PrimaryPlayer>>();
let pos = bodies.single(app.world_mut()).unwrap().pos;

// make it walk right for a frame
drive_control_frame(app.world_mut(), ControlFrame { axis_x: 1.0, ..Default::default() });
```

### Room coordinates

**+y points DOWN**, and `Block::solid(name, min, size)` takes a **MIN CORNER,
not a centre**. Getting that wrong puts your floor somewhere else in the room,
your character falls past it, and nothing reports anything — the host is running
correctly, the content is wrong. The reference fixture itself had this bug until
2026-07-30 and its own tests could not see it.

`PlatformerApp::headless()` is the same game with no display, where one
`App::update` is exactly one simulation tick. Both faces mount the same module
and install the same engine in the same order.

They are not yet fully interchangeable, though — see *Known gaps* below. The
visible face also prepares art, which currently requires content a minimal
module does not have.

**The reference to copy is `fixtures/minimal_game`** — the smallest thing that
is still a game: one room, one walker, no combat, no art, no plugin of its own.
It declares itself entirely through `ModuleDraft` and names only
`ambition::app`, `ambition::world` and the `bevy` re-export.

`fixtures/external_consumer` (Outlander) is the larger worked example — a
character, an enemy, a construction recipe, a transition, and a rollback host —
and it still composes some things by hand for the areas above that are not
started.

## The compatibility promise

A game depends on **`ambition`** and nothing else from this workspace (plus
`bevy`, because derive macros resolve `::bevy_ecs` through the consumer's own
manifest).

The promise is made at that surface and nowhere else. Inner `ambition_*` crates
stay independently usable by engine developers and carry **no stability
promise** — if your imports name one, you are depending on our implementation
topology and we will move it.

That is enforced, not asked for: `scripts/check_absence_contracts.py` carries a
module allowlist over consumer code, with a frozen baseline that may only
shrink. Run it to see what the reference consumer still names and how often:

```bash
python3 scripts/check_absence_contracts.py
python3 scripts/check_absence_contracts.py --allowlist-open-count
```

Every module in that output is a leak this SDK has not closed yet. Eighteen at
the start of slice A.

## Known gaps

⚠ **This section was wrong for eight commits and cost blind run 2 real time.**
It claimed a minimal module could not reach a windowed host and that there was
no way to ask whether the game started; slice B and C had closed both. A doc
that advertises gaps it no longer has sends readers into `crates/` exactly as
surely as one that hides the gaps it does. Both are the same defect.

Current, verified against `fixtures/minimal_game/tests/boots.rs`:

* **A castless game cannot be expressed.** `no_characters()` exists, but
  `playable()` requires a starting character and without `playable()` no
  gameplay route is registered — so a menu-only app has no route to a composing
  host. Tracked as consumer-matrix work, not a bug with a workaround.
* **The non-empty character roster schema is undocumented.** `spritesheet`,
  `manifest`, `tier`, `body_kind`, `composition`, `default_brain`,
  `default_action_set`, `playable_kit` and `tags` are all required with no
  documented values. `EMPTY_CHARACTER_ROSTER_RON` covers only the empty case;
  for one character, copy `fixtures/minimal_game/src/minimal_experience.rs`.
* **Component names are invisible.** `ambition::bevy` is re-exported without
  Bevy's `debug` feature, so `World::inspect_entity` reports
  `<Enable the debug feature to see the name>` for every component — which
  removes the one generic tool you have for "what did the engine spawn?".
* **Content, capabilities and runtime content revision are not started**
  (slices B+ in the campaign). Those areas are still composed by hand.

Closed since the runs that found them, and now covered by tests: a declared
route no capability registers is refused with the registered routes named; a
starting character no roster contains is refused at BUILD rather than hanging
forever; a host that can never start reports why instead of spinning;
`ModuleDraft::capability` no longer requires `Clone`; and `ambition::world` is a
curated module rather than a whole-crate mirror.
