# API reference

Everything a game calls, in one page.

`scripts/tests/test_sdk_api_reference_is_current.py` checks this page in both
directions: documented methods must exist, and public methods must be documented.

---

## `PlatformerApp` — the composition

| Method | What it does |
|---|---|
| `windowed(title)` | a game that opens a window |
| `headless()` | no display; one `update()` is one sim tick |
| `without_gpu()` | full render graph, no wgpu backend — for CI and display-less boxes |
| `offscreen()` | full render graph on a real backend, no window — so no `winit` and no app runner, and the caller steps the app itself |
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
| `actions(&[..])` | optional — semantic actions the capability contributes; the composition REFUSES if two claim the same id |
| `requires_rollback(&[..])` | optional — rollback state the capability must have restored; refused at assembly if nothing registered it |
| `provides_rollback(owner, name, probe)` | the other half — the typed registration that SATISFIES a requirement |

### What a capability CONTRIBUTES, beside its systems

A capability offers up to three things past its plugin, and the composition
installs each one:

```rust
module
    .capability(my_mechanic::MyPlugin)                  // behaviour
    .actions(&[my_mechanic::MY_ACTION])                 // the verbs it adds
    .requires_rollback(my_mechanic::REQUIRED_ROLLBACK); // what a rewind must restore
```

**`actions(&[SemanticActionDef])`** registers verbs into the shared vocabulary.
Two capabilities claiming the same id is a REFUSAL, not a last-one-wins — that
is the point of declaring them.

⚠ **an action can be declared and queried; it cannot yet carry a device binding
of its own.** `InputMap` is still keyed by the engine's closed `Platformer2dInputActionMonolith`,
so a consumer fires your mechanic by writing your own request message — which is
also how a scripted sequence or an AI would. Do not invent a private binding
path around it.

**`requires_rollback(&[RequiredRollbackState])`** declares what a rewind must
restore, and the composition refuses at assembly if nothing registered it.

**`provides_rollback(owner, name, probe)`** — turbofished with the component
type — is the half that satisfies it:

```rust
module
    .capability(my_mechanic::MyPlugin::default())
    .requires_rollback(my_mechanic::REQUIRED_ROLLBACK)
    .provides_rollback::<my_mechanic::MyCooldown>(
        my_mechanic::MY_CAPABILITY,
        my_mechanic::ROLLBACK_STATE,
        |c| u64::from(c.remaining_ticks),
    );
```

⚠ **the owner and name must MATCH the requirement.** A registration under
another owner satisfies nothing — that is what makes the pair a contract instead
of two lists. Contributions are applied before the requirement check, and only
when the composition declared `rollback(n)`.

Declare rollback requirements and contributions through `ModuleDraft` instead
of importing runtime registration machinery into the capability crate. Capability
systems belong in the host simulation schedule, not bare `Update`.

Each `RequiredRollbackState` carries a `why`. It is not decoration: a host that
hits the refusal needs to know whether it is looking at a desync or an optional
extra, and only the capability knows which.

`examples/capability_demo` is the worked example, and
[`../recipes/adding-a-capability.md`](../recipes/adding-a-capability.md) is the
recipe.

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

## Where your gameplay systems go

`ModuleDraft::capability(plugin)` is where a game adds its own systems. Gameplay
systems belong in the **sim schedule**, not bare `Update`, so they participate
in both fixed-step simulation and rollback re-simulation:

```rust
use ambition_platformer2d::sim::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};

impl Plugin for MyCapability {
    fn build(&self, app: &mut App) {
        // NOT `Update`. `sim_schedule()` returns whichever schedule this host
        // simulates in — FixedUpdate on a fixed-tick host, the GGRS schedule
        // under rollback. Asking is what makes one plugin correct on both.
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (charge_beacon, open_gate)
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );
    }
}
```

Pick the phase by what your system reads:

| Phase | For |
|---|---|
| `WorldPrep` | world state before the player ticks — hazards, feature ticks |
| `PlayerInput` | the input pipeline, timers, interaction buffers |
| `PlayerSimulation` | post-input body authority — **the usual one** |
| `RoomTransition` | detecting and applying a room change |
| `Combat` | attack lifecycle, projectiles, damage |
| `PresentationSync` | write-back and presentation timers |
| `FeatureCollection` / `FeatureInteraction` | pickups; switches, chests, breakables |

Ordering *within* a phase is yours (`.chain()`); ordering *between* phases is
the engine's. If two of your systems have a real read-after-write dependency,
chain them and say so — that is a dependency, not a preference.

## Supplying input without a device

A headless harness, a replay, an RL agent, an acceptance walk — anything
supplying input that is not a device — writes through one verb:

```rust
ambition_platformer2d::sim::drive_control_frame(app.world_mut(), ControlFrame { axis_x: 1.0, ..default() });
```

⚠ **It is correct on BOTH hosts, and that is the whole reason it exists.**
There are two resources underneath and picking the wrong one fails silently —
the walk runs, the body never moves, nothing says why. Under a fixed-tick host
`ControlFrame` is the input the sim reads; under rollback it is an *output*
written from the session's confirmed inputs, so a driver writing it directly
would feed re-simulated input back in as new input. `drive_control_frame` picks
for you, and defers to the device latch when a windowed build has one.

## Rollback

Rollback is a supported session mode. Its public contract includes frozen schema,
a complete authoritative baseline, stable participants, deterministic activation,
lifecycle rebasing, and confirmation boundaries.

**Two halves, and they are separate on purpose.** `PlatformerApp::rollback(n)`
composes the host. `ambition_platformer2d::rollback::start(&mut app, plan)` starts the
session — it cannot happen at build time, because a session rebases frame zero
onto a world that has to be CONSTRUCTED first.

```rust
let mut app = PlatformerApp::headless()
    .rollback(2)
    .mount(MyGame)
    .build();

let session = ambition_platformer2d::rollback::start(&mut app, RollbackPlan::new())?;
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

⚠ **The participant count is not on the plan.** Declare it once at composition
with `rollback(n)`; session restarts reuse that count.

⚠ **`rollback(n)` sizes the GGRS input streams. It does not create n
characters.** These are independent facts today and nothing reconciles them:
the count decides how many streams are checksum-compared; the *seating* comes
from the stage and its devices. A headless composition seats one body no matter
what you declare, and no error says so. Check what you actually got by querying
`ambition_platformer2d::actor::MatchSeat`.

Use `drive_control_frame` for the primary seat and
`drive_slot_frame` with `PlayerSlot(n)` to drive a specific seat. The
rollback participant count and the number of spawned/seated characters remain
independent facts, so query the seats when that distinction matters.

| `RollbackSession` | |
|---|---|
| `participants()` | how many the session seated |
| `encoded_types()` | how many kinds of authoritative state the session carries |
| `ticks_to_activation()` | how long the host took to start |

⚠ **Assert on the delta of `encoded_types()`, not its absolute value.** The
count includes engine registrations and may change as the engine evolves.

`RollbackRefused` names the fix, not just the fault: `NotComposedForRollback`,
`NeverActivated`, `NoAuthoritativeState`, `NoSessionWorld`, `SessionRejected`.

⚠ **`NoSessionWorld` means activation has not produced the session world yet.**
Starting there would rebase frame zero onto an empty world. Wait for
`settle_until_session_world` before starting the rollback session.

### Is it still running?

A host can be running while its rollback timeline is stalled. Use
`ambition_platformer2d::rollback::health` for rollback health:

```rust
match ambition_platformer2d::rollback::health(&app) {
    RollbackHealth::Healthy { frame, .. } => { /* compare frame across updates */ }
    RollbackHealth::Desynced { frames, .. } => panic!("nondeterminism at {frames:?}"),
    RollbackHealth::Invalidated { reason } => panic!("{reason}"),
    RollbackHealth::NoSession => {}
}
```

`Desynced` means a re-simulated frame produced a different answer — a
determinism bug in your game or the engine, and the whole reason to run a sync
test. Without this, GGRS reports it through a log line a headless game never
sees.

| `RollbackHealth` | |
|---|---|
| `is_healthy()` | simulating, nothing mismatched — the one-line check |
| `frame()` | the session's current frame, if there is a session |
| `generation()` | WHICH session — see below |

⚠ **Liveness needs TWO samples.** A frozen session reports `Healthy` forever;
what a stall looks like is a `frame()` that stops advancing. Sample it before
and after a batch of updates — the engine cannot decide for you how long a
quiet frame is allowed to be.

⚠ **and a frame number cannot tell you WHICH session it belongs to.** Every
session starts numbering at zero, and the engine legitimately installs a new one
— a confirmed lifecycle commit rebases the timeline, and so does
[`ambition_platformer2d::rollback::stop`] followed by another `start`. A `frame()` that went
backwards is a restart, not a rewind, and `generation()` is what says so: it
changes on every session install and never repeats.

`ambition_platformer2d::rollback::stop(&mut app)` ends the session. After it, `health` reports
`NoSession` — the session, not a leftover read model, is what it asks — unless
the timeline it tore down had diverged, in which case the DIAGNOSIS survives as
`Invalidated { reason }`. A divergence that disappeared when its timeline ended
would be exactly the laundering the engine refuses everywhere else.

### Putting your own state in the wire format

```rust
use ambition_platformer2d::rollback::{put_u32, Reader, SnapshotState};

impl SnapshotState for BeaconCharge {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.ticks);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { ticks: r.u32()? })
    }
}
```

The primitives are `put_bool`, `put_u8`, `put_u32`, `put_u64`, `put_i32`,
`put_f32`, `put_vec2`, `put_str`, `put_opt_str`, with matching `Reader`
accessors (`r.u32()`, `r.f32()`, `r.vec2()`, …).

⚠ **Encode and decode must read the same fields in the same ORDER.** This is a
wire format: a field added to one side and not the other, or read out of order,
does not fail to compile — it decodes garbage.

Then register it:

```rust
use ambition_platformer2d::rollback::AmbitionRollbackApp;
app.rollback_component_canonical::<BeaconCharge>("mygame", "mygame.beacon");
```

No engine file lists your type, and nothing in `ambition_platformer2d` has heard of it. The
registration is what puts it in the baseline — without it, your state silently
does not roll back.

⚠ **Spawn rollback entities before `start()`.** `start` rebases frame zero onto
the world it finds, so entities that must belong to the baseline must already exist.

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

The public surface is named for game concepts. These are curated modules: adding
something to an implementation crate does not make it public here automatically.

| Module | Holds |
|---|---|
| `ambition_platformer2d::app` | application composition, plus `app::prelude` |
| `ambition_platformer2d::engine` | simulation-host selection and engine foundation assembly |
| `ambition_platformer2d::participant` | participant ids, semantic actions, local devices/seats/channels |
| `ambition_platformer2d::session` | prepared/live session identity and canonical session-world access |
| `ambition_platformer2d::world` | rooms and geometry — `world::prelude` is the one to import |
| `ambition_platformer2d::actor` | body state, spawn/construction requests, ability sets |
| `ambition_platformer2d::character` | catalogs, action sets, sheets, brains |
| `ambition_platformer2d::item` | item simulation state when the item capability is installed |
| `ambition_platformer2d::settings` | user gameplay settings when persistence is installed |
| `ambition_platformer2d::sim` | input frames, participant driving, simulation time and schedule sets |
| `ambition_platformer2d::view` | `GameAssets`, `Platformer2dAssetCatalog`, `RoomVisual` |
| `ambition_platformer2d::capture` | offscreen screenshots with no window — behind `capture`, default-off |
| `ambition_platformer2d::observation` | what an observer READS: authoritative combat geometry (hurtboxes with the rule that produced them, live strike volumes, each strike's hit-once memory), stable `SimId`, and the geometric vocabulary both are expressed in |
| `ambition_platformer2d::presentation` | generic visible-game presentation when rendering is installed |
| `ambition_platformer2d::windowed_host` | window/input host plugin groups |
| `ambition_platformer2d::rollback` | rollback sessions, snapshot vocabulary and registration verbs |
| `ambition_platformer2d::content` | the optional runtime content compiler |
| `ambition_platformer2d::causal` | the optional causal inspector |
| `ambition_platformer2d::provider` | the experience-provider protocol |
| `ambition_platformer2d::bevy` | Bevy itself, re-exported |

There are still crate-shaped mirrors under `ambition_platformer2d::` while the
first-party games migrate. They carry no SDK compatibility promise. Real
consumers are ratcheted against naming them by `scripts/check_absence_contracts.py`;
when a consumer needs a missing concept, add the narrow semantic seam instead of
teaching it an implementation-crate path.
