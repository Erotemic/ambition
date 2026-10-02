# Track S — Sanic (momentum acceptance demo)

Inspired by the momentum-platformer contract, with parody-original art and level
design rather than copied content.

## Purpose

Prove that a second movement identity can coexist with classic AABB platforming
in one engine, selected per body by authored data. Prove that a new provider
gets input, simulation, presentation, camera, audio and hosted lifecycle without
editing engine internals.

Engine capabilities it stresses: the surface-momentum kernel (chains, attached
loops, filled ground, route junctions, `GravityZone` ceilings); `MotionModel`
selection per body; frame-aware input through the standard `ControlFrame` road;
world IR and LDtk conversion for chains, loops, ramps, boosters and routes;
`Departure` (a level asks to leave, the engine resolves `next_room`); the shared
economy and contact/combat pass for rings and badniks; `SimView` for HUD and
agent observation; provider-owned character, action, sprite, audio and world
catalogs.

## Status

Built: `game/ambition_demo_sanic` (content) and `game/ambition_demo_sanic_app`
(standalone shell).

- Three acts form a cycle. Act 1 is the speedway. Act 2 is the highway (speed
  picks the high road or the valley; two secrets). Act 3 is darkness (attached
  loops, two high roads, portals). Each act's LDtk level fields state its mode,
  biome and visual profile. The crate's `tools/author_*_ldtk.py` scripts author
  the maps; each script's docstring is its course map.
- Sanic and Super Sanic are provider-owned profiles. The Utility action is the
  only way into the super form. ⛔ Jon: *"the sanic level should not offer super
  form. at all. There is a key for it."* Do not add a super-form monitor.
- The ball dash is a provider technique on the standard action seam.
- Rings are authored `currency:1` pickups on the shared economy, drawn with the
  animated `sanic_ring_prop` sheet. A hit taken holding rings costs the rings
  (`BodyHitResolution::WalletShielded`) and scatters them radially. A hit taken
  holding none is fatal (`max_health: 1`).
- The badnik is defeated by a stomp with bounce or by a roll-through, through the
  shared contact pass. Badniks ride the surface solver.
- Crossing the goal clears the act: the clock stops, time and rings are captured,
  a results card holds, and the act replays through `RoomReplayRequested`. After
  the goal the course takes the controls, so Sanic cannot coast off the level.
- The demo app has no `ambition_app` dependency, no app-local input system, and
  no direct sprite binding. Ring art enters through `register_prop_sheet`.

Witnesses: `act_completion.rs`, `three_acts.rs`
(`the_three_acts_connect_and_clear`), `act_two_routes.rs`,
`spikes_spend_rings.rs`, `room_replay.rs`, `standard_input_path.rs` in
`game/ambition_demo_sanic_app/tests/`; the speedway and highway oracles in
`game/ambition_demo_sanic/src/tests/`.

## Ownership

`ambition_demo_sanic` owns its worlds, mode-scoped rules, ball-dash technique,
ring and drop policy, boosters and springs, enemy rows, act completion, result
sequence and HUD. These stay content even when they expose a reusable engine
gap.

## Open acceptance

This list is the single source. [`status.md`](../status.md) and
[`tracks.md`](../tracks.md) refer here.

- **High route beats safe route.** A headless gate that completes act 1 faster
  through the rewarded high route, with the same selected profile and control
  path as a visible run. Completion is proven; the two-route contest is not.
  `act_score` is a pure function (time bonus against par plus a per-ring bonus);
  no test drives the claim that the routes compete.
- **Swept ring collection.** `collect_ecs_pickups`
  (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/pickups.rs`)
  uses a per-frame overlap, so a ring can tunnel at high speed. The magnet range
  hides it today. The correct route is `cast::aabb_path_contacts`.
- **Ring collect animation.** The ring sheet has a `collect` row that is not
  played. The spark VFX covers collection.
- **Super-form ring drain.** The form should wear off through the same
  worn-identity seam the toggle uses. The `sanic.ring_loss` cue exists.
- **Ring milestones** (50/100 rings: extra life or jingle).
- **Authoring gap:** `area create` cannot author `next_room`. The Sanic scripts
  set it with `level set-field`.

Optional engine enhancement: a per-play pitch/gain on `SfxMessage` would let one
rev cue climb continuously instead of in bucketed tiers.

Authoring rules learned on this demo:

- A `ReboundPad` mostly along a riding body's surface only adds speed. A
  launcher must be a spring.
- `area create` lowers `Solid`/`OneWayPlatform` into the IntGrid and drops their
  names. Add a block that a rule finds by name (a monitor, a breakable wall)
  afterwards with `entity add`.

## How to run

```bash
./run_game.sh sanic                          # windowed standalone shell
./run_game.sh sanic --headless -- --ticks 600
cargo test -p ambition_demo_sanic -p ambition_demo_sanic_app
cargo run -p ambition_demo_sanic_app --features visible --bin capture_sanic \
    -- OUT.png [WIDTHxHEIGHT] [--warmup N] [--walk N] [--act N] [--no-ui]
```

`--act N` on `sanic_demo` and `capture_sanic` starts at that act.
