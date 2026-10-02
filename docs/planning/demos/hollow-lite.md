# Track H — Hollow Lite (exploration-combat + boss-quality acceptance demo)

Inspired by Hollow Knight's opening area, ending in a real boss fight.
Parody-original: a small errant automaton in the ruins of a dead machine-colony.

## Purpose

Two proofs in one:

1. The exploration-combat loop is content on the engine: interconnected rooms,
   melee-first combat with pogo, benches and saves, currency loss on death.
2. The boss-design pipeline produces a fight that Jon rates as fun, authored by
   an agent. This demo is the pipeline's acceptance test,
   [`../engine/boss-design.md`](../engine/boss-design.md) BD8.

Engine capabilities it stresses: axis-swept movement with pogo; authored respawn
policy ([ADR 0022](../../adr/0022-respawn-policy.md): mobs respawn on rest, the
boss dies forever); melee, pogo and recoil in the combat resolver; a channelled
heal technique; enemy brains and the boss pattern vocabulary; set-piece arena
beats and encounter phases; bench saves; an LDtk world of about ten rooms.

## Status

Not built. No crate exists. The built demo crates are listed in
[`README.md`](README.md).

H1, H2 and H4 have no blocker outside the engine prerequisites below. H3 is BD8.
BD8 needs BD6 (the playtester rig and its bands), and BD6 needs F1–F4 of
[`../engine/fighter-brain.md`](../engine/fighter-brain.md).

Engine prerequisites: the provider and session lifecycle tracks, encounter
lifecycle convergence, boss actions on the shared moveset path, the
fighter-brain and boss-quality foundations, and authored respawn and save policy.

## Ownership

The game crate (`hollow_content`) owns: the world (well, loop, shortcuts, bench,
boss door); the rules plugin (currency, shade drop on death, bench respawn
policy, mode-scoped); the focus/heal technique registration; four enemy rows; the
boss; HUD; title and results.

## Design (v1 scope)

- **World:** about ten interconnected rooms in one `.ldtk` world. A vertical well
  entrance, a loop with two shortcuts that unlock backward, one bench (the shrine
  save vocabulary), a currency cache, and the boss arena behind a heavy door.
- **Combat:** melee with pogo, directional slashes and two-way hit recoil use
  existing mechanics. A meter charged by hits feeds one heal channel. The heal is
  the one new technique: a technique with a channel window.
- **Death:** drop currency as a pickup at the death site. Respawn at the bench.
  The player's death policy is authored content and uses the same enum as actors.
- **Enemies:** four archetypes on existing brains (crawler, lunger, flyer,
  shielded). Each teaches one verb.
- **Boss:** authored through the full pipeline (seeds, control-flow atoms,
  telegraph grammar, validator, playtester metrics in band), shipped blind, then
  judged by Jon. Target: three phases, one arena beat (BD2), one signature move
  contributed back to the seed library. Its objective is legible and flawed: it
  optimizes for something (guarding its hoard, repeating what last hit it) and
  over-commits.

## Open acceptance

| slice | scope |
|---|---|
| H1 | world, traversal loop, bench and death rules |
| H2 | enemy quartet and the focus/heal technique |
| H3 | the boss (BD8); needs Jon's verdict |
| H4 | hosting wing in Ambition |

Exit: the playtester rig report is in band at three difficulty levels, the fight
validator is green, and Jon records that the fight is fun. A "no" loops H3, and
his feedback goes into the seed library.

## How to run

Nothing to run yet.
