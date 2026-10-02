# Reusable authored world composition — Engine 1.0 design program

**State:** INCUBATING — do not implement a universal prefab system yet.

**Hold evidence.** Both conditions for the hold are true:

- No abstraction has emerged. No `*Assembly`, `*Prefab`, `*Composition` or
  `*Motif` type exists for authored world structure. `ShellComposition`
  (`crates/ambition_platformer2d_provider/src/composition.rs`) is shell
  composition, not world composition.
- The authored unit is one placement. `PlacementKind`
  (`crates/ambition_entity_catalog/src/placements.rs`) has six flat variants
  (`Hazard`, `Interactable`, `Pickup`, `Chest`, `Breakable`, `Portal`), with no
  composite, no cross-placement reference and no grouping.

Each predicted pressure (portal mechanisms, moving machinery, shrines,
encounter assemblies, environmental hazards) has its own implementation. To
see the pressure, count files with `grep -rl <TERM> --include=*.rs crates game`
for `Portal`, `KinematicPath\|MovingPlatform`, `Shrine`,
`EncounterSpec\|EncounterId\|EncounterDefinition` and
`HazardSpec\|HazardRespawn`. A file count shows pressure, not a design.

## Goal

Discover the reusable composition abstraction, if any, for authored structures
larger than one placement and different from a character definition.

Likely Ambition pressures include encounter assemblies, moving machinery,
shrines, portal mechanisms, environmental hazards and repeated room motifs.

## Direction

Learn from `CharacterDefinition`: authored composition should be reusable data,
validated/prepared before runtime, and separable from placement/session policy.

But do **not** assume the correct answer is a Unity-style prefab clone, a Godot
scene clone, or one recursive `UniversalDefinition` enum.

A real abstraction must emerge from at least two materially different Ambition
uses.

## An emerging direction (recorded, not adopted)

Once prepared authored rules exist
([`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)),
a reusable composition plausibly contains:

- prepared component/entity data;
- internal semantic references;
- exposed parameters;
- optional references to prepared deterministic rule/orchestration programs.

For example, a reusable elevator composition might eventually be:

```text
lift
switch A
switch B
door
reference bindings
prepared power/control rules
```

That would give scene-like compositional **power** without adopting a universal
Node tree and without embedding arbitrary Rust plugins inside authored
compositions — which is the shape this program has been looking for.

This does not raise the priority of composition or lower its evidence
threshold. Do not move composition ahead of its evidence. The
two-materially-different-uses rule stands.

## Questions an eventual model must answer

- how child authored identities are defined;
- how a placement parameterizes a reusable composition;
- how references between children remain stable;
- how preparation/lowering works across authoring backends;
- how persistent instance state relates to the authored template;
- how nested composition and overrides work, if they exist at all;
- how an agent semantically inspects and edits the composition.

## Candidate crate / Bevy shape

Do not create a crate until the model is proven. If a generic prepared-composition
core emerges, it should be independent of LDtk and Ambition content and install
runtime behavior through domain plugins rather than a universal spawn dispatcher.

## Open design questions — deliberately unresolved

Almost all details are intentionally unresolved. Specifically:

- tree/hierarchy versus flat component bundle;
- inheritance versus explicit composition;
- nested instances and override semantics;
- whether runtime ECS hierarchy should mirror authored hierarchy;
- whether compositions can contain behavior plugins or only data;
- how much of this is already solved adequately by LDtk entity/level structure
  plus prepared provider data.

Do not harden any of these until real content hurts.

## Preparation bridge and repeated-instance proof

A3 in the [frontier](actor-monolith-work-frontier.md) keeps provider-neutral world
input separate from actor/catalog-specific construction lowering. A reusable room
composition should not import prepared character sheets merely to expose spatial
placements. Typed domain lowering remains explicit at the integration boundary.

Two live instances of one authored room are built and isolated
([open-world residency](open-world-runtime-and-residency.md)): content identity is
shared while live entities, mutable overlays and occurrence histories are scoped
by live room. A composition must keep that property. Different authored room IDs
do not prove repeated-instance safety. Do not require all identity types to share
one new namespace.
