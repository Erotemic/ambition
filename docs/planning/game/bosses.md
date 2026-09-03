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

> ⭐ **RE-VERIFIED against `8bb0dd5a7` (2026-09-03)** (this page had gone two months unread, the oldest
> in `docs/planning/`, and every claim above holds). The kit is where it says:
> `cellular_pulse` is named in five files including its own
> `game/ambition_content/src/cellular_automaton_moveset.rs`, and the glider,
> blink and dialogue→provoke bridge all resolve.
>
> ⚠ **ONE NAMING TRAP, which is the only thing a reader would trip on.** There
> is no LDtk level called "Noether Chamber" — that is the DESIGN name, and it
> survives in the tree only as a comment in `character_catalog.ron` ("Symmetry
> tutorial (Noether Chamber)"). The authored level id is **`symmetry_room`** in
> `sandbox.ldtk`. ⇒ Search for the design name and you conclude the placement
> was never made. ⭐ And it is placed TWICE: `symmetry_room` and
> `hall_of_characters`, the second of which this page does not mention.

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

* ⛔ **"Placed in the Noether Chamber via LDtk" names a room that does not
  exist.** There is no Noether Chamber in any world file. The PCA is authored
  into `hall_of_characters.ldtk` (and `sandbox.ldtk`) as the entity
  `hall_perfect_cellular_automaton`. The only "Noether" in the world data is a
  *character*, `hall_npc_emmy_noether`.
  ⚠ I checked the obvious rename before concluding this, because Noether's
  theorem IS the symmetry/conservation law and `symmetry_chamber` exists — but
  it is a synthetic combat-test `Stage` in
  `crates/ambition_combat/src/brain/smash/arena.rs:89`, not a room. So the
  chamber was not renamed; the placement line describes an arrangement the
  content no longer has.

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
