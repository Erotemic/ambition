# Articulated body rigs and runtime part animation

**State:** BUILT baseline, with PLANNED execution specialization below.
Rigged sprites (the visual half) are on in the shipped game. Body rigs (the
gameplay half: hurt parts and attachments) are built and off by default.
The planned visual work compiles stable clip/track relations, retains prepared
bindings, shares samples at the same presentation cut, and separates source
raster from finishing invalidation. Hardware measurements and gameplay rig
rollout retain their separate gates.
The 2026-11-22 static follow-up also binds native animation roles to named
source rows before visual specialization. That relation includes any
mechanical sample; loaded pixels never select gameplay geometry.

Related owners:

- [`render-animation-and-vfx.md`](render-animation-and-vfx.md): the
  simulation-to-presentation boundary and drawable scheduling.
- [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md):
  demand, preparation, device materialization, quality and residency.
- [`performance-and-iteration.md`](performance-and-iteration.md): runtime and
  memory evidence.
- [`sprite-renderer.md`](sprite-renderer.md) and the sprite-renderer
  repository: authoring and publishing contracts.
- [`svg-component-character-migration.md`](svg-component-character-migration.md):
  component-oriented character authoring.
- [`multiplayer-and-multiview.md`](multiplayer-and-multiview.md): view-local
  presentation.

## Current shape

Three concepts, kept separate:

```text
BodyRigDefinition   # prepared semantic topology, attachments, hurt parts
BodyRigPose         # this tick's resolved joints and attachments (derived sim state)
RiggedSpriteAsset   # optional visual: part atlas + per-frame draw table
```

A character can have a body rig without a rigged sprite, and the reverse.

**Filtered-cut receipt (2026-10-28, renderer `3243b40`):** packed frames keep
edge RGB under alpha zero; the Behemoth's repeated tiles include source context.
The sampling, coverage and incremental-pack tests and real door/Hall captures
hold the fix. The robot and Behemoth are rebuilt at every quality tier.
Keep shared rasters. See [the filter contract](../../../dev/benchmark-candidates/shared-part-cuts-under-linear-filtering.md).

### Switches

| Switch | Default | Environment | Effect |
| --- | --- | --- | --- |
| `RiggedSpriteAdmission` (`ambition_sprite_sheet::character::rigged`) | **on** | `AMBITION_RIGGED_SPRITES=0` turns it off | A character that publishes a flipbook draws from parts; every other character draws its baked sheet |
| `BodyRigAdmission` (`ambition_characters::actor::body_rig`) | **off** | `AMBITION_BODY_RIGS=1` turns it on | Off, no definition carries a rig, so no body has hurt parts or rig attachments |

`BodyRigAdmission` is read where a cast is registered, because an admitted rig
changes the character's content fingerprint, which a rollback timeline compares.

Flipbook coverage is determined by the selected publication. Each admitted
flipbook declares its part and baked rows. The
[Mary-O realization plan](mary-o-part-realization.md) records the part-first
migration; the earlier five-pirate/three-form list is not a current corpus census.

### Body rig (gameplay)

- `BodyRigDefinition` is optional on `CharacterDefinition` and prepared
  character data. Preparation validates topology (unique joints, valid parents,
  no cycles, valid attachment and hurt-part joints, finite values).
  `grant_prepared_character_body` publishes the rig in the same construction
  batch; a re-wear retracts it (`GrantedBodyFacts`).
- `BodyRigPose` is rollback-derived (`rollback_registration.rs`). It is resolved
  from the rig, `BodyPoseClock`, `MovePlayback` and facing, in the
  `BodyRigPoseResolved` set, after `CombatSet::Playback` and before
  `CombatSet::Materialize`.
- `BodyPoseClock` carries a gait (`Standing`, `Walking`, `Running`, `Skidding`)
  and a gait clock, from simulation facts only (`grounded_gait`, one band for
  every body). A body gets the clock when it has a rig or authors hurtbox pose
  profiles.
- Hurtbox precedence (`resolve_hurtboxes_with_rig`): an authored move override
  outranks the rig, and the rig outranks authored pose profiles and the default.
- `Muzzle::Hand` uses the resolved rig hand when the body has a rig
  (`brain_effects.rs`). Bodies without a rig keep the `HAND_OFFSET_NORM`
  heuristic. `ahead` stays authored on the muzzle.

### Rigged sprite (visual)

- The published format is an offline-solved **transform flipbook**:
  `<target>_parts.png` and `<target>_parts.ron`. Python, SVG and vector
  deformation remain offline. Runtime `PosedParts::hold` already has a
  bounded two-bone solve for authored contact targets during pose blends;
  current `CharacterAnim::planted_contacts` selects ledge-grab hands.
  That is not a terrain-foot solver. Each quality tier has its own table
  (`<target>.<tier>`) with its own part rects and `texel_scale`; a part keeps
  its full-resolution size and pivot at every tier.
  `scripts/regen/generate_visual_quality_variants.py` publishes tier atlases by
  cropping and downsampling each part, never by resizing a packed page.
- A flipbook states each sheet row as a part clip (`clips`) or a baked clip
  (`baked_clips`). `check_rows` refuses a flipbook that states a row as neither,
  both, or a row the sheet does not have.
- Pages ride on the sheet realization (`CharacterSpriteAsset::rigged`), so the
  sheet's demand, quality and retirement roads own them.
- `CharacterAnimator` selects row, frame and facing. The rigged driver retains
  one presentation owner and reusable part slots per body. Ordinary part
  presentation draws those slots directly; the root carries no drawable image
  on that road. Required whole-body effects or image consumers request
  composition, with the existing hold and page-retirement policies.
- A parts-only realization does not request or retain baked pages.
  `character_sprites::assets::parts_only_realization` keeps frame-layout metadata
  with `NO_BAKED_IMAGE`; a hybrid requests only pages referenced by its baked
  rows. Baked art can remain an offline parity reference without becoming
  mandatory resident art.
- A body changes to its parts only when every part page is ready, in one frame.
  A tier change keeps the old parts until the new pages are ready. A re-wear
  drops the old character's parts at once.
- A baked clip of a hybrid draws through the root and hides its part slots,
  preserving the shared row/frame clock and placement.
- Portals operate on the body's current drawable family through the carry
  capture/proxy road; far-side windows render scene sources. They do not
  require every rigged body to retain a baked frame. Completed-image reuse is
  proposed work, with complete drawable coverage retained. The current and
  planned contracts belong to
  [portal presentation](portals-one-transit-for-every-mover.md) and
  [material presentation](portals-a-body-in-two-places.md).
- Multiview: slots take their root's render layers. A second view adds no
  presentation entities.
- `StanceSquash` reaches the parts through the owner's transform.

### Tentative terrain foot placement

[CV1](curved-surfaces-and-local-gravity.md#cv1--ground-the-picture-from-accepted-support)
first restores the existing slope goal through one shared support-derived
visual root placement. Direct parts, composition and baked fallback must
agree. Physical body placement and righting remain native movement facts.

If separate boot contact is later selected, reuse the prepared part/limb
facilities. The robot's current foot attachments are ankles, so name the
sole contact offsets and stance phase before solving feet to terrain.
Require reach/pelvis limits, support-relative planting on moving surfaces,
release and unsupported-row fallback. Cosmetic feet do not require
gameplay `BodyRigAdmission`. A general runtime IK framework or a rig for
every character remains outside this optional customer.

### Historical economics

The following figures record earlier flipbook experiments, not current
residency or frame cost: approximately 38% fewer raw pirate part pixels;
1.62x baked texture bytes while both realizations remained resident;
approximately 1.1 us ECS work per actor/frame; and approximately 3x baked
frame time for 100 rigged actors on llvmpipe. Their publication, residency
and rendering conditions predate the current part-first and demand-driven
composition paths. The former zero-alpha baked root is not a current
explanation for batching cost.

Retain these results as historical receipts. Current comparisons must identify
the selected publication, resident realizations, direct/composed demand,
hardware and quality settings. The benchmark entry points below remain
measurement tools, not evidence that those historical ratios still hold.
[Performance and iteration](performance-and-iteration.md) owns current cost
attribution; [semantic-part rendering](semantic-part-rendering-and-ragdolls.md)
owns direct-versus-composed rendering decisions.

Bench: `crates/ambition_render/examples/rigged_sprite_bench.rs` (`--render`,
`--views 2`, `--tiny`). Matrix on real hardware:
`python3 scripts/measure/rig_packet8_gpu_bench.py` (refuses a software adapter, writes
`target/rig_packet8/<host>_<UTC>/report.md`).

## Architectural rules

- **One semantic body authority.** A rigged body's pose is a simulation fact.
  The renderer consumes it or data derived from the same semantic animation
  state. Render transforms are never read back into gameplay.
- **The rig does not replace movement physics.** It drives hurt geometry,
  attachments and optional hit-part metadata. It does not replace
  `BodyBaseSize`, stance collision, whole-body movement or the `CenteredAabb`
  broad phase.
- **Keep the broad phase coarse.** Coarse actor overlap first, then exact rig
  hurt parts. Use simple shapes (circles, capsules, boxes). Publish exact volumes
  through the existing damageable-volume road. Do not create ECS entities per
  limb.
- **One precedence, no reconciliation.** Move override > rig default > legacy
  default. Do not run two same-priority default hurtbox authorities.
- **Part IDs are optional metadata.** Damage stays actor-owned. No per-limb
  health.
- **Attachments are semantic** (`HandNear`, `HandFar`, `Head`, `FootNear`,
  `FootFar`). No render-specific names in gameplay APIs.
- **One facing authority.** The rig resolves near/far from body facing.
  Presentation mirroring consumes the same result. A whole-rig mirror is valid
  only when the published character supports it.
- **Quality-independent gameplay.** Full/Half/Potato rendering never moves a
  joint, an attachment or a hurt part.
- **One semantic animation vocabulary.** The realization boundary chooses baked
  or part per prepared clip. Do not add `RigAnim::*`. Do not pick a realization
  by runtime heuristic.
- **Explicit draw order.** Publish z-order. Do not derive anatomy order from
  part names in Rust.
- **Prepared-generation content.** The active prepared generation chooses the
  immutable rig data for new and reconstituted bodies. Do not mutate live
  topology during asset reload.
- **Ragdoll changes pose authority explicitly.** Animation and physics never
  both write the same joint transforms in one tick. Do not add a
  `BodyPoseAuthority` enum until a second pose provider exists.

## Semantic landmarks (Q41)

Ruling Q41 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
projectile launch points and every similar spatial interaction use authored
semantic landmarks on the character/rig, never sprite bounds or arbitrary body
offsets. A move may add a move-specific offset from a named landmark.
Presentation geometry is not authoritative for simulation. This is an
important rig capability, not only a texture or animation saving: hands touch
things, weapons attach consistently, shots leave believable places, riders
mount at authored anchors, and petting and contact align.

**Eye point (2026-11-16).** `Landmark::Eye` is an authored emission point.
Schema 4 part tables can publish `landmarks` as a map from clip name to one
map of named points per frame. Points are full-resolution pixels from the
feet; quality conversion keeps them unchanged. The robot publishes its eye
from the visor and head pose. `BodyLandmarks` reads the point on the selected
row's clock. Held rays use this point for their origin and the visor box for
width. A missing point does not use a guessed body offset.

Capture the standing and crouched firing poses through the production renderer:

```bash
cargo build -p ambition_app_tools --bin capture_scene
./target/debug/capture_scene central_hub_complex player target/eye-review.png 960x540 \
  --warmup 60 --frames 2 --stride 30 --gear eye_laser@30 \
  --player-holds projectile@40 --player-holds down+projectile@70 \
  --tick-per-frame --camera-zoom tight --art-snapshot
```

`--gear` uses the catalog row's grants, not only its display id. A content pin
clocks its selected row in either stance. Its fallback keeps the base clock.

**Vocabulary (target).** Hands (near/far), muzzle/projectile origin, feet,
head, held-item sockets, weapon grips, rider/mount anchors, petting/contact
points. The rig attachments (`HandNear`, `HandFar`, `Head`, `FootNear`,
`FootFar`) are the first members (`Landmark`,
`crates/ambition_characters/src/actor/landmarks.rs`). A landmark is a package slot (2026-08-17
ruling: optional, authored when useful), resolved per pose. A body with a rig
answers from `BodyRigPose`; a body without a rig answers from its package's
authored per-pose points. Each consumer asks one query, "where is landmark L of
body B this tick", and does not know which of the two answered. A missing
landmark is a named fallback the consumer states, not a silent offset.

**Where spatial interactions come from today (2026-10-04).**

| Interaction | Source today | Landmark it wants |
| --- | --- | --- |
| Action shot (`Discharge::muzzle`) | ✅ 2026-10-05: `Muzzle::Hand { ahead }` fires `ahead` past the hand the landmark query answers on that tick (`ambition_held_items::holding_hand_world`). `BodyOrigin` and `Offset { x, y }` are authored and read no hand (`action_set/mod.rs`) | done for `Hand` |
| Held item place, and the prop drawn over the hand | ✅ 2026-10-05: the same `holding_hand_world` (`ItemWorldPos`; `rebuild_hostile_wielded_items_view`), so a bomb bursts, a prop is drawn and a shot leaves at one hand | done |
| A holder with no answer | `HAND_OFFSET_NORM` × body height (`ambition_mount/src/lib.rs`, `rider_hand_world_pos_in_frame`): the named fallback of `holding_hand_world`, for a body that publishes no hand or states no art scale | hand |
| Player ranged shot | The shot starts at the first-frame hand of the `shoot` row. Guard: `a_shot_is_born_with_its_rear_edge_at_the_hand_that_fires_it` | done; keep the authored hand (Q158) |
| Pet ("pet the dog") | ✅ 2026-10-05: the landmark query. The petter stands where its near hand, in the `pet` row, is on the petted body's head, in the `petted` row, plus the offset its catalog row authors (`petting.contact_offset`). The box mark is the named fallback for a pair that publishes no such landmark | done |
| Rig attachments | `BodyRigPose` attachments, live only under `BodyRigAdmission` (off) | all of the above |
| Part flipbook tracks | ✅ 2026-10-05: the package's per-pose points. `build.rs` projects each `<target>_parts.ron` to its hand, head and foot tracks, in each of the three ways the rig families name them (`ambition_sprite_sheet::baked_landmarks`, `LANDMARK_TRACKS`, 124 tables, 1.1 MB) | the answer for a body without a rig |
| `_actor.ron` sockets | Not read, and not to be: overlaid on the robot's art (2026-10-05), `hand_r` and `muzzle` land on its face. They are profile proportions, not the art | none |

The "pet the dog" misalignment was the example case: the gesture positioned
bodies from boxes, so the robot's hand was 15 world units in front of the
dog's nose and 43 from the point its head turns about.

**The query (built 2026-10-05).** `BodyLandmarks`
(`crates/ambition_combat/src/body_landmarks.rs`) answers "where is landmark L
of body B", in the body's rig space or in the world, for this tick's pose or
for a named row at a phase (a gesture a script will play). The rig answers
first. Else the package table answers: body → `CharacterForm` → prepared sheet
→ table, at the scale the body states (`SpritePosedBody.world_per_pixel`, else
`ActorRenderSize` over the frame height). It holds no state: every input is
rollback state already, so there is no component and no schema change.

The tables are compiled in, so no world resource holds them. Their identity
reaches the content fingerprint as one BLAKE3 digest of the projected tables
(section `characters.baked-landmarks`,
`MechanicalRegistries::baked_landmarks`). The digest covers the points, the
frame height and the frame durations. It does not cover atlas packing.

**Named limits (2026-10-05).**

- The three track families are mapped by one table (`LANDMARK_TRACKS`): the
  `near_`, `front_` and `right_` hand is `HandNear`, and the `far_`, `back_`
  and `left_` hand is `HandFar`. The evidence is the draw order: in every
  frame that draws both hands, the first is drawn after the second (11195,
  1075 and 554 frames, mirror rows not counted, no exception; a test holds
  it). The renderer does not publish semantic names. For feet the draw order
  holds for `front_`/`back_` (1029 frames) and not for `near_`/`far_` (835
  of 10637 frames), so `FootNear` is a name and not a statement of depth.
- The hand a gesture uses is authored: the catalog row's `gesture_hand`
  (`Near` when it states none). No package states it, and no rule gives it.
  Three rules were measured on the published flipbooks and each failed: the
  near hand (the near hand of `player_robot_v2` trails 20 behind its body centre), the
  track family (the pirates name their hands `front_`/`back_` and are not
  drawn that way), and the idle stance (it names `player_robot_v3`, whose
  near hand is the right one, viewed). `robot`, `player_robot_v2` and the
  smash `smash_duelist_b` state `Far`. They were composed and viewed; no
  other row was.
- So the pet does not use a hand that is not forward of the petter's feet:
  the box mark answers for it. Of the 85 sheets that publish a hand, 49 give
  a pet reach from the hand and 36 keep the box mark. For 11 of the 36 (the
  `near_`/`far_` family) this corrects packet A, which put those petters on
  the petted body.
- A track point is the part's pivot: the wrist, and the point the head turns
  about (the dog's is at its ear). A contact point that is not a pivot is an
  authored offset from one (`petting.contact_offset`). The renderer publishes
  no contact track.
- The stated hand is of the whole art (`BodyLandmarks::gesture_hand`): a
  character cannot state one hand for its pet and another for its shot.
- The fireball's hand is of the middle of the `shoot` row for every aim. The
  robot has one shoot row, so a shot aimed up leaves the same hand.
- The shot is born on the tick of the press or the release. The `shoot` row
  starts on that tick, so the hand is where the row will put it, not where
  the body's row has it on that tick.
- A mirror row (`<row>~mirrored`) is not read: a body that faces the other
  way mirrors the row's points about its feet.
- A package clip loops or holds as its row does (`LandmarkClip::looping`,
  from the animator's rule for the row name, `row_loops`). No published file
  states which rows loop: the animator has a code table and each rig states
  its own bit, and they disagree on five rows (queue row LANDMARK-CLIP-TIME).
- A body that states no drawn scale (the legacy `collision_scale` render
  path) has no package answer.
- A posed body whose pose rectangle is not centred on its feet pixel is drawn
  up to about 1.5 world units from the answer (the robot's `pet` row). The
  renderer also adds a posed body's art offset without mirroring it, so the
  art of a body that faces left is off its box by twice that offset.
- A row that a hybrid character realizes from the baked sheet has no draws,
  so it has no points. For this tick's pose the answer then comes from the
  next row of the chain that has points, and at last from any row: a raider
  that fires answers from its `idle` row, because its package draws five
  rows (`death`, `hurt`, `idle`, `slash`, `taunt`).
- ✅ A match seat states its art scale (2026-10-05). The body seed resolves
  the quad with the collider (`ActorClusterSeed::render_size`), and each actor
  spawn road stated it as `ActorRenderSize`. The match-seat materializer did
  not, so a seat with no rig held its weapon at the fixed hand. It states it
  now, and the unrigged smash admiral fires from the hand its art draws: the
  shot is first seen (28.8 ahead, 15.0 below the centre), 9.0 above the feet;
  it was (37.0 ahead, 2.4 above), 26.4 above the feet. The rigged admiral's
  shot is first seen at (28.7, 15.0): the art and the rig agree to 0.04
  (`admiral_gun_sword`, both arms).
- ✅ A body that changes character states the quad of the character it wears
  (2026-10-06). The wear grant restated a quad only for a posed body, so a
  player that wore a character with a sheet and no posed body stated none.
  The grant now states the frame fitted to the box the body keeps
  (`sheets::fitted_render_size`, the renderer's `sprite_render_size`), when
  the body carries no quad, and its record says the quad is that wear's, so
  the retraction removes it. It is NOT the quad the seed resolves: a re-wear
  keeps the box, and a player that wears the kernel guide is drawn at 32.7 by
  33.5 while the guide is built at 50.4 by 51.6
  (measured for Q159, which was ruled 2026-10-19: a change of form takes the
  new form's authored geometry, so this fitting goes with queue row
  CHARACTER-FORM).
  A body its spawn built keeps the seed's quad. Witnesses:
  `a_reworn_body_states_the_quad_it_is_drawn_at` (the shipped game: stated
  equals drawn after each wear) and the grant's fixture arms.
- Two populations have no answer from their art. A sheet that publishes no
  idle body is drawn by the catalog's `collision_scale`, which the grant does
  not hold: 2 of the 149 characters of the main game (`npc_gnu_ton_boss`,
  `npc_weird_hermit`; 13 are posed and 134 are drawn from the box). And the
  development body-profile edit of `BodyBaseSize` leaves the stated quad as it
  was.
- A body with a sheet and no stated quad still gets its authored attack
  volumes scaled by a quad derived from its STANCE box
  (`moveset`: `kin.size` to `drawn_render_size`), while its art is drawn from
  its standing box. In the two compositions measured (the main game's first
  room, a smash match) each of the 5 bodies with a sheet states a quad, so no
  body there takes that road.
- The held-item place uses screen DOWN, as it did before: `ItemWorldPos`
  holds no body frame.

**Order of work (queue row RIG-LANDMARKS).** ✅ The query and the pet. ✅ The
player fireball: the move's offset from the hand is the shot's own half extent
along the aim, so the shot's rear edge is at the hand for each of the eight
aims and each charge size. ✅ The three track families and the authored
gesture hand. ✅ The hand muzzle, the held-item place and the prop over the
hand: one function, `holding_hand_world`. ✅ A match seat states its art
scale, so a seat answers as an adventure body does. Rigs are not admitted (`BodyRigAdmission`)
for this: the capability does not depend on rig rollout. Landmarks are
simulation facts: resolve them in simulation and never read them back from
render transforms.

## Rollback and determinism

- Canonical: body transform and motion, facing, `BodyPoseClock`,
  `MovePlayback`, and any later physicalized pose state.
- Derived: `BodyRigPose`, rebuilt before rig hurtbox publication and rig
  attachments.
- Presentation: `RiggedSpriteAsset`, frame selection, slot transforms, portal
  draw bounds and slot visibility. None participates in rollback.
- Headless simulation never depends on images or rendering.

## Planned — compile visual relations and select work by stage

**Status: PLANNED (2026-11-21 source audit, `aef73be1`).** This page owns
the animation/presentation specialization below. The shared contract is
[execution specialization and work selection](execution-specialization-and-work-selection.md).
The [semantic-part plan](semantic-part-rendering-and-ragdolls.md) continues
to own semantic part identity, effect coverage and the optional cell compositor.

### Current evidence and retained mechanisms

The renderer already retains part slots, avoids unchanged slot writes, rests
unchanged atlas pages after GPU readiness, and retires empty trailing pages.
Nov18 retained admitted part tables and removed realization-time decoding and
warm-up workers. Preserve these receipts; this is not another pooling campaign.

The remaining work is selected too late or by too broad a dependency:

| Current source | Work repeated despite stable inputs |
| --- | --- |
| `rigged.rs::body_warp` and `RiggedSpriteAsset::row_draws_the_body_whole` | Without the preview override, ordinary owners invoke a whole-clip predicate before determining whether they have a warp. Intact clips scan all their draws; missing or fading clips can exit early. Composed bodies invoke it again. Its inputs are immutable flipbook data. |
| `RiggedSpriteAsset::{legs_under,face_as,layer_over,head_turned_as}` | Samples reinterpret semantic track names and search other draw lists. Legs/expression joins can scan one frame for each draw of another. |
| `baked_sheet_rons::prepare_sheet_table`, `bind_posed_parts` | Admission prepares a rig and drops the value; rendering reparses/prepares it and measures the visual binding. The renderer's binding cache is cleared by any `GameAssets` change. Existing presentations retain their Arc, but a later new owner can redo an unchanged binding. |
| `frame_draws`, `posed_reach`, `part_set::answer` | Procedural poses can be sampled twice; readers rebuild name-based answers and local transforms before final equality prevents writes. |
| `drive_rigged_presentations` | Opacity/HSV/warp parameter changes set the same page dirty flag as source draws and run both source and finishing cameras. The terminal held-warp case already has a finishing-only exception. |

Owners are `ambition_sprite_sheet/src/character/rigged*`,
`ambition_sprite_sheet/src/baked_sheet_rons.rs`, and
`ambition_render/src/rendering/actors/rigged*`. These are source work claims,
not current frame-time measurements.

### PVS-1 — retain a compiled visual program

Extend the immutable flipbook preparation that already creates tween targets.
Retain exact per-clip whole-body/warp facts, per-track semantic role membership,
and frame track correspondence/anchor indexes. Keep current ordering, first-match
rules and missing-track fallbacks; do not assume unique draw occurrences without
establishing that invariant. Dynamic row selection, wounds, expressions and pose
remain runtime inputs. Do not enumerate their Cartesian product.
The [named source binding follow-up](#bind-semantic-roles-to-named-source-rows)
below replaces parallel implicit choices only where they claim to describe
one association. It does not change unrelated track ordering or fallback.

Retain the prepared rig from admission. Bind visual tracks to that exact rig
and flipbook mechanical program once per demanded compatible definition pair,
including named bindings, limbs and binding-algorithm identity. Use the existing
[preparation/readiness lifecycle](asset-preparation-and-residency.md); no new
process-wide name cache, live-World lookup, or metadata warm-up subsystem.
Unused definitions need not acquire visual bindings.

A quality tier changes page/rect realization while `for_tier` preserves part
identity, full-resolution geometry and draw programs. It can share the mechanical
binding. Replacing the rig, tracks or mechanical table invalidates that binding;
loading unrelated parallax or prop art does not. Old readers retain their admitted
generation. Unbound tracks retain authored-frame fallback.

**Acceptance:** whole-body decisions agree for opaque, self-fading, whole-frame
fading and translucent non-body effects; track operators preserve paint order,
mirrors, missing rows and hybrid baked clips. A pinned-sample comparison that
adds unsampled frames must preserve the compiled whole-body eligibility: those
frames are inputs to that predicate. Preparation computes these properties once
per admitted program, rather than once per body per presented frame. Report
preparation and retained-memory costs separately from runtime work removed.

**First slice (2026-11-22, clip and track facts).** `RiggedSpriteAsset`
preparation now keeps two immutable facts it used to re-derive per sample:

- Each `RigSpriteClip` carries `draws_body_whole`, computed over every frame
  against the body tracks of the first `idle` frame (the same rule as before).
  `row_draws_the_body_whole`, which `body_warp` calls for every owner every
  frame, is now one clip lookup instead of a scan of all the clip's draws
  with a linear body search per draw.
- A per-track `TrackRoles` bit set (lower body, hip, expression, layer
  anchor, `behind`, head, head pivot) is read once from the track names.
  `legs_under`, `face_as`, `layer_over`, `head_turned_as` and
  `has_expression_tracks` test a bit rather than splitting and matching the
  name of each draw.

`tracks` is private now. Readers use `tracks()`, and the one test that
renamed tracks uses `with_tracks`, which reads the roles again, so the role
table cannot drift from the names. `for_tier` clones the `Arc`s, so a tier
shares both facts. The retained cost is one byte per track and one bool per
clip. Wounds already had their prepared index (`wound_tracks`). No frame-time
measurement is claimed.

Still open in PVS-1: frame track correspondence indexes (the legs and face
joins still scan the other frame for each draw), and retaining the prepared
rig and visual binding from admission (`prepare_sheet_table`,
`bind_posed_parts`). The source binding below is a further preparation
customer; the completed clip/track facts stay complete.

#### Bind semantic roles to named source rows

**State:** PLANNED PVS-1/E2 refinement, static source `00bf92c22142`,
2026-11-22. First customer: the existing Flying Spaghetti Monster boss.
The source-to-native association belongs at content preparation, including
headless mechanical preparation. The renderer consumes its visual projection.
No renderer or gameplay run was made for this audit.

The [sprite source](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/flying_spaghetti_monster_boss.py)
names each row and maps source names to geometry keys. The separate
[`boss_sheets.ron`](../../../game/ambition_content/assets/data/boss_sheets.ron)
binds `BossAnim` by **position**, with copied frame counts and durations.
[`BossSheetSpec::record_row`](../../../crates/ambition_sprite_sheet/src/boss.rs)
uses that position as the sheet index.
`record_aligns_with_const` checks row capacity, frame counts and rectangle
counts; it does not check row identity. This predicate is a loader choice,
not complete content admission.

| Boss role | Named source row | Published geometry key |
| --- | --- | --- |
| `SideSweep` | `noodle_whip` | `side_sweep` |
| `FloorSlam` | `meatball_volley` | `floor_slam` |

Both source rows have seven frames. Swap their order while preserving their
names, art and key map: the capacity predicate still accepts the record,
but the base `SideSweep` draw selects `meatball_volley`, while its geometry
still names `side_sweep`. A named move pin can bypass that base draw path;
it does not repair the underlying binding. This is a source-derived
counterexample, not an edited or rendered asset.

**Prepared relation.** Keep each native role vocabulary, including
`BossAnim`. Bind a role to a named source row in the selected immutable
sheet and retain its declared geometry association. Preparation resolves
those references into dense handles scoped to their admitted products.
Keep canonical geometry keys such as `side_sweep`; do not replace them
with visual row names or pass one product's raw slot into another product.
If one row supplies both pixels and mechanical samples, the prepared value
must establish that correspondence. An intentional independent mechanical
program remains valid when its binding and sampling coordinate are explicit.

Use source frame facts by default. Preserve selected timing, frame-window,
loop and hold policy as explicit native overrides where they are deliberate;
do not silently replace all boss timing with the source duration. A changed
playback duration needs the source-time mapping already specified by
[CLIP3](../demos/smash-expressive-fighter-poses.md#clip3--preserve-source-motion-under-ruleset-retiming).
The immutable admitted mechanical binding remains stable for its session
generation. Visual quality tiers may resolve different pages and rectangles
without changing that binding.

Required named rows and geometry references fail at preparation if missing
or incompatible. Optional roles state their fallback. Preserve the explicit
unauthored/fixture sheet path. A malformed authored relation must not be
repaired by silently inventing a grid through `synth_record`.

**Reach the native consumers.** Changing `record_row` in the loader alone
would leave these decisions independent:

| Consumer | Required preservation |
| --- | --- |
| [`BossAnimFrame::tick`](../../../crates/ambition_sprite_sheet/src/boss.rs) | The simulation cursor reads the selected frame and timing policy. Windup and active phases remain distinct plays of the same role. |
| [`drive_boss_animators`](../../../crates/ambition_boss_encounter/src/ecs/tick.rs), [`ecs_boss_animation_frame_sample`](../../../crates/ambition_boss_encounter/src/anim.rs) | Resolve role and sample association through the retained session binding/catalog. Preserve the explicit rest sample and existing profile/sample precedence. |
| [`BossVolumeContext::hurtbox_selection`](../../../crates/ambition_boss_encounter/src/attack_geometry/mod.rs), [native target volumes](../../../crates/ambition_platformer2d_actor_monolith/src/body/target_volumes.rs) | The selected sample reaches actual damageable hurt parts. Preserve authored/rig `ResolvedHurtboxes` precedence over the boss frame-part fallback. |
| [Rig-driven hurt geometry](../../../crates/ambition_combat/src/body_rig.rs) | Pinned row, move binding and pose clocks remain distinct supported sources. Do not assume the boss enum controls this route. |
| [Boss construction](../../../crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs) | `DeathBeat` reads the selected death-row duration. Preserve lifecycle timing deliberately. |
| [Boss drawing](../../../crates/ambition_render/src/rendering/actors/boss.rs) | Base cells and parts consume the prepared named relation; companion products resolve their own named references. |

The [cursor snapshot contract](../../../crates/ambition_sprite_sheet/src/snapshot_impls.rs)
assumes a session-stable specification. Preserve that property when retaining
the prepared relation. The current driver reads live `Res<BossCatalog>`,
while the hurt-volume reader deliberately uses `SessionMechanics.bosses`.
Freezing `BossAnimFrame::spec` alone therefore does not establish agreement.
Role selection and emitted sample association must use that same retained
session product when the App publishes a later catalog, including during
resimulation. Reuse the existing generation owner. A newly resident
presentation tier cannot change the mechanical binding either.
Live boss attack damage is lowered through
[`boss_attack_moveset`](../../../crates/ambition_boss_encounter/src/attack_moveset.rs)
into ordinary move volumes. Agreement of sprite debug strike boxes alone
does not prove that live damage follows the intended motion.

**A second customer exposes the same missing relation.** The sprite
[`animation_key_map` builder](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/sheet_build.py)
renames geometry keys but does not retain the source-row association.
[`puppy_slug_velvet`](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/puppy_slug_velvet.py)
maps `slash` to `side_sweep`; the
[ordinary attack reader](../../../crates/ambition_character_sprites/src/attack_hitbox.rs)
looks up the requested animation name exactly. Its current catalog is
peaceful, so this is a reuse constraint, not a demonstrated missing bite.
Retain the source-to-geometry reference in the shared published record or
preserve geometry under its real source row. The generic artifact must not
import boss gameplay enums.

There is also no single alias law today:
[`CharacterAnim`](../../../crates/ambition_sprite_sheet/src/character/anim.rs)
maps both `jab` and `slash` to one role; the
[sheet reader](../../../crates/ambition_sprite_sheet/src/character/sheets/mod.rs)
selects the first row for that role, while
[`BodyMetrics::pose_body_bbox`](../../../crates/ambition_sprite_sheet/src/lib.rs)
selects the lexicographically first alias with a hurtbox. A sheet with
`slash` before `jab` and different boxes can therefore choose different
sources. Keep lean-sheet aliases, but declare the primary or ordered
selection where one association matters. Separate physical and visual
fallbacks may remain explicit. Do not impose one row vocabulary or one
hurtbox model on every authoring family.

**Removal, cost and acceptance.** Remove the boss positional join and
copied source layout facts that are not explicit overrides. Remove competing
implicit alias choices only for the binding they now share. Keep the
[boss/character 3b receipt](boss-character-unification.md) and the current
plural sprite authoring families. This is a native preparation change,
not a new DSL, string cache or runtime resolver.

The first implementation proof reorders the two equal-capacity boss rows
and preserves role selection, native cursor, actual hurt-part sampling,
and draw selection. A missing required name refuses preparation. An
explicit retiming case preserves its mapping and death duration; a visual
tier change or App catalog publication before session activation does not
alter the retained mechanics. One alias/key-map customer then
checks the shared record seam without requiring a cast-wide migration.
These are planned cases. Dense runtime lookup remains; names and static
joins resolve once. No frame-time or compile-time gain is claimed.

### PVS-2 — sample once for consumers at the same presentation cut

A simple stepped frame with no active modifiers may borrow its admitted draw
slice. The general path still performs tween, legs, head, expression, layer,
wounds, blend, follow-through and procedural pose in the current order. Keep
reusable scratch for dynamic results; do not quantize phases or suppress clocks.

Publish one local sampled result for slot drawing, actual sample bounds and
part-set readers when they request identical program, time/phase, dynamic inputs
and coordinate space. Bind each reader's track selection when its request or
program changes; retain independent reader leases. Separate world placement from
local draws, so root motion does not rebuild an unchanged local part selection.

The selected [2D/2.5D presentation work](presentation-order-and-25d-rendering.md)
adds a second consumer of this sample. Extract it before the current driver's
`Sprite`/`Anchor` and slot writes. Retain the baked alternative, full appearance
and group-operation meaning, provenance and unavailable state. Share one
interpretation and presentation clock/state for equal inputs; each concrete
renderer owns its GPU resources and completion. Do not use the current
`PartSetDraws` alone as the new contract: it contains placed 2D output, omits
full tint/opacity and does not represent every baked-row result. No sample
must be materialized when no view or other reader demands it.

Simulation hurt geometry/attachments and interpolated or finished presentation
can have different clocks and authorities. They may share immutable definitions
and pure sampling functions. They must not share a result merely because its row
and integer frame match, and gameplay must never depend on a renderer's output.
An arbitrary `PartPose` must match the compiled rig topology.

The sampled-output identity includes every input an active local operator reads,
including independent layer clocks and blend/follow-through state. Appearance
inputs belong to the stage that consumes them: direct slot tint needs full RGBA,
while a composed root quad's tint does not invalidate untinted local draws or
source pixels. Current direct equality retains only RGB although slot tint uses
alpha; do not carry that omission into the new contract. Room/layer, visibility,
demand and world transform retain their own updates when local geometry stays equal.

**Acceptance:** a held stepped frame does no new local interpretation while a
changing layer/pose still does; bounds and drawing use the same final sample;
rewear, rewind/cut, quality replacement, part↔baked rows and reader add/remove
select correct outputs. Keep the general operator sequence as the behavioral
reference for specialization equivalence within its contract; known cache-key
defects are not outputs to preserve.

### PVS-3 — separate source raster and finishing invalidation

First retain existing whole-page rendering and split its dirty states. Source
pixels depend on the sampled draws, source image/realization revisions, ownership,
cell placement and source compositing settings. Finished pixels depend on that
source revision plus opacity/HSV/warp and finishing settings. World placement
changes neither picture. A pure finishing change must not redraw source parts.

GPU completion belongs to the relevant revision: equal CPU inputs do not certify
a first, replaced, reused or evicted target. Finishing may reuse only completed
source pixels for the required revision. Preserve delayed-image/pipeline readiness
and exact color-space, alpha and draw-order behavior.

Then measure remaining page amplification under the semantic-part plan's existing
Phase 4. Pages may remain allocation units while cells become refresh units:
clear dirty cells completely, redraw their complete ordered source content, and
finish affected cells. Keep the exact whole-page fallback when many cells change
or the backend cannot efficiently preserve partial targets. Partial refresh
requires an explicit rendering mechanism; existing whole-page cameras do not
provide it for free. No camera per body.

Containment must cover the final sample and filtering fringe. Published tween
envelopes do not prove arbitrary pose/layer/blend transformations. Direct drawing
is a valid fallback only when no active consumer requires group composition.
When one does, provide an adequate group target or declare the output unavailable
under the existing containment owner's policy; do not silently drop its effect
or clip parts to make refresh cheap. A moved/disappeared part or reused cell
must leave no old pixels.

**Acceptance:** identical output for one changed cell beside held cells,
all-changing pages, opacity/HSV/warp-only changes, removal/reuse, source replacement,
page retirement and GPU readiness delay. Measure source draws, finishing pixels,
passes and retained bytes independently. Direct parts remain the default; true
group opacity and other whole-body consumers keep exact demanded composition.

## Open work

| Item | Work | Acceptance |
| --- | --- | --- |
| Hardware GPU run | Run `scripts/measure/rig_packet8_gpu_bench.py` on a machine with a hardware GPU | `report.md` returned; the decision below continues |
| Root batch split | Retained receipt: the direct part road gives the root `NO_BAKED_IMAGE`; portal presentation uses the current drawable family. A new hardware study must measure current batching | Preserve drawable ownership and optional exact group composition |
| Instance buffer | Only if sprite count is too expensive on hardware, replace the slot realization with an instance buffer. Keep `RiggedSpriteAsset`, `BodyRigDefinition` and semantic animation unchanged | Measured gain on hardware |
| Sheet residency | Retained receipt: parts-only realizations never request baked pages; hybrids request only their baked-row pages. Measure the selected current realization | Preserve omitted baked-page demand; report part, required baked and composed-target bytes separately |
| Body rig rollout | `BodyRigAdmission` is off. Turning it on changes shipped hurt geometry and the content fingerprint | Maintainer go-ahead; app suite green with it on |
| More rigid parts | Pirate dynamic limb/neck geometry is one overlay per frame. Convert more of it to reusable parts only if useful | Saving above the 38% floor |
| Semantic landmarks (Q41) | ✅ The query, the pet, the player fireball, the three track families with an authored gesture hand, and the hand muzzle with the held-item place, for a match seat too (2026-10-05). See "Semantic landmarks (Q41)" | ✅ A pet hand meets the authored contact point (`a_pet_hand_meets_the_contact_point.rs`: 1.4 world units; 15.2 on the box mark). Open: no consumer reads sprite bounds |
| Physicalized pose | Only with a real mechanic: cosmetic ragdoll after a KO fact, or deterministic constrained ragdoll as canonical rollback state | Separate focused packet |

Not measured: load and materialization time, and per-pane pixels of each view
headless (`capture_scene` can show them).

## Non-goals

- Replacing all baked sheets, or requiring every character to have a rig.
- Rendered pixels or sprite transforms as gameplay authority.
- Articulated rigid bodies for movement or stance collision.
- Runtime Python, SVG or vector deformation; a general terrain/body IK
  system without a selected customer. The small existing blend-contact
  solve is part of the current presentation road.
- General skeletal interpolation before a customer needs it.
- Fully physical locomotion, or gameplay-authoritative ragdoll without a
  mechanic.
- One gameplay entity, one portal relationship or one health pool per limb.
- Rig state duplicated per local view.
- A second texture-demand or quality-selection system.

## File map

| Concern | Location |
| --- | --- |
| Authored and prepared rig | `crates/ambition_characters/src/actor/body_rig.rs`, `actor/definition.rs`, `prepared.rs` |
| Grant and retraction | `crates/ambition_platformer2d_actor_spawn/src/character_body.rs` |
| Pose and schedule set | `crates/ambition_combat/src/body_rig.rs` |
| Hurtbox resolution | `crates/ambition_combat/src/hurtbox_resolution.rs` |
| Hand muzzle | `crates/ambition_platformer2d_actor_monolith/src/body/brain_effects.rs` |
| Landmark names and table | `crates/ambition_characters/src/actor/landmarks.rs` |
| Landmark query | `crates/ambition_combat/src/body_landmarks.rs` |
| Embedded landmark tables and digest | `crates/ambition_sprite_sheet/build.rs`, `src/baked_landmarks.rs`, `src/character/landmarks_published.rs` |
| Pet mark | `crates/ambition_platformer2d_actor_monolith/src/body/pet.rs` |
| Player shot origin | `crates/ambition_projectile_combat/src/systems.rs` |
| Flipbook asset and switch | `crates/ambition_sprite_sheet/src/character/rigged.rs` |
| Rigged draw | `crates/ambition_render/src/rendering/actors/rigged*` |
| Portal tint | `ambition_portal2d_presentation::PortalPieceTint` |
| Precedent part player | `game/ambition_content/src/presentation/vanity_card_made_this_meme.rs` |
| Witnesses | `game/ambition_app/tests/admiral_gun_sword.rs`, `game/ambition_demo_mary_o_app/tests/body_rig_trial.rs`, `game/ambition_app/tests/a_pet_hand_meets_the_contact_point.rs`, `game/ambition_app/tests/a_fireball_leaves_the_hand.rs` |
