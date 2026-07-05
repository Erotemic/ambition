# Design Vision: Authoring-Backend Agnostic Space

> **Status: Jon's design position (2026-07-05), captured verbatim.** This is the
> manifesto that extends the fable reviews (2026-07-02 / 2026-07-04) with the
> spatial-model direction: LDtk becomes one *backend* lowering into an
> engine-owned canonical spatial model; AABB stays the protected fast path;
> momentum-based locomotion (the "Sanic" demo) is the stress test. The execution
> plan derived from this doc is
> [`../../reviews/fable-review-2026-07-05.md`](../../reviews/fable-review-2026-07-05.md);
> this doc should eventually spawn one or more ADRs (it revises ADR 0017's
> "LDtk is for space" shorthand) but does not force all implementation
> immediately. Like ADR 0020: do not deviate without raising an explicit
> challenge Jon accepts.

## Premise

The fable review already established the world seam, content-out-of-core
direction, `RoomGeometry`, installed world manifests, content-registered LDtk
conversion, and the next `ambition_world`-style carve. This document starts from
that foundation.

The new claim is narrower and sharper:

```text
LDtk is the current map backend.
LDtk is not the engine ontology.
```

Ambition should design toward an editor-independent model of space. LDtk
remains the only backend we should actively develop for a while, but the engine
should stop treating LDtk as first-class truth. LDtk should be one source of
spatial data that lowers into Ambition's own world, map, and geometry
vocabulary.

The engine ontology must be elegant enough that editor backends feel like
translations into a coherent spatial model, not piles of format-specific
exceptions.

## Revised authoring principle

The old shorthand was useful but too narrow:

```text
Rust is for behavior.
RON is for content.
LDtk is for space.
```

The revised principle is:

```text
Rust is for behavior, systems, import logic, and explicit transformations.
RON is the preferred format for hand-authored structured data.
Map authoring backends are sources of spatial content.
Ambition owns the canonical spatial model.
```

LDtk remains the first and default source of space for current first-party
content. It should not define what space is.

## Editor formats are backends

A future game should be able to choose its map authoring backend the same way
it chooses other content infrastructure.

The long-term shape is something like:

```text
ambition_ldtk_map
ambition_tiled_map
ambition_godot_map
```

Only the LDtk backend needs real implementation for now. The others can remain
placeholders in the vision until content demands them.

These backend crates should be optional. A game crate chooses which ones it
brings in. Engine core should not depend on LDtk, Tiled, Godot, or any other
editor format as a privileged dependency.

A single game should be able to use multiple spatial sources if that becomes
useful:

```text
LDtk for ordinary rooms
generated IR for sandbox tests
custom authored surfaces for geometry-heavy rooms
another backend for path-heavy content
```

The backend choice is a game/content decision, not an engine identity decision.

## Ambition needs a canonical spatial IR

The existing world seam should evolve toward a canonical editor-independent
representation. The exact crate and type names should fall out of
implementation, but something like a `world_core`, `map_core`, or `world_ir`
layer makes sense if it fits naturally.

That representation should be capable of expressing:

```text
rooms
room graphs
colliders
surface primitives
surface semantics
spawn specifications
portal specifications
camera zones
moving surfaces
source metadata
```

Source metadata matters because authored content needs to be debuggable:

```text
this came from an LDtk entity
this came from a Tiled object
this came from a Godot node
this came from generated content
this came from a hard-coded sandbox room
```

Round-tripping does not need to be complete in the first pass. It can grow as
tests and tools require it. But the spatial IR should not make round-tripping
impossible by design.

## Backend limitations are authoring limitations

An editor backend may make some geometry easy and other geometry awkward. That
should not limit the engine's runtime model.

LDtk is good for rooms, tile layers, entity placement, and the current
first-party workflow. Tiled may be better for polygon and polyline-heavy maps.
Godot may be better for path-heavy or scene-graph-heavy authoring. A future
Ambition-native tool may be better for specialized surface graphs, moving
surfaces, loops, and portal-aware geometry.

Those are ergonomics differences.

The runtime question is different:

```text
Can Ambition represent and simulate this geometry elegantly?
```

That answer should not depend on whether the current editor makes the geometry
pleasant to draw.

A backend may eventually warn that a map is valid in Ambition's spatial IR but
awkward, lossy, or impossible to round-trip through that backend. That is a
useful distinction. It keeps the engine model from collapsing into the least
common denominator of the active editor.

## AABB is protected, not absolute

AABB collision should remain fast, common, and first-class.

Most rooms, simple actors, pickups, hazards, triggers, camera bounds,
rectangular platforms, and many moving objects should keep the AABB fast path.
Supporting richer geometry must not delete, slow down, or conceptually shame
the rectangular path.

The principle is:

```text
AABB is a fast and important primitive.
AABB is not the ontology of space.
```

The engine should grow toward a geometry/contact layer where AABBs coexist with
richer surface representations. The first richer primitive family is likely to
be segment chains or surface paths, but there is still uncertainty here. The
manifesto should not over-specify the final primitive set before the geometry
kernel earns it.

The design requirement is preservation:

```text
Existing AABB-friendly worlds and bodies must remain efficient.
Richer geometry is opt-in where content and body behavior require it.
```

## Momentum-based locomotion is a stress test for the spatial model

The Sanic demo should be treated as a focused stress test for Ambition's
spatial elegance, not as a new engine fork.

The formal feature area should be described neutrally as something like:

```text
momentum-based locomotion
surface-momentum locomotion
surface-based locomotion
```

"Sanic" can remain the internal demo/character name.

The important architectural pressure is that a momentum-based body wants to
interact with slopes, loops, curved-feeling surfaces, moving surfaces, and
portals through coherent contact frames. That should exercise the same spatial
model used by the rest of the engine.

The demo should prove that Ambition can express a body with radically different
movement feel inside the same world model as a crisp knight-like body.

## Different bodies must coexist in one world

Ambition should support multiple embodied movement identities in the same
simulation.

A knight-like body and a momentum-based body should be able to occupy, fight,
collide, trigger hazards, use portals, and interact in the same world while
each retains its own game-feel physics.

This directly supports the crossover/fighting/sandbox use case. The engine
should allow bodies from different platformer traditions to coexist without
turning the world into separate sub-engines.

The exact architecture split among controller, controlled body, capabilities,
intent, and locomotion policy remains open. There may be a cleaner model than
immediately coupling everything through a controller-intent abstraction.

The stable principle is:

```text
Movement identity travels with the possessed body.
```

When control authority moves into a body, that body's capabilities and movement
identity come with it. If the controlled body is Sanic, control of that body
means participating through Sanic's motion. If the controlled body is
knight-like, control of that body means participating through that body's
motion.

Capabilities belong to bodies. Control authority drives bodies; it does not
redefine what those bodies physically are.

## Surface interpretation may be body-relative

A future geometry/contact model should leave room for different bodies to
interpret the same surface differently.

A momentum-based body may treat a surface as something it can ride along. A
knight-like body may classify the same contact as a floor, wall, ceiling,
blocker, or non-walkable surface. There may be an even more elegant formulation
than making this a per-body policy table, so the document should keep this as a
design pressure rather than a fully settled API.

The engine goal is clear:

```text
The world exposes coherent contact information.
Bodies decide what that contact means for their capabilities and movement identity.
```

The implementation shape is still open.

## Slopes, loops, angled portals, and moving-platform portals are one pressure

These should not become independent hacks.

A slope, a loop, an angled portal, and a portal mounted on a moving platform
all force the engine to care about frames, contact normals, tangents, support,
transforms, and relative motion.

This is part of Ambition's broader relativity design force. The engine should
avoid global-axis assumptions when the problem is really about relationships
between bodies, surfaces, portals, platforms, and frames of reference.

The spatial model should be able to ask questions like:

```text
What frame is this surface in?
What frame is this body moving through?
What happens when a contact surface moves?
What happens when a portal endpoint moves?
What does support mean after a transform?
What is the body-relative meaning of this contact?
```

This does not require solving every portal and surface case immediately. It
does require designing the model so those cases feel like natural extensions
rather than violations.

## The near-term Sanic demo should stay small

The first Sanic work should be a focused sandbox demonstration.

It can be:

```text
one playable Sanic body
one sandbox room
slope traversal
loop traversal
AABB and non-AABB coexistence
debug visualization of surfaces, normals, tangents, support, and bounds
```

It may be authored with LDtk entities if convenient. It may also use hard-coded
demo geometry emitted from the canonical IR in the app or content layer.
Hard-coded sandbox geometry is acceptable when it proves a reusable engine
concept.

Hard-coded demo geometry must not enter engine core.

The demo is not a commitment to build a full momentum-platformer game
immediately. It is a proof of possibility and elegance. It should demonstrate
that the engine can host this kind of body without compromising the AABB path,
the current content path, or the broader architecture.

## LDtk remains the practical first backend

For the near term, LDtk should continue to carry current first-party spatial
content.

For the Sanic demo, LDtk entities can represent simple slopes, loop markers, or
semantic surface objects over visual tiles. That is enough to prove the runtime
direction.

This should be understood as a practical backend strategy, not a statement that
LDtk is the ideal tool for every future spatial problem.

The correct relationship is:

```text
LDtk authoring
  -> LDtk backend/importer
  -> canonical Ambition spatial IR
  -> runtime geometry/contact/locomotion
```

not:

```text
LDtk authoring
  -> engine ontology
```

## Validation should be pragmatic

Validation matters most where bad authored geometry would masquerade as engine
bugs.

Surface-heavy content especially needs debug overlays and targeted diagnostics.
Inverted normals, discontinuous joins, invalid support sides, bad moving-frame
assumptions, and portal/surface mismatches can all look like physics failures
if tooling does not expose them.

But validation should not become an expensive Rust cathedral by default.

Validation strategy should be chosen based on:

```text
line count
compile time
load time
developer time
runtime cost
test value
debug value
```

Some validation may live in backend importers. Some may live on the canonical
IR. Some may live in external tools. The project should implement the
validation that pays for itself, and let the rest emerge as tests and demos
require it.

## Design force: elegance

The canonical spatial model should be elegant enough to become one of
Ambition's major design stars.

Elegance here means:

```text
few concepts
strong concepts
composable concepts
minimal editor leakage
minimal special-case gameplay leakage
clear frame relationships
clear contact vocabulary
clear backend boundaries
```

The right model should make AABB rooms, LDtk-authored platformer spaces,
momentum-based surface traversal, angled portals, moving-platform portals,
generated maps, and future editor backends feel like variations of the same
underlying language.

The wrong model would accrete one-off concepts for every editor and every
character type.

The desired end state is:

```text
one elegant runtime ontology of space
many possible authoring frontends
many embodied movement identities
one shared world
```

## Direction of travel

This manifesto should eventually spawn one or more ADRs, but it should not
force all implementation immediately.

The likely path is:

```text
1. Keep LDtk as the only real backend for now.
2. Let the Sanic sandbox demo use LDtk entities or hard-coded content-layer IR.
3. Clarify the canonical spatial IR as the world carve matures.
4. Preserve AABB as the default fast path.
5. Add richer surface primitives only when the demo and engine pressure justify them.
6. Treat Tiled/Godot/custom backends as optional future plugin paths.
7. Implement additional backends only when content needs them.
```

This is additive to the existing fable-review direction. The fable review moves
named content and world seams out of core. This vision generalizes the result:
once space is no longer hardcoded in core, no single editor should become the
new hidden core.

## Summary

Ambition should remain LDtk-backed in practice while becoming
authoring-backend agnostic in architecture.

LDtk is the current map backend. It is not the definition of space.

The engine should own an elegant spatial model that can represent AABBs, richer
surfaces, portals, moving frames, generated rooms, and multiple embodied
movement identities. Editor backends should lower into that model. Bodies
should participate in that model according to their capabilities. AABB should
remain protected and fast. Momentum-based locomotion should stress the model
without forking the engine.

The destination is not "support every editor."

The destination is:

```text
an elegant runtime language of space
that many authoring tools can speak.
```
