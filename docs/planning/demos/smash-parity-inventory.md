# Smash parity — current feature inventory

This is the standing feature truth for the Smash demo: what ships, what is
partial, what is absent, and the clean implementation seam for each gap. It is
also the Smash execution order; [`queue.md`](../queue.md) row D72 points here.
Product intent is in [`super-smash-siblings.md`](super-smash-siblings.md).
Body-generic combat ownership is in
[`../engine/combat-model.md`](../engine/combat-model.md). Do not copy open rows
into another Smash plan.

Legend: `✔` shipped · `◐`/`~` partial · `▢` absent.
Effort: `S` small slice · `M` medium slice · `C` multi-slice campaign.
Engine: `—` content/demo/UI/presentation only · `E1` one small reusable engine
semantic (do it with the feature) · `E2` coordinated engine work with a clear
owner · `WAIT` do not expand the architecture for this yet.

## How to read and update a row

- `✔` means the mechanism is wired, correct and has an authored customer. It does
  not mean a player meets it. Before you mark a row shipped, make the mechanic
  appear in `match-report` or say in the row why it cannot.
- Pick the instrument for the question:

  | question | command |
  |---|---|
  | Is this mechanism wired enough to appear at all? | `cargo run -p ambition_demo_smash_app --bin smash_tool -- match-report 30 --runs 5` (engine-floor brains by default) |
  | Does a player meet it on the shipped ladder? | `smash_tool capture-probe --character <id> --ladder game/ambition_content/assets/data/fighter_brain_ladder.ron` (or `match-report --ladder <ron>`) |

  `capture-probe` measures one match; past the match length it measures an empty
  stage. One sample shows a move ran, never that it cannot run.
- Re-grep a row's concept, not its wording, before you work it. Row wording is
  the author's vocabulary, not the code's. Rows next to recently shipped work go
  stale first.
- A row with no mark means nobody checked. It does not mean absent.

## Smash-local P0: raw two-fighter combat semantics

This gate is the highest-priority Smash work (maintainer priority). While an
unblocked `S0.*` row is open, do not select another Smash feature, stage,
match-rule, roster, presentation, item, training or CPU row because it is smaller
or easier. Take the packets in order.

Only three things preempt the gate inside the Smash lane:

1. a correctness regression that prevents a gate row from being implemented or
   measured;
2. instrumentation required to prove a gate row on the production road;
3. an explicit maintainer decision.

The gate is Smash-local. It does not reorder unrelated Ambition programs. Scope
is the raw movement, contact and reaction interaction between fighters. Stage
layout, hazards, match rules, UI, content breadth and presentation polish do not
count toward it.

| Order | packet | status | open work / closure contract |
|---|---|---|---|
| `S0.1` | Clash and projectile priority | ◐ engineering done | Swings and shots arbitrate in one contest: `ambition_combat::clank::resolve_clashes`, supplied by `ambition_platformer2d_actor_monolith::clash::arbitrate_attack_clashes`, scheduled before both damage roads. Order keys are `SimId::strike_volume` and `ProjectileSeq`, never `Entity`. A beaten shot is spent, not despawned. Authored priority decides when present; damage is the proxy otherwise (Jon). **Open, Jon's:** every shipped ruleset declares `clank_damage_window: 0.0`, so clanking never runs in a shipped room. Turning it on retunes the ground game; play-test `9.0` first. |
| `S0.2` | Grab-contact arbitration | ◐ | A hit now breaks a grab in every host: `release_interrupted_captures` installs once in `CombatSchedulePlugin` under `ambition_combat::capture::GrabInterruptionApplied` (guard `the_shipped_engine_installs_the_grab_interruption_exactly_once`). Jon has not played the change. **Open:** `acquire_captures` resolves mutual same-tick grabs by greedy matching (one edge wins); they must cancel through a tuned grab-parry/rebound result. Hitbox-vs-grab outcome must be one explicit same-frame policy (ordinary hits beat ordinary grabs; exceptions are authored data, not fighter IDs). A source comment that calls the greedy result Ultimate behavior is wrong; do not keep it as design evidence. |
| `S0.3` | Hit-volume target and interaction semantics | ▢ | `HitVolume` has geometry, damage, knockback, reaction, on-hit and presentation fields only. Add victim posture eligibility (grounded/airborne/prone at minimum), guard/blockability and extra shield damage, clankability/transcendence, per-hit hitlag/SDI, a no-tech override, weight-independent/set-weight reaction, and armor bypass. Same-frame parts stay independently meaningful when target or policy differs. Defaults preserve every ordinary hit. |
| `S0.4` | Airborne capture semantics | ▢ needs a customer or ruling | No aerial grab is authored: every capture goes through `author_standing_grab`. The grounded rule is stated twice (the constructor name and two `on_ground` checks in `capture/systems.rs`). Put posture on the constructor (a sibling of `author_standing_grab`) or a wire type it builds, never as a field every `CaptureAttemptParams` literal must state. Existing standing grabs keep grounded defaults. |
| `S0.5` | Threshold armor | ◐ | Damage threshold shipped: `ArmorPolicy::{None, Super, Damage{breaks_at}}`, authored as `WindowTag::ArmorUnder{damage}`. Plain `WindowTag::Armor` still lowers to `Super`. `>=` breaks; nothing accumulates; overlapping windows take the stronger. **Open, needs a ruling:** knockback threshold. `HitKnockbackMagnitude` is `FeelScale` or `LaunchSpeed`, so a bare `f32` threshold is forbidden, and the launch velocity is computed below the armor gate. See the `ArmorPolicy` doc. |
| `S0.6` | Localized hurtbox defense | ▢ | `HurtboxVolume` holds only `shape`. Add the smallest region/defense policy for localized intangibility; hit eligibility reads it from the same resolved hurt geometry. A production-path fixture proves an intangible region and a vulnerable region on one body. |
| `S0.7` | Character-specific movement traits | ▢ | `SMASH_FIGHTER_CEILING` admits neither wall-jump nor wall-cling, and Smash sets one match-wide `crouch_speed_frac: 0.0`. Let a fighter author/retain wall-jump, wall-cling and crawl without granting them to the roster. Non-crawlers keep the stopped crouch. No fighter-ID checks. |
| `S0.8` | Special victim-reaction families | ▢ | No bury, gameplay freeze (distinct from hitlag), paralysis or crumple. Each owns its timer, escape and posture rules through shared reaction/status authority and composes with damage, launch, capture, invulnerability, rollback and KO. |
| `S0.9` | Target/effect interaction primitives | ▢ | No projectile follows a semantic target (the homing dash and steered bolt are different things). No production lifesteal/heal-on-hit consumer of `OnHitEffectMessage`. Ship both with projectile-owned deterministic steering state, the resolved-hit effect seam, production-path tests and stable rollback identity. |

**Gate acceptance:** a packet closes only when production-path tests exercise the
interaction, Smash rules or content select the behavior where required, rollback
identity and order are deterministic, and no character-ID branch exists. An
`S0.*` label on a row below means the gate overrides that section's order.

## Scope and implementation rules

The target is Smash-like platform fighting, not byte-for-byte Ultimate. When
Smash games differ on an authored rule, prefer a tuning or rules knob whose
default preserves the current game. Physics bugs do not need parity.

For every missing feature:

1. Re-grep the current consumer first. A missing animation selection is not
   evidence that the mechanic or art is missing.
2. Human and CPU fighters use the same body and combat mechanic. Do not add a
   player-only simulation path.
3. Put gameplay truth in the owning simulation domain. Presentation consumes
   resolved facts and events; shaders and particles do not infer hit
   eligibility, charge state or shield state.
4. Add a reusable semantic only when a real feature needs it. Do not build a
   generic platform-fighter scripting framework in advance.
5. Do not wait for the actor-monolith carve, simulation-phase migration or
   composition cleanup. `E1`/`E2` rows have usable owners now.
6. Broad fighter-AI architecture is the exception. Add only the smallest option
   or observation support a new mechanic needs.

## Shipped baseline

Preserve these and build new features on their seams.

| Area | Current capability | Where |
|---|---|---|
| Move authoring | Timeline windows, local hit volumes, move events, gates, self-motion, landing lag, autocancel | `ambition_entity_catalog::MoveSpec`, `MoveWindow`, `HitVolume`; `ambition_combat::moveset` |
| Multi-hit moves | Separated Active windows re-hit; contiguous Active windows keep one victim set | `ambition_combat::moveset` |
| Cancels | `Always`, `OnHit`, `OnWhiff`, `OnBlock` | `WindowTag::Cancelable`, `CancelCondition`, `MoveContact` |
| Ground attacks | Directional attacks, smashes, dash attack | `trigger_moveset_moves`, `BodyMotionFacts::running` |
| Aerial attacks | Directional aerials, landing lag, autocancel | `ambition_combat::moveset` |
| Damage | Percent, weight, scaled knockback, hitlag, hitstun, DI | `ambition_platformer2d_core::hit_response` |
| Smash rules | SDI, ASDI, crouch cancel, rage, stale-move queue, spike/meteor lock | `CombatRules`, `BodyStaleMoves` |
| Shield | Health, drain, regen, shrink, shieldstun, pushback, break, dizzy, tilt, drop lag, platform drop | `BodyShieldState`, `ShieldTuning` |
| Parry | Press- or release-timed perfect shield | `MovementTuning::parry_timing` |
| Evade | Roll, spot dodge, directional air dodge, dodge staling. Shield+direction is the grounded evade; shield in the air is the air dodge. | movement dodge state |
| Knockdown | Tumble, knockdown, floor/wall/ceiling tech, wall-tech jump, getups, jab lock, untechable threshold | `movement/knockdown.rs` |
| Jumping | Full hop, short hop, double jump, fast fall | `ambition_platformer2d_core::movement` |
| Locomotion | Walk/run continuum, initial dash, dash dance, foxtrot, turnaround, teeter | `LocomotionTuning`, `BodyMotionFacts` |
| Body contact | Jostle and pushback | movement sweep |
| Footstool | Grounded/airborne reactions, phantom footstool | `combat/src/footstool.rs` |
| Ledge | Grab, two-frame vulnerability, diminishing intangibility, regrab limit, getups, one holder per edge, trump pop and lockout, drop. See §5 "Ledge occupancy" | `ledge_grab`, `ledge_trump` |
| Capture | Grab relation, shield bypass, pummel, four throws, mash escape, hit interrupts hold | `ambition_combat::capture`, `entity_catalog/src/smash_capture.rs` |
| Dash grab | Derived from each fighter's standing grab | `SmashCaptureRepertoire`, `grab_dash` |
| Match | Stocks, blast zones, elimination, timer, tiebreak, sudden death, finish zoom on the verdict | `ambition_combat::stocks`, `ambition_combat::finish_zoom` |
| Teams | Friendly-fire policy | `CombatRules::friendly_fire` |
| Respawn | Placement, standable platform, untouchable grace a swing spends | `RespawnGrace`, `Invulnerability::RESPAWN` |
| Items | Identity, custody, pickup, use, throw, drop, physics for every driven body | `DrivenBodies`, `ItemCustody`, `GroundItem`, `HeldItem` |
| Presentation | Pose routing, shield bubble, hit sparks, KO burst, camera shake, launch trail, invulnerability blink | render/VFX/movement FX |
| Match ceremony | 3–2–1–GO and winner presentation. The countdown runs at 10x (`COUNTDOWN_SPEEDUP = 10`). It is a developer/debug affordance that stays (Q91 ruling, 2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)); it is not product gameplay, and it is not deleted for being outside normal game flow. | Smash match presentation |
| Character select | Per-pad cursor, role cycle, auto-claim, random fighter, stage cycle, stock cycle (1/3/5), any seat may press | `game/ambition_demo_smash/src/select*` |
| Frontend exit | Pause menu Quit to Title from character select | `ambition_game_shell::pause_menu` |
| Input | Remaps, controller profiles, `AttackStrengthHint`, right-stick tilt/smash mode, keyboard `Walk` | `ambition_input` |

## 1. Attack and move semantics

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| True hold/release smash charge | ✔ | M | E1 | `MoveSpec::charge_gesture` (`Smash` is the default). No CPU throws a smash on the shipped ladder, so no CPU charges one (see *Shipped but unreached*). |
| Charge cues and pose | ✔ | S | — | Presentation reads resolved charge state. |
| Charge storage | ✔ | M | E1 | `MoveChargeSpec::stores`, opt-in; decided in `cancel_move_playback`. Customer: Projectile Polygon's power ball. |
| Jab 1 → 2 → 3 chains | ✔ | — | E1 | Held Attack continues only to a successor the window names. |
| Rapid jab + finisher | ✔ | — | E1 | `MoveSpec::repeat: Option<MoveLoop>`. |
| Combat action buffer | ✔ | M | E1 | `BodyActionBuffer` (attack, grab, pogo, special); spent only when action acceptance accepts. |
| Move invulnerability windows | ✔ | S | E1 | `WindowTag::Invuln` → `Invulnerability::MOVE`; teleports author it via `TeleportParams::intangible_s`. |
| Move super-armor windows | ✔ | M | E1 | `ArmorPolicy::Super`. |
| Damage-threshold armor | ✔ | M | E1 | `S0.5`: `WindowTag::ArmorUnder`. |
| Knockback-threshold armor | ▢ | M | E1 | `S0.5`: needs a ruling on the quantity. |
| On-block cancel windows | ✔ | S/M | E1 | `CancelCondition::OnBlock` from `BlockedBodyHit`. Customer: George's jab → grab. |
| Sweetspot/sourspot | ✔ | — | E1 | `StrikeRank` author order; one pulse is one hit. |
| Same-frame hitbox parts | ▢ | M | E1 | `S0.3`. |
| Victim-state hitbox eligibility | ▢ | M | E1 | `S0.3`: per-volume posture filter in hit eligibility, not move selection. |
| Fixed/set knockback | ✔ | S/M | E1 | `knockback_growth: Some(0.0)`. |
| Autolink/follow-owner knockback | ✔ | M | E1 | `HitKnockback::follow`, resolved at the producer in the attacker's frame. Pulse gaps are load-bearing: a contiguous track lands once. |
| Weight-independent knockback | ▢ | S/M | E1 | `S0.3`. |
| Armor-bypass hit property | ▢ | S/M | E1 | `S0.3`. Resolve against armor policy, never by move or fighter id. |
| Windboxes / flinchless push | ◐ | M | E1 | `WindboxVolume` (`flinchless`, `repeating`) ships; no move authors one. Which fighter gusts is a design call. |
| Vacuum / suction | ✔ | M | E1 | A windbox with its launch aimed inward. |
| Extra shield damage | ▢ | S/M | E1 | `S0.3`. Resolve in shield contact, not by inflating damage. |
| Unblockable strike flag | ▢ | S/M | E1 | `S0.3`. Planner metadata (`ignores_guard`) is not gameplay authority. |
| Hitbox clanking | ◐ | C | E2 | `S0.1`: finished; stage declares it off. |
| Projectile clash / priority | ✔ | C | E2 | `S0.1`: same contest as melee. |
| Attack rebound after clang | ◐ | S/M | E1 | `rebound_from_clanks`; fires only when clanking is on. A shot has no body, so swing-vs-shot rebounds nobody. |
| Cannot-clank/transcendent hit | ▢ | S | E1 | `S0.3`. Wants a melee customer; projectiles already pass through melee arbitration by architecture. |
| Per-hit hitlag multiplier | ▢ | S/M | E1 | `S0.3`. |
| Per-hit hitstun multiplier | ▢ | S/M | E1 | Add only when a move needs reaction distinct from knockback. |
| Per-hit SDI multiplier | ▢ | S/M | E1 | `S0.3`. |
| Cannot-tech hit property | ◐ | S/M | E1 | Derived untechable launches ship (`untechable_launch_speed`). `S0.3` owes an authored override for a move untechable at any speed. |
| Edge-cancel move recovery | ✔ | M | E1 | `CombatRules::edge_cancel_recovery`; a rule, not a per-move field. |
| Pivot smash | ✔ | S/M | E1 | Falls out of the turnaround in `resolve_attack_gestures`. |

## 2. Defense, shield, evade, and tech

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Filled shrinking bubble shield | ✔ | S | — | |
| Shield-hit ripple | ✔ | S | — | |
| Low-shield danger treatment | ✔ | S | — | |
| Shield-drop lag | ✔ | — | E1 | `ShieldTuning::drop_lag`. |
| Out-of-shield action policy | ✔ | — | E1 | `ShieldTuning::out_of_shield`, one gate `OutOfShieldGate`. |
| Shield grab | ✔ | — | E1 | Attack on a raised guard is the grab (Jon). |
| Jump / Up-B / up-smash out of shield | ✔ | — | E1 | Same gate. |
| Shield shift/tilt | ✔ | M | E1 | `ShieldTuning::tilt_range`; the shift costs coverage on the other side. |
| Shield drop through one-way platform | ✔ | S/M | E1 | `ShieldTuning::platform_drop`; explicit declaration, not a fallthrough. |
| Dodge staling | ✔ | M | E1 | Wears the i-frames only. |
| Spot-dodge attack cancel near tail | ✔ | S/M | E1 | `evade_cancel_tail`, measured from the end. |
| Invulnerability blink | ✔ | S | E1 | |
| Tech flash/SFX | ✔ | S | — | `MovementOp::Tech` drives `PLAYER_TECH` and a burst. All three surfaces emit one variant (a design choice, not a defect). |
| Parry flash/chime | ✔ | S | — | |
| Ceiling tech / wall-tech jump | ✔ | — | E1 | |
| Untechable high-launch threshold | ✔ | S/M | E1 | |
| ASDI | ✔ | M | E1 | `TraversalAbilityTuning::asdi_step`, paid once per hit at the end of hitlag. |
| Hitfall | ✔ | M | E1 | Nothing gates fast fall mid-move. |
| Prone damage / jab lock | ✔ | M | E1 | `jab_lock_speed`, `jab_lock_limit`; the limit is the mechanic. |
| Partial-body intangibility | ▢ | C | E2 | `S0.6`. Do not special-case sprite bones in damage code. |

## 3. Damage, launch, and impact presentation

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Bury / freeze / paralysis / crumple | ▢ | M/C | E1 | `S0.8`. Bury must not be faked as hitstun or collision embedding. |
| Hard-launch trail and near-KO tier | ✔ | S | — | |
| Strong-hit impact flash | ✔ | S | — | |
| Finish zoom | ✔ | M | E1 | `FinishZoomRequest` in `camera_ease`, keyed to the match verdict, applied to the presented scale (not `zoom_multiplier`), quarantined on speculative frames. |
| Ground-bounce / wall-splat feedback | ✔ | — | — | |
| Launch beat distinct from tumble | ✔ | — | E1 | `LaunchedBodyFact::launch_beat_secs`. |
| HUD percent punch | ✔ | S | — | `HudStanding::emphasis`. |

## 4. Ground movement and neutral

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Walk distinct from run | ◐ | S | E1 | Analog continuum plus a keyboard `Walk` cap. A D-pad-only pad player still cannot walk. |
| Initial dash, foxtrot, dash dance | ✔ | M | E1 | One direction-change edge. A dash may only speed a body up. |
| Turnaround / pivot | ✔ | M | E1 | Arm on the request edge, not the condition. |
| Pivot grab | ✔ | S | — | Aim resolves against the flipped facing. |
| Run cancel into shield / crouch | ✔ | S/M | E1 | A brake may only take back speed the body could have walked up to. |
| Character-specific crawl | ◐ | M | E1 | `S0.7`. |
| Reverse aerial rush | ✔ | S/M | — | Leaving the floor finishes a turnaround. |
| Teeter | ✔ | S/M | E1 | `BodyMotionFacts::teetering`, a fact, not a rule. |
| Walk-stop / crouch transition beats | ▢ | S | E1 | Publish only if animation needs them. |

## 5. Air movement, recovery, and ledges

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Character-specific wall jump / cling | ▢ | M | E1 | `S0.7`. |
| B-reverse / wavebounce | ✔ | M | E1 | Two toggles; reverse the drift, not the whole velocity. |
| Double-jump cancel | ✔ | M | E1 | `BodyMotionFacts::air_jump_rise_owned`. |
| Fast fall after launch | ✔ | S/M | E1 | |
| Once-per-airtime recovery | ✔ | S/M | E1 | `recovery_charges` (integer); refunded on landing, ledge, capture, respawn and a flinching hit (Jon). |
| Post-recovery helpless | ✔ | M | E1 | Derived (`body_is_helpless`). |
| Ledge-trump pop | ◐ | S/M | E1 | Pop ships (`ledge_trump_pop`); the commitment window does not. |
| Two-frame ledge vulnerability | ✔ | S/M | E1 | |
| Ledge regrab limit | ✔ | — | E1 | Ultimate's rule (2026-10-08): six grabs per airtime, intangibility ×0.8, ×0.5, then none; landing or a hit resets the count. One grab site scales the airtime-earned window by the count, so the two inputs feed one authority. Superseded the 2026-08-24 "do not add a count" answer, under the Ultimate target in "Ledge occupancy" below. |
| Edgehog vs trump knob | ✔ | M | E1 | `CombatRules::ledge_occupancy`. |
| Ledge occupancy (one holder per ledge) | ✔ | M | E1 | One corner is one edge whatever the bodies' sizes; a body out of play holds no edge; a rewind across a trump gives the same holder; the trumped body's lockout is a declared rule (2026-10-08). See "Ledge occupancy" below. |
| Tether recovery | ▢ | M | E1 | Reuse grapple/spatial-link machinery. |
| Teleport recovery | ✔ | S/M | — | `smash.teleport`, `RecoveryRoute::Teleport`. |
| Stall-then-fall move | ▢ | S/M | — | Existing windows suffice unless a fighter proves otherwise. |

### Ledge occupancy

**Built 2026-10-08: one corner is one edge.** `resolve_ledge_trumps`
(`ambition_combat/src/ledge_trump.rs`) keys an edge by the ledge, not by the
hanging body's centre: `LedgeContact::edge_key` is the midpoint of the anchor
and the climb target, which the probe places on opposite sides of the corner
by the body's half size, so the size cancels; the face (`wall_normal_x`) is
part of the key. Occupancy is derived each tick from the rollback-registered
hang (`actor.motion_model`), so release, a death and a knockoff free the edge
with no stored occupant to forget. Witness:
`two_fighters_of_different_sizes_on_one_corner_are_one_edge` (contacts from
the kernel's probe for a 28 by 46 and a 44 by 76 body; control: one on each
face keeps both). Kept: a newcomer may grab while the holder is mid-getup
(`a_body_mid_getup_is_neither_trumper_nor_trumped`), and a grab is never
refused (the trump knocks off after both latched, by design).

**Built 2026-10-08: a rewind across a trump gives the same holder.**
`a_rewind_across_a_ledge_trump_gives_the_same_holder` (app_it) drops two
player bodies past one floating lip in the calibration lab: seat 1's body
takes the edge on tick 2 and Alice trumps it on tick 14. A GGRS sync-test
session (rewinding 4 frames each tick) gives the same holders on all 60
ticks as a world with no rollback session, and stays healthy. Poison: the
trump kept a one-shot memo in a `Local` (state the rollback does not
restore); the resimulated tick 15 had both bodies hanging and GGRS reported
a checksum mismatch at frame 14. A second poison, the trump moved to
`Update`, failed the fixed-tick precondition instead (both bodies hung for
one tick), so it says nothing about the rewind.

**Built 2026-10-08: the regrab limit.** `AxisManeuverState::ledge_grabs`
counts the grabs a body made since it landed or was hit (rollback state,
schema 328). Each grab's earned intangibility is scaled by
`ledge_regrab_invuln_scale` (1.0, 0.8, 0.5, then 0), and a grab past
`LEDGE_GRABS_PER_AIRTIME` (6) is refused. Landing resets the count, and so
does a launch that is not flinchless (the hit that charges hitstun), at the
kernel's launch gateway; a windbox push does not. The airtime clock still
buys the window: the two rules compose, as in Ultimate. Witnesses:
`each_regrab_before_landing_earns_less_and_the_seventh_is_refused` and
`a_hit_and_a_landing_give_the_ledge_grabs_back_and_a_push_does_not`.

**Built 2026-10-08: the trumped body's lockout, a declared rule.**
`CombatRules::ledge_trump_lockout` (seconds; `None` and `0.0` leave the
loser in control, as every trump did) sets
`BodyCombat::ledge_trump_lock_timer` on the body the trump knocks off. It
is a sixth cause of `BodyCombat::hard_lock_timer`, so the input gate strips
every verb while it runs (schema 329). The Smash rules declare 0.5 s, a
tuning value from Ultimate's unverified "about 30 frames". Witness:
`a_declared_lockout_holds_the_trumped_body_and_not_the_one_that_trumped`
(controls: the winner gets no lock; no declared lockout gives none).
Not built: Ultimate's "the trumping body cannot let go for about 20 frames"
(from the same unverified source).

**Built 2026-10-08: a body out of play holds no edge.** A hazard can kill a
hanging body (Q43), and no death road ends the hang: the body keeps it
through its death beat until its respawn starts it again
(`reset_body_clusters`). `resolve_ledge_trumps` reads every hang, so under
Hog a dead camper knocked a live newcomer off (measured before the fix:
the newcomer lost the edge). The resolver now skips a body with `OutOfPlay`.
Witness: `a_body_out_of_play_holds_no_edge` (control: the camper alive keeps
the edge under Hog).

**Target.** Super Smash Bros. Ultimate-like ledge occupancy and trump. Do not
guess Ultimate's timings from memory: research them (getup/roll/jump options,
trump, the trumped body's state, invincibility on regrab, what a hit, a death
or a respawn does to the hold) and write the result here before you build.

**Ultimate's rules, researched 2026-10-08.** Sources: SmashWiki
[Edge](https://www.ssbwiki.com/Edge) and [Ledge](https://www.ssbwiki.com/Ledge).

| Rule | Ultimate | Source |
|---|---|---|
| Occupancy | One holder per edge. The only exception is the two Ice Climbers of one player. | Edge |
| Trump | A body that grabs an occupied edge takes it; the holder is "gently removed" and falls with no intangibility. | Edge, Ledge |
| Trumped body's lockout | About 30 frames before it can act, and the trumping body cannot let go for about 20. **Unverified**: a forum measurement seen only through a search summary (the thread returned 403). Treat as a tuning value, not a fact. | — |
| Grab intangibility | `60 * (a / 300) + (44 - p / 120 * 44)` frames, `a` = airtime (max 300), `p` = damage percent (max 120); minimum 23, maximum 123. | Edge, Ledge |
| Regrab | At most 6 grabs before landing; hitstun resets the count. Intangibility ×0.8 after the first regrab, ×0.5 after the second, none from the third. | Edge, Ledge |
| Hang time | 6.5 s under 100%. | Edge, Ledge |
| Options | Climb, getup attack, roll, jump, drop (and the drop-down attack). | Edge |

Not found in the sources: whether a body may take an edge while its holder
is mid-getup, and what a hit or a KO does to the hold beyond freeing it. The
engine keeps its tested rule for the first
(`a_body_mid_getup_is_neither_trumper_nor_trumped`: a body mid-getup has left
the edge, so it is not trumped and does not block a newcomer), and a hit that
ends the hang, a death and a knockoff free the edge.

**Model to define.** A deterministic occupancy relation keyed by the ledge
itself (its authored corner in its live room, not a body-size-dependent
anchor): at most one holder per ledge. It covers contention between two actors
on one tick (an authored rule, named: Ultimate's trump, the `Hog` knob),
release, getup, death, knockoff and transfer of the hold. It is derived from or
stored in rollback state so a rewind resimulates it exactly; the existing
custody state is rollback-registered (`actor.motion_model`, `actor.ledge`).

**Acceptance.** Focused multiplayer regressions with two fighters of different
sizes on one corner: one holder; the trump rule applied; release, death and
knockoff free the ledge; a rewind across a trump gives the same holder. The
current tests in `ledge_trump/tests.rs` keep passing or are rewritten around
ledge identity.

## 6. Grabs and capture

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Grab-release beat/pose | ▢ | S/M | E1 | Publish a released/escaped fact if presentation needs it. |
| Grounded command grab | ✔ | S/M | E1 | `lunge_grab` (stand-in `special_forward`). Zero starts in one complete shipped-ladder match. |
| Aerial command grab | ▢ | M | E1 | `S0.4`. |
| Hit-grab | ▢ | M | E1 | A second consumer of `OnHitEffectMessage` (beside `apply_pogo_bounce`) plus an optional target on the capture attempt, because acquisition searches a box and can catch a different body. A connected hit is shield-blocked, unlike the overlap grab; choose that out loud. |
| Tether grab | ▢ | M | E1 | Capture half is authoring (a long reach box). The line is a fourth customer of the `flyline.rs` visual shape. Raise the reach ceiling (`no_authored_grab_reaches_further_than_the_stage_allows`, 96px) in the same commit. |
| Grab-vs-grab cancellation | ◐ | S/M | E1 | `S0.2`. |
| Hitbox-vs-grab arbitration | ▢ | M | E1 | `S0.2`. |
| Cargo carry / moving throws | ▢ | C | E2 | Captor locomotion while `CapturedBy` stays authoritative; not repeated teleports. |
| Grab escape / pummel poses | ◐ | S | — | `held`/`holding` facts ship; `CharacterAnim` has no held row (art). |

## 7. Character-mechanic primitives

Implement each through a real fighter, not as an unused framework.

| Mechanic | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Rising autolink Up-B | ✔ | M | E1 | `polygon_rising_edge`. |
| Counter | ▢ | M | E1 | Parry already consumes the contact; the counter adds an authored retaliation. Which fighter counters is Jon's. |
| Reflector | ▢ | M | E1 | Generalize the parry-reflection ownership transfer. |
| Projectile absorber | ▢ | M | E1 | Keep projectile custody in projectile authority. |
| Armored move | ✔ | S/M | E1 | Three shipped armored moves. |
| Invincible move/startup | ✔ | S | E1 | |
| Wind/vacuum special | ◐ | M | E1 | Mechanism ships; no move authors it. |
| Chargeable / stored-charge projectile | ✔ | M | E1 | |
| Remote mine | ▢ | M | E1 | Persistent occurrence plus owner trigger, stable identity. |
| Returning/boomerang projectile | ✔ | M | E1 | `ProjectileGameplay::accel`; `polygon_ponytail_boomerang`. |
| Homing projectile | ▢ | M | E1 | `S0.9`. |
| Pogo-on-hit attack | ✔ | — | — | |
| Self-damage/recoil move | ▢ | S | E1 | A second on-hit key; wants a customer. |
| Heal/lifesteal on hit | ▢ | S/M | E1 | `S0.9`. |
| Fighter resource meter | ▢ | M/C | E1 | Build for one fighter; no global meter manager. The Smash Limit meter (`ambition_demo_smash/src/limit.rs`, fill policy `LimitMeterFill::JONS_BASELINE`) awards `on_block: 1.0` for a successful block. Q71 (2026-10-04): 1.0 is the starting balance value; playtesting may tune it without reopening the policy. `guarding_is_the_safe_option` keeps it below `on_damage_taken`. |
| Transformation/stance | ▢ | C | WAIT | George's TRUE/FALSE state is the concrete fighter (see "George Booul TRUE/FALSE vocabulary (Q81)"). Design it from his concept before you build. |

## 8. Items

Smash declares no item spawns (Jon: *"we don't need items in smash right now. We
eventually will, but not right now."*). The machinery is built and tested.

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| All-participant pickup/use/throw | ✔ | C | E2 | `DrivenBodies`, ordered by `SimId`. An autonomous holder still draws no held item. |
| Deterministic match item spawning | ✔ | M | E1 | Tick-derived, no countdown resource; identity `SimId::match_spawn`. |
| Weighted spawn table | ✔ | M | — | `MatchItemSpawns`; points are the stage's. |
| Items on/off and rate | ✔ | S/M | E1 | `item_spawns: Option<..>`; `active()` knows all four ways to be off. |
| Directional / smash item throws | ▢ | M | E1 | Body-local throw direction. |
| Z-drop | ✔ | S | E1 | Same custody transition as a throw. |
| Item catch | ▢ | M | E1 | Deterministic interception; not inventory acquisition. |
| Thrown-item damage/knockback | ◐ | M | E1 | Route free-flight contact through normal hit attribution. |
| Thrower/KO attribution | ▢ | M | E1 | |
| Healing food | ▢ | S | — | |
| Melee weapon item | ✔ | S/M | — | Admiral's side-B draws the gun-sword via `MoveSpec::equips`. |
| Projectile weapon / ammo | ◐ | S/M | — | The weapon fires (`AimAssist`); nothing counts ammo. |
| Bomb | ✔ | S/M | — | Projectile Polygon's down-B (`fuse_s`, `impact_speed`). |
| Container that yields items | ▢ | M | E1 | |
| Item lifetime policy | ▢ | S/M | E1 | |
| Item rules UI | ▢ | M | — | |

## 9. Input and character select

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Right-stick tilt/smash mode | ✔ | S | E1 | `ControlSettings::right_stick_mode`. |
| Smash-input sensitivity options | ◐ | S | — | Expose `AttackGestureTuning` presets. |
| Tap-jump option | ▢ | S/M | E1 | Input policy emits the normal Jump semantic. |
| Short-hop aerial macro | ▢ | S/M | E1 | Use the ordinary short-hop path plus a buffered attack. |
| Alternate costume per seat | ▢ | M | — | Presentation variant; identity unchanged. |
| Player tags | ▢ | S/M | — | Not simulation identity. |
| Ready/start polish | ◐ | S/M | — | |

## 10. Stages

Three stages share one blast envelope: `smash_stage()` (flat),
`smash_platform_stage()` (three drop-through tiers at 64px and 120px, recomputed
from engine jump constants by the guard), `smash_narrow_stage()` (two thirds the
ground). Recorded spacing and recovery numbers were taken on `smash_stage`. Add a
stage beside the others rather than editing one, and keep the shared envelope
(`the_stage_choice_decides_which_stage_the_match_prepares`).

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Stage selection | ◐ | C | — | A stage cycle on the character-select screen (`SmashStageChoice`). A separate screen is a design question. |
| Stage metadata and thumbnails | ▢ | M | — | |
| Random stage | ▢ | S | — | Deterministic choice from the allowed catalog. |
| Moving-platform / hazard stage | ▢ | S/M | — | Existing mechanics; author a stage. |
| Hazards on/off knob | ▢ | M | E1 | Hazards read match rules. |
| Standardized stage forms | ▢ | M | — | Authored variants first. |
| Per-stage blast/respawn/camera tuning | ◐ | M | E1 | Stage-owned facts. Q87 (2026-10-04): on `smash_platform_stage` the respawn platforms (y 164–176) sit 4 px above the top tier (y 180–196), so a fighter whose respawn platform expires lands on the tier. Fix the authored layout (move the tier or the respawn point, whichever looks better) until the spawn is clearly valid; do not add runtime spawn avoidance. Re-take the flat-versus-platforms measurement in `fighter-brain.md` in the same change. |
| Training-grid stage | ▢ | S/M | — | |

## 11. Match rules, modes, and ceremony

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Respawn platform | ✔ | M | E1 | The ruleset's own grant (`RespawnGrace`), never a borrowed `Empowered`. |
| Sudden death | ✔ | M | E1 | A tie is not settled; `SuddenDeathEntered` latch is load-bearing. |
| Stock count selector | ✔ | S | — | `SelectTarget::Stocks`; guard `the_stocks_button_sets_the_count_the_published_match_is_played_at`. |
| Random character | ✔ | — | — | `SlotPick::Random`. |
| Rematch | ✔ | S/M | — | `coming_back_to_the_select_screen_offers_a_fresh_match`. |
| True Time / stamina / coin modes | ▢ | M | E1 | Match scoring or elimination policy. |
| Timer selector | ▢ | S | — | |
| Teams selector | ▢ | M | — | |
| Friendly-fire toggle UI | ◐ | S | — | `CombatRules::friendly_fire` exists. |
| Rules presets | ▢ | M | — | |
| Handicap / starting damage | ▢ | S/M | E1 | In match preparation. |
| CPU difficulty selector | ▢ | S/M | — | The brain owns the knobs, Smash owns the ladder (Q88); queue row CPU-LADDER. |
| Full results screen and stats | ◐ | M | E1 | Basic winner card; stats from causal combat/stock events. |
| Victory poses, fanfare, stock cues | ▢ | S/M | — | |
| Meter + authored super | ▢ | C | E1 | No cinematic Final Smash manager first. |

## 12. Presentation and audio

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Dizzy stars | ✔ | — | — | |
| Shield-break pose sequence | ◐ | M | E1 | Publish one break phase from the break occurrence. |
| Ledge-grab spark, respawn-platform FX | ▢ | S | — | |
| Offscreen indicators | ▢ | M | E1 | |
| Screen/star KO variants | ▢ | M | E1 | Resolve the kind from trajectory; stock authority unchanged. |
| Directional taunts, voice families, announcer | ▢ | S/M | — | |
| Rumble/haptics | ▢ | M | E1 | Through the input backend, from resolved impacts. |

## 13. Training and tuning tools

| Feature | Status | Effort | Engine | Notes |
|---|---:|---:|---:|---|
| Live hitbox/hurtbox overlay | ✔ | M | E1 | The `Combat` debug preset; `dev_tools::force_combat_overlay` with `CombatOverlayLayers`. Owner: [`../moveset-inspector.md`](../moveset-inspector.md). |
| Move/frame-data display | ▢ | S/M | — | Read `MovePlayback` and authored windows. |
| Pause / single-frame advance | ▢ | M | E1 | Through the simulation clock authority. |
| Reset fighters / set percent | ▢ | S | — | Narrow training API. |
| Dummy behaviors and DI | ▢ | M | — | Controller profiles over `ActorControl`. |
| Input record/replay | ▢ | C | E1 | Semantic control frames keyed to ticks. |
| Combo counter / true-combo diagnostic | ▢ | M | E1 | From resolved hit and escape facts. |
| Launch vector / blast-zone prediction | ▢ | S/M | E1 | Debug-only; separate prediction from authority. |
| Shield-health and frame-advantage readouts | ▢ | S/M | E1 | |
| Rollback reproduction capture | ◐ | C | E2 | Extend existing trace/replay seams only. |

## 14. CPU adoption

Simulation is controller-agnostic, so new mechanics work for CPU bodies. Add only
the smallest option or observation support that makes a mechanic reachable.
Strategy (charge timing, OOS punishes, parry by skill, tech direction, ledge
options, command grab against shield, reflector/counter decisions, item use,
recovery awareness) comes after the mechanic exists. `WAIT`: broad planner
redesign and multi-opponent strategy, until AI-policy ownership moves. Ladder
calibration is owned by [`../engine/fighter-brain.md`](../engine/fighter-brain.md).

### Shipped but unreached

The status column answers "does the engine have it?". It cannot say "nothing
reaches it in play". Known cases:

- **Smash attacks and their charge.** A 120-second `capture-probe` census of
  George on the shipped ladder recorded zero smash starts, so no CPU charges a
  smash in play. The cause is not established; the current move-choice
  investigation and its instrument are in
  [`../engine/fighter-brain.md`](../engine/fighter-brain.md). Re-run the census
  before you rely on this row.
- **Shield, parry, grab.** The brain offers shield only in disadvantage against
  an attacking foe, and prices a grab as a guard-beater (`GRAB_BEATS_GUARD`) plus
  a small percent term (`THROW_CONVERSION`). The triangle settles at "everybody
  attacks". `THROW_CONVERSION` is low on purpose (its doc says so). Changing it
  overrules a documented tuning decision; that is Jon's call. Do not "fix" shield,
  parry or capture mechanics on this evidence.
- **Back/up/down throws.** Every moveset authors all four throws. The CPU throws
  forward only; `capture_context_frame` declares that policy.
- **Capture perception.** `captured`, `holding_captive` and related fields are on
  `SelfView` only, and `BodyPhase` has no capture variant. At three or more seats
  (`MAX_SMASH_SEATS = 4`) a fighter cannot see a grab between two others.

### George Booul TRUE/FALSE vocabulary (Q81)

Ruling Q81 (2026-10-04): unreferenced art is not deleted for that reason, and
George's sign-flip art is evidence of his intended vocabulary.

**What exists (2026-10-04).** George's body sheet
(`tools/ambition_sprite2d_renderer/.../targets/characters/george_booul.py`)
has one state parameter: white = TRUE, charcoal = FALSE. Rows: `idle_false`
(the only FALSE loop), `toggle_state` (TRUE→FALSE flip), `not_fade` (NOT:
state to FALSE while opacity dips), `and_zap` (two-hand AND charge), plus
TRUE-only `idle`, `drift`, `hit`, `death`, `taunt` (FALSE variants are one
parameter away). His FX sheet `george_booul_vfx` has 21 rows: a negation
family (`false_sigil`, `binary_toggle`, `not_inversion`, `xor_split`), other
Boolean/ghost rows (`true_sigil`, `and_converge`, `boo_pop`,
`ghost_afterimage`, `proof_collapse`) and rows made for today's specials
(`bivalence_weak/strong`, `modus_ponens_dash/impact`,
`excluded_middle_{windup,launch,ascent,gate,tail}`,
`reductio_{drop,impact,bounce}`). `scripts/measure_fx_row_reachability.py
--owners` reports 1 of 21 rows named, and that name is in a render test, so
gameplay reach is 0. None is superseded: each special-move row targets a move
id that still exists, and the state rows wait for a mechanic.

**What is not wired.** His Smash specials draw generic effects (`sonic_boom`,
`classic_burst`, `smoke_burst`), and his attacks, specials, grab and throws
draw `idle` because his sheet has no `attack`/`special`/`grab`/`throw_*` rows
and `smash_moveset.ron` names none of his own rows. A move may name a row
directly (`anim_index.rs` resolves by row name), so binding the special-move FX
rows and his state rows is content work. The sound cue derived for a row
(`vfx.<family>.<row>`) is missing for the five `excluded_middle_*` rows and
`ghost_afterimage` (the bank packs them under other names); fix the cue names
when the rows are wired.

**Planned gameplay (not built).** George's concept is a hovering trap-zoner
whose TRUE state favours setup and whose FALSE state favours feints and fades.
He is the concrete customer the "Transformation/stance" row (§7) waits for: a
TRUE/FALSE state that a move toggles (`toggle_state`, `binary_toggle`), NOT as
a fade/feint (`not_fade`, `not_inversion`), AND as a two-input charge
(`and_zap`, `and_converge`). Design the mechanic from his concept first; do
not invent a mechanic per asset. Also: `demos/moveset-reviews.md` does not
list George.

**Pirate Admiral FX (same ruling).** `pirate_admiral_vfx` has 14 rows and
nothing names any of them. Superseded by later moves: `boarding_wake` (the old
side-B `boarding_run`, replaced by `run_out_the_guns`) and the three
`grapple_*` rows (up-B `grapple_line`, replaced by `call_the_shark`); keep them
as art until a tether/grab customer or a decision to delete. Match current
moves and are not wired: `black_powder_flash`, `grapeshot_cloud`,
`powder_smoke` (`grapeshot`), `heave_to_anchor`, `heave_to_brake`
(`heave_to`), `cutlass_wake`, `cutlass_clash`, `deck_splinter_burst`.

### Stand-in kit

The standalone demo seats George plus two stand-ins that share one contract
(`smash_duelist_a.ron`). That contract hand-builds its attack verbs instead of
using `SmashRepertoire`, whose nineteen non-`Option` fields make a partial kit
impossible. The stand-in binds four specials (`read_and_seize`, `riposte`,
`lunge_grab`, `slip_upward`) and leaves two special presses unanswered that
George answers; guard
`the_stand_in_is_george_s_genre_shape_with_the_special_button_removed`. Q89
(2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)): a
thin test stand-in may keep an incomplete kit on purpose. Do not invent
specials to fill each input slot. The ruling covers stand-ins and proof
characters only; when the real Robot becomes game content, its move vocabulary
is authored deliberately. The composed app's selectable fighters all have
complete kits
(`report_the_smash_kit_every_selectable_fighter_has`).

## 15. Engine primitives

| ID | Primitive | Class | Status |
|---|---|---:|---|
| `P01` | Move charge state | E1 | ✔ `MoveSpec::charge_gesture`; stored charge on one move. Unreached by CPUs (above). |
| `P02` | Hit reaction policy | E1 | ◐ `HitReaction` has `Strike` and `Windbox`; autolink has a road. Fixed reactions need new variants (`S0.3`/`S0.8`). |
| `P03` | Same-move hitbox arbitration | E1 | ✔ `StrikeRank`. |
| `P04` | Move defense windows | E1 | ✔ `project_move_defense_windows`. |
| `P05` | On-block cancel fact | E1 | ✔ `MoveContact`. |
| `P06` | Ground locomotion phase | E1 | ✔ `LocomotionTuning`; dash and pivot moves derive from it. |
| `P07` | Combat action buffer | E1 | ✔ `BodyActionBuffer`, spent only by acceptance. |
| `P08` | Attack-strength hint | E1 | ✔ `AttackStrengthHint`. |
| `P09` | Shield/OOS arbitration | E1 | ✔ `OutOfShield`, one gate. |
| `P10` | Tech surfaces and result | E1 | ✔ surfaces and presentation. The brain must not see `tumble_untechable`: a tech into an untechable launch costs the option, which makes teching a read. |
| `P11` | Capture acquisition policy | E2 | ◐ one writer of `CaptureAttemptRequested` (`translate_authored_capture_effects`); standing, dash, pivot and command grabs ship; tether, hit-grab and aerial do not (§6). |
| `P12` | Recovery-use budget | E1 | ✔ `recovery_charges`. Helplessness is a stored episode, not `charges == 0`. |
| `P13` | Participant-generic item path | E2 | ✔ `DrivenBodies`. |
| `P14` | Resolved presentation facts | E1 | ✔ charge, unhittable, launch beat, shield-break phase, `KnockoutBeatRequested::eliminated` and `::launch` (the ring-out blast shoots from the crossed edge, opposite the launch), `KnockedOutOfTheWorld` (a ringed-out fighter is not drawn until it respawns), finish zoom. |

None of these needs the actor-monolith carve or composition cleanup first. `E2`
means coordinated work, not a prerequisite.

## 16. Not worth generalizing yet

- a universal stance/transformation framework;
- a generic status-effect scripting VM;
- an exhaustive clone of every Ultimate hitbox flag;
- cinematic Final Smash infrastructure before a resource + authored super proves
  the need;
- Spirits/equipment parity, assist-trophy-scale summoning, collectible meta
  systems;
- broad fighter-AI planner expansion before AI-policy ownership moves;
- online matchmaking or lobby work;
- Melee bug parity (accidental wavedash, L-cancel). Add a wanted technique as a
  deliberate rule.

## 17. Current implementation order

1. The `S0.*` gate, in order `S0.1` to `S0.9`.
2. After all nine close, re-measure this inventory against HEAD and write a new
   post-gate order from the rows that are still open.

A cheap lower row is not higher priority. A shipped-but-unreached mechanic is not
higher priority because it is frustrating.
