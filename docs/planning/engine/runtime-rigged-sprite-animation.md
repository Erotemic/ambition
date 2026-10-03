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
| Flipbook asset and switch | `crates/ambition_sprite_sheet/src/character/rigged.rs` |
| Rigged draw | `crates/ambition_render/src/rendering/actors/rigged*` |
| Portal tint | `ambition_portal2d_presentation::PortalPieceTint` |
| Precedent part player | `game/ambition_content/src/presentation/vanity_card_made_this_meme.rs` |
| Witnesses | `game/ambition_app/tests/admiral_gun_sword.rs`, `game/ambition_demo_mary_o_app/tests/body_rig_trial.rs` |
