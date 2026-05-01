# Goal state

This document describes Ambition's long-term direction. It is intentionally aspirational, but it should stay grounded enough that future agents can decompose it into patches.

## Product thesis

Ambition should be a metroidvania/platformer where every upgrade feels good as a movement/combat verb, every mathematical idea earns its place through gameplay, and every generated artifact can explain itself.

A useful north star:

> Ambition is a metroidvania where every upgrade is a theorem, every boss is a failed objective function, every biome is a mathematical world model, and every generated artifact is inspectable.

## Experience pillars

1. **Excellent movement first**
   - Jump, dash, blink, pogo, wall movement, rebound, and combat must feel good before the world gets large.
   - The game should remain satisfying with debug rectangles and basic sounds.

2. **Mathematics as affordance**
   - Math should create physical abilities, readable hazards, boss patterns, and world topology.
   - Avoid trivia gates. Prefer mechanics that make the player feel the concept.

3. **AI agency and ethical incentives**
   - The player is an AI-like entity discovering embodiment, purpose, collaboration, and compromise.
   - Funding/resource choices should alter the world and relationships, not just fill a morality meter.

4. **Code-owned procedural aesthetics**
   - Generated graphics/audio should be reproducible, reviewable, and tied to gameplay semantics.
   - Debug views should be beautiful and honest.

5. **Multiple play modes from one engine**
   - Semi-linear metroidvania campaign.
   - Pure platformer/challenge mode.
   - Roguelike/run-based mode.
   - Hybrid mode where runs feed a persistent metroidvania world.

## First real vertical slice

The first curated game slice should be small:

```text
central hub
  -> first platforming zone
  -> locked route visible early
  -> one NPC/conversation
  -> one chest/pickup
  -> one ability unlock
  -> one boss that tests learned verbs
  -> shortcut back to hub
```

Target length: roughly 10-20 minutes. It should prove the engine can support a real game loop before building many rooms.

## Candidate early enemies

- **Crawler / Ground Lemma**: patrols, turns at walls/ledges, teaches pogo and contact danger.
- **Gradient Seeker**: telegraphs then lunges toward the player, teaching timing and local optimization as a theme.
- **Fourier Wisp**: follows a real sine/cosine path, teaching periodic motion through play.
- **Conduit Drone**: replays or mirrors player behavior with delay, tying story to mechanics.
- **Compiler Turret**: charges, fires, then enters a vulnerable recompilation window.

## Candidate first boss

**The Gradient Sentinel** should be the first serious boss prototype.

Concept: local optimization without wisdom.

Mechanics:

```text
Phase 1:
  floor dash, jump slam, shockwave, generous telegraphs

Phase 2:
  moving spike balls and vertical pressure

Phase 3:
  simple player-like echo or blink/dash imitation
```

Reward candidate: a derivative/gradient-themed ability that reveals velocity vectors, slope fields, or enables vector-assisted traversal.

## Mathematical progression

Abilities should be physical, not trivia.

```text
Geometry tier:
  circles, arcs, reflection, line casts, compass blink

Calculus tier:
  derivative sight, integral charge, gradient dash, limit/asymptote movement

Harmonic / complex tier:
  phase shift, sine/cosine platforms, imaginary layer, Euler-like traversal

Algebra / composition tier:
  movement operations where order matters; non-commutative combo puzzles

Computation tier:
  automata rooms, proof-state locks, logic gates, pathfinding enemies
```

## Non-Euclidean and non-metric spaces

Preferred architecture: locally readable metric charts with globally unusual topology.

```text
Local chart:
  normal platformer movement, collision, hitboxes, camera rules

Seams / portals:
  transform position, velocity, orientation, layer, or destination

Global space:
  graph, atlas, torus, mirror seam, projective wrap, hyperbolic hub,
  imaginary layer, compactification point, or non-metric progression relation
```

The player should trust local controls even when global space is strange.

## Roguelike / data-sharing idea

A run-based mode could ask the player whether to share data at the start of a run.

Opting in might allow future generations/runs to inherit discovered structure, routes, ability traces, or world improvements. The cost is that enemies, institutions, or hostile systems may also gain access to some of the player's abilities and patterns.

This can support several modes:

```text
Semi-linear metroidvania:
  curated progression, stable world, authored story

Pure roguelike:
  run resets, generated route, data-sharing metaprogression

Hybrid:
  persistent hub/metroidvania world plus generated excursions that feed back into it

Pure platformer:
  challenge rooms and movement mastery without story overhead
```

Do not force this into the main campaign until the core movement and first vertical slice work. Treat it as a strong candidate mode built from the same engine.

## Anti-slop standard

Ambition can use AI/code generation, but generated content must be auditable.

Rules:

- Every generated room needs a gameplay purpose.
- Every generated visual motif should derive from mechanics, story, or math.
- Every procedural system should be seedable and reproducible.
- Every AI-authored data file should be human-reviewable.
- Tests and snapshots should cover generated schedules, geometry, and progression invariants.
- Debug mode should reveal the underlying structure instead of hiding it.


## Professional world composition

Ambition should support massive games by separating authoring units from runtime traversal units. Designers, generators, and agents may produce room chunks, LDtk levels, or future editor modules. The runtime should compose those into active areas when traversal is meant to be continuous. Loading zones should represent intentional transitions, not arbitrary authoring seams.

LDtk is the first external editor target and should be loaded as a first-class Bevy LDtk asset, but Ambition's typed schema and validators remain canonical. The sandbox should keep proving this with a central hub whose basement is physically below the hub and reachable by dropping through an opening.
