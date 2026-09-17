# Reusable authored world composition — Engine 1.0 design program

**State:** INCUBATING — do not implement a universal prefab system yet.

> **RE-MEASURED against `36c5cbf7a` (2026-09-02). The hold still holds, and it now
> has numbers to be re-evaluated against instead of a judgement call.**
>
> ⭐ **Nothing has crystallised.** No `*Assembly`, `*Prefab`, `*Composition` or
> `*Motif` type exists for authored world structure. The one near-miss is
> `ShellComposition` (`ambition_platformer2d_provider/src/composition.rs`), and
> reading it settles rather than complicates the question: it holds an
> experience id and two route ids — shell/route composition, not world
> composition.
>
> ⭐ **And the authored unit is still exactly one placement.** `PlacementKind`
> has **six flat variants**: `Hazard`, `Interactable`, `Pickup`, `Chest`,
> `Breakable`, `Portal`. There is no composite, no reference from one placement
> to another, and no authored grouping above them.
>
> **Meanwhile every pressure this page predicted has grown a large bespoke
> implementation.** ⭐ **RE-TAKEN 2026-09-17 WITH THE SEARCH TERM RECORDED,**
> which this table asked for and did not have. Every row below is
> `grep -rl <TERM> --include=*.rs crates game`, case-SENSITIVE, counting files:
>
> | predicted pressure | term | 2026-09-02 | 2026-09-17 |
> |---|---|---:|---:|
> | portal mechanisms | `Portal` | 264 | **182** |
> | moving machinery | `KinematicPath\|MovingPlatform` | 58 | **70** |
> | shrines | `Shrine` | 40 | **19** |
> | encounter assemblies | `EncounterSpec\|EncounterId\|EncounterDefinition` | *(no term recorded)* | **20** |
> | environmental hazards | `HazardSpec\|HazardRespawn` | 11 | **11** |
>
> ⛔⛤ **THE ONLY ROWS THAT MEAN ANYTHING ARE THE ONES WHOSE INSTRUMENT WAS
> WRITTEN DOWN, AND THAT IS THE FINDING.** Hazards and machinery named their
> terms: hazards reproduce EXACTLY at 11 and machinery moved 58 → 70, and both of
> those are facts about the code. Portals and shrines named none, so their 264 →
> 182 and 40 → 19 are uninterpretable — and measurably so. A case-INSENSITIVE
> `.rs` sweep gives 299 portals and 50 shrines, so the old numbers sit INSIDE the
> bracket the term choice spans, and the same tree supports "it grew" or "it
> shrank" depending on a flag nobody recorded. ⇒ Do not read the fall as a carve
> and do not read it as rot; read it as a number that was never a measurement.
>
> ⚠ The encounter row is re-founded rather than corrected. It said *"27, in 2
> crates"*, and the crates are now four — `ambition_boss_encounter`,
> `ambition_encounter`, `ambition_encounter_features` and the monolith's own
> features. The term above is the one this page will use from now on; its 20 is a
> new baseline, not a comparison.
>
> ✔ **The two structural claims are unchanged, and they are what the hold rests
> on.** The `*Assembly` / `*Prefab` / `*Composition` / `*Motif` type census still
> returns exactly ONE hit — `ShellComposition`
> (`crates/ambition_platformer2d_provider/src/composition.rs`), which holds an
> experience id and two route ids, so it is shell composition and not world
> composition. And `PlacementKind` still has six flat variants — `Hazard`,
> `Interactable`, `Pickup`, `Chest`, `Breakable`, `Portal` — with no composite, no
> reference from one placement to another, and no authored grouping above them.
>
> ⇒ **So the two halves of the incubation are BOTH still true, which is why it
> stays incubating.** The pressure is real and unrelieved — five predicted
> families, all built out, none sharing a composition abstraction. And no
> abstraction has emerged on its own from any of them, which is the evidence the
> page said to wait for. ⛔ A file count is pressure, not a design: 182 files
> mentioning `Portal` is an argument that portals are load-bearing, not an
> argument for a prefab system. ⇒ Re-take the numbers with the terms in the table
> above — a count taken with an unrecorded instrument cannot be compared to
> anything, including itself.

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

## ⚠ An emerging direction — recorded, not adopted

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

⭐ that would give scene-like compositional **power** without adopting a universal
Node tree and without embedding arbitrary Rust plugins inside authored
compositions — which is the shape this program has been looking for.

⛔ **this does not raise the priority of composition and does not lower its
evidence threshold.** The state above still reads INCUBATING. Do not move
composition implementation ahead of its evidence merely because the direction is
becoming clearer; the two-materially-different-uses rule stands.

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

A8 uses two copies of the same authored room as the future instance-isolation
witness. Content identity can be shared while live entities, mutable overlays and
occurrence histories remain scoped. Do not claim that different authored room IDs
prove repeated-instance safety or require all identity types to share one new
namespace. Single-active-room customers should retain the current simple path.
