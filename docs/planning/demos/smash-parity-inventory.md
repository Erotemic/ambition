# Smash parity — inventory and roadmap

What the platform-fighter vocabulary already has, what it does not, and the
order to close the gap. Status was read off HEAD on 2026-08-19; re-grep a row
before working it.

`✔` shipped · `~` partial (named in the row) · `▢` absent from source

## Defense

| Feature | | Where |
|---|---|---|
| Shield held, directional (front only) | ~ | `combat/util.rs::shield_blocks_hit`, `core::BodyShieldState` |
| Shield health, decay while held, regen | ✔ | `core::ShieldTuning`, `tick_shield_resource` |
| Shield break → dizzy hard-lock, ring shatters | ✔ | `break_shield`, `MovementOp::ShieldBreak` |
| Shieldstun (a blocked hit costs the blocker tempo) | ✔ | `ShieldTuning::stun_per_damage` |
| Shield pushback (a blocked hit costs the blocker space) | ▢ | needs a velocity authority the damage resolver does not hold |
| Shield shrink → poke (a small shield exposes limbs) | ▢ | — |
| Shield-drop lag | ▢ | — |
| Parry (perfect-shield window) | ✔ | `BodyShieldState::parrying` |
| Ground dodge roll, air dodge (once per airtime) | ✔ | `BodyDodgeState`, `AxisManeuverState::dodge_roll_timer` |
| Spot dodge | ▢ | — |
| Tumble → knockdown → tech → getup (roll / attack / stand) | ✔ | `core/movement/knockdown.rs` |
| Wall tech and ceiling tech | ▢ | the tech tests a surface landing only |
| Crouch cancel | ▢ | — |

## Grabs

| Feature | | Where |
|---|---|---|
| Grab as a relationship, not a hit | ✔ | `combat/capture.rs::CapturedBy` |
| Grab beats shield | ✔ | same |
| Pummel | ✔ | `CapturePummelRequested` |
| Four throws authored per fighter | ✔ | `characters/smash_capture.rs` |
| Mash escape | ✔ | `actor_monolith/features/ecs/capture.rs` |
| Timed hold limit | ✔ | `CAPTURE_HOLD_LIMIT_SECONDS` |
| Escape difficulty scales with victim damage | ▢ | `CAPTURE_ESCAPE_PER_PRESS` is a constant |
| Grab release (grounded/aerial) as its own beat | ▢ | — |
| Dash grab / pivot grab distinction | ▢ | one standing grab per fighter |
| Command grabs (a special that captures) | ▢ | the capture effect is reachable; no fighter authors one |

## Movement

| Feature | | Where |
|---|---|---|
| Full hop, double jump, wall jump, fast fall | ✔ | `core::movement` |
| Jump squat, and a release inside it short-hops | ✔ | `movement/simulation.rs::tick_jump_squat` |
| Short hop as its own authored height (not a velocity cut) | ▢ | — |
| Footstool jump | ▢ | — |
| Jostle / body pushback between fighters | ▢ | — |
| Ledge grab with intangibility window | ✔ | `core/ledge_grab/` |
| Ledge getup: climb / roll / attack | ✔ | `LedgeGetupKind` |
| Ledge jump getup | ✔ | `MovementOp::LedgeJump` |
| Ledge trump (stealing an occupied ledge) | ▢ | — |
| Ledge intangibility decay with repeated grabs | ▢ | — |
| Platform drop-through | ✔ | `core::collision_semantics` |

## Damage and knockback

| Feature | | Where |
|---|---|---|
| Percent damage, weight, scaled knockback | ✔ | `core/hit_response.rs` |
| Hitlag, hitstun | ✔ | same |
| DI | ✔ | `hit_response::di_adjust` |
| SDI | ▢ | — |
| Meteor / spike (downward launch) | ~ | `launch_dir` expresses it; no meteor-specific rule or cancel |
| Meteor cancel | ▢ | — |
| Stale-move queue (repeat use weakens) | ▢ | — |
| Rage (damage taken raises knockback dealt) | ▢ | — |
| Charge attacks, landing lag, autocancel | ✔ | `combat/moveset` |

## Match rules

| Feature | | Where |
|---|---|---|
| Stocks, elimination, outcome | ✔ | `combat/stocks.rs` |
| Blast zones | ✔ | `demo_smash_app/tests/the_stage_kills.rs` |
| Timer mode | ▢ | — |
| Sudden death | ▢ | — |
| Teams and friendly fire toggle | ~ | `MatchTeam` exists; no attack toggle |
| Items | ~ | `combat/held_items.rs` holds one; no pickup, throw or spawner |
| Final Smash | ▢ | — |
| Respawn platform | ▢ | — |

## Presentation

| Feature | | Where |
|---|---|---|
| Sprite rows authored for most poses | ✔ | `sprite_sheet::CharacterAnim` (56 rows) |
| Taunt: input verb, authored move, drawn row | ✔ | `ControlSlot::Taunt`, `moveset_authoring::taunt`, one per fighter |
| Directional taunts (up / down / side) | ▢ | the verb chain supports it; no fighter authors one |
| Taunt voice line or cue | ▢ | — |
| Grab / held / pummel / throw poses | ▢ | no rows |
| Shield bubble, shrinking and reddening with integrity | ✔ | `render/rendering/bubble_shield.rs` |
| Shield-up and dizzy sprite poses | ~ | `CharacterAnim::Block` exists, unrouted; no dizzy row |
| Rows drawn but never selected | ~ | `WallJump`, `Shoot`, `Aim`, `Charge`, `Special`, `Punch`, `LedgeClimb`, `Interact` |
| Hit sparks, KO burst, screen shake | ✔ | `ambition_vfx` |
| Shield-break shatter burst and tone | ✔ | `features/movement_fx.rs` |
| Grab / throw / parry SFX cues | ▢ | — |

## Roadmap

Ordered by fun per slice. Each exposes the numbers a Smash game keeps tunable
and leaves the values rough; tuning is not this lane's licence.

1. ~~**Shield as a resource**~~, ~~**shieldstun**~~ and ~~**Taunt**~~ — landed 2026-08-19.
2. **Grab presentation** — the missing sprite rows, cues and VFX for the grab
   chain that already simulates correctly.
3. **Shield pushback and shield-shrink poke** — the two halves of shield
   pressure that shieldstun does not cover.
4. **Meteor and meteor cancel** — tag a launch as a meteor, forbid recovery for
   a window, allow a cancel after it.
5. **Footstool** — landing on a head bounces the stomper and buries the stomped.
6. **Stale moves and rage** — two multipliers on the shared knockback road.
7. **Grab depth** — escape difficulty scaling with damage, dash/pivot grabs,
   grab release as its own beat.
8. **Ledge trump and ledge-intangibility decay.**
9. **Match rules** — timer, sudden death, friendly-fire toggle, respawn platform.
10. **SDI, spot dodge, crouch cancel, jostle, wall tech** — the remaining verbs.
