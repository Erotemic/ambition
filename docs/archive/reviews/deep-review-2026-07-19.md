# Deep review — 2026-07-19

**Scope:** full-repo audit (Jon's request): bugs introduced since the 2026-07-16
recon, plans claimed complete vs reality, decomposition/elegance opportunities,
compile-time and code-size wins, agent navigability. Conducted by fable with
seven parallel audit passes (plans-vs-reality, recent-commit bug hunt, crate
graph/compile, bifurcation sweep, smell-log triage, determinism/GGRS risk,
navigability) plus the full headless test suite.

This is the evidence record. The executable queue derived from it is
[`../../planning/tracks.md`](../../planning/tracks.md); smells were logged to
`dev/journals/code_smells.md` the same day. Fixes landed during the review
itself are marked **[FIXED in this pass]**.

---

## 1. Regressions found (and their root cause)

**Root cause pattern:** commit `9754a79d9` ("Sprite work", 2026-07-19 01:43) was
authored from a checkout predating the evening-of-07-18 fix commits and
clobbered them on land. Its sibling `61eb57825` hand-reconciled the same hazard
for `basic_presentation.rs`; the combat/render files got no reconciliation.
The commit's legitimate payload (directional `SlashPose`, 3-row slash sheet)
was fine and is preserved.

1. **P0 — melee knockback units reverted** [FIXED in this pass].
   `crates/ambition_combat/src/moveset/mod.rs` re-authored melee knockback as
   `HitboxKnockback::FeelScale(volume.knockback)` and dropped `kb_growth`,
   undoing `2c465cc77` — authored px/s values (goblin swipe: 120) were again
   interpreted as ~120× multipliers over the standard feel vector (~53,000 px/s
   launches). Re-applied as `LaunchSpeed { base, growth }` with the regression
   assertion restored in `moveset/tests.rs`.
2. **P0 — `cargo test --workspace` red** [FIXED in this pass]. Two independent
   causes: (a) the reverted moveset test no longer type-checked
   (`(hb.damage, hb.knockback)` vs `(i32, f32)`); (b)
   `ambition_platformer_primitives` failed to compile standalone — 26 `KeyCode`
   errors — because `developer_hotkeys.rs` (from `a606dfb43`) needs
   `bevy_input_focus`, which only co-built siblings unified in. Same trap
   `ambition_game_shell`'s manifest already documents; fixed the same way.
3. **P1 — slash animation re-coupled to the sim tick** [FIXED in this pass].
   `slash_visuals.rs::animate_slash` reverted to `Res<WorldTime>` while every
   sibling uses `ambition_time::PresentationTime` (undoing `0693e5e88`); under
   the GGRS host that ties animation speed to display refresh. Re-applied.

**Process lesson** (for `dev/journals/lessons_learned.md` discipline): a
vaguely-messaged commit from a stale base silently reverted two same-day FIX
commits. Cheap guard: `git diff HEAD..@{u}`-style pre-land review, or simply
rebase before committing multi-hour-old work. The deleted debt-census tests
(see §3.3) would not have caught this; the workspace test job did — but only
if someone looks at it.

## 2. GGRS rollback gaps (post-ADR-0027 debt)

The registration set in `crates/ambition_runtime/src/rollback/mod.rs` is
genuinely comprehensive (body/actor/combat/brain/projectile/encounter/portal/
mount families, MapEntities, 40-entry cleared-on-rollback message list). The
gaps are the exceptions:

1. **Melee strike volumes are outside the rollback contract**, and
   `MovePlayback.live_boxes` is CLONED across rollback (pre-GGRS design assumed
   restore rebuilt it empty). After LoadWorld a live slot can name a dead
   entity for an active window → the `(inside, Some(live_slot))` arm skips the
   respawn → strikes silently whiff during resim.
   `FIXME(ggrs-live-boxes)` now marks the field.
2. **Unregistered sim-mutated state** (each is a resim/desync bug when hit):
   `WornEquipment` (armor rows spent by `resolve_body_hit`; `WornCharacter` IS
   registered — this is an oversight, not policy), `SwitchActivationQueue`
   (cross-frame Vec: produced in GameplayEffects, drained next frame in
   EncounterSimulation — non-empty at the save boundary, neither registered nor
   cleared), `BreakableFeature`/`StandTimer`/`RespawnTimer`/`HazardFeature`,
   portal gun runtime (`PortalTransitCooldown`, `PortalEmission`, `PortalShot`,
   `PortalGun`), `BombFuse`/`GravityGrenadeFuse`/`HeldProjectile`/`VortexWell`/
   `FallingHazard`/`FallingChest`, and demo content composed into the app shell
   (`BallDash`, `BallDashInput`, `SanicActState`, `MaryOLevelState`,
   `FlagSequence`).
3. **The forcing function is gone:** `50ab3b4bd` deleted the component/resource
   debt-census tests with no replacement, so nothing forces new sim state to be
   classified rollback/derived. The list above will silently grow. The schema
   fingerprint guards registration *conflicts*, not *coverage*.
4. **Sim-scheduled `Local` state:** `possession_trigger_system`'s
   `hold_timer`/`prev_down_interact` (`abilities/traversal/possession.rs`)
   gate the registered `PossessionState` but cannot be rolled back.
   (`advance_sim_tick`'s `first_step` latch and the puppy-slug Name counter are
   benign — checked.)
5. **Order-determinism under entity recreation** (scanner-invisible):
   `combat/targeting.rs:285` breaks exact-distance target ties by raw `Entity`
   comparison — raw ids are NOT stable across GGRS entity recreation; tie-break
   by `SimId`. Slot requests are built in raw query order
   (`features/ecs/actors/update.rs:255-320`) and only distance-sorted, so
   equal-distance pairings ride unstable iteration order into the registered
   `CombatSlotsRes`; sort requests by `actor_id` first.
6. **Latent:** `FactionRelations`/`FriendlyFire` are runtime-mutable combat
   resources, unregistered — safe today (only `Default` writes), a desync the
   day any stealth/alliance system mutates them mid-session.

## 3. External-effect quarantine exposure (work-list for the quarantine track)

Mechanism at HEAD: sim systems emit effect *messages*; `clear_message_on_rollback`
empties them at LoadWorld — that handles the **abandoned future** but nothing
gates **re-emission during resim**. Only `gameplay_trace` is quarantined
correctly (recorders `.run_if(simulation_pass_is_authoritative)`, flush in
`PostUpdate`) — **it is the pattern to copy**.

- **Audio:** ~20 sim emit sites via `SfxWriter::write`; consumer
  `audio_play_sfx_messages` runs `Update.after(CoreSimulation)` with no replay
  gate → duplicate playback on resim (visible builds only; headless drops the
  audio feature, so SyncTest never sees it).
- **VFX:** `VfxMessage`/`EffectRequest`/`ExplosionRequest`/`DebrisBurstMessage`/
  `FireworksRequest` written pervasively from sim; consumers unaligned to
  confirmed frames → duplicate particles.
- **Persistence:** `autosave_sandbox_save` triggers on `SandboxSave.is_changed()`
  — GGRS restore trips change detection, so the next `Update` writes
  **speculative state to disk**; `save_settings_on_change` analogous.
- Non-findings: no analytics/achievements subsystem; host has no sim-path I/O;
  no ambient RNG; no par_iter in sim; wall-clock only in dev/tests.

## 4. Plans vs reality

Verified solid: GGRS DONE, PreparedContent DONE (ADRs 0026/0027 match code),
encounter convergence (minus one dead test citation), Mary-O engine-seams list,
netcode.md (the model "current architecture" doc), unified-movement-kernel /
unified-actors / combat-model / collision-and-ccd honest self-grades,
character-actions.md precise (P1/P5/P6 genuinely absent).

Broken/stale (fix = small edits now; rewrites are queued tasks):

- **Docs BEHIND reality:** `room-transition-loading.md` says "not started" but
  Phases 1–4 landed 2026-07-17 (`LoadBarrierRef` minted at
  `room_transition_loading.rs:320`, presentation commands, neighbor prefetch);
  its gap analysis is inverted. `shell-vanity-sequence.md` executed the same
  night it was written (only VC5 fade-in open). Sanic remaining-list stale on
  2 of 5 bullets (ring economy + badnik enemy loop landed; drop-on-hit,
  goal/HUD/act, route oracle remain). Roadmap's "Immediate room-loading
  integration" reads as pending; the room-lifecycle planner half of
  "transactional migration" landed (`RoomConstructionPlan`, one artifact for
  startup/reset/transition/reload/reconstruction).
- **GGRS evidence rot:** encounter-orchestration.md:227 cites the deleted
  `ambition_runtime::snapshot::tests::restore_preserves_an_active_encounter`
  as E11's exit evidence (invariant survives at `rollback/codecs.rs:1871`);
  architecture.md §5 still narrates snapshot-restore room staging;
  headless-verification.md predates the sim-harness extraction;
  immutable-content §7.5–7.6 + Phase 5 cite deleted snapshot files;
  **fighter-brain FB6 is unimplementable as written** (rollouts via deleted
  `snapshot.take/restore`) — tracks #5 already demands the design correction.
- **Dead pointers:** frame-awareness.md / slower-light.md claim AJ13/AJ14 live
  in tracks.md (they live in the archived fable-demo-plan);
  boss-system.md points at a decomposition.md "E6" that no longer exists;
  decomposition.md claims no host→actors guard exists but
  `engine.toml:279/291` now enforce exactly that.
- **Structural:** demo remaining-lists are quadruplicated (status / tracks /
  roadmap matrix / demo docs) and drifted independently — single-source them
  in the demo docs. boss-system.md's surviving rules belong in boss-design.md.
  encounter-orchestration.md is a completion ledger living in the active tree.
- **KB linter contract broke:** `check_agent_kb.py` expects evidence markers
  (boss-validator, workspace-members, module-size, cc3) that the 2026-07-18
  status.md rewrite dropped, plus 17 inline-test marker disagreements — the
  linter has been red since; prune the stale marker expectations or restore
  the markers deliberately.

## 5. Bifurcation & unification (ONE BODY, ONE PATH)

Confirmed clean: melee end-to-end, `resolve_body_hit`, movement FX
(`emit_movement_fx`), faction targeting (`effective_faction`), input folding
(`ControlFrame`/`MenuControlFrame`), audio layering, load/presentation seams,
`PrimaryPlayer` usages (all home-avatar policy or controlled-subject
resolution).

Live forks / unification candidates (logged in code_smells.md, queue-worthy):

1. Projectile hit-detection: two victim loops (player/actor) with knockback
   asymmetry and a self-documented parry-term drift — unify onto the
   `hitbox/mod.rs:203` single-loop pattern. The clearest remaining fork.
2. "Body was struck" feedback keyed on `is_player` at two attacker-side emit
   sites with byte-identical payloads; death feedback per victim-kind at three.
   One victim-side feedback seam keyed on attack spec + victim feel profile
   also delivers Jon's "each attack binds its own VFX/SFX".
3. Menu stacks: `ambition_menu`/`ambition_settings_menu` (+3 app-side stacks)
   reimplement focus/list nav beside `ambition_ui_nav`.
4. Cutscene + encounter each hand-roll an input-freeze beside `GameMode`.
5. Room-transition staging (`world/rooms/stage.rs`) parallels `ambition_load`
   barriers — already the room-transition plan's stated direction.
6. Read-model leak: `portal_presentation/gun_visuals.rs` queries sim
   `BodyKinematics` directly (pose views exist).

**Named-content-in-core (inversions to schedule; eviction ruling M23 already
authorizes the first two):** `ambition_items::Item` closed enum (24 variants,
per-variant sprite paths) → provider-registered catalog;
`deep_dream_strength` knob baked into render/settings; the whole
`abilities/thrown/puppy_slug_gun.rs` named module inside `ambition_actors`
(generic mechanism: summon faction-allied minion) with a hardcoded
`"puppy_slug_gun"` row in `action_set/mod.rs`.

## 6. Crate graph, compile time, code size

- **Top compile-time defect:** `ambition_menu` and `ambition_menu_kaleidoscope`
  declare `bevy = "0.18.1"` with FULL default features; feature unification
  drags pbr/gltf/animation/audio/gilrs/hdr/ktx2/reflect_auto_register/winit
  into EVERY build (headless, web, Android, CI), nullifying the trimmed sets in
  the other 44 manifests. Two-manifest fix + make the kaleidoscope dep
  `optional = true` behind the existing `kaleidoscope_menu` feature.
- **Graph:** no production cycles; longest chain depth 11; the serialized spine
  engine_core → characters → combat → actors → sim_view → runtime → host →
  facade → content → app ≈ 192k LOC. `ambition_actors` out-degree 27 (includes
  menu/settings_menu/ui_nav/dev_tools/cutscene/dialog — the role anomaly).
  Tier tensions: sprite_sheet (Tier 0) → world/persistence/interaction (the
  interaction dep has ZERO path references — free deletion);
  `ambition_world → ambition_time` unused (free deletion); actors→ui_nav is
  forwarding-only; dev_tools is a normal dep of sim_view/render/runtime/actors
  vs "development-only" in architecture.md — needs a doc amendment or a
  vocabulary split.
- **Role-driven actors evictions (carve-doctrine-safe):** `menu/` (812 LOC
  product Map-tab UI; zero in-crate consumers; the sole reason actors depends
  on the menu crates) → game-side; `affordances/` (2,125 LOC observation
  vocabulary) → invert via the `ControlPrompt` pattern (this removes
  touch_input's biggest reason to name actors — the documented unsettled
  question); `character_sprites/` anim ladder (~2k) → sim_view/sprite_sheet;
  `music/`+`audio/` adapters are provider-identity material sitting in the
  heart.
- **Feature machinery gating nothing:** `headless` (zero cfg sites anywhere),
  actors' `rl_sim`/`static_sfx_bank`/`kaleidoscope_menu` stubs, ldtk_map's
  unreachable `dev_hot_reload`; `asset_manager/bevy` enabled by 100% of
  consumers. Runtime hard-enables actors `input`/`portal_ldtk` — the optional
  machinery is ballast; acknowledge always-on or make it real.
- **Hygiene:** no `[workspace.dependencies]` (bevy pinned 46×; ron 0.11/0.12
  and thiserror 1/2 drift already real). `ambition_inventory_ui` (149 LOC, one
  consumer) is the weakest crate — fold into menu or items when convenient.
- **Reflect is healthy** (28 derives, none hot); only 2 real build.rs. Largest
  files are known monoliths + 9/20 test files; dead code:
  `actors/effects/mod.rs`, `actors/debug_label.rs` (zero consumers).

## 7. Navigability

- **[FIXED in this pass]** `.agent` index generator crawled untracked/ignored
  trees — a stale June-12 full-repo snapshot (`.tmp-*-stage/`) contributed
  2,034 refs and phantom-crate packets (`ambition_sandbox`,
  `ambition_platformer_runtime`) that outranked live owners with NO staleness
  marker ("rollback snapshot" top hit was a deleted file in a dead crate).
  Generator now filters through `git ls-files --cached --others
  --exclude-standard`; stage dir deleted; phantom packets deleted; index
  regenerated; `agent_query.py` now prints a loud stale-index banner and a
  machine-local `generation_stamp.json` records provenance ("Generated at:
  unknown" bug fixed).
- **[FIXED in this pass]** AGENTS.md's flagship ONE-PATH paragraph routed
  melee through `spawn_melee_strike` — deleted 2026-07-15. Now names the live
  moveset seam.
- MODULES.md coverage is 100% and current (a real strength). Doc links pass.
- Two invariants effectively unreachable on the cold-start trail: the rustfmt
  mod.rs cascade and required-components-silently-skip → now in
  `docs/concepts/invariants.md`.
- `dev/journals/index.md` misses 8 entries incl. lessons_learned/code_smells
  themselves.
- ecs-inventory regeneration required `tree_sitter_rust`, installed nowhere on
  the dev machine (regen-invariant violation) — a repo-root `.venv` now carries
  it; consider adding it to `run_developer_setup.sh`.

## 8. Jon's reported breakage (untracked/jonnotes-FIXES.md), verified at HEAD

| Item | State at HEAD |
|---|---|
| NPCs infinitely respawn | Placement-pin half FIXED (`2f8371434`); adjacent hole remains: room reset full-heals every actor with no `RespawnPolicy` consult (`reset_to_spawn`, code_smells 2026-07-19 entry) |
| Swing VFX black square | Actively worked: `9754a79d9` landed `SlashPose` + 3-row slash sheet + regenerated `robot_slash` manifest; needs visual confirm |
| Attack-specific VFX/SFX | Open; elegant home = the victim-side feedback seam (§5.2) |
| Morph ball sprite not swapped | Open; render `morph_ball` path exists but worn-sprite swap doesn't; generalize as a transform/worn-identity presentation rule, not a special case |
| Shrine + glider sprites broken | Open (shrine mechanic itself is a TODO stub — `actors/shrine.rs:12`) |
| Kernel-guide NPC patrol-around-home brain | Open; brain vocabulary supports patrol; needs an authored home-base patrol policy |
| Possession-aware dialog | Open; dialog identity comes from stable dialogue identity (`187295de9`) but speaker≠controller adaptation is unmodeled |
| Sanic via `AMBITION_START_CHARACTER`: wrong moveset + can't move | Open; ActionScheme/moveset are per-character data, so likely the sanic character row grants defaults it shouldn't + missing control hookups outside the demo app; needs a trace |

## 9. What was landed during this review

- Regression re-fixes: knockback `LaunchSpeed`, moveset test guard,
  `PresentationTime` in `animate_slash`, `bevy_input_focus` in
  platformer_primitives.
- `FIXME(ggrs-live-boxes)` + truthful GGRS comments in `moveset/mod.rs`;
  stale `restructuring-blueprint` breadcrumbs repointed.
- `.agent` generator git-ls-files filtering + generation stamp + stale-index
  banner; phantom packets and the `.tmp-*-stage/` snapshot deleted; index
  regenerated at HEAD.
- AGENTS.md melee-seam correction; vision.md §3 reconciled with the settled
  no-carve ruling; planning pointer fixes (see tracks.md).
- code_smells.md triaged: 6 entries closed with evidence, statuses updated
  (alpha-clobber grew 139→373), ~11 new entries appended.
- Root hygiene: fossils archived, overlay residue removed, profiling scripts
  committed (they were referenced by committed docs but untracked).
