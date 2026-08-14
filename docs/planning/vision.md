# THE VISION — systemic 2D worlds on a Godot/Unity-class Bevy engine

Ambition is the flagship game. The engine exists to make Ambition unusually
expressive, robust and pleasant to build while turning the capabilities Ambition
proves into reusable Bevy-native engine surfaces.

The distinguishing product thesis is stronger than "a large Metroidvania":

> **Build a 2D platforming world with RPG-scale systemic depth — persistent
> actors and objects, embodied capability progression, open-world traversal,
> reactive characters, multiplayer residency and LLM-native authoring.**

The world should have meaningful simulation truth before story is required to
make it feel alive.

## Product order

1. **Ambition becomes an excellent, ambitious open-world 2D game.**
2. **The engine becomes credible competition for Godot/Unity-class 2D
   development through reusable Bevy plugins/crates, deterministic/headless
   simulation, semantic authoring and a strong SDK.**
3. **Secondary games stress capabilities Ambition does not yet pressure enough.**

The design oracle remains:

> Can Ambition use this deeply while another substantial game can opt into the
> same capability through supported engine/provider/plugin seams without editing
> Ambition-specific engine code?

## World-first game thesis

The controlled robot should be able to explore a substantial connected world,
change bodies, acquire capabilities/items, alter physical world state, interact
with persistent and spawned actors, leave objects behind, save/reload and return
to coherent consequences.

Progression should usually answer "why can I go there now?" with a simulation
fact: body capability, physical property, item/tool, world mechanism, danger or
social cooperation. Explicit story-stage gates remain available when sequence
itself is the design, but they are not the default world model.

The simulation determines what is true. AI/dialogue can decide what characters
believe, say and try to do about that truth.

See [`game/open-world-roadmap.md`](game/open-world-roadmap.md),
[`game/systemic-progression.md`](game/systemic-progression.md), and
[`game/reactive-characters-and-dialogue.md`](game/reactive-characters-and-dialogue.md).

## Engine 1.0

The post-D73 program is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
The immediate priority is the controlled-character actor kernel, followed by
world/LDtk mechanics, persistent-world semantics and multiplayer pressure.

A credible 1.0 has:

- one ordinary actor/body/control model for controlled bodies, NPCs, bosses,
  summons and fighters;
- explicit deterministic simulation authority;
- persistent-world residency independent of what is visible;
- definition/instance/provenance/lifetime semantics for actors/items/world data;
- explicit item custody/accounting and capability-driven progression;
- platformer reachability/navigation that understands bodies and world mechanics;
- local/remote participants independent of control assignment/view layout;
- shared, split and adaptive presentation over one simulation;
- agent-native authoring, preparation diagnostics and introspection;
- honest optional capabilities and semantic public APIs;
- reusable domain crates/plugins that can mature into Bevy ecosystem components.

## Agent-native authoring

We do not need to win by cloning a monolithic visual editor first. The existing
sprite/music/SFX/LDtk toolchains already make agent-operated content production a
strength. Engine work should unify discovery, inspection, semantic mutation,
validation, provenance and concise review artifacts across these surfaces.

Human visual editors remain useful optional frontends when manual editing matters.

## Secondary customers

Super Smash Siblings, TwinTrack, Sanic, Super Mary-O and Hollow Lite remain
serious customers. Smash may eventually become a first-class game, but Ambition
remains the flagship.

TwinTrack is particularly important for independent observer/reference-frame
views. Smash stresses N-participant combat. Sanic stresses high-speed movement.
Mary-O stresses classic level authoring. Hollow Lite stresses encounters/bosses.

## Bevy-native decomposition

The engine should increasingly feel like a set of coherent Bevy domains rather
than one historical workspace graph. Registration moves with the domain plugin;
composition roots compose plugins rather than importing private systems.

Use [`engine/bevy-plugin-and-crate-strategy.md`](engine/bevy-plugin-and-crate-strategy.md).
Publish reusable pieces only when they have genuinely general APIs and another
Bevy game can consume them through ordinary plugin/system composition.

## Execution

[`queue-72h-2026-08-08.md`](queue-72h-2026-08-08.md) remains intentionally
self-replenishing. [`tracks.md`](tracks.md) is the standing reservoir. Focused
plans own design and explicitly record ambiguities rather than making future
agents guess which details were settled.
