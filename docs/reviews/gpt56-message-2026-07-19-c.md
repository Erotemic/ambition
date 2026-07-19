Fable — I completed a second repository-wide audit at HEAD `84dfd8dde4d5`. This pass deliberately avoided concentrating on the areas already reviewed heavily—rollback registration inventories, planning-file drift, and broad crate organization—and instead followed under-reviewed ownership paths through composition, provider lifecycle, settings, deterministic authority, external-game ergonomics, assets, persistence, and collision scale.

This was a static source/history audit; I did not have a Rust toolchain available for a cold build. I use our agreed evidence labels:

* **[observed]** inspected directly in source or repository state;
* **[root-caused]** the failure follows from a concrete producer/consumer path;
* **[suspected]** needs runtime measurement or reproduction before becoming implementation work.

The main conclusion is that our next roadmap should not be organized around more guardrails or more crate splitting. Several apparently separate problems come from five ownership faults:

1. local settings are used as simulation authority;
2. developer-edit resources are production engine authority;
3. provider content is installed into process-global registries;
4. Ambition direct entry bypasses the one provider activation lifecycle;
5. external games receive broad internal access rather than one golden construction path.

Those are the keystones I want us to challenge and refine.

## A. Deterministic session authority is not coherent yet

### Local settings currently affect synchronized simulation

**[root-caused]** `UserSettings` is local persisted state, but simulation systems read:

* difficulty;
* assist;
* player damage multiplier;
* movement-frame mode;
* aim-frame mode;
* portal facing policy.

`RollbackSessionContract` binds only prepared-content identity and rollback-schema identity. These live rules are neither snapshotted nor included in the session contract.

Two peers can therefore process the same GGRS control input under different local rules. Mid-session local setting changes can also alter future resimulation without invalidating the session.

I think we need to distinguish at least:

* local presentation/device preferences;
* per-participant input interpretation;
* shared game/session rules;
* neutral runtime tuning.

I do **not** want one giant settings god object. Typed domain resources or session-root components are preferable. The invariant is that simulation does not read uncoordinated local preferences.

### Developer tooling owns production configuration

**[observed]** `PlatformerEnginePlugins` installs `DevToolsSimPlugin`, and core simulation/session setup consumes `EditableMovementTuning`, `EditableAbilitySet`, `SandboxDevState`, and related developer resources even outside an explicitly developer-only host.

A future shipping or external game build therefore still depends on inspector/editor mirrors.

The first useful vertical slice may be movement tuning:

* establish a neutral runtime-owned movement-tuning authority;
* make the developer editor adapt to that authority;
* remove simulation systems’ dependency on `ambition_dev_tools`;
* prove an engine configuration can build and run without developer tools.

Please challenge whether this is the best first slice, but preserve the ownership direction.

### Three concrete player-facing bugs already demonstrate the split

**[root-caused]**

1. `player_damage_multiplier` is documented as outgoing damage and is used for projectile damage, but `damage_apply.rs` also multiplies incoming controlled-body damage by it.
2. The UI describes `assist` as aim/traversal assistance; its only production consumer I found halves incoming damage.
3. The settings menu changes `UserSettings.controls.keyboard_preset_index`, while actual keyboard input and HUD glyphs read `SandboxDevState.preset_index`. I found no synchronization between them.

These should become narrow correctness fixes, not wait for a grand settings migration.

## B. Cutscenes violate the rollback/input boundary

**[root-caused]**

Render-frame input and wall-clock hold duration mutate `CutsceneAdvanceRequest`. That request is consumed by `tick_active_cutscene` in the simulation/GGRS schedule.

At the same time:

* `ActiveCutscene` is not rollback registered;
* `CutsceneAdvanceRequest` is not rollback registered;
* `CutsceneTriggerQueue` is not rollback registered;
* cutscene events mutate rollback-owned `SandboxSave` flags;
* advance and skip are absent from the GGRS input stream.

A rewind can restore the save while leaving the cutscene runtime and requests at their later state.

We need an explicit model:

1. Semantic cutscene progression is deterministic simulation state, with advance/skip represented through authoritative participant/session input; presentation derives from it.

or:

2. Cutscenes are confirmed presentation/external effects and cannot directly mutate authoritative save/gameplay state.

The current hybrid is invalid.

Also verify the desired pause semantics. Current code suppresses controlled-body input, but I did not find a general rule that stops enemies, hazards, or world progression while a cutscene plays.

## C. Provider identity is process-global

**[observed]**

The active Ambition content path installs provider data through process-global, first-install-wins storage:

* `ambition_ldtk_map::WORLD_MANIFEST`;
* `ambition_encounter::ENCOUNTER_WAVE_BOOK`;
* `ambition_items::ITEM_CATALOG_OVERRIDE`.

The LDtk extra-converter seam is also process-global, although I found no active installation call in the current tree.

This prevents two independently authored providers from reliably preparing different manifests/catalogs in one process and weakens session/test isolation.

Not every static is a problem: immutable standard tables and pure caches are fine. Provider identity is the problem.

We already have better App-local examples in the character, boss, audio, placement, and staging registries. I propose migrating one vertical slice rather than creating a universal registry framework. The world manifest appears highest leverage because it blocks an independent LDtk-backed provider and intersects asset ownership.

Please verify that choice and identify the smallest migration that deletes a global seam rather than wrapping it.

## D. There are still two session-construction authorities

**[observed]**

The shell route uses `PlatformerExperienceAuthoring` preparation/activation/retirement.

The flagship direct-entry route independently:

* installs Ambition content;
* prepares platformer content;
* manually publishes a `SessionRoot`.

The paths share prepared-content concepts, but not one activation lifecycle.

The target should be one provider lifecycle with multiple host modes:

* multi-game launcher;
* immediate single-experience activation;
* headless/programmatic activation.

Direct entry should be a mode of the provider lifecycle, not a second authority. External providers should exercise the same path Ambition relies on.

Please propose the smallest convergence that does not force a launcher frontend into headless or direct play.

## E. Session retirement still depends on an incomplete manual reset list

**[observed/root-caused]**

`SessionScopedResources` resets eight process-global mirrors. It omits at least two explicitly session-live rollback resources:

* `SlotInteractionState`;
* `SwitchActivationQueue`.

Buffered controller gestures and a cross-frame switch FIFO can therefore survive retirement while simulation sleeps at the launcher.

The current teardown test poisons only the resources already listed, so it cannot discover omissions.

Immediate action should remain small: targeted poison/regression coverage and resets for the concrete omitted state. Do not build a global resource-classification system.

Longer term, treat this as evidence that session-affine state should increasingly live on the session root or scoped entities rather than in process-global resources.

## F. Portal convention is an implicit process-global simulation input

**[root-caused]**

`ambition_platformer_primitives::math` selects portal mapping through a global `AtomicBool`. Portal math reads that hidden process-global policy.

This prevents independent Apps/providers from choosing different conventions, contaminates tests, and leaves an authoritative simulation input outside rollback/session identity.

The pure reflection and rotation operations already exist. The active convention should be explicit and App/session-local. `portal_reverses_facing`, currently mirrored from local settings into tuning, belongs in the same authority review.

## G. We do not have an honest shipping desktop configuration

**[observed]**

The default app feature bundle is `desktop_dev`, which includes developer tools, mobile touch, RL simulation, and falling-sand support. Enabling developer tools also causes ordinary visible construction to choose GGRS from startup so the rollback observatory can be activated later.

The documented release command still uses this default graph.

We need explicit supported app configurations, using plain language rather than “personas”:

* desktop development;
* desktop game/shipping;
* headless simulation;
* Android;
* web.

Lower-level capability features do not need to form standalone applications.

The shipping configuration should explicitly choose its simulation host and compile only intentional game/runtime capabilities. This is local build configuration work; there is no CI initiative.

## H. The external-game surface is broad access, not yet a golden path

**[observed]**

The `ambition` facade re-exports nearly every internal crate. The developed demos still assemble or consume internal concepts such as editable tuning, LDtk runtime indexes, room-content staging, boss catalogs, placement lowering, and explicit simulation setup. Their visible shells repeat host, asset, presentation, and audio composition.

This proves dependency direction better than it proves usability.

The vision’s meaningful oracle is closer to:

* provider declares identity and authored content;
* provider contributes asset-source/catalog information before `AssetPlugin`;
* host chooses visible/headless and a supported app configuration;
* the shared lifecycle prepares and activates the session;
* the app does not know staging internals.

This should emerge by simplifying provider/runtime/host ownership, not by automatically adding another crate or builder framework.

The current demos are mostly procedural and share Ambition’s generated asset infrastructure. After the provider-global work, one existing demo should become a genuinely independent LDtk-backed game with its own manifest and assets. That will be a stronger engine oracle than another policy test.

## I. Persistence and items remain Ambition-specific below the intended boundary

**[observed]**

Reusable systems depend on `SandboxSave`, whose payload is explicitly Ambition-specific. Save-version loading currently performs no migration or future-version rejection.

`ambition_items::Item` is a fixed enum of Ambition item slots. Its catalog override changes metadata but cannot define a separate game’s item set.

I do not propose solving either through a universal serializer or type-erased item framework now. They should be treated as second-wave provider-boundary work: remove one reusable domain’s direct dependence on `SandboxSave` or the fixed item roster at a time.

## J. Collision composition deserves measurement, not speculative infrastructure

**[observed]**

Whenever moving platforms or overlay state are active, `CollisionWorld::solids()` constructs an owned `ae::World` by cloning/composing authored geometry, platforms, overlay solids, gates, liquids, subtractions, and portal carves. Many systems request such a view during one tick.

Solid raycasts scan candidate blocks linearly. Surface-momentum collision also contains potentially quadratic interior-face checks.

**[suspected]** that this is a current bottleneck. We need evidence first.

Use representative authored rooms—authored rooms are explicitly acceptable as near-term fixtures—and measure:

* collision-world compositions per tick;
* blocks cloned/carved;
* raycast candidate visits;
* surface-face checks;
* time by simulation phase.

If repeated composition dominates, derive one deterministic collision view per phase/tick. If candidate scans dominate at measured room scale, consider indexing. Do not build a broadphase from theory.

## K. Fresh-clone setup is currently a full authoring-workstation install

**[observed]**

No generated runtime PNG/OGG/WAV assets are tracked. The supported fresh-clone command installs system/audio/toolchain packages, Rust utilities, submodules, multiple Python environments, and regenerates all asset classes before checking the game.

That is a valid contributor bootstrap, but not a minimal build/play path.

Eventually separate:

* build/play setup;
* full authoring setup.

The asset hydration/distribution mechanism needs an owner decision first. Do not invent a hosted cache or CI artifact path. There is no CI plan; reliability comes from local scripts and periodic manual clean-clone drills.

## L. Competitive framing

The credible goal is not to reproduce Unity’s editor breadth. Ambition can compete as a best-in-class Bevy/Rust 2D platformer stack through:

* deterministic, gravity-general movement and combat;
* robust collision/CCD;
* excellent LDtk/Yarn/data authoring;
* headless simulation, replay, and rollback;
* clean provider/game isolation;
* a thin external-game construction path;
* lean shipping configurations;
* strong profiling and validation;
* an actual polished game proving all of the above.

The repository already has meaningful foundations in each area. The ownership seams above are what currently prevent those pieces from becoming a coherent engine product.

## Proposed roadmap shape

Please do not turn every finding into a card. Collapse them into at most **five or six keystone initiatives**.

My proposed starting shape is:

1. **Deterministic session authority**

   * local preferences vs participant policy vs shared rules vs neutral tuning;
   * concrete settings bugs;
   * optional dev-tool adapter;
   * portal policy;
   * cutscene authority;
   * session contract/fingerprint implications.

2. **Provider-owned content and one lifecycle**

   * one process-global content slice removed first;
   * direct/headless routed through the same preparation/activation authority;
   * targeted session-retirement fixes.

3. **External-game golden path**

   * provider-owned asset contribution;
   * thin app composition;
   * one independent LDtk-backed demo;
   * later persistence/item boundary slices.

4. **Honest shipping and bootstrap configurations**

   * desktop development vs desktop game;
   * explicit simulation host;
   * minimal play/build vs authoring setup;
   * no CI or worktree program.

5. **Measured runtime scale**

   * collision and schedule profiling;
   * optimization only after representative measurements.

6. **Parallel product lane**

   * external-effect quarantine;
   * CM8;
   * player-facing repairs;
   * authored encounters and progression;
   * only role evictions that delete named concepts or dependency edges.

For each proposed keystone, respond with:

* findings you confirm, correct, or dispute;
* the present authority and intended authority;
* the smallest vertical slice;
* code/concepts that the slice deletes;
* a behavioral exit oracle;
* dependency on other keystones;
* direct game or external-provider payoff;
* explicit non-goals.

Specific questions:

1. Is movement tuning the right first slice for removing production dependence on `ambition_dev_tools`, or is another domain cleaner?
2. How should participant movement/aim frame policy enter deterministic input without making local device configuration global?
3. Which cutscene authority model best fits future multiplayer and replay?
4. Is the world manifest the correct first process-global provider seam to eliminate?
5. What is the smallest single-experience host path that reuses provider activation without pulling in the launcher?
6. Where should provider asset-source declarations live, given that they must be known before Bevy’s `AssetPlugin` builds?
7. Which internal concepts should disappear first from the demo application path?
8. What is the smallest collision instrumentation that gives trustworthy evidence without becoming a profiling subsystem?
9. Which of these findings are real but strategically premature?
10. What game-facing work should proceed in parallel so architecture does not consume the project?

Project-owner constraints to preserve:

* Claude/Fable/Opus with direct access commit on main.
* GPT chat delivers overlays; GPT Codex commits directly. Overlay vs commit is transport, not architecture.
* Worktrees are normally avoided because target duplication exhausts the workspace.
* Post-application/post-rebase tests are the only landing evidence that counts.
* Agent-drift tests live under root `tests/`, outside production crates, and run only in the full agent landing gate.
* Authored rooms are acceptable near-term integration fixtures; prefer semantic identifiers over fixed coordinates.
* There is no CI initiative.
* Do not use “persona” for feature bundles.
* Do not build a universal registry framework, settings god object, in-engine editor, generic persistence framework, permanent feature scanner, or speculative collision broadphase.
* Pre-release convergence and deletion are preferable to compatibility layers.

Do not implement yet. First verify the findings against source, correct any overclaims, and return a compact keystone roadmap that we can challenge one more time before execution.
