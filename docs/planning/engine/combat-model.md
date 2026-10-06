# Combat model — engine contract

**State:** ACTIVE body-generic contract. Smash feature priority and gap status
live in
[`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md).
Git history has the completed combat campaign record.

This page defines no numbered slices. Cite a mechanism by name, not by a slice
label from another campaign.

## Scope

Combat owns reusable body-to-body attack and reaction semantics. It does not own
Smash match rules, fighter identity, controller identity, stage policy, or
presentation styling.

A feature belongs here when the mechanic should behave the same for an ordinary
Ambition body, a Smash fighter, a human-controlled body, and a CPU-controlled
body given the same authored state and control intent.

## Current authority map

| Concern | Authority |
|---|---|
| Move timing and authored windows | `MoveSpec` / `MoveWindow` plus per-use move playback |
| Hit geometry and authored hit payload | `HitVolume` and the move/hitbox runtime |
| Victim eligibility and friendly/shield interaction | shared combat victim-resolution path |
| Damage, launch, DI, hitlag, hitstun | shared hit-response/combat state |
| Shield resource, coverage, stun, pushback, break | body shield state + shield tuning + combat resolution |
| Capture, pummel, release, throw | `ambition_combat::capture`, `CapturedBy`, capture requests |
| Stale-move accounting | combat-owned stale-move state |
| Action acceptance/buffering | body/control action authority; the existing `BodyActionBuffer` is the intended combat buffer seam |
| Presentation | resolved combat/read-model facts and events consumed by VFX/audio/camera/HUD |

Do not create a second authority because one game needs richer presentation or
a new reaction type.

The names live in three crates. What a move *is* is authored data in
`ambition_entity_catalog` (`MoveSpec`, `MoveWindow`, `HitVolume`). What a hit
*does* is `ambition_combat` (`capture`, `CapturedBy`). What a body will *accept*
is `ambition_platformer2d_core` (`BodyActionBuffer`).

## Extension rules

1. **Add semantics at the narrow owner.** A new knockback form extends hit
   reaction; a new cancel condition extends move/cancel semantics; a new capture
   entry form feeds the existing capture relationship.
2. **One body rule for every controller.** Combat systems consume resolved body
   control/state, not raw keyboard/gamepad state or `PrimaryPlayer` identity.
3. **Presentation does not decide combat.** Shaders, particles, audio, cameras,
   and poses read resolved charge, shield, hit, vulnerability, launch, capture,
   and KO facts.
4. **Use authored policy before adding code.** Existing move windows, events,
   gates, motion, landing lag, autocancel, on-hit effects, and rules knobs should
   express the feature when they are sufficient.
5. **Do not encode one mechanic through another.** Autolink is not capture;
   fixed knockback is not a magic growth constant; invulnerability is not a
   transparent sprite; command grabs do not need a second grabbed-body state.
6. **Small reusable engine work may ship with the feature.** `E1` rows in the
   Smash inventory are intended feature-driven extensions. They do not wait for
   the actor-monolith carve, simulation-phase migration, or capability/runtime
   composition cleanup.
7. **Coordinated work gets a campaign.** `E2` rows still have a clear owner, but
   touch enough systems that they should be planned/tested as a focused engine
   slice rather than hidden inside fighter content.
8. **Do not pre-generalize `WAIT` rows.** Require a concrete fighter/ruleset to
   establish the actual state and transition contract first.

## Preferred reusable seams

For direct projectile contact, the [contact protocol](projectile-contact-protocol.md)
is the detailed owner: actual travel segments, published geometry, compound
object surfaces, same-step interception and later identity-targeted reception.
Do not replace this with a generic damageability marker or broadcast re-query.
Its sampled-target sweep does not claim full dynamic CCD. Move-confirmation
latency and checked flow execution are owned by
[authored technique admission](authored-technique-admission.md).


The Smash inventory's `P01`–`P14` index is the current product-driven list of
missing reusable semantics. In combat, the important families are:

- explicit per-use move charge state;
- hit-reaction modes such as fixed, autolink, and flinchless reaction;
- deterministic same-move hitbox arbitration for sweetspots/parts;
- live consumption of authored invulnerability/armor windows;
- block-contact cancel conditions;
- the body-owned combat action buffer;
- one capture acquisition policy feeding the existing capture relationship;
- resolved combat facts/events for presentation.

The inventory owns whether each is shipped, partial, or absent. Do not duplicate
that status here.

## Capture

`CapturedBy` is the one temporary hold relationship. Standing, running, pivot,
command, aerial, tether, and hit-grab acquisition may differ in eligibility, but
a successful acquisition enters the same capture authority. Throws leave capture
through the ordinary launch/damage path.

Do not merge capture with mount/possession and do not create a Smash-only captive
body type.

## Shields and defensive windows

The existing body shield resource remains the one shield authority. New shield
features extend its state/resolution rather than creating a second shield
subsystem.

Move-authored `Invuln` / `Armor` windows should affect combat eligibility or hit
reaction in the combat runtime. Rendering may visualize the resolved result but
must not implement the mechanic.

### Hazards beat a ledge hang (Q43)

Ruling Q43 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
holding a ledge gives no implicit immunity from a hazard. A hanging body in a
lethal hazard gets the normal hazard result; an exception is an explicit,
named gameplay rule.

Two roads give the immunity today:

1. **Kernel hazard gate.** `apply_world_hazard_gate` runs only when the
   simulation phase reaches `SimPhaseReach::Completed`
   (`ambition_platformer2d_core/src/movement/mod.rs`). A frame that an active
   ledge grab consumes short-circuits, so a hanging body is never judged. The
   doc comment on `SimPhaseReach` and the test
   `movement/tests/hazard_sweep.rs::a_hanging_body_is_not_judged_by_the_hazard_gate`
   pin this immunity; both flip. Keep the other two short-circuits (a zero-dt
   tick, a drowning) as they are: the ruling is about the hang, and a frozen
   frame still judges no body.
2. **Combat hazard volumes.** `ambition_combat/src/hazards.rs` skips a body
   that is not `body_vulnerable`, and `BodyFacts::evading()` includes
   `ledge_intangible` (the grab window, `LEDGE_GRAB_INVULN_TIME`). Decision
   for this work: the ledge-grab window protects from attacks only; a hazard
   volume ignores `ledge_intangible`. Dodge, getup and other intangibility
   keep their present hazard behaviour (Q43 does not rule on them).

Work is queue row HAZARD-BEATS-LEDGE. Acceptance: a body hanging on a lip with
a lethal hazard under it dies on both roads (witness), the same body on a lip
without a hazard keeps hanging (control), and the attack-only reading of the
grab window has its own test (an attack during the window misses; a hazard
during the window kills).

Ledge occupancy (two bodies on one ledge) is a separate defect, owned by
[`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md#ledge-occupancy)
§5 (queue row LEDGE-OCCUPANCY).

## Damage and launch variants

New reaction forms should be explicit authored policy. If a fighter needs fixed
knockback, autolink, wind/vacuum, weight independence, shield-only tuning, or a
per-hit hitlag modifier, represent that property on the hit/reaction payload and
keep the ordinary formula unchanged for ordinary hits.

### The frame of an authored launch direction

An authored launch vector is local to the body that authors it: `+x` is the way
that body faces and `+y` is toward its feet. This is one law for a move volume
(`HitVolume::launch_dir`), a throw (`CaptureThrowParams`, `BodyHold::Throw`) and
a hold's geometry (`BodyHold::Seize`, the hold point).

The PRODUCER lowers the vector once, to a world direction, with the source
body's facing and resolved frame (`hit_response::WorldLaunchDir`): the hitbox
resolver for a volume, `apply_capture_throws` for a throw. `HitKnockback`
carries the world direction. The victim's reaction applies magnitude and DI and
does not read the direction again on the victim's axes. A captor and a captive,
or an attacker and a victim, in two gravity frames agree about the launch.

Two cases are not this default, and each is named:

- A hit with NO authored direction launches on the default diagonal: to the side
  away from the source, with a rise against the VICTIM's gravity.
- `HitboxLaunch::AwayFromSource` is a second authoring for a volume that pushes
  out from its source on each side (the riding-hitbox port, a boss's buck). Its
  `x` is away from the source on the source's side axis. The producer lowers it
  also.

Consequence: a volume that catches a body BEHIND its owner launches the body the
way the owner faces, through the owner. A move that must send a body behind it
backward authors a negative `x` on that volume.

## Kill envelope calibration

Measured over 21 fighters and 500 rows. Every KO% here is rage-pinned
(attacker meter 0), no-DI, no-recovery, against `player_robot_v3`, so all of them
are **lower bounds**. The global `victim_percent_knockback_scale = 1.25` was not
touched. Target band 80-160%.

**The vertical blast line is a stage constant; the lateral one is not.** Over ten
resolved `smash_up` rows the launch required to cross the rise line is ~1241
(spread 0.61%) even though bases span 112-178 and growths 1.90-6.40 — so the
authoring lever there is `growth = (1241 - base) / (1.25 * target_KO%)`. The same
derivation on `smash_forward` gives a 17.94% spread, still unexplained. Hitbox X
offset, `launch_dir`, `smash_charge_mult` and a `+damage` term do not explain
it; do not re-test them.

**Where a KO% is already measured, no stage constant is needed.**
`G_new = G_old * p0/p1` landed 29/29 pre-registered predictions with 0 refusals
across the up/forward/down smash passes, so it is the preferred method and the
vertical constant is the fallback for a censored cell.

**A pure spike cannot KO at any magnitude.** `launch_dir (0.0, 1.0)` drives a
grounded victim into the floor and crosses no blast line — carl's down-smash
measured `>600` at `ceiling=600`, where its launch would already be double the
lateral line. That is a fixture limit, not content weakness; making it kill means
changing the vector, which is a character-identity decision.

**Censoring is role-structured and validates the corpus.** Ground normals censor
at 95-100% because they were never kill moves; `*_up` pays the vertical tax;
`*_down` spikes; aerials hit the fixture's standing victim. A censored cell is a
claim about a role, not a verdict on a fighter. ⛔ Scope a tuning pass on **the
band**, never on whether the instrument hit its ceiling — that error left two
fighters (ninja 300, clerk 271) out of a pass they belonged in.

**What is deliberately left outside the band.** `npc_emmy_noether` and `npc_oiler`
are each built around one licensed kill move with everything else capped by a
named constant (`BREAK_GROWTH`, `TORQUE_GROWTH`) and enforced by
`exactly_one_move_grows_like_a_kill_move`. Two shared archetype tables move 3 and
4 fighters per edit, which is a roster decision. Role bands are sanity ranges, not
normalization targets, and authored ordering is preserved in every pass.

**Authoring hazards, all of which have bitten.**

- The host reads `assets/data/movesets/*.ron` through the content pack, and those
  files are the source. Fighters whose tables are still Rust (the robot lineage,
  and the Mary-O, Sanic and Smash demos) need a rebuild instead.
- A **borrower** (`borrows: (archetype, prefixes)` in its file) holds only what it
  changes, so editing the archetype's file retunes every borrower. There is no
  second copy to regenerate.
- Identical base+growth is **not** proof of a shared table. Verify sharing by the
  move id *and* the file; a borrow is declared in the borrower's own file.
- Six fighters share the literal id `smash_forward` and six share `smash_up`. Any
  helper that takes the first match across all moveset files reads an arbitrary
  fighter; resolve from the fighter's own file and assert the parsed base/growth
  equals the baseline's.
- A guard that boots the host and reads `PreparedCharacterRegistry` must run
  **after** the regen, or it measures the old growths and reports a meaningless pass.

**Open for the maintainer, surfaced but unchanged:** whether `rage_max_scale`
should stay 1.4 (strikes compress ~36% at the cap, so a 155% target kills nearer
110% against a damaged attacker) — a global decision over 22 fighters, like the
frozen 1.25; and whether the roster-wide generic throw scaling should be replaced
at all. `up_throw` and `down_throw` read `>300` for three unrelated fighters while
forward/back throws kill at 96-168, which is the same role structure rather than
per-fighter weakness.

## Body scale and equipment

Equipment-driven body scaling remains separate from Smash parity. If it is still
desired, one resolved size value must feed collision/simulation and presentation.
Do not patch body size locally in Smash.

## Exit criterion for a combat extension

A combat addition is complete when:

- the mechanic is body-generic and deterministic;
- human and CPU controllers reach it through the same body state/control seam;
- rollback/snapshot state covers gameplay-affecting state;
- presentation consumes explicit results instead of duplicating rules; and
- a real fighter or game feature demonstrates the semantic without a
  character-ID or Smash-mode engine branch.

## Package contents are not the combat boundary

Not everything in `ambition_combat` belongs to combat (see the target
authorities in [engine architecture](architecture.md#target-logical-authorities)).
Combat owns accepted hit, block, capture and reaction and move execution state.
Stocks and win/loss are rules; camera impulses and banners are presentation;
brain scoring is decision policy; falling-chest and path motion are world
mechanisms; held-item custody is the item authority. Cross-cutting feel tuning
needs field-by-field owners, not another shared tuning bag.

A destructible's intact/broken state, accepted damage, loot and spawn
transition, collision presence and replay policy close over one
destructible-object authority (A5 in the
[packet catalog](actor-monolith-work-frontier.md)). Combat supplies an accepted
hit; it need not own every object that can receive damage.

A contact is resolved against published authored geometry and world obstruction
(A2), never by feature-family reclassification. A contact can be real even when it causes no damage;
invulnerability, blocking, projectile consumption and damage must remain distinct
policies. Do not replace a direct legitimate dependency with a target registry.
