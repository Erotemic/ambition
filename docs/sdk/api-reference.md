# API reference

Everything a game calls, in one page.

⚠ **This page exists because the SDK was telling readers to do the thing its own
acceptance test measures.** `docs/sdk/README.md` recommended
`cargo doc -p ambition -p ambition_world --no-deps`, and both of blind run 4's
engine opens were exactly that — so the SDK's advice was generating the failures
the SDK is scored on. ADR 0031's gate is that an author never opens a file under
`crates/`; a document that sends them there cannot satisfy it, however useful
rustdoc is afterwards.

Kept honest by `scripts/tests/test_sdk_api_reference_is_current.py`: every
method named here must exist, and every public method must be named here. A
reference that drifts is worse than none, because a reader trusts it.

---

## `PlatformerApp` — the composition

| Method | What it does |
|---|---|
| `windowed(title)` | a game that opens a window |
| `headless()` | no display; one `update()` is one sim tick |
| `without_gpu()` | full render graph, no wgpu backend — for CI and display-less boxes |
| `with_game_assets()` | prepare art on a headless host (a window implies it) |
| `start_at_launcher()` | boot into a launcher over all mounted experiences, not into the first |
| `rollback(participants)` | compose for rollback, seating `participants` local players — see [Rollback](#rollback) |
| `mount(module)` | fold in a `GameModule`; the FIRST mounted owns the host's home |
| `try_build()` | the `App`, or a `CompositionError` listing every problem at once |
| `build()` | same, panicking with those problems |
| `run()` | build and run |
| `install_into(&mut app)` | add to an `App` you already own — ⚠ cannot register asset sources if `AssetPlugin` already built, and says so |

## `ModuleDraft` — what a module declares

Call `experience(id)` first; everything after it attaches to that experience. A
composition may hold several, keyed by id.

| Method | Required? |
|---|---|
| `experience(id)` | yes — the first call for each experience |
| `gameplay_route(route)` | yes, per experience |
| `launcher_route(route)` | yes, on the FIRST experience (the host's home) |
| `characters(ron)` / `no_characters()` | yes if the composition prepares art |
| `no_audio()` | in practice yes, unless you register a real audio fragment |
| `playable(label, description, starting_character, starting_room, rooms)` | this is what registers the gameplay route |
| `room(metadata)` | optional — picks block/biome art at `Startup` |
| `capability(plugin)` | optional — a Bevy plugin the engine installs in its own order |

## `GameModule` — the trait

```rust
fn manifest(&self) -> ModuleManifest;   // needed BEFORE the Bevy foundation
fn define(&self, module: &mut ModuleDraft);  // never touches `App`
```

`ModuleManifest::new(id)` and `.asset_source(AssetSource::at(scheme, root))` —
the asset source is optional; you need one only if your game ships its own art.

## `HostStatus` — did it start?

Useful field types the snippets use without naming: `BodyKinematics::pos` is a
`Vec2`, `ControlFrame::axis_x` is an `f32`.

`host_status(&app)` returns `NotComposed` / `Initializing` / `Activating` /
`Running { route, experience, prepared }` / `Refused { reasons }`.

| Method | Use |
|---|---|
| `is_running()` | live AND backed by a prepared session — both halves |
| `is_refused()` | it will never start; stop polling |
| `refusal()` | why — `&[String]` |
| `route()` | the active route, if any — `Option<&str>` |

## Rollback

Rollback is a supported session mode as of slice F. ADR 0031 deferred it until
it could carry six properties — frozen schema, complete authoritative baseline,
stable participants, deterministic activation, lifecycle rebasing, confirmation
boundaries — each of which now has a test in
`fixtures/external_consumer/tests/rollback_is_a_promise.rs`.

**Two halves, and they are separate on purpose.** `PlatformerApp::rollback(n)`
composes the host. `ambition::rollback::start(&mut app, plan)` starts the
session — it cannot happen at build time, because a session rebases frame zero
onto a world that has to be CONSTRUCTED first.

```rust
let mut app = PlatformerApp::headless()
    .rollback(2)
    .mount(MyGame)
    .build();

let session = ambition::rollback::start(&mut app, RollbackPlan::new())?;
assert_eq!(session.participants(), 2);
```

`start` activates the host, settles past activation, then rebases. Doing those
in the wrong order produces a checksum mismatch several frames later, where it
reads like a bug in your game — so the engine performs the sequence rather than
documenting it.

| `RollbackPlan` | Default |
|---|---|
| `new()` | 4 frames of comparison, 10 of prediction, 600-tick activation budget, 8 settle ticks |
| `check_distance(frames)` | how far back the session re-simulates and compares |
| `prediction_window(frames)` | how far ahead it may predict before stalling |
| `activation_budget(ticks)` | how long `start` waits for the host to run |
| `settle_ticks(ticks)` | quiet ticks after activation — ⚠ raise it, never lower it to zero |

⚠ **The participant count is not on the plan.** It is declared once, at
composition, so a restart reuses it instead of re-sampling. Every path that
guessed this number guessed one, and the engine ran a rollback oracle over a
single input stream for the week its versus mode seated four.

| `RollbackSession` | |
|---|---|
| `participants()` | how many the session seated |
| `encoded_types()` | how many kinds of authoritative state it carries — assert on this; a session over nothing passes |
| `ticks_to_activation()` | how long the host took to start |

`RollbackRefused` names the fix, not just the fault: `NotComposedForRollback`,
`NeverActivated`, `NoAuthoritativeState`, `SessionRejected`.

**Your own state joins the wire format** by implementing
`ambition::rollback::SnapshotState` and registering it:

```rust
use ambition::rollback::AmbitionRollbackApp;
app.rollback_component_canonical::<BeaconCharge>("mygame", "mygame.beacon");
```

No engine file lists your type, and nothing in `ambition` has heard of it. The
registration is what puts it in the baseline — without it, your state silently
does not roll back.

⚠ **Registering a component is not enough if YOU spawned the entity.** A
component only rolls back on an entity the session tracks, and the engine
tracks its own entities (the player body, projectiles, encounter authorities,
the room root) — not one your game created. Registration on an untracked entity
is *accepted*, counted in `encoded_types()`, and does nothing.

Declare the entity family too, once, with the component that identifies it:

```rust
app.require_rollback::<MyBeacon>("mygame", "entity:my_beacon");
```

Now any entity carrying `MyBeacon` is a rollback participant, and the
components registered on it roll back.

This is the one failure in this page with no error message behind it. Blind run
7 hit it, applied the remedy this section already gave, watched the count go
from 331 to 332, and still saw the component not roll back — and concluded a
third-party game could only roll back resources. It can do more; it needed one
more line, and the line was in rustdoc rather than here.

**Attaching to an engine entity works without this.** If your component rides
on the `PrimaryPlayer` body, that entity is already tracked. That is why the
engine's own external fixture never hit the gap — a difference between two
correct-looking programs that nothing in the API surfaces.

## Constants

| Name | For |
|---|---|
| `MINIMAL_CHARACTER_ROSTER_RON` | a working one-character roster; the character it declares is **`my_hero`** |
| `EMPTY_CHARACTER_ROSTER_RON` | the empty case, for `no_characters()`'s shape |

## The modules a game names

| Module | Holds |
|---|---|
| `ambition::app` | everything above, plus `app::prelude` |
| `ambition::world` | rooms and geometry — `world::prelude` is the one to import |
| `ambition::actor` | `PrimaryPlayer`, `BodyKinematics`, spawn requests, ability sets |
| `ambition::sim` | `ControlFrame`, `drive_control_frame`, `WorldTime`, schedule sets |
| `ambition::character` | catalogs, action sets, sheets, brains |
| `ambition::view` | `GameAssets`, `SandboxAssetCatalog`, `RoomVisual` |
| `ambition::rollback` | the rollback session mode, the snapshot vocabulary, and the registration verbs |
| `ambition::bevy` | Bevy itself, re-exported |

Anything else under `ambition::` is an implementation crate this facade mirrors,
carries no stability promise, and is measured as a leak by
`scripts/check_absence_contracts.py`.
