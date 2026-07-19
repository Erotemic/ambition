# Tracks — current executable queue

This file is the live queue, not a completion ledger. The completed July 15–16
architecture campaign is summarized in [`status.md`](status.md); the 2026-07-19
deep-review evidence behind the newer tracks is
[`../archive/reviews/deep-review-2026-07-19.md`](../archive/reviews/deep-review-2026-07-19.md).

**Executor grades** (vision §7, restored): **[fable]** genuinely hard
design/kernel work; **[opus, fable-specced]** the spec in the named doc IS the
design — execute it verbatim and STOP at the first factual mismatch;
**[opus]** well-bounded engineering with shape + exit criteria;
**[sonnet]** mechanical with exact file lists. An agent may not deviate because
a step is hard or looks unnecessary; deviation is legitimate only when the code
contradicts the plan's factual assumptions — surface the mismatch.

**Standing fable-hard list:** the FB6 rollout redesign (track 6); the
falling-sand solver correctness pass (pooling/termination — Jon: "getting
falling sand to work right is part of the engine"); boss-fight *quality*
grammar beyond validation (boss-design.md's open iteration loop).

## 0. Pay down the GGRS correctness debt — **LARGELY LANDED 2026-07-19**

Spec: deep-review §2. Landed:

- registered the unregistered sim state: `WornEquipment`, `SwitchOn`,
  `SwitchFeature`, `SwitchActivationQueue`, breakable/hazard/respawn/stand
  timers, portal-gun runtime (`PortalTransitCooldown`/`PortalEmission`/
  `PortalShot`/`PortalGun`), and `RoomVisual`;
- `MovePlayback.live_boxes` is now VALIDATED against the world every tick, so a
  cloned cache slot naming a dead entity is dropped and the window respawns its
  volume. Mechanism-agnostic: it fixes the GGRS clone case and any future path
  that despawns a volume out from under a playback;
- `possession_trigger_system`'s `Local` hold/edge state moved onto the
  registered `PossessionState`;
- target selection no longer compares raw `Entity` (candidates are sorted by
  `SimId`, ties go to canonical order); slot requests are sorted by `actor_id`
  before `assign_slots`;
- **the coverage forcing function exists**:
  `game/ambition_app/tests/rollback_coverage.rs` boots the real sim and asserts
  every component ON a simulated entity is registered, derived, or waived with a
  reason. It is computed, not a checked-in ledger, and it found the last two
  gaps (`SwitchFeature`, `RoomVisual`) on its first run. NOTE it checks entity
  COMPOSITION, not system access — Bevy 0.18 does not expose per-system
  `FilteredAccessSet` publicly — so **resources still need review by hand**.

Remaining:

- the demo-content state composed into the app shell (`BallDash`,
  `BallDashInput`, `SanicActState`, `MaryOLevelState`, `FlagSequence`) — these
  live in `game/` crates and need the content-side registration seam;
- `FactionRelations`/`FriendlyFire` are unregistered and latent-safe (only
  `Default` writes today); register them when anything mutates them in-session;
- the exit oracle below.

**Exit:** a sync-test run that lands a melee hit, spends armor, flips a switch,
and breaks a brick across a forced rollback window stays checksum-identical.

## 1. Quarantine external effects to confirmed GGRS frames — [opus, fable-specced]

Unchanged goal; the exposure map is now precise (deep-review §3): ~20 sim-side
SFX emit sites replay-unguarded, all five VFX message families likewise, and
`autosave_sandbox_save` writes speculative state because GGRS restore trips
`is_changed()`. **`gameplay_trace` is already quarantined correctly**
(`.run_if(simulation_pass_is_authoritative)` + `PostUpdate` flush) — copy that
pattern; do not invent a new mechanism.

- classify audio, VFX, save writes, and host I/O per the exposure map;
- buffer effect intents by GGRS frame; release at the confirmed boundary;
- discard abandoned predicted intents on rollback (the message-clear list
  already does this half);
- gate the autosave/settings writers on confirmed frames, not change detection;
- prove a forced sync-test rewind emits each accepted effect exactly once.

**Exit:** repeated rollback cannot duplicate an external effect, and a Matchbox
transport can be attached without changing simulation systems.

## 2. Build-graph hygiene (compile-time wins) — **LANDED 2026-07-19**

Deep-review §6. Landed:

- `ambition_menu` and `ambition_menu_kaleidoscope` now declare
  `default-features = false` with minimal feature sets (`bevy_ui` +
  `bevy_picking`; `bevy_pbr` + `bevy_ui`). **Measured result:** the
  `ambition_actors` build graph dropped `bevy_pbr`, `bevy_gltf`,
  `gltf_animation`, `bevy_audio`+`vorbis`, `mesh_picking`, `smaa_luts`,
  `tonemapping_luts`, `ktx2`, `sysinfo_plugin`, and `bevy_light` — the whole 3D
  stack that plain `bevy = "0.18.1"` was pushing into every build via feature
  unification, headless and CI included;
- `ambition_menu_kaleidoscope` is optional on the app, wired to the existing
  `kaleidoscope_menu` feature (its module is now cfg-gated to match), so
  bevy_lunex + bevy_rich_text3d leave non-cube builds entirely;
- `[workspace.dependencies]` adopted for serde/ron/thiserror across 30
  manifests, ending the ron 0.11-vs-0.12 split in our own tree. **Honest
  result:** the duplicate COMPILES remain, because ron 0.12 comes from
  `bevy_animation` and thiserror 1 from `bevy_ecs_ldtk` — transitive, not ours;
- deleted the dead `ambition_world → ambition_time` edge and made actors→ui_nav
  an optional feature-conduit. **Correction:** `sprite_sheet → interaction` is
  NOT dead (8 real path references) — the deep review was wrong there;
- deleted the vestigial `rl_sim` feature chain (actors' and the facade's copies
  gated nothing once the RL surface moved to `ambition_sim_harness`; the app's
  switch is the real one). `headless` stays: it is an intentional empty
  composite whose value is what it leaves off;
- **the trim exposed three crates that only ever compiled by accident**, because
  the untrimmed dep was donating features workspace-wide:
  `ambition_platformer_primitives` (needs `bevy_input_focus` for `KeyCode`),
  `ambition_game_shell` and `ambition_load_presentation` (need a windowing
  backend for the winit that `ui_api` pulls). All three now declare what they
  use — see `dev/journals/lessons_learned.md` (2026-07-19) for the pattern to
  expect on the next trim.

Remaining (own pass): `ui_api` is a bundle that pulls `bevy_animation` (unused
here) into ~6 crates; replacing it with explicit feature lists would drop that
plus ron 0.12. And `bevy` itself is still pinned in ~46 manifests — a
workspace-dependency conversion is mechanical but wants its own review, since
the per-crate feature sets legitimately differ.

## 3. Close Super Mary-O level 1 — [opus]

Engine seams proven (pickups/equip, grown form, ranged powerup, bricks, crony
stomp, flag, clock, tally, cyclic restart). The remaining customer work is the
list in [`demos/super-mary-o.md`](demos/super-mary-o.md) — **that doc is the
single source; do not copy the list back here** (it had already drifted: this
queue had dropped "enters the secret" from the scripted-run gate and omitted the
post-acceptance levels bullet).

**Exit:** visible and headless customers use the same provider, body, item, and
level state with no Mary-O-only engine path.

## 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. The remaining work is the list in [`demos/sanic.md`](demos/sanic.md),
which already declares itself the single source — **refer, never copy** (these
are the copies that "drifted independently — again").

Do not absorb movement/contact work owned by another active campaign.

## 5. Build the provenance + three-origin `ConstructionPlan` vertical slice — [opus, fable-specced]

Unchanged (verified genuinely open: no `SpawnOrigin`/`RecipeId` at HEAD). Note
the room-lifecycle planner half already landed (`RoomConstructionPlan` is one
artifact for startup/reset/transition/reload/reconstruction) — build on it, do
not re-plan it. Spec: `engine/immutable-content-and-transactional-construction.md`
Phase 3:

- add explicit `SpawnOrigin` and internal stable `RecipeId`;
- plan one authored placement, one provider-staged actor, and one
  runtime-dynamic family through a common pure `ConstructionPlan`;
- validate identities and relationships before mutation;
- use the same recipes for ordinary spawn and reconstruction;
- remove `SimId` parsing as provenance authority for the selected family;
- prove deterministic plan dumps and planned-versus-committed roster parity.

**Exit:** failed planning leaves the active world untouched; all three origins
share one inspectable planner/executor; the runtime-dynamic family reconstructs
without inferring its recipe from an id string.

## 6. Correct the fighter-rollout design before FB6 — [fable]

Unchanged, and now urgent-if-touched: FB6 as written depends on the DELETED
snapshot engine (`snapshot.take/restore`) and is unimplementable at HEAD.

- prefer a fixed work budget over a wall-clock cutoff (or make decisions
  recorded external inputs);
- rollouts need a hypothetical state reconstructed solely from allowed
  `Perceived` facts, or a deliberately limited perceived-state forward model —
  now necessarily built on GGRS-era machinery.

**Exit:** the determinism and no-cheat contracts are explicit enough that an L3
implementation cannot accidentally violate either one.

## 7. Role evictions from the sim heart — [opus]

Deep-review §6 (all carve-doctrine-safe; the settled "no size-driven carve"
ruling stands — these are ROLE moves, each with a named destination):

- move `ambition_actors/src/menu/` (product Map-tab/settings-IR content) to the
  game side; drop actors' menu/settings_menu edges;
- invert `affordances/` behind a sim_view-style read model (the `ControlPrompt`
  precedent — decomposition.md names it the preferred direction); this removes
  touch_input's largest reason to name actors;
- migrate the `character_sprites/` anim pick-ladder toward
  sim_view/sprite_sheet (its own doc says lower authorities live there);
- ~~delete the compat facades~~ **PARTIALLY DONE 2026-07-19**: `effects/mod.rs`
  (never even declared in `lib.rs`) and `debug_label.rs` (zero consumers) are
  gone. `host/` is NOT dead — actors' own settings model consumes
  `crate::host::windowing`; the deep review was wrong there. The ~73
  `pub use ambition_*` lines remain [sonnet-able, needs consumer repointing];
- execute the already-ruled content evictions still outstanding (M23 / recon
  accepted #1): `ambition_items::Item` closed enum → provider-registered
  catalog; `deep_dream_strength` → content-owned presentation knob;
  `puppy_slug_gun.rs` → parameterized summon-ally ability + content data.

**Exit:** actors' out-degree drops (no menu/settings_menu/ui_nav edges); the
oracle "add a character/item without editing core" holds for items.

## 8. Combat unification batch — [opus] (first item LANDED 2026-07-19)

Deep-review §5; BIFURCATION entries in code_smells.md 2026-07-19:

- ~~collapse the projectile player/actor victim loops~~ **DONE**: one victim
  loop over every body, `Has<PlayerEntity>` picking only payload policy. Killed
  three drifts with the fork — actors now receive knockback, the player side
  gained the grudge term, and vulnerability became feedback-only for both (§A2).
  The vulnerability cluster is deliberately `Option` so simple feature bodies
  are not silently dropped from the query;
- build ONE victim-side hit/death feedback seam keyed on the attack/volume
  spec + the victim's feel profile; delete the two `is_player` attacker-side
  emit blocks — this is also where Jon's "each attack binds its own VFX/SFX"
  lands (authored effect identity on the volume/move spec). **Surveyed
  2026-07-19, still OPEN; the design is now PRE-SOLVED — execute
  [`engine/combat-model.md`](engine/combat-model.md) §8 CM8, do not re-derive
  it.** Headlines: the gap is wider than "two emit blocks" (no effect identity
  crosses `Hitbox`→`HitEvent` at all, so ALL move audio/visual fires at a
  TIMESTAMP, never on CONTACT); there are THREE payload forks, not two, one of
  which is a live bug (enemy-vs-enemy contact plays `PLAYER_DAMAGE` + the red
  "player got hurt" burst); and a plain delete of the attacker-side blocks
  would REGRESS player feel, because the rich payload exists only there;
- ~~fix the portal gun-visuals `BodyKinematics` read-model leak~~ **DONE
  2026-07-19**, and it was hiding a real bug. `ambition_portal_presentation`
  now reads a host-published `PortalBodyView` (pos/size/facing) on two host
  seams — `PortalSceneBody` (whose sprite decomposes) and the new
  `PortalAffordanceBody` (who operates the portals). The crate no longer names
  `BodyKinematics`, `PlayerEntity`, or `PrimaryPlayer` **at all**.
  **The bug:** the affordance body is tagged from `ControlledSubject`, not
  `PrimaryPlayer` — so while possessing, the held gun and the disorientation
  indicator followed the HOME AVATAR while the fire adapter already resolved
  the shot from the controlled body holding the gun (its own test:
  `portal_fire_origin_comes_from_the_holding_controlled_body`, explicitly "no
  fallback" to the primary). The visual and the mechanic disagreed; they now
  agree. Pinned by two host tests, the untag half poison-verified.
  **Deviation surfaced** (per the executor rule): the deep review said "pose
  views exist" — use them. They do, but `BodyPoseView` lives in
  `ambition_sim_view`, which depends on `ambition_actors`; consuming it would
  add an upward edge the presentation crate's manifest explicitly forbids
  ("never a host crate"). Fixed with the crate's OWN host-seam idiom instead
  (the same shape as `PortalCameraContinuityHostView`).

**Exit:** no `is_player` branch selects an effect payload anywhere in combat;
a moveset volume can author its own strike/hit effects and the goblin swipe
sounds different from the sword.

## 9. Player-facing repairs (Jon's fix list) — [opus]

From `untracked/jonnotes-FIXES.md`, verified state in deep-review §8:

- ~~room reset must consult `RespawnPolicy`~~ **DONE 2026-07-19**: a room reset
  revives a corpse only under `OnRoomReenter`/`InPlace`; `DeadStaysDead`/`OnRest`
  corpses stay dead instead of being briefly alive for the rest of the frame.
  Pinned by `integration/respawn_policy_tests.rs`, poison-verified. Together with
  the earlier placement-pin fix this closes Jon's "NPCs seem to infinitely
  respawn";
- morph-ball (and transform-mode generally): worn presentation follows the
  body's active mode — design it as the general transform/worn-identity rule,
  not a morph-ball special case;
- shrine + glider sprite repair (shrine mechanic itself is still a stub);
- kernel-guide NPC: peaceful-state patrol around a home base (authored brain
  policy, existing vocabulary);
- possession-aware dialog: speaker/listener identity derives from the actors
  in the conversation, not "the player" (dialog already has stable identity;
  model listener-side adaptation);
- `AMBITION_START_CHARACTER=sanic`: trace why the persona grants
  blink/fireballs and loses move/jump in the full app — per-character
  ActionScheme data + host input hookups; fix as data/seams, not special cases.

**Exit:** each item demonstrated in the real app (feel ships blind where
visual; behavior verified headless where steppable).

## Parallel maintenance — [sonnet unless noted]

Small non-blocking work when it does not collide with the campaigns:

- finish the bounded boss animator fold [opus]: converge `BossAnim`/boss frame
  projection toward the shared `CharacterAnim` vocabulary and retire obsolete
  `target_pos`-style mirrors where still live. Do **not** reopen boss body
  integration (`integrate_boss_bodies` already delegates to the canonical
  body kernel);

- planning-doc repairs queued by the deep review: rewrite
  `engine/room-transition-loading.md` to current-architecture shape (Phases 1–4
  landed; keep Phase 6 performance closure) [opus]; reframe
  `engine/immutable-content-…md` §7.5–7.6 + Phase 5 under ADR 0027 [opus];
  fold `engine/boss-system.md`'s surviving rules into `boss-design.md` and
  archive it; compress `engine/encounter-orchestration.md` to the durable
  model; archive `engine/shell-vanity-sequence.md` once VC5 lands; repoint
  `headless-verification.md` at `ambition_sim_harness`;
- ~~single-source the demo remaining-lists in `demos/*.md`~~ **DONE 2026-07-19**:
  tracks #3/#4 and the two `status.md` rows now POINT at
  `demos/super-mary-o.md` / `demos/sanic.md` instead of restating them. The
  copies had already drifted (this queue's Mary-O copy had lost "enters the
  secret" from the scripted-run gate and omitted the post-acceptance levels
  bullet);
- **KB linter: 6 of 7 failures fixed 2026-07-19** (the four mechanically
  recomputed evidence markers the 07-18 rewrite dropped are restored with real
  values, `docs/concepts/invariants.md` gained frontmatter, AGENTS.md is back
  under the line cap). ONE remains, deliberately left red rather than papered
  over: 17 files carry inline `#[cfg(test)]` modules ≥200 lines, which
  `docs/concepts/test-placement.md` says belong in an adjacent `src/foo/tests.rs`.
  Fixing it means either performing those moves or reviewing each and recording
  an accepted-inline marker — both are per-file judgement, not bookkeeping.
  Files: `ambition_actors/src/{action_scheme.rs, features/ecs/autonomous_reconcile.rs}`,
  `ambition_audio/src/catalog.rs`, `ambition_characters/src/{action_scheme.rs,
  actor/character_catalog/{binding.rs, mod.rs}, equipment.rs}`,
  `ambition_encounter/src/{lifecycle.rs, waves.rs}`,
  `ambition_ldtk_map/src/conversion/mod.rs`,
  `ambition_platformer_provider/src/lifecycle.rs`,
  `ambition_sim_view/src/control_prompt.rs`,
  `ambition_touch_input/src/bevy_plugin.rs`,
  `ambition_content/src/presentation/dialog.rs`,
  `ambition_demo_mary_o/src/{flag.rs, lib.rs, powerups.rs}`.
  Watch the `use super::*` depth when moving — an adjacent child module keeps it,
  a nested one does not (see `integration/dash_tests.rs`'s header);
- ~~backfill the 8 unindexed `dev/journals/` entries into
  `dev/journals/index.md`~~ **ALREADY DONE — the bullet was stale on arrival**:
  the same 2026-07-19 deep-review commit that queued this also backfilled the
  index (+15 link rows; its own message says "journals index backfilled").
  Recomputed at HEAD: all 31 journal files are linked from `index.md`, 0
  unindexed;
- ui_nav adoption in `ambition_menu`/`ambition_settings_menu`; the shared
  input-suspended gate for cutscene/encounter [opus];
- one structurally complete content eviction at a time when a real named
  family remains in a reusable crate;
- ~~add `tree_sitter` + `tree_sitter_rust` to `run_developer_setup.sh`~~ **DONE
  2026-07-19**: setup now provisions the repo-root `.venv` that `scripts/*.py`
  use, so `scripts/ecs_inventory.py` regenerates on a fresh clone instead of
  dying on `ModuleNotFoundError` and silently leaving the committed navigation
  packets stale.

## Standing execution rule

Use Rust types, ownership, crate direction, visibility, and ordinary behavioral
acceptance tests before adding policy/scanner machinery. Historical journals stay
historical. Completed execution narratives do not remain in this live queue.
