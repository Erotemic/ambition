# Articulated body rigs and runtime part animation — implementation handoff

**State:** DISJOINT / TRIAGE — this document does not change queue priority. It is an implementation-ready design report based on repository discovery completed against `ac775a11351d` on 2026-09-29.

**Purpose:** add an optional semantic body-rig abstraction for deterministic articulated geometry, collision, and attachments, plus an optional runtime character-presentation path that reuses rasterized body parts instead of storing every pose as a complete raster frame. Baked sprite sheets remain first-class. Ragdoll is a later optional pose provider, not part of the initial implementation.

**Rollout (2026-10-01):** Jon gave the go-ahead to put the rigged sprites into the shipped game. `RiggedSpriteAdmission` is now ON unless the composition inserts it off or the environment says `AMBITION_RIGGED_SPRITES=0` (`from_setting`; witness `the_flipbooks_are_on_unless_the_environment_turns_them_off`, and in the app `the_shipped_game_draws_the_admirals_from_their_parts`, poison: unset reads as off). A character that publishes a flipbook (the five pirates; Mary-O's three forms, whose walk is drawn from parts and every other row baked) draws from it; every other character draws its baked sheet. The baked sheet stays resident as the parity oracle and the portal candidate (Packet 7), so a rigged character holds about 1.62 × its baked texture bytes until the root stops drawing (Packet 8, step 2). With the switch on by default, the app suite and six other crates show no new red. `BodyRigAdmission` (the gameplay rigs: hurt parts, attachments) is a separate switch and is still off by default. Packet 8's GPU run is `scripts/rig_packet8_gpu_bench.py` (see Packet 8).

This document intentionally contains the discovery work that an implementation agent would otherwise have to repeat. The implementation agent should verify that named symbols still exist after rebases, but should not begin with another architecture survey or asset-economics study.

Related current owners:

- [`render-animation-and-vfx.md`](render-animation-and-vfx.md) owns the simulation-to-presentation boundary and drawable scheduling;
- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md) owns demand, preparation, device materialization, quality, and residency;
- [`performance-and-iteration.md`](performance-and-iteration.md) owns runtime and memory evidence;
- [`sprite-renderer.md`](sprite-renderer.md) and the sprite-renderer repository own authoring and publishing contracts;
- [`svg-component-character-migration.md`](svg-component-character-migration.md) covers component-oriented character authoring where it is already useful;
- [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md) owns view-local presentation requirements.

## Executive implementation decision

The repository already contains enough evidence to justify implementation. Do not add a discovery phase.

Use two different production characters for two different proofs:

1. **Pirate family — first runtime part-rendering prototype.**
   - The authoring pipeline already captures a deduplicated component scene.
   - All five measured pirates have 38 baked poses but only 21–22 registered rigid parts.
   - A conservative representation that deduplicates only those existing rigid parts and leaves current dynamic limb/neck geometry as one per-frame overlay reduces estimated packed texture pixels by **75.6–76.3%**. ⚠ Superseded: the publisher measures about **38%**, because each frame also needs overlay layers for its dynamic geometry (see *Measured by the Packet 5 publisher*).
   - The Pirate Admiral already has a real `Muzzle::Hand` gameplay consumer, so the same family can also prove one semantic hand attachment without inventing a toy customer.

2. **Mary-O — first semantic `BodyRig` / collision prototype.**
   - The shipping sprites already come from a production SVG rig. The renderer source explicitly states: **“THE SVG RIG IS MARY-O NOW.”**
   - The runtime already has deterministic `BodyPoseClock`, move playback, authored hurtboxes, rollback-derived hurtbox resolution, and body-geometry consumers.
   - Keep Mary-O visually baked during the first semantic-rig packets. The rig initially changes simulation geometry and attachments, not her rendering.

Use a third role for Mary-O later:

3. **Mary-O — hybrid visual control.**
   - Ordinary locomotion is a good rigid-rig candidate.
   - Grow/shrink/transform/fire effects use bespoke compositing and should remain baked until hybrid clip support exists.
   - Do not require one representation for every clip.

The first visual runtime format is an **offline-solved transform flipbook**, not a runtime bone solver:

```text
authoring rig / component scene
        ↓
offline solve
        ↓
part atlas + resolved per-frame draw transforms
        ↓
Rust part player
```

This is already proven by the `vanity_card_made_this_meme` exporter and runtime player. Do not evaluate Python rigs, IK, SVG constraints, or vector deformation in the game.

The first body-rig pose is **derived deterministic simulation state**, not canonical rollback state:

```text
PreparedBodyRigDefinition
+ MovePlayback / BodyPoseClock / facing
        ↓
BodyRigPose       # rebuilt before every simulation consumer
```

The renderer may consume a corresponding presentation realization, but rendered frames and draw transforms never become gameplay authority.

## Why this work is worth doing

Ambition currently publishes most character animation as baked sprite-sheet frames. That is simple and fast at runtime, but it repeats the same pixels across many poses. The cost grows quickly with:

- larger move and social-animation repertoires;
- open-world populations;
- clothing and equipment variations;
- multiview;
- weak target GPUs;
- runtime content where small visual changes should not require another large sheet.

The repository measurements below show that this is not merely theoretical. For the current pirate family, a conservative part representation can remove about three quarters of the raw texture pixels without first solving fully articulated reusable limbs.

There is a second architectural benefit that is independent of texture savings: **semantic articulated body geometry**. A deterministic body rig can provide heads, hands, feet, limbs, and attachment points while the visible actor remains one baked sprite. This gives collision and gameplay consumers a structured body without making the renderer authoritative.

The target architecture is therefore:

```text
shared semantic body topology
        │
        ├── deterministic body pose
        │       ├── collision / hurt geometry
        │       ├── attachments
        │       └── later physicalized pose provider
        │
        └── optional visual realization
                ├── baked sheet
                ├── part atlas + transform flipbook
                └── hybrid per-clip realization
```

## Completed repository discovery

### Current source snapshot

Discovery was completed against repository commit:

```text
ac775a11351dddde78b88430aa901e7fe2f92bba
```

The source remains authoritative after rebases. If a named symbol moves, follow it rather than recreating the old path.

### Pirate authoring already exposes deduplicated component scenes

Key files:

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_pirate_common.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_pirate_rig.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/sheet_build.py`
- `tools/ambition_sprite2d_renderer/tests/test_pirate_svg_fidelity.py`

`_pirate_common.render_target` captures every pirate frame into one component scene. Registered rigid parts are emitted once and frames refer to them through placements. The fidelity test explicitly treats the pirate as a deduplicated rigid-paper-doll scene and checks that the component scene matches the source rendering.

The current pirate animation rows are:

| Clip | Frames |
|---|---:|
| idle | 6 |
| walk | 8 |
| slash | 6 |
| taunt | 6 |
| hurt | 4 |
| death | 8 |
| **total** | **38** |

The current component scenes do **not** make every visible pixel a reusable rigid part. Limb and neck geometry still appears as per-frame dynamic geometry. The measured alternative below deliberately preserves that dynamic geometry as one per-frame overlay. This makes the savings estimate conservative and keeps the first runtime format compatible with current published art.

### Measured pirate texture savings

The table below compares the current generated baked sheets with a conservative alternative:

```text
unique registered rigid-part rectangles
+ one tight dynamic-overlay rectangle per frame
```

The alternative applies the current sheet's measured packing-overhead ratio to the tight alternative pixels. It therefore compares approximately equal packing behavior instead of comparing a tight part set against a padded baked atlas.

`RGBA8-equivalent` means `atlas width × atlas height × 4`. The current character image-loading road does not explicitly configure mipmaps, so no mip overhead is included.

| Character | Frames | Unique rigid parts | Rigid part uses | Current atlas | Current packed texels | Current RGBA8-equivalent | Tight current pose texels | Tight part + dynamic texels | Est. packed alternative | Est. RGBA8-equivalent | Est. texture-pixel saving |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `pirate_raider` | 38 | 22 | 342 | 640×563 | 360,320 | 1.3745 MiB | 276,059 | 67,246 | 87,771 | 0.3348 MiB | **75.64%** |
| `pirate_admiral` | 38 | 21 | 304 | 639×566 | 361,674 | 1.3797 MiB | 281,396 | 66,697 | 85,725 | 0.3270 MiB | **76.30%** |
| `pirate_quartermaster` | 38 | 22 | 342 | 640×563 | 360,320 | 1.3745 MiB | 276,059 | 67,246 | 87,771 | 0.3348 MiB | **75.64%** |
| `pirate_lookout` | 38 | 21 | 304 | 640×563 | 360,320 | 1.3745 MiB | 276,059 | 67,204 | 87,717 | 0.3346 MiB | **75.66%** |
| `pirate_navigator` | 38 | 21 | 304 | 640×563 | 360,320 | 1.3745 MiB | 276,059 | 67,204 | 87,717 | 0.3346 MiB | **75.66%** |

Current PNG disk sizes are 298,267, 310,272, 286,048, 293,361, and 297,146 bytes respectively. No alternative PNG disk-size claim is made because compression behavior has not been measured and is not the primary runtime-memory question.

The draw-count tradeoff is also concrete:

- Raider and Quartermaster average `342 / 38 = 9` registered rigid parts per frame plus one dynamic overlay: about **10 quads per actor**.
- Admiral, Lookout, and Navigator average `304 / 38 = 8` registered rigid parts per frame plus one dynamic overlay: about **9 quads per actor**.

The transform payload is small relative to texture pixels. If the first runtime draw record is kept at or below 32 bytes:

- Raider / Quartermaster: `(342 + 38) × 32 = 12,160` bytes maximum uncompressed draw-table payload.
- Admiral / Lookout / Navigator: `(304 + 38) × 32 = 10,944` bytes maximum.

The texture term still dominates by a wide margin.

### Measured by the Packet 5 publisher (2026-09-30)

⚠ **The 75% estimate above is not correct. The publisher measures about 38%.**

The publisher (`authoring/part_flipbook.py`) keeps every rigid part as one raster and keeps the dynamic geometry as per-frame overlays. It measures these values:

| Character | Rigid parts | Overlays | Draws per frame | Tight texels | Packed texels | Current atlas | Texel saving | Worst frame parity |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `pirate_raider` | 23 | 152 | 13.0 | 183,752 | 222,336 | 360,320 | **38.3%** | 1.06% |
| `pirate_admiral` | 22 | 152 | 12.0 | 184,619 | 224,064 | 361,674 | **38.1%** | 1.95% |
| `pirate_quartermaster` | 23 | 152 | 13.0 | 183,752 | 222,336 | 360,320 | **38.3%** | 1.03% |
| `pirate_lookout` | 22 | 152 | 12.0 | 183,584 | 222,336 | 360,320 | **38.3%** | 1.25% |
| `pirate_navigator` | 22 | 152 | 12.0 | 183,584 | 222,336 | 360,320 | **38.3%** | 1.24% |

Why the estimate is wrong:

1. The dynamic geometry is 92% of the texels. For the raider, the overlays are 164,214 texels and the rigid parts are 14,347.
2. The method of the estimate does not give its number. One tight overlay per frame (the method above) measures 166,662 overlay texels for the raider. The estimate needs about 53,000. The drawn pixels alone (alpha above zero) are 62,837.
3. One overlay per frame cannot keep the z-order. The dynamic geometry is 4 or 5 runs per frame, and parts are painted between the runs (the legs under the boots, the back arm under the torso). Thus each run is one overlay. This does not increase the texels (per-run boxes: 164,214; one box per frame: 166,662), but it adds draws: 12 to 13 per frame, not 9 to 10.

A larger saving needs reusable limb parts. A pirate limb is a stroke between two joints, so a limb segment can become a rigid part. That is publisher work after the first world realization.

Parity: every frame of the five pirates is recomposed from the published atlas and draw table and compared with the published sheet frame. A pixel is wrong when no pixel within one pixel of it in the other image is within 64 in all premultiplied RGBA channels. The worst frame is 1.95%, and the test bound is 2.5%. Real defects are above the bound: a dropped hat is 8.5%, a torso two pixels off is 7.9%, and a sword turned the wrong way is 4.0%. A part one pixel off is 0.47%, which the bound accepts.

**Implementation consequence (original):** the visual prototype is justified now. Do not rerun an asset-economics discovery study before implementing it. Validation should reproduce these numbers from the new publisher and explain material deviations.

### Existing transform-flipbook precedent: vanity card

Key files:

- `tools/ambition_sprite2d_renderer/scripts/export_director_vanity_card.py`
- `game/ambition_content/src/presentation/vanity_card_made_this_meme.rs`

The exporter already establishes the exact first runtime strategy:

```text
Python owns rig / IK / choreography
        ↓
offline solve
        ↓
part images rasterized once
+ per-frame part placements
        ↓
Rust draws quads
```

Important existing behavior:

- `PartAtlas` rasterizes each rigid part once.
- The exporter stores resolved per-frame part positions and rotations.
- `part_draws` explicitly rejects non-rigid vector-deforming parts instead of pretending they are rigid.
- `--verify` recomposites exported placements and diffs them against the canonical direct renderer.
- `pack_atlas` produces one packed image for all parts.
- The Rust player allocates a fixed maximum number of part slots once, then reuses and hides them per frame rather than spawning and despawning per frame.

This path is UI rather than world-actor presentation. It proves the data model and publisher verification strategy. It does not prove world-actor portal integration or 100-actor CPU cost.

**Resolved choice:** the first world format is a transform flipbook. Runtime skeletal/keyframe solving is deferred until a real customer requires interpolation or procedural pose synthesis.

### Mary-O already ships from a production SVG rig

Key files:

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/mary_o_v2.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_mary_o_v2_model.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_mary_o_v2_svg_poc.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/mary_o_v2_svg_poc.py`
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_mary_o_v2_gameplay.py`
- `tools/ambition_sprite2d_renderer/assets/mary_o_v2.svg`

The shipping source states:

> THE SVG RIG IS MARY-O NOW.

Frames come from the artist-edited SVG, including art, pivots, and z-order. The old procedural road seeded the SVG but no longer ships the frames.

The current rig is small enough to publish as semantic topology without inventing a new skeleton:

| Form/view | Bones | Notable parts / clips |
|---|---:|---|
| short side | 6 | far leg, near leg, torso, near arm, far arm, head; idle/walk/jump/skid/climb/swim |
| short front | 6 | left/right legs, torso, head, left/right arms; death expression; death clip |
| tall side | 6 | same basic side topology; idle/walk/jump/skid/crouch/climb/swim/crouch-walk/crouch-jump |
| tall front | 6 | front topology plus death expression |
| fire side | 7 | adds `torso_back` / back-wings relationship; locomotion plus fireball and transform-related content |
| fire front | 7 | front topology plus back-wings and death expression |

Normal body parts are sprite parts bound to bones with authored pivots. `RigDocument.sprite_raster` already provides reusable cropped rasters and pivots.

Current baked Mary-O sheet baselines:

| Form | Baked frames | Current atlas | Packed texels | Tight pose texels | Packing ratio | RGBA8-equivalent | PNG bytes |
|---|---:|---:|---:|---:|---:|---:|---:|
| short | 25 | 618×514 | 317,652 | 268,331 | 1.184× | 1.212 MiB | 148,958 |
| tall | 30 | 823×676 | 556,348 | 475,655 | 1.170× | 2.122 MiB | 233,795 |
| fire | 32 | 908×797 | 723,676 | 651,394 | 1.111× | 2.761 MiB | 313,224 |
| **combined** | **87** | — | **1,597,676** | **1,395,380** | — | **6.094 MiB** | **695,977** |

No Mary-O part-atlas saving percentage is claimed yet. That measurement is not needed to choose Mary-O for the semantic body-rig proof.

Why Mary-O is **not** the first visual prototype:

- grow/shrink/transform/big-shrink use bespoke transition art;
- fire effects use palette/effect compositing;
- visual migration would immediately require hybrid clips.

Why Mary-O **is** the first semantic prototype:

- the production rig already exists;
- gameplay already owns deterministic pose timing;
- current authoring already publishes gameplay body/hurtbox information;
- baked rendering can remain unchanged while semantic joints and collision are introduced.

### Current deterministic hurtbox and pose authority

Key files:

- `crates/ambition_combat/src/hurtbox_resolution.rs`
- `crates/ambition_combat/src/components/features.rs`
- `crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs`
- `crates/ambition_platformer2d_actor_monolith/src/features/mod.rs`
- `crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs`

Current semantic facts:

- `AuthoredHurtboxes(HurtboxDoc)` is authored simulation data.
- `BodyPoseClock { pose, elapsed_s }` is authoritative simulation state and is rollback registered.
- Move-specific timing comes from `MovePlayback.t`.
- `ResolvedHurtboxes` is rollback-derived and recomputed in simulation.
- `resolve_body_hurtboxes` has no renderer dependency.
- `DamageableVolumes` already accepts multiple exact combat volumes while exposing a coarse bounds union.
- `CenteredAabb` / the body envelope already serves the coarse actor footprint role.

The current scheduling road is approximately:

```text
CombatSet::Playback
        ↓
advance_body_pose_clocks
        ↓
resolve_body_hurtboxes          # BodyHurtboxesResolved
        ↓
refresh_body_damageable_volumes
        ↓
CombatSet::Resolve
```

The larger combat ordering is:

```text
Trigger → Playback → Materialize → Resolve → Settle
```

**Resolved choice:** `BodyRigPose` is deterministic derived simulation state. Rebuild it after `Playback` and before every consumer. Because attachments can affect projectile materialization, the rig-pose publication must be available before `Materialize`, not merely before `Resolve`.

A suitable schedule shape is:

```text
CombatSet::Playback
        ↓
advance_body_pose_clocks
        ↓
resolve_body_rig_pose            # BodyRigPoseResolved
        ├── rig attachment consumers before Materialize
        └── rig hurtbox resolver
                ↓
CombatSet::Materialize
        ↓
refresh_body_damageable_volumes
        ↓
CombatSet::Resolve
```

Exact Bevy set wiring should preserve the repository's existing combat set ownership. Do not create a parallel schedule.

`BodyRigPose` itself should be recorded in rollback documentation as derived state, not serialized canonical state. If a later pose provider adds future-affecting canonical state, that new state must be registered separately.

### Character authoring, preparation, and construction seam

Key files:

- `crates/ambition_characters/src/actor/definition.rs`
- `crates/ambition_characters/src/prepared.rs`
- `crates/ambition_platformer2d_actor_spawn/src/character_body.rs`

Current architecture:

- `CharacterDefinition` owns authored character facts such as sheet, body source, and hurtboxes.
- `PreparedCharacterDefinition` / prepared overrides are immutable admitted runtime facts.
- `CharacterBodyBlueprint` carries construction-ready facts.
- `grant_prepared_character_body` is the one body-grant seam.
- `GrantedBodyFacts` tracks what the worn-character projection placed on a body and retracts those facts when identity changes.

**Resolved ownership:** semantic rig data belongs with character semantics in `ambition_characters`, not in `ambition_render`, `ambition_sprite_sheet`, or Python authoring code.

Add optional semantic rig data to the authored/prepared character definition. Validate it during preparation. Carry it through `CharacterBodyBlueprint`. Grant it in the same construction batch. Add its provenance to `GrantedBodyFacts` so re-wearing another character removes outgoing rig facts correctly.

Do not add a system that attaches a rig one tick after construction.

The runtime format must be serialized/published content. Rust must not import SVG or Python rig semantics directly.

### First real attachment customer already exists: `Muzzle::Hand`

Key files:

- `crates/ambition_characters/src/brain/action_set/mod.rs`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs`
- `crates/ambition_mount/src/lib.rs`
- `game/ambition_content/assets/data/character_catalog.ron`
- `game/ambition_app/tests/admiral_gun_sword.rs`

The semantic action vocabulary already contains `Muzzle::Hand { ahead }`.

Current `muzzle_world_pos` resolves that semantic request through `ambition_mount::rider_hand_world_pos_in_frame(...)`, which uses the generic `HAND_OFFSET_NORM = (0.18, -0.05)` heuristic. Its own comment says the offset is sprite-layout-derived but must live in simulation because projectile origin is gameplay state.

The Pirate Admiral's gun-sword action already authors a hand muzzle. This is the correct first non-collision body-rig customer.

**Resolved migration:**

```text
Muzzle::Hand
        ↓
if body has BodyRigPose hand attachment:
    use semantic hand attachment + ahead
else:
    use existing compatibility heuristic
```

The fallback remains for non-rigged characters. Do not remove `HAND_OFFSET_NORM` until all supported hand-muzzle users have a semantic hand attachment.

### Runtime character presentation seam

Key files:

- `crates/ambition_sprite_sheet/src/character/mod.rs`
  - `CharacterSpriteAsset`
  - `CharacterSpritePage`
  - `build_character_presentation_with_render_size`
- `crates/ambition_sprite_sheet/src/character/animator.rs`
  - `CharacterAnimator`
- `crates/ambition_render/src/rendering/actors/mod.rs`
  - `character_render_basis`
  - `bind_worn_character_presentation`
- `crates/ambition_render/src/rendering/actors/animation.rs`
  - `apply_character_frame`
  - `animate_characters`
- `crates/ambition_render/src/rendering/mod.rs`
  - `BodyOwnedDrawableSync`
- `crates/ambition_sim_view/src/anim_index.rs`
  - `ActorAnimIndex`
- `crates/ambition_sim_view/src/pose_view.rs`
  - `BodyPoseView`

Current actors use one `Sprite` plus `CharacterAnimator`. `CharacterAnimator` already owns:

- semantic `CharacterAnim` selection;
- optional clip slot/phase;
- move-normalized clip phase;
- facing/mirror behavior;
- frame time;
- render basis.

`ActorAnimIndex` and `BodyPoseView` are presentation/read-model seams. Headless simulation does not pay for sprite assets.

**Resolved design:** do not turn `CharacterAnimator` into a universal sheet/rig union. Add a sibling rigged realization type, but share semantic clip/facing/timing helpers above the realization boundary. Do not create a second animation-policy vocabulary.

### Asset demand, quality, and residency seam

Key files:

- `crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs`
- `crates/ambition_sprite_sheet/src/character/assets.rs`
- `crates/ambition_sprite_sheet/src/game_assets/mod.rs`

Current architecture already supplies:

- lazy/demanded character sheet realization;
- requested and resolved quality tiers;
- strong-handle release for eviction;
- `load_sheet_image` as the image-demand funnel.

**Resolved design:** `RiggedSpriteAsset` must use the same demand/readiness/quality/retirement authority. Do not add a second cache or a separate quality selector. Use the existing image-loading funnel for part-atlas pages.

Track texture bytes and transform metadata separately in telemetry.

### Portal integration

Key file:

- `crates/ambition_render/src/rendering/portal_compositing.rs`
  - `publish_portal_compositing_candidates`

Current portal candidate publication reads a top-level sprite or declared mesh and publishes one world-space `PortalCompositingCandidate { drawn_centre, drawn_half }` per drawable. Child sprites are deliberately excluded because their local `Transform` is not a valid pre-propagation world transform.

**Resolved first implementation:** the rig presentation owner publishes **one union world-space drawn AABB** covering all currently visible part instances. Portal composition continues to see one actor-level candidate.

**As built (Packet 7):** the one candidate is the rigged root with its baked frame, not a union AABB on the owner. See the Packet 7 status for the reason.

Do not publish one portal candidate per body part.

This keeps initial portal fidelity equivalent to the current one-rectangle full-sprite treatment. Per-part portal clipping is later work only if visible artifacts justify it.

### Multiview integration

Relevant concepts:

- `ambition_sim_view::LocalView`
- `LocalViewId`
- `PresentsView`
- `PresentedForView`
- `crates/ambition_render/src/rendering/view_isolation.rs`

**Resolved design:** world rig parts belong to one shared world presentation. Multiple cameras view the same world drawables. Do not create one rig pose, animator, or part population per `LocalView`.

Only genuinely view-local UI/presentation receives `PresentedForView`.

TwinTrack split view is the acceptance witness that visible part entity/instance count does not multiply merely because the scene has two views.

## Architectural rules

### One semantic body authority

A rigged body's deterministic pose is a simulation fact. The renderer consumes it or consumes presentation data derived from the same semantic animation state. The renderer never tells simulation where a hand, head, or hurt volume is.

Preferred flow:

```text
authored / prepared rig definition
        ↓
rollback-authoritative pose clocks and move state
        ↓
derived BodyRigPose
        ├── collision
        ├── attachments
        └── presentation realization
```

Do not introduce:

```text
render sprite transforms
        ↓
read back into gameplay
```

### Separate topology, pose, and visual realization

Use three concepts:

```text
BodyRigDefinition       # semantic topology / attachments / collision parts
BodyRigPose             # current deterministic resolved transforms
RiggedSpriteAsset       # optional visual atlas + flipbook draws
```

A character can have `BodyRigDefinition` without `RiggedSpriteAsset`.

That is the Mary-O first proof:

```text
Mary-O
    semantic body rig: yes
    rig-based collision: yes
    baked sprite renderer: unchanged
```

### Do not replace movement/body physics in the first migration

The initial body rig drives:

- hurt/damage geometry;
- attachment points;
- optional semantic hit-part metadata.

It does **not** replace:

- `BodyBaseSize`;
- stance/locomotion collision;
- whole-body movement physics;
- `CenteredAabb` broad-phase identity.

Those systems have different gameplay constraints. Do not turn this project into a character-controller rewrite.

### Keep broad phase coarse

Retain the coarse body envelope:

```text
attack volume
    ↓
coarse actor overlap?
    no → done
    yes
      ↓
exact rig hurt parts
```

The rig improves articulation and semantic placement. It does not require pixel-perfect collision.

Use simple shapes such as circles, capsules, and boxes.

### Move-specific authored overrides remain legitimate

Current `HurtboxDoc` can describe move- or pose-specific behavior. A semantic body rig should replace the generated/default body silhouette for a migrated character, not erase deliberate move-specific exceptions.

Use one explicit precedence rule:

```text
move-specific authored hurtbox override
        > rig-derived default articulated hurt geometry
        > legacy static/default compatibility geometry
```

Do not run two same-priority default hurtbox authorities and reconcile them.

### Part IDs are optional combat metadata

A detailed hit may optionally identify:

```text
body = actor 72
part = Head
contact = ...
```

This can support weak points, headshots, shields, breakable appendages, or localized reactions.

Do not require every limb to have independent health. The normal damage model remains actor-owned.

### Pose authority changes explicitly for ragdoll

Normal animated simulation:

```text
rollback gameplay state
→ animation/body-pose clocks
→ BodyRigPose
```

A later gameplay-authoritative ragdoll would instead use:

```text
rollback articulated physics state
→ BodyRigPose
```

Do not let animation and ragdoll both write the same joint transforms.

Do not add a general `BodyPoseAuthority` enum until a second pose provider actually exists. The first implementation has one provider: animated deterministic pose resolution.

## Semantic body-rig data model

The exact Rust spelling can follow repository conventions, but preserve these responsibilities.

### `BodyRigDefinition`

Prepared immutable semantic data owned by the character definition.

Minimum content:

```text
BodyRigDefinition {
    joints: stable semantic joint ids + parent topology,
    clips/poses: resolved local transforms or references to prepared pose samples,
    attachments: named semantic anchors bound to joints,
    hurt_parts: optional simple collision parts bound to joints,
}
```

Do not include texture handles, materials, atlas rectangles, or Python/SVG concepts.

Stable semantic joints should be typed/string-stable content IDs, not ECS entity IDs.

### `BodyRigPose`

Derived per-body simulation state:

```text
BodyRigPose {
    resolved joint transforms in body-local simulation space,
    resolved semantic attachment points,
    optional resolved part transforms needed by collision,
}
```

It is rebuilt from prepared rig data plus authoritative move/pose clocks before consumers.

Do not persist it in saves as independent truth.

Do not make it a presentation-only component if simulation reads it.

### Mary-O minimal first topology

Publish the side-view production rig as the first semantic body topology:

```text
root / feet origin
├── torso
├── head
├── far_leg
├── near_leg
├── far_arm
└── near_arm
```

For fire form, preserve the existing additional back/torso relationship needed by the authored rig.

First semantic attachments:

```text
head
hand_near
hand_far
foot_near
foot_far
```

These can be published from existing bone endpoints/pivots. Do not invent runtime image analysis.

First collision-part set should be deliberately small:

```text
head   → circle or capsule
torso  → capsule / rounded box
legs   → two simple capsules
```

Arms can remain outside default hurt geometry initially if adding them changes Mary-O's current damage envelope unnecessarily. The point is to prove articulated default geometry without broadening damage semantics.

### Facing

Keep one facing authority.

`BodyRigPose` resolves semantic left/right or near/far attachments according to body facing. Presentation mirroring consumes the same facing result.

Do not infer gameplay-facing from a flipped sprite transform.

## Runtime visual data model

### `RiggedSpriteAsset`

The first published visual asset is a transform flipbook.

Required fields:

```text
RiggedSpriteAsset {
    atlas_pages,
    parts: [
        atlas page,
        atlas rect,
        authored pivot,
    ],
    clips: {
        semantic clip id → frame sequence,
    },
    frames: [
        ordered draws,
    ],
    frame timing,
    logical render basis / feet anchor,
    quality-tier metadata,
}

PartDraw {
    part_index,
    local centre,
    rotation,
    scale / size,
    z order,
}
```

Use a compact runtime index such as `u16` for part selection when practical. Keep the packed runtime draw record at or below 32 bytes unless alignment or a demonstrated requirement makes that impossible.

A missing part draw means the part is hidden for that frame.

### Dynamic overlays are valid first-class draws

For the pirate first implementation, current per-frame dynamic limb/neck geometry should be rasterized as one dynamic-overlay atlas entry per frame and included as one normal draw.

Do **not** block the project on converting every current dynamic curve into a reusable rigid limb.

The initial saving without that work is about 38% (measured by the Packet 5 publisher). The 75% estimate before it is superseded.

Later publisher work can convert more dynamic geometry into reusable parts if useful.

### Semantic animation remains above visual realization

Keep current semantic concepts such as `CharacterAnim`, clip slot/phase, move-normalized progress, and facing.

The realization boundary chooses:

```text
baked CharacterSpriteAsset
or
RiggedSpriteAsset
```

Do not introduce `RigAnim::Run`, `RigAnim::Jump`, etc. as a second semantic vocabulary.

### First world renderer uses fixed reusable part slots

Use the vanity-card strategy for the first world implementation:

- create one rigged presentation owner;
- allocate a fixed maximum number of child `Sprite` slots needed by the asset;
- reuse/hide those slots as frames change;
- put parts on one atlas/material where practical;
- do not spawn/despawn child entities every frame.

These child sprites are disposable presentation objects, not semantic body parts or gameplay entities.

This is the lowest-risk implementation because the repository already has the same pattern in production UI and Bevy can batch sprites sharing texture/material state.

Do **not** build a custom render-extraction pipeline before measuring the fixed-slot world path.

If runtime validation shows that child-entity/extraction cost is unacceptable at open-world population sizes, a later optimization can replace the realization with a compact extracted part-instance buffer without changing `RiggedSpriteAsset`, `BodyRigDefinition`, or semantic animation ownership.

### Portal bounds come from the owner

Because child sprites are excluded from the current pre-propagation portal candidate query, the rigged presentation owner computes one union draw AABB from current part transforms and publishes one portal candidate.

**As built (Packet 7):** the root is the candidate instead, and the owner hides with it. See the Packet 7 status.

Do not make every child part a portal entity.

### Hybrid visual realization

Long-term clips may choose different realizations:

```text
idle / walk / run / talk   → part flipbook
transform / smear          → baked clip
body + weapon glow         → part body + effect overlay
```

Mary-O is the first hybrid control. Do not migrate her visual representation until Pirate proves the world part-player road.

The first hybrid implementation should select realization per prepared clip, not by runtime heuristics.

## Authoring and publishing contract

### Preserve authoring plurality

The sprite renderer may continue to use:

- SVG rigs;
- procedural Python;
- component scenes;
- rig documents;
- bespoke composites;
- hybrids.

The game consumes stable published products only.

### Pirate part publisher

Generalize the existing pirate component-scene output into a published runtime visual asset.

For each frame:

1. resolve all registered rigid placements;
2. rasterize registered parts once into tight part rasters;
3. rasterize remaining dynamic geometry into one tight per-frame overlay;
4. pack parts and overlays into the part atlas;
5. publish ordered local transforms relative to the same logical render basis used by the baked sheet;
6. preserve current z-order;
7. publish the same semantic clip row names and timing.

Add a publisher verification mode equivalent to the vanity-card `--verify` road:

```text
published atlas + draw table
        ↓
offline recomposition
        ↓
pixel diff against canonical direct pirate renderer
```

The first visual packet is not complete until all 38 frames of the selected pirate pass this parity check.

### Mary-O semantic rig publisher

Use the existing production SVG rig as the source of semantic topology and sampled pose transforms.

Publish a runtime-neutral body-rig product alongside existing baked sheet products. Do not make Rust parse the SVG.

Use side-view topology for the first semantic gameplay product. Front death art can remain visual-only.

The published semantic product must be independent of texture tier. Collision and attachment positions are gameplay facts and must not move when Full/Half/Potato rendering changes.

### Prepared-generation compatibility

Semantic rig definitions and visual rig assets are prepared-generation content.

The active prepared generation chooses the immutable rig data used by new/reconstituted bodies. Do not mutate live topology row-by-row during asset reload.

Follow existing prepared-content generation semantics when this plan is implemented after that architecture is available.

## Implementation packets

There is no discovery packet. Start with Packet 1.

### Packet 1 — publish stable semantic body-rig content

**Primary customer:** Mary-O.

Work:

1. Add the serialized authoring/runtime-neutral body-rig schema in `ambition_characters` or the existing character-definition schema owner.
2. Add optional `body_rig` to `CharacterDefinition` and prepared character data.
3. Validate topology during character preparation:
   - unique joint IDs;
   - valid parent references;
   - no parent cycle;
   - valid attachment joint references;
   - valid hurt-part joint references;
   - finite transforms and dimensions.
4. Extend `CharacterBodyBlueprint` and `grant_prepared_character_body` to publish the prepared rig in the same construction batch.
5. Extend `GrantedBodyFacts` so re-wearing/retemplating retracts the outgoing rig fact.
6. Extend Mary-O publishing to emit the side-view semantic topology and sampled deterministic poses from the shipping SVG rig.
7. Keep Mary-O's existing baked visual output unchanged.

Acceptance:

- no post-construction “attach rig” system;
- invalid rig content is rejected during preparation;
- Mary-O construction produces the correct prepared rig immediately;
- changing from Mary-O to a character with no rig removes Mary-O's character-owned rig fact;
- headless construction requires no texture or render asset.

### Packet 2 — resolve deterministic `BodyRigPose`

**Primary customer:** Mary-O.

Work:

1. Add derived `BodyRigPose` simulation state.
2. Resolve it from prepared `BodyRigDefinition`, `BodyPoseClock`, `MovePlayback`, facing, and any existing deterministic semantic pose selector.
3. Add a named schedule boundary such as `BodyRigPoseResolved` in the existing combat/runtime schedule.
4. Ensure it runs after `CombatSet::Playback` and before `CombatSet::Materialize` for current-tick attachments.
5. Ensure every rig-based damage consumer also runs before `CombatSet::Resolve`.
6. Document/register the state as rollback-derived and rebuild it before any consumer after restore.

Acceptance:

- deterministic replay of the same pose clock gives identical resolved joints;
- rollback restore followed by one normal schedule pass rebuilds identical rig pose before collision/projectile consumers;
- no canonical future-affecting state is hidden inside the derived component;
- headless tests exercise the resolver with no render plugins.

### Packet 3 — make Mary-O's default damage geometry rig-driven

**Primary customer:** Mary-O.

**Fix (2026-09-30, review finding [P3]): a walking body is solved from its walk clip.** Before this fix the "walk" half of the acceptance below was not met. Two causes:

- Walking was not a simulation fact. `BodyPoseClock` now also carries a **gait** (`Standing`, `Walking`, `Running`, `Skidding`) and a gait clock, written by `advance_body_pose_clocks` from simulation facts only: the ground state, the velocity along the body's own run axis, and `BodyMotionFacts::running` / `skidding`. The rule is `grounded_gait`. The sprite picker's grounded Idle/Walk/Run branch calls the same rule with its own dead band (12 for a player, 8 for an actor), so the shipped rows do not change. ⚠ Corrected by review, 2026-09-30: the gait used to take the player's 12 when the body had a `PlayerBlinkCameraState`, so two bodies with the same motion differed in hurt geometry because of a camera component. The gait is now one body fact with one band for every body (`STANDING_BELOW` = 8). The row and the rig clip agree outside the player's band of 8 to 12. Witness: `a_camera_state_does_not_change_a_bodys_gait_or_rig_pose` (speed 11, with and without the camera state: same gait, clip, frame and hand. Poison: with the split restored, Standing/idle against Walking/walk). In the `idle` pose the rig asks for the gait's clip (`walk`, then `idle`; `run`, then `walk`, then `idle`; `skid`, then `idle`) on the gait clock. The hurtbox pose ids are not changed, so the authored pose profiles select as before.
- No production body had a `BodyPoseClock`: only tests spawned one. So every rigged body was solved as idle frame 0, standing or not. A body built with a rig now gets the clock in the same batch. ⚠ Corrected by review, 2026-09-30: the clock first came and went only with the rig, so the authored hurtbox POSE PROFILES (`doc.poses`) were never selected on a shipped body. The clock now follows its readers: a body gets it when it has a rig OR authors pose profiles (`reads_the_pose_clock`, the granted fact `pose_clock`). The shipped authors are the two versus duelists, each with a bigger `hitstun` box, so this changes shipped gameplay: a duelist in hitstun is now hit through its 13 × 24 box, as its catalog comment says it should be. The box changes one tick after the hit, because the hit lands after the clocks advance. No codec shape changes (the clock is clone-probed), so there is no schema bump. Witness: `a_duelist_in_hitstun_is_hit_through_its_authored_hitstun_box` (versus, rigs not admitted: 13 × 24 in hitstun, 11 × 22 after; poison: with the clock only on a rig, 11 × 22 in hitstun).
- Witness: `a_walking_mary_o_is_posed_from_her_walk_clip` (Mary-O app, rigs admitted). It drives her on the flat test course and checks three things: a moving gait solves `walk`, the walk frame advances, and her near hand leaves its idle place. It fails when the rig solve ignores the gait (poison run).


Work:

1. Add simple semantic hurt parts to the prepared Mary-O body rig.
2. Teach the hurtbox resolver to select rig-derived default body geometry when the character declares it.
3. Preserve explicit move-specific authored hurtbox overrides at higher precedence.
4. Keep `CenteredAabb` / whole-body footprint as coarse broad-phase geometry.
5. Do not change locomotion/body physics geometry in this packet.
6. Once the rig path is enabled for Mary-O's default body geometry, remove the same-priority generated pose-box authority for those default cases. Do not reconcile two defaults every tick.

Acceptance:

- idle, walk, jump, and crouch produce expected articulated head/torso/leg geometry;
- representative attacks still hit or miss Mary-O consistently with intended current gameplay;
- move-specific override rows still override the rig where authored;
- a broad-phase rejection avoids detailed part checks;
- the body remains fully damageable as one actor even when exact contact reports an optional semantic part.

### Packet 4 — replace the first hand-position heuristic with a rig attachment

**Primary customer:** Pirate Admiral.

Work:

1. Publish the Admiral's semantic hand attachment from the existing pirate rig/component authoring source.
2. Give the Admiral a prepared `BodyRigDefinition` sufficient for the hand anchor.
3. Extend `muzzle_world_pos` / its helper road so `Muzzle::Hand` uses the resolved hand attachment when available.
4. Preserve the current `rider_hand_world_pos_in_frame` / `HAND_OFFSET_NORM` compatibility path for characters without a rig.
5. Keep `ahead` authored in `Muzzle::Hand`; the rig supplies the hand origin, not weapon-policy distance.

Acceptance:

- `admiral_gun_sword` proves the projectile originates from the deterministic rig hand;
- facing changes mirror/resolve the hand correctly;
- non-rigged `Muzzle::Hand` users remain unchanged;
- projectile origin is simulation-owned and does not depend on a rendered child sprite.

### Packet 5 — publish Pirate transform-flipbook assets

**Status (2026-09-30): done** for all five pirates. See *Measured by the Packet 5 publisher*: the saving is about 38%, not 75%. The pirates publish `<target>_parts.png` and `<target>_parts.ron` beside the sheet. `tests/test_pirate_part_flipbook.py` checks the parity of all 38 frames for the raider and the admiral. Packet 6 added the tier tables: each tier publishes its own part rects and `texel_scale`.

**Primary visual prototype:** `pirate_raider` first, then the remaining pirate family once the format is stable.

Work:

1. Reuse the existing component scene rather than creating another pirate rig representation.
2. Rasterize registered rigid parts once.
3. Rasterize current dynamic geometry as one overlay per frame.
4. Pack one or more part-atlas pages through the sprite publishing road.
5. Publish compact ordered `PartDraw` records per frame.
6. Preserve the existing logical render basis / feet anchor.
7. Preserve semantic clip IDs, frame timing, facing behavior, and authored z-order.
8. Add offline parity verification against the canonical baked renderer.

~~Required measured target for Raider~~ — ⚠ SUPERSEDED (2026-09-30). The targets that were here (about 67,246 tight texels, about 87,771 packed texels, about 75.6% reduction, and "below roughly 65% is a packing regression") were an estimate before the publisher existed. They are not acceptance criteria. The publisher measures 183,752 tight and 222,336 packed texels for the Raider, a **38.3%** reduction, with every frame within the parity bound. The reason is in *Measured by the Packet 5 publisher*. The packet-5 acceptance is that measured result: the parity bound on every frame, and a saving of at least 35% (`tests/test_pirate_part_flipbook.py`). A drop below that is a packing regression to investigate.

Do not require every dynamic limb to become a reusable rigid part in this packet.

### Packet 6 — add runtime `RiggedSpriteAsset` demand and world presentation

**Status (2026-09-30): done, behind the trial switch.** The switch is `RiggedSpriteAdmission` (env `AMBITION_RIGGED_SPRITES`). It was off in every shipped game until 2026-10-01; it is on by default since then (see *Rollout* at the top), and `AMBITION_RIGGED_SPRITES=0` turns it off. With it off no part page is loaded and no body draws a part.

- `ambition_sprite_sheet::character::rigged::RiggedSpriteAsset` parses `<target>_parts.ron`. The draw table is baked into the build like a body rig. A quality tier has its own table (`<target>.<tier>`), with its own part rects and a `texel_scale`. A part keeps its full-resolution size and pivot at every tier.
- `scripts/generate_visual_quality_variants.py` publishes the tier part atlases. It crops each part from its page on its own, downsamples it by the tier factor of the sibling sheet, and packs it again. It never resizes a packed page.
- The pages ride on the sheet realization (`CharacterSpriteAsset::rigged`). Thus the demand, quality and retirement roads of the sheet own them. `attach_rigged_sprite_pages` loads them through `load_sheet_image` on the `character-parts` road.
- A body changes to its parts only when every part page is ready (`texture_is_ready`, the rule of the baked binder). Until then it keeps what it draws: its baked sprite, or the parts of its old tier. Then it changes in one frame (`a_body_stays_baked_until_every_part_page_is_ready`, `a_tier_change_keeps_the_old_parts_until_the_new_pages_are_ready`). ⚠ Corrected by review, 2026-09-30: a re-wear to another character also kept the old character's parts while the new pages loaded, and drove them with the new character's rows. Now only a tier change of one character keeps its parts. A re-wear drops them in the same frame, and the root draws the new character's baked sheet until its pages are ready (`a_rewear_drops_the_old_characters_parts_while_the_new_pages_load`; poison: the raider's parts stayed on the lookout's root).
- `ambition_render::rendering::actors::rigged` draws a rigged root. `CharacterAnimator` still selects the row, the frame and the facing, so clip timing and move phase are the same as for the sheet. A top-level owner follows the root. Its children are `max_draws` reusable part sprites. A frame change does not spawn or despawn them. The root keeps its baked sprite at zero alpha, and that sprite stays as the parity oracle.
- Measured in the real renderer (llvmpipe, `capture_scene hall_of_characters`, ultra, 16 shots over the idle cycle): each rigged shot matches the baked shot of the same tick, with a residual of 0.22 to 0.35 of the typical frame-to-frame difference. The pose, the sword angle, the feet and the size agree.
- At the potato tier, the rigged pirate stays readable: 12 or 13 exact quads, with textures of a few texels each. The baked potato frame is one 8 × 9 texture.

Gaps that remained after this packet, all closed later: the portal far side copied the root sprite, which has zero alpha (Packet 7: `PortalPieceTint`). The crouch squash of a sheet without a crouch row did not apply to parts (closed 2026-09-30, see Packet 7). The hit flash was listed here by mistake: its material samples the root's texture and frame with its own tint and never reads the sprite color, so a rigged body flashes with its baked silhouette.

**Primary visual prototype:** Pirate Raider.

Work:

1. Add `RiggedSpriteAsset` alongside current baked character sprite assets.
2. Route atlas-page loading through the existing character image demand/loading funnel.
3. Reuse requested/resolved quality authority; do not make a second quality selector.
4. Add a sibling rigged animator/realization that consumes the same semantic clip/facing/phase inputs as `CharacterAnimator`.
5. Create one presentation owner and a fixed reusable set of child sprite slots sized to the asset's maximum simultaneous draws.
6. Update slot atlas rect, local transform, visibility, and z-order from the selected transform-flipbook frame.
7. Reuse slots; never spawn/despawn parts per animation frame.
8. Drop strong atlas handles through the existing retirement/eviction road.
9. Keep the baked Pirate asset available as the parity oracle during this packet.

Acceptance:

- all 38 Raider frames match the canonical baked presentation within the publisher parity tolerance;
- clip timing and move-normalized phase match `CharacterAnimator` behavior;
- facing/mirroring matches the existing sheet path;
- body feet/render basis does not jump when switching test realization;
- the rigged asset is lazy/demanded, not eagerly loaded for the entire catalog;
- disabling the presentation plugin leaves headless simulation unaffected.

### Packet 7 — portal and multiview integration

**Status (2026-09-30): done, behind the trial switch.** One change from the plan: the portal candidate is the rigged ROOT, drawn from its baked frame. It is not a union AABB of the parts on the owner.

- Why: the compositor draws a far-side candidate as clipped pieces of ONE textured quad. A union AABB gives a rectangle, but a set of parts has no one texture to fill it. The baked frame of the root is that texture, and the flipbook matches it within the publisher parity bound. So the body through a portal is the actor-level rectangle of today, which is what the acceptance asks for.
- `ambition_portal2d_presentation::PortalPieceTint` is a tint that a candidate states for its pieces. The rigged root keeps zero alpha, so the driver states its visible tint there. Both piece builders (`far_side::piece_look` and `visuals::sync_portal_body_pieces`) read it before the sprite color.
- The owner is `PresentationOf(root)` and has no sprite. Thus it is not a candidate, and `resolve_portal_source_visibility` hides it in the same pass that hides the root. The parts and the pieces never draw together. Witness: `a_far_side_rigged_body_is_pieced_opaque_and_its_parts_hide_with_it` (`portal_compositing.rs`). It fails when the tint or the `PresentationOf` is removed.
- Multiview: the part slots take the render layers of their root, so each camera that draws a root draws its parts (`the_parts_are_drawn_by_each_camera_that_draws_their_root`). `a_second_view_draws_the_same_parts_and_makes_no_more` (`ambition_app`) adds a second pane to the seated admirals as TwinTrack does: a `LocalView` in a column and a `MainCamera` that presents it. The presentations, the slots and the entity count do not change. TwinTrack itself casts no character that publishes a flipbook, so the witness uses its pane shape, not its route.
- Not measured: without a window, the `VisibleEntities` of the host camera lists no sprite at all. Thus no headless test shows the pixels of each pane. The offscreen capture (`capture_scene`) can, when that is necessary.

- The crouch squash of a sheet without a row for the compact pose (`StanceSquash`) now reaches the parts. The driver reads the squash off the root itself: its drawn height and anchor against the animator's `current_render`. From these it gets the ratio and the line that holds still, and puts that squash on the owner's transform. So both of `StanceSquash`'s pivots (the anchor, the quad's foot edge) are followed with no copy of its rule, and rotated parts squash as the baked quad does. Witness: `a_squashed_root_squashes_its_parts_about_the_same_line`. It fails when the held line is dropped (poison run).
- The hit flash needed no change; see the Packet 6 note.

No known gap of the trial realization remains.

Work:

1. Compute the union world-space draw bounds of active rig parts on the presentation owner.
2. Publish one `PortalCompositingCandidate` for the owner.
3. Keep child part sprites out of portal candidate authority.
4. Verify the same rig presentation is visible to multiple local cameras without duplicating rig animation state or part populations.
5. Exercise TwinTrack/split-view as the multiview witness.

Acceptance:

- portal clipping/transit presentation is no worse than the current actor-level rectangular candidate behavior;
- no per-part portal lifecycle exists;
- two local views do not double rig ECS entities or animation state;
- view-local UI isolation remains unchanged.

### Packet 8 — benchmark the first world implementation and decide whether extraction optimization is needed

**Status (2026-09-30): measured on this machine; the decision needs a hardware GPU.** The bench is `crates/ambition_render/examples/rigged_sprite_bench.rs`. It uses the real `bind_rigged_presentations` and `drive_rigged_presentations` systems, the published admiral sheet and its flipbook (`max_draws` = 12), and a stand-in animator that changes pose every two seconds. The predictions were written before each run; the record is below the tables.

ECS only (no renderer, `--profile profiling`, 3000 frames, the median of the update time):

| Actors | Baked µs | Rigged µs | Added µs | Rig entities | Visible sprites (baked / rigged) |
|---:|---:|---:|---:|---:|---:|
| 1 | 3.3 | 4.5 | 1.2 | 14 | 1 / 13 |
| 10 | 4.9 | 15.0 | 10.1 | 140 | 10 / 130 |
| 50 | 13.6 | 68.7 | 55.1 | 700 | 50 / 650 |
| 100 | 25.6 | 133.4 | 107.8 | 1400 | 100 / 1300 |

The added CPU is about 1.1 µs for each actor and frame, and it is linear. "Rig entities" is exactly 14 for each actor (the root, the owner and 12 slots).

With Bevy's renderer (`--render`, a 1280 × 720 offscreen target, llvmpipe, 300 frames, the median frame time):

| Actors | Views | Frame ms (baked / rigged) | Extracted sprites | Sprite batches |
|---:|---:|---:|---:|---:|
| 1 | 1 | 8.5 / 9.2 | 1 / 13 | 1 / 2 |
| 10 | 1 | 9.3 / 11.2 | 10 / 130 | 1 / 20 |
| 50 | 1 | 9.5 / 13.8 | 50 / 650 | 1 / 100 |
| 100 | 1 | 11.1 / 33.6 | 100 / 1300 | 1 / 200 |
| 10 | 2 | 18.0 / 22.8 | 10 / 130 | 2 / 40 |
| 100 | 2 | 20.4 / 71.0 | 100 / 1300 | 2 / 400 |

- A second view extracts the same sprites and doubles the batches. That is one batch list per view, as for the baked path.
- Each rigged actor makes two batches. The root still draws its baked quad with zero alpha, from the sheet page, between the parts of two bodies. So the root splits the batch of the part page at each body.
- `--tiny` (a target ten times smaller that shows the same actors, so almost no pixels are filled) keeps most of the difference: 6.3 / 24.8 ms at 100 actors. So the difference is not the fill rate.
- 1300 baked actors (1300 sprites in 1 batch) take 19.1 ms with `--tiny`. Thus, of the 18.5 ms that 100 rigged actors add, about 12.8 ms comes with the sprite count and about 5.7 ms with the batches.
- On llvmpipe the vertex work for each sprite is CPU work. A hardware GPU does not do that work on the CPU. So these frame times show where the cost is, but they do not show the cost on a player's machine.

Texture bytes (RGBA8, full tier): the admiral sheet is 361,674 texels (1.45 MB); its part page is 224,064 texels (0.90 MB, 0.62 of the sheet). In the trial both are resident, so the rigged path costs 1.62 × the baked texture bytes. The baked frame is also what a portal draws (Packet 7). So the saving needs the sheet page to go, and the portal to draw parts first.

Not measured: the load and materialization time, the frame time on a hardware GPU, and the CPU time of each render system set.

**Decision:** do not build a custom part-instance renderer now. The ECS cost is small (0.11 ms for 100 actors). The render cost on this machine comes from the sprite count and the batch count, and the per-sprite cost here is the software rasterizer's. Before a custom renderer, do these in this order:

1. Run `rigged_sprite_bench --render` (with `--views 2`) on a machine with a hardware GPU.
2. If the batches matter there, stop drawing the zero-alpha root. Then the parts of all actors of one target share one page and can share batches. The root must stay the portal candidate, so this needs another way to state its size and frame to the portal.
3. Only if the sprite count itself is too expensive on hardware, replace the slot realization with an instance buffer, as this packet's text says.

Pre-registration record (written before each run):

- Hit: the rig entities for each actor (2 + `max_draws`, exact), the visible sprites (1 + the frame's draws), the added CPU at 100 actors (band 0.05 to 1 ms: 0.108), the scaling from 10 to 100 (band 5 to 15: 10.7), the part page at 0.62 of the sheet, the extracted sprites (N and 13N, exact), the batches (1 and 2N, exact), and a second view (the same sprites, the batches doubled, exact).
- Missed: the rigged/baked frame-time ratio at 100 actors (band 1 to 3): 3.0 with one view, 3.5 with two.
- Falsified: "the difference is mostly fill" (the `--tiny` run).
- Between the bands: the 1300-sprite run (19.1 ms, between "the batches" below 10 ms and "the sprite count" at 20 ms or more), so the record gives the two parts in ms and no single cause.

**Run it on a hardware GPU (2026-10-01).** On a machine with a hardware GPU, from the repository root:

```sh
python3 scripts/rig_packet8_gpu_bench.py
```

The script needs only `cargo` and the Python standard library. It builds the bench with the `profiling` profile and runs the matrix: the ECS mode, the renderer with one view, with two views, and with `--tiny`, each for 1, 10, 50 and 100 actors. Each measured render frame waits for the GPU (`RenderDevice::poll`), so the frame time includes the GPU work and not only its submission. The bench prints the adapter, and the script stops when the adapter is a software rasterizer (llvmpipe, lavapipe, SwiftShader, WARP, or a `Cpu` device type), because that is the measurement above. It writes `target/rig_packet8/<host>_<UTC time>/report.md` (the tables), `report.json`, and the log of each run. Send `report.md` back, and the decision above continues from step 2 of its list.

This is implementation validation, not architecture discovery.

Benchmark the completed fixed-slot path with:

```text
1 actor
10 actors
50 actors
100 actors
```

and repeat a representative scene under split view.

Record:

- texture bytes resident for baked vs rigged prototype;
- loaded atlas/page count;
- sprite/entity count;
- extracted sprite count;
- CPU animation/update time;
- render extraction time if available;
- actual draw-call/batch count if available;
- frame time;
- load/materialization time.

The expected Pirate texture result is the measured one: about 38% fewer raw texture pixels (Packet 5). The 75–76% estimate before it is superseded.

Do **not** implement a custom part-instance renderer unless the fixed-slot world path demonstrates a material CPU/entity/extraction problem. If it does, preserve the published asset and semantic animation contracts and replace only the presentation realization.

### Packet 9 — hybrid clips

**Status (2026-09-30): done, behind the trial switch.** The runtime half and the Mary-O publish are both done. Not measured: Mary-O drawn in the game from parts while she walks. Her demo's tests are headless, so the runtime crossing is shown by the raider witness below.

- A flipbook states each row of its sheet as a part clip (`clips`) or a baked clip (`baked_clips`, optional in the RON, absent for the pirates). `RiggedSpriteAsset::realization(row)` gives the choice. It is published with the clip, so no runtime rule picks it (work items 1 and 4).
- `check_rows` refuses a flipbook that states a sheet row as neither, a row as both, or a clip for a row that the sheet does not have. The attach road calls it, so a body never meets a row that has no realization. All five pirates state every row as a part clip.
- The driver draws a baked clip from the root: the root takes its tint back and the slots hide. The root, its animator and its feet are the same for both kinds of clip, so the crossing has no jump in place or in timing. Witness: `a_hybrid_body_crosses_between_part_and_baked_clips_in_place` (the raider with `slash` left baked). It fails when the driver does not give the root its tint back.
- Measured in the game, 2026-10-01: `capture_mary_o --walk 40` (960 × 540, `--no-ui`, llvmpipe) with `AMBITION_RIGGED_SPRITES=1` binds six part sprites at Mary-O's body (x = -1429, the drawn root's x; a 24 × 23 body part and five smaller parts). With the switch off, the same capture has none of them. Every other drawable is in both lists. She is drawn in both frames. ⚠ This shows that her parts are bound and placed in the game. It does not show that each part is visible on that frame. ⚠ The pre-registered pixel comparison could not be read: two baked runs differ (9089 px), because the capture walks on the wall clock, so her position and the enemies' animation are not the same between runs.
- Work item 3, the Mary-O publish. Her `RigDocument` paints every frame from rigid sprite parts. So her flipbook is RECORDED from the real render, with nothing reconstructed: `part_flipbook.recorded_blits` records each `rigdoc.blit_rotated` call, and `build_rig_flipbook` makes the draws from those calls. `mary_o_v2.PART_ROWS = ("walk",)`: each form (short, tall, fire) draws its walk from parts and states every other row as baked, the transition clips and their effects among them. Every walk frame of all three forms recomposes within the pirates' 2.5% bound (`tests/test_mary_o_part_flipbook.py`). The short form's worst frame is 1.74%: a half-pixel placement that the render rounds. Her walk turns no part, it only moves them. The Rust side reads all three forms and their tiers as hybrids that state every sheet row (`mary_os_flipbooks_draw_her_walk_from_parts_and_leave_the_rest_baked`).

**First hybrid control:** Mary-O.

Only begin after Pirate proves runtime part rendering.

Work:

1. Allow prepared visual clips to choose baked or part realization explicitly.
2. Keep transformation/effect-heavy Mary-O clips baked.
3. Migrate one ordinary locomotion clip only if its publisher can produce verified rigid-part output cleanly.
4. Keep one semantic animation request above both realizations.

Acceptance:

- one character can cross between baked and part clips with the same feet/render basis and semantic timing;
- no duplicated facing/clip policy appears;
- baked special effects remain first-class rather than being approximated to satisfy the rig.

### Packet 10 — later physicalized pose work, only with a real mechanic

Do not include gameplay-authoritative ragdoll in the initial implementation tranche.

If a real mechanic later needs it, start a separate focused packet using the existing `BodyRigDefinition` topology.

Possible levels:

1. **Cosmetic ragdoll:** presentation-only after an authoritative KO/death fact.
2. **Deterministic constrained ragdoll:** canonical rollback state with fixed ordering/iterations and explicit positions/velocities.
3. **Fully physical locomotion:** explicit non-goal.

Animation and physics must never both own the same joint transforms in one tick.

## Collision and attachment semantics in detail

### Broad phase

Keep current actor envelope / `CenteredAabb` coarse tests.

Do not make every sword swing test every body part of every actor before broad-phase rejection.

### Narrow phase

For a rigged body:

```text
prepared hurt part
+ resolved joint transform
        ↓
world-space CombatVolume
```

Publish those exact volumes through the existing damageable-volume road.

Do not create separate ECS entities for head/torso/limbs merely to collide them.

### Optional hit-part identity

Where the combat query can preserve the originating semantic part cheaply, expose it as optional contact metadata. Do not require downstream damage code to branch on it unless a mechanic explicitly asks for it.

### Attachments

Attachments are semantic named points or local transforms on the rig:

```text
HandNear
HandFar
Head
FootNear
FootFar
Muzzle / weapon-specific socket later if authored
```

Use attachments for consumers that already ask semantic questions such as `Muzzle::Hand`.

Do not add render-specific names such as `sprite_arm_slot_3` to gameplay APIs.

## Render and asset behavior

### Fixed slots are the first implementation, not independent actors

The first world player intentionally copies the proven vanity-card allocation strategy. Each child part is presentation-only and lifetime-owned by the rigged presentation owner.

The root/owner keeps semantic animation state. Child sprites contain only disposable current draw state.

A later custom extraction path can eliminate the children without changing semantic architecture.

### Batching expectation

Keep all parts for one character on as few atlas pages/materials as practical. The pirate first asset should normally be one page at current sizes.

Measure actual Bevy batching before optimizing it.

### Draw order

Publish order/z explicitly. Do not derive anatomy order from part names in Rust.

### Facing and asymmetric art

Use the same semantic facing authority as baked characters.

A whole-rig mirror is acceptable only when the published character supports it. If asymmetric art requires an authored alternate, publish that alternate rather than introducing a runtime guess.

### Quality tiers

The publisher should generate rig atlas quality variants through the same tier policy as current character assets.

Semantic `BodyRigDefinition` is quality-independent. Full/Half/Potato rendering must not move gameplay joints or collision.

## Rollback and determinism contract

### Canonical state

Existing future-affecting facts remain canonical, including:

- body transform / motion;
- facing;
- `BodyPoseClock`;
- `MovePlayback`;
- any later physicalized pose state if introduced.

### Derived state

`BodyRigPose` is derived. It must be rebuilt in deterministic simulation order before:

- rig hurtbox publication;
- `Muzzle::Hand` or other simulation attachments;
- any future gameplay consumer.

The rollback census/registry should explicitly classify it as derived so a future maintainer does not accidentally serialize a second authority or forget its rebuild requirement.

### Presentation state

`RiggedSpriteAsset`, visual frame selection caches, child sprite transforms, portal draw bounds, and part slot visibility are presentation state. They do not participate in gameplay rollback.

## Validation matrix

### Publisher

- Pirate component-scene export has stable part IDs/order.
- All 38 Raider frames recompose against the canonical direct renderer.
- Dynamic overlays are tight-cropped and appear once per frame.
- Atlas pack is deterministic for identical inputs.
- Published render basis matches current baked sheet.
- Mary-O semantic rig publish is independent of raster quality tier.

### Preparation / construction

- Invalid joint parent is rejected.
- Parent cycle is rejected.
- Unknown attachment joint is rejected.
- Unknown hurt-part joint is rejected.
- Non-finite transform/shape is rejected.
- Rig fact exists on the body at construction publication time.
- Re-wear removes character-owned outgoing rig facts correctly.

### Deterministic pose

- Same authoritative inputs produce identical `BodyRigPose`.
- Facing gives deterministic mirrored/resolved semantic anchors.
- Rollback restore rebuilds pose before consumers.
- Headless app resolves pose with no image/material resources.

### Collision

- Mary-O idle/walk/jump/crouch produce expected simple part volumes.
- Broad-phase miss avoids detailed checks.
- Move-specific hurtbox override wins where authored.
- Rig default and legacy default are not simultaneously authoritative.
- Damage remains actor-owned even when optional part metadata is available.

### Attachment

- Pirate Admiral hand muzzle uses the rig hand when present.
- Non-rigged hand muzzle uses the compatibility heuristic.
- Projectile origin does not read child sprite transforms.

### Visual parity

- Raider 38-frame offline parity passes.
- Runtime clip order/timing matches baked path.
- Facing/mirror semantics match baked path.
- Render basis/feet anchor is stable.
- z-order matches authored component scene.

### Assets

- lazy demand works;
- readiness means atlas + manifest are usable;
- tier change uses the existing character quality authority;
- retirement drops strong handles through the existing road;
- semantic body rig survives graphics-quality changes unchanged.

### Portal / multiview

- one union portal candidate per rigged actor;
- no candidate per child part;
- split view does not duplicate rig animation or part populations;
- view-isolation tags remain limited to genuinely view-local presentation.

### Performance

Report baked and rigged results for 1/10/50/100 visible Pirates and a split-view case.

Do not call texture savings a runtime win without reporting CPU/presentation cost too.

The measured economic baseline is:

```text
Pirate texture-pixel saving: about 38% (Packet 5; the 75.6–76.3% estimate is superseded)
Pirate visible quads: 12 or 13 per actor (Packet 8: max_draws 12, plus the zero-alpha root)
```

## Risks and resolved mitigations

### More quads and ECS presentation objects

The baked road is one quad; the conservative pirate path is about 9–10 quads.

Mitigation: fixed reusable child slots first, shared atlas/material, measure at 100 actors, and optimize extraction only if evidence requires it.

### Alpha overdraw

Separate parts can overlap. Texture-memory savings do not guarantee lower GPU bandwidth.

Mitigation: tight crops and real GPU/frame evidence before broad migration.

### Seams and pivots

Bad pivots can create visible gaps that do not exist in baked frames.

Mitigation: publisher parity diff and use existing authored pivots.

### Dynamic/deforming geometry

Not all current pirate geometry is rigid.

Mitigation: one dynamic overlay per frame in the first format. Do not force deformation into a rigid model.

### Mary-O special effects

Mary-O has transition and palette/effect clips that are poor first rigid-runtime candidates.

Mitigation: semantic body rig first, baked rendering unchanged, hybrid visual migration later.

### Two collision authorities

A new rig default could accidentally coexist with generated pose boxes.

Mitigation: explicit source precedence and remove same-priority default duplication when a character is migrated.

### Ragdoll determinism

A gameplay ragdoll adds constrained bodies, solver ordering, contacts, velocities, and rollback state.

Mitigation: no gameplay ragdoll in the initial tranche. Add it only for a real mechanic under its own deterministic design.

### Authoring/runtime coupling

It would be easy to serialize Python/SVG implementation details and make Rust depend on the authoring tool's internal model.

Mitigation: publish stable semantic topology/pose and visual flipbook products only.

## Explicit non-goals for the initial tranche

Do not:

- replace all baked character sheets;
- require every character to have a body rig;
- make rendered pixels or sprite transforms gameplay authority;
- replace movement/stance collision with articulated rigid bodies;
- add runtime Python, SVG, IK, or vector-deformation evaluation;
- implement general skeletal interpolation before a customer needs it;
- implement fully physical character locomotion;
- implement gameplay-authoritative ragdoll;
- give every limb independent health;
- create one gameplay ECS entity per limb;
- publish one portal relationship per limb;
- duplicate rig state per local view;
- add a second texture-demand or quality-selection system;
- require Mary-O visual migration before the Pirate visual prototype succeeds;
- convert every dynamic pirate limb into a reusable rigid part before shipping the first prototype.

## Concrete implementation handoff

The implementation agent should start at Packet 1, not with repository discovery.

Use this file/symbol map as the starting orientation:

| Concern | Existing owner / starting point |
|---|---|
| character authored semantics | `crates/ambition_characters/src/actor/definition.rs` |
| prepared character facts | `crates/ambition_characters/src/prepared.rs` |
| body construction/grant/retraction | `crates/ambition_platformer2d_actor_spawn/src/character_body.rs` |
| deterministic hurtbox resolution | `crates/ambition_combat/src/hurtbox_resolution.rs` |
| pose clock / runtime schedule | `crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs` |
| rollback classification | `crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs` and existing rollback census/registry |
| semantic muzzle vocabulary | `crates/ambition_characters/src/brain/action_set/mod.rs` |
| projectile muzzle resolution | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs` |
| existing hand heuristic | `crates/ambition_mount/src/lib.rs` |
| baked character asset | `crates/ambition_sprite_sheet/src/character/mod.rs` |
| baked animator | `crates/ambition_sprite_sheet/src/character/animator.rs` |
| character binding | `crates/ambition_render/src/rendering/actors/mod.rs` |
| frame application | `crates/ambition_render/src/rendering/actors/animation.rs` |
| body-owned drawable ordering | `crates/ambition_render/src/rendering/mod.rs` (`BodyOwnedDrawableSync`) |
| character asset demand | `crates/ambition_platformer2d_actor_monolith/src/character_sprites/assets.rs` |
| image loading | `crates/ambition_sprite_sheet/src/character/assets.rs`, `game_assets/mod.rs` |
| portal drawable publication | `crates/ambition_render/src/rendering/portal_compositing.rs` |
| multiview isolation | `crates/ambition_render/src/rendering/view_isolation.rs` |
| semantic presentation read model | `crates/ambition_sim_view/src/anim_index.rs`, `pose_view.rs` |
| pirate component source | `tools/ambition_sprite2d_renderer/.../targets/characters/_pirate_common.py` |
| pirate rig/source | `tools/ambition_sprite2d_renderer/.../targets/characters/_pirate_rig.py` |
| pirate fidelity witness | `tools/ambition_sprite2d_renderer/tests/test_pirate_svg_fidelity.py` |
| Mary-O production rig | `tools/ambition_sprite2d_renderer/.../targets/characters/mary_o_v2.py`, `_mary_o_v2_svg_poc.py`, `assets/mary_o_v2.svg` |
| proven part-atlas publisher | `tools/ambition_sprite2d_renderer/scripts/export_director_vanity_card.py` |
| proven Rust part player | `game/ambition_content/src/presentation/vanity_card_made_this_meme.rs` |
| Admiral integration witness | `game/ambition_app/tests/admiral_gun_sword.rs` |

Expected first implementation sequence:

```text
1. Publish Mary-O semantic body rig.
2. Prepare/grant it atomically.
3. Resolve deterministic BodyRigPose before Materialize.
4. Move Mary-O default damage geometry onto the rig.
5. Publish Pirate Admiral hand attachment and migrate Muzzle::Hand to it.
6. Publish Pirate transform-flipbook visual asset with offline parity.
7. Add demanded runtime RiggedSpriteAsset and fixed-slot world player.
8. Integrate owner-level portal bounds and shared multiview presentation.
9. Benchmark 1/10/50/100 actors and split view.
10. Only then consider hybrid Mary-O visuals or a custom extraction optimization.
```

That sequence gives useful semantic rig architecture before the renderer bet, and it gives the renderer a real memory-saving candidate (measured at about 38%, not the 75% first estimated) rather than a synthetic demo.

## Definition of success for this disjoint plan

The initial implementation tranche is successful when all of the following are true:

1. Mary-O has a prepared semantic body rig derived from her existing production SVG rig.
2. `BodyRigPose` is deterministic derived simulation state resolved before collision and attachment consumers.
3. Mary-O can use rig-derived default hurt geometry while continuing to render her existing baked sheet.
4. Pirate Admiral's `Muzzle::Hand` uses a semantic hand attachment when rig data is present.
5. Pirate Raider can render from a published part atlas + transform flipbook with offline and runtime parity against the baked source.
6. The Pirate visual prototype keeps the measured reduction in raw texture pixels: about 38%, and at least 35% (`tests/test_pirate_part_flipbook.py`). (The 75%+ first written here was an estimate, superseded by the publisher's measurement.)
7. Rigged presentation participates in existing demand, quality, retirement, portal, and multiview ownership rather than adding parallel subsystems.
8. Headless simulation never depends on images or rendering.
9. Baked sprites remain first-class and hybrid clips remain possible.
10. No ragdoll or custom extraction subsystem is added without a demonstrated customer or measured need.

At that point the engine has gained a reusable body-rig capability with independent value for collision and attachments, plus a proven optional part-rendering road with known memory economics. Broader migration can then be triaged using measured runtime cost rather than speculation.
