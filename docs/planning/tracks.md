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

## 0. Pay down the GGRS correctness debt — [opus, fable-specced]

Spec: deep-review §2 (each item carries file:line). The registration set is
comprehensive; these are the exceptions, found by cross-referencing every
sim-mutated component/resource against the registry:

- register (or clear-on-rollback) the unregistered sim state: `WornEquipment`,
  `SwitchActivationQueue`, breakable/hazard timers, portal-gun runtime, fuses
  and held projectiles, and the demo-content state composed into the app shell
  (`BallDash`, `SanicActState`, `MaryOLevelState`, `FlagSequence`);
- fix `MovePlayback.live_boxes` across rollback (see `FIXME(ggrs-live-boxes)`
  in `crates/ambition_combat/src/moveset/mod.rs`): clear on LoadWorld or
  liveness-check the `(inside, live)` arm; bring strike volumes into the
  rollback contract or prove them derived;
- move `possession_trigger_system`'s `Local` hold/edge state into rollback
  state;
- tie-break targets by `SimId` not raw `Entity`
  (`combat/targeting.rs:285`); sort slot requests by `actor_id` before
  `assign_slots` (`features/ecs/actors/update.rs:255-320`);
- restore a coverage forcing function: a test that walks sim-schedule write
  access and asserts every mutated component/resource is registered, declared
  derived, or explicitly waived (the deleted census had 93+236 rows; the
  replacement should be a live computation, not a checked-in ledger).

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

## 2. Build-graph hygiene (compile-time wins) — [sonnet]

Deep-review §6. Exact edits:

- `crates/ambition_menu/Cargo.toml` + `game/ambition_menu_kaleidoscope/Cargo.toml`:
  replace plain `bevy = "0.18.1"` with `default-features = false` + the minimal
  feature set (iterate until green) — this alone stops pbr/gltf/animation/audio/
  gilrs/reflect_auto_register leaking into every build of every app;
- `game/ambition_app/Cargo.toml`: make `ambition_menu_kaleidoscope`
  `optional = true` wired to the existing `kaleidoscope_menu` feature;
- adopt `[workspace.dependencies]` for bevy/serde/ron/thiserror; fix the
  ron 0.11/0.12 and thiserror 1/2 drift;
- delete the free dead edges: `ambition_world → ambition_time`,
  `ambition_sprite_sheet → ambition_interaction`; make actors→ui_nav a proper
  optional forward;
- delete feature machinery that gates nothing (`headless`, actors'
  `rl_sim`/`static_sfx_bank`/`kaleidoscope_menu` stubs, ldtk_map's unreachable
  `dev_hot_reload`) or make it real; make `asset_manager/bevy` default.

**Exit:** `cargo tree -e features -p ambition_app` shows no bevy default-only
features pulled by menu crates; headless build drops bevy_lunex; workspace
builds green.

## 3. Close Super Mary-O level 1 — [opus]

Engine seams proven (pickups/equip, grown form, ranged powerup, bricks, crony
stomp, flag, clock, tally, cyclic restart). Remaining customer work:

- secret pipe and underground room;
- sliding shell prop;
- HUD, title, and results presentation;
- a deterministic scripted run that completes level 1 through real controls,
  collects a powerup, and exercises its effect.

**Exit:** visible and headless customers use the same provider, body, item, and
level state with no Mary-O-only engine path.

## 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. Remaining:

- ring drop-on-hit scatter;
- goal, HUD, results, and end-of-act sequence;
- one complete authored act;
- deterministic headless completion proving the rewarded high route is faster
  than the lower safe route under the same control contract.

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
- delete the compat facades: `effects/mod.rs`, `debug_label.rs`, `host/`, and
  repoint the ~73 `pub use ambition_*` lines at canonical homes [sonnet-able];
- execute the already-ruled content evictions still outstanding (M23 / recon
  accepted #1): `ambition_items::Item` closed enum → provider-registered
  catalog; `deep_dream_strength` → content-owned presentation knob;
  `puppy_slug_gun.rs` → parameterized summon-ally ability + content data.

**Exit:** actors' out-degree drops (no menu/settings_menu/ui_nav edges); the
oracle "add a character/item without editing core" holds for items.

## 8. Combat unification batch — [opus]

Deep-review §5; BIFURCATION entries in code_smells.md 2026-07-19:

- collapse the projectile player/actor victim loops onto the
  `hitbox/mod.rs:203` single-loop pattern (fixes the knockback asymmetry and
  the dropped parry term);
- build ONE victim-side hit/death feedback seam keyed on the attack/volume
  spec + the victim's feel profile; delete the two `is_player` attacker-side
  emit blocks — this is also where Jon's "each attack binds its own VFX/SFX"
  lands (authored effect identity on the volume/move spec);
- while there: fix the portal gun-visuals `BodyKinematics` read-model leak.

**Exit:** no `is_player` branch selects an effect payload anywhere in combat;
a moveset volume can author its own strike/hit effects and the goblin swipe
sounds different from the sword.

## 9. Player-facing repairs (Jon's fix list) — [opus]

From `untracked/jonnotes-FIXES.md`, verified state in deep-review §8:

- room reset must consult `RespawnPolicy` before reviving dead actors
  (code_smells 2026-07-19; the "NPCs respawn" placement half is fixed);
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
- single-source the demo remaining-lists in `demos/*.md`; status/tracks refer,
  never copy (the Sanic copies drifted independently — again);
- reconcile `check_agent_kb.py`'s evidence-marker expectations with the
  rewritten status.md (linter is red; prune stale markers or restore them
  deliberately);
- backfill the 8 unindexed `dev/journals/` entries into `dev/journals/index.md`;
- ui_nav adoption in `ambition_menu`/`ambition_settings_menu`; the shared
  input-suspended gate for cutscene/encounter [opus];
- one structurally complete content eviction at a time when a real named
  family remains in a reusable crate;
- add `tree_sitter` + `tree_sitter_rust` to `run_developer_setup.sh` so
  `scripts/ecs_inventory.py` regenerates on a fresh clone.

## Standing execution rule

Use Rust types, ownership, crate direction, visibility, and ordinary behavioral
acceptance tests before adding policy/scanner machinery. Historical journals stay
historical. Completed execution narratives do not remain in this live queue.
