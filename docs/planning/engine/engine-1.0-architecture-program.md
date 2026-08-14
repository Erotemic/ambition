# Engine 1.0 architecture program

**State:** OPEN — successor program after D73 authority convergence closed on 2026-08-13.

This is the long-horizon architecture program for turning the engine under
Ambition into a credible Unity/Godot-class 2D game engine on top of Bevy.
It is not a second execution queue. The live 72-hour queue chooses slices;
this document explains what those slices are trying to converge toward.

## Product order

**Ambition is the flagship game and the primary product driver.** Engine work
should make Ambition better while making the capability reusable. Acceptance
games exist to force honest reuse and expose missing seams. Super Smash
Siblings may eventually graduate into a first-class game in its own right, but
that does not displace Ambition as the main game.

The governing oracle is stronger than "can the Ambition app do this?":

> Can Ambition use the capability deeply, while another game can opt into the
> same capability through supported composition seams without editing
> Ambition-specific engine code?

## What D73 bought us

D73 closed the most expensive character/body authority split:

- one authored `CharacterDefinition` model supplies intrinsic body facts;
- preparation resolves it before construction;
- NPC/enemy/encounter/summon/match roads converge on ordinary actor bodies;
- the enemy `ArchetypeSpec` / `CharacterRoster` body authority is deleted;
- controller, disposition, placement, ruleset and participant facts stay
  contextual instead of being folded into character identity.

The full migration evidence is archived under
`docs/archive/planning-superseded/2026-08-13/`. Do not reopen D73 merely because
one of its historical documents describes a now-deleted representation.

## Successor programs

The next architecture generation is organized by engine capability rather than
by migration chronology.

### E1 — Ambition-first authoring, world tools and kinematic world objects

Use [`authoring-and-tools.md`](authoring-and-tools.md),
[`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md) and
[`kinematic-world-objects.md`](kinematic-world-objects.md).

LDtk is already Ambition's main spatial editor and moving platforms are already
partially authored there. The next step is not another ad-hoc entity converter;
it is a first-class authoring/compiler path with typed references, strong
validation, useful editor affordances, and a reusable world-domain model.
Moving platforms are the first vertical slice because they exercise authored
identity, dynamic geometry, paths, rollback state, presentation and contact
semantics at once.

### E2 — simulation authority and determinism

Use [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).

Make deterministic behavior emerge from explicit phase and ownership structure,
not from Bevy topology accidents, tuple-packed god systems, mirrored state, or a
generic rollback runtime that imports every domain to census its types.

### E3 — multiplayer, multi-view presentation and world residency

Use [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) and
[`../game/multiplayer.md`](../game/multiplayer.md).

Transport, control assignment, world/room residency and presentation layout are
separate axes. Ambition should ultimately support local couch co-op, online
co-op, mixed local+remote parties, shared-screen play, fixed split-screen and
adaptive shared/split presentation. Participants need not be in the same room
when the game mode permits independent exploration.

### E4 — capability and runtime composition

Use [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

A consumer should be able to ask for the engine capabilities it needs without
silently inheriting bosses, portals, rollback adapters, persistence, debug UI,
audio or Ambition-specific machinery. This program drains hidden composition
roots and turns optionality into an honest dependency property.

### E5 — public SDK 1.0

Use [`public-sdk-1.0.md`](public-sdk-1.0.md).

The public surface should describe game concepts rather than crate topology.
Internal crates remain aggressively refactorable while the engine converges;
external consumers should see stable semantic seams for games, content,
characters, actions, worlds, sessions, views, services and diagnostics.

### E6 — performance and iteration

Use [`performance-and-iteration.md`](performance-and-iteration.md).

Compile graph, runtime cost, asset residency, quality profiles, mobile budgets,
headless throughput and authoring iteration are engine-product concerns. Measure
them as user-facing engine ergonomics rather than as one-off campaign numbers.

## Cross-program rules

1. **Ambition first, reusable second, neither sacrificed.** The flagship game is
   where capabilities earn product value; reusable boundaries keep those
   capabilities from becoming Ambition-only implementation accidents.
2. **One authoritative representation.** A new abstraction is successful when
   it removes or makes unreachable the authority it replaces.
3. **Composition over taxonomy.** Bodies, capabilities, participants, views,
   world objects and services compose; avoid multiplying special engine types
   for modes that differ only in policy.
4. **Authoring is part of the engine product.** A runtime feature that cannot be
   authored, validated, inspected and iterated on without internal knowledge is
   not finished.
5. **Headless and visible hosts consume the same simulation contracts.** Rendering
   and local presentation may vary; authoritative game semantics may not.
6. **Transport is not gameplay ontology.** A local controller and a remote peer
   can own the same participant/control contract.
7. **Views are presentation.** Shared, split and adaptive layouts observe one
   simulation; they do not duplicate worlds merely to render them twice.
8. **Prefer structural guarantees.** Types, ownership, dependency direction,
   preparation and behavioral tests beat permanent source scanners.
9. **Do not pre-generalize without a customer.** Moving platforms may reveal a
   reusable kinematic-world-object model; that does not authorize a universal
   "everything moves" DSL before another object needs the same semantics.

## Program-level exit shape

Engine 1.0 does not mean every conceivable feature exists. It means the common
paths are coherent enough that new games add content and capabilities through
supported seams rather than learning Ambition's migration history.

A credible 1.0 architecture should make these statements unsurprising:

- Ambition is a deep first-class game built on supported engine surfaces.
- A second substantial game can coexist without private engine forks.
- local/online participants and one/many views use orthogonal composition.
- multiple rooms can be resident/simulated when a game needs participants in
  different places.
- LDtk-authored worlds compile into backend-neutral validated world content.
- dynamic world geometry such as moving platforms is ordinary engine data.
- rollback/determinism participation is declared by the owning domain.
- optional capabilities are reflected in dependencies and runtime composition.
- public SDK users need not import historical internal crates.
- profiling/quality/authoring diagnostics support real iteration on desktop and
  mobile.

The live queue should continually promote the highest-value slice from these
programs after verifying that HEAD still contains the named gap.
