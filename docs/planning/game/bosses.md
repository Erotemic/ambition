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

### Re-measured 2026-09-03 — the kit is real, the PLACEMENT line is stale

* ✔ **The kit claims hold where spot-checked.** The signature move is authored
  content, not a plan: `cellular_pulse` appears in the content crate's
  `cellular_automaton_moveset.rs`, `authored_movesets.rs` and
  `authored/perfect_cellular_automaton.rs`, with coverage in
  `game/ambition_content/tests/aerial_authoring.rs` and in
  `crates/ambition_combat/src/brain/smash/action/tests.rs`.

* ⛔⛔ **RETRACTED, SAME DAY — the placement line is CORRECT and my correction
  was wrong.** I wrote here that *"Placed in the Noether Chamber via LDtk"*
  named a room that does not exist. It does exist: it is the level
  **`symmetry_room`** in `sandbox.ldtk`, and the PCA is placed in it exactly as
  this page describes — an `NpcSpawn` with `character_id:
  perfect_cellular_automaton`, `dialogue_id: perfect_cellular_automaton`,
  `brain_override: stand_still` and the prompt *"Challenge the Perfect Cell-ular
  Automaton"*. A dormant NPC with a Challenge branch, which is the encounter
  this page specifies.
  ⭐ Noether's theorem IS the symmetry/conservation law, and the actor kernel's
  switch code calls that room's cardinal gravity switches *"Noether Chamber
  kernel faces"* — so the design name and the level id are the same place.
  ⚠ **How I got it wrong is the useful part, and it is the error this session
  kept finding in other people's rows.** I DID suspect the rename and went
  looking for it — but I checked ONE candidate, `symmetry_chamber`, found it was
  a synthetic combat-test `Stage` rather than a level, and stopped. The actual
  match was `symmetry_room`, one word away, in a world file I had already listed.
  Chasing a hypothesis and stopping at the first near-miss reads exactly like
  having chased it, which is why the write-up sounded careful and was not.

* ⚠ **A sibling this page does not mention.** `imperfect_cellular_automaton` is
  fully authored in `character_catalog.ron:850` — its own display name,
  spritesheet, `tier: MainHall`, `body_kind: Floating` and a
  `hall_dialogue_id` — and it is placed in the same world as the PCA. Whether an
  *Imperfect* Cellular Automaton belongs in a page about failed objective
  functions is a design call for this page's owner, not a gap I should close:
  it may be a peaceful hall NPC by intent. Recorded so the roster is a decision
  rather than an oversight.

## Roster (story bosses)

- **Perfect Cell-ular Automaton** — the dialogue-gated fighter above.
- **Mockingbird** — mimics/steals the player's moves (a boss that proves you can do
  better than your own copied policy).
- **Clockwork Warden** — reads the player's patterns; beating it means breaking pattern.

Each is authored as content data on the engine boss system; none needs a bespoke
simulation path.
