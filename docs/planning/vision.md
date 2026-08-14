# THE VISION — Ambition on a Godot/Unity-class 2D engine, the Bevy way

Ambition is the flagship game. The engine exists to make Ambition unusually
expressive, robust and pleasant to build while turning the capabilities Ambition
proves into a reusable 2D game-engine surface on top of Bevy.

The project is therefore pursuing two outcomes at once:

1. **Ambition becomes an excellent game with ambitious mechanics and content.**
2. **The engine underneath it becomes credible competition for Godot/Unity-class
   2D development in architecture, runtime capability, authoring ergonomics,
   deterministic/headless execution and extensibility.**

Neither goal is served by making Ambition a thin demo for an abstract framework.
Ambition is the deepest customer and primary product driver. Reuse matters
because it keeps the flagship from accumulating one-off machinery and makes the
successful engine usable by other games.

## The design oracle

> Can Ambition use this capability deeply while another substantial game can opt
> into the same capability through supported composition seams without editing
> Ambition-specific engine code?

This judges the end state rather than each commit. Named game policy and content
belong to the game/provider. Reusable world, simulation, input, combat,
presentation, authoring and service capability belongs to the engine.

## Product pillars

### 1. Ambition — flagship game

*Every upgrade a theorem, every boss a failed objective function, every biome a
mathematical world model.* Ambition's world, characters, story and unusual
mechanics are not delayed until an abstract engine is complete. They drive the
engine programs by presenting real product requirements.

Ambition should eventually support local and online multiplayer, including
shared-screen, fixed split-screen and adaptive share/split play, with participants
able to occupy different rooms when the rules allow independent exploration.
See [`game/multiplayer.md`](game/multiplayer.md).

### 2. Engine 1.0

The engine is a set of coherent crates/plugins and public semantic APIs rather
than an exposed historical crate graph. The post-D73 successor program is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).

A credible 1.0 has:

- one body/construction model for controlled bodies, NPCs, bosses, summons and
  match fighters;
- explicit simulation authority and deterministic phase structure;
- local/remote participants independent of control assignment and presentation;
- indexed local views so one simulation can render shared or split presentation;
- multi-room residency sufficient for real co-op separation;
- strong LDtk/world authoring, preparation diagnostics and intent-level tools;
- ordinary kinematic/dynamic world geometry such as moving platforms;
- honest optional capabilities and narrow runtime composition;
- a public SDK expressed in game concepts rather than internal topology;
- headless execution as a supported product surface;
- desktop/mobile quality, asset residency and iteration budgets that are measured
  as engine ergonomics.

### 3. Serious secondary games and acceptance customers

Sanic, Super Mary-O, Super Smash Siblings, Hollow Lite and TwinTrack force the
engine to prove capabilities Ambition alone might not stress soon enough. They
are persistent customers rather than disposable test fixtures.

A customer may later **graduate into a first-class game**. Super Smash Siblings
is an obvious candidate if it becomes compelling enough. Graduation increases
product investment; it does not move Ambition out of the flagship role or create
private engine semantics.

TwinTrack is especially important for multi-view work because two participants
can require different observer/reference-frame presentations over one shared
simulation.

### 4. Intelligence and headless simulation

Headless/RL-first simulation is not merely a testing convenience. Fighter AI,
boss authoring/evaluation, deterministic replays and future training hooks should
consume the same authoritative simulation used by visible hosts.

## The authoring position

We do not need to win by building another monolithic editor executable. Bevy and
Rust let the runtime remain composable while best-in-class external tools author
validated content.

**LDtk is Ambition's preferred spatial editor today and should receive serious
investment.** If a real Ambition room needs a concept LDtk cannot express
pleasantly, improve the LDtk schema/tooling/compiler rather than normalizing a
parallel hard-coded world path.

The world IR remains backend-neutral so other spatial importers are possible in
the future. That does not make today's LDtk experience second-class.

Character/content authoring may be RON, Rust values, generated data, SVG/sprite
metadata or other provider-owned source formats where each is appropriate. The
important property is declarative/transactional composition into validated
prepared content, not one universal syntax.

## Acceptance/customer matrix

| Customer | Primary architectural pressure |
|---|---|
| **Ambition** | deep world/content authoring, portals, possession, persistence, multiplayer, multi-room residency, adaptive split-screen, long-term ergonomics |
| **Super Smash Siblings** | N participants, body-generic combat, fighter AI, match rules, stage authoring; possible future first-class game |
| **TwinTrack** | independent observer/reference-frame views, split-screen and unusual presentation derived from one simulation |
| **Sanic** | high-speed movement, collision, momentum and host/provider composition |
| **Super Mary-O** | classic platforming, level authoring, equipment/powerups and sequencing |
| **Hollow Lite** | exploration/combat, boss/encounter authoring and quality evaluation |

Additional small games should be added when they expose a capability family the
existing customers do not adequately pressure. We are not optimizing for a high
demo count.

## What "done enough for 1.0" means

The common path should be coherent enough that a competent developer can build a
substantially different 2D game without learning Ambition's migration history or
editing Ambition-specific machinery.

That means, among other things:

- the actor monolith and shared high-fan-in foundations no longer function as
  accidental composition roots;
- rollback participation is declared by owning domains rather than censused by a
  generic runtime;
- simulation behavior does not change because unrelated Bevy systems perturb
  implicit schedule topology;
- a game can select capabilities without silently inheriting unrelated domains;
- participant/control/view/world-residency concepts support solo through mixed
  local+network multiplayer;
- split-screen is a normal presentation configuration, not a second simulation;
- moving platforms and future kinematic world objects are authored and validated
  like ordinary world content;
- public APIs and diagnostics let an external game author work in game concepts;
- Ambition itself uses those same supported surfaces rather than privileged
  internal shortcuts.

## Execution model

[`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) is intentionally
self-replenishing and owns immediate work order. [`tracks.md`](tracks.md) is the
standing reservoir. Focused plans own technical design. Completed migration
narratives move to `docs/archive` so future agents see the current architecture
first.

The roadmap is [`roadmap.md`](roadmap.md). Explicit maintainer rulings remain in
[`maintainer-decisions.md`](maintainer-decisions.md).
