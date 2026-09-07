# Render, animation and VFX — Engine 1.0 program

**State:** OPEN — the built-in semantic presentation road is established. Current
work is about completing body-owned drawable scheduling/compositing and adding
new providers only when a real effect requires them.

This is a **live design/acceptance document**, not an investigation log. Dated
render investigations, rejected hypotheses, screenshots, intermediate numeric
tables, and superseded repairs belong in git history once their surviving rule
is captured here.

## Scope

This page owns the boundary between simulation facts and visible presentation:

- sprite animation and authored visual clips;
- one-shot VFX;
- persistent body/world presentation derived from semantic state;
- procedural particles and optional richer providers;
- body-owned drawables such as hit flash and status clocks;
- portal-aware presentation;
- presentation quality policy;
- multiview and confirmed/external-effect constraints.

Asset preparation, quality-tier materialization, residency and first-use hitches
are owned by [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).
Runtime/frame-cost measurement is owned by
[`performance-and-iteration.md`](performance-and-iteration.md).

## Authority contract

### Simulation says what happened

Gameplay/domain code publishes semantic facts or requests. It does not depend on
particle entities, sprite materials, view-local clipping pieces, render layers,
or provider-specific components.

A visible host may omit the presentation consumer without changing simulation
outcome.

### Presentation owns representation, never gameplay state

Presentation may create, update and remove derived entities freely. Rewinding or
changing views may rebuild those entities from current semantic state.

Do not put ordinary render/VFX entities into rollback state merely to make them
survive a rewind.

### One-shot and persistent presentation have different lifetimes

A one-shot request carries enough information to draw one effect.

A persistent visual needs:

1. stable semantic ownership (`PresentationOf` or an equivalent domain fact);
2. reconciliation from semantic state;
3. an explicit presentation lifetime/retraction rule.

Do not force persistent sources into a growing one-shot `ParticleKind` enum.

### Body-owned drawable geometry has one finalization boundary

Every body-owned drawable that portals or other late compositors consume must
publish its **current-frame geometry/appearance before candidate publication**.
The ordering model is:

```text
body-owned drawable producer/update
    -> body-owned drawable geometry finalized
    -> portal candidate publication
    -> portal compositing / source visibility resolution
    -> draw
```

This should be represented by semantic sets/phases, not by each new overlay
ordering itself against concrete foreign systems.

### Portal relationships are per pane and per drawable

A single entity z/visibility value cannot express a body that is near one portal
and far from another.

The current compositor therefore reasons from drawable geometry and creates
per-pane representations. Keep these invariants:

- source z is not mutated to solve portal optics;
- a disjoint drawable is unchanged;
- far-side overlap clips only the overlapping representation;
- transit remains its own split representation;
- two panes may produce different relations for the same drawable;
- body-owned secondary presentation participates independently from the base
  sprite when it declares enough geometry.

`DeclaredFrame` is the reusable road for non-Sprite presentation that can be
reconstructed from a texture/UV/size/anchor/tint description. Do not reintroduce
whole-body scalar hiding for a drawable that can describe its own geometry.

### Quality is presentation policy

Visual quality may alter particle counts, expensive shaders, texture tiers,
trail density and similar visual cost. It must preserve gameplay readability.

FX sheet tier selection now follows the shared quality authority. The narrow prop
loader that explicitly requests full-resolution art is an authored exception,
not an omitted quality road.

### Reference frames are explicit

Gravity-relative, surface-relative, facing-relative and world-fixed effects must
receive or derive the correct semantic frame. Do not reconstruct orientation
from world axes after the authoritative producer has lost that information.

### Providers are demand-driven

The built-in authored-sprite/procedural path is the default.

Do not add a universal VFX backend trait or a third-party particle dependency in
advance. A second provider must first have a concrete effect the current road
cannot express adequately on the supported targets.

## Current implementation state

The current engine already has:

- authored sprite effects and generic action FX;
- built-in procedural particle fallback;
- presentation quality/budget policy;
- explicit body-presentation ownership (`PresentationOf`);
- portal candidate publication and per-pane clipped pieces;
- non-Sprite frame declaration through `DeclaredFrame`;
- hit-flash presentation using the non-Sprite portal road;
- a generic body-clock read model and visible clock presentation;
- shared render-basis logic for sheet-authored player/actor sprites.

The old Mary-O offset investigation is closed at the architecture seam: player
and actor presentation share the same render-basis authority. Do not retain that
investigation diary here; re-run the capture/probe tools if a new visual defect
appears.

## Current work

### R1 — publish a body-owned-drawable-finalized scheduling seam

**Problem:** new body-owned producers can currently exist without an explicit
edge into portal candidate publication. That permits first-frame or one-frame
stale geometry.

**Required change:**

1. publish a semantic render/presentation set owned by the presentation layer;
2. put body-clock creation/update, hit-flash frame update and equivalent
   body-owned geometry writers before it;
3. publish portal candidates after it;
4. ensure deferred spawns are visible to the publisher in the same frame;
5. keep `ambition_demo_smash` and other rulesets from naming concrete
   `sim_view`/render systems for this ordering.

**Acceptance:**

- a body clock created while its fighter is partially far-side is clipped on its
  first visible frame;
- a moving/shrinking clock is classified from this frame's geometry;
- the capability/ruleset foreign-private-ordering ratchet returns to zero.

### R2 — make normal presentation reassert visibility before portal resolution

Presentation owners should write their ordinary desired visibility each frame;
portal resolution should then reassert `Hidden` while a portal hide reason
exists.

Concrete poison: an active hit flash goes far-side and then near-side on the
next frame. It must be visible immediately on the near-side frame, with no
one-frame disappearance caused by the previous frame's portal bookkeeping.

### R3 — persistent emitter reconciliation, only with a real customer

When a semantic source needs a persistent particle/emitter representation:

- simulation owns source existence and parameters;
- presentation reconciles provider entities;
- removal follows source lifetime;
- rewind may rebuild representation from source state.

Do not create this abstraction without a current persistent source.

### R4 — optional rich provider, only after a capability failure

Reopen a provider spike only when a named effect cannot be expressed reasonably
by authored sprite + built-in particle presentation. Compare platform support,
compile/runtime cost, ownership and multiview behavior—not screenshot richness.

### R5 — multiview and confirmed external effects

View-local presentation consumes the same semantic state independently. No
camera's presentation entity may become authority for another view.

Irreversible external effects must use the confirmed-frame/netcode contract;
ordinary VFX stays derived and disposable.

## Asset/authoring contract

Effect identities and clips use the same catalog/preparation principles as other
assets. Runtime presentation consumes published clip/frame metadata rather than
maintaining a second hand-written copy of sheet geometry.

If art and collision disagree, establish which semantic coordinate basis is
wrong before editing both code and assets. Measurement tools should report the
logical frame, rendered frame, anchor and final drawable transform together.

## Acceptance

This program is healthy for Engine 1.0 when:

1. gameplay emits semantic presentation facts without importing renderer
   backends;
2. built-in presentation covers common feedback and remains optional for
   headless/minimal hosts;
3. body-owned Sprite and non-Sprite drawables use one finalization -> portal
   publication -> compositing schedule;
4. portal behavior is correct for far-side, near-side, transit, disjoint and
   multi-pane cases;
5. persistent presentation has semantic ownership and explicit retraction;
6. quality changes visual cost without removing essential gameplay reads;
7. any richer provider is justified by a demonstrated missing capability.

## Standing prohibitions

- no global-z portal bandaid;
- no gameplay dependency on renderer/provider entities;
- no per-effect foreign private-system ordering;
- no hidden second copy of authored sprite geometry;
- no third-party provider added because a category called “particles” exists;
- no dated investigation diary appended below the current answer.

Use git history for the removed portal/Mary-O/provider investigations and the
measurements that led to the current contract.
