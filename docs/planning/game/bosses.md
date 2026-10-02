# Bosses (game content)

The boss *system* is engine work
([`../engine/boss-system.md`](../engine/boss-system.md)). This page is the
**design language** and the specific bosses. Generic machinery (fighter-brain
verbs, the glider projectile primitive, `CharacterAnim::Special`, the
dialogue→provoke Yarn command) belongs to the reusable actor, projectile,
animation and interaction owners, not to a common `core`. A boss's stats,
tuning, placement and dialogue are game content in `ambition_content`.

## The design language

> Every boss is a failed objective function.

A boss is a character whose flawed optimization the player reads, exploits and
out-learns. Its defeat is the player demonstrating a better policy than the
boss's.

## The Perfect Cell-ular Automaton (the exemplar)

The PCA proves the unified actor pipeline: it is not a special-case boss. It
starts as a talking NPC and becomes a melee boss only if the player chooses
"Challenge" in dialogue. It is the same body and the same `Brain` +
`ActorControlFrame` seam from peaceful to hostile (and, one day, possessed).

- **Concept:** a cellular-automaton entity. Its ranged zoning tool is a Conway
  Game-of-Life glider.
- **Brain:** the fighter brain. Brain output is abstract intent; the per-actor
  `ActionSet` resolves it to concrete verbs. Difficulty is data
  (`reaction_delay_s`, `commit_probability`, `accuracy`), and the brain perceives
  a lagged opponent.
- **Kit (built):** melee, jump, fly-reposition, the glider, blink-evade, the
  aerial dive/perch game, and the `cellular_pulse` signature move
  (`game/ambition_content/src/cellular_automaton_moveset.rs`). Glider, blink and
  fly are body capabilities, so a possessing player inherits them.
- **Encounter:** dormant NPC → Yarn dialogue (a Challenge branch and peaceful
  exits) → combat → win or loss. The dialogue→provoke bridge flips the brain and
  disposition and arms the hostile volumes.
- **Placement:** the design name "Noether Chamber" is the LDtk level
  `symmetry_room`. The PCA is an `NpcSpawn` there (`character_id` and
  `dialogue_id` `perfect_cellular_automaton`, `brain_override: stand_still`). It
  is also placed in `hall_of_characters`.

Remaining PCA work is encounter and narrative polish, not kit.

`imperfect_cellular_automaton` is a separate catalog character (a hall NPC). It
wears the PCA's move table. Whether it is a boss is an open design call.

## Roster (story bosses)

- **Perfect Cell-ular Automaton** — the dialogue-gated fighter above.
- **Mockingbird** — mimics and steals the player's moves.
- **Clockwork Warden** — reads the player's patterns; beating it means breaking
  pattern.

Each is authored as content on the engine boss system. None needs a bespoke
simulation path.
