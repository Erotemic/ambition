# FABLE FINAL AUDIT — 2026-07-07 (the last fable pass)

Whole-repo audit after the opus/codex decomposition landing (E1a–e, E2
projectiles, W3 world/LDtk split, W-queue step 3 lowering proof, asset-manager
carve, sprite-sheet absorb, encounter mint, boss tail, `gameplay_core` →
`ambition_actors`, `game/` re-home). **Findings are appended IN PRIORITY ORDER
as they land — treat every entry as a plan item even if the session cut off
before it was folded into the ledgers/cards.** Anything here that contradicts
an older card wins (it is the newer ruling).

Audit order (most valuable first):
1. Dep-graph / tier audit — does the crate DAG match architecture.md's arrows?
2. `ambition_actors` (68k) — the residual monolith's next decomposition line.
3. Facade/shim census — the E7/E8 dissolution checklist, made explicit.
4. Ruling-compliance spot checks (W3 zero-LDtk, [W-e] hard error, GeoId
   adoption, SweepSample adopters, Tier-0 purity).
5. Subtle-correctness greps (query order, time domains, pushout, Entity
   identity, seam races).
6. Full test gate.
7. Elegance directions newly visible in the post-carve structure.

## Findings

(appended below, newest last)
### F1 — Dep-graph audit: the DAG is sound; ELEVEN arrows need work (none design-hard)

The workspace DAG has no cycles (`actors → sim_view` is dev-dep only) and the
big shape is RIGHT: engine_core/entity_catalog at the bottom are dep-free,
characters/combat/primitives sit above engine_core, `game/` sits on top.
The arrows below are the full remaining cleanup list, each with its
prescription — log-once so E7/E8 executors don't re-derive:

1. **`ambition_world` → combat + interaction + portal (VIOLATES the §4b
   "world IR names ZERO runtime types" ruling — transitionally).** Cause: the
   legacy `Authored<DamageVolume>` / `Authored<Interactable/Pickup/Chest/
   Breakable>` / `PortalSpec` families still ride `RoomSpec`. This is EXACTLY
   what [W-b] record-over-schema dissolution removes. **Prescription: each
   W-queue step-3 branch conversion's exit test is "delete the corresponding
   Cargo dep from ambition_world"** — hazards → drop combat; interactables/
   pickups/chests/breakables → drop interaction; portals → drop portal
   (portal placement becomes a Tier-0 schema variant: color/link/normal are
   plain data). `zone_sfx: Option<ambition_sfx::SfxId>` on the room graph is
   the same disease in miniature — an authored sfx REFERENCE should be a
   plain string/id newtype in the IR (Tier-0 idiom), killing world → sfx.
2. **`ambition_actors::portal` is a FACADE that re-exports
   `ambition_portal_presentation::*`** — the sim crate structurally deps a
   presentation crate to keep old `crate::portal::` paths alive.
   **Prescription: repoint the (few) consumers to the two real crates and
   delete the facade module + the Cargo dep.** A sim crate must never dep a
   presentation crate, even for re-export.
3. **`ambition_vfx` → `ambition_characters` for ONE type (`ActorFaction`).**
   The effect vocabulary crate pulls the whole cast crate for a tag it only
   uses to pick a tint/side. **Prescription: the vfx message carries the
   presentation-neutral fact it actually needs (a `HitSide`/tint enum owned
   by vfx, mapped at the emit site); drop the dep.**
4. **`GameMode` lives in `ambition_actors` and leaks it into host,
   touch_input, and (via schedule/run-conditions) render.** It is a tiny
   session-state enum. **Prescription: move `game_mode` DOWN (candidate: its
   own ~50-line `ambition_session_state` crate, or into
   `platformer_primitives::schedule` next to the schedule labels — states and
   labels are the same kind of vocabulary). This single move, plus schedule
   labels already being in primitives, frees host + touch_input from
   `ambition_actors` almost entirely** (host's remaining reads are dialog +
   camera_ease ticks — camera_ease is presentation-side time easing and can
   move to host/render; dialog is `ambition_dialog` already, repoint).
5. **`ambition_render` → `ambition_actors` (the E4 dep-flip blocker), now
   precisely enumerable:** rooms (11 — REPOINT to `ambition_world::rooms`,
   actors::rooms is a facade), features (9 — live ECS components; these are
   the true E4 stragglers: convert to SimView facts or repoint to combat),
   assets (8 — **`GameAssets` is an ASSET CATALOG living in the actor crate;
   move it to `ambition_asset_manager`/`ambition_sprite_sheet` side**),
   session (6 — messages like RespawnRoomVisualsRequested: move the message
   defs to a crate both can see, e.g. sim_view or world), dev (6 — debug
   overlay reads; gate behind dev_tools), portal (4 — the facade above),
   shrine/player/items/schedule (2 each — repoint/move-down leftovers).
   None of these is the hard identity work (that landed in E4 slices); they
   are moves + repoints.
6. **`ambition_items` contains `inventory_ui`** (deps ui_nav for
   `MenuFocusState`). The item MODEL and the inventory UI are different
   tiers. **Prescription: split inventory_ui out (menu-side or its own
   `ambition_inventory_ui`); items drops ui_nav.**
7. **`ambition_characters` → `ambition_input` for `ControlFrame`** — the
   two-port body means brains EMIT control frames, so the dep direction is
   defensible; but `ControlFrame` being input-crate vocabulary while
   `InputState` is engine_core vocabulary is a SPLIT-BRAIN worth one look
   when netcode lands (N-track): the brain-facing control vocabulary should
   probably live with the body contract (engine_core or primitives), making
   ambition_input purely a device-adapter crate. NOT urgent.
8. **`ambition_asset_manager` → `ambition_sfx`** — only for `SfxId` +
   `BankProvider` adapter. Acceptable today; if asset_manager is ever meant
   to be engine-generic, the sfx adapter is a feature-gated module. LOW.
9. **`ambition_runtime` → actors/combat/projectiles/etc.** — correct BY
   DESIGN (runtime is the composition tier).
10. **`ambition_host` → render + actors** — see 4; after GameMode/camera_ease
    move, host should dep only input/render/runtime/sim_view (its charter).
11. **`ambition_touch_input` → render** — touch draws its own overlay quads
    through render helpers; acceptable (it IS presentation), but then its
    name is wrong-tier: it is a presentation adapter, not an input crate.
    Optional rename/re-home under a `presentation/` grouping someday. LOW.

### F2 — `ambition_actors` (68k): what it still smuggles + the residual's true shape

Module census (top): features 24.7k (ecs 19.8k — the REAL actor domain:
actors/mount/damage/bosses/spawn/perception/attack/damage_apply), boss_encounter
6.9k, player 6.6k, abilities 4.1k, character_sprites 2.8k, projectile 2.3k +
enemy_projectile 0.7k, world 2.0k, dev 1.6k, assets 1.6k, encounter 1.6k,
items 1.5k, time 1.4k, persistence 1.3k, session 1.3k, audio 1.1k, menu 1.0k,
body_mode 0.8k, portal 0.76k, schedule 0.6k, dialog 0.5k, music 0.4k.

**The actor DOMAIN itself (features+player+abilities+boss_encounter+body_mode
≈ 43k) is legitimately here.** The rest divides into three disposition
classes — log-once so the next sessions don't re-derive:

1. **MISPLACED (move whole, mechanical):**
   - `assets/` (GameAssets + sandbox_assets + loading) — an asset catalog in
     the actor crate; it is also render's biggest reason to dep actors (F1.5).
     Destination: `ambition_asset_manager` (catalog machinery) +
     `ambition_sprite_sheet` (character-sprite-specific lookups).
   - `character_sprites/` remainder (2.8k) — sheets/anim/animator modules are
     ~50% facade re-exports of `ambition_sprite_sheet` already (8 facade
     files); finish the absorb, delete the tree.
   - `world/physics.rs` (avian adapter) + `world/overlay{,_rebuild}.rs` —
     these stayed actors-side because the overlay REBUILD reads live feature
     components. Correct interim home, but name the end-state: after W-queue
     step-3 dissolution the rebuild's inputs become plain solids and the pair
     joins `ambition_world`; physics.rs (debris/avian) is presentation-adjacent
     and can join render/host side whenever.
   - `projectile/` + `enemy_projectile/` ECS half (3k) — joins
     `ambition_projectiles` in the carded dedicated session.
2. **RESIDUAL GLUE for already-minted crates** (audio/menu/dialog/items/
   encounter/persistence/music/dev modules, ~7k total): each is the actor-side
   wiring for a carved crate. Per ADR 0019, the plugin/schedule wiring belongs
   in `ambition_runtime`; actor-DOMAIN reactions stay. Treat each module as a
   two-way split, one commit each — do NOT move them wholesale into runtime
   (that would just relocate the god-hub).
3. **FACADES (60 `pub use ambition_*` re-export sites in actors).** These are
   the deliberate hub-continuity aliases. The dissolution ratchet: **a facade
   may be deleted the moment `grep -rn "ambition_actors::<mod>"` outside
   actors returns zero** — put that one-liner in the E7/E8 card as the
   per-facade exit test, and burn them down opportunistically (each is a
   5-minute repoint+delete).

**North star for the residual (fold into unified-actors.md):** `player/`
(6.6k) existing as a SIBLING of `features/ecs` is the last structural
player-centrism — the fighter-unification S5/S6 endgame folds the player's
remaining special-cased systems into the one actor pipeline (the single
control seam already made the player "an actor wearing Brain::Player"). The
right long-term shape is ONE `actors` module tree where player-ness is a
brain + a slot, not a directory. Do not force this before S5/S6; DO stop
adding new player-only systems (new work lands body-generic or brain-side).

### F3 — Ruling-compliance spot checks: four green, THREE corrections

Verified green:
- **[W-e]/[W-b] lowering registry** — `ambition_world::placements` has the
  duplicate-registration panic AND the unknown-kind hard error, both pinned by
  `#[should_panic]` tests. Exactly as ruled.
- **§3.6 GeoId stamping survived the W3 move** — `ambition_ldtk_map::intgrid`
  still stamps level-scoped `TileLayer` ids with the merge ordinal.
- **Tier-0 purity** — `ambition_entity_catalog` still deps NOTHING. ✓
- **W4/ADR 0021 first cut recorded**; `ambition_world` has no LDtk dep. ✓

Corrections (log-once, all small):
1. **`ron_room` landed on the WRONG side of the W3 cut.** The serialized-IR
   loader (`RonRoomDoc`, `room_doc_from_ron`, `load_manifest_ron_rooms`) and
   the `WorldManifest.ron_rooms` rows live in `ambition_ldtk_map` — the LDtk
   BACKEND crate. Its entire purpose is "a room enters the graph with no LDtk
   anywhere in the path", so a RON-only app currently needs the LDtk crate to
   load a serde room. **Prescription: move `ron_room.rs` (+ `RonRoomSource`,
   or a backend-neutral manifest seam for it) into `ambition_world`;
   `ldtk_map::to_room_set` keeps calling it (backend → IR is the legal
   arrow). Make the W4 "second backend" fixture test live in
   `ambition_world`'s own tests to pin it.**
2. **`integrate_boss_bodies` still hasn't adopted `SweepSample`** (the §3.1
   known-remaining mover) and **`PortalSweepAnchor` still exists** (retired by
   CC6's relative swept trigger). Both were carded — RE-CONFIRMING they are
   still open so the CC6 executor doesn't assume otherwise.
3. **`ambition_world` still contains no dep-direction regression TEST.** The
   Cargo graph is clean today, but the ruled invariants ("world names zero
   LDtk", "world names zero runtime crates" — the second currently
   VIOLATED-by-design via legacy families, see F1.1) have no enforcement.
   **Prescription: add a tiny build-graph test (parse `cargo metadata` or
   just grep Cargo.toml in a unit test) asserting ambition_world's dep list
   against an explicit allow-list, so step-3 branch conversions RATCHET
   (removing combat/interaction/portal from the allow-list one at a time).**

### F4 — Correctness findings: two REAL regressions FIXED in-session + three logged hazards

**Fixed in this audit (commits on main):**
1. **The `game/` re-home broke desktop asset-root resolution** —
   `desktop_asset_root()` + `capture_scene` hopped `../ambition_actors/assets`
   from `game/ambition_app` (lands in `game/`, not `crates/`); the silent
   fallback to exe-relative `assets` reproduces "game runs but nothing
   renders / no music". Fixed to `../../crates/…`; caught by the
   (well-written) cli test. Every other `CARGO_MANIFEST_DIR` hop audited —
   correct.
2. **The `gameplay_core → ambition_actors` rename broke the music tools** —
   `_paths.py` repo-root probe + cli/bundle/audit registry paths pointed at
   the dead crate dir (submodule commit + bump). The regen shell scripts were
   already updated.

**Logged hazards (small, opus-executable):**
3. **`WorldClock.time_scale` is written DIRECTLY outside the time-control
   owner** — `features/ecs/damage_apply.rs:207,369` and
   `world/rooms/load.rs:114` hard-set `time_scale = 1.0` (respawn/transition
   resets), bypassing the ADR 0010/0011 `ClockScaleRequest` seam. A reset
   racing a live bullet-time/hitstop request silently clobbers it.
   **Prescription: replace with a `ClockScaleRequest::reset()` (or a
   dedicated ResetClock message) handled by the one owner in
   `time/time_control`.**
4. **Non-deterministic player pick under multiplayer:**
   `save_sync.rs:79` and `actors/update.rs:227` use `query.iter().next()`
   as a "the player" fallback. Single-player-safe; with slots (the RL/
   multiplayer target) query order is unstable — pick by lowest
   `PlayerSlot` instead. Tag both with `AMBITION_REVIEW(determinism)`.
5. **The full app gate** re-ran clean after fix (1): all suites green except
   the two documented REDs (`unified_melee::a_hostile_actor` feel-RED;
   verify gnu_ton in the final run below).

### F5 — Elegance directions the new structure makes visible (NOT yet in any card)

1. **Mint the `ambition` UMBRELLA CRATE — the engine-for-other-games keystone
   made concrete.** `game/ambition_app` currently declares ~26 `ambition_*`
   deps; a second game would copy that wall. The oracle ("another platformer
   by ADDING a content crate without editing core") wants: one `ambition`
   facade crate that deps the engine face (runtime, host, world, ldtk_map,
   combat, actors, render, sim_view, …) and exposes (a) the plugin groups a
   game composes and (b) a curated prelude. A NEW game then deps `ambition`
   + its own content crate, PERIOD. This crate is also where the old
   "framework spine" doc-comments in actors/world already point. Cheap to
   mint (it is re-exports + plugin-group structs), and it RATCHETS: once the
   demo apps compile against it, any core-editing regression shows up as an
   umbrella-surface change. Suggested card: **E9 — the `ambition` facade
   crate**; exit = `game/ambition_app/Cargo.toml` lists ≤ 4 ambition deps.
2. **The Sanic/SMB1 demo games ARE the oracle test — give them their homes
   now.** `game/ambition_demo_sanic/`, `game/ambition_demo_smb1/`, each
   depping ONLY the umbrella + (optionally) `ambition_content`-style data of
   their own. The demo-game matrix in roadmap.md becomes three Cargo.tomls
   you can grep. Do this WITH E9, not before.
3. **At the S5/S6 player-fold, rename `features/` away.** The module name is
   pre-decomposition residue ("content features" that are now just the actor
   ECS). When player/ folds in (F2 north star), the tree becomes
   `ambition_actors::{bodies, brains, spawn, damage, mount, perception,
   bosses}` — names that say what they are. Do not rename before the fold
   (one churn, not two).
4. **Tests should travel with their subject:** `features/conversion_tests.rs`
   (849 lines) tests LDtk conversion that now lives in `ambition_ldtk_map`;
   actors' `world/rooms/tests.rs` (602) tests the room graph that lives in
   `ambition_world`. Moving them tightens both crates' change-detection
   (a conversion regression should fail IN the backend crate).
5. **Anti-goal (Jon's tiny-crate skepticism, restated for the tail):** the
   remaining wins are MOVES and DELETIONS, not new crates. Beyond E9 + the
   demo crates + possibly `ambition_session_state` (F1.4), no new crate
   should be minted without a consumer that exists today. The crate count
   (38) is already at the top of the comfortable range; the value now is
   thinning `ambition_actors` and deleting facades, not adding boxes.

### F6 — Final gate + close

Full `cargo test -p ambition_app --features rl_sim` after the F4 fixes: **44
suites green; the only failure is the documented `unified_melee::
a_hostile_actor` feel-RED** (unchanged, feel-reserved for Jon). The
decomposition landing is behaviorally sound.

Ops note: the dev box hit 100% disk mid-audit; `~/ambition-target/debug/
incremental` (149G of regenerable build cache) was deleted to recover.
Consider `CARGO_INCREMENTAL=0` for CI-style full-gate runs, or a periodic
`cargo clean` cron, so a full disk doesn't silently kill background gates.

**Priority order for the next sessions (all opus-executable, most valuable
first):** F4.3 clock-reset seam → F1.1/F3.1 world-purity ratchet (dep test +
ron_room re-side + branch conversions) → F1.4 GameMode move-down (frees
host/touch_input) → F1.5/F2.1 GameAssets move + render repoints (finishes
E4's dep flip) → F2.3 facade burn-down → E9 umbrella + demo homes (F5.1/2)
→ projectiles dedicated session (already carded) → F2.2 glue splits.
