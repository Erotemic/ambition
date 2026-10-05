# Articulated body rigs and runtime part animation

**State:** BUILT, with two switches. Rigged sprites (the visual half) are on in
the shipped game. Body rigs (the gameplay half: hurt parts and attachments) are
built and off by default. The open work is a hardware-GPU measurement and the
rollout decisions below.

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

### Switches

| Switch | Default | Environment | Effect |
| --- | --- | --- | --- |
| `RiggedSpriteAdmission` (`ambition_sprite_sheet::character::rigged`) | **on** | `AMBITION_RIGGED_SPRITES=0` turns it off | A character that publishes a flipbook draws from parts; every other character draws its baked sheet |
| `BodyRigAdmission` (`ambition_characters::actor::body_rig`) | **off** | `AMBITION_BODY_RIGS=1` turns it on | Off, no definition carries a rig, so no body has hurt parts or rig attachments |

`BodyRigAdmission` is read where a cast is registered, because an admitted rig
changes the character's content fingerprint, which a rollback timeline compares.

Flipbook publishers today: the five pirates (every row from parts) and Mary-O's
three forms (walk from parts, every other row baked).
Moving Mary-O to every row from parts (and lifting two non-goals below for
her): [`mary-o-part-realization.md`](mary-o-part-realization.md).

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
  `<target>_parts.png` and `<target>_parts.ron`. No Python, SVG, IK or vector
  deformation runs in the game. Each quality tier has its own table
  (`<target>.<tier>`) with its own part rects and `texel_scale`; a part keeps
  its full-resolution size and pivot at every tier.
  `scripts/generate_visual_quality_variants.py` publishes tier atlases by
  cropping and downsampling each part, never by resizing a packed page.
- A flipbook states each sheet row as a part clip (`clips`) or a baked clip
  (`baked_clips`). `check_rows` refuses a flipbook that states a row as neither,
  both, or a row the sheet does not have.
- Pages ride on the sheet realization (`CharacterSpriteAsset::rigged`), so the
  sheet's demand, quality and retirement roads own them.
- `ambition_render::rendering::actors::rigged` draws a rigged root.
  `CharacterAnimator` still selects row, frame and facing. One presentation
  owner follows the root, with `max_draws` reusable child slots. A frame change
  does not spawn or despawn slots. The root keeps its baked sprite at zero
  alpha as the parity oracle.
- A body changes to its parts only when every part page is ready, in one frame.
  A tier change keeps the old parts until the new pages are ready. A re-wear
  drops the old character's parts at once.
- A baked clip of a hybrid draws from the root: the root takes its tint back and
  the slots hide, with no jump in place or timing.
- Portals: the rigged root is the portal candidate, drawn from its baked frame.
  `PortalPieceTint` states its visible tint. The owner is `PresentationOf(root)`
  and hides with the root.
- Multiview: slots take their root's render layers. A second view adds no
  presentation entities.
- `StanceSquash` reaches the parts through the owner's transform.

### Measured economics

- Pirate part pages save about 38% of raw texture pixels against the baked
  sheet. `tests/test_pirate_part_flipbook.py` (renderer repo) requires at least
  35% and the parity bound on every frame.
- While the root keeps its baked sheet resident, a rigged character holds about
  1.62x its baked texture bytes.
- ECS cost is about 1.1 us per rigged actor per frame, linear. Each actor is 2 +
  `max_draws` entities.
- On llvmpipe, 100 rigged actors cost about 3x the baked frame time. The cost
  follows sprite count and batch count (each rigged actor makes two batches,
  because the zero-alpha root splits the part page's batch), not fill rate. A
  software rasterizer does vertex work on the CPU, so this does not give the
  cost on a player's machine.

Bench: `crates/ambition_render/examples/rigged_sprite_bench.rs` (`--render`,
`--views 2`, `--tiny`). Matrix on real hardware:
`python3 scripts/rig_packet8_gpu_bench.py` (refuses a software adapter, writes
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
| Action shot (`Discharge::muzzle`) | `Muzzle::{BodyOrigin, Hand { ahead }, Offset { x, y }}` (`action_set/mod.rs`); `Hand` uses the rig hand only when the body has a rig | muzzle, or hand + move offset |
| Rider's hand without a rig | `HAND_OFFSET_NORM` × rider height (`ambition_mount/src/lib.rs`, `rider_hand_world_pos_in_frame`) | hand |
| Player fireball | ✅ 2026-10-05: the landmark query. The shot is born with its rear edge at the hand of the `shoot` row (`projectile/systems.rs`, `player_projectile_hand_local_offset`), lifted clear of the feet line. The box edge plus `PLAYER_PROJECTILE_MUZZLE_CLEARANCE` is the named fallback for a body that publishes no hand | done; the reach it costs is [Q158](../awaiting-maintainer-decision.md#q158--the-fireball-now-leaves-the-hand-at-knee-height-and-reaches-30-less-accept-retune-or-except) |
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
first. Else the package table answers: body → `WornCharacter` → prepared sheet
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
- A package clip wraps on the body's own clock: the table does not say which
  rows hold their last frame.
- A body that states no drawn scale (the legacy `collision_scale` render
  path) has no package answer.
- A posed body whose pose rectangle is not centred on its feet pixel is drawn
  up to about 1.5 world units from the answer (the robot's `pet` row). The
  renderer also adds a posed body's art offset without mirroring it, so the
  art of a body that faces left is off its box by twice that offset.
- A row that a hybrid character realizes from the baked sheet has no draws,
  so it has no points.

**Order of work (queue row RIG-LANDMARKS).** ✅ The query and the pet. ✅ The
player fireball: the move's offset from the hand is the shot's own half extent
along the aim, so the shot's rear edge is at the hand for each of the eight
aims and each charge size. ✅ The three track families and the authored
gesture hand. Next the rider hand. Do not admit rigs (`BodyRigAdmission`)
for this: the capability must not depend on rig rollout. Landmarks are
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

## Open work

| Item | Work | Acceptance |
| --- | --- | --- |
| Hardware GPU run | Run `scripts/rig_packet8_gpu_bench.py` on a machine with a hardware GPU | `report.md` returned; the decision below continues |
| Root batch split | If batches matter on hardware, stop drawing the zero-alpha root. The root must stay the portal candidate, so its size and frame need another road to the portal | One batch per part page across actors |
| Instance buffer | Only if sprite count is too expensive on hardware, replace the slot realization with an instance buffer. Keep `RiggedSpriteAsset`, `BodyRigDefinition` and semantic animation unchanged | Measured gain on hardware |
| Sheet residency | The saving needs the baked sheet page to retire while parts draw, and the portal to draw parts first | Rigged character resident bytes below baked |
| Body rig rollout | `BodyRigAdmission` is off. Turning it on changes shipped hurt geometry and the content fingerprint | Maintainer go-ahead; app suite green with it on |
| More rigid parts | Pirate dynamic limb/neck geometry is one overlay per frame. Convert more of it to reusable parts only if useful | Saving above the 38% floor |
| Semantic landmarks (Q41) | ✅ The query, the pet, the player fireball, and the three track families with an authored gesture hand (2026-10-05). Left: the rider hand as a consumer. See "Semantic landmarks (Q41)" | ✅ A pet hand meets the authored contact point (`a_pet_hand_meets_the_contact_point.rs`: 1.4 world units; 15.2 on the box mark). Open: no consumer reads sprite bounds |
| Physicalized pose | Only with a real mechanic: cosmetic ragdoll after a KO fact, or deterministic constrained ragdoll as canonical rollback state | Separate focused packet |

Not measured: load and materialization time, and per-pane pixels of each view
headless (`capture_scene` can show them).

## Non-goals

- Replacing all baked sheets, or requiring every character to have a rig.
- Rendered pixels or sprite transforms as gameplay authority.
- Articulated rigid bodies for movement or stance collision.
- Runtime Python, SVG, IK or vector deformation.
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
| Hand muzzle | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/brain_effects.rs` |
| Landmark names and table | `crates/ambition_characters/src/actor/landmarks.rs` |
| Landmark query | `crates/ambition_combat/src/body_landmarks.rs` |
| Embedded landmark tables and digest | `crates/ambition_sprite_sheet/build.rs`, `src/baked_landmarks.rs`, `src/character/landmarks_published.rs` |
| Pet mark | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/pet.rs` |
| Player shot origin | `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs` |
| Flipbook asset and switch | `crates/ambition_sprite_sheet/src/character/rigged.rs` |
| Rigged draw | `crates/ambition_render/src/rendering/actors/rigged*` |
| Portal tint | `ambition_portal2d_presentation::PortalPieceTint` |
| Precedent part player | `game/ambition_content/src/presentation/vanity_card_made_this_meme.rs` |
| Witnesses | `game/ambition_app/tests/admiral_gun_sword.rs`, `game/ambition_demo_mary_o_app/tests/body_rig_trial.rs`, `game/ambition_app/tests/a_pet_hand_meets_the_contact_point.rs`, `game/ambition_app/tests/a_fireball_leaves_the_hand.rs` |
