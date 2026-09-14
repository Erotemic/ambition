# Combat model — engine contract

**State:** ACTIVE body-generic contract. Smash feature priority and gap status
live in
[`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md).
The completed combat campaign record is archived at
`../../archive/planning-superseded/2026-08-13/engine/combat-model.md` (docs/archive/planning-superseded/2026-08-13/engine/combat-model.md — removed from the checkout 2026-09-05; still in git history).

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

✔ **Re-measured 2026-09-03: every name in the table above still resolves**, and
the map is worth reading with one split made explicit, because the table does not
say which crate each name lives in:

* the **authored vocabulary** is in `ambition_entity_catalog` —
  `MoveSpec` (`crates/ambition_entity_catalog/src/lib.rs:1097`), `MoveWindow`
  (`:776`), `HitVolume` (`:332`);
* the **runtime authority** is in `ambition_combat` — `capture`
  (`crates/ambition_combat/src/lib.rs:29`), `CapturedBy`
  (`crates/ambition_combat/src/capture/mod.rs:20`);
* the **action-acceptance seam** is lower still — `BodyActionBuffer` in
  `crates/ambition_platformer2d_core/src/body_clusters.rs:1063`.

⇒ That is consistent with the scope statement above rather than a drift from it:
what a move IS is authored data in the catalog, what a hit DOES is combat's, and
what a body will ACCEPT is the core's. Recorded because a reader treating this
table as a map to the code has three crates to visit, not one.

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

## Damage and launch variants

New reaction forms should be explicit authored policy. If a fighter needs fixed
knockback, autolink, wind/vacuum, weight independence, shield-only tuning, or a
per-hit hitlag modifier, represent that property on the hit/reaction payload and
keep the ordinary formula unchanged for ordinary hits.

## Kill envelope calibration

Measured 2026-09-14 over 21 fighters / 500 rows. Every KO% here is rage-pinned
(attacker meter 0), no-DI, no-recovery, against `player_robot_v3`, so all of them
are **lower bounds**. The global `victim_percent_knockback_scale = 1.25` was not
touched. Target band 80-160%.

**The vertical blast line is a stage constant; the lateral one is not.** Over ten
resolved `smash_up` rows the launch required to cross the rise line is ~1241
(spread 0.61%) even though bases span 112-178 and growths 1.90-6.40 — so the
authoring lever there is `growth = (1241 - base) / (1.25 * target_KO%)`. The same
derivation on `smash_forward` gives 687.9-820.1, a 17.94% spread. Four hypotheses
were falsified and should not be re-walked: hitbox X offset (spans 20px, moves
launch by 4), `launch_dir` (three fighters share `(1.0, -0.50)` and land 675/724/751),
`smash_charge_mult` (the source says it scales *damage*), and a `+damage` term
(17.94% → 17.62%). Recorded as unexplained.

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

- The host reads `assets/data/movesets/*.ron` off disk. Editing a `*_moveset.rs`
  for a migrated table is a no-op until
  `cargo run -p ambition_app_tools --bin moveset_source_export -- <table>`;
  non-migrated fighters need a rebuild instead.
- Regeneration must cover the **borrowers**, not just the donor table. A borrower's
  `.ron` keeps the stale value until it is regenerated itself.
- Identical base+growth is **not** proof of a shared table. Verify sharing by the
  move id *and* the file; borrows are declared in `archetype_moveset.rs`.
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

The [responsibility map](architecture-responsibility-map.md) supersedes any
inference that everything in `ambition_combat` belongs to combat. Combat owns
accepted hit/block/capture/reaction and move execution state. Stocks and win/loss
are rules; camera impulses/banners are presentation consequences; brain scoring
is decision policy; falling-chest/path motion is a world mechanism; held-item
custody is the item authority. Cross-cutting feel tuning needs field-by-field
owners, not another shared tuning bag.

A destructible's intact/broken state, accepted damage, loot/spawn transition,
collision presence and replay policy should close over one destructible-object
authority. Combat supplies an accepted hit; it need not own every interactive
object that can receive damage. Packet A5 waits for A2's contact contract.

[A2](actor-monolith-work-frontier.md) first aligns boss/projectile contact with
published authored geometry and world obstruction. It then removes historical
family reclassification. A contact can be real even when it causes no damage;
invulnerability, blocking, projectile consumption and damage must remain distinct
policies. Do not replace a direct legitimate dependency with a target registry.
