# Bevy plugin and reusable crate strategy — Engine 1.0 program

**State:** OPEN — direction is settled; individual crate cuts remain evidence-driven.

## Goal

Turn reusable Ambition engine capability into **idiomatic Bevy crates and plugins
that other Bevy games can actually depend on** without turning the workspace into
a crate-per-directory maze.

Ambition should increasingly look like a substantial Bevy game composed from
coherent domain plugins. When a capability proves reusable, the preferred
progression is:

```text
private module
    -> coherent Ambition workspace domain
    -> workspace crate with an honest Bevy Plugin/system API
    -> independently consumable Bevy crate
    -> optional crates.io / public ecosystem release when mature
```

There is **no goal to move Ambition subsystems into Bevy itself**. The contribution
back to the ecosystem is to make strong reusable crates that ordinary Bevy games
can add as dependencies and compose through normal Bevy systems/plugins.

This is a product and ownership strategy, not a requirement to publish every
internal crate.

## What "idiomatic Bevy crate" means here

A reusable domain boundary should normally:

- own its authoritative `Component`, `Resource`, `Message`/event, query and local
  `SystemSet` vocabulary;
- install its behavior through a domain `Plugin` (or a deliberately small plugin
  family);
- expose explicit inputs/outputs instead of asking consumers to call private
  systems in a prescribed order;
- let application composition use normal Bevy plugin composition rather than
  reaching into implementation modules;
- move registration and schedule ownership with the domain instead of leaving a
  callback or census in an old composition root;
- use public symbols that describe domain concepts rather than Ambition content
  or historical crate topology;
- install into a small `App` or test harness without `ambition_app`;
- support a headless path when the domain itself is simulation-side;
- depend only on the Bevy features and Ambition-independent lower-level crates it
  actually needs when that materially improves capability closure or compile
  isolation;
- avoid named Ambition content, story policy, global `PrimaryPlayer` assumptions,
  and actor-monolith dependencies in its reusable layer;
- make optional integrations explicit through separate plugins/features rather
  than silently importing the world.

Do **not** mechanically replace `bevy` with dozens of subcrate imports if that
makes a small crate harder to understand. Dependency narrowing is useful when it
improves optionality, portability, compile behavior, or the truthfulness of the
public boundary.

## Crate extraction test

Before creating or publishing a crate, answer:

1. **Authority:** what data or policy does this crate uniquely own?
2. **Registration:** can its `Plugin` register the domain without an upward
   dependency or registration callback left in the old owner?
3. **Consumers:** who can use it independently today or in the next credible
   slice?
4. **Dependency closure:** can a consumer opt into it without inheriting
   unrelated Ambition capabilities?
5. **Testing:** can its core behavior be exercised with a small Bevy `App`,
   headless harness, or pure data tests?
6. **Public API:** is there one obvious happy path rather than exposing internal
   assembly details?
7. **Compile/change amplification:** does the split remove a meaningful rebuild
   edge or isolate a volatile domain?
8. **Bevy ergonomics:** does the API feel like ordinary Bevy composition rather
   than an Ambition-specific framework hidden behind generic names?
9. **Independent value:** would another Bevy game plausibly choose this crate for
   the capability itself?

A "no" does not forbid a carve. It identifies what is not yet a durable reusable
crate boundary.

## Maturity ladder

### Level A — coherent internal domain/plugin

The implementation may remain inside a larger workspace crate, but authority,
registration, inputs and outputs are already explicit. This is often the correct
first step while semantics are still changing quickly.

### Level B — dedicated Ambition workspace crate

Extract when the boundary has independent ownership, a measurable dependency or
compile benefit, and a useful test surface. It may still use `ambition_*` naming
and internal versioning while the API is being proven.

### Level C — independently consumable Bevy crate

A mature candidate should have:

- Ambition-independent naming and documentation;
- a small feature/dependency surface;
- a documented `Plugin` entry point and any optional integration plugins;
- one or more minimal runnable examples or harnesses;
- no requirement for Ambition content registries or application composition;
- public components/resources/messages that are useful to external games;
- an explicit Bevy-version compatibility policy;
- enough semantic stability that publishing it does not freeze a migration
  facade as the public API.

Publishing to crates.io or another public location is optional. The important
milestone is **independent consumability**.

Potential future candidates include platformer reachability, participant/view
presentation helpers, item custody/accounting primitives, instance provenance,
agent-facing inspection, and narrowly scoped world-residency facilities. This is
not a promise that any current implementation is ready to split or publish.

## Composition guidance

Prefer a shape like:

```rust
app.add_plugins((
    PlatformerWorldPlugin,
    ActorKernelPlugin,
    ItemCustodyPlugin,
));
```

with optional bridges such as:

```rust
app.add_plugins(RollbackItemCustodyPlugin);
```

rather than a facade that imports every capability or asks the host to manually
register each domain's components and systems.

A reusable crate may expose extension traits, system sets, events/messages,
resources, `SystemParam`s, `QueryData`, schedules, or preparation APIs where
those are natural Bevy seams. Do not force every domain into the same pattern.

## Immediate applications

- the controlled-character/actor kernel should become a coherent simulation
  plugin/domain before the actor monolith is renamed;
- world residency, navigation, item accounting and instance lifetime should be
  designed so their reusable cores can live below Ambition game policy;
- UI, diagnostics and multiview work should prefer small plugins with explicit
  inputs over another all-knowing host layer;
- rollback integration should be an optional bridge owned alongside the domain,
  not the reason a generic runtime crate imports every gameplay crate;
- authoring/preparation crates should expose structured data/diagnostics that can
  be consumed without running the Ambition game.

## Open design questions — deliberately unresolved

- Which current workspace crates already have a strong enough domain boundary to
  become independently consumable with little redesign?
- Should reusable crates retain `ambition_*` names until their APIs stabilize, or
  should independently published crates receive separate names?
- How aggressively should public crates depend on `bevy_ecs` / `bevy_app`
  subcrates rather than `bevy` with selected features?
- What Bevy-version compatibility policy is realistic while Bevy itself evolves
  quickly?
- Which integrations should be crate features versus separate bridge plugins?
- How should cross-crate schedule ordering be expressed without a global engine
  schedule god-object?
- Which domains are too Ambition-specific to publish but still deserve clean
  internal plugin boundaries?
- When does splitting a crate improve compile/change isolation enough to justify
  the maintenance and versioning cost?

These questions should remain open until a concrete extraction forces an answer.
