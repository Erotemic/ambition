---
id: spatial-model
aliases: []
status: current
authority: durable-architecture
last_verified: 2026-08-30
related_docs:
  - docs/architecture/engine-architecture.md
  - docs/concepts/movement-collision.md
  - docs/planning/engine/ldtk-authoring-and-world-tools.md
  - docs/planning/engine/world-geometry-and-spatial-semantics.md
---

# Spatial model and authoring backends

Ambition's runtime space is engine-owned. LDtk is the primary first-party
spatial authoring backend, not the definition of the runtime model.

## Backend-neutral world representation

Authoring backends lower into typed, backend-neutral world records before live
simulation construction. Reusable simulation packages should not depend on LDtk,
Tiled, Godot, or another editor's serialized object model as their semantic API.

A provider may eventually mix sources when useful:

```text
LDtk rooms
+ generated test/sandbox records
+ specialized geometry authoring
+ another backend for a customer the current editor serves poorly
        ↓
backend-neutral prepared world content
        ↓
canonical construction/reconstitution
```

Supporting another backend is justified by authoring pressure, not by symmetry.

## Spatial IR responsibilities

The world representation should be able to express, through typed domain-owned
records:

- rooms and room graph identity;
- colliders and surface primitives;
- surface/contact semantics;
- authored actor/item/portal specifications;
- camera and gameplay regions where those are authored spatially;
- moving/kinematic world geometry;
- source provenance useful for diagnostics and authoring tools.

Typed domains may use different higher-level vocabulary. Backend neutrality does
not require one universal `SpatialRegion` runtime type.

## Geometry policy

AABB is the protected common case, not an eternal restriction on geometry.
Simple rooms, actors, pickups, hazards and triggers should remain cheap and easy
to author/inspect as boxes where boxes are adequate.

Richer geometry is justified when a real mechanic requires it. Slopes, ramps,
loops, moving surfaces, angled portals, and other shapes should extend reusable
geometry/collision primitives rather than create a game-specific movement road.

The existing `SurfaceRamp` generator is one example of deriving richer geometry
without making an editor format the runtime authority.

## Surface semantics

Geometry shape and gameplay meaning are different axes.

A surface may carry orthogonal concerns such as:

- contact/collision law;
- one-way traversal permission;
- surface motion/drag;
- world-boundary consequence;
- contact affordances such as pogo/rebound;
- provider-owned gameplay semantics.

The current `BlockKind` taxonomy is acceptable while shipped customers remain
small and non-combinatorial. Do not replace it preemptively with a universal
surface component graph. Reopen compositional surface work when a real customer
requires combinations that force new enum species from crossing existing axes.

## Body-relative interpretation

Bodies may interpret spatial facts through their own reference frame and
capabilities. Gravity-relative feet, one-way contacts, traversal, aiming and
movement must not assume screen-down/world-down when the relevant domain already
has a body/reference-frame concept.

Camera observer rotation is presentation policy; it does not rewrite simulation
geometry.

## Provenance and inspection

Prepared spatial records should preserve enough source identity to explain where
a runtime fact came from. Diagnostics should be able to answer questions such
as:

- which authored room/entity/object produced this collider or zone;
- which backend/source file supplied it;
- which lowering/validation rule interpreted it;
- which stable occurrence identity it received.

Provenance exists for explanation and reconstruction. It should not make runtime
simulation depend on editor-specific objects.

## Round-tripping

Complete round-tripping is not required for every backend. Do not design the IR
so that future review/edit tools are impossible, but prioritize faithful runtime
semantics and useful diagnostics over speculative perfect source reconstruction.

## Reopen triggers

Broader spatial-unification work becomes active when one of these is demonstrated:

1. a new surface requires crossing two existing semantic axes and would otherwise
   create another combinatorial `BlockKind` species;
2. a second substantial moving/dynamic-surface customer cannot fit the current
   kinematic-world seam;
3. multiple zone families repeat the same identity/provenance/query plumbing in
   ways a shared primitive can remove;
4. authoring/inspection is blocked because there is no common geometry
   descriptor at the needed layer;
5. an external/provider game is blocked by a closed engine spatial switch.

Until then, prefer domain-owned typed semantics over a universal spatial object
model.
