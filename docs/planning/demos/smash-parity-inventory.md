# Smash parity — inventory and roadmap

What the platform-fighter vocabulary already has, what it does not, and the
order to close the gap. Status was read off HEAD on 2026-08-19; re-grep a row
before working it.

`✔` shipped · `~` partial (named in the row) · `▢` absent from source

## ⭐⭐ THE TARGET IS SMASH-*LIKE*, AND WHERE THE GAMES DIFFER THE ANSWER IS A KNOB

**Jon, 2026-08-20, verbatim:**

> Our point is to build a smash-like game, not exactly ultimate. It would be nice
> if there was a set of knobs we could tune to reproduce ultimate, but it doesn't
> have to be ultimate. Reproducing smash 4 or brawl, or melee (bugs are not
> reuqired parity) would be nice too

⇒ **so a `▢` here means "no knob covers this", NOT "we differ from Ultimate".**
Where the games agree, a missing mechanic is research and you go and ship the
standard. Where they DIFFER FROM EACH OTHER — and they differ constantly — the
question is not *"which one is right"* but *"what is the knob, and what does each
setting reproduce"*. Picking one throws the others away.

```text
perfect shield     PRESS-timed (Smash 4, and ours) | RELEASE-timed (Ultimate)
```

⇒ that pair is `MovementTuning::parry_timing`, shipped 2026-08-20 as the FIRST
knob under this ruling. ⭐ **it is the worked example**: the question arrived as
"which one is right" (§27), the ruling reshaped it into "what is the knob", and
the answer changed no shipped body's feel because a knob's default is the
behaviour that already existed.

⚠ **bugs are not required parity.** Melee's wavedash and L-cancel are artefacts
of its physics rather than authored rules; reproducing Melee does not oblige
reproducing them. A knob that spans the games spans their RULES.

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
| Parry (perfect-shield window), press- or release-timed | ✔ | `MovementTuning::parry_timing` — `OnRaise` is Smash 4's (the default) and `OnRelease` is Ultimate's; the stage declares which via `MatchBody`. Drawn as its own row since 2026-08-20 |
| Shieldstun is VISIBLE, not just true | ✔ | `body_state_clip` asks for `shield_hit` off `BodyShieldState::stun_timer` — the beat a blocked hit costs a defender |
| Ground dodge roll, air dodge (once per airtime) | ✔ | `BodyDodgeState`, `AxisManeuverState::dodge_roll_timer` |
| Tumble → knockdown → tech → getup (roll / attack / stand) | ✔ | `core/movement/knockdown.rs` |
| Wall tech | ✔ | `knockdown::tick_knockdown` reads `BodyWallState`; `WALL_TECH_SPEED` pushes off the normal |
| Ceiling tech | ▢ | a head contact is not yet a surface the tech press can land on |

## Grabs

| Feature | | Where |
|---|---|---|
| Grab as a relationship, not a hit | ✔ | `combat/capture.rs::CapturedBy` |
| Grab beats shield | ✔ | same |
| Pummel | ✔ | `CapturePummelRequested` |
| Four throws authored per fighter | ✔ | `characters/smash_capture.rs` |
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
| Jostle / body pushback between fighters | ✔ | LANDED `da884be08`. AVOID PUSHOUT is about PORTALS (§25); body contact is an OPT-IN capability of the movement SWEEP — proposed motion constrained BEFORE integration, never separated after — and smash grants it to its cast. ⭐ it turned the whole suite green, 26/7 → 34/0. ⛔ an acceleration term cannot work: the kernel overwrites `vx` toward the input target. ▢ the resistance number (0.85) is an unmeasured feel choice |
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
| Spike (a downward hit drives the victim, no attacker rebound) | ✔ | `rules::DownwardHitStyle::Spike`, declared by the smash stage |
| Meteor lock (a spiked body cannot recover for a window; the window ending IS the cancel) | ✔ | `DeclaredCombatRules::meteor_lock_time`, declared 0.30 by the smash stage and 0.0 by versus |
| Crouch cancel (a crouching victim takes less launch) | ✔ | `DeclaredCombatRules::crouch_cancel_scale`, declared 0.85 by smash |
| SDI (a frozen body shifts itself during hitlag) | ✔ | `hit_response::smash_di_shift` off `MovementTuning::sdi_step`; a HOLD where the genre counts stick inputs |
| Rage (damage taken raises knockback dealt) | ✔ | `DeclaredCombatRules::rage_per_damage` + `rage_max_scale`, declared 0.004/1.4 by smash |
| Stale-move queue (repeat use weakens) | ✔ | `BodyStaleMoves` (a nine-slot ring of move-id hashes) + `DeclaredCombatRules::stale_step`/`stale_floor` |
| Charge attacks, landing lag, autocancel | ✔ | `combat/moveset` |
| Dash attack (Attack out of a RUN) | ✔ | `move_for_attack` asks `attack_dash` off `BodyMotionFacts::running`, ahead of the direction; authored by all fifteen fighters |

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

## ⛔⛔ HALF THE SHEET IS ART NOTHING CAN ASK FOR

Measured 2026-08-20 — Carl's published sheet against every string literal in
`crates/` and `game/`:

```text
133 rows drawn        66 of them mentioned NOWHERE in the code
```

⭐ **and that is ONE gap, not sixty-six.** A large group of the sixty-six names a
state the engine ALREADY HAS and simply never asks a sheet about:

```text
jump_squat                  AxisManeuverState::jump_squat_timer   ✔ asked, 2026-08-20
wall_tech · wall_tech_jump  landed the same day; the tech does not say WHICH surface
footstool_jump              MovementOp::Footstool fires; a one-tick op has no FACT
launch                      the first beat of a tumble, distinct from `tumble`
parry · shield_hit          ✔ asked, 2026-08-20 — `parrying()` and
                            `stun_timer > 0.0`, two rows in `body_state_clip`
shield_raise
· shield_release            TRANSITION beats: the sim publishes the STATE
                            (`active`), not its edges
shield_break_launch
· _fall · _collapse
· _recover                  ONE `break_timer` covers a four-beat sequence, and
                            the beat is a fraction of the break's own length.
                            ⛔ `break_timer / ShieldTuning::break_stun_time` is
                            NOT computable at the readers: both pose sites hold
                            `BodyShieldState` and no tuning, and reaching for
                            `MotionModel::shield_tuning()` there would put
                            presentation back inside policy internals.
                            ⭐ THE ANSWER IS ONE f32 ON THE COMPONENT:
                            `break_total`, stamped from the tuning at the moment
                            the guard shatters. Then `break_timer / break_total`
                            is the phase for every reader, threading nothing.
                            ⚠ it is NOT a derive memo — it is the authority for
                            how long THIS break was, written once, so a rewind
                            restores the same answer. Costs a codec edge, a
                            version bump and the three baselines
ledge_catch · ledge_drop
· ledge_jump · ledge_attack  LedgeGetupKind and MovementOp::LedgeJump exist
getup_attack · getup_roll
· tech_roll                 the anim doc already names why: ONE
                            `getup_invulnerable` flag, so the sim has not made
                            the distinction the rows draw
turnaround · teeter
· walk_stop · crouch_start
· crouch_end · stumble      locomotion detail with no published fact
smash_charge                MoveSpec::smash_charge_mult exists
grabbed_pummel
· grab_escape              capture states with no fact on the CAPTIVE
platform_drop               drop-through ships
prone_damage · ground_bounce
· splat · roll_back         floor-game detail
```

⚠ **the rest are genuinely unreachable** and should stay that way until a
mechanic wants them: bury, trip, item handling (`item_pickup` through
`item_throw`), sleep, stamina, and Carl's own flavour rows (`stargaze`,
`use_telescope`, `cosmic_drift`).

⇒ **the pattern this branch hit five times is systemic**: `grabbed`, `pummel`,
`dizzy`, `spot_dodge` and `dash_attack` were all drawn and never requested. The
fix was usually one row in `body_state_clip` or one verb in `bound()`, never a
frame of art. ⛔ **so "the sprite is missing" should be the LAST hypothesis**,
after "nothing asks for it".

⚠ **but `dash_attack` cost FOUR edits, not one, and that is the shape to expect
when the missing thing is a whole mechanic rather than a row.** The verb had to
be bound in `bound()`, registered in the runtime's verb vocabulary, given a
reachable STATE to select it (`BodyMotionFacts::running` — the first attempt read
the traversal dash's timer, which the fighter kit switches off), and then given
priority over the smash gesture, because the flick that enters a run is the same
input that makes a press a smash. ⭐ each of the four was invisible to the tests
that covered the previous one; only pressing the key in the host found the last
two.

## The rest of Ultimate's list

The mechanics above are the ones a platform fighter needs to feel like one.
These are the remainder of Smash Ultimate's surface, kept so the next session
does not have to enumerate the genre again. None is on the roadmap yet; promote
one when it has a reason.

**Ground movement.** Walk as a distinct gait from run · initial dash and
foxtrot · dash-dance · pivot and turnaround · run-cancel by crouch or shield.

**Air movement.** ~~Directional air dodge~~ — SHIPS, verified 2026-08-20:
`apply_dodge`'s airborne arm aims along the stick in the body's own frame, any
diagonal, and a neutral stick dodges in place. That IS Ultimate's, and the row
was describing it as a gap · ~~aerial drift out of hitstun~~ — SHIPS:
`apply_post_hit_input_gates` scales the axes by
`Platformer2dFeelTuningMonolith::hitstun_control_scale` once the HARD lock
clears, preserving the attack verb ⚠ **grep the INPUT GATE, not the movement
kernel** — the kernel never reads `hitstun_timer`, which is why a first pass
concluded there was no penalty at all · double-jump cancel · b-reverse and
wavebounce · fast-fall out of a bounce.

**Attack surface.** ~~Dash attack as its own verb~~ — landed 2026-08-20, and it
took THREE tries to become reachable ⛔⛔ **a GAIT is not the traversal dash**:
the selector first asked `BodyMotionFacts::dashing`, which is
`AbilitySet::dash`'s timer, and `SMASH_FIGHTER_KIT` switches that ability OFF on
purpose — so the move was unreachable in the only game that authors one, and
every unit test passed because each TOLD the selector the body was dashing.
`BodyMotionFacts::running` is the fact it wanted ⛔⛔ **and then a RUN had to
pre-empt the SMASH GESTURE**, because the two inputs are the same one: a
direction FLICK inside the window makes a press a smash, and flicking a
direction is how a player enters a run, so the canonical input (tap forward,
press Attack) produced `smash_forward`. All four games answer Attack-out-of-a-run
with the running attack and none lets a forward smash come straight out of one
⇒ no knob, ship the standard · pivot smash · jab combos with
a rapid-jab finisher · charge storage · a two-frame ledge-vulnerability window ·
z-drop and item throws · edge-cancel.

**Defense.** ~~Perfect shield as a RELEASE-timed parry~~ — SHIPPED as a KNOB
2026-08-20 (§27): `ParryTiming::OnRaise` is Smash 4's and `OnRelease` is
Ultimate's, and the stage declares which · shield tilt to cover a limb · shield-drop into an aerial ·
ASDI and hitfall (⚠ NOT SDI — that shipped 2026-08-20 and is in the
**Damage and knockback** table; listing it here as a remainder was the same
double-entry as the crouch-cancel row).

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
6. ~~**SDI**~~, ~~**crouch cancel**~~, ~~**wall tech**~~, ~~**spot dodge**~~ and
   ~~**jostle**~~ (all landed 2026-08-20); still open: ceiling tech.
7. **What the 2026-08-20 review left open.** ~~Dash attack keyed to the wrong
   state~~ and ~~the footstool claim's lifetime~~ landed the same day; three
   items did not:
   - ~~**Stale-move accounting counts CONTACTS, not USES.**~~ Landed 2026-08-20
     (v55): the recording folds into `mark_move_playback_landed_hits`'s
     false→true edge, which already meant *this use connected* for the
     OnHit/OnWhiff cancels, and the separate `Settle` system is deleted. A swing
     that catches two fighters staled the move twice.
   - ~~**`BodyStaleMoves` lives in the movement core**~~ — moved to
     `ambition_combat::stale` and registered as `combat.stale_moves` by combat's
     own seam (v55). `ActorMoveset` `#[require]`s it, so the bodies carrying a
     nine-slot history are the bodies that can land a move rather than every
     body in every platformer composition. ⚠ the behaviour was always opt-in
     through `stale_step`; a rule being switchable was never a reason for its
     STORAGE to be global.
   - **The shared evade control is still called `Dash`** — ⚠ TAKEN by the
     main lane 2026-08-20, not this one's to work. `action_scheme.rs`
     derives `ControlSlot::Dash` from `abilities.dash || abilities.dodge`, so a
     Smash fighter — `dash: false`, `dodge: true` — puts a **Dash** button on
     the touch overlay for a body that has no traversal dash. The kernel is
     already correct (it calls the shared channel a BURST internally and
     resolves `GroundDodge | AirDodge | Dash`); it is the upper half that still
     spells the old word. A Smash body must present **Dodge**.
