# Smash Siblings: foundational fighter poses and expressive motion

**Status:** Base pose art and wiring are complete. Remaining visual work is selected in
[POSE-CONTINUITY](../queue.md#pose-continuity--one-visual-transition-path-after-the-completed-base-poses) (2026-10-31).
**Current shared customers:** Scout and campaign bodies in Ambition; fighters in Smash.
The original base-pose pass used Smash first. CLIP1–CLIP3 below are shared work.
**Scope:** Character art, presentation selection and prepared source-time
bindings. CLIP3 also corrects the source sample consumed by admitted physical
pose/geometry. Combat balance and AI policy remain with their own plans.
**Reference:** The Melee reconstruction at `/home/joncrall/code/melee`, not a requirement to copy Melee's animations or timings.

## Implementation status (2026-10-30)

**Phase 1 art: authored, published, and approved by Jon at game scale (2026-10-31).** Renderer `385dcf6`: `devtools/pose_core.py` (the shared rig, solver, keys and clip writer), `devtools/reaction_keys.py` (the twelve anchors), `devtools/reaction_review.py` (contact sheets, `--silhouette`, `--gif` previews in both facings) and `devtools/clip_alias_audit.py`. All twelve rows exist on all three Polygon fighters. `teeter` and `tech` no longer alias other rows, and no new row aliases another. The hit recoils end on their settled pose, because the animator holds a clip's last frame for the length of the hitstun. Sheet budget (MEASURED, previous bundle to this one), pointed: full 9.98 to 11.06 MB, half 3.81 to 4.22 MB, quarter 1.46 to 1.61 MB, potato 186 to 201 KB. The other two fighters grew by the same fraction. Renderer suite: **1610 passed, 0 failed** (MEASURED 2026-10-30; the one failure in the last full run was a peer's effect target added mid-run, listed since). This work fixed the 18 failures: clip counts for the new rows (`385dcf6`); the roster tests read `authored/sprites.yaml`, which found `ninja_kagero_vfx` unlisted and never published, now listed and published (`ca94a72`, `c72f90679`); the install audit claims the actor, rig and part companions the install copies (`446e878`); the bundle test asserts its stated intent (`446e878`); `door_swing.py` moves out of core (`446e878`); the rig editor's right-dock pages scroll so the Timeline can grow, and the oiler tests read views and faces (`dc8eaa0`); content draws composite through `blending_draw`, pixel-identical, with four measured raw exceptions, George Booul's draws that break his part replay (`9b057f8`); the blueprint door's leaf composites too, solid navy (`200afcd`).

**Phase 2 wiring: done for launch, prone, getup and tech, recoil, and run stop** (`611177e91`). The kernel publishes `BodyMotionFacts::{launched, prone_face_down, getup}`. `BodyCombat::struck_height` records where the hit resolver accepted a hit. `body_state_clip` routes all four families. `StopClock` is one stop rule for the player road and the actor road (the actor road had no stop rule before). `AMBITION_CLIP_TRACE=1` prints the requested chain and the row drawn. That encoding change used rollback schema v392.

Evidence from a traced CPU match (MEASURED: `smash_match_profile --character pointed_polygon --fighters 2 --ticks 2400`). Drawn: `launch_horizontal` 8, `launch_up` 6, `launch_down` 1, `prone_face_down` 6, `prone_face_up` 2, `hit_high` 4, `hit_mid` 2, `hit_air` 2, `getup` 5, `walk_stop` 3.

Evidence for every family, human and CPU seats (MEASURED: `app_it::smash_reaction_poses`, in the shipped composition). The test checks that each reaction row a body asks for is the row its animator draws, and prints each sequence (T tumbling, K knocked down, then the getup kind):

```text
human thrown up:      launch_up[T] -> launch_horizontal[T] -> tumble[T] -> prone_face_up[K] -> getup[Getup] -> Idle
human thrown back:    launch_horizontal[T] -> tumble[T] -> prone_face_up[K] -> getup[Getup] -> Idle
human thrown forward: launch_horizontal[T] -> tumble[T] -> prone_face_down[K] -> getup[Getup] -> Idle
human spiked:         launch_down[T] -> prone_face_down[K] -> getup[Getup] -> Idle
human tech:           tech[Tech]
human hit high / low: hit_high / hit_low
human run, stop:      Idle -> Walk -> Run, then Run -> Walk -> run_stop
human teeter:         Teeter, then Teeter -> Walk -> Run off the edge
cpu thrown up:        launch_up[T] -> tumble[T] -> air_down[T] -> tumble[T] -> tech[Tech] -> ...
cpu thrown back:      launch_horizontal[T] -> tumble[T] -> prone_face_up[K] -> getup_roll[GetupRoll] -> ...
cpu thrown forward:   launch_horizontal[T] -> tumble[T] -> tech_roll[TechRoll] -> ...
cpu spiked:           launch_down[T] -> prone_face_down[K] -> getup_roll[GetupRoll] -> ...
cpu hit low:          hit_low
```

The launches go through the launch gateway a hit uses, the run and the teeter use a real pad, and the CPU's techs are its own. The human's tech and the recoils write the state the kernel and the hit resolver write. In the up-launch, a second hit mid-flight launched the body again, which re-decided the direction. The test found one defect, now fixed: a run slows through a few Walk frames before it stops, so a run's stop drew `walk_stop`. `StopClock` now remembers a run it left less than `RUN_STOP_AFTER_RUN_SECS` before.

Old/new contact sheets at the full tier and at about gameplay scale (0.35): `reaction_review` with the fallback rows interleaved (`launch,launch_up,...,walk_stop,run_stop`). The weakest changes at gameplay scale are pugnacious's `run_stop` and `hit_low`.

**Second pass and close (2026-10-31).** A harder `run_stop` and `hit_low` on all three fighters, and the beast rears back to brake (renderer `4ac311e`, `4183e06`). Smash keeps drawing fighters between ticks (`interpolate_poses`). **Phase 1 and Phase 2 exit gates: met.** Next: Phase 3 continuity, and the backlog (getups matching the prone orientation, `teeter_start`, wall/ceiling tech routing and art review).

## Decision and outcome

**Author the missing base silhouettes first. Wire them into Smash Siblings second. Improve transitions and procedural motion third.** Do not make runtime rig publication, inverse kinematics at runtime, or animation blending prerequisites for new art.

A rough fighter with clear reactions should come before a fluid fighter that reuses the same generic reaction for every event. The first result can look rough. It must still show a visibly different body shape for a high hit, a low hit, an upward launch, a downward launch, a hard stop, and a floor tech.

**De-risk on one reference fighter before producing all 36 targets.** Author the first families on one fighter, publish them through the normal renderer, and view them at gameplay scale. A narrow smoke test that puts a new clip on screen is allowed. Extensive game wiring is not a prerequisite for completing the foundational art.

**Start with the three directional launch silhouettes on the reference fighter.** Their visual inputs already exist in the game. Then author the localized hit reactions. The complete twelve-anchor target remains the Phase 1 goal.

The goal is a reusable visual vocabulary. A small set of authored base poses can support more visual states than Melee's explicit motion-state enumeration through facing, phase, direction, severity, pose composition, and character-specific art. Do not create a separate gameplay state for each possible visual combination.

This is a focused design document, not a second execution queue. Keep the live work in `docs/planning/queue.md` and the Smash feature inventory in `docs/planning/demos/smash-parity-inventory.md`.

## Evidence from Melee

The Melee reconstruction defines 341 **common motion states** (`ftCo_MS_Count`), with IDs 0 through 340, in `src/melee/ft/kinds/ftCommon/forward.h:287-631`. A motion state is not the same thing as a unique rendered pose. The `ftData_MotionStateList` table in `src/melee/ft/ftmotionstates.c` links those states to animation and behavior callbacks.

The following distinctions are the useful reference for this art pass:

| Physical event | Melee states | What it tells us to depict |
| --- | --- | --- |
| Local hit | `DamageHi1/2/3`, `DamageN1/2/3`, `DamageLw1/2/3`, `DamageAir1/2/3` (`forward.h:364-375`) | The place and strength of a hit can change the body silhouette. |
| Launch | `DamageFlyHi`, `DamageFlyN`, `DamageFlyLw`, `DamageFlyTop`, `DamageFlyRoll` (`forward.h:376-380`) | A launch deserves its own base shapes, separate from a flinch. |
| Prone | `DownBound/Wait/Damage/Stand/Attack/...` for both `U` and `D` (`forward.h:472-487`) | Face-up and face-down ground states need different poses and exits. |
| Tech | `Passive`, `PassiveStandF/B`, `PassiveWall`, `PassiveWallJump`, `PassiveCeil` (`forward.h:488-493`) | A successful impact recovery should look unlike an ordinary getup. |
| Teeter | Play an entry, then a held teeter pose; interrupt on movement or fall. | `BodyMotionFacts::teetering` exists. Recheck the current pose clock before adding an entry timer or `teeter_start` art. |
| Ledge | `CliffCatch`, `CliffWait`, climbs, attacks, escapes, jumps (`forward.h:541-552`) | The first hand contact, the hang, and the exit should read separately. |

### How Melee selects a damage motion

`src/melee/ft/kinds/ftCommon/ftCo_Damage.c:61-74` defines `ftCo_803C5520[2][4][3]`, and `ftCo_Damage.c:318-362` indexes it as `[ground or air][knockback level][damaged hurtbox]`:

- **Region (Hi / N / Lw) is the hurtbox that was struck**, not a contact point.
- **Severity is the knockback level.** Levels 0–2 select `Damage*1/2/3`; level 3 selects `DamageFlyHi/N/Lw`, again by the struck hurtbox.
- **In the air, the ordinary rows ignore region.** Every hurtbox maps to `DamageAir1/2/3`. This supports a single airborne-hit silhouette.
- **Airborne launch selection is specialized.** `DamageFlyTop` and `DamageFlyRoll` are chosen by separate logic, not by this table.

Melee's precise animation parameters are not the target. The lesson is the deliberate vocabulary of physical events and the two axes (where it struck, how hard) behind ordinary hit reactions.

**Ambition's launch posing is a deliberate departure.** This plan chooses upward, horizontal and downward launch silhouettes from the **launch direction**, where Melee's `DamageFlyHi/N/Lw` follow the struck hurtbox. Direction is what a viewer reads in flight, and it is already available to presentation. It is a design choice, not a reproduction.

## Current art foundation

The three Polygon libraries are `fighting_polygon_v1`,
`fighting_brawler_v1` and `projectile_beast_v1`, under the renderer's
`data/motion/humanoid/`. Their character bindings select each rig's motion.
The October base-pose pass replaced selected shared sequences with distinct
launch, hit, prone, tech, teeter and stop art. Current source includes
`run_stop`; the old claims that teeter aliases look-up and tech aliases
walk-stop are superseded.

The old counts and duplicate-key census remain in Git history. They are not
a current missing-art list. The committed `clip_alias_audit` compares
keyed sequences; a distinct sequence still needs review at game scale.
CLIP1/CLIP2 checks whether the runtime selects and times the existing art
before a new source-art pass.

Use the renderer's current motion vocabulary and declared source authority.
SVG/rig geometry, procedural motion programs and prepared motion products
have different edit owners; [SA1](../engine/asset-authoring-and-prepared-media.md#sa1--editable-sources-and-disposable-sprite-projections)
owns that boundary. No new pose format is required.

### The existing pose generator

The Polygon fighters' attack rows are not hand-edited JSON. `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/devtools/polygon_keys.py` generates them:

- **Poses in world terms.** Each pose is a `Key` in forward-relative world degrees, with eases (`out` for a cut, `gather` for a wind-up).
- **Per-rig solving.** Each fighter's own rig (`Body`) solves the poses into Motion IR deltas.
- **Planted feet.** It places the boots with two-bone IK (`authoring/skeleton.py::two_bone_ik`).
- **One run, three fighters.** `apply` writes all three from their own skeletons.

It also fits each attack to `SMASH_CLOCKS` and writes the attack swing specs, which reaction art must not depend on. See Phase 1, "Authoring path".

## Phase 1 - Author foundational silhouettes (first major deliverable)

**Finish and review the art before Phase 2 starts.** Author the first families on one reference fighter, smoke-test them in the published sheet, then complete the twelve anchors there, then bring the other two Polygon fighters to comparable silhouette coverage from their own rigs. Shared movement ideas are fine; copied transforms are not automatically good art.

### Authoring path

**Write a sibling reaction-pose generator** (for example `devtools/reaction_keys.py`) on the same foundation as `polygon_keys.py`:

- **Reuse** its world-space `Key`, `sample`, ease table, the `Body` rig solver, foot IK, and Motion IR clip writer.
- **Do not reuse** `SMASH_CLOCKS`, `active_frames`, or the swing-spec writer. Reaction art must not be coupled to attack timing or to hitbox generation.
- **Separate the shared part first.** `Body` currently takes an attack `clocks` name. Lift the rig/solver/writer half into a module both generators import, so neither generator carries the other's concerns.
- **Use the `out` ease for impacts and launches.** The `gather` ease is for wind-ups and would soften an impact.

The output stays SVG plus Motion IR JSON, published by the normal renderer. No handwritten spritesheet replacement is the authority.

### The minimum initial pack: 12 base pose anchors

These are **twelve art targets**, not necessarily twelve complete animations or exactly twelve files. A useful transition usually needs several keys around its main pose. Reuse the existing motion vocabulary and choose stable clip/pose names that its mappings can resolve.

| Order | Base silhouettes to author | Count | Evidence and art requirement |
| --- | --- | ---: | --- |
| 1 | Upward launch; horizontal launch; downward/meteor launch | 3 | `launch` and `meteor` share keyed transforms. Give each a distinct trunk angle and limb arrangement. A rotation of one whole generic pose is not sufficient. **Author these first, on the reference fighter.** |
| 2 | High-body hit; mid-body hit; low-body hit; airborne hit | 4 | `hit` is the generic fallback. Show a distinct point of recoil, stance, and center of mass for each region. Use controlled strength changes within each base (Melee's second axis is knockback level). |
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
6. **Feet and every supported body part actually contact the floor.** A prone body lies on the floor, a tech brace and a run brake plant their feet, and nothing hovers. The body box is the body alone.
7. **No baked drop shadows**, in these poses or in any later impact or splat art.
8. Render a contact sheet of old and new poses side by side, with the same scale and orientation, at `high` or `ultra` visual quality. Add a short animated preview for each reaction. Then validate at real gameplay scale, not only in a large editor preview.

**Authoring order:** directional launches first, then localized hits, on the reference fighter; then prone orientation, teeter, floor tech, and run brake; then the other two fighters. The first two families give the largest change in the visual language of impact.

### Phase 1 smoke test (narrow, allowed)

After the launch family is published on the reference fighter, put each new clip on screen once: a deterministic presentation fixture or a temporary clip override is enough. The purpose is to catch publication, sheet-budget, part-order, and scale problems on one fighter before authoring 36 targets. It is not Phase 2 wiring and must not become shipped selection logic.

### What can be reused without new art

Do a review, not a reauthoring pass, for the already distinct `ledge_catch` and `ledge_grab` clips, ordinary `tumble`, `land_hard`, and existing ledge exits. They are likely Phase 2 wiring or Phase 3 transition tasks. Inspect the exact keys before declaring any existing pose finished.

### Art backlog after the first 12

Do not let this defer Phase 2. Candidate next art families are a braced `shield_hit` (currently identical to `hit`), wall and ceiling impact/splat, separate forward/back floor-tech rolls, distinct face-up/down getups, aerial-attack landing, and fighter-specific idle/guard identity. Choose these after reviewing the initial set in play.

### Source-authoring tasks that support the art

- **Reproducible alias audit.** Commit a small script (in the renderer's `devtools`) that loads each library's `clips/*.clip.json`, extracts the ordered `pose_keys[*].state` or `pose_keys[*].pose` values, groups clips by identical ordered sequences, and reports duration differences separately. Re-run it after art changes and record which intentional aliases remain.
- **Sheet budget.** Record the published sheet size change per quality tier for the reference fighter after the smoke test, before scaling to three fighters.

### Phase 1 exit gate

- All 12 anchor targets have actual authored silhouette changes on the reference fighter, with supporting clip keys where required.
- The source is SVG plus Motion IR JSON written by the reaction-pose generator. The normal renderer can validate, sample, and publish every new or changed clip.
- The art review includes old/new contact sheets at `high`/`ultra` quality and short motion previews, including left/right facing and gameplay scale.
- Feet and supported parts contact the floor in every grounded and prone target; no baked shadows.
- The renderer submodule's own test suite passes (`uv run python -m pytest -q` in `tools/ambition_sprite2d_renderer`; no main lane runs it), with intentional, reviewed golden-image updates where a target changed.
- The alias-audit script reports the resolved aliases.
- The second and third Polygon fighters have equivalent basic coverage and a recorded review of any anatomy-specific changes.

**This gate does not require the game to select the new poses yet.** That is Phase 2.

## Phase 2 - Wire the authored poses into the game (second major deliverable)

Use the existing clip-selection seam instead of introducing 341 new gameplay states. The game already publishes rich motion facts. Add a minimal presentation fact or cue only when the exact distinction is lost before the renderer receives it.

| Family | Proposed presentation routing | Existing facts or missing input |
| --- | --- | --- |
| Launch direction | Select up / horizontal / down from the accepted launch. | Built: `BodyMotionFacts::launched` retains launch-time direction. Do not reclassify it from later falling velocity or a pooled VFX view. |
| Hit region and severity | Select the authored recoil from accepted hit processing. | Built: `BodyCombat::struck_height` records the accepted contact height; the shared clip selector reads it. Do not recompute damage or treat an overlap as an accepted hit. |
| Prone orientation | Select the prone orientation and matching getup. | The movement owner publishes `BodyMotionFacts::prone_face_down` and `BodyMotionFacts::getup`. Remaining orientation-specific art belongs to POSE-CONTINUITY. |
| Teeter | Play an entry, then a held teeter pose; interrupt on movement or fall. | `BodyMotionFacts::teetering` exists. Recheck the current pose clock before adding an entry timer or `teeter_start` art. |
| Floor tech | Select a tech clip after a successful tech. | Built: the movement owner publishes the getup kind and the shared selector routes the tech and roll families. |
| Run brake | Select `run_stop` after a run and `walk_stop` after a walk. | Built: `StopClock` carries the shared stop rule for human and CPU bodies. Do not select it from `BodyMotionFacts::skidding`. |

The common selector is `body_state_clip` in
`crates/ambition_character_sprites/src/anim/mod.rs`. It already distinguishes
tech, tech roll and getup roll. CLIP1 fixes their clock and occurrence;
CLIP2 keeps wall and ceiling tech distinct.

Smash seats use the shared `BodyPoseView`, including upper-body clips over
locomotion legs. Ordinary campaign `ActorAnimFrame` requests still lack that
leg request. Treat this as a bounded campaign presentation follow-up, not a
human-versus-CPU Smash split. Keep one semantic rule through both readers.

`crates/ambition_combat/src/hitbox/mod.rs:36-46` defines `LandedBodyHit` with a world contact point, but that message means geometric overlap. `ResolvedBodyHit` (from line 67) means the hit was accepted. The localized-impact cue must come from the accepted hit, with stable replay and rollback behavior. Keep gameplay outcomes authoritative.

The new clips can first run through the **existing sampled sheet and part-flipbook playback**. Do not require a runtime `PartPose` provider to show the twelve new silhouettes. This keeps the art and the new selection logic testable before continuous visual-joint evaluation exists.

### Phase 2 exit gate

- Each new pose can be triggered by an ordinary game event or a deterministic presentation fixture. No renderer-only diagnostic input may be required in shipped play.
- Human-driven and CPU-driven bodies show the same pose for the same event and rig.
- A trace names the requested clip and the selected published clip. Missing clips fall back by documented policy, not silently to unrelated gestures.
- Recorded Smash matches show the full sequences: high/low hit; up/down launch; launch to prone (orientation held through getup); teeter enter/hold/leave; floor tech; run and stop.
- Visual-only changes do not alter collision, hitboxes, damage, move windows, or rollback results. Replays and camera captures can reproduce the same motion choice, including prone orientation.

## Phase 3 - Improve continuity and body weight (polish)

### CLIP1 — Carry the beat through the existing clip request

**State:** PLANNED from source `34b4ab0b7f4f`, 2026-11-22.
Renderer source: `2b52adb65a3235c6b1f4f9fc4954ee31cc7b2bb8`.
The archive has no generated visual payload. These are source findings,
not a review of the currently published pictures.

**Shared first customers:** Scout's landing and repeated recoil in Ambition;
the Admiral's recovery and tumble in Smash. The previous interpolation,
carriage blend, planted-hand and follow-through work remains implemented.

A row name does not identify the event that selected it, the event's duration
or the row's end rule. Preserve those facts through `ClipRequest`.
Use the existing `Phase` and `Seconds` request paths. Add occurrence and
end-policy facts where the current request loses them; do not add a second
animation graph.

| Source | Current result |
| --- | --- |
| [`ClipRequest::from_chain`](../../../crates/ambition_sim_view/src/anim_index.rs), [pose producer](../../../crates/ambition_sim_view/src/pose_view.rs) | Non-move reactions carry no authoritative clock or occurrence; move clips already carry phase |
| [`CharacterAnimator::request_clip`](../../../crates/ambition_sprite_sheet/src/character/animator.rs) | Selecting the same resolved row preserves its cursor, even for a new hit or launch |
| [`ClipRequest::ahead_of`, `with_pin`](../../../crates/ambition_sim_view/src/anim_index.rs) | Charge fallback discards the base time/mask; pin handling repairs base time but can apply the pin's mask when its row is absent |
| [Landing overlay](../../../crates/ambition_platformer2d_actor_monolith/src/body/movement_fx.rs), [Scout motion source](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/player_robot_v3_motion.py) | Soft/hard landing holds last 160/340 ms; the selected authored clips last 492/760 ms |
| [Getup facts](../../../crates/ambition_platformer2d_core/src/movement/facts.rs), [Admiral motion source](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_pirate_admiral_fighter.py) | The 300 ms invulnerability selects recovery art; authored getup/getup roll last 504/406 ms |
| [Sheet row](../../../crates/ambition_sprite_sheet/src/lib.rs), [prepared part clip](../../../crates/ambition_sprite_sheet/src/character/rigged.rs), [part publisher](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/part_flipbook.py) | Sheet/part clip metadata drops authored looping, although the semantic rig can retain it |
| [Rig presentation](../../../crates/ambition_render/src/rendering/actors/rigged.rs), [animation adapter](../../../crates/ambition_render/src/rendering/actors/animation.rs) | Blend/follow-through use raw render delta; the animation uses entity presentation time |

The landing durations do not prove that gameplay holds should grow. The
visual request currently ends before its clip reaches its authored exit.
Map the intended contact/settling beat to the clip, or deliberately revise the
art. Art duration must not determine collision, landing lag or input permission.

**Implementation:**

1. The domain that accepts the reaction, landing or maneuver supplies its
   occurrence and elapsed time or phase. Restart once for a new occurrence
   even when it resolves to the same row. Preserve phase for the same
   occurrence and semantic beat. Launch can become tumble, and guard break
   has several beats; each needs its declared local origin or phase.
   Resolve a complete candidate: row preference chain, clock, occurrence/
   beat, end/transition rule and channel mask. The current bounded stack is
   pin, charge and base action/reaction. A missing upper candidate leaves
   the lower candidate's entire payload intact. An unchanged selected pin
   must not restart because the base reaction changed. Do not infer a new
   event from a timer increase or restart on every render request.
2. Keep the event's lifetime separate from defense. For tech/getup, consume
   [ACT3's recovery episode](../engine/body-action-execution.md#act3--give-floor-recovery-its-own-episode).
   A shorter invulnerability window must not end the recovery picture.
   Scout's ordinary visual landing can use its existing contact beat; it
   does not need to wait for ACT3.
3. Preserve the authored loop/one-shot rule in the existing published and
   prepared row metadata. The Admiral's tumble explicitly loops; Polygon
   tumble explicitly does not. A global rule for a row named `tumble`
   would override one of those sources. Replace name-based `row_loops`
   inference where explicit metadata now owns the answer. Keep a documented
   fallback for a source that declares no rule, and migrate touched sources.
   An owner-supplied phase or seconds request remains authoritative. Authored
   looping is the sheet-clock default, not permission to wrap a phase-driven
   attack.
4. State the visual transition policy by semantic beat, independent of whether
   the caller used a `CharacterAnim` enum or an explicit row. Keep immediate
   impact/contact, an authored settling interval, loop continuation, a new-hit
   restart and an interruptible carriage blend as distinct operations.
   A named recoil must not bypass the chosen policy merely because it is named.
5. Advance pose modifiers on the pose's declared clock. Consume
   [ACT2's local hold](../engine/body-action-execution.md#act2--resolve-body-action-time-once-per-step)
   for an action-linked reaction; use entity presentation time for ordinary
   carriage, and wall time only for a deliberately independent visual effect
   or preview. Sample accumulated owner time; never add a fixed simulation
   step once per rendered frame. Do not let a paused clip finish a live 70 ms blend or 180 ms
   follow-through through raw render delta.
6. Preserve the selected content generation and existing part-first pipeline.
   Prepare bindings and end policy once. Do not parse a rig, allocate a clip
   program or scan all rows per body and frame.

An appearance may start from the last compatible drawn pose. That does not
authorize a blanket crossfade through a hit, attack-active window, dodge or
death. Existing raster/scale-sign tween limits remain. A decorative tumble
does not rotate the physical body. Simulation geometry continues to use its
own deterministic pose/landmark authority.

**Remove:** row-name-as-event identity, defense-timer-as-recovery duration,
duplicated end-rule inference, and raw-delta advancement for pose-linked
modifiers. Keep the existing clip selector, animator, limited blends and
prepared assets.

This is more than adding fields to the current flat row chain. `ahead_of`
currently clears `time`, `row_clock` and `legs`. The charge request uses it,
so a missing charge row loses the held move's phase and advances on sheet
time before returning to move phase at release. `with_pin` separately
restores base time but assigns a mask before knowing which row won. Replace
these exceptional metadata repairs with candidate selection. Two candidates
may resolve to the same row and still have different owners; aliases within
one candidate must not change its owner. Remove the single exceptional
`row_clock` slot when the complete candidate makes it redundant. Bind
candidates against prepared content; do not allocate a new graph per frame.

**Proof:** a real repeated same-region hit must restart once; a soft/hard
landing must reach its intended exit without delaying control; the Admiral's
loop and Polygon's one-shot must keep their distinct source rules. During one
pause/slow interval, the pose and its modifiers must follow the declared
clock. Observe the current game's selected bundle at game scale.
`smash_reaction_poses` is still a useful row-routing test; its helper manually
cancels moves, so use ACT1's real-hit crossing to prove the reaction itself.
Use a held charge with no charge row, then an optional missing pin above it,
to check that the base phase and mask survive both fallbacks. A resolving
pin is the positive control. Require one continuous release, not a cursor
reset inferred only from row equality.

### CLIP2 — Bind the art the author already supplied

**State:** PLANNED, same source review. This is a small companion to CLIP1.

Two inspected routes lose an authored distinction:

- Full chargen publication includes `land`, but the ordinary soft landing
  requests `LandRecovery`, whose enum binding does not recognize `land`
  and can fall back to idle. Odile and Coulomba select the full row set.
  Source: [chargen sheets](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/chargen/sheets.py),
  [animation names](../../../crates/ambition_sprite_sheet/src/character/anim.rs)
  and [character catalog](../../../game/ambition_content/assets/data/character_catalog.ron).
- `body_state_clip` maps both `WallTech` and `CeilingTech` to
  `wall_tech`. The three Polygon motion libraries have distinct keyed
  wall and ceiling clips; George authors both too. Source:
  [selector](../../../crates/ambition_character_sprites/src/anim/mod.rs),
  [Polygon motion](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid)
  and [George motion](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_george_booul_fighter.py).

Decide the intended contact versus recovery meaning, then bind the existing
landing row through the current alias/request mechanism where those meanings
agree. Request ceiling tech separately, with an explicit fallback for a
family that has no such art. Preserve the shipped `hit`/`hurt` and
`idle`/`rest` aliases. Do not require every rig to publish the same rows.

Use one ordinary campaign landing and one Polygon ceiling tech to prove the
selector reaches the existing art. Extend the current clip trace to retain
requested meaning, selected row, occurrence, clock and frame when needed.
An inventory that counts file names cannot prove this binding.

### CLIP3 — Preserve source motion under ruleset retiming

**State:** PLANNED from the same source review. This is shared preparation
and playback work. It does not wait for ACT3 or a cast-wide art pass.

The current retimer changes gameplay windows by a piecewise map, but the
clip still receives `selected_time / selected_pictured_duration`. Those
operations preserve different beats. The Admiral supplies a concrete case:

| Forward smash | First active | Last active | Total |
| --- | --- | --- | --- |
| Source move and motion | 0.34 s | 0.42 s | 0.80 s |
| Smash timing facet | 0.30 s | 0.35 s | 0.90 s |

At the selected first active instant, the current clip phase is 0.3333.
The corresponding source phase is 0.425. The former samples source time
about 0.2667 s instead of 0.34 s. This proves a coordinate disagreement.
The source art leads the strike slightly and samples frame midpoints;
this audit has not measured its visible pixel error in the game.

Sources: [`move_timing_over`](../../../crates/ambition_characters/src/move_damage.rs),
[`scale_moveset`](../../../crates/ambition_characters/src/prepared.rs),
[`MoveSpec::phase_at`](../../../crates/ambition_entity_catalog/src/lib.rs),
[Admiral move](../../../game/ambition_content/assets/data/movesets/pirate_admiral.ron),
[Smash timing](../../../game/ambition_content/assets/data/fighters/npc_pirate_admiral.ron),
and [authored motion](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/_pirate_admiral_moves.py).
The existing retime test checks windows/events and lag, not the motion sample.

**Implementation:** retain a compact monotone map between selected action
time and source motion time in the immutable prepared move. The current
retimer needs only its few segment boundaries. At selected time `t`, sample
the source clip through the inverse map and the source's pictured end.
Keep the direct identity path when no retime applies. Preparation owns this
map; render code must not reconstruct it from active-window names.

Use exactly one coordinate conversion for each consumer:

| Consumer | Coordinate and conversion |
| --- | --- |
| Playback, active/cancel windows and event firing | Selected gameplay seconds; already retimed, so do not inverse-map them again |
| Source-authored move hurt keys | Inverse-map lookup to source seconds, or retime the prepared keys once; choose one |
| Raster/part clip, admitted semantic rig and baked landmarks | Source clip phase, including the source `Settled` boundary |
| Source sheet hit-area track | Convert its declared sampling instant through the source map; if its API takes row seconds, use phase times that row's duration |
| Explicit charge hold instant, repeat endpoints and absolute window/event instants | Apply the forward map at preparation |
| Charge budget, loop duration, flow timeout and external lease | Preserve the declared owner's duration, or explicitly specialize it; these are not instants on the move timeline |

The explicit `hold_at_s` is currently not retimed. A source hurt profile
also reads playback time directly. The Admiral has no authored hurt profile,
so the latter is a supported-composition gap, not an Admiral hurtbox finding.
The sheet hit-area API expects row seconds while its caller supplies a move
window instant. Source comments claiming every character uses coarse
per-row tracks are stale: the Performer publishes explicit per-frame
swing samples. The [empty-geometry audit](../engine/combat-model.md#preserve-an-authored-empty-geometry-sample)
traces its borrowed second jab into an authored empty frame that becomes
a coarse fallback. That is a separate result-semantics defect, not proof
of an observed retiming error. Frame-duration rounding is also enough
reason not to assume row duration equals source move duration.
Preserve the sampling cut: current strike geometry is sampled at window
entry and retained for that window. Convert that selected entry instant;
do not silently replace the shape with current rendered-frame geometry.
See [hurt selection](../../../crates/ambition_combat/src/hurtbox_resolution.rs),
[landmarks](../../../crates/ambition_combat/src/body_landmarks.rs),
[rig sampling](../../../crates/ambition_combat/src/body_rig.rs) and
[sheet hit-area selection](../../../crates/ambition_character_sprites/src/attack_hitbox.rs).

Keep physical geometry bound to its admitted source program. A visual
fallback must not choose a different damage program or make simulation
depend on a loaded sprite. Decorative motion and physical pose can project
the same semantic beat differently under explicit bindings. All gameplay
sampling remains available headlessly.

Preserve repeats, charge and settlement. A repeat returns through the same
map; a charge hold keeps the same source pose while its charge budget runs.
`Settled` can release the picture before the action ends. A fallback can use
the same source phase or an explicit prepared beat binding; do not infer a
new map from row spelling at runtime.

Validate the prepared product. Find active bounds by temporal extrema, or
validate an explicit chronological-order contract; current vector order is
not proof. Preserve authored volume priority when doing so. Collapsed active
or tail intervals have no unique inverse: refuse a collapse that must
preserve a live source span, or state its boundary selection explicitly.
Do not divide by zero or promise to preserve a beat with no selected time.
Do not recursively retime every field whose name ends in `_s`.

**Remove:** total-duration normalization as the source sample for a
nonuniform retime, repeated per-consumer clock guesses, and comments in the
retimer/inspector that claim this already preserves the drawn swing.
Keep the completed facet/preparation owner, authored geometry and phase
driven presentation. No new animation graph or runtime retiming interpreter
is required.

**Proof:** extend the existing unequal-segment retime test. Source phase
must agree at mapped contact boundaries, an explicit charge hold and a
source profile key. Check the identity path and a repeat/settled boundary
where touched. In the existing inspector, review the Admiral's first active,
last active and first actionable steps under source and Smash timing using
the actual prepared sample. This validates phase correspondence; controller
and visual review still decide whether the authored anticipation and recovery
feel good.

### Review through the actual request

Use the current `AMBITION_CLIP_TRACE`, `reaction_review`,
`clip_alias_audit`, moveset inspector and game captures. Do not add another
visual test service. `pose_transitions.py` compares selected source row
pairs and their last frames; the game may interrupt earlier or choose another
row. Feed a production request/exit into that review when testing a transition.
`campaign_sheet_gaps.py` checks reaction chains; it does not prove ordinary
landing enum binding.

Keep authoring work in the sprite renderer submodule, then update its pin
through the normal policy. Do not repair a generated JSON file that a Python
motion source will overwrite. Existing
[SA1/SA2](../engine/asset-authoring-and-prepared-media.md) owns source/edit
authority and motion-to-rig units. Existing E2 owns prepared rig/binding reuse.
Those are valid separate tasks, not measured causes of the reported jaggedness.

Start with the two shared customers above. A cast-wide choreography pass
follows the shared fixes and remains below the main Ambition work.

### Existing continuity work and receipts

Only after the silhouettes appear in the game, improve how one pose becomes another:

1. Blend compatible clip changes at joint level while preserving hit-contact timing and clear attack silhouettes.
2. Keep a planted foot stable during stops, teeters, and landing. Keep hands visually attached on a ledge catch and hang. (The generator's foot IK already plants feet within each authored clip; this item is about the transitions between clips.)
3. Add bounded follow-through to arms and head after hits, braking, and landings. Do not smooth away a sharp impact or active attack pose.
4. Bring upper/lower body composition, facing, interruption, and rollback-reset policy into one shared presentation path.
5. Evaluate part merging, joint binding, quality tiers, portal rendering, and off-cell bounds before enabling arbitrary procedural joint offsets.

**Tween policy:** the part publisher honors a target's declared
`tween_rows`; selected locomotion loops are its fallback. Other rows can
already opt in. Keep raster and transform compatibility checks. The animator
resets on a resolved row change; CLIP1 adds the missing occurrence/beat rule.
Do not turn all clips into tweens to repair that missing rule.

**Known runtime pose limit:** `crates/ambition_sprite_sheet/src/character/rigged/posed.rs:77-182` can place parts from joints, but binds parts by matching sampled animation frames within a tolerance. `crates/ambition_render/src/rendering/actors/rigged.rs:1019-1027` needs a baked body rig for that path. The audited Polygon publishers call `build_sheet()` and `publish_rig_flipbook()`, but do not publish that body-rig product. Part optimization can also merge always-rigid neighboring tracks (`part_flipbook.py:1675+`). If Phase 3 needs independent runtime joint control, first publish a suitable **visual** rig and stable semantic part bindings. Do not enable experimental gameplay-rig hurt geometry as a side effect. Do not front-load this bridge into Phase 1 or 2.

**Measured 2026-11-14 on the reference fighter** (`player_robot_v3`, Scout:
150 clips, 33 body tracks; `scripts/measure/pose_transitions.py`). The
animator resets to the new clip's frame 0, so a transition draws the jump
from the old clip's frame to the new clip's first. Of 34 gameplay
transitions, 6 jump at their natural end by more than twice the larger
in-clip step, and 7 more only when the old clip is interrupted:

| transition | end jump (px) | in-clip step A / B (px) | verdict |
|---|---:|---:|---|
| fall -> land | 41.7 | 3.8 / 9.4 | pop |
| fall -> land_hard | 43.9 | 3.8 / 11.8 | pop |
| hit -> fall | 34.0 | 9.6 / 3.8 | pop |
| fall -> ledge_catch | 32.3 | 3.8 / 2.3 | pop |
| ledge_climb -> idle | 33.3 | 4.6 / 12.0 | pop |
| fall -> wall_slide | 14.1 | 3.8 / 0.9 | pop |
| dash_attack -> idle (interrupted) | 52.2 max | 18.6 / 12.0 | interrupt-pop |
| jump -> fall (interrupted) | 43.3 max | 20.4 / 3.8 | interrupt-pop |
| teeter -> idle (interrupted) | 42.9 max | 16.2 / 12.0 | interrupt-pop |

The ordinary ground loop is continuous: idle, walk, run, crouch, talk and
shield changes stay within the clips' own motion. What pops is the change
from the airborne `fall` pose, whose frames barely move, into a contact
pose. Two of the six are contacts the art should keep sharp (`land`,
`land_hard`: the impact is the point); `hit -> fall`, `fall -> ledge_catch`,
`fall -> wall_slide` and `ledge_climb -> idle` are the candidates for a
short blend on the shared path, and the interrupt pops are what a blend
from the current frame (not from frame 0) would remove.

**The clip blend landed (2026-11-14)** on the shared rigged path
(`drive_rigged_presentations`, so human and CPU bodies alike): when a body's
authored row changes (facing alone is not a change) into one of its carriage
poses (`CharacterAnim::blends_in`: idle, walk, run, jump, fall, fly, glide,
ledge grab/climb/getup, wall grab, crouch, crawl, ladder, swim, hit), each
part of the new pose starts where its track was last drawn and slides to its
place over `CLIP_BLEND_SECONDS` (0.07 s, smoothstep), keeping the new pose's
picture (`rigged::blend_from`). It blends from the pose last DRAWN, so an
interrupted blend goes on from where it is. A move clip and the poses that
read by being sharp (slash and the attacks, land hard, blink, death, dodge)
step as before; a body out of view starts clean when it is seen. Witnesses:
`a_clip_blend_slides_the_new_poses_parts_from_where_the_old_pose_drew_them`,
`a_change_between_carriage_poses_blends_and_a_strike_does_not`. Planted contacts (item 2) and follow-through (item 3)
landed in the November 18 work below.

**Contacts measured (2026-11-14, Scout's published parts, last frame of A
to the first of B).** Where a clip ends naturally, the feet are already
planted: `land -> idle` and `land_hard -> idle` move neither foot (0 px),
`teeter <-> idle` about 2 px, and `walk -> idle`'s far foot 0.5 px (its near
foot is mid-step, 10 px off the ground, and comes down). `run`'s last frame
has both feet up, so a stop lands them. A foot glides only when a clip is
interrupted with that foot down. The hands of a ledge catch slide 24 to 31
px from the fall's raised hands (`fall -> ledge_catch`, `fall ->
ledge_grab`) during the 0.07 s blend.

**Visual rig prerequisite: complete for Scout.** The November 18 rig,
part bindings, planted hands and follow-through below supersede the earlier
instruction to wait for a rig. Do not repeat that work for CLIP1/CLIP2.

**The visual rig and its part bindings (2026-11-18).** Scout publishes a
body rig (`_player_robot_v3_body_rig.py`): the fifteen bones of their side
rig document as world frames from the feet (no parents: the torso and head
squash and the head flips on the part alone, which a parent-local frame
could only state with a shear), for their side rows and their mirrors. Left
out: the back rows, the turns (front and back frames inside a side row) and
the pure side rows, whose rig hangs the shared limb rasters a few degrees
and about a pixel apart. The rig names each part track's joint
(`part_bindings`, read by `published_part_bindings`; the definition a body
carries does not hold it), because guessing the joint from the frames
fails on what this flipbook does:

- one track draws many rasters (each subpixel phase of a part is its own),
  and a mirrored row draws the same raster flipped under a mirrored joint
  frame, whose angle reads with the opposite sign. So a part's place in its
  joint's frame is kept per raster, per handedness of the joint frame and
  per flip of the draw;
- the flipbook draws one raster for every raster that is a lossless
  transform of it: Scout's oval visor and capsule shins come back drawn half
  a turn about another pivot. A named part is rigid on its joint, so a
  second place of one raster can only be such a transform: each place must
  be tight, and the binding is the one most frames drew.

Measured on the published files: every named draw of the 2028 shared frames
is placed from the rig's pose, 41707 exactly and 35 half a turn away on the
same raster (`scouts_named_parts_are_placed_by_his_body_rigs_pose`). Mary-O
and the pirates name no joints and bind as before.
`scripts/measure/measure_track_joints.py` still measures one place per
track, so it reports Scout's limbs as loose.

**Planted hands (2026-11-18), item 2.** A pose names the limb ends it
plants on the world (`CharacterAnim::planted_contacts`, body-rig
attachment names); the ledge grab plants both hands. The rig states each
limb's upper, lower and end joints (`limbs`, read by `published_limbs`).
While a change into a planting pose blends, `PosedParts::hold` puts each
planted end's parts where the pose draws them and solves its upper and
lower bones between the root (a shoulder), where the blend now draws it, and
that end, bent to the side the pose bends them. So Scout's hands are on the
ledge from the first frame of a catch while his body comes up to it
(`a_ledge_catch_holds_scouts_hands_on_the_ledge_through_the_blend`: both
hands within a pixel of the ledge grab's at every point of the blend, each
arm one piece, both facings; unheld, the blend moved them more than 10 px).
Feet were not planted: at a natural clip end they are already down, and an
interrupted step is a later question.

**Follow-through (2026-11-18), item 3.** A pose names what it sets
swinging when the body comes into it (`CharacterAnim::follow_through`): a
hard landing (arms 14 degrees down and forward, a 6 degree nod; a pure
"down" turned Scout's two hanging arms apart, one each way), a soft landing
(9 and 4), a brake (`Skid`: arms 9 forward, nod 5) and a blow (`Hit`: arms 12
forward, the head snapping 8 back). The rigged presentation keeps a clock
per body (`FOLLOW_THROUGH_SECONDS`, 0.18 s) and draws one arc out and back,
`sin(pi t / T)`: it starts at nothing, so the impact frame is drawn as
authored, and any clip or strike drops it at once; it carries on through
the carriage poses after the landing. `PosedParts::follow_through` turns
each arm rigidly about its shoulder so the hand goes the swing's way, the
forearm lagging 0.6 as far again about the elbow, and the head about its
joint (the rig's `head` attachment); the torso and the legs stay as drawn.
Witness: `a_landing_swings_scouts_arms_down_and_leaves_the_body_as_drawn`.
The amplitudes are first values, to be judged by watching the game at 60
and 144 Hz.

## Phase 4 - Expand expressive coverage after the first playable pass

Review the rest of Melee's motion vocabulary and Ambition's `fighter_motion_vocabulary.yaml` by physical event family, not by raw state count. Add new bases only when the current rig cannot make the event legible. Give different fighters their own stance and recoil style after the shared pose language works. The original base-art pass used Smash first. Campaign rollout below and the shared CLIP1/CLIP2 consumers now use the same visual system; no broad campaign conversion is required for those two slices.

### Roster rollout (Jon, 2026-10-31: "everyone really")

Every character gets the reaction vocabulary and the turn. Each art family takes it once, where its poses are made:

| Family | Where the poses live | State |
| --- | --- | --- |
| Polygon fighters | `devtools/reaction_keys.py` | Done (Phase 1). |
| Director, Officer, Medic, Performer (the Polygons' rig profile, `humanoid-articulated-v1`) | `devtools/reaction_keys.py` (`FIGHTERS`) | Done: the same keys, solved on each rig. |
| Character generator: Alice v2, the townsfolk, every later generated character | `chargen/motion.py` `build_library` (the twelve anchors as `Kit` poses); `chargen/sheets.py` `COMPOSED` (the turns, made from the views) | Anchors and turns in the library. Alice publishes them (`_alice_v2_motion.FIGHTER_ROWS`, `TURN_ROWS`). |
| Player robot v3 | `player_robot_v3_reactions.py` (its own rig) | Done, with its own turnaround. |
| The rest of the Smash roster: Bob, PCA, Ramen-nujan, Shadow Oni Leader, Goblin (and skins), GNU-ton, Patent Clerk, Carl Stargan, Noether, Mary-O (tall), Sanic, Pirate Admiral, Oiler, George Booul | Each rig's own motion module | Done 2026-10-31: every Smash fighter's published sheet has the twelve anchors, the floor game and the turn (MEASURED by sheet rows). The small sheets also gained the whole `fighter_core` vocabulary. |
| Campaign-only characters | Each rig's own motion module | Gaps measured 2026-11-19 (`scripts/measure/campaign_sheet_gaps.py`, below). The 240 default-row townsfolk publish the reaction anchors (2026-11-19); the hand-made rigs do not yet. |

**Campaign gaps, measured 2026-11-19** (`python3 scripts/measure/campaign_sheet_gaps.py`):
the 28 preference chains of `body_state_clip` against the newest published
record of each catalog sheet that no Smash fighter draws: 510 sheets, all
published here.
- No campaign sheet draws every first choice. Each state's own pose is
  missing from 497 to 510 of them (`pet` from all 510, `tumble` from 497).
- Where the chain landed was the finding: 329 sheets had no row named `hit`,
  so a struck, launched, grabbed or knocked-down body on them was drawn in
  `idle`. All 244 character-generator townsfolk were among them.
- **Fixed the same day, in the binding.** Most of those sheets draw the
  pose: they spell it `hurt`, which the sheet parser has always read as the
  `hit` pose (`CharacterAnim::from_name`). A chain bound row names exactly,
  so it stepped past the pose to `idle`. `SheetRecord::first_bound_row` now
  binds a name to the sheet's other spelling of the same pose (a boss's
  `rest` is its `idle` the same way). Remeasured with the binding modelled:
  492 of the 504 sheets draw `hit`/`hurt` for a struck body, and 12 still
  fall to `idle`.
- Nine sheets have no `idle` row: boss sheets drawn by their conductors
  (`rest` rows: the bossofthegame, Galwah, the gradient sentinel, the
  Mockingbird, the Smirking Behemoth, the three GNU-ton sheets and GNU2).
  They now bind `rest` for `idle`.

- **The townsfolk publish the poses (2026-11-19).** `TOWNSFOLK_ROWS`
  (renderer `chargen/sheets.py`) gained the motion library's reaction
  anchors: `hit_high`, `hit_mid`, `hit_low`, `hit_air`, the three launches,
  `tumble`, both prone faces, `getup` and `grabbed`. The 240 residents on the
  default rows republished part-first (the four on `full` rows did not
  change). Remeasured: each of those first choices is now missing from 264
  sheets, not about 504, and `tech`, `tech_roll` and `getup_roll` land on
  `getup` on 240 sheets instead of `idle`. On the way, 19 sheets refused at
  the replay audit on two resampling differences, and its tolerance moved
  from 64 to 96 levels (`CONTINUOUS_REPLAY_TOLERANCE` in
  `authoring/part_flipbook.py`, with the measurement).

Left: the hand-made rigs' state poses (each its own motion module), the four
`full`-row residents, and the poses no campaign sheet draws (`pet`, the
shield break, the dodges, `grab_hold`).

Both `BodyPoseView` and ordinary actor animation carry the turn clock. The
turn is drawn in the new facing, so its row must end on that side view.

## Risks and constraints

| Risk | Response | Blocks |
| --- | --- | --- |
| New clips are only renamed copies | Require new joint silhouettes and side-by-side art review; re-run the alias-audit script. | Phase 1 acceptance |
| Reaction art coupled to attack timing or hitboxes | Sibling generator on a shared rig/solver core; no `SMASH_CLOCKS` or swing specs in reaction authoring. | Phase 1 structure |
| 36 targets authored before any is seen at game scale | Reference fighter first, launch family first, narrow smoke test, sheet-budget record. | Scaling to three fighters |
| Published assets clip or change gameplay geometry | Validate part extents, mirrored facing, attachment positions, and sheet publication. Keep visual art separate from hit and hurt volumes. | Phase 1/2 acceptance |
| Hovering prone or braced bodies, baked shadows | Floor-contact and no-shadow checks in every art gate. | Phase 1 acceptance |
| Current game cannot name a hit region, tech result, or prone orientation | Add the smallest resolved presentation fact in Phase 2, from the owning subsystem; hurt volumes are unlabeled and may be portal-cut, so classify accepted contact or carry semantic metadata. | Phase 2, not Phase 1 |
| Runtime visual joint rig is absent on Polygon | Use existing sampled clips first; defer arbitrary live articulation. | Phase 3 only |
| Sprite authoring lives in a submodule | Make art changes in `tools/ambition_sprite2d_renderer` at its own source authority, run its own test suite, then update the superproject pin according to repository policy. | Packaging and integration, not art design |
| New pose looks good in the editor but unreadable in play | Review `high`/`ultra` contact sheets, then compare at the 128x192 authoring frame size and at actual Smash camera distance; review timed gameplay clips. | Every visual acceptance gate |
| Early polish hides missing art | Do not substitute cross-fades, rotation of one generic hit pose, or procedural wobble for a missing silhouette. | Phase 1 |

## Source index and reproducible checks

**Melee reference:**

- `/home/joncrall/code/melee/src/melee/ft/kinds/ftCommon/forward.h` - common motion-state vocabulary.
- `/home/joncrall/code/melee/src/melee/ft/ftmotionstates.c` - shared state table.
- `/home/joncrall/code/melee/src/melee/ft/kinds/ftCommon/ftCo_Damage.c:61-74` (`ftCo_803C5520`) and `:318-362` - damage-motion selection by ground/air, knockback level, and damaged hurtbox.

**Ambition source and owner docs:**

- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/devtools/polygon_keys.py` - the existing world-space pose generator (attack rows): `Key`, `sample`, eases, `Body`, foot IK, clip writer.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/fighter_motion_vocabulary.yaml` - long-term motion names and baseline-slot policy.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid/{fighting_polygon_v1,fighting_brawler_v1,projectile_beast_v1}/` - measured pose and clip source.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/{pointed_polygon,pugnacious_polygon,projectile_polygon}/` - rig bindings.
- `tools/ambition_sprite2d_renderer/docs/authoring/motion_ir_v1.md` - pose/clip contract, schema checks, and the Godot pose editor workflow.
- `tools/ambition_sprite2d_renderer/tests/test_motion_ir.py`, `tests/test_pointed_polygon_target.py`, `tests/test_pugnacious_polygon_target.py`, `tests/test_projectile_polygon_target.py` - focused authoring validation.
- `crates/ambition_character_sprites/src/anim/mod.rs` - current semantic selection; CLIP1/CLIP2 own the request and binding gaps.
- `crates/ambition_platformer2d_core/src/movement/facts.rs` - authoritative movement facts, including `teetering` (and `skidding`, which is false for `AxisSwept` walkers).
- `crates/ambition_platformer2d_core/src/movement/knockdown.rs` - prone orientation and floor-recovery authority.
- `crates/ambition_combat/src/hitbox/mod.rs` - difference between overlap and accepted hit; `victim_pieces`.
- `crates/ambition_sim_view/src/pose_view.rs` (`BodyPoseView`: `vel`, `turn_secs`, `stop_secs`) and `anim_index.rs` - player and actor presentation readers.
- `docs/planning/engine/runtime-rigged-sprite-animation.md` and `semantic-part-rendering-and-ragdolls.md` - current art/render contracts and future rig work.

**Checks:**

- **Clip-alias audit:** use the committed `devtools/clip_alias_audit.py`.
  It compares ordered keyed states and reports timing differences. It does
  not prove visual quality or runtime selection.
- **Source validation:** `motion_ir_v1.md` documents `motion_ir_tool validate`, the generated Godot pose-sheet workflow, and its headless export round-trip check.
- **Publication:** the existing renderer, target tests, and the renderer submodule's own suite. Do not require a new art editor or a new animation schema.

## Definition of complete

This work is complete when a player can watch a normal Smash Siblings match and distinguish at least the following without reading UI or debug text: where a hit landed, which way a body was launched, which way a knocked-down body faces, whether a body is bracing for a floor tech, when a fighter is about to lose balance, and when a run ends in a hard stop.

The character may still have stepped frame transitions at that point. It should already **look like it is responding to the physical rules of a platform fighter**, not cycling through a small set of generic poses. Smoothness and secondary motion then make that visual language more natural.
