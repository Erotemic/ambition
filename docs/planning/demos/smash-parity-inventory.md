# Smash parity — inventory and roadmap

What the platform-fighter vocabulary already has, what it does not, and the
order to close the gap. Status was read off HEAD on 2026-08-19; re-grep a row
before working it.

`✔` shipped · `~` partial (named in the row) · `▢` absent from source

⛔ **A `▢` IS A CLAIM WITH A GREP BEHIND IT, OR IT IS NOT A ROW.** This table's
first pass filed eight features as missing that ship — tech, ledge-jump getup,
the short hop, and five sprite rows the picker already selects — because it was
assembled from doc comments rather than from consumers. A comment that says *"not
yet auto-routed"* measures when the comment was written, so **an inventory of
GAPS built from prose dates the prose, not the code.** Grep the consumer before
you add a `▢`, and before you work one.

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
| Footstool jump (teammates included, as the genre does) | ✔ | `features/ecs/footstool.rs`, `collision_semantics::feet_on_head` |
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
| Spike (a downward hit drives the victim, no attacker rebound) | ✔ | `rules::DownwardHitStyle::Spike`, declared by the smash stage |
| Meteor lock and meteor cancel (a spiked body cannot recover for a window) | ▢ | — |
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
| Shield-up sprite pose | ✔ | `pick_body_anim` draws `Block` off `shield.active` |
| Dizzy pose for a broken shield | ▢ | a broken guard draws its ordinary locomotion row |
| A captive draws as HELD | ▢ | nothing in the anim layer reads `CapturedBy` |
| Rows drawn but never selected | ~ | `Charge`, `Punch`, `LedgeClimb`, `Interact` |
| Hit sparks, KO burst, screen shake | ✔ | `ambition_vfx` |
| Shield-break shatter burst and tone | ✔ | `features/movement_fx.rs` |
| Grab / throw / parry SFX cues | ▢ | — |

## The rest of Ultimate's list

The mechanics above are the ones a platform fighter needs to feel like one.
These are the remainder of Smash Ultimate's surface, kept so the next session
does not have to enumerate the genre again. None is on the roadmap yet; promote
one when it has a reason.

**Ground movement.** Walk as a distinct gait from run · initial dash and
foxtrot · dash-dance · pivot and turnaround · run-cancel by crouch or shield.

**Air movement.** Directional air dodge (ours is one evade along the stick) ·
double-jump cancel · b-reverse and wavebounce · aerial drift out of hitstun ·
fast-fall out of a bounce.

**Attack surface.** Dash attack as its own verb · pivot smash · jab combos with
a rapid-jab finisher · charge storage · a two-frame ledge-vulnerability window ·
z-drop and item throws · edge-cancel.

**Defense.** Perfect shield as a RELEASE-timed parry (Ultimate moved it off the
press) · shield tilt to cover a limb · shield-drop into an aerial ·
directional-influence variants (SDI, ASDI, hitfall).

**Match surface.** Time, stamina and coin rulesets · sudden death · handicap ·
Final Smash and the meter alternative · items and item spawn rate · stage
hazards toggle · Battlefield and Omega stage forms · echo fighters · spirits and
equipment · training-mode readouts (damage-per-hit, launch distance, hitbox
overlay).

**Presentation.** Screen KO and star KO variants · the launch-star zoom · victory
poses and the results screen · announcer and per-fighter voice · damage-percent
shake · the shield-break dizzy stars.

## Roadmap

Ordered by fun per slice. Each exposes the numbers a Smash game keeps tunable
and leaves the values rough; tuning is not this lane's licence.

1. ~~**Shield as a resource**~~, ~~**shieldstun**~~, ~~**Taunt**~~ and ~~**Footstool**~~ — landed 2026-08-19.
2. **Grab presentation** — the missing sprite rows, cues and VFX for the grab
   chain that already simulates correctly.
3. **Shield pushback and shield-shrink poke** — the two halves of shield
   pressure that shieldstun does not cover.
4. **Meteor lock and cancel** — the spike already lands; what is missing is
   the window in which the spiked body cannot recover, and the cancel that
   ends it.
5. **Stale moves and rage** — two multipliers on the shared knockback road.
6. **Grab depth** — escape difficulty scaling with damage, dash/pivot grabs,
   grab release as its own beat.
7. **Ledge trump and ledge-intangibility decay.**
8. **Match rules** — timer, sudden death, friendly-fire toggle, respawn platform.
9. **SDI, spot dodge, crouch cancel, jostle, wall tech** — the remaining verbs.
