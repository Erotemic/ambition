# Bosses (game content)

The *system* is engine ([`../engine/boss-system.md`](../engine/boss-system.md)); this
is the **design language** and the specific bosses. The engine machinery (Smash brain
verbs, the glider projectile primitive, `CharacterAnim::Special`, the dialogue→provoke
command) lives in core; a boss's stats, tuning, placement, and dialogue live in
content.

---

## The design language

> Every boss is a failed objective function.

A boss is a character whose flawed optimization the player reads, exploits, and
out-learns. A boss *proves the player can learn* — its defeat is the player
demonstrating a better policy than the boss's. Concretely: the Mockingbird mimics and
steals your moves; the Clockwork Warden reads your patterns; the PCA is a cellular
automaton converging on a poor fixed point.

## The Perfect Cell-ular Automaton (the exemplar)

The PCA is the proof that the unified actor pipeline works: it is **not a special-case
boss**. It starts as a talking NPC and becomes a reactive melee boss *only if the
player chooses "Challenge it"* in dialogue — the same body, the same `Brain` +
`ActorControlFrame` seam, from peaceful to hostile to (one day) possessed.

- **Concept:** a cellular-automaton entity; its ranged zoning tool is a Conway
  Game-of-Life **glider**.
- **Brain:** the Smash fighter — a 5-stage utility pipeline (observe → mode → action →
  difficulty filter → emit). Brain output is abstract *intent*; the per-actor
  `ActionSet` resolves it to concrete verbs (the policy/capability split). Difficulty
  is data: `reaction_delay_s`, `commit_probability`, `accuracy` — it perceives a
  *lagged* opponent, so it can't frame-perfectly counter.
- **Kit:** melee (approach / dash / jab / reactive block), jump, fly-reposition, the
  **glider** ranged poke, **blink-evade**, the aerial dive/perch game, and the
  data-driven **Cellular Pulse** signature move are all landed (glider/blink/fly are
  body-enforced capabilities, so a possessing player inherits them). Remaining work is
  encounter/narrative polish, not kit.
- **Encounter:** dormant NPC → Yarn dialogue (a "Challenge" branch + peaceful exits) →
  combat → win/loss. The dialogue→provoke bridge flips the brain + disposition and arms
  the hostile volumes. Placed in the Noether Chamber via LDtk as a peaceful archetype.

> Engine vs content split: generic machinery (Smash verbs, the glider primitive,
> `CharacterAnim::Special`, the dialogue→provoke Yarn command) lives in the engine;
> the PCA's stats / tuning / placement / dialogue live in `ambition_content`.

## Roster (story bosses)

- **Perfect Cell-ular Automaton** — the dialogue-gated fighter above.
- **Mockingbird** — mimics/steals the player's moves (a boss that proves you can do
  better than your own copied policy).
- **Clockwork Warden** — reads the player's patterns; beating it means breaking pattern.

Each is authored as content data on the engine boss system; none needs a bespoke
simulation path.
