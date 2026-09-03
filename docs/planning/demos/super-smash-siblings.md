# Super Smash Siblings — platform-fighter product charter

**State:** ACTIVE product push; serious engine customer and possible future
first-class game.
**Project order:** Ambition remains the flagship and primary product driver.

Super Smash Siblings is a Smash-like platform fighter built from ordinary
Ambition bodies, controls, combat, world geometry, presentation, and content.
It is a product in its own right and a pressure test for reusable engine
capabilities.

## Start here

- **Choose or inspect a feature, and start here:**
  [`smash-parity-inventory.md`](smash-parity-inventory.md) is the canonical
  shipped/partial/absent inventory and records the implementation seam for each
  gap.
- ⛔ **The "current push" was `campaigns/smash-fun-push-2026-08-22.md` and it is
  CLOSED** — corrected 2026-09-03, when this was still the first thing a new
  session was told to implement. That file's own header now reads *"execution
  campaign closed; do not use this file as Smash feature status"* and
  *"replaying that chronology is actively harmful because the parity inventory
  has since been reconciled against HEAD"*. ⇒ Read it only for the standing
  lessons it deliberately retains
  ([`campaigns/smash-fun-push-2026-08-22.md`](campaigns/smash-fun-push-2026-08-22.md)),
  never as a task list. ⚠ The campaign closed itself correctly and pointed here;
  this page is what never learned, which is the direction that rot usually
  travels — the closing document knows, the linking one does not.
- **Change reusable combat semantics:**
  [`../engine/combat-model.md`](../engine/combat-model.md) owns the body-generic
  combat contract. The inventory owns feature priority and gap status.
- **Change fighter decision policy:**
  [`../engine/fighter-brain.md`](../engine/fighter-brain.md) owns broad fighter-AI
  evaluation and calibration. A Smash feature may add the smallest semantic
  observation/option support it needs without starting a second brain stack.
- **Change local multiplayer/view behavior:**
  [`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md)
  owns participant/view architecture.

The superseded body-generic successor plan is archived. No open Smash work is
owned by that document.

## Product target

The target is **Smash-like**, not byte-for-byte Ultimate. The engine should be
able to express useful rule differences among platform fighters through normal
content or rules knobs when those differences are worth supporting. Physics
bugs do not need parity.

A strong demo should have:

- responsive neutral built from walk/run, attacks, shield, grab, dodge, jump,
  recovery, ledges, tech, and launch;
- several fighters whose movement and move kits feel materially different;
- readable hit, launch, defense, invulnerability, KO, respawn, and victory
  presentation;
- multiple local human participants plus CPU participants through the same body
  control model;
- several stages with meaningful platform/geometry differences;
- enough rules, character-select, stage-select, results, training, and rematch
  UX that the demo feels like a game rather than a mechanics harness.

The exhaustive feature surface lives in the parity inventory. Do not maintain a
second backlog here.

## Architecture contract

1. **Fighters are ordinary bodies.** Do not add a fighter-only body ontology or
   a second movement/combat implementation.
2. **Human and CPU fighters obey the same simulation rules.** Controllers
   provide intent; they do not define different physics or combat semantics.
3. **Feature-driven engine work is allowed now.** A missing mechanic may add a
   small reusable semantic in the domain that already owns it. Do not wait for
   the actor-monolith carve, simulation-phase migration, capability/runtime
   composition cleanup, or public-facade cleanup merely to ship a cleanly owned
   Smash feature.
4. **Do not hide real engine work in the demo.** The inventory marks small
   reusable additions as `E1`, coordinated engine campaigns as `E2`, and work
   that should wait as `WAIT`.
5. **Simulation owns gameplay truth.** Rendering, shaders, particles, audio,
   cameras, and HUD consume resolved facts/events rather than reconstructing
   charge, vulnerability, shield, hit, or launch rules.
6. **Prefer one reusable semantic over one fighter exception.** Add autolink,
   hitbox arbitration, capture acquisition policy, or a locomotion phase when a
   real move needs it; do not branch on character identity.
7. **Do not pre-generalize.** Stances, status frameworks, cinematic supers, and
   other broad systems wait until a concrete fighter or ruleset defines the
   actual requirement.

## Engine capabilities consumed

Smash already consumes ordinary engine support for:

- prepared character definitions and ordinary actor construction;
- shared movement, collision, body contact, jump, dodge, knockdown, tech, ledge,
  and recovery mechanics;
- move timelines, hurt geometry, hit volumes, damage, knockback, DI, hitlag,
  hitstun, shields/parry, capture, pummel, and throws;
- participant/action routing for human and AI-controlled bodies;
- fighter-brain profiles through ordinary control intent;
- item identity/custody, pickup, held use, throw, and world-item physics;
- world/stage geometry, blast zones, and kinematic stage objects;
- deterministic/headless simulation and rollback state;
- shared VFX, audio, camera, HUD, and multi-view presentation infrastructure.

The inventory records which additional semantics are genuinely missing.

## What Smash owns

Smash owns product policy and content:

- stocks, timer, sudden death, stamina/time variants, teams, item rules, and
  other match rules;
- roster declaration, CPU-fill/difficulty policy, character-select and
  stage-select UX;
- stage layouts and platform-fighter-specific stage policy;
- percent presentation, stocks HUD, respawn-platform behavior, results, rematch,
  victory presentation, announcer/fanfare, and other ceremony;
- fighter move content, frame data, balance, pose selection, character audio,
  and game feel;
- which reusable mechanics earn engine implementation and how the demo tunes
  them.

A product-owned rule may consume reusable engine primitives without moving the
named Smash policy into core.

## Character and stage composition

A match selects authored character identities that preparation resolves to the
complete body/kit consumed by ordinary construction. Hosted Smash may use
characters installed by Ambition; a standalone build installs the content it
wants through supported provider/SDK seams.

Stages should use supported world tooling. Moving platforms, hazards, one-way
platforms, collision geometry, blast zones, and camera bounds should remain
ordinary world/stage concepts rather than a parallel Smash scene format.

## Multiplayer

Local and future network participants feed the same participant/control model.
Arena matches normally choose one shared framing policy, but that is a
presentation choice rather than an engine-wide single-camera rule.

## Product checkpoints

1. **Core fight:** attacks, shield, grab, dodge, movement, launch, recovery,
   ledges, tech, stocks, respawn, and readable feedback support a fun short
   match.
2. **Roster depth:** several fighters exercise distinct reusable move semantics
   without character-ID engine branches.
3. **Local play:** two or more human participants can join, select fighters, and
   complete matches alongside CPUs.
4. **Stage breadth:** several authored stages change spacing/recovery decisions,
   including at least one kinematic-platform customer.
5. **Match completeness:** stage select, rule selection, results/rematch, and
   training/tuning support make iteration and ordinary play coherent.
6. **CPU adoption:** the fighter brain can use and answer the mechanics that
   define the current roster without a Smash-only AI stack.

## Exit

Smash has graduated from acceptance demo to a strong game slice when adding a
fighter, stage, or match rule normally means authoring content or extending one
reusable semantic owner; CPU and human fighters obey the same body laws; the
same characters remain ordinary Ambition characters outside the ruleset; and a
short local match is fun without developer interpretation of what the systems
are doing.
