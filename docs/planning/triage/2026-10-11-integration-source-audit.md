# October 11 integration audit: source evidence, falsifiers and non-findings

**Review source:** archived tree at `ed0d1ccd` (2026-10-11), covering
`abf79de..ed0d1ccd` (133 commits). **Method:** trace producers, consumers,
life cycles, scheduling and existing plan owners. This is a *source* audit;
no production Cargo simulation, GPU profiling or rollback replay was run.
Do not equate an apparent static defect with a measured user-facing failure.

**Execution order:** [queue](../queue.md#review-1011-integration---real-engine-contracts-for-new-demo-behavior).
Each engine owner document specifies the target abstraction. This is evidence,
not a second work tracker.

**Current-tree status check at `e473d2c` (2026-10-11):** S1-S3 of the
`actor_think` migration have landed. The loaded Void Lizard and Laden Swallow
submit fire before the common control gate and have held-body witnesses;
`select_actor_targets` filters `ActorTarget` candidates by `LiveRooms`.
The table's late-fire and primary-target rows describe the **older audited
snapshot**, not outstanding defects in those two modules. Snake Plant still
reads `PrimaryPlayer` without a live-room join and can retain its pose marker.
The rollout also found an open, reproducible brain rollback mismatch at shot
expiration: [SHOT-END-REPLAY](../queue.md#shot-end-replay--a-brain-that-saw-its-shot-end-replays-differently---done-2026-10-11) (fixed 2026-10-11, d215fcbdf: the cause was `BodyMotionFacts` read across a rewind, not the shot's end).

| Case | Source evidence at this commit | Static verdict | Minimum falsifier / witness | Owner |
| --- | --- | --- | --- | --- |
| Portal link renumbering (✅ fixed 2026-10-12, see the owner page) | `crates/ambition_portal2d/src/link.rs:69-127`; rank in sorted *current* hashes **across all rooms** becomes channel; at most 63 distinct link hashes globally | Definite identity instability on insertion/removal of lower-sorted link, even in another room, plus global scale cap | Two static pairs, insert a third in a different room, inspect both old channels/cooldowns; distribute 64 names across rooms | [portal identity](../engine/portal-pair-identity-and-aperture.md) |
| Portal A/B endpoint swap (✅ fixed 2026-10-12, see the owner page) | `link.rs:83-113` sorts positions, no authored endpoint role | Definite channel swap when positions cross; equal positions both choose first slot | Move two endpoints through each other, then overlap; assert role and transit continuity | [portal identity](../engine/portal-pair-identity-and-aperture.md) |
| Irreversible aperture shrink (✅ fixed 2026-10-12, see the owner page) | `link.rs:130-153` writes min opening into the same `PlacedPortal.half_extent` it reads next frame | Definite lossy update if an opening later grows | Shrink one mate, restore it, inspect previously authored size | [portal identity](../engine/portal-pair-identity-and-aperture.md) |
| GNU-ton duplicate portal request (interim repair 2026-10-12; port open, see the owner) | `game/ambition_content/src/bosses/gnu_ton/portals.rs:125-228` joins by room+line, fixed channels 24/25, singleton `SimId` | Definite missing encounter scope | Two bosses, same live room, distinct portal destinations before/after one dies | [boss debt 12](../game/gnu-ton-fight-debts.md#12-the-portal-and-trail-names-omit-the-encounter) |
| GNU-ton gate reads corpse state (✅ fixed 2026-10-12, see the owner) | `game/ambition_content/src/bosses/gnu_ton/mod.rs:79-106` uses `any(!alive())` | Definite early opening for two bosses; potential reclose after corpse cleanup | Two bosses defeat-one/defeat-both, despawn corpses, reload/rollback | [boss debt 13](../game/gnu-ton-fight-debts.md#13-the-exit-gate-reads-a-corpse-not-an-encounter-outcome) |
| Mary-O fire bypass (historical, repaired) | At `ed0d1ccd`, `game/ambition_demo_mary_o/src/{void_lizard,swallow}.rs` wrote control in `WorldPrepSet::AfterIntegrate` after the gate | Repaired by I4 S1-S3 in `e473d2c`: native fire writers deleted, module intent pre-gate; production witnesses in `app_it` and `mary_o_it` | Keep held-body/module-file witnesses; do not reopen the gate migration | [I4](../engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference) |
| AI sees wrong live room (✅ Plant fixed 2026-10-12, see the owner) | At `ed0d1ccd`, Void Lizard, Swallow and Plant used global `PrimaryPlayer`. Now modules read `ActorTarget`, whose selector filters to same `LiveRooms` instance; Plant still uses a global player | Remaining Snake Plant room filter is absent; its gameplay impact needs simultaneous-room witness | One player in room A, plant in B at overlapping coordinates; module targets remain actor-room scoped | [control](../engine/control-authority-and-ai-policy.md) |
| Scripted pipe borrows combat lock (✅ fixed 2026-10-12, see the owner) | `game/ambition_demo_mary_o/src/pipe.rs:253-264` writes `recoil_lock_timer`, clears to zero on exit | Definite overlapping-writer hazard; actual collision requires overlapping sources | Apply independent recoil during transit; exit must not erase it | [control](../engine/control-authority-and-ai-policy.md) |
| Pose marker can be stolen (✅ fixed 2026-10-12, see the owner) | Mary-O Plant/Pipe and mount path use `PoseOwnedExternally`; independently insert/remove same boolean | Source-overlap can release somebody else's hold | Overlap two pose sources, remove one, confirm pose remains owned | [movement](../engine/unified-movement-kernel.md) |
| Void fields cross camera layers (✅ fixed 2026-10-12, see the owner) | `crates/ambition_render/src/void_field.rs:158-191` projects all visible effects without camera/source `RenderLayers` check | Definite omitted layer filter; external visibility impact depends on view arrangement | Two overlapping camera projections on disjoint layers; include wards and multiple live rooms | [R7](../engine/render-animation-and-vfx.md#r7---full-screen-field-visibility-must-match-the-cameras-render-layers) |
| Aura binding unreconciled (✅ fixed 2026-10-12, see the owner) | `game/ambition_demo_mary_o/src/void_look.rs:65-88` scans bodies by string for each visual and only adds effects | Definite repeated search and incomplete source retraction on form changes | Switch one body form, remove/rebind visual, ensure exactly its aura goes away | [R6](../engine/render-animation-and-vfx.md#r6---reconcile-semantic-screen-field-sources-by-visual-ownership) |
| Barrier singleton/geometry (✅ fixed 2026-10-12, see the owner) | `crates/ambition_render/src/rendering/actors/rigged/part_set.rs`, Mary-O `cosmic_barrier.rs:150-227` | Definite single `PartSetRequest` slot, destructive owner removal and limited geometry | Two independent readers plus fifth part/second page/rotation; hide/read lifetime | [part sets](../engine/semantic-part-rendering-and-ragdolls.md) |
| Cosmic grants are primary-only (✅ fixed 2026-10-12, see the owner) | Mary-O `cosmic.rs:165-210` after selected form applied to driven subject | Definite `With<PrimaryPlayer>` recipient mismatch | Non-primary selected/controlled body enters form | [form combat](../engine/combat-model.md) |
| Cosmic freeze policy mismatch (✅ decided and measured 2026-10-12: her own stop, see the owner) | Mary-O `cosmic.rs:180-190` clamps attacker timer; `ambition_combat/src/impact_hitstop/mod.rs:74-125` reads resolved connects, excluding Contact | Definite implementation not equivalent to global eligible-connect freeze; desired behavior needs product choice | Same eligible connect with ordinary/cosmic attacker; verify global expiry and untouched victim; separately test Contact exclusion | [combat](../engine/combat-model.md#a-forms-shorter-global-hitstop-belongs-at-hit-resolution) |
| Portal render-piece churn | `crates/ambition_portal2d_presentation/src/{far_side,visuals}.rs` wholesale despawn/recreate and `PortalClipMaterial` adds | Definite repeated creation, **not** a proven leak or bottleneck | Stable 600-frame scene and measured material/mesh/entity/draw-call deltas vs control | [R8](../engine/render-animation-and-vfx.md#r8---portal-fragment-lifetime-and-measured-render-churn) |
| Durable save row key split (✅ fixed 2026-10-12, see the owner) | `crates/ambition_extension_host/src/save.rs:56-103` merge keys outer `SavedRow.key`, adoption uses embedded `StateSchema.key` | Validation hole for conflicting and duplicated input, not a demonstrated corrupt production save | Two crafted contradictory/duplicate rows, active and dormant module versions | [state save](../engine/extension-state-and-execution.md#saved-row-identity-validation-new-durable-session-customer-2026-10-11) |
| Title visual provider coupling | `game/ambition_app/src/app/title_screen.rs:79-86`, `ambition_game_shell/src/experience.rs` | Route-name switch prevents declared per-provider art without host changes; default Hub works | New route with declared theme, launcher draws it without host edit | [UI](../engine/ui-localization-and-accessibility.md#launcher-theme-quality-and-provider-composition) |
| Title quality-live cleanup (✅ fixed 2026-10-12, see the owner) | `title_screen.rs:153-178,268-272` early return on disabled budget, teardown only when launcher closes | Existing background remains after dynamic budget disable; resolution tier not in scene identity | Toggle enabled->disabled->enabled and scale while launcher remains active | [UI](../engine/ui-localization-and-accessibility.md#launcher-theme-quality-and-provider-composition) |
| Duplicate sprite logical keys (✅ fixed 2026-10-12, see the owner) | `crates/ambition_sprite_sheet/src/sprite_tree.rs:70-104` sheet scanner preserves duplicates, companion scanner silently drops duplicates | Source-validation ambiguity when duplicate logical names occur | Two dirs publish one logical key; ensure refusal cites both paths or explicit overlay rule | [reload](../engine/content-generation-and-reload.md#sprite-source-identity-and-live-presentation-refresh) |
| Running sprite edit not applied | `crates/ambition_sprite_sheet/src/baked_sheet_rons.rs:12-23` and decoded rig caches | First-read caching means the successful no-recompile publish is **not** live hot reload | Edit and activate sprite while visible without process restart/relink | [I3](../engine/fast-iteration-implementation.md#i3---coordinated-generation-publication-and-local-reload) |
| Tick-rate hitstop literal | `ambition_combat/src/impact_hitstop/mod.rs:50-62` uses `60.0` converting seconds to ticks | Real cross-rate incompatibility **already** tracked by P0-3; **not** a new review queue row | 100 ms freeze at 30/40/60/120 Hz | [rate audit](../engine/simulation-tick-rate-compatibility.md#p0-3-convert-hitstop-using-the-selected-rate) |

## Explicit corrections to earlier review claims

1. **At `ed0d1ccd`, Mary-O late fire was gate bypass, not necessarily a
   dropped attack.** In the fixed-tick schedule the combat move trigger
   followed the old late writer. I4 S1-S3 have since repaired the admission
   path: loaded module fire is pre-gate and Void Lizard's hop submits the
   `ambition.motion.velocity` port, rather than directly editing kinematics.
   The completed tests are receipts; do not ask an architecture agent to
   implement this old migration again. The separate shot-end rollback bug
   remains queued.
2. **Cosmic hitstop intent is a choice, not established by the function name.**
   `ImpactHitstop` ignores contact damage intentionally. A modifier for
   eligible attack connects must not let repetitive contact attrition freeze
   the entire match. Personal recovery and world pause are distinct outputs.
3. **Portal render churn does not prove a leak.** Material assets can be
   reclaimed by engine asset lifetime policy; count live handles/residency
   before claiming growth. The fragment objects are presentation derivatives,
   not rollback state, and may be safely reconstructed where cheap.
4. **No-recompile sprite publishing is delivered.** The remaining live
   sprite-refresh feature is independent and should remain P2. Do not undo
   the source-tree/compiled-binary separation as part of the fix.
5. **The title cursor's filtered available rows are consistent with shell
   routing.** `title_screen.rs:173` and the shell cursor registration both
   filter unavailable entries; this is not a newly found selection bug.
6. **The 60 Hz world-hitstop conversion has an existing plan.** Reference
   `simulation-tick-rate-compatibility.md`; do not introduce a duplicate
   rate authority or reopen the whole rate migration just for cosmic form.

## What was not verified

- No production `cargo test`, end-to-end Mary-O/GNU-ton play session, GPU
  capture, or generated-assets QA was performed. The archive has missing
  submodule contents and no confirmed safe Cargo target bind mount in this
  review environment. Failing fixtures are **proposed**, not passing.
- Portal-pair collisions in the *current* player experience depend on two
  simultaneously active same-room GNU-ton instances; the data join is bad
  regardless, but no shipped level was shown to spawn two of them.
- The exact authority for the GNU-ton completion policy (one arena versus
  one exit per boss) is a design decision. Do not hardcode 'all bosses dead'
  until the authoring contract states which encounters own each exit.
- Texture/mat cache reclamation and portal frame-time significance need
  profiling on the intended hardware/quality target.
- The user-visible title quality toggle must be exercised in the assembled
  host; the early-return and retained-root lifetime mismatch is established
  by code, but the current settings UI may only apply quality changes on
  screen entry.
