# Benchmark candidate: a new grounded-state branch can be silently shadowed by an upstream phase that flips `on_ground`

## Context

Ambition's directional player attacks resolve from a `(player_state, axis_x,
axis_y, pogo_pressed)` tuple inside `ae::resolve_attack_intent` and pick an
`AttackIntent` whose `attack_spec(...)` carries hitbox / damage / `can_pogo`.
The new design (`5bfd5e4 Down pokes, and fixes`) split the old combined
`Down | AirDown` spec in two:

- **Grounded `Down`** is now a kneeling forward poke (Marth-style down-tilt):
  forward hitbox at knee height, `damage_kind: Slash`, `can_pogo: false`,
  `self_impulse: (0, 0)`.
- **Aerial `AirDown`** keeps the old downward spike: vertical hitbox below
  the player, `damage_kind: Pogo`, `can_pogo: true`.

Both intents are reachable purely by reading `resolve_attack_intent`: it
returns `Down` when `forced_pogo || axis_y > 0.25` *and* `player.on_ground`,
else `AirDown`.

After the change shipped and the player sheet was regenerated, the grounded
kneeling-poke was unreachable from gameplay: pressing down+attack on a
normal floor consistently produced the aerial spike animation and hitbox.
The user reported it twice — "Why can't I down poke anymore?" — including
once after a candidate fix that only addressed a downstream `vel.y = 80`
clobber.

## Symptom

- `AttackIntent::Down`'s `attack_spec` was correct in source.
- The `attack_down` sprite row, `CharacterAnim::AttackDown` mapping, and
  `directional_attack_anim` dispatch were all correct.
- The sandbox `start_attack` (in `attack_phase`) gated on
  `controls.attack_pressed || controls.pogo_pressed` and reached
  `resolve_attack_intent` correctly.
- Despite that, every grounded down+attack played the `AirDown` row and
  applied the aerial spike hitbox, never the kneeling poke.

## Root cause

`sandbox_update` runs the engine's player control on `frame_dt` (phase 4,
`player_control_phase`) **before** the sandbox attack phase (phase 10,
`attack_phase`). The engine's `handle_attacks` looked like this:

```rust
let can_pogo = player.abilities.pogo;
if input.pogo_pressed && can_pogo {
    if let Some(orb_aabb) = try_pogo(world, player, tuning) { /* ... */ }
} else if input.attack_pressed {
    if can_pogo && input.axis_y > 0.25 {
        if let Some(orb_aabb) = try_pogo(world, player, tuning) { /* ... */ }
        // ...
    }
}
```

And `try_pogo` (in `movement/collision.rs`) builds a small hitbox 18 px
below the player's feet and treats `BlockKind::Solid` as a valid pogo
target — so it considers any normal floor a pogo surface:

```rust
let hitbox = Aabb::new(
    Vec2::new(feet.center().x, feet.bottom() + 18.0),
    Vec2::new(feet.half_size().x * 0.76, 22.0),
);
let hit = world.blocks.iter().find(|block| {
    let valid_target = matches!(
        block.kind,
        BlockKind::PogoOrb | BlockKind::Solid
            | BlockKind::BlinkWall { .. } | BlockKind::Rebound { .. }
    );
    valid_target && hitbox.strict_intersects(block.aabb)
});
if let Some(block) = hit {
    player.vel.y = -tuning.pogo_speed;
    player.on_ground = false;       // ← critical mutation
    Some(block.aabb)
}
```

Pressing down+attack on a normal floor therefore meant:

1. **Phase 4 (engine `handle_attacks`):** `try_pogo` matched the floor under
   the player's feet, bounced the player up, and flipped `on_ground = false`.
2. **Phase 10 (sandbox `start_attack`):** `resolve_attack_intent` saw
   `axis_y > 0.25` **and** `!player.on_ground` → returned `AttackIntent::AirDown`,
   not `Down`. The grounded kneeling poke branch was unreachable from gameplay.

The new branch in `resolve_attack_intent` was correct in isolation; what
broke it was a **prior-phase mutation of one of its inputs (`on_ground`)**.

## Failed first repair

A natural first instinct is to look at the sandbox `start_attack`. The user
reported the issue after a kneeling-poke fix that only changed this:

```rust
// before
if !controls.pogo_pressed
    && matches!(intent, ae::AttackIntent::AirDown | ae::AttackIntent::Down)
    && runtime.player.vel.y < 80.0
{ runtime.player.vel.y = 80.0; }

// after
if !controls.pogo_pressed
    && intent == ae::AttackIntent::AirDown
    && runtime.player.vel.y < 80.0
{ runtime.player.vel.y = 80.0; }
```

That's a real bug (the legacy clobber would punch a stationary kneeling
poke into the ground / through one-way platforms), but it didn't restore
the kneeling poke. By the time `start_attack` saw `intent`, the intent
had already been classified as `AirDown` — the velocity clobber was
firing on the *wrong* intent, but downstream of the actual mis-classification.

## Real repair

Pogo is fundamentally an aerial verb. Gate both `try_pogo` entry points
in the engine's `handle_attacks` on `!player.on_ground`:

```rust
let can_pogo = player.abilities.pogo && !player.on_ground;
if input.pogo_pressed && can_pogo {
    /* try_pogo ... */
} else if input.attack_pressed {
    if can_pogo && input.axis_y > 0.25 {
        /* try_pogo ... */
    } else {
        /* generic recoil */
    }
}
```

After the gate, the down+attack flow becomes:

- **Grounded:** engine no longer touches `on_ground` or `vel.y`. Sandbox
  `start_attack` sees `axis_y > 0.25, on_ground=true` → `AttackIntent::Down`
  → kneeling poke fires.
- **Airborne:** unchanged. `try_pogo` still runs, can still bounce off pogo
  orbs / blink walls / rebound blocks / ledges from above, and `AirDown`
  still resolves with `can_pogo: true`.

## Benchmark question (Level A)

You're maintaining a 2D platformer's player combat. The codebase has a
sandbox/engine seam: per-frame updates run in a fixed phase order — the
engine handles control input + physics in early phases, then a sandbox
phase later resolves a directional `AttackIntent` from
`(player_state, axis_x, axis_y, pogo_pressed)` and spawns the attack
hitbox.

You're asked to split the existing combined `Down | AirDown` attack spec
in two: grounded `Down` becomes a Marth-style kneeling forward poke
(forward hitbox at knee height, no pogo), and aerial `AirDown` keeps the
existing downward spike.

You add the new `AttackIntent::Down` spec, give it a forward hitbox,
update the animation mapping, regenerate the sprite sheet, and verify
`resolve_attack_intent` returns `Down` when `axis_y > 0.25 && on_ground`.
`cargo check` passes. Every unit test on the attack spec passes.

The user reports they cannot trigger the new kneeling poke from gameplay.
Every grounded down+attack still plays the aerial spike.

What is the most likely class of failure, and where would you look first?

## Expected answer

The new intent branch is correct in isolation — the failure is in a
**prior phase mutating one of `resolve_attack_intent`'s inputs**. In a
phase-ordered pipeline where two systems read the same input
(`attack_pressed`, `axis_y`, `pogo_pressed`) and one runs first, the
early system can silently reclassify the late system's view of player
state. Specifically, an upstream pogo helper that treats ordinary `Solid`
floor blocks as valid pogo targets will, on press, set `vel.y` and
**flip `player.on_ground` to `false`** before the sandbox phase reads it
— so the late `resolve_attack_intent` call sees an airborne player and
returns `AirDown`, never reaching the grounded `Down` branch.

Investigation order:

1. Confirm `resolve_attack_intent`'s logic with the same inputs you
   *expect* the sandbox phase to see (`axis_y > 0.25, on_ground=true,
   pogo_pressed=false`). If those return `Down`, the spec is fine.
2. Diff `player` state between "just before phase 10" and "just after
   input is sampled." Look specifically at `on_ground`, `vel`, `facing`,
   `body_mode`. Any earlier system that wrote to a state field consumed
   by `resolve_attack_intent` is a suspect.
3. Audit *every* prior phase that reads `attack_pressed` / `pogo_pressed`
   for state mutations gated on those inputs — the engine's
   `handle_attacks` and any pogo/dash helper are obvious candidates.
4. Repair at the source by restricting the upstream mutation to the
   semantic scope where it makes sense — here, "pogo can only fire when
   airborne." Do not paper over by special-casing the intent downstream:
   that leaves the silent reclassification in place for any future
   grounded-down branch.

The fix is a single gate in `handle_attacks`:

```rust
let can_pogo = player.abilities.pogo && !player.on_ground;
```

Validate with the pre-existing pogo regression tests
(`pogo_bounce_records_orb_aabb_on_frame_events`,
`forced_pogo_slash_is_below_player`) and a new test that exercises
"grounded down + attack → `AttackIntent::Down`" through the full
phase-ordered update — not just `resolve_attack_intent` in isolation —
so any future early-phase mutation that reclassifies the input is caught.

## What this tests

- Whether the agent recognizes that a *correct* late-phase branch can be
  unreachable because an early phase silently mutates one of its inputs.
- Whether the agent debugs by auditing **cross-phase state writes**
  rather than re-reading the local switch.
- Whether the agent prefers a fix at the upstream source (semantic
  scope of pogo) over a downstream workaround (force `on_ground` back
  on, or special-case the intent in `start_attack`).
- Whether the agent's regression test reaches through the actual phase
  order, not just the function whose output it cares about.

## Tags

`game-input`, `cross-system-signal`, `architecture-seam`,
`phase-order-coupling`, `edge-vs-held-state`.
