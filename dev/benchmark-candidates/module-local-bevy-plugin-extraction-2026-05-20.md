# Module-local Bevy plugin extraction: when a `register_*_systems` fn moves to a domain module vs. stays in `app/`

**Trap shape**: A Bevy app crate has a 1200-line `app/plugins.rs`
that owns every schedule registration via `register_*_systems(app)` /
`install_*_systems(app)` helper. Refactoring opens the question of
where each helper should live — alongside the systems it registers
(domain modules), or in the orchestrator (`app/`).

**Decision criteria** (validated against Ambition's 13 successful
extractions in OVERNIGHT-TODO #6):

A helper SHOULD move to a domain module's `plugin.rs` (or
inline-added to the module's main file) when:

1. **Every system in the chain lives in the same domain module** —
   no cross-module references except for `crate::app::SandboxSet::X`
   (the schedule set is universal vocabulary).
2. **Cross-module dependencies are unambiguous and one-way** — e.g.
   `WorldPrepSchedulePlugin` lives in `content/features.rs` because
   four of its five systems are in features; the one outlier
   (`ldtk_world::poll_ldtk_file_changes`) is referenced by full
   path. That's tolerable because features is the dominant domain.
3. **No app-local helpers (`pub(super)`) need their visibility
   widened** — or widening from `pub(super)` to `pub(crate)` is
   genuinely reasonable (e.g. `setup_presentation_system` widened
   for `SandboxAudioPlugin` to pin `.after(...)`).

A helper SHOULD stay in `app/plugins.rs` when:

1. **The chain crosses 3+ domains roughly equally** — e.g.
   `register_progression_chain_systems` spans quest, boss, features,
   map, dev_tools, content; no single owner.
2. **App-local helpers dominate** — e.g.
   `register_player_simulation_systems` uses `sandbox_update` and
   `apply_player_damage_system` which both live in `app/`.
3. **The Plugin would just wrap the existing helper without
   simplifying anything** — turning `fn register_x(app)` into
   `struct XPlugin; impl Plugin for XPlugin { fn build(app) {
   register_x(app) } }` is syntactic sugar that adds indirection
   without architectural value.

**Anti-pattern**: extracting a 9-line install fn into a domain
module just to shrink `plugins.rs`. The cross-module visibility
churn costs more than the orchestrator line count.

**Pre-flight checks** before moving:

1. Run `grep -rn 'fn the_system_name' crates/<sandbox>/src/` —
   confirm every referenced system lives in the candidate domain
   module (or a sibling submodule of it).
2. Check what `pub(super) fn` references exist in the install body —
   would moving the install force widening visibility? If so, is
   the widened API reasonable (the function would be `pub(crate)`-
   eligible anyway), or is it a leak?
3. Confirm the new Plugin struct uses the same set + ordering
   constraints as the original helper. Tests should still pass with
   zero scheduling changes.

**Ambition session example** (2026-05-20): 13 helpers moved cleanly
to their domain modules (`Trace`, `LdtkRuntimeSpine`, `SandboxReset`,
`Cutscene`, `GameplayEffects`, `WorldPrep`, `FeatureCollection`,
`FeatureInteraction`, `FeatureViewSync`, `SandboxAudio`,
`Persistence`, `EncounterSimulation`, `PlayerVisual`,
`PresentationVisualAnimation`) — every system in those chains lived
under the destination domain. The remaining 8 helpers
(`PlayerInput`, `PlayerSimulation`, `RoomTransition`, `Combat`,
`PresentationSync`, `ProgressionChain`, `ProgressionPopulate`,
`MiscVisualSync`) stayed in `plugins.rs` because they all span
multiple equally-weighted domains.

**Bench question for a future agent**: "I want to move a
`register_*_systems(app)` helper from plugins.rs to its domain
module. How do I tell whether the move is clean?"

The expected answer: walk the helper's body, group the systems by
module-of-origin, and check whether any one module owns ≥80% of the
chain. If yes, that's the destination; if no, the helper stays in
the orchestrator.

**Reference**: see `OVERNIGHT-TODO.md`'s "Recently retired
(autonomous-mission pass 2026-05-20)" section for the per-plugin
landing list.
