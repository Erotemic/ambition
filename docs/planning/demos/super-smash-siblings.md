# Super Smash Siblings — platform-fighter product charter

**State:** active product push; serious engine customer and possible future
first-class game. Ambition remains the flagship and primary product driver.

**Current Smash priority:** close the
[`S0.*` raw two-fighter combat-semantics gate](smash-parity-inventory.md#smash-local-p0-raw-two-fighter-combat-semantics)
before any other Smash work. It outranks roster breadth, stages, match rules,
items, training tools, presentation polish, CPU polish and character novelty
inside the Smash lane. Only a blocker to implement or measure the gate, or an
explicit maintainer decision, preempts it. It does not reorder unrelated Ambition
programs.

## Purpose

Super Smash Siblings is a Smash-like platform fighter built from ordinary
Ambition bodies, controls, combat, world geometry, presentation and content. It
is a product in its own right and a pressure test for reusable engine
capabilities: body-generic combat (move timelines, hit and hurt geometry,
shields, capture, knockback, tech, ledges), per-seat participants with human and
CPU control through one body model, the fighter brain, rollback-safe match
rules, stage geometry, and shared-view presentation.

## Start here

- **Feature status and gaps:** [`smash-parity-inventory.md`](smash-parity-inventory.md)
  is the canonical shipped/partial/absent inventory with the implementation seam
  for each gap. Do not keep a second backlog here.
- **Maintainer move intent:** [`moveset-reviews.md`](moveset-reviews.md).
- **Reusable combat semantics:** [`../engine/combat-model.md`](../engine/combat-model.md).
- **Fighter decision policy:** [`../engine/fighter-brain.md`](../engine/fighter-brain.md).
  A Smash feature may add the smallest observation or option support it needs;
  do not start a second brain stack.
- **Local multiplayer and views:** [`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md).

Standing lessons:

- A guard for a new mechanic uses the production input, spawn and authoring road.
- Fresh-state defaults must not represent the spent state.
- Independent grants need independent ownership, not one shared single-slot
  component.
- Resolve reference-frame and attacker-local facts at the producer that has them.
- Smash is a customer of generic engine mechanics, not a reason for a second body
  or combat path.
- Reproduce an apparent input-ordering defect through the real input and control
  road before you change gesture semantics.
- An instrument configured differently from the shipped game measures the
  instrument. Probe the shipped ladder (`--ladder`) and the shipped match clock.

## Product target

The target is **Smash-like**, not byte-for-byte Ultimate. The engine should
express useful rule differences among platform fighters through content or rules
knobs when they are worth supporting. Physics bugs do not need parity.

A strong demo has:

- responsive neutral from walk/run, attacks, shield, grab, dodge, jump,
  recovery, ledges, tech and launch;
- several fighters whose movement and kits feel materially different;
- readable hit, launch, defense, invulnerability, KO, respawn and victory
  presentation;
- several local humans plus CPUs through the same body control model;
- several stages with meaningful geometry differences;
- enough rules, character select, stage select, results, training and rematch UX
  that it feels like a game.

## Architecture contract

1. **Fighters are ordinary bodies.** No fighter-only body ontology and no second
   movement or combat implementation.
2. **Human and CPU fighters obey the same simulation rules.** Controllers provide
   intent only.
3. **Feature-driven engine work is allowed now.** A missing mechanic may add a
   small reusable semantic in the domain that owns it. Do not wait for the
   actor-monolith carve, phase migration or facade cleanup.
4. **Do not hide engine work in the demo.** The inventory marks `E1`, `E2` and
   `WAIT`.
5. **Simulation owns gameplay truth.** Rendering, audio, cameras and HUD consume
   resolved facts.
6. **Prefer one reusable semantic over one fighter exception.** Never branch on
   character identity.
7. **Do not pre-generalize.** Stances, status frameworks and cinematic supers
   wait for a concrete fighter or ruleset.

## What Smash owns

Product policy and content: stocks, timer, sudden death, mode variants, teams,
item rules; roster declaration, CPU fill and difficulty policy, character and
stage select UX; stage layouts; percent HUD, respawn platform, results, rematch,
victory ceremony; fighter move content, frame data, balance, poses, audio and
feel; which reusable mechanics earn engine work. A product rule may consume
engine primitives without moving named Smash policy into core.

## Composition

A match selects authored character identities that preparation resolves to the
complete body and kit used by ordinary construction. Hosted Smash may use
characters installed by Ambition; a standalone build installs its content through
provider/SDK seams. Stages use supported world tooling: moving platforms,
hazards, one-way platforms, blast zones and camera bounds stay ordinary world
concepts. Local and future network participants feed the same participant model.
An arena normally uses one shared framing, which is a presentation choice, not an
engine-wide single-camera rule.

## Product checkpoints

| # | checkpoint | status |
|---|---|---|
| 1 | **Core fight:** attacks, shield, grab, dodge, movement, launch, recovery, ledges, tech, stocks, respawn and feedback support a fun short match. | ◐ The basic loop works. The `S0.*` interaction gate is open. |
| 2 | **Roster depth:** several fighters exercise distinct reusable move semantics without character-ID engine branches. | ✔ Roster depth holds. Roster reach in the standalone demo is thinner: two stand-ins share one contract with four specials and two unanswered special presses. A stand-in may stay incomplete (Q89 ruling, 2026-10-04). |
| 3 | **Local play:** two or more humans join, select fighters and finish matches with CPUs. | ✔ `smash_tool select-walkthrough` drives the real select screen headlessly and prints what it shows. |
| 4 | **Stage breadth:** several stages change spacing and recovery decisions, including one kinematic-platform customer. | ◐ Three stages (flat, platforms, narrow) differ on two axes. No kinematic-platform stage yet. |
| 5 | **Match completeness:** stage select, rule selection, results/rematch, training/tuning. | ✔ Stage and stock cycles on the select screen, rematch, and the `smash_tool` probes. |
| 6 | **CPU adoption:** the fighter brain uses and answers the roster's mechanics without a Smash-only AI stack. | ◐ The brain answers shields and ledges. Censuses found it rarely uses smashes, shield or grab, and it cannot perceive a capture it is not part of (see the inventory's *Shipped but unreached*). |

## Exit

Smash graduates from acceptance demo to a strong game slice only after the
`S0.*` gate closes. After that, adding a fighter, stage or rule normally means
authoring content or extending one reusable semantic owner; CPU and human
fighters obey the same body laws; the characters stay ordinary Ambition
characters outside the ruleset; and a short local match is fun without developer
interpretation. Stage, roster or match completeness cannot substitute for an open
`S0.*` packet.

## How to run

```bash
./run_game.sh smash                          # standalone shell, opens on character select
./run_game.sh smash-match                    # a live round of the shipped composition
cargo test -p ambition_demo_smash -p ambition_demo_smash_app
cargo run -p ambition_demo_smash_app --bin smash_tool -- --help
```

`smash_tool` is one binary with subcommands (`capture-probe`, `ladder-probe`,
`ladder-rig`, `match-diagram`, `match-report`, `match-shots`, `roll-probe`,
`select-walkthrough`, `stage-diagram`). `match-shots` needs
`--features visible,capture`; without them it exits non-zero and names the
features. `--features causal` adds the engine's own resolution facts to
`ladder-probe` and `match-report`.

Binary rules (the modal CLI):

- Collapse binaries within one crate, never across crates. Each `capture_*` tool
  stays in the demo crate it photographs; one binary for all of them would couple
  demos (`scripts/check_absence_contracts.py`,
  `capability-footprint-may-not-grow`).
- Product entry points (`ambition_game_bin` and the `*_demo` shells) stay
  standalone binaries.
- `smash_tool` declares no `required-features`. Gate a subcommand at its own
  definition, keep its name in `--help`, and exit non-zero naming the features.
- Subcommand names are kebab-case and public. Flags keep their spelling.
- `split-debuginfo` stays unset (see the note beside `[profile.dev]` in the
  workspace `Cargo.toml`).
- `cargo clean -p` does not remove a binary whose target is gone. After you
  rename or remove a bin, delete its old artifact by name.
