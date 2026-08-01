# Ambition SDK

**Building a game on this engine? Start here — and you should not need to open
anything under `crates/`.**

That is the acceptance test, not a courtesy. ADR 0031 makes the blind-agent run
one of two mechanical gates on the public API: *can an agent implement a
character, a room and a mechanic with only `docs/sdk/` and `ambition_platformer2d::prelude`
in context, never opening a file under `crates/`?* The recorded result includes
**which engine file it had to open first**, because that field names the next
leak. If you had to open one, that is a bug in this directory.

## Status: slices A–F closed

This SDK was built one leak at a time by
[the API 1.0 campaign](../planning/engine/api-1.0-campaign.md). Being honest
about what is not here yet is part of the method — a doc that implies coverage
it lacks sends a reader into `crates/` with no warning.

⚠ **This table was stale for four slices and is the reason four staleness
guards exist.** It still said "slice A, in progress" and listed content,
capabilities and rollback as *not started* after all three had shipped. Prose
went wrong here three times in four blind runs; the countermeasures are
`scripts/tests/test_sdk_*.py`, and they can check that a named module exists
but not that a sentence about coverage is true. This paragraph is the honest
statement of that limit.

| Area | Status |
|---|---|
| Host composition — standing up a game, visible and headless | **IMPLEMENTED** — `ambition_platformer2d::app`, designed in [api-prototype.md](api-prototype.md) |
| Declaring content — characters, rooms, packs | **IMPLEMENTED** — `ModuleDraft` (slice B) |
| Multi-experience composition and host policy | **IMPLEMENTED** — several experiences per composition, `start_at_launcher()` (slices D, E) |
| Rollback — sessions, participants, your own state in the wire format | **IMPLEMENTED** — `ambition_platformer2d::rollback` (slice F) |
| Revising content at runtime | not started — no consumer has needed it yet |

## Before any of that: your `Cargo.toml`

**This engine does not currently compile for you without a patch table, and
nothing tells you so.** Found by the 2026-07-30 blind run, which hit it before
it could ask a single API question:

```toml
[dependencies]
ambition_platformer2d = { path = "../path/to/ambition/crates/ambition_platformer2d" }

# Only if you `#[derive(Component)]` or `#[derive(Resource)]` yourself — the
# derive macros resolve `::bevy_ecs` through YOUR manifest and a re-export does
# not satisfy that. Otherwise `ambition_platformer2d::bevy` is enough.
bevy = "0.18"

# Toolchain: rustc/cargo 1.95.0 or newer. `edition = "2021"` is fine.
#
# ⚠ Budget ~1.5 GB of disk WITH the settings below (measured: check + build +
# test, ~250 crates), and ~2 min to check / ~3 min to build cold. Without
# `debug = 0` it is many times that — a default build blew an 11 GB budget, and expect ~2 min for
# `cargo check` / ~3 min for `cargo build` warm (~250 crates). Without
# `debug = 0` a default build blew an 11 GB budget and died in `rust-lld` with
# SIGBUS and an LLVM stack dump that never mentions disk.
[profile.dev]
debug = 0

# ⚠ REQUIRED. Ambition builds against a fork of bevy_ggrs (a backported
# `GgrsFrameTiming` accessor). Cargo patch tables do NOT cross a workspace
# boundary, so you must repeat this one yourself — copy the current value from
# the engine's workspace-root Cargo.toml.
[patch.crates-io]
bevy_ggrs = { git = "https://github.com/Erotemic/bevy_ggrs", rev = "4d2eff2a89f00c127e17fd26dd3f25d3a1113fa2" }
```

**And a `.cargo/config.toml`**, for the same reason the patch table is needed —
it does not cross a workspace boundary either:

```toml
# The engine repo uses these; a slow default linker on a ~250-crate graph is the
# difference between a 3-minute build and a 15-minute one.
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Blind run 5 had to open the engine's own `.cargo/config.toml` to find this —
the same class of leak as the patch table, and the last file any blind run has
needed.

Without the patch table a fresh lockfile resolves `bevy_ggrs` from crates.io and the build
dies in `ambition_platformer2d_runtime` with `cannot find type GgrsFrameTiming in crate
bevy_ggrs` — an error with no visible connection to a patch table you have never
seen.

You do **not** need to declare `bevy` yourself for ordinary use: `ambition_platformer2d`
re-exports it (`ambition_platformer2d::bevy`). You *do* need it in your own manifest if you
`#[derive(Component)]` or `#[derive(Resource)]`, because Bevy's derive macros
resolve `::bevy_ecs` through the consuming crate's manifest and a re-export does
not satisfy that.

This is a real defect, not a documentation quirk: an engine another game can be
built on has to be an engine another game can *link*. It is recorded as the
top-ranked finding in
`docs/planning/engine/slice-evidence/blind-agent-runs/2026-07-30-slice-a-baseline.json`.

## Standing a game up

```rust
use ambition_platformer2d::app::prelude::*;

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

The same poll applies to the windowed face — `without_gpu().build()` returns an
`App` you step exactly like the headless one, and "composed" is not "running"
there either.

**On a machine with no display** — CI, a container, a headless box — the
windowed face still composes and runs:

```rust
PlatformerApp::windowed("My Game").without_gpu().build()
```

A bare `run()` there panics inside `bevy_winit`, and the message depends on
which flavour of "no display" you have — neither mentions Ambition:

* **`DISPLAY` unset** — `neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY
  is set`;
* **`DISPLAY` set but no server reachable** (CI, a container with a stale
  `DISPLAY`, `ssh` without `-X`) — `XNotSupported(XOpenDisplayFailed)`.

⚠ Both are listed because the second is the common one and this document used to
promise only the first. Somebody grepping their actual error text would not have
found it.

If you need to prove a real window in CI, install Xvfb and set `DISPLAY=:99`;
that is the one actionable option and this document used to omit it while
correctly telling you what not to claim.

And `without_gpu()` proves COMPOSITION and art preparation — not winit, not
wgpu, not pixels. On a display-less box you cannot verify a real window at all,
and reporting "windowed boot achieved" from `without_gpu()` alone overstates
what was tested.

### Cargo features

Default features work for both faces and are the right starting point. The
facade exposes ~30 more; the ones a game reaches for first:

| Feature | For |
|---|---|
| `visible` | the render chain — required for a real window |
| `input` | the standard host input path |
| `basic_shell_presentation` | the minimal shell's own frames |
| `desktop_platform` / `web_platform` / `android_platform` | target selection |
| `audio` | sound (you still declare `no_audio()` if you author none) |

`ambition_platformer2d::app::prelude` re-exports `PlatformerApp`, `GameModule`,
`ModuleManifest`, `ModuleDraft`, `AssetSource`, `SessionMode`, `StartAt`,
`CompositionError`, `HostStatus`, `host_status`, `RoomSpec`, `RoomMetadata`,
`EMPTY_CHARACTER_ROSTER_RON`, `MINIMAL_CHARACTER_ROSTER_RON` — and Bevy's `App`,
which you need the moment you factor the poll loop into a helper and have to
name a parameter type.

**The full surface is [api-reference.md](api-reference.md)** — every method on
`PlatformerApp`, `ModuleDraft` and `HostStatus`, in one page, kept in sync with
the source by a test in both directions.

`cargo doc -p ambition_platformer2d -p ambition_platformer2d_world --no-deps --open` is good for browsing
afterwards. ⚠ It should not be your first stop: ADR 0031's acceptance test is
that you never open a file under `crates/`, and this document used to send you
there before saying anything else.

## Asking your game what it is doing

`host_status` answers *did the engine start*. It does not answer *is my game
playable* — blind run 3 shipped a build reporting `Running { prepared: true }`
for 600 ticks while its character fell out of the world on a loop. Four names
close that gap:

```rust
use ambition_platformer2d::actor::{BodyKinematics, PrimaryPlayer};
use ambition_platformer2d::bevy::prelude::With;          // Bevy's query filter
use ambition_platformer2d::sim::{drive_control_frame, ControlFrame};

// where is my character?
let mut bodies = app.world_mut()
    .query_filtered::<&BodyKinematics, With<PrimaryPlayer>>();
let pos = bodies.single(app.world_mut()).unwrap().pos;

// make it walk right for a frame
drive_control_frame(app.world_mut(), ControlFrame { axis_x: 1.0, ..Default::default() });
```

### A room, in full

The room vocabulary lives in **`ambition_platformer2d::world::prelude`** — not in
`ambition_platformer2d::app::prelude`, which carries `RoomSpec` but not `Block` or
`AuthoredWorld`. `use ambition_platformer2d::world::*` resolves to nothing; you want the
prelude.

```rust
use ambition_platformer2d::world::prelude::*;

fn my_room() -> RoomSpec {
    let size = Vec2::new(640.0, 360.0);
    let world = AuthoredWorld::new(
        "My Room",
        size,                          // room extent
        Vec2::new(64.0, 256.0),        // where the character spawns
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, 320.0),     // MIN corner
            Vec2::new(size.x, 40.0),   // size
        )],
    );
    RoomSpec::new("my_room", world)
}
```

`AuthoredWorld` is the authored world IR, exported under that name because
`bevy::prelude::World` is a different type and every Bevy game imports it.

**If you use `MINIMAL_CHARACTER_ROSTER_RON`, your starting character id is
`my_hero`** — that is the character it declares, and a starting character no
roster contains is refused at build.

`RoomSpec` is not `Copy`. If you pass both `.room(room.metadata.clone())` and
`.playable(.., vec![room])`, clone the metadata.

**`AssetSource` is optional.** You need one only if your game ships its own art;
a module that declares none still resolves the engine's own assets.

### Observing what the engine drew

`ambition_platformer2d::view` is the read side of presentation — what exists to be drawn,
rather than how it is drawn:

```rust
use ambition_platformer2d::view::{GameAssets, RoomVisual, AmbitionGameAssetCatalog};
```

`GameAssets` holds the decoded sheets, `AmbitionGameAssetCatalog` every asset
path/source policy the presentation reads, and `RoomVisual` marks the entities a
room contributed. A game reads these; it does not own the render path.

### Room coordinates

**+y points DOWN**, and `Block::solid(name, min, size)` takes a **MIN CORNER,
not a centre**. Getting that wrong puts your floor somewhere else in the room,
your character falls past it, and nothing reports anything — the host is running
correctly, the content is wrong. The reference fixture itself had this bug until
2026-07-30 and its own tests could not see it.

**Shipping more than one game?** Mount each as its own module and start at a
launcher instead of inside one of them:

```rust
PlatformerApp::windowed("My Collection")
    .start_at_launcher()
    .mount(FirstGame::default())
    .mount(SecondGame::default())
    .run();
```

Experiences are keyed by id, so distinct ids coexist and a duplicate is refused
naming both modules. Without `start_at_launcher()` the host boots straight into
the first one mounted.

`PlatformerApp::headless()` is the same game with no display, where one
`App::update` is exactly one simulation tick. Both faces mount the same module
and install the same engine in the same order.

The visible face additionally prepares art, which needs a character roster —
declare one with `characters(MINIMAL_CHARACTER_ROSTER_RON)` and a minimal module
reaches it fine.

⚠ This paragraph claimed the two faces were "not yet fully interchangeable" and
that the windowed face "requires content a minimal module does not have". That
was true before slice B and false afterwards, and blind run 5 disproved it by
booting a minimal module on the windowed face while the sentence was still here.
Second time this document has advertised a gap it no longer had.

**Everything you need is on this page and in
[api-reference.md](api-reference.md).** You should not have to read engine
source to write a game — that is this SDK's acceptance test, not a courtesy.

⚠ This section used to say "the reference to copy is `fixtures/minimal_game`".
Blind run 6 pointed out that taking that advice fails the very gate this
document opens with, and that it is the identical defect `api-reference.md` was
created to fix for `cargo doc`: **the SDK telling readers to do the thing it is
scored on.** A less suspicious reader would have followed it and logged
`fixtures/minimal_game/src/lib.rs` as their first engine open.

`fixtures/minimal_game` and `fixtures/external_consumer` remain the engine's own
worked examples, for engine developers. If you find yourself needing them,
that is a bug in this directory — please say which page failed you.

`fixtures/external_consumer` (Outlander) is the larger worked example — a
character, an enemy, a construction recipe, a transition, and a rollback host —
and it still composes some things by hand for the areas above that are not
started.

## The compatibility promise

A game depends on **`ambition_platformer2d`** and nothing else from this workspace (plus
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
* ~~The non-empty character roster schema is undocumented.~~ **CLOSED.**
  `MINIMAL_CHARACTER_ROSTER_RON` is a working one-character roster, and its
  rustdoc names every enum-valued field — `tier: MainHall`,
  `body_kind: Standard`, `composition: None`, `playable_kit: Authored` (vs
  `HostCode`, which silently overrides your action set), `move_style: Walk`,
  `StandStill`. Those are the ones no error message can give you: the parser
  reports one missing field per build and stops dead at the first enum, because
  variant names cannot be guessed.
* **Component names are invisible.** `ambition_platformer2d::bevy` is re-exported without
  Bevy's `debug` feature, so `World::inspect_entity` reports
  `<Enable the debug feature to see the name>` for every component — which
  removes the one generic tool you have for "what did the engine spawn?".
* ~~Content, capabilities and runtime content revision are not started.~~
  **PARTLY CLOSED.** Content and capabilities are `ModuleDraft` (slices B–E);
  rollback is `ambition_platformer2d::rollback` (slice F). Runtime content revision is still
  not started, and no consumer has yet needed it — which is why it has not been
  designed rather than why it is fine.
* **The rollback surface has never been seen by a blind author.** Every other
  part of this SDK has been through at least one run where a third-party agent
  built against it with no access to `crates/`; `ambition_platformer2d::rollback` shipped
  after the last one. It is the newest surface and therefore the likeliest to
  send you into the engine. If it does, that is a defect — please say where.

Closed since the runs that found them, and now covered by tests: a declared
route no capability registers is refused with the registered routes named; a
starting character no roster contains is refused at BUILD rather than hanging
forever; a host that can never start reports why instead of spinning;
`ModuleDraft::capability` no longer requires `Clone`; and `ambition_platformer2d::world` is a
curated module rather than a whole-crate mirror.
