# Two presentations of one ECS content model sharing a despawn-by-marker

**Date:** 2026-06-07
**Tags:** `bevy-ecs`, `architecture-seam`, `marker-components`, `despawn-query`, `bevy-0.18`, `menu-refactor`
**Prompt level:** A (pre-error operation)

---

## Background

Ambition's pause/inventory menu is one backend-agnostic content model
(`MenuPageModel<PageId, Action>`) rendered by TWO interchangeable presentations,
swapped with a runtime `InventoryUiBackend` enum:

- **Kaleidoscope** — a 3D bevy_lunex cube. Its renderer spawns one entity per
  page tagged with the marker `AmbitionMenuPage`, and a `rebuild_cube_faces`
  system keeps them in sync with a shared `ActiveMenuPages` resource:

  ```rust
  fn rebuild_cube_faces(
      mut commands: Commands,
      pages: Res<ActiveMenuPages<PageId, Action>>,
      faces: Query<Entity, With<AmbitionMenuPage>>,
      // ...
  ) {
      if !pages.is_changed() { return; }
      for e in &faces { commands.entity(e).despawn(); }   // reap ALL AmbitionMenuPage
      // ...respawn one face per page...
  }
  ```

- **Grid (new)** — a flat `bevy_ui` renderer of the SAME `MenuPageModel`. To make
  it work, the host UN-GATES the cube's `republish_kaleidoscope_pages` so it runs
  for *both* backends and both consume the same `ActiveMenuPages`. The new
  renderer spawns a root → panel → tab bar → **body** → content nodes, and (to
  "mirror the cube") the agent tagged the body with `AmbitionMenuPage { id, .. }`
  plus its own `BevyUiMenuBody` marker.

The interactive controls in BOTH renderers share `AmbitionMenuControl` and
`MenuVisualState` (genuinely shared data the host's picking/nav reads).

---

## Observable symptoms

In Grid mode the panel + tab bar render and persist, but the **body is empty**;
the (provably built — logs show `rendered=System nodes=15`) content only *flashes
in for one frame* when navigating. At idle the republish runs exactly once (not
per-frame), so it is not an every-frame rebuild. Picking, nav, click-dispatch all
work; the content simply disappears a frame after it spawns.

---

## The invariant

A `for e in Query<Entity, With<SharedMarker>> { despawn }` in one backend will
**silently reap another backend's entities that reuse `SharedMarker`** when both
are driven by the same change signal (`ActiveMenuPages`). When two presentations
share a content model, they may share component *data* read by host systems, but
must NOT share the *marker* components that either backend's `despawn`/rebuild
queries match — each presentation needs distinct structural markers.

---

## The question (pre-error)

> You are adding a second `bevy_ui` renderer for the same `MenuPageModel` the
> cube renders. The cube's `rebuild_cube_faces` despawns every `With<AmbitionMenuPage>`
> entity whenever the shared `ActiveMenuPages` changes, and you've just un-gated
> `republish_kaleidoscope_pages` so both backends consume that resource. Your new
> renderer spawns root → panel → tabs → body → content. Which marker components
> may the new entities carry, and which must they NOT — and why? Predict the
> runtime symptom of getting it wrong, before any compiler error.

A correct answer identifies that tagging the new body (or any new entity) with
`AmbitionMenuPage` makes the cube's despawn query reap it (and its content
children) one frame after each spawn — manifesting as a persistent shell (root /
panel / tabs, which lack the marker) with an empty, flashing body — while the
fix is to use a renderer-private marker (`BevyUiMenuBody`) and reserve shared
components for host-read interactive data only. Bonus: note the belt-and-suspenders
alternative of gating the cube's republish/rebuild off when the other backend is
active.

## Expected fix (minimal shape)

```rust
// new renderer's body — NOT tagged with the cube's face marker
panel.spawn((
    Node { /* ... */ },
    BevyUiMenuBody,          // renderer-private structural marker only
    Name::new("menu body"),
)).with_children(|body| { /* content */ });
```

## Validation

Hard to unit-test directly (it needs both backends' systems + the shared
`ActiveMenuPages` in one app). A regression test spawns the new renderer, runs
the cube's `rebuild_cube_faces` once (with `ActiveMenuPages` changed), and asserts
the new body entity + its children still exist. The trap: a test that exercises
only the new renderer in isolation passes while the feature is broken in the real
app (cf. the journal's "a check that mirrors the thing it verifies" entries).

## Why it's a good benchmark

It rewards reasoning about ECS *ownership* and cross-system signals rather than
rendering, and the wrong answer is the natural one ("mirror the other backend's
components"). The symptom (renders one frame then vanishes) reads like a render
bug but is a despawn-ownership bug — a transferable "renders-then-vanishes ⇒ find
who despawns it" heuristic.
