# Smash Siblings: foundational fighter poses and expressive motion

**Status:** Proposed design and art plan (2026-10-30).  
**First customer:** Smash Siblings; later use in Ambition's campaign.  
**Scope:** Character art and visual presentation only. This is not an AI or combat-rules project.  
**Reference:** The Melee reconstruction at `/home/joncrall/code/melee`, not a requirement to copy Melee's animations or timings.

## Decision and outcome

**Author the missing base silhouettes first. Wire them into Smash Siblings second. Improve transitions and procedural motion third.** Do not make runtime rig publication, inverse kinematics, or animation blending prerequisites for new art.

A rough fighter with clear reactions should come before a fluid fighter that reuses the same generic reaction for every event. The first result can look rough. It must still show a visibly different body shape for a high hit, a low hit, an upward launch, a downward launch, a hard stop, and a floor tech.

The goal is a reusable visual vocabulary. A small set of authored base poses can support more visual states than Melee's explicit motion-state enumeration through facing, phase, direction, severity, pose composition, and character-specific art. Do not create a separate gameplay state for each possible visual combination.

This is a focused design document, not a second execution queue. Keep the live work in `docs/planning/queue.md` and the Smash feature inventory in `docs/planning/demos/smash-parity-inventory.md`.

## Evidence from Melee

The Melee reconstruction defines 341 **common motion states** (`ftCo_MS_Count`), with IDs 0 through 340, in `src/melee/ft/kinds/ftCommon/forward.h:287-631`. A motion state is not the same thing as a unique rendered pose. The `ftData_MotionStateList` table in `src/melee/ft/ftmotionstates.c` links those states to animation and behavior callbacks.

The following distinctions are the useful reference for this art pass:

| Physical event | Melee states | What it tells us to depict |
| --- | --- | --- |
| Local hit | `DamageHi1/2/3`, `DamageN1/2/3`, `DamageLw1/2/3`, `DamageAir1/2/3` (`forward.h:364-375`) | The place and strength of a hit can change the body silhouette. |
| Launch | `DamageFlyHi`, `DamageFlyN`, `DamageFlyLw`, `DamageFlyTop`, `DamageFlyRoll` (`forward.h:376-380`) | Directional launch and tumble deserve different base shapes. |
| Prone | `DownBound/Wait/Damage/Stand/Attack/...` for both `U` and `D` (`forward.h:472-487`) | Face-up and face-down ground states need different poses and exits. |
| Tech | `Passive`, `PassiveStandF/B`, `PassiveWall`, `PassiveWallJump`, `PassiveCeil` (`forward.h:488-493`) | A successful impact recovery should look unlike an ordinary getup. |
| Teeter | `Ottotto`, `OttottoWait` (`forward.h:534-535`) | Losing balance and holding balance are separate visual beats. |
| Ledge | `CliffCatch`, `CliffWait`, climbs, attacks, escapes, jumps (`forward.h:541-552`) | The first hand contact, the hang, and the exit should read separately. |

`src/melee/ft/kinds/ftCommon/ftCo_Damage.c:61-74` also contains an indexed table for damage-motion selection. Melee's precise animation parameters are not the target. The lesson is the deliberate vocabulary of physical events.

## Evidence from the current Ambition art

**Audit source:** Ambition source snapshot `ambition-agent-source-20261030T193907Z-f34bb10815eb`, dated 2026-10-30. The numbers below count source JSON, not finished visual quality or successful runtime selection.

The three relevant motion libraries are under:

`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid/`

- `fighting_polygon_v1`: Pointed Polygon.
- `fighting_brawler_v1`: Pugnacious Polygon.
- `projectile_beast_v1`: Projectile Polygon.

Their character bindings are in `data/characters/{pointed_polygon,pugnacious_polygon,projectile_polygon}/*.motion.json`. Each library contains **136 named clip JSON files** and **16 separately named pose JSON files**. Counting the serialized `state` or `pose` entries in each clip's `pose_keys` gives **99 distinct keyed sequences** for Pointed and Pugnacious and **100** for Projectile. This is not a count of unique visual silhouettes: named pose references, clip timing, interpolation, and rendering can change the visible result.

The duplicate-key evidence is more useful than the raw count:

| Existing clips | Current source relationship | Art implication |
| --- | --- | --- |
| `launch`, `meteor`, `shield_break_fall` | Identical keyed pose sequences | A meteor does not have its own directional body shape. |
| `hit`, `impact`, `shield_hit`, `prone_damage`, `grabbed_pummel` | Identical keyed pose sequences | Distinct hit events share one recoil movement. |
| `idle_look_up`, `loss`, `teeter` | Identical keyed pose sequences | The teeter hold has no dedicated balance movement. |
| `air_land`, `ceiling_tech`, `tech`, `walk_stop` | Identical keyed pose sequences | A floor tech does not have a distinct braced body shape. |
| `getup_roll`, `grab_escape`, `spot_dodge`, `tech_roll`, `trip_roll` | Identical keyed pose sequences | Several different escapes share one roll. |
| `knockdown`, `sleep_start`, `trip_fall` | Identical keyed pose sequences | Floor reactions lack independent body language. |

These groups were found by comparing keyed transforms in all three libraries. The clips can still have different playback durations. The audit does not claim that every pair looks identical at every timestamp.

There is real work to preserve: `teeter_start`, `ledge_catch`, `ledge_grab`, `ledge_climb`, `ledge_getup`, `ledge_jump`, `ledge_drop`, `land_hard`, `wall_tech`, `tumble`, and other clips already exist. Do not redraw these only because they have separate names. Review their silhouettes first.

The canonical `data/fighter_motion_vocabulary.yaml` has **271 motion labels**, of which **196** have `fighter_core` scope. It already names high/mid/low/aerial hit reactions, directional launches, face-up/down prone reactions, floor-tech variants, and a distinct teeter hold. Its own policy says full variation coverage is aspirational and several labels can share one baseline slot. We now want to replace a selected set of those shared baselines with real art.

The authored motion contract is documented in `tools/ambition_sprite2d_renderer/docs/authoring/motion_ir_v1.md`: SVG owns rig geometry and part bindings; motion JSON owns named poses, clips, key times, and transform deltas. `PreparedCharacterMotion` feeds the existing sheet publisher. Use that contract. Do not create a new pose format.

## Phase 1 - Author foundational silhouettes (first major deliverable)

**Finish and review the art before Phase 2 starts.** First complete one reference fighter, then bring the other two Polygon fighters to comparable silhouette coverage using their own motion libraries and geometry. Shared movement ideas are fine; copied transforms are not automatically good art.

### The minimum initial pack: 12 base pose anchors

These are **twelve art targets**, not necessarily twelve complete animations or exactly twelve files. A useful transition usually needs several keys around its main pose. Reuse the existing motion vocabulary and choose stable clip/pose names that its mappings can resolve.

| Order | Base silhouettes to author | Count | Evidence and art requirement |
| --- | --- | ---: | --- |
| 1 | High-body hit; mid-body hit; low-body hit; airborne hit | 4 | `hit` is the generic fallback. Show a distinct point of recoil, stance, and center of mass for each region. Use controlled strength changes within each base. |
| 2 | Upward launch; horizontal launch; downward/meteor launch | 3 | `launch` and `meteor` share keyed transforms. Give each a distinct trunk angle and limb arrangement. A rotation of one whole generic pose is not sufficient. |
| 3 | Face-up prone; face-down prone | 2 | `prone`/`knockdown` do not represent both ground orientations. Differentiate how the back, torso, head, and legs contact the floor. |
| 4 | Teeter hold | 1 | `teeter_start` exists, but `teeter` shares `idle_look_up`. Author a clear off-balance hold with one foot near the edge and counterbalancing upper-body parts. |
| 5 | Floor-tech brace | 1 | `tech` shares `walk_stop`. Show a deliberate catch of the landing force, not a generic standing recovery. |
| 6 | Run brake / hard stop | 1 | No `run_stop` clip in the audited Polygon libraries. Show momentum with planted foot, displaced hips, and trailing upper body. |
| **Total** | | **12** | Initial coverage target; add or merge anchors based on actual silhouette review. |

Use the vocabulary's target slots where practical: `hit_high`, `hit_mid`, `hit_low`, `hit_air`, `launch_up`, `launch_horizontal`, `launch_down`, `run_stop`, `tech`, and `teeter`. Keep old clip IDs available as fallbacks until the runtime routes to new clips. For prone orientation, use named face-up and face-down source poses even if the first published clip format keeps one shared `prone` fallback.

**Art requirements for every target:**

1. Start from the character's actual SVG rig and art proportions. Keep the character recognizable at ordinary Smash camera scale.
2. Make the main silhouette visibly different from the current fallback. Judge it in a flat silhouette view before judging facial detail or effects.
3. Set meaningful torso, head, arm, hand, hip, and leg positions. Keep a believable center of mass. A different clip name or playback speed is not sufficient.
4. Record an entry, contact/peak, and exit or hold when the motion needs them. Do not force every base pose to have the same number of keys.
5. Keep attachments, sword grip, feet, and part order plausible. Compare mirrored facing. Do not use the visual pose to change gameplay hit or hurt geometry.
6. Render a contact sheet of old and new poses side by side, with the same scale and orientation. Add a short animated preview for each reaction. Review the result at real in-game display size, not only a large editor preview.

**Authoring order:** localized hits and directional launches first; then prone orientation; then teeter, floor tech, and run brake. These sets give the largest change in the visual language of impact and movement.

### What can be reused without new art

Do a review, not a reauthoring pass, for the already distinct `ledge_catch` and `ledge_grab` clips, ordinary `tumble`, `land_hard`, and existing ledge exits. They are likely Phase 2 wiring or Phase 3 transition tasks. Inspect the exact keys before declaring any existing pose finished.

### Art backlog after the first 12

Do not let this defer Phase 2. Candidate next art families are a braced `shield_hit` (currently identical to `hit`), wall and ceiling impact/splat, separate forward/back floor-tech rolls, distinct face-up/down getups, aerial-attack landing, and fighter-specific idle/guard identity. Choose these after reviewing the initial set in play.

### Phase 1 exit gate

- All 12 anchor targets have actual authored silhouette changes on the reference fighter, with supporting clip keys where required.
- The source is SVG plus Motion IR JSON. The normal renderer can validate, sample, and publish every new or changed clip. No handwritten spritesheet replacement is the authority.
- The art review includes old/new contact sheets and short motion previews, including left/right facing and normal screen scale.
- The authoring tests for each changed target pass or have intentional, reviewed golden-image updates.
- The second and third Polygon fighters have equivalent basic coverage and a recorded review of any anatomy-specific changes.

**This gate does not require the game to select the new poses yet.** That is Phase 2.

## Phase 2 - Wire the authored poses into the game (second major deliverable)

Use the existing clip-selection seam instead of introducing 341 new gameplay states. The game already publishes rich motion facts. Add a minimal presentation fact or cue only when the exact distinction is lost before the renderer receives it.

| Family | Proposed presentation routing | Existing facts or missing input |
| --- | --- | --- |
| Hit region and severity | Pick the authored high/mid/low/air recoil, then vary strength as art permits. | An accepted hit is known to the combat resolver, but `ResolvedBodyHit` lacks the contact region and final direction. Publish a small resolved visual-impact cue from authoritative hit processing. Do not treat overlap alone as a landed hit. |
| Launch direction | Choose up/horizontal/down from the resolved launch direction; retain tumble when applicable. | Add only the resolved launch information not already preserved by the presentation read-model. Avoid a second calculation of combat knockback. |
| Prone orientation | Pick face-up or face-down prone and the matching getup, using the outcome that put the body on the ground. | Verify whether the knockdown owner retains orientation. If it does not, add a small pose-facing outcome fact, not an inferred image-space guess. |
| Teeter | Play `teeter_start` once on entry, then `teeter` as a hold; interrupt on movement or fall. | `BodyMotionFacts::teetering` already exists. A presentation transition can observe its start without changing physics. |
| Floor tech | Select a specific tech clip only when the corresponding tech succeeded. | `body_state_clip()` currently receives `getup_invulnerable` but not the actual tech/getup distinction. The movement owner must publish the resolved action type. |
| Run brake | Play `run_stop` on a real stop or skid, not on every frame below run speed. | `BodyMotionFacts` already includes running and turning information, and the movement/animation path knows the grounded gait. Use its authoritative transition. |

The principal routing point is `crates/ambition_character_sprites/src/anim/mod.rs`, especially `body_state_clip()` (around lines 330-430) and the common animation selection path. `body_state_clip()` explicitly states that `tech`, `tech_roll`, and `getup_roll` collapse because the available facts do not distinguish them. `pick_body_anim()` already checks `teetering` (around lines 566-589), but does not select the start clip separately.

Keep one rule for player and actor bodies. The two relevant readers are `crates/ambition_sim_view/src/pose_view.rs` (`BodyPoseView`) and `crates/ambition_sim_view/src/anim_index.rs` (`ActorAnimFrame`). The player path supports upper-body moves with continuing locomotion legs (`pose_view.rs:645-659`), while the actor path creates its active-move `ClipRequest` without the same leg request (`anim_index.rs:421-424`). Check and unify this visual behavior while wiring the new poses; do not create a player-only workaround.

`crates/ambition_combat/src/hitbox/mod.rs:36-46` defines `LandedBodyHit` with a world contact point, but that message means geometric overlap. `ResolvedBodyHit` (from line 67) means the hit was accepted, and it does not expose final direction or hit region. This distinction is an actual Phase 2 data task for localized impact art. Use a small visual cue from hit resolution, with stable replay/rollback behavior. Keep gameplay outcomes authoritative.

The new clips can first run through the **existing sampled sheet and part-flipbook playback**. Do not require a runtime `PartPose` provider to show the twelve new silhouettes. This keeps the art and the new selection logic testable before continuous visual-joint evaluation exists.

### Phase 2 exit gate

- Each new pose can be triggered by an ordinary game event or a deterministic presentation fixture. No renderer-only diagnostic input may be required in shipped play.
- Human-driven and CPU-driven bodies show the same pose for the same event and rig.
- A trace names the requested clip and the selected published clip. Missing clips fall back by documented policy, not silently to unrelated gestures.
- Recorded Smash matches show the full sequences: high/low hit; up/down launch; launch to prone; teeter enter/hold/leave; floor tech; run and stop.
- Visual-only changes do not alter collision, hitboxes, damage, move windows, or rollback results. Replays and camera captures can reproduce the same motion choice.

## Phase 3 - Improve continuity and body weight (polish)

Only after the silhouettes appear in the game, improve how one pose becomes another:

1. Blend compatible clip changes at joint level while preserving hit-contact timing and clear attack silhouettes.
2. Keep a planted foot stable during stops, teeters, and landing. Keep hands visually attached on a ledge catch and hang.
3. Add bounded follow-through to arms and head after hits, braking, and landings. Do not smooth away a sharp impact or active attack pose.
4. Bring upper/lower body composition, facing, interruption, and rollback-reset policy into one shared presentation path.
5. Evaluate part merging, joint binding, quality tiers, portal rendering, and off-cell bounds before enabling arbitrary procedural joint offsets.

**Known future technical limit:** `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/part_flipbook.py:1347-1403` enables smooth part-transform interpolation by default only for selected locomotion loops; other published clips step. `crates/ambition_sprite_sheet/src/character/animator.rs:443-466` resets the frame when the clip changes. These are polish issues, not reasons to postpone new art.

**Known runtime pose limit:** `crates/ambition_sprite_sheet/src/character/rigged/posed.rs:77-182` can place parts from joints, but binds parts by matching sampled animation frames within a tolerance. `crates/ambition_render/src/rendering/actors/rigged.rs:1019-1027` needs a baked body rig for that path. The audited Polygon publishers call `build_sheet()` and `publish_rig_flipbook()`, but do not publish that body-rig product. Part optimization can also merge always-rigid neighboring tracks (`part_flipbook.py:1675+`). If Phase 3 needs independent runtime joint control, first publish a suitable **visual** rig and stable semantic part bindings. Do not enable experimental gameplay-rig hurt geometry as a side effect. Do not front-load this bridge into Phase 1 or 2.

## Phase 4 - Expand expressive coverage after the first playable pass

Review the rest of Melee's motion vocabulary and Ambition's `fighter_motion_vocabulary.yaml` by physical event family, not by raw state count. Add new bases only when the current rig cannot make the event legible. Give different fighters their own stance and recoil style after the shared pose language works. The same visual system must later serve Ambition campaign characters, but no campaign conversion belongs in this first pass.

## Risks and constraints

| Risk | Response | Blocks |
| --- | --- | --- |
| New clips are only renamed copies | Require new joint silhouettes and side-by-side art review. | Phase 1 acceptance |
| Published assets clip or change gameplay geometry | Validate part extents, mirrored facing, attachment positions, and sheet publication. Keep visual art separate from hit and hurt volumes. | Phase 1/2 acceptance |
| Current game cannot name a hit region or tech result | Add the smallest resolved presentation cue in Phase 2, from the owning subsystem. | Phase 2, not Phase 1 |
| Runtime visual joint rig is absent on Polygon | Use existing sampled clips first; defer arbitrary live articulation. | Phase 3 only |
| Sprite authoring lives in a submodule | Make art changes in `tools/ambition_sprite2d_renderer` at its own source authority, then update the superproject pin according to repository policy. | Packaging and integration, not art design |
| New pose looks good in the editor but unreadable in play | Compare at 128x192 authoring frame size and at actual Smash camera distance; review timed gameplay clips. | Every visual acceptance gate |
| Early polish hides missing art | Do not substitute cross-fades, rotation of one generic hit pose, or procedural wobble for a missing silhouette. | Phase 1 |

## Source index and reproducible checks

**Melee reference:**

- `/home/joncrall/code/melee/src/melee/ft/kinds/ftCommon/forward.h` - common motion-state vocabulary.
- `/home/joncrall/code/melee/src/melee/ft/ftmotionstates.c` - shared state table.
- `/home/joncrall/code/melee/src/melee/ft/kinds/ftCommon/ftCo_Damage.c` - damage-motion selection.

**Ambition source and owner docs:**

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/fighter_motion_vocabulary.yaml` - long-term motion names and baseline-slot policy.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid/{fighting_polygon_v1,fighting_brawler_v1,projectile_beast_v1}/` - measured pose and clip source.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/{pointed_polygon,pugnacious_polygon,projectile_polygon}/` - rig bindings.
- `tools/ambition_sprite2d_renderer/docs/authoring/motion_ir_v1.md` - pose/clip contract, schema checks, and the Godot pose editor workflow.
- `tools/ambition_sprite2d_renderer/tests/test_motion_ir.py`, `tests/test_pointed_polygon_target.py`, `tests/test_pugnacious_polygon_target.py`, `tests/test_projectile_polygon_target.py` - focused authoring validation.
- `crates/ambition_character_sprites/src/anim/mod.rs` - current animation selection and tech/getup limitation.
- `crates/ambition_platformer2d_core/src/movement/facts.rs` - authoritative movement facts, including teetering.
- `crates/ambition_combat/src/hitbox/mod.rs` - difference between overlap and accepted hit.
- `crates/ambition_sim_view/src/pose_view.rs` and `anim_index.rs` - player and actor presentation readers.
- `docs/planning/engine/runtime-rigged-sprite-animation.md` and `semantic-part-rendering-and-ragdolls.md` - current art/render contracts and future rig work.

To reproduce the clip-alias audit, load each library's `clips/*.clip.json`, extract the ordered `pose_keys[*].state` or `pose_keys[*].pose` values, and group clips by identical ordered sequences. Do not include duration in the grouping; report timing differences separately. This audit is a planning baseline, not a visual comparison. Re-run it after art changes, and record which intentional aliases remain.

For source validation, `motion_ir_v1.md` documents `motion_ir_tool validate`, the generated Godot pose-sheet workflow, and its headless export round-trip check. Use the existing renderer and target tests for actual publication. Do not require a new art editor or a new animation schema.

## Definition of complete

This work is complete when a player can watch a normal Smash Siblings match and distinguish at least the following without reading UI or debug text: where a hit landed, which way a body was launched, which way a knocked-down body faces, whether a body is bracing for a floor tech, when a fighter is about to lose balance, and when a run ends in a hard stop.

The character may still have stepped frame transitions at that point. It should already **look like it is responding to the physical rules of a platform fighter**, not cycling through a small set of generic poses. Smoothness and secondary motion then make that visual language more natural.
