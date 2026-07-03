# Ambition — Master Plan

This directory is the **plan state** for Ambition: where it's going and *how we
build it here*. It is deliberately small and principle-dense. Operating rules for
agents live in [`AGENTS.md`](../../AGENTS.md); this is the *what* and the *why*,
ordered by elegance.

> Read this, then the one engine or game doc your task touches. Don't read the
> whole tree.

---

## North star

> **Every upgrade a theorem, every boss a failed objective function, every biome a
> math world model.**

Ambition is, first, a **reusable 2D-platformer engine**. The game is its first
*content crate* — the proof that the engine composes. We are **engine-first**: the
engine must become *perfect* — so easy and composable that anything you'd want in a
real platformer falls out of adding data and a small content crate, with little
code. The game (story, bosses, biomes) is real and we care about it, but it is
**secondary to and downstream of** an elegant engine.

There is no release, no production code, **nothing depends on this yet**. That is a
gift: we get the base right *before* we build the game on it.

## The four goals

Everything we do serves one or more of these. If a change serves none, it is not
the work.

1. **Fast compile time.** ~150k LOC compiles too slowly; design for incremental
   rebuilds. Extracting leaf crates is the lever — blocked today by one dependency
   sink (see the keystone refactor in the roadmap).
2. **Agent-navigability.** The codebase must be navigable by an AI agent. That
   means the *right abstractions* and getting **named content** (bosses, abilities,
   rooms) **out of the foundation crates into content**, generalized where possible.
3. **Idiomatic Bevy plugins.** Each subsystem is a plugin that owns its vocabulary,
   its authoritative state, its schedule sets, and its public extension points
   (the `ambition_portal` split is the exemplar).
4. **Audit-grade reusability.** A *second* platformer game should be buildable by
   adding only a content crate atop the unmodified engine — zero core edits. This
   is the design oracle: *"could another platformer be built by ADDING a content
   crate without editing core?"*

## How we build here (the stance)

The full statements live in [`AGENTS.md`](../../AGENTS.md); the spine:

- **Elegance is the objective function. Correctness emerges from it.** Find the
  elegant solution; reject hacks. Smaller, composable code that does more.
- **Behavior and feel are NOT sacred** (pre-release, zero dependents, no polish
  pass). Refactor for elegance even when output changes. The gates are: *it
  compiles* (incl. `ambition_app`) and *invariants hold* — not bit-identical replay.
  Don't write regression tests to pin unpolished behavior.
- **Relativity, not player-centrism.** Mechanics are frame-agnostic and shared by
  every actor; nothing assumes `-y` is up or that the player is special.
- **Delete, don't bridge. Rename in place, don't alias. Add a seam when the second
  use case lands.** Pre-release means single-commit replacement over compat shims.
- **Verify against the REAL headless sim**, never a proxy. The game runs headless
  (`SandboxSim.step`, the `headless`/`trace_replay` binaries) — step the actual
  simulation from any state and observe. "Can't test it" is only true of subjective
  *visual feel*, and even that is headed for headless render-to-disk. The strongest
  tests are **symmetry/covariance** (an action identical under C4 gravity rotation
  and through portals) — they survive feel tweaks.

## The map

### `engine/` — the product

- **[`engine/unified-actors.md`](engine/unified-actors.md)** — the flagship. Every
  actor (player included) is ONE body = kinematics + composable ability limbs + a
  capability mask, driven by a **Controller** (Human / Brain / RL) through one input
  seam, observed through one headless `WorldView`. "Player"/"Enemy" are *data*, not
  types or code paths. This is the heart of the engine and the bulk of the live work.
- **[`engine/architecture.md`](engine/architecture.md)** — crate layering (Foundation
  → Runtime Domains → Composition Root), the import/boundary rules, the reusable-engine
  oracle, the Bevy-plugin shape, the **keystone `Player*`/`Actor*` collapse** that
  unblocks crate extraction.
- **[`engine/headless-verification.md`](engine/headless-verification.md)** — the real
  headless sim as the verification substrate; invariants over tuned values; symmetry.
- **[`engine/sprite-renderer.md`](engine/sprite-renderer.md)** — measure-by-default
  sprite metadata.
- **[`engine/visual-quality-profiles.md`](engine/visual-quality-profiles.md)** — one
  global quality profile → a structured runtime/device budget every visual subsystem
  reads; the Android FPS fix (portal-capture throttle + live-switchable texture-res
  variants).
- **[`engine/boss-system.md`](engine/boss-system.md)** — bosses as entity-local actors
  + content.

### `game/` — the first content crate (secondary)

- **[`game/vision.md`](game/vision.md)** — the story spine (an AI whose abilities are
  theorems; uncertainty as the central pressure), the three pillars made concrete, and
  the triaged idea backlog.
- **[`game/bosses.md`](game/bosses.md)** — the Perfect Cell-ular Automaton and the
  "boss = a failed objective function" design language.
- **[`game/characters.md`](game/characters.md)** — the Hall of Characters + the
  character catalog; barks/dialogue.

### [`roadmap.md`](roadmap.md)

**Rewritten 2026-07-03 (fable): the full path to a Unity/Godot-class 2D platformer
engine** — phases P1–P5, the demo-game capability matrix (SMB1/Celeste/Metroid/
Smash/… as expressibility test vectors), the binding design-decision register
(M1–M12), the uncertainty watch-list (U1–U7), and the open questions only Jon can
answer (Q1–Q12). Start there for the big picture; the engine ordering summarized
below is the older, narrower cut of the same arc.

## Roadmap at a glance (elegance order)

The engine is sequenced so each step deletes duplication the next depends on.

1. **Unified actors** — make the player's rich movement pipeline the ONE body
   pipeline; raise enemies onto it; collapse the `Player*`/`Actor*` dual hierarchy.
   *This is the keystone* — it is the prerequisite that unblocks crate extraction
   (goal 1) and de-player-centers the codebase (goals 2 + 4). See
   [`engine/unified-actors.md`](engine/unified-actors.md).
2. **Extract leaf crates** — once the dual hierarchy is collapsed and the player
   stops being a 20-module dependency sink, split the runtime domains into crates
   (compile time + reusability).
3. **Pluginize the domains** — each domain a Bevy plugin with owned vocabulary +
   extension points (the Portal shape, copied).
4. **Get named content out of the foundation** — bosses/abilities/rooms move to the
   content crate behind install-time data seams; prove the reusability oracle by
   extracting one domain clean.
5. **Then the game** — build out story, bosses, and biomes on the finished engine.

---

*Provenance: this plan consolidates the prior `docs/planning/` tree (engine
unification, restructuring blueprint, brain interface, locomotion split, and the
game-design notes), reprioritized by elegance and re-grounded in the stance above.
Anything in the old docs that contradicts this plan is superseded.*
