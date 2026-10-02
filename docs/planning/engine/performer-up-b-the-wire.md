---
status: built; open for feel
owner: game/ambition_content/src/performer_moveset.rs
---

# The Performer's up-B is a wire, not a teleport

**State.** Built: the kernel's wire (`crates/ambition_platformer2d_core/src/movement/`),
the move and the rope (`performer_moveset.rs`, `movesets/performer.ron`), and
the `wire_probe` instrument. The six clauses below are measured. Feel is open:
nobody has played it. The tuning knobs are named in `performer_moveset.rs`.
Under [`../decision-principles.md`](../decision-principles.md) this is tuning
work for Jon. It blocks nothing.

## Maintainer intent

> *"Now we need to fix her up-b. It is not a teleport and should not get the
> teleport sound. It needs to be a rope or wire that reaches down from the sky
> (it can instantly appear as if it went from visible to invisible), but she
> doesn't teleport up, she gets lifted up by the wire, a fairly large vertical
> distance, and while she is being lifted by the wire her motion controls should
> let her swing like a pendulum so she has a bit of horizontal recovery with it
> too. So, this might need a bit of engine work to make the motion work and be
> expressable elegantly."*

## Measurement on the Smash stage, with presentation live

`cargo run -p ambition_app_tools --bin wire_probe -- right render`

| Clause | Measured |
|---|---|
| 1. Not a teleport | 32 ticks on the wire; largest single tick 16.6 px, against 215 px in one frame for the old teleport |
| 2. No teleport sound | `player.blink` emitted 0 times in the move |
| 3. A wire from the sky | rope drawn on 32 of 32 on-wire ticks, on the actor road |
| 4. She is lifted | rose 431.9 px, monotonically |
| 5. A fairly large distance | 0.90 platform widths, 1.80 times the fall blast depth, 2.01 times the teleport |
| 6. She swings | ±90.6 px across at the cut, leaving at ±169 px/s, an exact mirror |
| Still a recovery | `gates.recovery` spent at t0 |

Run `offstage` to start below the lip: a swing toward the stage lands her on the
boards by t90, and a swing away drops her past the blast line.

## Rules for similar moves

- Do not silence a teleport by muting the executor's cue. A move that runs the
  teleport executor is a teleport; the move must stop running it.
- A swing must not become flight. Keep the up-B slot's `gates.recovery`.
- Put a derived rate beside the profile it inverts (`winch_rate_for`), so the
  executor and the tests ask one function.
- Do not use a hard stop at a swing cap. It makes the exit speed depend on which
  side of a tick the stop occurs.
- Bound a measurement loop by the state it measures, not by a tick count. A
  `for _ in 0..N` loop that reads a value at the end measures the wrong moment.

## The proof bar

A Smash move is not done until it works in play. For a Smash move:

1. A moveset test proves the spec, not the move.
2. The simulation is not the game. Observe visible behavior through a host with
   a render app.
3. There are two visibility roads: `BodyPoseView` (the session's exploration
   player) and `FeatureView` (every actor, including a Smash fighter). A rule on
   one road only is not a rule.
4. Verify Smash moves on the Smash stage, not in an Ambition room.
5. Two authorities for one fact means that you delete one.
6. Guard both layers: an authoring guard that the move has one motion authority,
   and an executor guard that the engine moves the body.

### The instrument

`trap_probe` and `wire_probe` seat a real `smash_roster` on the Smash stage,
drive the production input road (`drive_control_frame`) and print per-tick
state on both visibility roads. Extend them; do not start from a unit test.

If you write a new rendering probe:

- `VisibleRenderMode::NoWindow` omits the render app, so presentation numbers
  from it are meaningless. Use `OffscreenGpu` through `build_visible_app_with`
  with a `HeadlessDisplaySurface`.
- A hand-stepped `app.update()` does not wait for the wgpu device. Pump
  `plugins_state()`, `finish()` and `cleanup()` first.
- `capture_scene` cannot drive a controlled press. Use it to look, not to stage.

## Also open

- **Art file names on disk.** The generated sheets are git-ignored. If your
  checkout still has `actor_*` files in
  `crates/ambition_platformer2d_actor_monolith/assets/sprites/`, regenerate or
  rename them to `performer_*`. `declared_art_resolves` shows when it is done.
- **Trap duration (tuning, Jon's call).** She crosses the ~465 px stage in 1.9 s
  against a 3.0 s beat, so from mid-stage she waits at a ledge for more than half
  of it. `MAX_UNDER_S` is the knob.
