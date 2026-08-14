# Super Smash Siblings — platform-fighter customer

**State:** OPEN serious engine customer; possible future first-class game.
**Project order:** Ambition remains the flagship and primary product driver.

Inspired by early platform fighters while using parody-original characters,
stages and presentation.

## Purpose

Prove that several ordinary controlled bodies with genuinely different authored
movement/combat identities can coexist in one match without a fighter-only body
ontology or player-only combat semantics.

The full body-generic engine plan is
[`../smash-body-generic-combat-2026-08-09.md`](../smash-body-generic-combat-2026-08-09.md).

## Engine capabilities consumed

- prepared `CharacterDefinition`/`PreparedCharacterDefinition` composition and
  ordinary actor construction;
- shared movement kernels and body capabilities;
- body-generic damage, knockback, DI, hitlag/hitstun, charge attacks,
  landing-lag/autocancel, hurtboxes and hitbox tracks;
- participant/action routing for multiple human and AI-controlled fighters;
- fighter-brain profiles through ordinary controller intent;
- LDtk/world IR stage authoring, blast-zone policy and kinematic stage geometry;
- shared presentation/multi-view infrastructure even when an arena normally
  selects one shared view;
- deterministic/headless simulation and rollback-ready match state.

Remaining reusable gaps belong in the focused Smash plan rather than being
implemented privately here.

## What Smash owns

- stock/timer/sudden-death rules;
- percent-style presentation of the shared damage meter;
- roster declaration and character-select UX;
- CPU-fill/difficulty policy;
- stage selection and platform-fighter-specific stage policy;
- respawn platform behavior, match results and victory presentation;
- content tuning and game feel.

## Character composition after D73

A fighter is not `catalog row + archetype + moveset`. The match selects an
authored character identity that preparation resolves to the complete character
body/kit consumed by ordinary actor construction. Catalog/provider source data
may participate in authoring/preparation, but there is no separate enemy
archetype body authority for the match to invoke.

Hosted Smash uses the characters installed by Ambition. A standalone build
installs the character definitions/content it wants through the same supported
provider/SDK seams; it does not depend on Ambition's game-content crate as an
engine substitute.

## Stage authoring

Stages should be authored through supported world tooling. The intended moving
platform stage is deliberately a second consumer of
[`../engine/kinematic-world-objects.md`](../engine/kinematic-world-objects.md):
Ambition motivates the engine feature, Smash proves it is reusable.

## Multiplayer

The eventual game should support multiple local participants and may later use
network transport. Both must feed the same participant/control model described
in [`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md).

Arena matches normally choose a shared framing policy. That is a game/presentation
choice, not a requirement that the engine have only one gameplay view.

## Incremental acceptance

1. deterministic full CPU match through ordinary bodies and fighter brains;
2. several materially different characters/body movement policies in one arena;
3. two or more local human participants through the shared participant/action
   seam;
4. LDtk-authored stages including a moving/kinematic platform;
5. character-select/match/results UX entirely game-owned;
6. remaining body-generic platform-fighter mechanics such as grabs/throws added
   through reusable engine vocabulary when product feel requires them;
7. later, if product investment justifies it, packaging and content depth that
   lets Smash graduate from acceptance customer to first-class game.

## Exit

Smash succeeds architecturally when adding a fighter, stage or match rule does
not require an engine-named Smash branch, CPU and human fighters obey the same
body laws, and the same characters remain ordinary Ambition characters outside
the match ruleset.
