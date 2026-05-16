# Intro vertical slice — handoff to the next agent

## Completed by autonomous follow-up

(Started 2026-05-16; agent committed to working until backlog is zero per
`docs/intro_autonomous_followup_prompt_v3.md`.)

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
