# Intro vertical slice — handoff to the next agent

## Completed by autonomous follow-up

(Started 2026-05-16; agent committed to working until backlog is zero per
`docs/intro_autonomous_followup_prompt_v3.md`.)

- `7a8a677` — P0 step 1 — ClockDomain enum + sim/player/wall dt accessors on
  WorldTime; legacy raw_dt/scaled_dt kept as aliases; SP behavior unchanged.
- `44e028a` — P0 step 2 — ClockScaleRequest message + RegimePolicy resource
  (Solo / RL / Cinematic); apply_clock_scale_requests system wired into the
  schedule before refresh_world_time; existing mutations untouched.
- `18c0419` — P0 step 3 — ProperTimeScale component (default 1.0) +
  WorldTime::entity_dt accessor; animate_player / animate_characters /
  animate_props / animate_bosses migrated; SP behavior unchanged because
  no entity carries the component yet.
- `390cb7b` — P0 step 4 — bullet-time wired through emit→apply→smooth
  via ClockScaleRequest + RequestedClockScale; legacy update_time_scale
  removed from phases.rs (now #[deprecated]); apply_suspended also
  zeroes the requested target.
- `ddbad80` — P1 §1.3 — regression test asserts Prop entities never
  grow Interactables and no NPC Interactable overlaps a prop position
  in intro.ldtk (pins 195b5ce structurally).
- `66e62ad` — P2 step 1 (ADR 0015) — intro_lab + town tileset defs
  registered in intro.ldtk via `tileset add`; additive only, no
  Tiles layer instances or Bevy runtime changes yet.
- `f7618df` — P5 — creator_final_fast + creator_final_impossible
  dialogue variants registered (nodes + INTRO_DIALOGUE_IDS); intro
  cutscene variant-selection logic still TODO.
- `b7bdcc0` — P6 — `def update-entity` python subcommand. Adds
  fields to an existing entity def via `--add-field name:type:default`.
  Unblocks ADR 0016 Actor unification (adding aggression /
  dialogue_id / brain / path_id to a baseline Actor entity).
- `16a8e4f` — P4 step 1 — intro.ldtk flipped to GridVania layout
  with 16px grid (existing 2000-px-spaced level positions align
  cleanly). Re-packing levels to be edge-adjacent is deferred —
  requires walking every authored entity to update its __worldX.
- `22447cd` — P2 end-to-end (ADR 0015) — `tileset add-layer` +
  `tileset paint` python subcommands; intro_lab tileset grid sizes
  fixed (32/64); IntroLabTiles layer def + per-level instance on
  all 5 intro levels; 458 tiles painted from Collision IntGrid;
  `sync_ldtk_world_transform` Bevy system aligns LdtkWorldBundle
  with Ambition's centered active-area frame.
- `3d54368` — P2 follow-up — spawn second LdtkWorldBundle for
  intro.ldtk (`IntroLdtkWorldRoot`) so bevy_ecs_ldtk can render
  the intro's painted tile layers; sync systems updated to
  Or<sandbox, intro>.
- `9af4d2f` — P4 step 2 — `world repack` python subcommand;
  intro.ldtk levels packed edge-adjacent starting at (0,0); 55
  entity __worldX/__worldY fields updated. Final intro span =
  6528px wide.
- `8e0efb6` — playtest pass: fascist sprite fallback by enemy
  name; `tech_bros_disruption` set on every intro level;
  `animate_props` holds `intro_cart` at frame 0 (static-until-
  moving); cart top painted as OneWayUp so the player can stand
  on the cart in the wake room.

## Playtest follow-ups (from 2026-05-16 review)

These items came out of a manual intro playthrough; the easy fixes
landed in `8e0efb6`, the rest are documented here so they can be
picked up without re-deriving them.

### Cart visual sits "on/in the ground" — needs art-side review

After `8e0efb6` the wake-room cart has a OneWayUp surface at its
top edge (cells 8..18, row 14) and sits with its AABB bottom at
y=352 — exactly the room's authored floor top. The sprite renders
at `feet_anchor_y = -0.500` so its bottom edge aligns with the
AABB bottom, i.e. the floor.

If the cart STILL reads as floating after the tile commits land
(`22447cd` / `3d54368` paint the floor as `wall_plain` tiles
behind the cart), the likely causes are:

- The cart sprite has transparent padding at the bottom of its
  frame. Fix: bump `CART_SHEET.feet_anchor_y` (less negative,
  e.g. -0.45) so the sprite drops further into the AABB.
- The floor's tile sprite has internal padding above its actual
  pixel content. Re-check `intro_lab_tileset.png` tile 0
  (`wall_plain`).

Re-verify in a visible build before tuning.

### Wake-room arrival door spawns in mid-air (intentional)

`wake_room_arrival` is the LoadingZone that links the creator's
basement back from `central_hub_complex`. Per the design, the
door should appear suspended in the lab — it's the "you cracked
back through the wall" exit, not a flush ground-level door.
Document for the next door-pass: this is a deliberate exception
to the "doors snap to a Solid surface" convention; new doors
elsewhere should keep snapping.

### Cart should be its own visual category, not piggyback NPC sprite anim

The cart's `CART_SHEET` reuses `CharacterAnim::Idle` for what is
really a wheel-roll loop. The `static-until-moving` list in
`animate_props` is the v1 patch; the proper fix is a per-prop
animation enum (`PropAnim::Static`, `PropAnim::Rolling`) keyed
off a `PropMotionState` component the gameplay layer writes when
the cart actually moves. Filed as P7 tech-debt §5 in the
original backlog ("Replace overloaded CharacterAnim variants").

### Cart collision — needs a proper Prop-with-collision concept

The OneWayUp band painted in `8e0efb6` works for the wake-room
cart specifically, but every cart-shaped prop the design adds
later (lab benches that block movement, crates that the player
pushes) would need its own hand-painted IntGrid patch. The
proper fix is:

- Extend `PropSpec` with an optional `collision_kind` field
  (`Solid | OneWayUp | None`).
- Have the prop-spawn path emit the IntGrid/Block alongside the
  visual.

This is a small additive engine change but spans
`ldtk_world/conversion.rs`, the Prop entity def, and
`rendering/world.rs::spawn_room_prop`. Defer to the next
authoring batch when a second prop needs collision.

### Compose a dedicated intro music track

`tech_bros_disruption` is a placeholder that landed in `8e0efb6`.
The design calls for an original "creator's basement, raid is
moments away" cue — propulsive, anxious, low-end heavy. Until
that lands, the placeholder track will play across all 5 intro
levels (including the drain alley + gate stack which probably
want a different mood). Filed as a follow-up for the music
composer pass.

## Open questions for next agent

### P2 — LDtk tileset rendering (remaining work)

The tileset DEFS are now in intro.ldtk (commit `66e62ad`), but
no Tiles layer instances exist and the Bevy runtime hasn't been
flipped on yet. Remaining work, in dependency order:

1. **`tileset add-layer-def` python subcommand** — add a Tiles
   layer def to `defs.layers[]` pointing at a registered
   tileset uid. Mirror `area_authoring::ensure_climbable_layer_def`
   for shape; new layer type is `"Tiles"` with `tilesetDefUid` set.
2. **Add Tiles layer defs** — one for `intro_lab` (uid 104433) and
   one for `town` (uid 104434). Use the tool from step 1.
3. **Empty Tiles layer instances on every level** — same pattern
   as `ensure_climbable_layer_def` adds empty Climbable instances
   to all levels for schema consistency. Empty `gridTiles: []` is
   valid.
4. **`tileset paint` subcommand** *or* hand-author in LDtk editor —
   actually place tiles. Without this, the runtime change in step 5
   shows nothing.
5. **Rust runtime — `level_background: LevelBackground::Translucent`**
   in `app/resources.rs::LdtkSettings`. Changes the per-level
   background quad from "skipped" to "rendered behind tiles." Low-
   risk if no tiles are authored; needed before tiles can render.
6. **Rust runtime — per-room `LdtkWorldBundle` transform sync** —
   this is the hard part the ADR calls out. `bevy_ecs_ldtk` renders
   tiles in raw LDtk world-pixel space; Ambition's renderer centers
   each active area at the origin via `world_to_bevy`. The seam is
   in `ldtk_world/bevy_runtime/asset.rs`. The fix: a per-room
   transform on the single `SandboxLdtkWorldRoot` entity that
   piggybacks on the active-area change events. Pseudocode:
   ```rust
   ldtk_world_transform.translation =
       world_to_bevy_origin(active_area_min, WORLD_Z_BLOCK - 1.0);
   ```
   The `active_area_min` is already tracked in `LdtkRuntimeIndex::
   area_bounds`. The risk is in the *interaction* with hot-reload
   and room transitions — those swap `LevelSet`, and the transform
   must move on the same frame.

**Decision deferred:** whether to ship Tiles + tilesets alongside
Ambition's existing colored-block renderer (two-pass, debug-toggle
the blocks) or fully replace blocks with tile visuals (more work,
cleaner shipping look). ADR 0015 leans toward two-pass; the
`RenderDebugBlocks` boolean is the seam.

**Recommended next-session entry point:** start with the python
`tileset add-layer-def` tool. Steps 1-4 are author-side only and
can be committed independently without touching Rust. Step 5 is
trivial. Step 6 is the spike — do it after authoring real content
so you can see whether the transform aligns.

### P3 — Actor unification (remaining work)

ADR 0016 lists a 6-commit sequence. Step 1 (`feat(actor): introduce
Aggression + unified ActorRuntime`) was not landed in this session
because the scope is wide (combat, AI, dialogue, save format,
content_validation, conversion_tests, ecs_actor_view_compat all
touch `ActorRuntime`). The ADR's recommended order stands.

**Decision deferred:** whether to start Actor unification BEFORE or
AFTER tileset rendering. ADR 0016 itself recommends Actor first
because it doesn't touch the renderer; ADR 0015's recommended
order has them as independent.

### P4, P5, P6, P7

Not started in this session — see the original backlog above.

Status as of commit `195b5ce`: the intro is playable end-to-end
(spawn on cart → wake → raid (with hostile enemies + combat barks)
→ escape shaft → drain alley → gate stack → portal back to sandbox
hub). All 471 sandbox tests pass; both `sandbox.ldtk` and
`intro.ldtk` validate clean. Run: `./run_game.sh`.

This doc collects every known bug + every still-pending design
ask in priority order so the next agent can pick a slice without
re-reading the whole prior conversation.

## Recent progress (2026-05-16 session)

- §1.1 + §1.2 (portal animator override + gate ring spin row):
  **fixed** in commit `8d963c7`. PortalSprite marker excludes the
  gate portal + ring entities from `animate_characters`, and the
  portal/ring systems own their animator request + tick + atlas
  index. Visually verify by toggling the gate switch in
  gate_stack_lower and watching the portal cycle through
  Opening → On → Closing.
- §2.2 (dedicated Prop LDtk entity type): **landed** in commit
  `195b5ce`. The cart, lab props, gate ring, and gate portal are
  now `Prop` entities (no Interactable; no dialogue prompt). This
  also closes §1.3.
- §2.1 + §2.3 (tileset rendering + Actor unification): captured
  as **ADR 0015** + **ADR 0016** (both Proposed). Implementation
  is the next agent's pick.

---

## 1. Known bugs in shipped v1 portal

### 1.1 Portal animator override conflict ✅ FIXED (commit 8d963c7)

**Symptom:** the portal sprite rendered but didn't visually
transition between phases. The opening / closing one-shots barely
appeared; the portal mostly showed row 0 (the opening anim, looped).

**Root cause:** `crate::rooms::sync_portal_sprite_animation`
called `animator.request(Idle / Walk / Run)` based on phase, but
`crate::rendering::actors::animate_characters` ran every frame
and called `animator.request(pick_npc_anim(state))` which for the
portal (no movement, no dialog) returned `Idle` every frame. The
portal-system request got clobbered.

**Fix shipped:** `PortalSprite` marker inserted by
`sync_portal_sprite_visibility` (portal) and
`sync_portal_ring_rotation_system` (ring); `animate_characters`
filters `Without<PortalSprite>`. The portal/ring systems now own
the animator request + tick + atlas index for those entities.

### 1.2 Gate ring `spin` row ✅ FIXED (commit 8d963c7)

**Symptom:** the ring rotated physically during Opening but
didn't switch to its faster `spin` animation row (12 frames,
85ms vs idle's 8 frames, 140ms).

**Root cause:** `GATE_RING_SHEET` only registered the Idle row;
the sheet on disk had a `spin` row that was unwired.

**Fix shipped:** added `Walk` row binding for `spin` in
`GATE_RING_SHEET` (mirrors the portal sheet pattern).
`sync_portal_ring_rotation_system` requests `Walk` during
`Opening` and `Idle` otherwise.

### 1.3 NPC-as-prop interact prompt ✅ FIXED (commit 195b5ce)

**Symptom:** pressing Interact near a lab prop / gate ring /
gate portal popped a "this NPC has no Yarn node yet" generic
dialog. Per the v1 plan they were authored as `NpcSpawn` with
`prompt: ""` and `dialogue_id: generic_npc`.

**Fix shipped:** dedicated `Prop` LDtk entity type (see §2.2);
all six intro NpcSpawn-as-prop entries migrated to Props with
identical px/size. Props never grow an `Interactable`.

---

## 2. Bigger structural items from the design feedback

### 2.1 LDtk tileset rendering ★ user explicitly asked for this (see ADR 0015)

**What:** the intro_lab_tileset + town_tileset spritesheets exist
on disk but `intro.ldtk` doesn't reference them, and
`bevy_ecs_ldtk` is configured with `LevelBackground::Nonexistent`
+ `IntGridRendering::Colorful` overrides — so the LDtk editor
view and the in-game view diverge. Ambition renders blocks
itself, the ecs_ldtk runtime spine doesn't draw tiles.

**Steps:**
1. Add a Python tool `tileset add <ldtk> <png> <grid_size>` that
   registers a tileset definition in `defs.tilesets`. Probably
   needs uid allocation, image path, columns/rows computed from
   PNG.
2. Add a Tiles layer def to `intro.ldtk` and a tile layer
   instance to each intro level. Hand-author tile content, or
   add auto-tile rules tied to Collision IntGrid values
   (Solid → wall tile, OneWayUp → platform tile, etc.).
3. Reconcile coordinate frames. `bevy_ecs_ldtk`'s
   `LdtkWorldBundle` renders tiles in raw LDtk world-pixel
   space; Ambition's renderer centers each area at the origin
   via `world_to_bevy`. The seam is in
   `crates/ambition_sandbox/src/ldtk_world/bevy_runtime/asset.rs`
   (the comment block calls out the disagreement). Two options:
   align the LdtkWorldBundle transform per-active-area, or
   keep Ambition's block visuals for collision and let
   bevy_ecs_ldtk draw a decorative tile layer behind them.
4. Flip `LevelBackground::Nonexistent` /
   `IntGridRendering::Colorful` per-layer so tilesets render
   without Ambition's debug overlay fighting them.

**Risk:** medium-high. The coordinate-frame reconciliation is
the trickiest part. May need an ADR (0010 LDtk tile rendering).

### 2.2 Dedicated `Prop` LDtk entity type ✅ LANDED (commit 195b5ce)

**What:** the cart, lab props, gate ring, and gate portal are
all authored as `NpcSpawn` with empty prompts (v1 hack). They
visually render but are erroneously interactable. A proper
`Prop` entity type would:
- Have fields `name: String`, `kind: String` (e.g. `intro_cart`,
  `lab_genesis_vat`, `gate_ring`, `gate_portal`).
- Render via a `PropRegistry` that maps `kind` → sheet spec
  (similar to `INTRO_NPC_SPRITE_REGISTRY`).
- Have NO `Interactable` component so the player walks past it
  without an Interact prompt.

**Steps:**
1. `python -m ambition_ldtk_tools def register-entity` with a
   spec for `Prop`. Will need to clear current cart/lab/gate
   `NpcSpawn` entries and re-add as `Prop`.
2. In `ldtk_world/conversion.rs`, add a `"Prop"` arm to
   `entity_to_runtime` that emits a new `PropSpec` (parallel
   to the existing `LoadingZone` / `CameraZone` / etc.). The
   `RuntimeEntityEmission` struct grows a `props: Vec<PropSpec>`
   field; `compose_runtime_area` accumulates them into
   `RoomSpec.props`.
3. Add a Bevy system that reads `RoomSet::active_spec().props`
   and spawns sprites at each prop's position, looking up the
   sheet via a `PropRegistry`. Story-content plugins
   contribute prop kinds via the same plugin pattern as
   `IntroPlugin::install_intro_npc_sprites_system`.
4. Migrate every authored NpcSpawn prop in
   `gate_stack_lower_area.yaml` / `intro_wake_room_area.yaml` /
   `intro_raid_corridor_area.yaml` to `type: Prop`.
5. Drop the lab-prop entries from `INTRO_NPC_SPRITE_REGISTRY`.

**Risk:** low-medium. Mostly additive Rust + spec churn.

### 2.3 NPC / Enemy unification around an `Actor` entity (see ADR 0016)

**What:** the design doc's "no distinction between NPC and Enemy
except aggression level" ask. Today `NpcSpawn` and `EnemySpawn`
are separate LDtk entity defs with different fields; the
runtime carries `ActorRuntime::Peaceful(NpcRuntime)` vs
`ActorRuntime::Hostile(EnemyRuntime)` and migrates between
them via `hostile_from_npc`.

**Target:** single `Actor` LDtk entity with
- `name`, `dialogue_id` (optional),
- `aggression: Peaceful | Wary | Hostile` (and maybe
  `Aggressive` as a faster variant),
- `brain` (optional, overrides default for the aggression tier),
- `path_id` (optional patrol).

The runtime composes the appropriate behavior from `aggression`
without two separate spawn paths.

**Risk:** highest. Touches combat, AI, dialogue, save format,
content_validation. Deserves ADR 0010 (or 0011 if tilesets
take 0010).

**Recommended order if doing both 2.1 + 2.3:** Actor unification
first (clean schema), then tilesets (visual polish on top of the
new schema).

### 2.4 GridVania world layout for intro.ldtk

**What:** intro.ldtk uses `worldLayout: Free` with rooms at
x=100000, 102000, 104000, … (large gaps). The EdgeExit
transitions work but the LDtk editor view shows isolated rooms.
GridVania would snap rooms to a world grid + place them
adjacently.

**Steps:**
1. Flip `worldLayout: GridVania` in `world init` defaults.
2. Set `worldGridWidth: 64`, `worldGridHeight: 64` (or smaller).
3. Re-pack intro level world coords so adjacent rooms share an
   edge.
4. Verify the runtime active-area switching still fires correctly
   on edge crossings (may need a small adjustment to detect the
   player's containing LDtk level and update RoomSet).

**Risk:** medium. Mostly authoring + a runtime smoke test.

---

## 3. Smaller story-content asks

(All from the design doc; each is additive.)

- **Faster creator final fragments** (skill-route variants):
  `creator_final_fast`, `creator_final_impossible` dialogue ids
  with longer lines. The intro_raid cutscene picks the variant
  based on player speed.
- **Ripple interaction trigger** in `gate_stack_lower` — a
  `Switch`-like entity that fires the `first_ripple` cutscene
  the first time the player overlaps it.
- **Erdish appearances** in drain alley / gate stack —
  `NpcSpawn` with `dialogue_id: erdish_first`; sprite already
  registered in `INTRO_NPC_SPRITE_REGISTRY` and rendered via
  `oiler_spritesheet`-style toon-side adapter.
- **Galwah duel** scripted set-piece in a future town zone.
- **Real Manifest / Pirate / Ninja / Nazi-fortress routes** —
  each is a new `.ldtk` file authored with `world init` and
  added to `SECONDARY_WORLD_FILES`. The gate-stack labels
  already signpost where they go.
- **Return-to-lab unlock path** — secondary world `lab_ruins.ldtk`
  with a story-flag-gated `LoadingZone`.

---

## 4. Tooling gaps the work above will surface

- **`def update-entity` / `entity field add`** — currently no way
  to add a new field to an existing LDtk entity def via the
  tools. The portal v1 work would have benefited from a
  `required_switch` field on LoadingZone instead of a side
  registry; same constraint will hit the Actor unification work
  (need to add `aggression` to a unified Actor entity).
- **`tileset add`** — for §2.1.
- **`intgrid paint`** — currently TODO in cli.py. Useful for
  hand-painting Solid stripes after `area create` without
  re-authoring the level from spec.

---

## 5. Pre-existing tech debt revealed during this work

- `intro/tests.rs` and `intro/banter::tests` should be moved to
  `#[cfg(test)] mod tests` blocks inside the consumer files
  rather than living in dedicated test files. Currently fine
  but inconsistent with the rest of the crate.
- The `apply_feature_damage_events` system is enormous (>250
  lines). The new combat-banter hook lives inside the enemy
  branch alongside damage handling, which is the right place
  but the file is begging for a split (one system per actor
  variant, or per damage-source type).
- `CharacterAnim` has overloaded variants (Walk → "stable"
  portal anim, Run → "closing" portal anim). When the new
  Prop entity lands, consider replacing this with a per-prop
  custom anim enum so prop sheet variants don't borrow
  character-animator slots.
