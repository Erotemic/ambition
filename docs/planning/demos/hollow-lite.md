# Track H — Hollow Lite (exploration-combat + boss-quality acceptance demo)

Inspired by Hollow Knight's opening area (Forgotten Crossroads energy)
ending in a real boss fight (False Knight energy). Parody-original: a
small errant automaton in the ruins of a dead machine-colony.

**Purpose:** two proofs in one. (1) The exploration-combat loop —
interconnected rooms, melee-first combat with pogo, benches/saves,
currency-loss-on-death — is content on the engine. (2) **The boss-design
pipeline produces a fight Jon rates as actually fun, authored by an
opus-level agent** ([`../engine/boss-design.md`](../engine/boss-design.md)
BD8 — this demo is that pipeline's acceptance test).

**Depends on:** E5-finish; boss pipeline BD1–BD7 (vocabulary, seeds,
validator, playtester rig, calibrated on the pilot re-author); fighter
brain FB1–FB4 (the playtester); combat-model CM5 (per-move sfx/vfx);
respawn-policy unification (dead-stays-dead + authored Mob respawn —
this demo is the respawn policy's real consumer: mobs respawn on bench
rest, the boss dies forever).

## Design (v1 scope)

- **World:** ~10 interconnected rooms, one .ldtk world: a vertical well
  entrance, a loop with two shortcuts that unlock backward (the
  metroidvania contract), one bench (shrine=save vocabulary exists), a
  currency cache, the boss arena behind a heavy door.
- **Combat feel:** nail-analog melee with pogo (exists), directional
  slashes (exists), hit recoil both ways (exists via knockback), soul-
  analog meter charged by hits → one heal channel (a technique with a
  channel window — the focus/heal is the one new technique; [opus]).
- **Death rule:** drop currency as a shade-analog pickup at death site;
  bench respawn (respawn-policy: the PLAYER'S death policy is authored
  content, exercising the same enum actors use).
- **Enemies:** 4 archetypes on existing brains (crawler, lunger, flyer,
  shielded) — each teaching one verb, per the boss pipeline's
  answer-coverage philosophy applied to trash design.
- **THE BOSS:** authored by an opus agent through the full pipeline —
  seeds + control-flow atoms + telegraph grammar + validator + playtester
  metrics in band + BLIND ship → Jon's verdict. Target: 3 phases, one
  arena beat (BD2), one signature move contributed back to the seed
  library. A failed objective function, legible: it optimizes for
  something (guarding its hoard; repeating what last hit you) and
  over-commits.

## Slices

H1 world + traversal loop + bench/death rules [opus]; H2 enemy quartet +
focus/heal technique [opus]; H3 the boss (the BD8 acceptance) [opus +
Jon]; H4 hosting wing in ambition [opus].

**Exit:** doctrine exits + the quality one: the playtester rig report is
in band at 3 difficulty levels, the fight validator is green, AND Jon's
taste pass says the fight is fun (recorded verdict; a NO loops H3 with
his feedback banked into the seed library — the pipeline improving is
part of the exit).
