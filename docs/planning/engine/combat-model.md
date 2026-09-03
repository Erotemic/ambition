# Combat model — engine contract

**State:** ACTIVE body-generic contract. Smash feature priority and gap status
live in
[`../demos/smash-parity-inventory.md`](../demos/smash-parity-inventory.md).
The completed combat campaign record is archived at
[`../../archive/planning-superseded/2026-08-13/engine/combat-model.md`](../../archive/planning-superseded/2026-08-13/engine/combat-model.md).

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
