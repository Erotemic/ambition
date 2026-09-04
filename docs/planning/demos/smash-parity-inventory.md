# Smash parity — current feature inventory

This is the working inventory for the Smash demo: what already ships, what is
partial, and what can be added cleanly with the engine we have now.

Source audit: 2026-08-22, snapshot `1e6f8f81`; fifteen rows re-marked 2026-08-23
after the first push of the fun campaign. Re-check the named seam against HEAD
before changing a row.

⚠ **A `✔` HERE MEANS THE MECHANISM IS WIRED, NOT THAT ANYBODY HAS SEEN IT.** Four
rows on this page were `✔` all week while doing nothing in a match — the smash
charge, directional influence, the tech, and the launch trail — and the thing
that caught each of them was
`cargo run -p ambition_demo_smash_app --bin smash_tool -- match-report 30 --runs 5`, not a
test. ⇒ before marking a row shipped, make it appear in that report or say in the
row why it cannot.

⛔⛔ **RE-RUN 2026-08-26 (`30 --runs 3`, George vs George on the demo shell) AND
FOUR MORE SHIPPED MECHANICS ARE INVISIBLE IN A MATCH:**

```text
shielding    0–2–5      the CPU essentially never raises a guard
parries      0–0–0      so the perfect shield never fires
held         0–0–0      no grab was ever held — the whole capture kit, unseen
downed       0–0–0      tumbling was constant (121–203–401) and NOBODY landed
                        while helpless: a tumbling CPU jumps or attacks out the
                        moment the helpless window ends
```

⭐⭐ **AND THE CAUSE IS ONE THING, BUT NOT THE ONE I FIRST WROTE (which was "the
brain never presses shield" — READ THE SCORER BEFORE BLAMING IT).** The fighter
brain does offer both verbs; they are valued in a way that collapses the triangle:

```text
Shield   offered ONLY in Disadvantage AND only while a hostile
         `phase.is_attacking()` (options.rs:752) — deliberate, and the comment
         says why: shield used to be an ABSORBING STATE two cornered fighters
         entered in the opening second and never left
Grab     `capture_value` = GRAB_BEATS_GUARD (only if the foe's shield is UP)
         + damage_frac × THROW_CONVERSION
Parry    needs a shield RAISE at the right moment
```

⇒ **the legs are valued in terms of each other.** Shield is rare by design →
`GRAB_BEATS_GUARD` almost never applies → a grab is worth only the percent term,
which is near zero early and moot late, because by then the KO comes first →
grabs never happen → the shield is never punished for being rare. ⭐ **the
rock-paper-scissors triangle is stable at "everybody attacks", and every leg is
individually well-reasoned.**

⇒ so this is a TUNING question about the scorer, not a wiring gap in shield,
parry or capture. ⛔ do not "fix" any of those three mechanics on this evidence.

⭐ **AND THE "BEFORE" IS ON RECORD NOW, which is the expensive half of changing
it.** Whoever tunes the scorer should re-run the same command and compare
`shielding`, `parries`, `held`, `moves`, `damage` and `KOs` against the numbers
above — plus the repertoire probes, for the same reason D117 asks a first authored
body to run them: a scorer edit moves what the whole cast does, not one character.

⚠ **the genre's own answer is worth naming before anybody invents one.** Smash
CPUs shield because approaching into a guard is PUNISHED (the shield grab) and
because a held shield DECAYS. Both mechanics ship here. What is missing is that a
grab is priced almost entirely as a guard-beater, while in the genre it is also
the combo STARTER at 0% — which is exactly the term that would break the
stalemate without touching shield at all.

⛔⛔ **AND THAT TERM'S ABSENCE IS DELIBERATE — the constant says so, so do not
"fix" it as an oversight.** `THROW_CONVERSION`'s own doc: *"Kept well under
`GRAB_BEATS_GUARD` so that percent alone never makes a grab the answer to a
neutral opponent standing out of reach."* ⇒ somebody already considered grabbing
a neutral opponent and priced it down ON PURPOSE.

⚠ what is NEW is the evidence, not the idea: **zero grabs in ninety seconds of
play**, and a shield leg too rare to ever justify the guard term. ⇒ this is
"overrule a documented tuning decision on evidence it did not have", which is
Jon's call rather than a refactor — and if it is taken, note that the doc's stated
worry was reach, and reach is already `reach_fit`'s job, not this function's.

⚠ **`downed` is the one that might be correct.** A CPU with perfect reactions
acts out of tumble as soon as it may; a human eating a spike does not. Worth one
playtest before anybody tunes the knockdown window.

⭐ **what the same run confirms IS live**: tumble (121–401 ticks), launches
(8–11–14), KOs (2–3–4), peak launch 1086–2591 px/s, hitstun 475–694, evading
82–159, and the charge reaching full at least once (`best charge` max 1.00).

**Authority:** this file is the one current Smash feature backlog — and since
2026-09-03 it is also the execution order, because there is no live campaign.
Product intent lives in
[`super-smash-siblings.md`](super-smash-siblings.md); reusable combat ownership
lives in [`../engine/combat-model.md`](../engine/combat-model.md). Do not copy
open rows into another standing Smash plan.

⛔ **This line used to say execution order lived in
[`campaigns/smash-fun-push-2026-08-22.md`](campaigns/smash-fun-push-2026-08-22.md)
"for the active push". That campaign is CLOSED** — its own header says *"do not
use this file as Smash feature status"* and names THIS file as the successor —
and it is the only file in `campaigns/`. ⚠ So the two pages pointed at each
other, one of them wrongly, and a reader arriving from either end could bounce
between them without finding the live list. Read the campaign only for the
standing lessons it deliberately retains.

`✔` shipped · `~` partial · `▢` absent

⛔⛔ **RE-GREP A ROW BEFORE WORKING IT, AND DISTRUST THE DEFENSIVE AND HIT-PAYLOAD
REGIONS FIRST.** Five rows were corrected on 2026-08-25 alone, all of them
claiming work that HAD ALREADY LANDED: *Armored move* and *Invincible
move/startup* (`project_move_defense_windows` consumes both tags and is
scheduled), *Hitfall* (nothing gates fast-fall on being mid-move — the row wanted
something unblocked that was never blocked), and *Vacuum/suction* (the windbox
with its launch aimed inward — no second mechanic).

⭐ **THE STALENESS IS NOT SPREAD EVENLY**, which is the useful half: it clusters
where work has recently happened, because those rows were written before the work
and never revisited. ⇒ trust the untouched regions; verify the ones next door to
whatever last shipped.

⛔⛔ **AND THE EXAMPLE THIS PARAGRAPH USED HAS SINCE EXPIRED — which is the
sharpest possible case of its own rule.** It read: *"Spot-checked the same day
and found ACCURATE: the whole ground-locomotion block (`Initial dash`,
`Turnaround / pivot phase`, `Teeter` — none of these exist in any form)."*
Re-checked 2026-09-03 and all three exist:

* `LocomotionTuning` carries `initial_dash_time`, `turnaround_time` and
  `teeter_margin` (`crates/ambition_platformer2d_core/src/abilities.rs:889-897`);
* the movement authority runs the turnaround clock —
  `state.turnaround_timer` against `tuning.locomotion.turnaround_time`
  (`crates/ambition_platformer2d_core/src/movement/abilities.rs:265-300`);
* and **this page's own row already said so**: *"Teeter at platform edge | ✔ |
  … LANDED 2026-08-25. `LocomotionTuning::teeter_margin`"*.

⇒ So the page contradicted itself, and the stale half was the one presented as a
VERIFICATION RESULT. ⚠ **A dated spot-check is evidence with a shelf life, and
this one was being read as a standing fact** — the paragraph's own advice
("verify the ones next door to whatever last shipped") is exactly what would
have caught it, applied to the paragraph itself. The rule survives; only its
example was overtaken.

Effort: `S` small slice · `M` medium slice · `C` multi-slice campaign

Engine column:

- `—` — content, demo, UI, or presentation work; no new engine mechanic.
- `E1` — one small reusable engine semantic is needed. This is good feature-driven
  engine work and should be done now when the feature is implemented.
- `E2` — coordinated engine work across several systems, but ownership is clear.
  Treat it as its own campaign rather than hiding it inside fighter content.
- `WAIT` — do not expand the current architecture for this feature yet.

## Scope and implementation rules

The target is Smash-like platform fighting, not byte-for-byte Ultimate. When
Smash games differ on an authored rule, prefer a tuning/rules knob whose default
preserves the current game. Physics bugs do not need parity.

For every missing feature:

1. Re-grep the current consumer before implementing it. A missing animation
   selection is not evidence that the mechanic or art is missing.
2. Human and CPU fighters use the same body/combat mechanic. Do not add a
   player-only simulation path.
3. Put gameplay truth in the owning simulation domain. Presentation consumes
   resolved facts/events; shaders and particles do not infer hit eligibility,
   charge state, or shield state independently.
4. Add a reusable semantic only when a real feature needs it. Do not build a
   generic platform-fighter scripting framework in advance.
5. Do not wait for the actor-monolith carve, simulation-phase migration, or
   capability/runtime composition cleanup. The gaps marked `E1`/`E2` below have
   usable owners now.
6. Broad fighter-AI architecture work is the exception: the AI-policy ownership
   migration is still open. Add only the smallest option/observation support
   needed for a new mechanic; defer large strategy rewrites.

## Shipped baseline

These are not backlog items. Preserve them and build new features on their
existing seams.

| Area | Current capability | Where |
|---|---|---|
| Move authoring | Timeline windows, local hit volumes, move events, gates, self-motion, landing lag and autocancel | `ambition_entity_catalog::MoveSpec`, `MoveWindow`, `HitVolume`; `ambition_combat::moveset` |
| Multi-hit moves | Separated Active windows can re-hit; contiguous Active windows preserve the victim set for one moving hit track | `ambition_combat::moveset` |
| Cancels | Authored cancel windows support `Always`, `OnHit`, and `OnWhiff` | `WindowTag::Cancelable`, `CancelCondition` |
| Ground attacks | Directional attacks, smashes, and dash attack selection | `trigger_moveset_moves`, `BodyMotionFacts::running` |
| Aerial attacks | Directional aerial move selection, landing lag, autocancel | `ambition_combat::moveset` |
| Damage | Percent, weight, scaled knockback, hitlag, hitstun, DI | `ambition_platformer2d_core::hit_response` |
| Smash rules | SDI, crouch cancel, rage, stale-move queue, spike/meteor-lock policy | `DeclaredCombatRules`, `BodyStaleMoves` |
| Shield | Health, drain, regen, shrink-to-poke, shieldstun, pushback, break and dizzy | `BodyShieldState`, `ShieldTuning`, combat shield resolution |
| Parry | Press-timed or release-timed perfect shield is already a rules knob | `MovementTuning::parry_timing` |
| Evade | Ground roll, spot dodge, directional air dodge, one air dodge per airtime; shield+direction is the grounded evade and shield IN THE AIR is the air dodge | movement dodge state/facts |
| Knockdown | Tumble, knockdown, floor/wall/CEILING tech, getup stand/roll/attack | `movement/knockdown.rs` |
| Jumping | Full hop, jump squat/short hop, double jump, wall jump, fast fall | `ambition_platformer2d_core::movement` |
| Hitstun drift | After the hard lock, directional control returns at the authored hitstun-control scale | post-hit input gates + movement tuning |
| Body contact | Fighter jostle/body pushback | movement sweep/body contact |
| Footstool | Grounded/airborne victim reactions and phantom-footstool behavior | `combat/src/footstool.rs` |
| Ledge | Grab, intangibility, climb/roll/attack/jump getups, trump ownership, drop-through | `ledge_grab`, `ledge_trump` |
| Capture | Grab relationship, shield bypass, pummel, four throws, mash escape, damage-scaled hold time | `ambition_combat::capture`, `characters/smash_capture.rs` |
| Dash grab | A running grab is derived from each fighter's standing grab | `SmashCaptureRepertoire`, `grab_dash` |
| Match | Stocks, blast zones, elimination, timer, stock/damage timeout tiebreak | `ambition_combat::stocks`, `features/stocks_match.rs` |
| Teams | Friendly-fire policy exists in match combat rules | `DeclaredCombatRules::friendly_fire` |
| Respawn | Per-seat placement, velocity reset, timed untouchable grant | Smash respawn placement/empowerment systems |
| Items | World item identity/custody, pickup, held-item use, throw, item physics | `items/pickup`, `GroundItem`, `ItemCustody`, `HeldItem` |
| Presentation | Character pose routing, shield ring, hit sparks, KO burst, hit-strength camera shake, shield-break burst | render/VFX/movement FX |
| Match ceremony | 3–2–1–GO and basic winner presentation | Smash demo match presentation |
| Character select | Per-connected-pad cursor before joining, role cycle, fighter selection, selecting a fighter auto-claims an absent slot, random selection | `game/ambition_demo_smash/src/select*` |
| Frontend exit | Universal pause/system menu can Quit to Title from character select | `ambition_game_shell::pause_menu` |
| Taunt | A generic taunt action/move is reachable; directional variants remain backlog | Smash input/moveset routing |
| Input customization | Per-user binding overrides/remaps, controller profiles/deadzones, `StrongAttack` semantic hint, analog `AimStick` | `ambition_input` |

## 1. Attack and move semantics

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| True hold/release smash charge | ✔ | M | E1 | `smash_charge_mult` already scales payoff, but charge is inferred from ordinary Startup time. Add optional charge policy to `MoveSpec` and per-use hold/release state to `MovePlayback`; freeze the timeline at the authored hold point, release on button-up/max, and freeze the charge fraction for the rest of that use. |
| Charge-start/full cues and charge pose | ✔ | S | — | Presentation reads the resolved charge state; one latch/cocking cue on entry, one loaded cue at max, accelerating pulse while charging. No input polling in rendering. |
| Charge storage | ✔ | M | E1 | SHIPPED 2026-08-27 as `SmashChargeSpec::stores`, and it landed with the caveat this row asked for: OPT-IN, default `false`, so no smash stores charge unless its own spec says so. A full charge waits instead of firing itself and an interrupted one is banked for the next press, decided in `cancel_move_playback` — the one teardown path — from a fact the playback already carries. Authored customer: Projectile Polygon's power ball (`projectile_polygon_moveset.rs`). Re-marked 2026-08-28; the ▢ was stale. |
| Jab 1 → 2 → 3 chains | ✔ | — | E1 | `jab_string_continuations()` authors jab2 and jab3 as their own moves and pushes them into BOTH shipped tables; a held Attack continues the string only to a successor the playing window NAMES, so a move with no chain does nothing — which is what makes it safe for a brain to hold every neutral basic. |
| Rapid jab + finisher | ✔ | — | E1 | `MoveSpec::repeat: Option<MoveLoop>` — an authored `from_s`/`to_s` jump with its own timeout, and the finisher starts where the loop's stretch ends (`FLURRY_FROM_S` / `FLURRY_TO_S`, named because four places must agree). No fighter id anywhere in it. |
| Combat action input buffer | ✔ | M | E1 | ⭐ SHIPPED, and this cell's *"nothing writes them"* was STALE — re-measured 2026-08-31 after the queue recorded the correction but never applied it here. `BodyActionBuffer` is rollback-registered and has FOUR writers, each arming its slot with `tuning.action_buffer_s` at a semantic press edge: `attack` (with the resolved intent, so a buffered press replays VERBATIM rather than being reinterpreted from the live stick), `grab`, `pogo`, and `special` (which used to be a bare timer — press Up+Special during endlag and it came out NEUTRAL). `tick` decays every slot through an exhaustive destructure, and `proposer.spend(action_buffer)` spends it only when the action authority ACCEPTS, which is exactly what this row asked for. `spend_attack` clears attack AND pogo together because both resolve to one timeline. No per-move grace timers; no raw device input. |
| Move invulnerability windows | ✔ | S | E1 | `WindowTag::Invuln` already exists in the authoring schema but runtime does not consume it. Make hit eligibility read the active move window. |
| Move super-armor windows | ✔ | M | E1 | `WindowTag::Armor` already exists. Resolve hit damage normally while suppressing/altering reaction according to the active window. |
| Damage-threshold armor | ▢ | M | E1 | After basic armor is live, make armor policy carry a threshold when a real move needs it. Keep it in hit reaction, not character code. |
| Knockback-threshold armor | ▢ | M | E1 | Same reaction-policy seam; compare resolved launch/reaction, not attacker identity. |
| On-block cancel windows | ✔ | S/M | E1 | ✔ SHIPPED 2026-08-31. `CancelCondition::OnBlock` exists and fires. The one bool that meant OVERLAP became `MoveContact { overlapped, connected, blocked }`: `OnHit` asks `connected` (so an ordinary held guard no longer confirms it), `OnWhiff` asks `!overlapped` (a blocked move touched something and is not a whiff), `OnBlock` asks `blocked`. The verdict comes from the damage resolver's own channels — `ResolvedBodyHit` and a new POSITIVE `BlockedBodyHit`, never from a missing connect, because the two roads resolve on different frames. Schema 143→144. ⚠ the connect-frame cancel window is unchanged against an ACTOR victim and opens one frame later against a PLAYER one. ✔ AND IT HAS A CUSTOMER SINCE 2026-08-31: George's jab nominates `grab` on block — a shielded jab buys the option that beats the shield that just ate it, which is the genre's canonical use of this window. ⛔ authored THIRD in his cancel list, because the chain takes the first successor it resolves by move id and `jab2` has to keep that slot; `grab` is a VERB and resolves through the contract's verb map, which `a_shielded_jab_buys_george_a_grab` asserts rather than assuming. |
| Sweetspot/sourspot hitboxes | ✔ | — | E1 | `StrikeRank` orders one move's live volumes by authoring order, and the STRIKE PULSE shares one per-victim ledger across a continuous Active interval — so sour-then-sweet within one pulse is ONE hit, and a real inactive gap earns a second. |
| Same-frame hitbox parts | ▢ | M | E1 | Build on hitbox arbitration when a move needs several independently meaningful regions rather than only a priority winner. |
| Fixed/set knockback | ✔ | S/M | E1 | ✔ SHIPPED, and this row was STALE — `knockback_growth: Option<f32>` says it outright: `None` = the stage's growth scales this hit, `Some(0.0)` = fixed knockback, launching the same at 0% and 200%. ⛔ it WAS an `f32` where zero meant both things, and the field's own doc described the behaviour you could not get; the `Option` is the explicit mode this row asked for. Multi-hit carry is built on it. |
| Autolink/follow-owner knockback | ✔ | M | E1 | ✔ KERNEL SHIPPED 2026-08-24: `HitKnockback::follow: Option<AutolinkFollow>` + `hit_response::autolink_velocity`, resolved in the shared `apply_body_hit_reaction`. Attacker-local anchor resolved AT THE PRODUCER through the ATTACKER'S frame and facing (⛔ this row said "rotated through the VICTIM'S `AccelerationFrame`" until 2026-08-25 — that is the design a review rejected and the code never had; a later agent following the row literally would have reintroduced it); `carry` hands over the attacker's own velocity (a rising move outruns any gap-closing term); `pull`/`max_speed` bound the correction only. NOT a capture — no relation, no clock, the victim keeps every verb. Exempt from crouch-cancel and from the meteor lock. Schema 77→78, the follow is in the hit fingerprint. ✔ AUTHORING WIRED: `AutolinkVolume` on the catalog `HitVolume` (serde-default; no shipped `.ron` changed) → `Hitbox` → the producer, which samples the ATTACKER'S VELOCITY because the reaction holds a victim and no attacker entity. ✔ AND SPENT: Pointed Polygon's `polygon_rising_edge` is four holding pulses then one launch, via the shared `multihit` combinator (campaign W3). ⛔ the pulse GAPS are load-bearing — the runtime's re-hit rule refuses a contiguous track, so touching windows land once and the mechanic silently vanishes. |
| Weight-independent knockback | ▢ | S/M | E1 | A compact reaction modifier on the hit payload; keep ordinary hits unchanged. |
| Windboxes / flinchless push | ◐ | M | E1 | MECHANISM LANDED 2026-08-25 (D215, `e06333002`). ⭐ HALF EXISTED ALREADY: `damage_floor` kept an authored `damage: 0` at zero, so a damageless volume was authorable and already LAUNCHED — what it still did was STUN and spend its hit-once slot. `WindboxVolume` is only that remainder: `flinchless` + `repeating`. ⛔ NO MOVE AUTHORS ONE; that is a character-design call, in `awaiting-maintainer-decision.md`. |
| Vacuum / suction hitboxes | ✔ | M | E1 | ⭐ NOTHING FURTHER TO BUILD: it is the row above with the launch aimed back at the owner. One authored move closes both. |
| Extra shield damage | ▢ | S/M | E1 | Author shield-resource damage separately from body percent; resolve it in shield contact, not by inflating normal damage. |
| Unblockable strike flag | ▢ | S/M | E1 | Add explicit guard-interaction policy to the hit payload. Do not infer unblockable behavior from move IDs. |
| Hitbox clanking | ~ | C | E2 | ◐ THE MECHANISM IS FINISHED AND PROVEN ON THE PRODUCTION ROAD; the STAGE declares it OFF pending a play session. Reopened once already for testing a surrogate — it filtered `With<HitboxLifetime>` and authored volumes carry none, so no Smash attack reached it. Now: queries `StrikeVolume`, orders by `SimId::strike_volume` (⛔ an `Entity` is an allocator identity and two peers index differently), resolves once per ATTACK PAIR (⛔ per-volume announced a 2×2 trade four times and rebounded four times), ends the losing MOVE rather than its rectangle, and only GROUNDED attacks clank — the genre's rule. ⛔⛔ TURNED ON IT RE-TUNES THE WHOLE GROUND GAME: at 9 damage two CPUs traded so constantly that `every_live_fighter_stays_inside_the_frame` measured ZERO body-frames outside the stage in a full match. `clank_damage_window: 9.0` is the number to try first. |
| Attack rebound after clang | ~ | S/M | E1 | ◐ Built and unit-tested, and it fires exactly when clanking is declared on — which the stage currently does not. `arbitrate_attack_clanks` owns ending the moves (so the stronger-wins case, which announces nothing, ends its loser by the same road) and `rebound_from_clanks` owns the push and the hard lock. |
| Cannot-clank/transcendent hit | ▢ | S | E1 | ⭐ UNBLOCKED and DELIBERATELY UNBUILT — it wants a customer. The arbitration exists, so this is one authored field consulted before the damage comparison; what is missing is a move that needs it. ⛔ the genre's transcendent hits are mostly PROJECTILES, and projectiles do not use `Hitbox` at all — they never reach the arbitration — so the obvious customer is already answered by the architecture. Build it the day a melee move wants to pass through a swing, the same rule D127's `when … then` form is waiting on. |
| Per-hit hitlag multiplier | ▢ | S/M | E1 | Optional scalar on `HitVolume`/resolved strike. Useful for multihits and heavy impacts without changing global hitlag tuning. |
| Per-hit hitstun multiplier | ▢ | S/M | E1 | Same payload; add only when a move needs reaction distinct from knockback. |
| Per-hit SDI multiplier | ▢ | S/M | E1 | Same payload; useful for multihit escape tuning. |
| Cannot-tech hit property | ◐ | S/M | E1 | ⭐ THE ARCHITECTURE THIS ROW ASKS FOR IS ALREADY BUILT, and by a DERIVED answer rather than an authored one. `AxisManeuverState::tumble_untechable` is stamped at `launch_into_tumble` — the one place the launch speed exists — and `tick_knockdown` reads it to refuse the tech press, which is exactly "stored in the resolved launch/reaction state so the tech system owns eligibility". The source is `untechable_launch_speed` (1400 for Smash): hard hits commit, which is the genre rule and needs no per-move authoring. ⛔ WHAT IS ACTUALLY MISSING is an OVERRIDE road — a move that is untechable at any speed — and that wants a customer, like the transcendent-hit row above. Build the field the day a move needs it; do not add an authoring surface whose only value would be to restate the derived answer. |
| Edge-cancel move recovery | ✔ | M | E1 | LANDED 2026-08-25. `DeclaredCombatRules::edge_cancel_recovery` (`Some(true)` for Smash, `None`/false everywhere else): an aerial's landing lag ENDS the moment ground support disappears, so landing on a platform lip and sliding off cancels the recovery. ⛔⛔ IT COULD NOT LIVE IN `resolve_aerial_landings`, and not for style: the lag OUTLIVES the playback — charging it cancels the move — so a body paying recovery has no `MovePlayback` and that query cannot see it. `edge_cancel_landing_recovery` is its own body-generic system over the two components every body has, chained straight after the landing that charges the lag so a body that lands and leaves in one frame is charged then released, never the reverse. ⛔ A RULE, NOT A PER-MOVE FIELD: every move cancels or none does; per-move would be an exemption list. Poisoned both ways (ignore the declaration; cancel regardless of ground). |
| Pivot smash | ✔ | S/M | E1 | ✔ 2026-08-25 — ALREADY TRUE THE DAY THE TURNAROUND LANDED, and the row's own warning is why: the pivot went in at `resolve_attack_gestures`, the ONE place a facing is folded into an aim, so every attack family inherits it and a smash needs no rule of its own. Pinned by `a_smash_thrown_out_of_a_turnaround_points_the_new_way` (a flick-then-press, because a same-tick press is a TILT and would have measured nothing). |

## 2. Defense, shield, evade, and tech depth

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Filled shrinking bubble shield | ✔ | S | — | Current `bubble_shield.rs` is a thin procedural ring but already receives integrity/parry facts. Replace the texture/material presentation; preserve the existing shield simulation. |
| Shield-hit ripple/deformation | ✔ | S | — | Drive a short cosmetic pulse from resolved shield contact/shieldstun. |
| Low-shield danger treatment | ✔ | S | — | Current ring shrinks/reddens; strengthen presentation without changing resource values. |
| Shield-drop lag | ✔ | — | E1 | `ShieldTuning::drop_lag`, 11 frames on `PLATFORM_FIGHTER` (Ultimate's), charged by `apply_shield` when a guard is let go BY ITSELF — an out-of-shield action already took it down through `spend_on_action`. `0.0` for a game that declares no rule. |
| Out-of-shield action policy | ✔ | — | E1 | `ShieldTuning::out_of_shield: Option<OutOfShield>` names five action CLASSES, and ONE gate reads it — `OutOfShieldGate` in the movement kernel, used by the kernel and the moveset trigger alike (unified `4a70be7e0`; combat keeps only the DIRECTION half, `rises_out_of_shield`). No per-move exceptions exist. |
| Shield grab | ✔ | — | E1 | Attack on a raised guard IS the grab (Jon, 2026-08-23), asked through `gate.permits(OutOfShieldAction::Grab)` like the dedicated button — a road to the grab, never an exemption from the policy. |
| Jump / Up-B / up-smash out of shield | ✔ | — | E1 | All three through the same gate: `Jump` in the kernel's `apply_intent`, `UpSpecial` and `UpAttack` through `rises_out_of_shield`, which is what makes only the UP directions rise. |
| Shield shift/tilt | ✔ | M | E1 | LANDED 2026-08-25. `ShieldTuning::tilt_range` (0.34 of half-height on `PLATFORM_FIGHTER`, `0.0` everywhere else) leans the guard along the body's OWN gravity; `apply_shield` resolves it once into `BodyShieldState::shield_tilt` and both consumers read that one value — `guard_covers_hit` SHIFTS its covered band, `ShieldRingsView` shifts the drawn bubble by the same half-height. ⭐ it competes with nothing: past `SPOT_DODGE_STICK` the stick is already a roll, so tilt lives in the band that was dead input. ⛔ THE SHIFT MUST COST: leaning toward the feet exposes the head by the same amount, else it is a free `min_coverage` upgrade nobody would ever let go of — poisoned both ways. ⛔ ONE AXIS: the lateral question is already answered by which side the body FACES. Wire format v92. |
| Shield drop through one-way platform | ✔ | S/M | E1 | LANDED 2026-08-25. `ShieldTuning::platform_drop` (true on `PLATFORM_FIGHTER`), no new collision rule — the same `drop_through_timer` grace the jump road sets, through one shared `begin_drop_through`. ⭐ THE TERRAIN ARBITRATES: guard+down is the spot dodge on solid ground and the drop on a soft platform, so no new gesture was needed. ⛔⛔ the existing drop-through was UNREACHABLE without a jump: it sits behind `if !current_press && buffer_jump <= 0.0 { return; }` inside the jump-buffer handler, so the gesture needed its own arm ahead of that gate. ⛔ an explicit declaration, NOT a fallthrough on `out_of_shield` — that gate reads "no policy" as "restricts nothing", which would have handed a platform drop to every exploration body with a shield. Wire format v93. |
| Dodge staling | ✔ | M | E1 | LANDED 2026-08-25 (`b75aee1af`). It lives on `BodyDodgeState` as the row said, not on the move-staling queue: an evade has no id, and what wears out is the OPTION. ⭐ IT WEARS THE I-FRAMES ONLY — a stale roll still travels and still recovers, it is simply no longer safe, which reads without a HUD. Smash: a quarter off per recent evade, floored at a third, forgiven one at a time every 1.2s; every other world declares nothing. ⛔ one helper for all three evades so the spot dodge, roll and air dodge cannot drift. |
| Spot-dodge attack cancel near tail | ✔ | S/M | E1 | LANDED 2026-08-25 (`89c608eda`). ⚠ the row understated it: the moveset consulted dodge state NOT AT ALL, so an attack cancelled a dodge on frame one. `evade_cancel_tail` commits the evade until its last N seconds; Smash declares four frames. ⛔ measured from the END, not the start — dodge staling shortens the window per body, so the authored total is the wrong total. `0.0` DISABLES the rule rather than committing everything. |
| Invulnerability/intangibility blink | ✔ | S | E1 | Publish one resolved presentation fact from actual hit eligibility (dodge, tech/getup, ledge, respawn, move invuln) and reuse the overlay-material pattern. |
| Tech flash/SFX | ✔ | S | — | Tech exists but currently shares light movement feedback. Route a distinct one-shot cue from successful Tech, separate from getup roll. |
| Parry flash/chime | ✔ | S | — | Trigger on successful parry contact, not merely on the parry window. Ordinary shield block stays visually distinct. |
| Ceiling tech | ✔ | — | E1 | `ground.head_contact` is the third tech arm in `knockdown.rs`, beside the wall's. It pushes DOWN off the surface it caught, the way the wall tech pushes along the wall normal, into a fall the body controls. Guarded by `a_tumbling_body_can_tech_off_a_ceiling`. |
| Wall-tech jump | ✔ | S/M | E1 | ⚠ THIS ROW WAS STALE — it shipped 2026-08-23 (`42570c0cd`), two days BEFORE the sweep that marked it `~`. The wall tech itself ships (`wall.on_wall` + a live tech press → an impulse along `wall_normal_x` at `WALL_TECH_SPEED`, position untouched — not a pushout), and the JUMP variant is guarded by `a_wall_tech_asked_to_jump_leaves_the_wall_higher_than_one_that_is_not`, which asserts `MovementOp::WallJump` in the same beat at no air-jump cost. Re-marked 2026-08-31. |
| Untechable high-launch threshold | ✔ | S/M | E1 | LANDED 2026-08-25 (`d3b4b4abb`). `untechable_launch_speed`, decided at `launch_into_tumble` — the only place the launch SPEED exists, since it is gone by the time the body reaches a surface. ⛔ THE PRESS is what an untechable tumble refuses, not each surface, so a fourth tech surface added later cannot quietly become techable. The flag clears on every tumble entry (a footstool is techable). Smash: 1400px/s against a 500px/s tumble line. |
| ASDI | ✔ | M | E1 | LANDED 2026-08-25. `TraversalAbilityTuning::asdi_step` (6.0 for Smash, `0.0` everywhere else), spent by `step_body` on the FIRST step after the freeze lifts, in the direction held then. ⭐ WHAT MAKES IT DISTINCT FROM SDI IS WHAT IT IS PAID PER: `sdi_step` is paid per TICK of hitlag, so a one-tick multihit freeze is worth almost nothing; this is paid once per HIT whatever the freeze was worth. ⛔ AT THE END, not the start — the defender has the whole freeze to choose, and paying at the start would just be one more SDI tick (poisoned: that version reddens arm 1). ⛔ A LATCH (`BodyCombat::asdi_owed`), not a `hitstop_timer <= dt` comparison, because the decay is a separate system whose order against the body step is not declared — a latch is answered by two consecutive steps of one function. Wire format v95. |
| Hitfall | ✔ by another key | M | E1 | ⚠ THIS ROW'S PREMISE WAS WRONG, measured 2026-08-25. Nothing gates fast-fall on being mid-move or mid-hit — `can_fast_fall` is the ability flag and nothing else — so a player holding down through the freeze of their own connecting aerial falls fast on the first live tick, which IS hitfall. Guarded by `an_attacker_holding_down_fast_falls_the_tick_its_hitlag_ends`, with a stick-neutral control so a body falling for some other reason cannot satisfy it. ⛔ no post-hit acceleration was added: there was nothing to unblock. |
| Prone damage / jab lock | ✔ | M | E1 | LANDED 2026-08-25. `jab_lock_speed` (320 for Smash, `0.0` = off) + `jab_lock_limit` (3): a launch AT OR BELOW that speed landing on a body already in knockdown re-pins it where it lies instead of throwing it, up to the limit, then the next hit launches whatever it is worth. ⭐ ASKED AT THE ONE LAUNCH GATEWAY beside `launch_into_tumble` — prone is model-private maneuver state and the reaction that resolved the knockback does not hold it, so any other site would be a follow-up call someone forgets. ⛔ A SPEED THRESHOLD, NOT A MOVE LIST: a jab is worth a few hundred px/s and a smash thousands, so the read separates itself without naming a move. ⛔⛔ THE BOUND IS THE MECHANIC — unbounded it is an infinite, and a happy-path test would never notice. Wire format v96. |
| Partial-body intangibility | ▢ | C | E2 | Requires hurtbox-region identity/policy. Add only when a fighter move needs “arm/head intangible”; do not special-case sprite bones in damage code. |

## 3. Damage, launch, and impact presentation

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Hard-launch smoke/speed trail | ✔ | S | — | Gate cosmetic emission on semantic tumble/launched state plus speed, not world velocity alone. |
| Strong/near-KO launch trail tier | ✔ | S | — | Presentation threshold layered on the same launch fact after the base trail reads well. |
| Strong-hit impact flash | ✔ | S | — | Hit sparks/camera shake already scale with strength; add a brief high-strength flash without changing hit resolution. |
| Finish zoom on probable KO hit | ▢ | M | E1 | Combat publishes a resolved “finish-zoom eligible/probable KO” event/fact; camera/VFX consume it. Never let camera code predict physics independently. |
| Ground-bounce / wall-splat feedback | ✔ | — | — | Both halves ship (`49ea1d7e5`, `3e39edd02`): a CRASH (`Landed { involuntary }`) kicks brighter dust plus a ring at any speed, and the WALL splat reads `Contact::impact_speed` under `ContactKind::Side`. ⭐ the wall has its OWN speed band (150–440) because gravity never accelerates a body into one — the hardest side arrival measured is 440 against a floor onset of 520, so sharing the floor's numbers would have shipped it inert. |
| Launch animation distinct from tumble | ✔ | — | E1 | `LaunchedBodyFact::launch_beat_secs` publishes the sim's own `recoil_lock_timer` (`8f1b3c47f`), so a white-hot flare rides the front of a launch over the grey plume. Confirmed on CAPTURED PIXELS by diffing one match with it off, not by reading the fact back. |
| HUD percent punch/shake | ✔ | S | — | ✔ SHIPPED 2026-08-24. `HudStanding::emphasis` is a presentation primitive (`0..=1`, default 0 so no other HUD changes) and the renderer scales the value text by it; the game derives it from `BodyCombat::hitstop_timer`, which is non-zero exactly when a hit lands and already scales with the damage. ⛔ not a percent delta tracked in presentation — that disagrees with the sim the frame a hit is blocked or absorbed. |

## 4. Ground movement and neutral

Current `BodyMotionFacts` distinguishes traversal dash from platform-fighter
`running`, but there is no platform-fighter locomotion phase for initial dash,
turnaround, or walk/run gait. Add that vocabulary once and derive the techniques
below from it.

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Walk distinct from run | ~ | S | E1 | ⚠ THIS ROW'S PREMISE WAS WRONG, measured 2026-08-24. It IS a continuum, and the gait line already cuts it: the stick magnitude scales the TARGET speed (not the acceleration toward one shared cap), so a half tilt settles at 135 px/s where a full one settles at 270, and `run_commit_frac` publishes `BodyMotionFacts::running` false at the first and true at the second — which the move selector already reads for the dash attack. Pinned by `a_light_tilt_walks_and_a_full_one_runs`, whose poison (scale accel, share the cap) reddens it. ⇒ **the real remaining gap was narrower and was INPUT, not locomotion**: a digital source can only ever say 1.0, so a keyboard fighter could not walk. ✔ CLOSED 2026-08-31 by a `Walk` action bound to `ShiftRight` on every keyboard preset, whose hold CAPS the movement magnitude at `WALK_AXIS_CAP` (0.5, below `RUN_COMMIT_FRAC`). ⛔ A CAP, NOT A SCALE: an analog stick already inside the walk band is untouched, so asking to walk while walking does not slow you. ⛔ NOT the shared `Modifier` slot, which Mary-O already reads as her RUN — capping on it would make her run key slow her down. ⚠ A D-PAD-ONLY PAD PLAYER STILL CANNOT WALK: `Move` binds the D-pad beside the stick, every gamepad button is spent, and stealing one to duplicate what the stick does would be a worse trade. ⛔ the locomotion half was NOT rebuilt. |
| Initial dash | ✔ | M | E1 | LANDED 2026-08-25. `initial_dash_time` 14 frames for Smash, `0.0` everywhere else. ⭐⭐ ONE EDGE IS THE WHOLE RULE: a steer direction that DIFFERS from last tick starts the phase — the dash, its free reversal AND the foxtrot's re-tap; a held direction never re-triggers, which lets the phase expire into a run. ⛔ THE DASH SETS THE SPEED, it does not `approach` it. ⛔⛔ TWO REAL DEFECTS CAME OUT OF TURNING IT ON: it made a body "running" on frame one and stole `smash_forward` (⇒ a dash is not a run, `running` excludes the window), and it DELETED KNOCKBACK — a launch held into became 270px/s, held away became 0 — which is why a one-stock match stopped ending. Same class as the ground roll's shed: a dash may only SPEED YOU UP. Wire format v97. |
| Foxtrot | ✔ | S/M | E1 | ✔ ALREADY TRUE, MEASURED 2026-08-25 — no production code. A re-tap of the same direction through neutral re-arms the initial dash, because the entry rule is a direction CHANGE and neutral resets `prev_steer_dir`. Driven end to end by `the_foxtrot_and_the_dash_dance_fall_out_of_the_same_edge`, poisoned by narrowing the rule to reversals only (which reddens the re-tap arm). |
| Dash dance | ✔ | M | E1 | ✔ ALREADY TRUE, MEASURED 2026-08-25 — no production code. Alternating directions re-arms the phase every couple of ticks and the body stays put: measured ≥4 re-arms in 24 ticks with under a quarter of a run-speed of drift, which is the difference between a dance and a run. Same test, same edge. |
| Turnaround / pivot phase | ✔ | M | E1 | LANDED 2026-08-25. `LocomotionTuning::turnaround_time` (3 frames for Smash, `0.0` = instant flip everywhere else): reversing out of a COMMITTED run delays the facing flip; reversing inside the initial-dash window stays free. ⭐⭐ THE PAIR IS THE MECHANIC — either half alone is just a speed. ⛔ IT DELAYS THE FACING FLIP AND NOTHING ELSE; inventing a skid would be a second opinion about ground speed. ⛔⛔ ARM ON THE REQUEST EDGE, NOT THE CONDITION: a body still running and still asking to reverse satisfies the condition every tick, so a condition-armed phase re-arms the instant it expires and the body turns FOREVER (poisoned; reddens the "never completed" arm). ⚠ 3 FRAMES IS WHAT THE PROVING GROUND TOLERATES, not a feel measurement — at 7, seat 0 stopped ever being knocked off in a 3600-tick match while seat 1 went off 57 times. Launch survives a mid-turnaround hit (measured), so that is balance, not the velocity corruption the dash had. Published as `BodyMotionFacts::turning_around`. Wire format v98. |
| Pivot grab | ✔ | S | — | LANDED 2026-08-25 and it needed NO move of its own, exactly as the row said capture would not. A move thrown while `BodyMotionFacts::turning_around` resolves its aim against the FLIPPED facing, so the existing forward grab points the other way — that is the pivot. ⭐ THE SAME RULE THE REVERSE AERIAL RUSH USES: a turnaround is finished by whatever you commit to out of it; jumping resolves it in the kernel, acting resolves its DIRECTION in `resolve_attack_gestures`. ⛔⛔ AND THE FIRST IMPLEMENTATION WAS IN THE WRONG PLACE AND READ PERFECTLY: putting the flip at the move SELECTOR compiled and changed nothing, because the direction is already decided by then. Only a WIRING test caught it — the pure `attack_dir_from_axis` passes either way. |
| Run cancel into shield | ✔ | S/M | E1 | LANDED 2026-08-25, and it was a LIVE BUG rather than a missing feature. ⛔⛔ MEASURED: a body running at 270px/s that raised its guard was still doing 270 sixty ticks later — the whole ground-speed block, FRICTION INCLUDED, sits inside `can_move_horizontal`, which a raised guard turns off. "May not steer" is not "may not stop", and a shielding fighter GLIDED across the stage. Now 270 → 143 → 17 → 0, planted in three frames. ⛔ THE FIX IS NOT "BRAKE WHEN SHIELDING": a roll is shield-held too and SETS its own velocity, and a body that was just HIT is also grounded-and-not-steering — the first version deleted knockback and reddened `a_ground_roll_ends_stopped_but_never_eats_a_launch`. Bounded by ownership: a brake may only take back speed the body could have walked up to. ⚠ shield+DIRECTION is the roll (Jon 2026-08-23), so the cancel is shield with a NEUTRAL stick. |
| Run cancel into crouch | ✔ | S/M | E1 | ✔ ALREADY SATISFIED BY THE CROUCH CAP — measured 2026-08-25, no production code needed. A body at full run (270px/s) that crouches while STILL HOLDING the direction reads 183 → 97 → 10 → 0 and is stopped on the FOURTH tick. ⭐ that is what the CAP buys: an accel-scaling version would coast at run speed and still satisfy the steady-state test, because it eventually arrives — poisoned exactly that way and it reddens. `crouching_out_of_a_run_kills_the_momentum_within_a_few_frames` now pins the TRANSITION, which the neighbouring steady-state test could not: it starts a body already crouching. |
| Crouch walk | ✔ | M | E1 | ⛔ THE ROW ABOVE WAS STALE AND IS WITHDRAWN (checked 2026-08-25): it claimed `Crouching` never reaches the locomotion law. It does — `integration.rs` scales the ground speed CAP by `crouch_speed_frac` when `ctx.crouching`, landed 2026-08-24 by the very measurement the row quotes, and `a_crouch_costs_speed_only_where_a_ruleset_asks_for_it` guards it. ⭐ AND SMASH ADOPTED IT: `crouch_speed_frac: 0.0` — "in every Smash, crouching stops you outright", so the smaller hurtbox and the shortened launch (`crouch_cancel_scale: 0.85`) are paid for with mobility. ⛔ THE CAP, NOT THE ACCELERATION: scaling accel would make a crouch slow to START and then just as fast, which is a delay rather than a stance. |
| Reverse aerial rush | ✔ | S/M | — | LANDED 2026-08-25, and it DID emerge as the row predicted — but only after one missing rule. ⛔ MEASURED FIRST, and the sequence gave the OPPOSITE of a rush: turn → jump → aerial left the fighter facing its ORIGINAL way with REVERSED momentum, because an airborne body may not turn at all, so a turnaround jumped out of was ABANDONED rather than resolved. ⇒ A TURNAROUND IS A GROUND PHASE AND LEAVING THE FLOOR FINISHES IT: the body takes into the air the facing it was already paying for. No RAR state, as the row requires. ⚠ THE MOMENTUM HALF IS NOT A PROPERTY THIS ENGINE HAS — its air stop assist halts a released stick dead, so the rush is bought by holding FORWARD after the jump (the reversed facing sticks precisely because airborne bodies cannot turn) rather than by drift. Poisoned by abandoning instead of resolving. |
| Teeter at platform edge | ✔ | S/M | E1 | LANDED 2026-08-25. `LocomotionTuning::teeter_margin` (a quarter of the footprint for Smash, `0.0` everywhere else) publishes `BodyMotionFacts::teetering`: supported where you stand, but your LEADING FOOT is over air. ⛔ A FACT, NOT A RULE — collision, speed and refusals are all untouched, exactly as the row required. ⛔⛔ SUPPORT IS ANY LATERAL OVERLAP, so a body hanging 14px past a platform with 15px of half-width is still fully supported — what matters is where the PROBE'S TRAILING EDGE sits, and a first attempt that leaned the whole body by `half_width * margin` found no edge anywhere. ⚠ a poison proved a whole-body shift by the same amount is EQUIVALENT (both put the trailing edge in one place; the far side never matters), so the documented rationale was corrected — the real poison is centre-over-air, which reddens the lip arm. Measured: 15px half-width, platform ending at 500, brink begins at x=492. |
| Walk-stop/crouch-start/crouch-end pose beats | ▢ | S | E1 | Publish the transition facts only if animation needs them; do not infer them from sprite phase. |

## 5. Air movement, recovery, and ledges

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| B-reverse | ✔ | M | E1 | LANDED 2026-08-25, and the RECOGNISER landed the same day: press forward, flick BACK inside the window the accepted special opened, and the fighter turns AND its drift reverses. ⛔ THE FLICK IS WHAT BUYS THE DRIFT — it used to reverse on every back-special, which is this technique's final state applied unconditionally, so one gesture could not choose between the three. ⭐ the window is the ruleset's own `flick_window_ticks`, already how long a flick and a press count as one intent. |
| Wavebounce | ✔ | M | E1 | LANDED 2026-08-25 as the COMPOSITION of the other two, not a mechanic of its own: back before the press AND a flick after flips the facing twice — which is no flip — and reverses the drift once. ⭐⭐ THE THREE TECHNIQUES ARE TWO TOGGLES, which is why the fourth outcome needs no recognition. ⛔ THE DRIFT, NOT THE WHOLE VELOCITY — reversing `vel` outright would flip a launch the fighter is riding — and the axis is the body's own SIDE, pinned under rotated gravity. |
| Double-jump cancel | ✔ | M | E1 | LANDED 2026-08-25. `DeclaredCombatRules::double_jump_cancel` (Smash only): an aerial thrown out of a jump spent in the air kills the rest of that jump's rise, so a double jump is an approach rather than a commitment. ⭐⭐ THE OWNERSHIP BOUND LIVES IN THE PUBLISHER: `BodyMotionFacts::air_jump_rise_owned` (an `f32`; it was the bool `air_jump_rising` when this row was written, and became a QUANTITY — how much rise a cancel may take back) means "rising on a jump I OWN" — a body going up faster than its own air jump is riding a launch and the fact is FALSE for it. That is the fifth appearance of that bound today and the first where the publisher carries it, so no consumer needs the jump tuning and none can disagree. ⚠ AND MOVING THE BOUND MOVED ITS TEST: dropping it left the core suite green, because the combat test injects the fact directly. A core arm now proves the publisher carries it. |
| Fast-fall after launch/bounce recovery | ✔ | S/M | E1 | ✔ ALREADY TRUE, MEASURED 2026-08-25 — no production code. `tick_knockdown` strips control for the tumble's duration and hands it back whole, so fast-fall is REFUSED inside the tumble and returns the moment it ends (863 → 931 px/s). Both halves are now guarded, because either alone is a different game: refused forever is a fighter who cannot come down, permitted always is a launch you can cancel. ⛔⛔ THE REFUSAL ARM MUST BE TAKEN WHILE DESCENDING — fast-fall on a RISING body does nothing anyway, so a probe at the top of the arc passes whether the tumble suppresses control or not, and the first version of this test did exactly that. |
| Once-per-airtime recovery budget | ✔ | S/M | E1 | ✔ SHIPPED 2026-08-24, and its precondition was confirmed at the SOURCE rather than by play: `MoveSpec` carries no cooldown, no cost and no per-airtime rule, and `MoveGates` knew only `grounded` — so a rising special could be pressed forever and the fighter could only die to a launch that outran it. `BodyJumpState::recovery_charges` (an integer, not a flag) + `MoveGates::recovery`, refunded by the landing-class refresh — landing, ledge, capture, respawn — ⚠ **AND, since 2026-08-26, by a FLINCHING strike** (Jon: *"once hitstun ends, you can act again, INCLUDING using your up-B again"*); a damage-only tick, an armored hit and a windbox still refund nothing. ⭐ **AND IT IS THE DEFAULT NOW, not an opt-in**: `SmashRepertoire::up_special` is an `UpSpecial` whose ordinary arm is the genre's rule, because opt-in meant one fighter in the tree had opted in. Schema 79→80. |
| Generic post-recovery helpless state | ✔ | M | E1 | ✔ SHIPPED 2026-08-24 — a fighter that spent its recovery, is still airborne, and whose move has ENDED keeps its drift and its fast fall and nothing else: no attack, no special, no jump, no air dodge. That is the whole of the edgeguard game — going offstage after somebody costs nothing until a spent recovery is final. ⭐ DERIVED, not stored (`body_is_helpless`): every term already rewinds, so there is no marker to keep true and nothing to register. ⛔⛔ AND IT CANNOT REACH A GAME THAT DOES NOT WANT IT WITHOUT A FLAG — charges only fall to zero when a move whose `MoveGates::recovery` spends one does, so a cast that authors no recovery never satisfies it, by CONSTRUCTION. Three terms and all three necessary: drop the move term and a fighter is helpless during the recovery it is still throwing. |
| Ledge-trump outward pop/commitment | ◐ | S/M | E1 | THE POP LANDED 2026-08-25 (`2b6711740`): `ledge_trump_pop` is a declared match rule (Smash 260px/s; every other world drops the loser in place, which is what every trump did before). ⛔ OUTWARD is the hang's `wall_normal_x`, read BEFORE the knock-off clears it — a reading off the body's facing is backwards for a body hanging facing out. ▢ the brief COMMITMENT half is not built: the trumped body can still act immediately. |
| Two-frame ledge vulnerability | ✔ | S/M | E1 | LANDED 2026-08-25. `LEDGE_GRAB_VULNERABLE_TIME` (2 frames) delays the earned ledge intangibility, so a body is hanging but hittable at the catch. ⭐⭐ WITHOUT IT THE EDGE IS AN UNCONDITIONAL SAFE POINT and the whole recovery is decided off-stage — nothing can contest a ledge that is safe on contact. ⛔ IT DELAYS THE WINDOW, IT DOES NOT SPEND IT: the earned invuln is HELD while the exposure runs, so a fighter who bought 0.5s with a long recovery still gets 0.5s, two frames later. ⛔ A MODULE CONSTANT, following the convention `LEDGE_HANG_LIMIT` states for the whole ledge vocabulary; only bodies with the `ledge_grab` ability reach it. ⭐ the existing intangibility test was REPAIRED, not weakened: its claim (untouchable for its OWN reason, not the dodge roll's) is unchanged and it now also pins the exposure. |
| Ledge regrab count/limit | ✔ by another key | — | E1 | RESEARCHED 2026-08-24: the genre's punishment for stalling from below is DIMINISHING intangibility, and `ledge_invuln_for(time_off_ledge)` already delivers it — 0.10s to 0.50s linear over 1.20s of pre-catch airtime, so a fast regrab gets the floor. A regrab COUNT would be a second authority over one punishment. Reopen only if play shows the airtime key failing where a count would not. |
| Edgehog vs trump rules knob | ✔ | M | E1 | LANDED 2026-08-25 as ONE COMPARISON. `DeclaredCombatRules::ledge_occupancy` (`Trump` default = today; `Hog` = Melee's): whoever sorts FIRST keeps the edge, so trumping and hogging are the same authority read in opposite directions. ⛔ NO SECOND RULE ABOUT WHO MAY GRAB — a hog that refused the grab outright would be a second ledge authority, which the row rules out; the loser is knocked off by the same path with the same `ledge_trump_pop` either way. The `SimId` tiebreak stays ascending in both, so a same-tick contest is still deterministic. ⭐ the test is the SAME FIXTURE TWICE, one declared rule apart, which is what makes this a policy rather than two mechanics. |
| Tether recovery | ▢ | M | E1 | Reuse the existing grapple/spatial-link machinery and integrate ledge/recovery eligibility only where needed. |
| Tether grab | ▢ | M | E1 | Grapple/tether acquisition feeds generic capture semantics. |
| Teleport recovery | ✔ | S/M | — | SHIPPED 2026-08-27 exactly as this row prescribed — a technique over the existing blink primitive, not a teleport subsystem. `smash.teleport` with a ledge assist, authored by TWO fighters (`player_robot_moveset` up-B and `author_moveset`) that share the technique and differ only in effect ids. The recovery half is a `RecoveryRoute::Teleport` the AI planner reads, added because a planner modelling one thrown velocity could not see a teleport (`lift_speed == 0.0`). Re-marked 2026-08-28; the ▢ was stale. |
| Stall-then-fall move | ▢ | S/M | — | Existing move windows/self-motion are sufficient unless a fighter proves a missing semantic. |

## 6. Grabs and capture depth

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Distinct grab-release beat/pose | ▢ | S/M | E1 | Capture lifecycle already has release. Publish a short released/escaped fact if presentation needs to distinguish it. |
| Pivot grab | ✔ | S | — | LANDED 2026-08-25 and it needed NO move of its own, exactly as the row said capture would not. A move thrown while `BodyMotionFacts::turning_around` resolves its aim against the FLIPPED facing, so the existing forward grab points the other way — that is the pivot. ⭐ THE SAME RULE THE REVERSE AERIAL RUSH USES: a turnaround is finished by whatever you commit to out of it; jumping resolves it in the kernel, acting resolves its DIRECTION in `resolve_attack_gestures`. ⛔⛔ AND THE FIRST IMPLEMENTATION WAS IN THE WRONG PLACE AND READ PERFECTLY: putting the flip at the move SELECTOR compiled and changed nothing, because the direction is already decided by then. Only a WIRING test caught it — the pure `attack_dir_from_axis` passes either way. |
| Grounded command grab | ▢ | S/M | E1 | A special requests generic capture acquisition/effect instead of normal knockback. |
| Aerial command grab | ▢ | M | E1 | Generalize capture eligibility/posture policy; do not duplicate `CapturedBy`. |
| Hit-grab | ▢ | M | E1 | A normal blockable hitbox whose successful victim hit transitions into generic capture. Shield interaction remains hitbox semantics. |
| Tether grab | ▢ | M | E1 | Spatial tether feeds the same capture request/eligibility path. |
| Grab-vs-grab cancellation | ~ | S | E1 | The ARBITRATION ships and is deterministic (`acquire_captures`, `829a7067b`): same-tick attempts are a sorted-greedy MATCHING keyed on captor then victim `SimId`, so no body is ever both captor and captive and the outcome does not depend on message order. What is unshipped is the genre's OUTCOME — a mutual attempt currently resolves ONE edge (lowest `SimId` wins) where Smash cancels both. That is a rules knob on top of a settled arbitration, not new arbitration. |
| Cargo carry | ▢ | C | E2 | Captor locomotion while `CapturedBy` remains authoritative. Needs one explicit movement/capture contract; do not encode it as repeated captive teleports. |
| Moving/cargo throws | ▢ | M | E1 | Extend cargo/capture state only after cargo carry exists. |
| Grab escape / pummel reaction poses | ~ | S | — | The FACTS ship: `mirror_capture_into_anim_facts` publishes `held` / `holding` onto `BodyAnimFacts` from the one relation, idempotently so a rollback re-run does not churn change ticks. What is left is ART — `CharacterAnim` has no held row, so a captive currently draws the hurt one. |

## 7. Character-mechanic primitives worth adding when a fighter needs them

These are good feature-driven engine additions. Implement each through a real
fighter rather than building an unused framework.

| Mechanic | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Rising autolink spin Up-B | ✔ | M | E1 | ✔ SHIPPED — Pointed Polygon's `polygon_rising_edge`: four autolink pulses then one launch, via the shared `multihit` combinator, and NOT a capture. Reshaped 2026-08-24 after Jon's W8 note that it read as a rising poke: a 96×48 disk centred on the body rather than a 52×60 column in front of it, the gather anchor at x=0 (⛔ `autolink_anchor_world` MIRRORS with facing, so a non-zero x made the gather side depend on which way she looked), the finisher widened to cover what the pulses held, and `sprite_spin_hz` for a crude rotational read. ⛔ the pulse GAPS are load-bearing — a contiguous track lands once. |
| Counter | ▢ | M | E1 | Authored defensive window records/consumes a qualifying contact and emits an authored retaliation. One generic contact primitive, no fighter-ID branch. ⭐ **PRICED 2026-08-28 AND MOST OF IT SHIPS**: the PARRY is already that primitive — `BodyShieldState::parrying()` is `active && parry_window_timer > 0.0`, and `body_vulnerable` consults it as one of four independent reasons a body cannot be hit, so *"a parried hit never registers, where a blocked one registers and arms a guard i-frame"*. ⇒ what a counter adds is the RETALIATION: the parry consumes the contact and nothing answers it. ⛔ the missing half is not mechanism — the windows vocabulary already carries `Invuln` and `Armor` beside `Active` — it is WHICH FIGHTER counters, which is character identity and Jon's. Build it beside the fighter that wants one. |
| Reflector move | ▢ | M | E1 | Reuse/generalize projectile ownership/trajectory transfer already demonstrated by projectile parry reflection. |
| Projectile absorber | ▢ | M | E1 | Defensive volume consumes a projectile and emits an authored resource/effect. Keep projectile identity/custody in projectile authority. |
| Armored move | ✔ | S/M | E1 | ⛔ **ZERO AUTHORED USERS, censused 2026-08-28** — one of exactly two dormant mechanics in the whole authored move vocabulary (the other is the windbox), and the same question with two names: which fighter takes a hit and keeps swinging. See `awaiting-maintainer-decision.md`. ⚠ THIS ROW WAS STALE, corrected 2026-08-25. `project_move_defense_windows` consumes the tag and is scheduled in `combat_schedule.rs:155`; it writes `BodyCombat::armored`, which `apply_body_hit_reaction` already reads. ⛔ what is missing is a MOVE that authors an `Armor` window — the mechanism is not the gap. |
| Invincible move/startup | ✔ | S | E1 | ✔ **FIVE AUTHORED USERS as of 2026-08-28** (the Actress's trapdoor plus three teleports and the Author's) — it was ONE until that day, which is why the note below still reads as though nothing uses it. ⚠ STALE, same correction and the same one writer: the tag becomes `Invulnerability::MOVE`, one more reason in the bitset `body_vulnerable` already reads. ⭐ both are written EVERY tick for every combat body, move or not, which is what makes the grant RETRACT when the window closes. ⭐⭐ **AND IT HAS MOVES NOW, 2026-08-28.** The row said *"no shipped move authors one"*; that was already off by one — the Actress's trapdoor authors i-frames for being underground — and all three TELEPORT recoveries author one now, through `TeleportParams::intangible_s` on the shared `smash.teleport` technique so the robot, the Author and the Actress cannot drift apart on it. 0.12s from the transit, clamped to the move and ending well before it does: the genre's answer to a recovery whose deciding frame is the one where the body is nowhere, shipped as a KNOB (`0.0` disables, which is what the ambush variant wants). Guarded by a straddling pair — intangible during, hittable sixty ticks later — poison-verified by authoring `0.0`. |
| Wind/vacuum special | ◐ | M | E1 | The flinchless primitive LANDED 2026-08-25 (D215, `e06333002`): `WindboxVolume` gives a volume `flinchless` + `repeating`, and suction is the same volume with its launch aimed inward — there is no second mechanic. ⛔ NO MOVE AUTHORS ONE; which move gusts is in `awaiting-maintainer-decision.md`. |
| Command-grab special | ▢ | M | E1 | Use generic capture acquisition. |
| Chargeable projectile | ✔ | S/M | E1 | SHIPPED 2026-08-27 as prescribed — no charge manager: `SmashChargeSpec` on the move itself, decided in `cancel_move_playback`, the one teardown path, from a fact the playback already carries. See *Charge storage* and *Stored projectile charge* above for the `stores` half. Re-marked 2026-08-28. |
| Stored projectile charge | ✔ | M | E1 | SHIPPED 2026-08-27 — the same `SmashChargeSpec::stores` as *Charge storage* above, seen from the projectile side: Projectile Polygon's power ball banks an interrupted charge and fires it at the stored size on the next press. Re-marked 2026-08-28; the ▢ was stale. |
| Remote mine / remote detonation | ▢ | M | E1 | Persistent projectile occurrence plus authored owner trigger; maintain stable simulation identity. |
| Returning/boomerang projectile | ✔ | M | E1 | SHIPPED 2026-08-27, and NOT as an owner-relative return target — `ProjectileGameplay::accel` is one constant world acceleration pointed back along the launch axis, which needs no reference to the thrower and so stays inside the stepper's pure signature. The leg (outbound/returning) is the sign of `vel · accel`, which is also what lets a return hit a body the outbound leg already hit. Authored customer: `polygon_ponytail_boomerang` — she throws her own tail. Re-marked 2026-08-28; the ▢ was stale. |
| Homing projectile | ▢ | M | E1 | Projectile owns deterministic steering toward a semantic target; AI/perception may choose target but does not move the projectile. |
| Pogo/bounce-on-hit attack | ✔ | — | — | Existing on-hit pogo effect; author content instead of another mechanic. |
| Self-damage/recoil move | ▢ | S | E1 | Owner-side on-hit/on-use effect through existing effect/event seam. ⚠ **AND IT WANTS A CUSTOMER, measured 2026-08-26.** The seam is one string key per volume (`MoveSpec` volumes carry `on_hit: Option<EffectRef>`), and the tree currently holds exactly ONE key — `technique::POGO_BOUNCE_KEY`. So this is a second key plus its consumer, which is small; what is absent is a fighter that wants to hurt itself. Build it beside the move that needs it, not before. |
| Heal/lifesteal on hit | ▢ | S/M | E1 | Resolved hit effect targeting owner resource/health. |
| Fighter resource meter | ▢ | M/C | E1 | Character-owned rollback state plus authored spend/gain effects. Build for one fighter; do not create a global fighter-meter manager. |
| Transformation/stance | ▢ | C | WAIT | Wait for a concrete fighter to define what changes: moveset, body tuning, art, hurtboxes, resource, or all of them. Do not prebuild a universal stance framework. |

## 8. Items

The item system is no longer absent. It already has world objects, custody,
pickup, held-use, throw, and physics. The main Smash blocker is that key pickup /
throw / held-use paths still resolve through the singular `ControlledSubject`.
A Smash match needs the same item actions for every participating body.

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| All-participant pickup/use/throw | ✔ | C | E2 | ⚠ **AND THE PRESENTATION HALF WAS STILL SINGULAR UNTIL 2026-08-28**: `HeldItemView` was an `Option` filled from `ControlledSubject`, so exactly one held item existed in the world — a couch match drew seat zero's weapon and nothing else, and a CPU-versus-CPU match drew neither Admiral's gun-sword. Plural now, over driven bodies, ordered by `SimId` because the renderer spawns one visual per row. ⛔ an autonomous holder still draws nothing; that is a wider NPC-presentation question and widening it here would have been a visual change to every enemy in the game. SHIPPED 2026-08-27 (R10) exactly as prescribed, and with no second system: `DrivenBodies` is a `SystemParam` yielding the union of `ControlledSubject` and every `DrivingParticipant`, ordered by `SimId`, and the pickup, throw and fire loops all iterate it. `ItemCustody` is untouched. ⛔ the ordering is not decoration — item pickup is a race between bodies, and Bevy query order would decide it differently per run. Re-marked 2026-08-28; the `~` was stale. |
| Deterministic match item spawning | ✔ | M | E1 | ✔ SHIPPED 2026-08-24 on D208's source. `spawn_match_items` drops on the ticks where `elapsed % every_ticks == 0` — ⛔ NO countdown resource and no "last spawned" tick, because a ticking timer here is authoritative mutable state inside the rollback window, the trap `ambition_match::prepared` documents paying for once already (it was `prepared_match` <!-- cite-ok --> until D33 cut 2b split that module, `7e625e5a5`). Identity is DERIVED (`SimId::match_spawn(activation, tick)`): the pickup road mints under the THROWER and a match-level spawner has no thrower, and `(match, tick)` settles the object completely. ⛔ tick zero never drops — the fighters are still held by the countdown. |
| Weighted item spawn table | ✔ | M | — | ✔ SHIPPED — `MatchItemSpawns { every_ticks, table, points }`, ONE struct because the three are meaningless apart. A zero weight is a row switched OFF and genuinely unreachable, which is what lets a rules screen turn an item off without deleting it. ⛔ the POINTS are the stage's: a spawn point is a fact about authored geometry, and an item system that guessed one would have an opinion about level design. |
| Items on/off and spawn-rate rules | ✔ | S/M | E1 | ✔ SHIPPED — `MatchParticipantRoster::item_spawns: Option<MatchItemSpawns>`, `None` for every match that does not ask. ⛔⛔ AND SMASH DECLARES `None` TODAY — Jon, 2026-08-24: *"we don't need items in smash right now. We eventually will, but not right now."* The machinery is built and tested; what is absent is the DECLARATION, and the table that was there is recorded beside it in `apply_smash_match_rules`. ⭐ FOUR ways to be off and `active()` is the one place that knows them: no declaration, a zero interval, no points, or every row's weight zero. A caller checking two and missing the third would drop nothing while believing items were on. |
| Directional item throws | ▢ | M | E1 | Extend throw intent with body-local direction and author launch tuning. |
| Smash throws | ▢ | S/M | E1 | Strong directional throw variant after directional throws exist. |
| Z-drop / neutral drop | ✔ | S | E1 | ✔ SHIPPED 2026-08-24 — `Grab` while holding releases the item where the body stands, at rest. ⛔ ONE enum inside `throw_held_item_system`, not a second system: a throw and a drop differ only in the launch and are the SAME custody transition, so a copy would give the custody rules a second place to drift. Guarded by a CONTRAST (the same fixture thrown goes ahead and moving), because "the item is in the world somewhere" is true of a throw too. |
| Airborne/ground item catch | ▢ | M | E1 | Deterministic item/body interception that transitions custody; do not treat it as inventory acquisition. |
| Thrown-item damage/knockback | ~ | M | E1 | Route free-flight item contact through normal combat hit attribution. |
| Stable thrower/KO attribution | ▢ | M | E1 | Preserve thrower/side causal identity on the thrown object until the interaction expires. |
| Healing food | ▢ | S | — | Content/effect on top of pickup/use. |
| Melee weapon item | ✔ | S/M | — | AUTHORED 2026-08-27: the pirate Admiral's side-B `run_out_the_guns` DRAWS the gun-sword through `MoveSpec::equips`, whose timer is the MOVE'S OWN CLOCK — so the weapon's life is the move's, and nothing else has to put it away. The Officer's moveset is the same move WITHOUT `equips`, which is the one place the two diverge and is what proves the draw is the authored half. Re-marked 2026-08-28. |
| Projectile weapon/ammo item | ~ | S/M | — | ✔ THE WEAPON HALF SHIPPED 2026-08-27: the drawn gun-sword FIRES — `MoveEventKind::Ranged` prefers the weapon a move drew — and its shot bends toward the nearest foe in the commanded half-plane via `AimAssist`, a property of the WEAPON applied where a shot's direction becomes world-space. ▢ **AMMO is the open half**: nothing counts or exhausts shots, so a drawn weapon fires as long as its move lets it. Re-scoped 2026-08-28. |
| Bomb/timed explosive | ✔ | S/M | — | SHIPPED 2026-08-27 as Projectile Polygon's down-B: a `GroundItem` with `fuse_s` AND an `impact_speed`, so it goes off *"in 4 seconds or if it hits something with enough velocity, whichever comes first"* — Jon's own words in the brief, in the module's doc. ⭐ the pick-up-and-throw half needed no new machinery: it is a ground item, so the engine's existing custody path carries it. Re-marked 2026-08-28. |
| Container/crate that yields items | ▢ | M | E1 | Existing breakable/world-item seams should emit deterministic item spawn requests. |
| Item lifetime/despawn policy | ▢ | S/M | E1 | Item occurrence owns lifetime; match rules may tune it. |
| Item whitelist/rules UI | ▢ | M | — | UI over authored item/rules data after match spawning is generic. |

## 9. Input and character-select depth

The generic input/remap layer is already strong. Smash-specific work should
translate device intent into existing body-generic attack/control semantics rather
than add controller-brand branches to combat.

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Right-stick smash/tilt attack mode (C-stick) | ✔ | S | E1 | ✔ SHIPPED 2026-08-31, both halves. The SEAM: `AttackStrengthHint` (`Auto`/`Tilt`/`Smash`) replaced the one-way `attack_strong_hint` bool from `ControlFrame` to `resolve_attack_gesture`, and an explicit hint overrules the stick in BOTH directions. The DEVICE: `ControlSettings::right_stick_mode` (`Aim` / `TiltAttack` / `SmashAttack`, default `Aim`), a hysteretic flick past `AIM_STICK_ATTACK_THRESHOLD` that presses attack once per push, and `ControlFrame::attack_from_aim_stick` so the attack points where the RIGHT stick went while the left keeps meaning walk. Surfaced as a Controls row. ⚠ in an attack mode the right stick no longer aims the blink — that is what makes it a mode. Rollback schema 142→143. |
| Smash-input sensitivity options | ~ | S | — | `AttackGestureTuning` already owns flick threshold/re-arm/window. Expose selected presets/values through Smash settings rather than inventing another gesture detector. |
| Tap-jump option | ▢ | S/M | E1 | Optional input policy converts a fresh upward movement edge into the normal Jump semantic. Movement still sees only Jump; do not teach the movement kernel about keyboards/sticks. |
| Short-hop aerial macro | ▢ | S/M | E1 | If desired for Ultimate-like controls, resolve simultaneous Jump+Attack into the ordinary jump-squat short-hop path plus buffered attack. Do not add a second short-hop physics rule. |
| Alternate costume/color per seat | ▢ | M | — | Character-select/staging metadata chooses a presentation variant; gameplay character identity/moveset remains the same. Ensure duplicate-character seats remain visually distinguishable. |
| Player tags/names | ▢ | S/M | — | Seat/profile presentation on select/HUD/results; do not store the display tag as fighter simulation identity. |
| Ready/start affordance polish | ~ | S/M | — | Current select already resolves participants and start; improve readiness feedback without adding a second roster authority. |

## 10. Stages and stage selection

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Stage-select screen | ▢ | C | — | Use the shell/experience preflight seam; stage selection is presentation/staging, not fighter simulation. |
| Stage metadata and thumbnails | ▢ | M | — | Authored stage catalog consumed by stage-select UI. |
| Random stage | ▢ | S | — | ⛔ BLOCKED ON D208, same source as the item rows in §8: "deterministic choice" is exactly the thing this engine cannot do yet outside the fighter brain's private stream. Deterministic choice from the allowed stage catalog/rules selection. |
| Multiple real Smash layouts | ~ | C | — | Mostly authored map/content work using existing platforms, one-ways, moving platforms and hazards. |
| Moving-platform stage | ▢ | S/M | — | Existing moving-platform mechanics; author and tune a stage. |
| Hazard stage | ▢ | S/M | — | Existing hazard/damage vocabulary; author stage hazards. |
| Hazards on/off rules knob | ▢ | M | E1 | Stage hazard activation reads match rules; do not duplicate hazard entities for on/off variants. |
| Battlefield-style standardized form | ▢ | M | — | Prefer authored standardized stage variants first. Generalize transformation only if several stages need it. |
| Omega/flat standardized form | ▢ | M | — | Same: authored variants are enough until repetition proves a generator is useful. |
| Per-stage blast zones/respawn anchors/camera tuning | ~ | M | E1 | Keep these stage-owned facts. Extend the stage spec only for values not already authored; do not make fighters know stage geometry. |
| Stage preview in select UI | ▢ | S | — | Thumbnail/render presentation only. |
| Training-grid stage | ▢ | S/M | — | Content useful for tuning and diagnostics. |

## 11. Match rules, modes, and ceremony

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Standable respawn platform/drop-off | ✔ | M | E1 | ✔ SHIPPED 2026-08-24, all three halves. **Placement** and the untouchable beat existed. **The release rule**: a swing spends the protection — before it, a flat 2s timer nothing could end let a returning fighter attack while untouchable, a free hit every stock. **The platform**: one stationary `MovingPlatformState` per protected fighter, present iff its seat carries `RespawnGrace`, with no clock of its own and sorted by id so the rollback-canonical Vec never depends on query order. ⛔⛔ the grant is the RULESET'S OWN (`RespawnGrace` + `Invulnerability::RESPAWN`), never a borrowed `Empowered`: that is ONE component, so granting through it overwrote whatever power-up the body carried and ending the beat removed every semantic in it. Schema 79→81. |
| Sudden death | ✔ | M | E1 | ✔ SHIPPED 2026-08-24, and the row's own warning is what shaped it: sudden death is entered by NOT SETTLING the match, so nothing mutates a finished match back into a running one — it was never finished. `decide_stocks_match` refuses to decide a level timeout when the ruleset declares `sudden_death_damage` (Smash: 150), the stage puts every SURVIVING fighter on that damage, and the fight ends it the ordinary way. ⛔⛔ THE LATCH IS LOAD-BEARING: `time_expired` stays true for every tick after, so without `SuddenDeathEntered` the tie re-enters sixty times a second and both fighters are pinned at 150% forever. Stamped with the `MatchInstance` like the verdict beside it, so the next match starts un-entered with nobody retracting anything. ⛔ only a genuine TIE — a timeout with a leader is a win, and a fighter who was ahead must not be sent to a coin flip. Schema 83→84. |
| True Time mode scoring | ▢ | M | E1 | Track KO/fall score as match scoring rather than deriving winner only from remaining stocks. |
| Stamina mode | ▢ | M | E1 | Match elimination policy on health/damage threshold; reuse body combat and match outcome infrastructure. |
| Coin mode | ▢ | M | E1 | Feasible but low priority; implement as match scoring/resource only if desired. |
| Stock count selector | ▢ | S | — | Rules UI over existing stock configuration. |
| Timer selector | ▢ | S | — | Rules UI over existing time configuration. |
| Teams selector | ▢ | M | — | Staging/UI over existing participant/team representation. |
| Friendly-fire toggle UI | ~ | S | — | `DeclaredCombatRules::friendly_fire` already exists. |
| Rules presets | ▢ | M | — | Named authored `MatchRules` bundles. |
| Handicap / starting damage | ▢ | S/M | E1 | Staging applies authored initial percent/body tuning; keep it in match preparation. |
| Rematch | ▢ | S/M | — | Re-stage the same prepared selections/rules through the normal match preparation path. |
| Random character | ✔ | — | — | `SlotPick::Random`, resolved by `roster_seeded` from one seeded stream advanced once per random seat in slot order — so two random seats draw independently and CAN draw the same fighter, which is a legal outcome of two people both asking to be surprised. |
| Random stage | ▢ | S | — | Stage-select staging choice. |
| CPU difficulty selector | ▢ | S/M | — | UI/staging chooses existing/new brain tuning profile; avoid branching simulation by controller kind. |
| Full results screen | ~ | M | — | Basic winner presentation exists; build a post-match screen from resolved outcome/stat facts. |
| Per-player KO/fall/damage stats | ▢ | M | E1 | Accumulate deterministic match stats from causal combat/stock events, then present after match. |
| Victory poses/camera/fanfare | ▢ | M | — | Presentation after outcome; fighter content supplies pose/audio identifiers. |
| Last-stock / stock-loss cues | ▢ | S | — | Presentation from stock-spend facts. |
| Final-Smash-like meter + authored super | ▢ | C | E1 | A compact fighter resource + authored super move is feasible. Do not start with a cinematic global Final Smash manager. |

## 12. Additional presentation and audio

Presentation tied directly to charge, defense, and launch is listed with its owning
mechanic above. These are the remaining standalone presentation gaps.

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Dizzy stars on shield break | ✔ | — | — | `rendering/dizzy_stars.rs` off a pooled `GuardBreaksView`; the stars orbit the body's own up. |
| Shield-break launch/fall/collapse/recover pose sequence | ~ | M | E1 | If art has distinct rows, publish one normalized break phase/state from the authoritative break occurrence. Do not make presentation reach back into `ShieldTuning` to reconstruct timing. |
| Ledge-grab spark | ▢ | S | — | One-shot from ledge acquisition. |
| Respawn-platform materialize/vanish FX | ▢ | S | — | Presentation on platform lifecycle. |
| Offscreen fighter indicators/magnifiers | ▢ | M | E1 | Camera/presentation consumes fighter visibility and projected direction; combat remains unchanged. |
| Screen KO / star KO variants | ▢ | M | E1 | Resolve KO presentation kind from exit trajectory/rules, then render it. Do not change stock authority. |
| Directional taunts | ▢ | S/M | — | Input/move selection plus authored content; ordinary taunt already ships. |
| Taunt voice cue | ▢ | S | — | Authored fighter audio on taunt event. |
| Hurt/KO/attack voice families | ▢ | M | — | Character audio content routed from existing semantic hit/KO/move events. |
| Announcer countdown/GO/winner voice | ▢ | M | — | Match ceremony already has the states; add audio presentation. |
| Victory fanfare | ▢ | S/M | — | One-shot/sting audio after outcome. |
| Controller rumble/haptics | ▢ | M | E1 | Add only through the input/backend's normal output capability; drive from resolved impact events, never from fighter-specific code. |

## 13. Training and tuning tools

These are not match mechanics, but they reduce the cost of every later Smash
feature. Prefer live runtime facts over duplicating the offline sprite author's
geometry model.

| Feature | Status | Effort | Engine | Elegant implementation |
|---|---:|---:|---:|---|
| Live hitbox/hurtbox overlay | ▢ | M | E1 | Render the actual resolved runtime volumes/ownership; the offline sprite tool's `debug-hitboxes` is not a substitute. |
| Current move/frame-data display | ▢ | S/M | — | Read `MovePlayback` + authored windows: move id, startup/active/recovery, current phase. |
| Pause simulation | ▢ | S/M | E1 | Use the time/simulation clock authority; do not pause by skipping random systems. |
| Single-frame advance | ▢ | M | E1 | Advance exactly one simulation tick through the existing frame-stepped/time seam. |
| Reset fighters to spawn | ▢ | S | — | Training command uses normal reset/placement authority. |
| Reset/set percent | ▢ | S | — | Training command mutates combat state through a narrow debug/training API. |
| Dummy stand/shield/jump/tech behaviors | ▢ | M | — | Small controller/brain profiles using ordinary `ActorControl`, not special simulation paths. |
| Dummy DI/SDI direction | ▢ | S/M | — | Controller profile emits ordinary directional intent. |
| Short input record/replay | ▢ | C | E1 | Record semantic control frames keyed to simulation ticks, replay through the ordinary controller seam. |
| Combo counter | ▢ | M | E1 | Consume resolved hit/hitstun escape facts; do not infer combos from wall-clock timing. |
| True-combo / escape-window diagnostic | ▢ | M | E1 | Ask whether the victim had a legal control/defense escape between hits. |
| Launch-vector visualization | ▢ | S/M | — | Present the actual resolved launch vector. |
| Predicted blast-zone trajectory | ▢ | M | E1 | Debug-only deterministic projection using the same movement/launch parameters; clearly separate prediction from authority. |
| Shield-health numeric debug | ▢ | S | — | Read `BodyShieldState`. |
| Hit/shield advantage readout | ▢ | M | E1 | Compute from authoritative remaining hitstun/shieldstun and attacker recovery clocks. |
| Rollback input/state reproduction capture | ~ | C | E2 | Extend existing trace/replay seams only if current reproduction tooling cannot capture a Smash defect. Avoid a second replay format. |

## 14. CPU adoption

New mechanics must work for CPU bodies because simulation is controller-agnostic.
That does not mean every mechanic needs sophisticated CPU strategy immediately.

Safe now: add the smallest semantic option/observation support required to make
new mechanics reachable by the existing fighter brain. Examples include choosing
a tap/partial/full smash charge, recognizing a recovery-use budget, selecting a
new ledge option, or authoring a reflector/command-grab option.

Useful but secondary after the corresponding mechanic exists:

- charge timing strategy;
- shield/OOS punish choices;
- parry probability by skill;
- tech direction and wall-tech jump choices;
- ledge option selection, ledgetraps, and edgeguards;
- command-grab use against shield;
- reflector/counter decisions;
- item pickup/use/throw decisions;
- recovery-resource awareness.

`WAIT`: broad rollout/planner redesign, multi-opponent strategy overhaul, or a
large expansion of fighter policy in its current dependency-floor home. Those
should follow the planned AI-policy ownership migration rather than making that
migration larger.

## 15. Engine primitives that unlock many rows

When several feature rows are assigned in parallel, coordinate around these
shared semantics. The ID is a planning reference, not a new runtime registry.

⚠ **THIS TABLE UNDER-CLAIMED, AND ALL FOURTEEN ROWS ARE NOW MEASURED
(2026-09-03).** It carried **two** ✔ marks out of fourteen. The finished count:

| verdict | count | rows |
|---|---:|---|
| ✔ shipped | 10 | `P01` `P03` `P04` `P05` `P06` `P07` `P08` `P09` `P12` `P13` |
| ◐ partial, with the split named | 3 | `P02` `P10` `P11` |
| ⬚ absent | **0** | — |
| ✔ shipped, one named fact short | 1 | `P14` |
| **total** | **14** | ⚠ counting ✔ MARKS in the rows gives 11, not 10 — `P14` carries a ✔ and is split out on its own line here because "shipped" would hide the gap it still has |

⛔ **THE ABSENT COLUMN EMPTIED ITSELF UNDER MEASUREMENT, and that is the
finding.** Not one of the twelve unmarked rows turned out to be an unbuilt
primitive. A reader treating blank as "not built" would have rebuilt foundations
that already exist — which is the specific cost this table was carrying.

⇒ **What is actually thin is the layer ABOVE the primitives.** The mechanisms
shipped; the things that spend them mostly did not. `P10` has every tech
surface and **no** published result. `P11` has the capture seam and **2 of 6**
authored roads. `P06` has the locomotion facts and **one** derived move (the
running grab) where the row names three more.

⛔ **BUT THIS PARAGRAPH ALSO OVERSTATED THE CASE, AND THE CORRECTION IS THE
LESSON.** It used to lead with *"`P01` has charge with one authored move using
it"*. That was wrong: I had counted the STORED-charge spec (authored once) and
read it as the charge mechanism's only customer, when held smashes charge on
**every fighter by default** via `charge_gesture: Smash`. ⇒ Before quoting any
"shipped but unspent" claim from this table, check that the field you counted is
the AUTHORING gate and not a rollback-state field or a rarer sibling feature —
that single confusion produced the most quotable wrong sentence on this page. ⇒ The next slice off this table is almost
never "build a primitive" — it is "author a customer for one that already
exists", which is cheaper than the row's Class suggests.

⇒ **A row with no mark means NOBODY HAS CHECKED, not that it is absent** — kept
here because it is the rule that made the count wrong, and the next table
inherits it.

| ID | Primitive | Class | Owner / purpose |
|---|---|---:|---|
| `P01` | True move charge state | E1 | ✔ **SHIPPED, AND SPENT — this row was corrected twice in one day; see the second correction, it matters.** `MoveSpec` carries `charge: Option<MoveCharge>` (`ambition_combat/src/moveset/mod.rs:369`), `MoveCharge` and `StoredMoveCharge` are real types (:386, :413), and playback consults `charge.charging()` (:602). ⛔ **I first wrote here that exactly ONE authored move uses it and the primitive was "unlocked and unspent". That was wrong, and it was wrong because I counted the wrong field.** The runtime `charge` is ROLLBACK STATE, not authoring; the authoring gate is `MoveSpec::charge_gesture`, whose `Smash` variant is *"Every smash attack, and the default when a move says nothing"* (`ambition_entity_catalog/src/lib.rs:1355`). ⇒ **Held smashes are authored on EVERY fighter by default**, and the CPU pays for them in real matches — `the_cpu_charges_a_smash_and_techs_a_landing_in_some_match` measures a best charge fraction of **0.99** within a 5400-tick window, and its own failure text reads *"the charge multiplier is authored on every fighter and nobody paid for any of it"*. ⚠ **What IS authored once is a different and rarer feature**: the STORED charge, `smash_charge: Some(SmashChargeSpec)` with `stores: true`, on the projectile polygon's neutral-B (`game/ambition_content/src/projectile_polygon_moveset.rs:174`) — Jon's samus/mewtwo parity ask. Charging a smash and BANKING a special are two mechanics, and conflating them is what produced the wrong row. |
| `P02` | Hit reaction policy | E1 | ◐ **PARTIAL — re-measured 2026-09-03, and the split is worth knowing before anyone plans this.** `HitReaction` (`ambition_platformer2d_core/src/hit_response.rs:98`) has exactly TWO variants: `Strike` and `Windbox`. So **flinchless shipped** (the windbox row above records it) and `autolink` has a road (`ambition_combat/src/hit_reaction.rs:293`), while **fixed reactions and set knockback are absent from the vocabulary entirely** — they are not a modifier away, they need enum variants. ⇒ The unshipped part is smaller than the row and differently shaped. |
| `P03` | Same-move hitbox arbitration | E1 | ✔ **SHIPPED — measured 2026-09-03, and it is the deterministic form the row wanted.** `StrikeRank` (`ambition_combat/src/moveset/mod.rs:963`) carries `window: u16` + `volume: u16` — *"which of these two did the author write first"* — and hit resolution reads the siblings through it: *"the whole of the sweetspot rule's input: which other volumes one move has live right now, and which of them the author wrote first"* (`hitbox/mod.rs:351`). Author order, not entity order, so it is rollback-stable. |
| `P04` | Move defense windows | E1 | ✔ **SHIPPED — and this page already said so two screens up.** `project_move_defense_windows` (`ambition_combat/src/moveset/mod.rs:3536`) consumes both tags and is scheduled in `ambition_platformer2d_runtime/src/combat_schedule.rs:172`; the staleness paragraph above names it as consumed. The tags are no longer inert vocabulary. |
| `P05` | On-block cancel fact | E1 | ✔ SHIPPED 2026-08-31 — the fact is `MoveContact`'s three outcomes, fed by `ResolvedBodyHit` and `BlockedBodyHit`, and `CancelCondition::OnBlock` consumes it. |
| `P06` | Ground locomotion phase | E1 | ✔ **SHIPPED — measured 2026-09-03.** `LocomotionTuning` carries `initial_dash_time`, `turnaround_time` and `teeter_margin` (`ambition_platformer2d_core/src/abilities.rs:889-897`), and the movement authority runs the clock: `state.turnaround_timer` against `tuning.locomotion.turnaround_time` (`movement/abilities.rs:265-300`). ⚠ **The derived-move half is thin but NOT empty, and the row's own list hid that.** The three moves it names — foxtrot, dash-dance, pivot grab/smash — are absent. But a locomotion-stance-derived move DOES ship: the **running grab**, selected by `move_for_flat_verb(GRAB_VERB, grounded, running)` (`ambition_combat/src/moveset/mod.rs:2854`) and derived through `dash_stance_verb` rather than authored twice. ⇒ The seam from a locomotion fact to a derived move is proven end-to-end by one customer; the missing three are authoring, not plumbing. |
| `P07` | Combat action buffer | E1 | ✔ **SHIPPED, exactly as the row specified.** `BodyActionBuffer` is threaded through move acceptance (`ambition_combat/src/moveset/mod.rs:1936, 2261, 2573`) and spent only there: `ProposedVerb::spend` (:2027) is the single road, with a variant per verb — `Unbuffered` spends nothing, attack/grab/special each clear their own. That is the row's *"spend buffered actions only through normal action acceptance"*, met rather than approximated. |
| `P08` | Attack-strength hint | E1 | ✔ SHIPPED 2026-08-31 — `AttackStrengthHint` at the seam; a right-stick mode no longer has to spoof stick history to throw a tilt. The device binding is the open half. |
| `P09` | Shield/OOS arbitration | E1 | ✔ **SHIPPED — one owner, which is what the row asked for.** `OutOfShield` (`ambition_platformer2d_core/src/movement/abilities.rs:142`) enumerates the legal actions — `Jump`, `Burst`, `Grab`, `UpAttack`, `UpSpecial` — and move acceptance takes it as `oos_policy` (`ambition_combat/src/moveset/mod.rs:2268`) beside `rises_out_of_shield` (:2111). The vocabulary and its single consumer both exist. |
| `P10` | Complete tech surface/result vocabulary | E1 | ◐ **PARTIAL — the SURFACES shipped, the RESULT VOCABULARY did not.** All three surfaces plus the wall-tech jump have arms in `movement/tests/combat_actions.rs`: floor (`a_tech_on_the_landing_skips_the_knockdown_entirely`), wall (`a_tumbling_body_can_tech_off_a_wall`), ceiling (`a_tumbling_body_can_tech_off_a_ceiling`), the jump (`a_wall_tech_asked_to_jump_leaves_the_wall_higher_than_one_that_is_not`), plus lockout and untechable-threshold arms — eight in all, and the state lives in `movement/model.rs` (`tech_press_timer`, `tech_lockout_timer`, `tumble_untechable`). ⛔ **What is missing is the second half of the row's own title**: no `TechResult`-shaped fact exists for presentation or AI to read, so the surfaces are simulated and unpublished. |
| `P11` | Capture acquisition policy | E2 | ◐ **PARTIAL, and the open half is now measured too (2026-09-03).** ✔ The arbitration shipped: `CapturedBy` (`capture/mod.rs:20`) is a real relationship with `MapEntities`, and `acquire_captures` (`capture/systems.rs:70`) is the single acquisition system. ⭐ **There is exactly ONE production writer of `CaptureAttemptRequested`** — `translate_smash_capture_effects` (`game/ambition_demo_smash/src/capture.rs:60`), which turns an authored `Special` carrying the `smash.capture_attempt` key into the typed request. So the row's *"six roads"* are not six code paths to audit; they are six AUTHORED MOVES that would all share the one key, and the audit is a count of what is authored. ⇒ **Authored today: 2 of 6.** `GRAB_VERB` (standing) and `GRAB_DASH_VERB` (running) — George carries both (`george_booul_moveset.rs:843`: `george_grab`, `george_grab_dash`), and the running form is DERIVED by `dash_stance_verb` rather than spelled twice, with a contract that authors no running grab left untouched. **Aerial is answered by fallback, not authoring** (*"an ungated grab must still answer an airborne press"*, `ambition_entity_catalog/src/tests.rs:462`). **Absent: pivot, command grab, tether, hit-grab.** ⇒ The seam is finished and cheap to extend — a new road is an authored move plus the existing key, not new plumbing. |
| `P12` | Recovery-use budget | E1 | ✔ **SHIPPED — and the row's hedge (*"only if real play needs it"*) was already spent; play needed it.** `AxisLocomotion::recovery_charges: u8` (`ambition_platformer2d_core/src/body_clusters.rs:348`), spent at `moveset/mod.rs:2331` (`saturating_sub(1)`) and refreshed to `ae::DEFAULT_RECOVERY_CHARGES` on the landing road (`hit_reaction.rs:276`). ⭐ It is **an integer, not a flag** — the field's own comment: *"The genre's default is one, and a winged fighter authoring two is an ordinary tuning statement rather than a second mechanic"*, which is precisely the 0/1/N the row asked for. ⭐⭐ It also carries a derived-vs-stored ruling worth reading before touching it: **helplessness is a stored EPISODE, not `recovery_charges == 0` derived** — because a refresh is landing-shaped, and an accepted hit that hands the dodge back must not also end the helpless episode. |
| `P13` | Participant-generic item action path | E2 | ✔ **SHIPPED — `DrivenBodies` is exactly this primitive.** (`ambition_held_items/src/lib.rs:988`, a `SystemParam` used by every wielded ability: `volley.rs:91`, `beam.rs:85`, `meteor.rs:88`.) Its own comment states the row's premise — *"EVERY DRIVEN BODY, not the one the primary seat happens to hold. `ControlledSubject` is singular by construction, so a possessed body or a second seat holding the same gauntlet simply never fired."* It answers from the seat half when no possession road is registered, so a composition without one is an ordinary answer rather than a panic. |
| `P14` | Resolved presentation facts/events | E1 | ✔ **SHIPPED for four of its five named facts, plus a fifth it did not name — measured 2026-09-03 against `ambition_sim_view`, the read-model presentation actually consults, searching by CONCEPT rather than by the row's spelling.** **charge** → `smash_charge_fraction`, `charge_tier`; **unhittable** → `unhittable`; **launch beat** → `launch_beat_secs`, and `the_front_of_a_launch_is_published_apart_from_the_tumble` names the split as a test; **shield-break phase** → `rebuild_guard_breaks_view` (`pose_view.rs:588`) reading `BodyShieldState::break_phase`. Unnamed by the row but published anyway: the **knockout beat WITH ITS PLACE** (`pose_view.rs:609`). ⚠ The one genuine gap is **finish-zoom ELIGIBILITY**: `camera_snapshot.rs` has the zoom machinery (`zoom_multiplier`, `cinematic_lock`, `zoom_in_rate`/`zoom_out_rate`) but drives it from camera ZONES, and no fact anywhere says *this blow is the finishing one*. ⇒ The mechanism is there and the gameplay fact that would aim it is not. |

> ⚠ **Instrument note for whoever extends this table — read this before trusting any ⬚ mark.** Two of the last three rows were first written WRONG, both in the same direction, both by grepping the row's own wording instead of the concept:
>
> - **P14** was first written *"shield-break and finish-zoom: absent, 0 files each"* from `shield_break` / `finish_zoom`. The code spells them `break_phase` and `zoom_multiplier` / `cinematic_lock`.
> - **P12** was first written *"ABSENT, and correctly so — nothing has asked"* from `recovery_budget` / `recoveries_used` / `air_recover`. The code spells it **`recovery_charges`**, 39 occurrences, shipped for so long it has its own derived-vs-stored ruling attached.
>
> **A row's wording is the author's vocabulary, not the codebase's**, and these rows were written years before the code that answers them. The failure is one-directional and therefore invisible: a concept found under an unexpected name still reads as a clean ⬚. Search the concept (`git grep -o -E` for a family of stems, then look at what actually matched), and confirm the hit IS the concept before marking either way. This is the fourth and fifth time this exact error has been caught on this page's audit — and the only two caught before they were committed.

None of these requires the actor-monolith carve, simulation-phase migration, or
capability/runtime composition cleanup as a prerequisite. Keep each change in
its current semantic owner. `E2` means the work deserves its own coordinated
campaign, not that unrelated architecture must land first.

## 16. Features not worth generalizing yet

These are either low return or would encourage architecture before a concrete
fighter/ruleset defines the requirement:

- a universal stance/transformation framework;
- a generic status-effect scripting VM;
- an exhaustive clone of every Ultimate hitbox flag;
- full cinematic Final Smash infrastructure before a simple resource + authored
  super proves the gameplay need;
- Spirits/equipment parity, assist-trophy/Pokéball-scale summoning, and large
  collectible/meta systems;
- broad fighter-AI planner expansion before AI-policy ownership moves;
- online matchmaking/lobby work as part of this parity inventory;
- Melee bug parity such as accidental wavedash/L-cancel behavior. If a similar
  technique is wanted as an authored rule, add that rule deliberately.

## 17. Suggested implementation order

This is an ordering by dependency and visible payoff, not a requirement to
finish one category before touching another.

1. **Combat feel:** true smash charge; charge cues; launch trail; i-frame blink;
   tech/parry feedback; filled shield bubble.
2. **Combat vocabulary:** activate the combat action buffer; consume
   invuln/armor windows; `OnBlock`; fixed/autolink reaction; sweetspot
   arbitration; Pointed Polygon autolink Up-B.
3. **Ground game and controls:** walk/run phase, initial dash, turnaround/pivot,
   foxtrot, dash-dance, pivot grab/smash, then right-stick attack mode.
4. **Defense depth:** shield-drop/OOS policy, shield shift, dodge staling,
   ceiling tech, wall-tech jump.
5. **Stage and match completeness:** respawn platform, stage select, multiple
   stage layouts, rules UI, sudden death/rematch/results.
6. **Character primitives:** counter, reflector, command grab, wind/vacuum,
   stored charge/resource mechanics as real fighters request them.
7. **Items:** participant-generic item interaction, match spawner/rules, then a
   small proof set of food/melee/ranged/bomb items.
8. **Training tools:** live hitboxes, frame data, reset/set damage, frame step,
   dummy behaviors, combo/launch diagnostics.
9. **CPU adoption:** teach the current semantic option model enough to use and
   answer the mechanics above; leave broad planner architecture for its own
   migration.
