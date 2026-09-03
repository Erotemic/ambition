# Tracks — standing backlog and work reservoir

**Role:** preserve worthwhile unresolved work that is not the current executable
queue. [`queue.md`](queue.md) owns execution order. A focused plan owns design.

When the queue needs another row, re-check the candidate against HEAD and promote
one concrete slice. Do not copy campaign history here.

## Replenishment order

Unless Jon or a new reproducible report changes the order:

1. direct maintainer observations;
2. Ambition flagship needs that expose reusable engine capability;
3. authoritative-state/lifetime/reconstitution correctness;
4. product-facing measured performance or build/iteration blockers;
5. ownership/dependency/SDK improvements with a real consumer;
6. serious secondary game customers;
7. trigger-based work only after its trigger exists.

## Competitive 2D engine bar

Use [`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md)
as a gap detector when replenishing work. It is not a second queue and it is not
an editor-parity checklist. Promote a capability gap only when a real Ambition or
secondary-game customer lacks a supported engine path, then prefer Bevy/ecosystem
machinery for generic concerns and add Ambition policy only where stronger
semantics are needed.

Standing competitive pressure that is legitimate even when not immediately
executable:

- external/minimal game proof that public capabilities compose into a buildable,
  testable, packageable 2D project;
- LLM-first discovery/inspection/mutation/validation across engine and authored
  vocabulary;
- structured diagnostics/provenance/why-not answers that do not require a GUI
  inspector or implementation grep;
- real-customer audits of rendering, animation/VFX, audio, UI, input, assets and
  platform/export completeness;
- measured runtime, hitch/memory and build/test budgets on declared target
  profiles.

Do not promote scene-editor cloning, visual scripting, GDScript parity, general
3D breadth, a plugin marketplace, or a general rigid-body layer without an
actual product requirement.

## Engine architecture reservoir

### Persistent systemic world

- ▢ **Open-world residency.** Preserve the distinction among world existence,
  room residency, simulation activity and local visibility. Decide background
  simulation/residency granularity from actual Ambition/multiplayer pressure.
  Owner: [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).
- ▢ **Persistent occurrence/reconstitution semantics.** Terminal versus
  resettable occurrences, foreign-room relocation, unloaded-room items and
  durable relationships should consume the canonical construction/reconstitution
  model rather than bespoke reset code. Owner:
  [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).
- ▢ **Item custody/accounting residual.** Complete body-owned instance/count
  semantics for held weapons/abilities and unloaded-room occurrence behavior.
  I1 (the process-global equipped mirror) closed 2026-09-02 — the hand is the
  record; I2–I4 remain. Owner:
  [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).
- ▢ **Capability progression/world gating.** Physical verbs remain body-owned;
  knowledge/keys/theorems remain participant-owned. Grow the authoring vocabulary
  only from concrete progression needs. Owner:
  [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md).
- ▢ **Platformer navigation/reachability.** Keep the broad navigation program;
  do not reinterpret the confirmed fighter L6 rollout regression as proof that
  the navigation architecture is wrong. Owner:
  [`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md).

### Capability, package and SDK boundaries

- ▢ **Capability/runtime composition.** Continue when a minimal consumer still
  inherits a capability it did not request or a real host needs substitution.
  The value is dependency/ownership/test/SDK quality, not currently measured
  frame/startup savings. Owner:
  [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
- ▢ **Public SDK 1.0.** Hide implementation topology behind semantic game APIs and
  prove them with external/minimal consumers. Owner:
  [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md).
- ▢ **Reusable Bevy-domain extraction.** Extract/publish only where a mature
  domain has coherent ownership, plugin registration and a second useful
  consumer. Durable doctrine:
  [`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).

### Multiplayer and multiview

- ▢ **N-view production composition.** Per-view projection foundations exist;
  broader layout/HUD ownership/input routing stay deferred until Ambition or
  TwinTrack needs them. Owner:
  [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- ▢ **Per-view camera/reference-frame policy.** The one-view player option and
  per-view camera state are shipped. Remaining shared/split-view policy belongs
  with [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md).
- **Real network transport / Matchbox signaling** — trigger only with an actual
  online Ambition/Smash customer. Use [`engine/netcode.md`](engine/netcode.md);
  do not build transport ceremony just to complete a plan.

### World facts, orchestration and agentic characters

- ▢ **Deterministic world facts + observations/memory.** Simulation truth should
  be explicit and separate from what a character has observed/believes. Owner:
  [`engine/world-facts-observations-and-memory.md`](engine/world-facts-observations-and-memory.md).
- ▢ **Agentic character runtime.** Typed actions/dialogue consume world truth;
  model-backed realtime characters should eventually enter through participant
  seams rather than nondeterministic tick mutation. Owner:
  [`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).
- ▢ **Authored gameplay orchestration.** The prepared-call half exists. A general
  `when ... then` rule representation still deliberately waits for a customer;
  do not promote it merely because the mechanism could be generalized. Owner:
  [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).

### Presentation and observability

- ▢ **Render/animation/VFX.** Continue semantic presentation cues, backend
  separation and quality behavior from product pressure. Owner:
  [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md).
- ▢ **Inspection/diagnostics/workbench.** Build read-only discoverability from
  domain-contributed descriptors rather than a new simulation authority. Owner:
  [`engine/inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md).
- ▢ **Localization/accessibility.** Grow when actual translation/accessibility
  product requirements exist. Owner:
  [`engine/ui-localization-and-accessibility.md`](engine/ui-localization-and-accessibility.md).

## Authoring and content reservoir

- ▢ **LDtk/world tools.** Keep authoring semantics provider/domain-owned and
  validate against the same model runtime construction consumes. Owner:
  [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md).
- **Kinematic-world second customer** — moving-platform K2–K6 work is closed.
  Reopen only when a new moving/dynamic-geometry customer requires the unresolved
  geometric-displacement versus surface-drag split.
- **Compositional spatial semantics** — do not run a universal spatial-object
  campaign. Reopen when a real customer crosses existing surface-semantic axes,
  repeats shared zone identity/provenance plumbing, or is blocked by a closed
  engine spatial switch. Owner:
  [`engine/world-geometry-and-spatial-semantics.md`](engine/world-geometry-and-spatial-semantics.md).
- ▢ **Provider-defined actions through the physical/UI seam.** Provider action
  registration exists; finite controller/touch presentation and multi-map reader
  policy remain real. Owner:
  [`engine/participant-action-system.md`](engine/participant-action-system.md).
- ▢ **Declared-ID/binding diagnostics.** Keep source-qualified authoring failures
  and repair/validation useful where real authored references still bypass them.
  The current residuals are source-qualified per-frame item-art diagnostics and
  concrete registered-but-unloadable asset failures; do not create a universal
  asset census. Owner:
  [`engine/binding-resolution-boundary.md`](engine/binding-resolution-boundary.md).
- ▢ **Semantic dependency/reference graph.** Authoring tools should eventually
  answer questions such as "what references this character?" and support
  structured unresolved-reference diagnostics and transactional rename/delete
  planning across authoring backends. Extend the existing inspection/binding
  model from a real authoring customer; do not build a second runtime registry.
- ▢ **Remaining named-content evictions.** Move named game content out of reusable
  engine crates only when a current dependency/provider boundary still owns it.
- ▢ **Editable SVG/component character authoring.** Continue from concrete
  character-production pressure, not an editor feature checklist.

## Build, platform and performance reservoir

- ▢ **Project build/iteration architecture.** Decide development profile,
  resource-aware test lanes, supported feature-combination checks and generated
  artifact/bootstrap guarantees from measured iteration cost. Owner:
  [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md).
- ▢ **Rendered external-consumer proof.** Headless external consumption exists;
  run the visible consumer on suitable hardware when available rather than
  treating lack of a display/GPU as an engine failure.
- ▢ **Cross-compile persona audit.** Check one host-buildable target/persona at a
  time. Android remains prerequisite-gated on the NDK/toolchain; do not call a
  missing toolchain a product defect.
- ▢ **The Bevy 0.19 Android FONT path is TYPECHECKED, NOT RUN.** The port deleted
  the hand-rolled `seed_android_system_fonts` (`CosmicFontSystem` is gone) and <!-- cite-ok: deleted; the row records the removal -->
  turned on Bevy's `system_font_discovery` for the `android_platform` feature
  instead, which is 0.19's own answer now that fontique owns fallback. ⛔ NOBODY
  HAS SEEN IT RESOLVE A GLYPH: `aarch64-linux-android` cannot even link here, so
  `android-activity`'s build script dies looking for `clang++`. ⇒ **And the
  blocker is a MISSING PREREQUISITE, not a broken path** — re-checked
  2026-09-03: the target is not in `rustup target list --installed`, no NDK is on
  disk, and `ANDROID_NDK_HOME` is simply unset. `scripts/setup/android_prereqs.sh`
  is the repository-owned installer for exactly this (SDK, NDK, Gradle, the Rust
  target and `cargo-ndk`), and `--doctor` reports what is missing without
  installing. That is B5's own distinction in
  [`engine/project-build-and-distribution.md`](engine/project-build-and-distribution.md)
  applied to this row: unsupported-because-unequipped, not code failure. This is
  the one
  0.19 change whose whole job is to find fonts the HOST does not have, so a
  desktop green says nothing about it. Closing it needs a device: launch, read
  logcat, confirm menu and dialogue text render rather than falling back to
  boxes. ⛔ Until then never write "verified on Android" for it.
- ▢ **Asset residency/materialization followups.** Add a budget/eviction
  policy from measured resident-memory or hitch pressure. Since 2026-09-02 the
  instrument has four stages (demand → insert → GPU → first draw), sheets load
  render-world-only, ownership is a guarded rule (a resident character page
  belongs to a realization; two hub↔hall laps return the same working set),
  the shell route's first room decodes before it activates, and the intro cast
  preload is gone. The residual is the LIMIT itself (a host number: `resident_mb`
  at Full after a hub→hall→hub walk; the hall's never-drawn headroom is 5.8×),
  the FX preload's demand seam (8 of 13 sheets are character-owned), and the
  LDtk preview tileset (Jon's submodule). Owner:
  [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).
- **Generic CPU optimization** — no standing campaign. Reopen only with a
  representative measured hotspot/budget failure. Current disproven/low-leverage
  directions are recorded in
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

## Bevy 0.19 follow-ups, ranked

The port to Bevy 0.19.1 landed the engine bump and the typography rework. These
are the remaining 0.19 capabilities worth Ambition's attention, most valuable
first. ⛔ None is a reason on its own to reopen a subsystem — each names the
Ambition pain it would remove.

- ✔ **`FontSize::Vh` for the menu height fractions.** DONE 2026-08-31 (campaign
  section C). `MenuTextHeightFraction`, its per-frame conversion system, the <!-- cite-ok: deleted; the row records the removal -->
  once-only installer and its marker resource are deleted; `MenuNode::Text`'s
  `size` is spawned straight as `FontSize::Vh` and the unit is documented on the
  field itself. The engine resolves against the UI render target, which for
  Ambition is the order-9 default UI camera with no viewport override — i.e. the
  window, exactly what the deleted system read.
- ✔ **The ECS census counted resources as scene content.** DONE 2026-08-31
  (campaign section D). See `runtime_census::EcsPopulation`.
- ▢ **Text gizmos for developer ASCII overlays.** 0.19 ships debug text gizmos on
  a stroke font. Several dev overlays (`fps_overlay`, `gamepad_probe`,
  `rollback_observatory`, `debug_overlay`) draw ASCII only and currently pull the
  bundled monospace face through `UiFonts`; gizmos would drop the asset dependency
  from paths that never ship. ⛔ Developer-only — never dialogue, nameplates or
  product UI.
  - ⚠ **Re-measured 2026-09-03: two of the four named overlays, and the stated
    benefit does not follow.** Only `fps_overlay` and `rollback_observatory` name
    `UiFonts`; `gamepad_probe` spawns a bare `TextFont { font_size, ..default() }`
    (the default face, not the bundled mono) and `debug_overlay` draws NO text at
    all — it is already gizmos (`debug_overlay/gizmos.rs`, `prims.rs`). ⛔ And
    converting both would NOT drop the asset dependency: `UiFontWeight::Monospace`
    is requested twice more in `game/ambition_app/src/app/scene_setup.rs:328`
    and `:351` — the debug HUD and the **quest panel**, and a quest log is
    product UI.
    ⭐ **There is a real COMPILE-time edge here, but it is narrower than I first
    wrote.** `JetBrainsMono-Regular.ttf` is untracked and gitignored
    (`.gitignore:156`, fetched by `scripts/grab_font_assets.py`) and it is
    `include_bytes!`d by `embed_core_assets!` as `FONT_DEBUG_MONO_URL`
    (`crates/ambition_asset_manager/src/platformer_assets/embedded.rs:64`) — so
    on a fresh clone the file is absent and the embed cannot compile.
    ⛔ **CORRECTION, same day:** I first wrote here that a tree missing the file
    *"fails `cargo check --workspace` outright"*. **I could not reproduce that
    mechanism and am retracting it.** The `include_bytes!` sits behind
    `#[cfg(feature = "static_core_assets")]`, which only `visible_web` enables
    (`ambition_platformer2d_actor_monolith/Cargo.toml:174`,
    `game/ambition_app/Cargo.toml:93`); no member's default features enable it,
    and under `resolver = "2"` a workspace-wide feature resolution does not turn
    it on. ⇒ The compile-time dependency is real for a **`visible_web` build**
    and not for a default workspace check. What actually failed on this host on
    2026-09-02 I cannot now reconstruct, so it is recorded as unexplained rather
    than attributed to this edge.
    ⇒ If this item is ever taken, the thing worth removing is the COMPILE-time
    embed on the web path, and that needs the quest panel answered first — not
    the two overlays.
  - ⚠ **A second re-measurement the same day (yardrat), kept beside the first:**
  ⭐ **`debug_overlay` HAS ALREADY DONE THIS, and its receipt is the argument for
  the rest.** `render_debug_overlay_labels` draws the per-frame label buffer with
  `Gizmos::text_2d`; what went with the retained `Text2d` entities was the spawn
  churn, the per-frame despawn sweep, the `DebugOverlayLabel` marker — and *"the
  one that mattered — the dependency of F1 world labels on the PRODUCT font
  stack"*. It also settles the question this row would otherwise have to ask
  first: `Gizmos::text_2d` takes a world-space `Isometry2d`, so `font_size` stays
  world units and labels keep scaling with camera zoom.

  ⚠ **The remaining candidates are TWO, not four.** Measured 2026-09-03:
  `fps_overlay.rs` and `rollback_observatory.rs` are the only dev overlays that
  name a face through `UiFonts`. `gamepad_probe` spawns `TextFont { font_size,
  ..default() }` — it never asks for the bundled face, so converting it buys
  nothing.

  ⛔ **AND `fps_overlay` MAY NOT WANT IT.** It reaches for `UiFonts` deliberately
  — *"a counter whose digits change width jitters on every frame it updates"* —
  so a conversion has to establish that the stroke font is fixed-advance before
  it can claim parity. Check that first; if it is not, this row is about
  `rollback_observatory` alone and is probably not worth a card.
- ▢ **`Rem` sizing for UI accessibility scaling.** `FontSize::Rem` plus the
  `RemSize` resource is a global UI text scale for free. Wants a concrete
  accessibility or Steam-Deck legibility requirement first; do not convert
  existing fixed sizes speculatively.
- ▢ **Resources-as-components for singleton authorities.** `Resource: Component`
  means observers and component hooks now work on singletons. Possibly useful for
  authorities that currently need a follow-up call to stay coherent. ⛔ Rollback
  ownership and deterministic ordering outrank the ergonomics; `IsResource` also
  now leaks resources into broad `EntityMut` queries, which is a hazard before it
  is a feature.
- ▢ **Upstream text entry.** 0.19 has first-party text editing. Note it where
  Ambition would otherwise hand-roll an input field; nothing today needs one.
- ▢ **BSN for large declarative spawn trees.** Watch it for menus/presentation.
  Not a migration target: the runtime content that would benefit is exactly the
  content a scene format would freeze.

## Combat, AI and behavior reservoir

- ▢ **Boss animation vocabulary fold.** Converge remaining boss animation/frame
  vocabulary only where a current boss/product customer needs it.
- ▢ **Dialogue continuity in a running world.** Body-generic interruption,
  separation and station-keeping. Owner:
  [`engine/dialogue-continuity.md`](engine/dialogue-continuity.md).
- ▢ **Listener-side dialogue adaptation.** Speaker/listener identity exists;
  content adaptation to listener context remains future product capability.
- ▢ **Character dialogue from suggestions/barks.** Intentionally shelved product
  design; preserve intent without promoting it absent a customer.
- ▢ **Falling-sand extensions.** Water/oil/Oiler-related mechanics remain desired
  deferred work. Owner: [`engine/falling-sand.md`](engine/falling-sand.md).
- ▢ **Per-route music within one experience.** Preserve as a product capability;
  do not keep the old completed audio campaign as its execution authority.

## Game/customer reservoir

- ▢ **Ambition open-world production.** Primary product driver. Use
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md),
  [`game/systemic-progression.md`](game/systemic-progression.md), direct
  observations and current focused engine plans.
- ▢ **Super Smash Siblings.** Current product work is executable as D72; standing
  feature truth belongs in
  [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).
- ▢ **TwinTrack.** Keep as a future multiview/reference-frame customer; Jon has
  explicitly deprioritized it relative to the main games.
- ▢ **Sanic / Super Mary-O / Hollow Lite.** Retain their focused acceptance lists
  as movement/collision/world-authoring/encounter customers; do not duplicate
  those lists here — they are
  [`demos/sanic.md`](demos/sanic.md), [`demos/super-mary-o.md`](demos/super-mary-o.md)
  and [`demos/hollow-lite.md`](demos/hollow-lite.md). ⚠ The links are the point,
  not decoration: `demos/sanic.md` states that *"this list is the single source;
  status.md and tracks.md refer here"*, and until 2026-09-03 neither file named
  it, so the deferral was true and unfollowable.
- ▢ **Player-facing authored-art repairs.** Morph-ball presentation, shrine/glider
  presentation and similar content fixes remain product work unless a reproduced
  defect demonstrates a reusable renderer/authoring-system problem.

## Trigger-based work

Do not promote these until the trigger exists:

- Slower Light — wait for a real 3D runtime/customer.
- Leafwing clash-scan optimization — only if dependency/version changes make the
  measured 1–3.1% CPU cost relevant; do not carry a permanent fork.
- Broader stable-ID centralization — introduce common infrastructure only after
  concrete identity families share actual operations.
- Provider-owned placement-family extension — wait for a provider outside the
  common authored vocabulary.
- Reusable menu-host extraction — wait for a real second consumer.
- Boss crate extraction — wait until boss vocabulary/ownership is coherent.
- Body-generic NPC economy/world interaction — wait for NPC agency or multiplayer
  currency pressure.
- Dormant `GravityFlipSwitch` cluster — delete the unused actor/render/rollback
  path unless a real authored overlap-plate customer appears; keep the live LDtk
  switch/action path as the gravity-switch authority.
- Provider-owned persistence/item identities — extract Ambition-specific save or
  item vocabulary only when a second real provider needs to own that domain.
- Test execution parallelism — re-measure only if execution, rather than
  compile/link or memory pressure, becomes dominant.

## Standing execution rule

Before promoting a card:

1. inspect HEAD and confirm the missing thing still exists;
2. prefer a focused plan that already owns the design;
3. state the product/authority/dependency payoff;
4. give one inspectable acceptance criterion;
5. promote only executable work to `queue.md`;
6. let completed history disappear into git instead of growing another ledger.
