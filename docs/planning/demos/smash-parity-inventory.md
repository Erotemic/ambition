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
| Shield pushback (a blocked hit costs the blocker space) | ✔ | `ShieldTuning::pushback_per_damage`, applied inside the block via `GuardUnderFire` |
| Shield shrink → poke (a spent guard exposes the head and feet) | ✔ | `ShieldTuning::min_coverage`, `combat::util::guard_covers_hit` |
| Shield-drop lag | ▢ | — |
| Parry (perfect-shield window) | ✔ | `BodyShieldState::parrying` — ⚠ PRESS-timed (Smash 4's); Ultimate's is RELEASE-timed, asked as `awaiting-maintainer-decision.md` §25 |
| Ground dodge roll, air dodge (once per airtime) | ✔ | `BodyDodgeState`, `AxisManeuverState::dodge_roll_timer` |
| Tumble → knockdown → tech → getup (roll / attack / stand) | ✔ | `core/movement/knockdown.rs` |
| Wall tech | ✔ | `knockdown::tick_knockdown` reads `BodyWallState`; `WALL_TECH_SPEED` pushes off the normal |
| Ceiling tech | ▢ | a head contact is not yet a surface the tech press can land on |
| Crouch cancel | ▢ | — |

## Grabs

| Feature | | Where |
|---|---|---|
| Grab as a relationship, not a hit | ✔ | `combat/capture.rs::CapturedBy` |
| Grab beats shield | ✔ | same |
| Pummel | ✔ | `CapturePummelRequested` |
| Four throws authored per fighter | ✔ | `characters/smash_capture.rs` |
| Dash attack as its own move | ✔ | `MovesetContract::move_for_attack` asks `attack_dash` before the direction; authored by all fourteen fighters |
| Mash escape | ✔ | `capture.rs`; `DeclaredCombatRules::grab_mash_seconds`, 14.4f per press |
| Timed hold limit | ✔ | `grab_hold_max_seconds`; the baseline's flat 4.0s is `FLAT_GRAB_HOLD_SECONDS` |
| Escape difficulty scales with victim damage | ✔ | `grab_hold_base_seconds` + `grab_hold_per_damage`, Ultimate's 90 + 1.7p, read ONCE at the grab |
| Grab release (grounded/aerial) as its own beat | ▢ | — |
| Dash grab / pivot grab distinction | ▢ | one standing grab per fighter |
| Command grabs (a special that captures) | ▢ | the capture effect is reachable; no fighter authors one |

## Movement

| Feature | | Where |
|---|---|---|
| Full hop, double jump, wall jump, fast fall | ✔ | `core::movement` |
| Jump squat, and a release inside it short-hops | ✔ | `movement/simulation.rs::tick_jump_squat` |
| Short hop as its own authored height (not a velocity cut) | ▢ | — |
| Footstool jump — claims the press, costs no air jump, 4f i-frames, Team-Attack gated | ✔ | `features/ecs/footstool.rs`; grounded victim flinches, airborne one tumbles (`ae::footstool_victim`) |
| Phantom footstool (a target mid-move is not interrupted) | ✔ | the stomper still takes the bounce; `BodyMelee::phase()` is the committed test |
| Jostle / body pushback between fighters | ▢ | ⛔ ASKED, not skipped: `awaiting-maintainer-decision.md` §24 — Jon's "AVOID PUSHOUT" rule may or may not reach body-vs-body |
| Ledge grab with intangibility window | ✔ | `core/ledge_grab/` |
| Ledge getup: climb / roll / attack | ✔ | `LedgeGetupKind` |
| Ledge jump getup | ✔ | `MovementOp::LedgeJump` |
| Ledge trump (stealing an occupied ledge) | ~ | `features/ecs/ledge_trump.rs`; the trumped body is DROPPED, where Ultimate pops it outward into a brief helpless state |
| Ledge intangibility scales with airtime (a regrab earns near nothing) | ✔ | `ledge_grab_invuln_earned` off `AxisManeuverState::time_off_ledge` |
| Spot dodge (down + evade, in place) | ✔ | `MovementOp::SpotDodge`, `MovementTuning::spot_dodge_time`; the `spot_dodge` row already shipped |
| Platform drop-through | ✔ | `core::collision_semantics` |

## Damage and knockback

| Feature | | Where |
|---|---|---|
| Percent damage, weight, scaled knockback | ✔ | `core/hit_response.rs` |
| Hitlag, hitstun | ✔ | same |
| DI | ✔ | `hit_response::di_adjust` |
| SDI | ▢ | — |
| Spike (a downward hit drives the victim, no attacker rebound) | ✔ | `rules::DownwardHitStyle::Spike`, declared by the smash stage |
| Meteor lock (a spiked body cannot recover for a window; the window ending IS the cancel) | ✔ | `DeclaredCombatRules::meteor_lock_time`, declared 0.30 by the smash stage and 0.0 by versus |
| Crouch cancel (a crouching victim takes less launch) | ✔ | `DeclaredCombatRules::crouch_cancel_scale`, declared 0.85 by smash |
| SDI (a frozen body shifts itself during hitlag) | ✔ | `hit_response::smash_di_shift` off `MovementTuning::sdi_step`; a HOLD where the genre counts stick inputs |
| Rage (damage taken raises knockback dealt) | ✔ | `DeclaredCombatRules::rage_per_damage` + `rage_max_scale`, declared 0.004/1.4 by smash |
| Stale-move queue (repeat use weakens) | ✔ | `BodyStaleMoves` (a nine-slot ring of move-id hashes) + `DeclaredCombatRules::stale_step`/`stale_floor` |
| Charge attacks, landing lag, autocancel | ✔ | `combat/moveset` |

## Match rules

| Feature | | Where |
|---|---|---|
| Stocks, elimination, outcome | ✔ | `combat/stocks.rs` |
| Blast zones | ✔ | `demo_smash_app/tests/the_stage_kills.rs` |
| Timer mode | ✔ | `MatchRules::time_remaining`, DERIVED from `ActiveMatch::activated_on` (no rollback state); smash declares 8 minutes |
| Timeout tiebreak: stocks, then damage | ✔ | `stocks_match::clock_outcome` |
| Sudden death | ▢ | a level timeout is a DRAW; sudden death is a second match staged from the first's result |
| Teams and friendly fire toggle | ~ | `DeclaredCombatRules::friendly_fire` IS the toggle (smash declares `false`); no menu exposes it |
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
| Grab / held / pummel / throw poses | ✔ | the rows SHIP (Carl draws `grab_hold`, `pummel`, `throw_forward/back/up/down`, `grabbed`); `smash_capture::bound` asks per verb, `BodyAnimFacts::held`/`holding` for the two poses no move owns |
| Shield bubble, shrinking and reddening with integrity | ✔ | `render/rendering/bubble_shield.rs` |
| Shield-up sprite pose | ✔ | `pick_body_anim` draws `Block` off `shield.active` |
| Dizzy pose for a broken shield | ✔ | the `dizzy` row ships; `body_state_clip` asks for it off `FighterClipFacts::guard_broken` |
| A captive draws as HELD | ✔ | `BodyAnimFacts::held` → `FighterClipFacts` → the `grabbed` row, with `Hit` as the tail |
| Rows drawn but never selected | ~ | `Charge`, `Punch`, `LedgeClimb`, `Interact` |
| Hit sparks, KO burst, screen shake | ✔ | `ambition_vfx` |
| Shield-break shatter burst and tone | ✔ | `features/movement_fx.rs` |
| Grab / pummel / throw cues | ✔ | `smash_capture`: the reach, the impact and the release each burst |
| Parry cue | ▢ | — |

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

**Attack surface.** ~~Dash attack as its own verb~~ — landed 2026-08-20 · pivot smash · jab combos with
a rapid-jab finisher · charge storage · a two-frame ledge-vulnerability window ·
z-drop and item throws · edge-cancel.

**Defense.** Perfect shield as a RELEASE-timed parry — ⛔ ASKED, not skipped:
`awaiting-maintainer-decision.md` §25, because ours is Smash 4's press-timed
parry and BOTH are shipped genre standards · shield tilt to cover a limb · shield-drop into an aerial ·
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

1. ~~**Shield as a resource**~~, ~~**shieldstun**~~, ~~**Taunt**~~, ~~**stale
   moves and rage**~~ and ~~**the footstool's victim reaction**~~ — landed
   2026-08-19/20.
2. ~~**The missing sprite rows**~~ — landed 2026-08-20, and the row was WRONG:
   the art ships. Carl's sheet has `grabbed`, `pummel`, `throw_forward/_back/
   _up/_down`, `grab_hold` and `dizzy`; nothing ever asked the sheet for them.
3. **Grab depth** — ~~escape difficulty scaling with damage~~ (landed
   2026-08-20: `DeclaredCombatRules::grab_hold_*`, Ultimate's 90 + 1.7p read
   ONCE at the grab); still open are dash/pivot grabs and grab release as its
   own beat.
4. ~~**Ledge trump and ledge-intangibility decay**~~ — both landed 2026-08-20,
   and the second row's word was wrong: the genre buys the window with AIRTIME,
   not a regrab counter. Still open on the ledge: the trumped body is dropped
   where Ultimate pops it outward into a brief helpless state.
5. **Match rules** — ~~timer~~ (landed 2026-08-20, derived from the activation
   tick); still open are sudden death, a menu for the friendly-fire toggle the
   rules already carry, and the respawn platform.
6. ~~**SDI**~~, ~~**crouch cancel**~~, ~~**wall tech**~~ and ~~**spot dodge**~~
   (landed 2026-08-20); still open: ceiling tech, and jostle — which is ASKED
   rather than skipped (`awaiting-maintainer-decision.md` §24).
