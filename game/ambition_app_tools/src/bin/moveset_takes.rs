//! Record what the REAL simulation does when a fighter throws each of its moves.
//!
//! ⭐⭐ THIS IS THE HALF OF THE INSPECTOR THAT CANNOT BE FAKED. Jon, 2026-08-27:
//! *"This should let us 'prove' that up-b works because to build this we run the
//! characters in the real engine and use control frames to show how the game
//! reacts to their inputs and we will see things like the pirate flying around
//! on the shark."* A frame-data table reports what a move DECLARES; a take
//! reports what the engine DID with it — where the body went, which hitboxes
//! were live, what the move spawned, and whether the fighter ended up riding it.
//!
//! ⛔ ONE APP, ONE PROCESS, for the reason `shark_ride_probe` writes down: the
//! tracing subscriber is process-global, so a tool that builds several Apps
//! cannot keep the log. This builds one and seats every take in it.
//!
//! ⛔ A MOVE THAT DOES NOT COME OUT IS STILL RECORDED. A take whose `move` field
//! stays empty is the honest report that the press did not reach the move — a
//! posture gate, a spent recovery, a shield. Dropping those would make the
//! inspector show only the moves that already work.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

/// Ticks recorded per take. Long enough for a five-second shark ride to show its
/// shape without every take carrying the tail of an idle stage.
const TAKE_TICKS: usize = 150;
use ambition_sim_harness::combat_observation::{CombatObservation, ScenarioRoles};
use ambition_sim_harness::move_exercise;
use move_exercise::{settle, step, VERBS};

/// Everything one recorded tick says.
#[derive(Default)]
struct Frame {
    bodies: Vec<serde_json::Value>,
    hitboxes: Vec<serde_json::Value>,
    projectiles: Vec<serde_json::Value>,
    /// What connected with what, as the RUNTIME says — never as geometry
    /// suggests. See `CombatObservation::contacts`.
    contacts: Vec<serde_json::Value>,
    move_id: Option<String>,
    grounded: Option<bool>,
    subject_pos: Option<(f32, f32)>,
    subject_vel: Option<(f32, f32)>,
    riding: Option<String>,
    /// Which way the body is pointing. A directional press is resolved against
    /// this, so a take that came out forward when back was driven is only
    /// readable with it on the recording.
    facing: Option<f32>,
    /// The gesture the engine resolved from the press, e.g. `Back/Tilt/Airborne`.
    gesture: Option<String>,
    /// Recorded bodies that carry NO `SimId`, by the label a reader would see.
    ///
    /// ⛔⛔ THE TAKE'S ORDERING AND JOIN CONTRACT ARE BUILT ON `SimId`, and a
    /// body without one used to be written as `"id": null` and sorted under the
    /// empty string — canonical-looking output whose ordering is query order.
    /// ⇒ counted here, refused at the take.
    unidentified: Vec<String>,
}

const USAGE: &str = "\
moveset_takes — drive real control frames through the engine and record what it did.

USAGE:
    moveset_takes [--characters ID,ID] [--target ID] [--target-behavior WHICH]
                  [--out PATH]

OPTIONS:
    --characters ID,ID   comma-separated catalog ids to record
                         `grid` (or `all`) records every fighter on the smash
                         grid: 21 fighters at ~1m17 each is about 27 MINUTES
                         (measured 2026-08-27, after the settle stopped
                         serialising a frame to read three booleans)
                         [default: npc_pirate_admiral]
    --verbs V,V          record only these repertoire verbs
                         [default: every verb the exercise can drive]
                         ⭐ ONE MOVE IS ~4s; a fighter's whole moveset is ~1m17.
                         Re-recording the move you are looking at should not
                         cost the other eighteen.
    --spacing PX         walk the subject to within PX of the target before the
                         press [default: the match's own seat placement]
                         ⭐ A move recorded from across the stage can never show
                         a CONTACT. This is how reach is asked about.
    --target ID          who the subject performs the move AGAINST
                         [default: the subject's own character, a mirror match]
    --target-behavior WHICH
                         passive | cpu                        [default: passive]
                         `passive` seats the target on the stand-still brain: a
                         real, damageable, seated fighter that makes no
                         decisions, so what the recording shows is the MOVE and
                         not an opponent's reaction to it. `cpu` restores the
                         live duelist brain, which is a different measurement
                         and says so in the take.
    --out PATH           where to write the takes
                         [default: tools/ambition_moveset_inspector/data/takes/takes.json]
    -h, --help           print this and exit

NOTES:
    Seats a real smash match and presses every verb, recording bodies, live
    hitboxes, projectiles and which move the engine actually PLAYED. A take that
    reports `MISMATCH` means the press and the move disagreed, which is the whole
    reason this exists.

    There is no positional argument; use --out.

    ~1m17 per character, measured 2026-08-27. It was 7m08 until `settle`
    stopped calling the full sampler — which built a JSON frame, and rebuilt the
    catalog join, up to 480 times a take to read three booleans.
    Prints a `[presentation]` census at the end — see the docs for what its
    zeroes mean.
";

/// Count the presentation components the REAL animation path needs, once.
///
/// ⭐⭐ THE QUESTION THIS ANSWERS. The frame cursor in the viewer is a
/// reimplementation of `CharacterAnimator`, and a reimplementation drifts. The
/// only reason it exists is that the real one was not present in this headless
/// app — so the standing question is WHICH LINK is missing, and a census beats
/// another round of inference. `PlayerVisual` gates the pose read-model
/// (`rebuild_body_pose_views` filters `With<PlayerVisual>`); `CharacterAnimator`
/// is the cursor itself; `BodyPoseView` is the published result.
/// ⛔⛔ MEASURED 2026-08-27, and the answer is TWO separate blockers, neither of
/// which is "headless cannot animate":
///
///  1. `BodyPoseView` is not available to a smash fighter IN ANY MODE. Its query
///     is filtered `With<PlayerVisual>`, and `PlayerVisual` is granted in exactly
///     ONE production place — `session/setup.rs`, to the exploration player's
///     avatar. A seated `MatchSeat` fighter never carries it, windowed or not.
///  2. `CharacterAnimator` is built by the RENDER layer from a loaded
///     `CharacterSpriteAsset`. `NoWindow` sets `backends: None`, which omits the
///     render app by design — its own doc says so: *"Nothing is ever drawn...
///     That is not a limitation to route around; it is what this mode is."*
///
/// ⭐ AND THE ROUTE OUT IS `OffscreenGpu`, which HAS a render app and which
/// `capture_scene` already runs headlessly on this machine. Switching this tool's
/// mode alone is not enough — it panics in `bevy_pbr`'s skin batching, because
/// `capture_scene` boots through `build_visible_app_with` plus its own camera and
/// render-target setup. That is the bounded piece of work that would let this
/// tool read `CharacterAnimator::frame` directly and delete the viewer's
/// reimplementation of it.
fn presentation_census(world: &mut World) -> String {
    let bodies = world
        .query::<&ambition_platformer2d::engine_core::BodyKinematics>()
        .iter(world)
        .count();
    let visuals = world
        .query_filtered::<
            bevy::prelude::Entity,
            bevy::prelude::With<ambition_platformer2d::platformer::lifecycle::PlayerVisual>,
        >()
        .iter(world)
        .count();
    let animators = world
        .query::<&ambition_platformer2d::sprite_sheet::character::CharacterAnimator>()
        .iter(world)
        .count();
    let poses = world
        .query::<&ambition_platformer2d::sim_view::BodyPoseView>()
        .iter(world)
        .count();
    format!(
        "[presentation] bodies={bodies} PlayerVisual={visuals} \
         CharacterAnimator={animators} BodyPoseView={poses}"
    )
}

/// Which sheet ROW this body is being drawn from, as `(sheet key, row index)`.
///
/// ⭐ THE CLIP FIRST, THEN THE POSE, which is the order the renderer resolves in
/// (`CharacterAnimator::drawn_row`). A move that authors a clip its sheet has is
/// drawn from that row and from no other, and the semantic pose is what every
/// body without one falls back to.
///
/// `None` when the character names no sheet, the sheet is not baked into this
/// build, or the sheet has no row for the pose — three different absences that
/// all mean the same thing to a viewer: draw the box and no picture.
fn drawn_row_of(
    sheet_keys: &std::collections::HashMap<String, String>,
    worn: Option<&str>,
    playing: Option<&ambition_platformer2d::entity_catalog::MoveSpec>,
    on_ground: Option<bool>,
) -> Option<(String, u32, bool)> {
    use ambition_platformer2d::sprite_sheet::character::sheets::{
        try_load_spec_for_target, SheetTuning,
    };
    let key = sheet_keys.get(worn?)?.clone();
    let spec = try_load_spec_for_target(&key, &SheetTuning::default())?;
    // ⭐ THE MOVE'S OWN CLIP CHAIN, which is the authored answer to "what does
    // this look like" and the same chain the renderer resolves. `first_bound_row`
    // walks it and stops at the first row the sheet actually has, so a fighter
    // whose sheet lacks `smash_forward` falls back exactly as it does in game.
    if let Some(spec_move) = playing {
        let chain: Vec<&str> = std::iter::once(spec_move.clip.clip.as_str())
            .chain(spec_move.clip.fallbacks.iter().map(String::as_str))
            .collect();
        if let Some(slot) = spec.clip_slot(chain) {
            // ⛔⛔ A MOVE'S CLIP PLAYS ONCE AND HOLDS ITS LAST FRAME. That is
            // `CharacterAnimator::tick_slot`, which sets `clip_held` and stops —
            // a swing does not loop back to its windup while the recovery runs.
            // A viewer that looped it would show the move restarting mid-move.
            return Some((key, slot as u32, true));
        }
    }
    // ⛔ AND A RESTING BODY IS NOT NOTHING. Without this the view would show art
    // only while a move is playing and a bare box the rest of the time, which
    // reads as the art being broken rather than the fighter standing still.
    let resting = if on_ground == Some(false) {
        "jump"
    } else {
        "idle"
    };
    // ⭐ AND A RESTING POSE LOOPS, which is the other half of the same rule.
    spec.clip_slot([resting, "idle"])
        .map(|slot| (key, slot as u32, false))
}

/// Read the world once. Everything here is a read; nothing is mutated, so a
/// take can never be the reason a run diverges.
///
/// ⛔⛔ COMBAT GEOMETRY COMES FROM `CombatGeometryView`, THROUGH
/// `combat_observation`. This function used to query `Hitbox` and call
/// `world_volume` itself, which made it a SECOND implementation of the rule the
/// engine already owns — and it had no hurtboxes at all, so the recording could
/// show an attack volume passing through a fighter and could not say whether
/// that fighter was hittable there.
fn sample(world: &mut World, scenario: &ScenarioRoles) -> Frame {
    let mut frame = Frame::default();
    // ⛔⛔ RESOLVED THIS TICK. The two fighters are fixed for the take; what they
    // OWN is discovered every sample, because the summon this recording exists
    // to show is spawned by the move itself.
    let roles = scenario.resolve(world);

    // ⭐ CHARACTER ID -> SHEET KEY, off the catalog the composed host loaded. The
    // catalog stores `sprites/<name>_spritesheet.png`; the baked sheet index is
    // keyed by the bare name, and that reduction is the join.
    //
    // ⛔⛔ BUILT ONCE. This was rebuilt on EVERY `sample`, and `settle` calls
    // `sample` up to 480 times per take -- roughly nine thousand rebuilds of a
    // 48-entry map, each one re-splitting every catalog path, per character. The
    // catalog cannot change during a run, so the map is a constant wearing a
    // loop's clothes.
    static SHEET_KEYS: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    let sheet_keys = SHEET_KEYS.get_or_init(|| {
        world
            .get_resource::<ambition_platformer2d::character::CharacterCatalog>()
            .map(|catalog| {
                catalog
                    .data()
                    .characters
                    .iter()
                    .filter_map(|(id, entry)| {
                        let base = entry
                            .spritesheet
                            .rsplit('/')
                            .next()?
                            .trim_end_matches(".png")
                            .trim_end_matches("_spritesheet")
                            .to_string();
                        (!base.is_empty()).then(|| (id.clone(), base))
                    })
                    .collect()
            })
            .unwrap_or_default()
    });

    let mut bodies = world.query::<(
        Entity,
        &ambition_platformer2d::engine_core::BodyKinematics,
        Option<&ambition_platformer2d::actor::MatchSeat>,
        Option<&ambition_platformer2d::character::WornCharacter>,
        Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
        Option<&ambition_platformer2d::mount::RidingOn>,
        Option<&ambition_platformer2d::mount::MountSlot>,
        Option<&ambition_platformer2d::engine_core::BodyGroundState>,
        // ⭐ WHAT THE ENGINE UNDERSTOOD THE PRESS TO BE. The recording already
        // shows which move came out; this shows why. A take that drove BACK and
        // played the forward air is unreadable without it — the direction is
        // resolved against facing, a turnaround flips that facing, and none of
        // it is visible from the move id alone.
        Option<&ambition_platformer2d::characters::actor::attack_gesture::ResolvedAttackGesture>,
        // ⭐⭐ WHICH PICTURE IS ON SCREEN. Jon, 2026-08-27: *"The UI does not show
        // any art, or how the move looks animated in game."* A take that records
        // only rectangles can prove a move CAME OUT and can never show what it
        // LOOKS like — and the brief asked for the second from the start (*"we
        // will see things like the pirate flying around on the shark"*). Sheet,
        // row and frame are what a viewer needs to blit the same sub-rect the
        // engine drew.
        // ⛔⛔ THE POSE VIEW, NOT THE RENDERER'S ANIMATOR. `CharacterAnimator` is
        // the obvious component and it is the WRONG ONE HERE: the render layer
        // inserts it once a sprite ASSET has loaded, and this tool runs
        // `NoWindow` where that never happens. Measured, not assumed — the first
        // version asked for the animator and recorded 14446 bodies with art on
        // exactly ZERO of them. `BodyPoseView` is published by the SIM every tick
        // and carries the same two facts: the semantic pose, and the CLIP an
        // active move asked to be drawn as.
        //
        // ⛔⛔ AND IT WAS PUBLISHED FOR NOBODY THIS TOOL WATCHES UNTIL 2026-08-29.
        // The read model was gated on `With<PlayerVisual>`, granted in exactly
        // one production place — the exploration player's avatar — so every
        // `MatchSeat` fighter recorded `has_pose: false` and the viewer fell back
        // to reconstructing a frame cursor in JavaScript. Every granted character
        // body carries `PosedBody` now and the gate is `Or` of the two.
        Option<&ambition_platformer2d::sim_view::BodyPoseView>,
        // ⛔⛔ A SUMMON WEARS NO CATALOG CHARACTER, and the summon is the one
        // everybody opens this view to watch — Jon asked to *"see things like the
        // pirate flying around on the shark"*. The shark has no `WornCharacter`,
        // so joining the sheet on that alone drew the rider in full art and his
        // mount as an empty box. `ActorConfig::sprite_character_id` is what the
        // renderer itself falls back to for exactly these bodies.
        Option<&ambition_platformer2d::combat::actor_tuning::ActorConfig>,
        // ⛔⛔ A RAW ENTITY ID IS NOT AN IDENTITY. The label fell back to
        // `format!("{entity}")`, and an entity index depends on every spawn and
        // despawn the whole app made first — so two runs of this binary labelled
        // the SAME shark `1311v10` and `1329v6`, and a byte-diff of two
        // recordings reported 15 of 19 takes as changed when their physics were
        // identical to the last float. `SimId` is the engine's own stable
        // identity, the one rollback remaps across a rewind.
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
    )>();
    let rows: Vec<_> = bodies
        .iter(world)
        .map(
            |(e, kin, seat, worn, play, riding, slot, ground, gesture, pose, config, sim_id)| {
                (
                    e,
                    (kin.pos.x, kin.pos.y),
                    (kin.vel.x, kin.vel.y),
                    (kin.size.x * 0.5, kin.size.y * 0.5),
                    kin.facing,
                    seat.map(|s| s.0),
                    worn.map(|w| w.id().to_string()),
                    play.map(|p| p.spec.id.clone()),
                    riding.map(|r| r.mount),
                    slot.is_some(),
                    ground.map(|g| g.on_ground),
                    gesture
                        .and_then(|g| g.pressed)
                        .map(|i| format!("{:?}/{:?}/{:?}", i.direction, i.strength, i.posture)),
                    // ⛔ THE ROW, NOT THE POSE NAME. A clip draws from a row the
                    // semantic pose does not name; asking the pose would blit the
                    // wrong picture for exactly the frames a move is playing, which
                    // is every frame anybody opens this view to look at.
                    drawn_row_of(
                        &sheet_keys,
                        worn.map(|w| w.id())
                            .or_else(|| config.and_then(|c| c.sprite_character_id.as_deref())),
                        play.map(|p| &p.spec),
                        ground.map(|g| g.on_ground),
                    ),
                    pose.is_some(),
                    sim_id.map(|id| id.as_str().to_string()),
                )
            },
        )
        .collect();

    // ⛔⛔ WHO THE OUTPUT BELONGS TO. The take seats a real CPU opponent on
    // purpose — a move recorded against an inert stage is a move recorded in a
    // game nobody plays — but that opponent SWINGS AND FIRES, and this sampler
    // collected every live hitbox and every projectile in the world. So a
    // hitless movement special could show a hitbox and a ranged move could
    // report more shots than it fires: the opponent's offence, credited to the
    // subject.
    //
    // ⭐ THE FIX IS PROVENANCE, NOT AN INERT OPPONENT. Both are still recorded —
    // the viewer wants to see what was happening — but each carries whose it is,
    // and the move's own statistics count only the subject's.
    //
    // ⭐⭐ AND PROVENANCE IS A ROLE, NOT A BOOLEAN. `subject_owned` could say
    // "not the subject's" and never say whose: the target's swing, the target's
    // summon and a stage hazard were one answer. `ScenarioRoles` names all five,
    // and every ownership question in this file is asked of it.

    // ⛔⛔ THE COMBAT HALF OF EVERY ROW, FROM THE SEMANTIC VIEW. Geometry, move
    // clock and the tuning readout are read here ONCE and merged onto the
    // identity rows below; nothing in this file resolves a volume.
    let observation = CombatObservation::capture(world, &roles);
    let combat_facts: std::collections::HashMap<Entity, serde_json::Value> = observation
        .bodies
        .iter()
        .map(|body| (body.entity, body.facts.clone()))
        .collect();

    for (
        entity,
        pos,
        vel,
        half,
        facing,
        seat,
        worn,
        playing,
        riding,
        is_mount,
        on_ground,
        gesture,
        drawn,
        has_pose,
        sim_id,
    ) in &rows
    {
        let role = roles.role_of(*entity);
        let subject = role == ambition_sim_harness::ScenarioRole::Subject;
        if subject {
            frame.subject_pos = Some(*pos);
            frame.subject_vel = Some(*vel);
            frame.move_id = playing.clone();
            frame.grounded = *on_ground;
            frame.facing = Some(*facing);
            frame.gesture = gesture.clone();
            // ⛔⛔ THE MOUNT'S LABEL, NOT ITS WORN CHARACTER. Reading the ride
            // through `WornCharacter` reported `riding: null` for a real,
            // boarded shark — a summoned mount wears no catalog character, so
            // the `and_then` erased the very fact this take exists to show. The
            // ride is `RidingOn` existing; the label is a nicety.
            frame.riding = riding.map(|mount| {
                rows.iter()
                    .find(|(e, ..)| *e == mount)
                    .and_then(|row| row.6.clone())
                    // ⛔ THE MOUNT'S STABLE ID, never its entity index: a raw
                    // entity id is not an identity and makes two runs differ.
                    .or_else(|| {
                        rows.iter()
                            .find(|(e, ..)| *e == mount)
                            .and_then(|row| row.14.clone())
                    })
                    .unwrap_or_else(|| "<unidentified mount>".to_string())
            });
        }
        if sim_id.is_none() {
            frame
                .unidentified
                .push(worn.clone().unwrap_or_else(|| "<unnamed body>".to_string()));
        }
        let mut body = serde_json::json!({
            // The KINEMATIC box: where the body is and how big it is. The
            // COMBAT envelope and the volumes that decide a hit arrive with the
            // observation below, under `collision` and `hurtboxes`.
            "pos": [pos.0, pos.1],
            "half": [half.0, half.1],
            "seat": seat,
            // ⭐⭐ WHAT THIS BODY IS IN THE SCENARIO, in a word. A reader must
            // never have to work the subject out from a seat index or a colour
            // — and cannot, when the scenario deliberately seats one character
            // twice.
            "role": role.as_str(),
            // ⛔⛔ IDENTITY AND APPEARANCE ARE TWO FIELDS, NOT ONE. Preferring
            // the worn character as a "label" cannot identify a body in this
            // recording at all: the take deliberately seats TWO FIGHTERS WEARING
            // THE SAME CHARACTER, so `npc_pirate_admiral` names both of them.
            // `SimId` is the engine's deterministic identity, independent of
            // Bevy entity allocation and ordered so snapshots can establish a
            // canonical order.
            "id": sim_id.clone(),
            "character": worn.clone(),
            // What a reader recognises, which is allowed to be ambiguous
            // because it is not what anything joins on.
            "label": worn
                .clone()
                .or_else(|| sim_id.clone())
                .unwrap_or_else(|| "<unidentified body>".to_string()),
            // A summoned mount is neither a seat nor scenery, and a viewer that
            // could not tell it apart would draw the shark as another fighter.
            "kind": if *is_mount { "summon" } else if seat.is_some() { "fighter" } else { "body" },
            "move": playing.clone(),
            // Which way the art is mirrored. The sheets are drawn facing one
            // way and the engine flips them; a viewer that ignored this would
            // draw every left-moving fighter running backwards.
            "facing": facing,
            // `[sheet_key, row_index]`, or absent when this body has no sheet
            // or the sheet has no row for its pose. The FRAME INDEX is not here
            // on purpose: the viewer derives it by counting how many consecutive
            // ticks a body has held the same row, which is exact playback timing
            // out of the recording itself rather than a second clock to keep in
            // step with the first.
            // `[sheet_key, row_index, holds_last_frame]`. The third is the
            // difference between a swing and a stance: a move's clip plays once
            // and holds, a resting pose loops.
            "art": drawn
                .as_ref()
                .map(|(sheet, row, holds)| serde_json::json!([sheet, row, holds])),
            // ⭐ WHY THERE IS NO ART, when there is none. "the picture is missing"
            // has three causes that look identical in a viewer — no pose published,
            // no sheet joined, or a sheet with no row for this pose — and a take
            // that does not distinguish them sends the next reader back through
            // the whole chain. Cheap, and it has already paid for itself once.
            "has_pose": has_pose,
            "sheet": drawn.as_ref().map(|(sheet, ..)| sheet),
        });
        // The combat half, merged onto the identity half. A body the combat
        // view does not carry — a summoned mount is not a `BodyCombat` — keeps
        // its identity row and simply has no combat fields, which is the honest
        // answer rather than a row of zeroes.
        if let (Some(facts), Some(object)) = (combat_facts.get(entity), body.as_object_mut()) {
            if let Some(facts) = facts.as_object() {
                for (key, value) in facts {
                    object.insert(key.clone(), value.clone());
                }
            }
        }
        frame.bodies.push(body);
    }

    // ⭐ A RANGED MOVE'S DAMAGE IS ITS PROJECTILE, and a take that recorded only
    // hitboxes showed the pirate's new side-B as a move that fires nothing.
    // Projectiles are excluded from every actor-generic query by construction
    // (`ProjectileGameplay` is the marker that keeps them out), so they have to
    // be asked for by name.
    let mut shots = world.query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::platformer::projectile::ProjectileGameplay,
        Option<&ambition_platformer2d::projectiles::ProjectileOwner>,
        // ⛔ THE STABLE IDENTITY, for the same reason bodies carry one: the
        // ORDER these arrive in is ECS query order, and a recording that two
        // runs cannot compare byte-for-byte is one nothing can be diffed
        // against.
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
    )>();
    let flying: Vec<_> = shots
        .iter(world)
        .map(|(kin, shot, owner, sim_id)| {
            (
                kin.pos,
                kin.vel,
                kin.size,
                shot.damage,
                owner.map(|owner| owner.0),
                sim_id.map(|id| id.as_str().to_string()),
            )
        })
        .collect();
    for (pos, vel, size, damage, owner, sim_id) in flying {
        // Whose shot this is, in the same vocabulary the bodies and strikes
        // use. An unowned shot is a hazard and belongs to nobody — which is not
        // the same as belonging to the target.
        let role = owner.map_or(ambition_sim_harness::ScenarioRole::Other, |owner| {
            roles.owned_role_of(owner)
        });
        frame.projectiles.push(serde_json::json!({
            "id": sim_id,
            "pos": [pos.x, pos.y],
            "vel": [vel.x, vel.y],
            "half": [size.x * 0.5, size.y * 0.5],
            "damage": damage,
            "role": role.as_str(),
            // A shot with no owner belongs to nobody in particular — a hazard,
            // a stage emitter — and is not the subject's either way. Kept
            // beside the role for readers written before roles existed; the
            // role is the authority and this is its projection.
            "subject_owned": role.is_subjects(),
        }));
    }

    // ⭐⭐ THE STRIKES, WHOLE, FROM THE SEMANTIC VIEW. Volume, shape, damage,
    // owner, role and identity all arrive resolved: `CombatGeometryView` puts
    // every live strike into world space with the same `Hitbox::world_volume`
    // the resolver uses, and `combat_observation` writes the row.
    //
    // ⛔⛔ THIS FILE NO LONGER RESOLVES A VOLUME. It queried `Hitbox`, anchored
    // it against a position map of its own and called `world_volume` — a second
    // implementation of a rule the engine owns, which had already been wrong
    // once (`world_aabb` recorded a sweeping arc as the rectangle containing
    // it). `check_absence_contracts.py` keeps it gone.
    frame.hitboxes = observation.strikes.clone();
    frame.contacts = observation.contacts.clone();

    // ⛔⛔ SORTED BY STABLE IDENTITY BEFORE IT IS WRITTEN. Removing entity numbers
    // from the strings is not enough to promise byte-stable JSON: the ORDER of
    // these rows is Bevy query iteration order, which is archetype order, which
    // changes when anything about component composition changes. A recording
    // that two runs cannot compare byte-for-byte is a recording nothing can be
    // diffed against.
    frame.bodies.sort_by(|a, b| {
        let key = |v: &serde_json::Value| {
            (
                v["id"].as_str().unwrap_or("").to_string(),
                v["label"].as_str().unwrap_or("").to_string(),
            )
        };
        key(a).cmp(&key(b))
    });
    frame.projectiles.sort_by(|a, b| {
        let key = |v: &serde_json::Value| {
            (
                v["id"].as_str().unwrap_or("").to_string(),
                v["pos"][0].as_f64().unwrap_or_default().to_bits(),
                v["pos"][1].as_f64().unwrap_or_default().to_bits(),
            )
        };
        key(a).cmp(&key(b))
    });

    frame
}

fn platforms(app: &mut App) -> Vec<serde_json::Value> {
    app.world_mut()
        .run_system_once(
            |world: ambition_platformer2d::world::collision::CollisionWorld| -> Vec<serde_json::Value> {
                let Some(solids) = world.solids() else {
                    return Vec::new();
                };
                solids
                    .blocks
                    .iter()
                    .map(|b| {
                        serde_json::json!([
                            (b.aabb.min.x + b.aabb.max.x) * 0.5,
                            (b.aabb.min.y + b.aabb.max.y) * 0.5,
                            (b.aabb.max.x - b.aabb.min.x) * 0.5,
                            (b.aabb.max.y - b.aabb.min.y) * 0.5,
                        ])
                    })
                    .collect()
            },
        )
        .unwrap_or_default()
}

/// How the fighter opposite the subject behaves.
///
/// ⭐⭐ A SCENARIO PARAMETER, NOT A TOOL DETAIL. Which of these was used decides
/// what the recording MEANS — a move measured against a fighter that walks into
/// it is a different measurement from the same move against one that stands
/// there — so the take writes the answer down beside the frames.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetBehavior {
    /// Seated, damageable, and making no decisions.
    Passive,
    /// The live duelist brain: a real opponent, with a real opponent's noise.
    Cpu,
}

impl TargetBehavior {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "passive" => Some(Self::Passive),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Passive => "passive",
            Self::Cpu => "cpu",
        }
    }
}

/// Put a clean match on the stage.
///
/// ⛔⛔ A TAKE THAT STARTS FROM A CORPSE MEASURES NOTHING. Two takes reported
/// their move as producing nothing because the previous one had knocked the
/// admiral off the stage: the recording showed a body frozen below the floor
/// with `grounded: false` forever, and the press went to somebody who was not
/// there. The settle can detect that state; only a re-seat can fix it.
///
/// ⭐⭐ THE TARGET IS AN ARGUMENT. This seated `[character, character]` — the
/// same fighter twice, told apart only by a seat index — so every recorded frame
/// needed a seat convention to read, and a screenshot could not be read at all.
fn reseat(app: &mut App, character: &str, target: &str, behavior: TargetBehavior) -> bool {
    let roster = match behavior {
        // ⛔ THE STAND-STILL BRAIN IS A DRIVER, NOT A MISSING ONE. A CPU seat
        // that names no brain profile is REFUSED at preparation, on purpose;
        // this asks for the policy that stands still by name.
        TargetBehavior::Passive => {
            ambition_demo_smash::smash_roster_with_passive_targets([character, target])
        }
        TargetBehavior::Cpu => ambition_demo_smash::smash_roster([character, target]),
    };
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // ⛔⛔ STAGING IS A POSTCONDITION, NOT A DURATION. This spent a fixed 240
    // updates and returned nothing, so a route that did not come up or a
    // character the host could not build meant recording a whole moveset off an
    // EMPTY STAGE in that character's name. The 240 stay as a ceiling rather
    // than the answer, and the common case got faster: the loop ends the moment
    // the subject is there.
    for _ in 0..240 {
        app.update();
        if move_exercise::subject(app).is_some() {
            return true;
        }
    }
    false
}

/// This fighter's repertoire, as the host prepared it.
fn moveset_of(
    app: &mut App,
    character: &str,
) -> Option<ambition_platformer2d::entity_catalog::MovesetContract> {
    app.world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .and_then(|registry| registry.get(character))
        .and_then(|prepared| prepared.kit.projectable_moveset())
        .cloned()
}

/// Does this move AUTHOR offence — a strike volume, or an event that fires?
///
/// ⭐ THE AUTHORING, NOT THE RECORDING. It is the only independent answer the
/// tool has to "should this take show any offence at all", which is what makes
/// it usable to check the recording rather than to describe it.
fn authors_offense(spec: &ambition_platformer2d::entity_catalog::MoveSpec) -> bool {
    spec.windows.iter().any(|w| !w.volumes.is_empty())
        || spec.events.iter().any(|e| {
            matches!(
                e.kind,
                ambition_platformer2d::entity_catalog::MoveEventKind::Ranged
            )
        })
}

/// Sample the world and append it to a take, reporting any body it could not
/// identify.
///
/// ⛔⛔ THE CALLER MUST NOT WRITE A TAKE THAT REPORTED ONE. `SimId` is what the
/// bundle joins and orders on, and a body without one was written as
/// `"id": null` and sorted under the empty string — so its position in a
/// "byte-stable, canonical" recording was query order wearing a contract's
/// clothes. `ensure_sim_id` covers authored placements and the primary player
/// only; its own doc says a dynamically spawned body stays unidentified unless
/// its spawn site mints an id, so this is reachable rather than theoretical.
///
/// ⭐ THE VIEWER'S `label + seat` FALLBACK IS RIGHT FOR LEGACY TAKES and wrong as
/// a licence for the recorder: what an old file may contain does not define what
/// a new one may emit. Measured on the admiral: 8202/8202 bodies and 166/166
/// strikes already identified, so this refuses a regression rather than
/// demanding something new.
fn record(
    app: &mut App,
    scenario: &ScenarioRoles,
    frames: &mut Vec<serde_json::Value>,
) -> Vec<String> {
    let frame = sample(app.world_mut(), scenario);
    let unidentified = frame.unidentified.clone();
    frames.push(serde_json::json!({
        "bodies": frame.bodies,
        "hitboxes": frame.hitboxes,
        "projectiles": frame.projectiles,
        "contacts": frame.contacts,
        "move": frame.move_id,
        "grounded": frame.grounded,
        "subject_pos": frame.subject_pos.map(|p| vec![p.0, p.1]),
        "subject_vel": frame.subject_vel.map(|v| vec![v.0, v.1]),
        "facing": frame.facing,
        "gesture": frame.gesture,
        "riding": frame.riding,
    }));
    unidentified
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // ⛔⛔ BEFORE THE APP BOOTS, and before it seats a match and drives a hundred
    // takes. `--help` used to build the engine, ignore the flag, and record the
    // default fighter anyway.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    // ⛔ AN UNKNOWN FLAG IS A REFUSAL. `--character` (singular) is the obvious
    // typo for `--characters` and this parser would have ignored it and silently
    // recorded the default fighter instead — a wrong answer that looks like a
    // right one.
    if let Some(bad) = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with('-'))
        .find(|a| {
            !matches!(
                a.as_str(),
                "--out"
                    | "--characters"
                    | "--target"
                    | "--target-behavior"
                    | "--verbs"
                    | "--spacing"
            )
        })
    {
        eprintln!("moveset_takes: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    let arg = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    let out = arg("--out")
        .unwrap_or_else(|| "tools/ambition_moveset_inspector/data/takes/takes.json".to_string());
    let asked = arg("--characters");
    // ⛔ A MIRROR MATCH IS THE DEFAULT AND IT IS A CHOICE. Recording a fighter
    // against itself keeps the geometry comparable across the grid; naming a
    // target makes the two bodies visibly different when that is what a reader
    // needs.
    let target = arg("--target");
    let spacing: Option<f32> = match arg("--spacing") {
        None => None,
        Some(word) => match word.parse::<f32>() {
            Ok(px) if px >= 0.0 => Some(px),
            _ => {
                eprintln!("moveset_takes: --spacing wants a non-negative number of pixels");
                std::process::exit(2);
            }
        },
    };
    // ⛔ A NAME THAT MATCHES NOTHING IS A REFUSAL, not an empty run. `--verbs
    // upb` would otherwise record nothing and report success.
    let only: Option<Vec<String>> = arg("--verbs").map(|list| {
        let asked: Vec<String> = list.split(',').map(|v| v.trim().to_string()).collect();
        for verb in &asked {
            if !VERBS.iter().any(|known| known.verb == verb) {
                eprintln!(
                    "moveset_takes: '{verb}' is not a verb this exercise can perform.\n\
                     known: {}",
                    VERBS.iter().map(|v| v.verb).collect::<Vec<_>>().join(", ")
                );
                std::process::exit(2);
            }
        }
        asked
    });
    let behavior = match arg("--target-behavior").as_deref() {
        None => TargetBehavior::Passive,
        Some(word) => match TargetBehavior::parse(word) {
            Some(behavior) => behavior,
            None => {
                eprintln!(
                    "moveset_takes: unknown --target-behavior '{word}'; expected passive or cpu"
                );
                std::process::exit(2);
            }
        },
    };

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.add_plugins(bevy::log::LogPlugin::default());
    // ⭐⭐ THE CLOCK IS OURS FROM HERE. Without this the rollback host advances
    // from a WALL-CLOCK accumulator, so one `app.update()` runs zero, one or
    // several sim ticks depending on how long the previous iteration took —
    // which made `TAKE_TICKS = 150` mean 150 LOOP ITERATIONS, made a recording
    // cost at least as much real time as the game time it contained, and made
    // two runs of this binary disagree on 13 of 19 takes.
    // ⭐ THE ENGINE'S OWN MANUAL-STEP CONTRACT, not a private one. See
    // `ambition_platformer2d::app::manual_step_period`: the two simulation hosts
    // need periods that differ by one nanosecond, and a driver that computed its
    // own would be silently wrong under one of them.
    //
    // ⭐ THE APP ANSWERS WHICH HOST IT HAS. This passed a literal `true` under a
    // comment admitting the app should know; `SimulationHost` is a resource and
    // was already the canonical answer, so the caller had no business having an
    // opinion about it.
    ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    for _ in 0..30 {
        app.update();
    }

    // ⭐⭐ `--characters grid` RECORDS THE WHOLE SMASH GRID, and it is the flag
    // this tool should always have had. The default of one fighter made the
    // inspector's take view look broken — *"There are 2 fighters now, why not
    // them all?"* — because the picker lists what was RECORDED and recording is
    // opt-in per name. Nobody should have to type twenty-one ids to answer a
    // roster-wide question that a balance tool exists to answer.
    //
    // ⛔ THE GRID, NOT THE WHOLE CAST. The registry holds 48 prepared characters
    // and most are NPCs with no moveset; the grid is the set a player can pick,
    // which is the set a balance view is about.
    let who: Vec<String> = match asked.as_deref() {
        Some("grid") | Some("all") => {
            let registry = app
                .world()
                .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
                .expect("the composed host has a prepared-character registry");
            ambition_demo_smash::select::SmashRoster::assemble(registry)
                .ids()
                .map(|id| id.to_string())
                .collect()
        }
        Some(list) => list.split(',').map(|x| x.trim().to_string()).collect(),
        None => vec!["npc_pirate_admiral".to_string()],
    };
    // ⛔ SAY HOW LONG THIS WILL TAKE. Every take settles a real match between
    // presses, so a grid run is tens of minutes — and a tool that goes quiet for
    // half an hour without saying so reads as hung.
    eprintln!(
        "[moveset-takes] recording {} character(s): {}",
        who.len(),
        who.join(", ")
    );

    let mut takes = Vec::new();

    for character in &who {
        // A partner, so contact rules and targeting behave as they do in a
        // match. A solo stage is a different simulation from the one the
        // inspector claims to be showing.
        // ⛔ THE TARGET IS RESOLVED PER SUBJECT. `--target` names one fighter
        // for the whole run; without it each subject faces itself, which keeps
        // a grid recording comparable fighter to fighter.
        let target = target.clone().unwrap_or_else(|| character.clone());
        reseat(&mut app, character, &target, behavior);
        let stage = platforms(&mut app);
        // The fighter's whole repertoire, so a take can ask what the move it played
        // AUTHORS and check its own recording against that. See
        // `authors_offense`.
        let repertoire = moveset_of(&mut app, character);

        for verb in VERBS
            .iter()
            .filter(|verb| only.as_ref().is_none_or(|only| only.iter().any(|v| v == verb.verb)))
        {
            // ⛔⛔ A FRESH MATCH FOR EVERY TAKE, not only when the settle fails.
            // Re-seating on failure alone left every take depending on the one
            // before it, and the up-B is where that shows: `afford_recovery`
            // refuses a recovery whose airtime already spent one, so a take that
            // followed a lunge recorded the fighter's RECOVERY as a move that
            // produces nothing. Three separate false findings came out of that
            // ordering before it was the ordering that got measured.
            //
            // ⛔ IT COSTS 240 TICKS PER TAKE and is worth every one: an
            // instrument whose answer depends on what ran before it is not
            // measuring the thing it names.
            // ⛔⛔ A STAGE WITH NOBODY ON IT IS NOT A QUIET ONE. A route that
            // did not come up, or a character the host could not build, would
            // otherwise record a whole moveset against an empty stage under that
            // character's name.
            if !reseat(&mut app, character, &target, behavior) {
                println!(
                    "[take] {character:<24} {:<16} SKIPPED - no fighter reached seat zero",
                    verb.verb
                );
                continue;
            }
            if !settle(&mut app) {
                println!(
                    "[take] {character:<24} {:<16} WARNING - the stage would not go quiet \
                     even after a re-seat; read this take with that in mind",
                    verb.verb
                );
            }
            // ⛔ SPACING BEFORE POSTURE. Walking closes the gap on the ground;
            // an aerial verb then takes off from where it arrived. Doing it the
            // other way round would walk a body that is already in the air.
            let closed = spacing.map(|px| move_exercise::approach(&mut app, px));
            if closed == Some(false) {
                println!(
                    "[take] {character:<24} {:<16} WARNING - could not close to {} px;                      this take records the gap it reached",
                    verb.verb,
                    spacing.unwrap_or_default()
                );
            }
            // ⭐⭐ ONE PREPARATION, SHARED WITH THE RENDERER. This had its own
            // take-off loop, its own aim settle and its own retry count, so
            // "perform a back air" meant one thing here and another in
            // `moveset_render` — and only this one was ever tested. See
            // `move_exercise::prepare`, which is this algorithm promoted.
            let prepared = move_exercise::prepare(&mut app, verb);
            if !prepared {
                println!(
                    "[take] {character:<24} {:<16} WARNING - could not be airborne at the \
                     press; this take records the GROUNDED answer to that button",
                    verb.verb
                );
            }
            // ⛔⛔ AFTER THE RE-SEAT AND BEFORE THE PRESS. Roles are entity
            // identities, and every re-seat spawns new bodies — resolving them
            // once per RUN would have named the previous take's corpses.
            let roles = ScenarioRoles::from_seats(app.world_mut(), 0, 1);
            // ⛔ AT THE PRESS, WHICH IS WHERE THE NAME SAYS. Read after the take
            // it was the gap at the END — and a connect LAUNCHES the target, so
            // the forward smash reported 70px of spacing for a press thrown at
            // 33. A measurement named for a moment must be taken at it.
            let spacing_at_press = move_exercise::gap_to_seat(&mut app, 1).map(f32::abs);
            let mut frames: Vec<serde_json::Value> = Vec::new();
            // Every body this take could not identify, collected across its ticks.
            let mut unidentified: std::collections::BTreeSet<String> = Default::default();
            let facing = move_exercise::facing_of(&mut app);
            // ⛔⛔ THE SCHEDULE IS `move_exercise::action_frame`, AND NOTHING
            // ELSE DECIDES IT. This held while `tick < TAKE_TICKS / 4` and the
            // renderer while `shot < frames / 4`, which happened to agree at 37
            // and would have drifted the moment either tool changed how much it
            // records. What the player does is not the recorder's business.
            step(&mut app, move_exercise::action_frame(verb, 0, facing));
            // ⛔ THE PRESS TICK IS FRAME ZERO. `ResolvedAttackGesture::pressed`
            // is set on the press tick and cleared after, so a recording that
            // started one tick later showed `gesture: null` on every frame of
            // every take — the one field that says what the engine understood
            // the input to be, absent from all of them.
            unidentified.extend(record(&mut app, &roles, &mut frames));
            for tick in 1..TAKE_TICKS {
                step(&mut app, move_exercise::action_frame(verb, tick, facing));
                unidentified.extend(record(&mut app, &roles, &mut frames));
            }

            // ⛔⛔ A TAKE WITH AN UNIDENTIFIED BODY IS NOT CANONICAL, so it is not
            // written. Its row order would be query order, and the bundle joins
            // on `SimId` — a reader comparing two recordings would see changes
            // that are allocation order rather than physics. The message names
            // the body so the fix is at ITS SPAWN SITE, which is the only place
            // that can mint the id.
            if !unidentified.is_empty() {
                println!(
                    "[take] {character:<24} {:<16} SKIPPED - {} recorded \
                     without a SimId, so this take cannot be ordered or joined; \
                     mint one at that body's spawn site",
                    verb.verb,
                    unidentified.iter().cloned().collect::<Vec<_>>().join(", ")
                );
                continue;
            }

            let moves: std::collections::BTreeSet<String> = frames
                .iter()
                .filter_map(|f| f["move"].as_str().map(str::to_string))
                .collect();
            let rode = frames.iter().any(|f| !f["riding"].is_null());
            // ⭐ THE SUBJECT'S OWN OUTPUT. Everything in the world is still in
            // the frame for the viewer; what the MOVE is credited with is only
            // what the move's owner produced. See `subject_owned`.
            let subject_owned = |f: &serde_json::Value, key: &str| {
                f[key].as_array().map_or(0, |xs| {
                    xs.iter()
                        .filter(|x| x["subject_owned"].as_bool().unwrap_or(false))
                        .count()
                })
            };
            let live = frames
                .iter()
                .map(|f| subject_owned(f, "hitboxes"))
                .max()
                .unwrap_or(0);
            let shots = frames
                .iter()
                .map(|f| subject_owned(f, "projectiles"))
                .max()
                .unwrap_or(0);
            // ⭐⭐ THE TAKE CHECKS ITS OWN OWNERSHIP, and this is the arm that
            // makes provenance a claim rather than a hope. The take seats a live
            // CPU opponent on purpose — a move recorded against an inert stage is
            // a move recorded in a game nobody plays — and that opponent SWINGS
            // AND FIRES. Before `subject_owned` existed, its offence was counted
            // as the subject's, so a hitless movement special reported a hitbox
            // and a ranged move reported more shots than it fires
            //. Several of that review's quantitative conclusions were
            // wrong for that structural reason rather than a balance one.
            //
            // ⛔ THE INDEPENDENT ANSWER IS THE AUTHORING. A move whose windows
            // carry no volumes and whose timeline fires nothing CANNOT produce
            // offence of its own, whatever the world was doing around it — so a
            // nonzero count here is the recorder crediting somebody else's.
            //
            // ⛔⛔ AND IT REFUSES RATHER THAN WARNS. This file is tuned against;
            // a contaminated take that gets written is a number somebody
            // balances a fighter with.
            let opponent_output: usize = frames
                .iter()
                .map(|f| {
                    let all = |key: &str| f[key].as_array().map_or(0, |xs| xs.len());
                    (all("hitboxes") + all("projectiles"))
                        - subject_owned(f, "hitboxes")
                        - subject_owned(f, "projectiles")
                })
                .sum();
            if let Some(repertoire) = repertoire.as_ref() {
                let played: Vec<_> = repertoire
                    .moves
                    .iter()
                    .filter(|spec| moves.contains(&spec.id))
                    .collect();
                let hitless =
                    !played.is_empty() && !played.iter().any(|spec| authors_offense(spec));
                assert!(
                    !(hitless && (live > 0 || shots > 0)),
                    "[take] {character} {}: the move(s) {moves:?} author no strike volume and \
                     fire nothing, and this take recorded {live} subject-owned hitbox(es) and \
                     {shots} subject-owned shot(s). The stage produced {opponent_output} \
                     output(s) credited to somebody else over the same frames — and a ZERO \
                     there is not innocence, it is the classifier calling everything the \
                     subject's. A take that credits the stage to the subject is not a \
                     measurement.",
                    verb.verb
                );
            }

            // The view: the stage plus everything this take reached, padded.
            // Computed per take rather than per frame, so scrubbing does not
            // make a rising fighter look stationary while the world slides.
            let (mut x0, mut y0, mut x1, mut y1) = (
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            );
            for block in &stage {
                let v: Vec<f32> = block
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|n| n.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .unwrap_or_default();
                if v.len() == 4 {
                    x0 = x0.min(v[0] - v[2]);
                    y0 = y0.min(v[1] - v[3]);
                    x1 = x1.max(v[0] + v[2]);
                    y1 = y1.max(v[1] + v[3]);
                }
            }
            for f in &frames {
                for b in f["bodies"].as_array().into_iter().flatten() {
                    let (Some(px), Some(py)) = (b["pos"][0].as_f64(), b["pos"][1].as_f64()) else {
                        continue;
                    };
                    x0 = x0.min(px as f32 - 40.0);
                    y0 = y0.min(py as f32 - 40.0);
                    x1 = x1.max(px as f32 + 40.0);
                    y1 = y1.max(py as f32 + 40.0);
                }
            }
            if !x0.is_finite() {
                (x0, y0, x1, y1) = (-320.0, -240.0, 320.0, 240.0);
            }

            // ⛔⛔ THE TAKE SAYS WHETHER IT REACHED THE MOVE IT DROVE, which is
            // the whole reason it can be trusted. `attack_air_back` was the
            // standing counter-example — the recorded gesture read
            // `Forward/Tilt/Airborne`, so the fighter turned to face the back
            // input before the press was read — and the horizontal aim settle in
            // `move_exercise::prepare` closed it: measured 2026-08-28, all
            // seventeen bound verbs on the admiral reach their move and the
            // eighteenth, `special_air_down`, reports UNBOUND rather than
            // crediting itself with the down-B the chain answered with.
            // ⛔⛔ ONE VOCABULARY WITH THE RENDERER. `Outcome` has four answers and
            // none of them collapse into another — in particular an UNBOUND verb
            // is not a SUCCESS. Collapsing them lets this diagnostic panel and
            // `moveset_render` disagree about the same press.
            let intended = move_exercise::intended_move(&mut app, character, verb.verb);
            let verdict = move_exercise::outcome(prepared, intended.as_deref(), &moves);
            let reached = verdict.reached();
            println!(
                "[take] {character:<24} {:<16} moves={:?} hitboxes<={live} shots<={shots} rode={rode}{}",
                verb.verb,
                moves,
                if reached {
                    String::new()
                } else {
                    format!(
                        " {}: drove {} but the engine played {:?}",
                        verdict.as_str().to_uppercase(),
                        intended.as_deref().unwrap_or("<unbound>"),
                        moves
                    )
                }
            );
            // ⭐⭐ WHO WAS IN THIS SCENARIO, BY NAME AND BY IDENTITY. `seat: 0`
            // was the only thing that said which body the take was about, so
            // reading a frame — or a screenshot, or an exported SVG — needed a
            // seat convention nothing wrote down.
            let identity_of = |entity: Option<bevy::prelude::Entity>| {
                entity.and_then(|entity| {
                    ambition_sim_harness::combat_observation::sim_id_of(app.world(), entity)
                })
            };
            takes.push(serde_json::json!({
                "character": character,
                "verb": verb.verb,
                "label": verb.label,
                "seat": 0,
                "subject": character,
                "subject_id": identity_of(roles.subject()),
                "target": target,
                "target_id": identity_of(roles.target()),
                // ⛔ THE PREMISE, RECORDED. A passive target produces no offence
                // BY CONSTRUCTION, so `opponent_output: 0` below is a fact about
                // the scenario rather than evidence of a clean recording.
                "target_behavior": behavior.as_str(),
                // ⛔ THE SPACING ASKED FOR AND THE SPACING REACHED. A move that
                // could not close the gap is a finding; a take that reported
                // only the request would hide it.
                "requested_spacing": spacing,
                "spacing_at_press": spacing_at_press,
                "view": [x0, y0, x1, y1],
                "platforms": stage,
                // What the ENGINE did, which is the whole claim this file makes.
                // A take that reached no move says so here rather than looking
                // like a move with nothing in it.
                "moves_seen": moves.iter().cloned().collect::<Vec<_>>(),
                "rode_a_mount": rode,
                "max_live_hitboxes": live,
                "max_live_projectiles": shots,
                // ⛔ THE PREMISE, RECORDED. A take with zero here could not have
                // detected contamination however clean it looks: nothing else
                // was on the stage to be miscredited.
                "opponent_output": opponent_output,
                "intended_move": intended,
                "reached_intended_move": reached,
                // ⭐ WHICH KIND OF "no" it was. `reached: false` covers a verb
                // this fighter does not bind, a press that reached another move,
                // and a posture that could not be established — three different
                // things to a reader and to the viewer.
                "outcome": verdict.as_str(),
                "prepared": prepared,
                "frames": frames,
            }));
        }
    }

    let bundle = serde_json::json!({
        // ⛔ v2: every body, strike and shot carries a scenario ROLE, bodies
        // carry runtime hurtboxes and a move clock, and the take names its
        // subject and target. A v1 reader shown a v2 file still draws — the
        // added fields are additive — but a reader that needs roles must check.
        "schema": "ambition.moveset_takes.v2",
        "observation_schema": ambition_sim_harness::OBSERVATION_SCHEMA,
        "sim_hz": 60.0,
        "takes": takes,
    });
    let path = std::path::Path::new(&out);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("the output directory is creatable");
    }
    std::fs::write(
        path,
        serde_json::to_string(&bundle).expect("the takes serialize"),
    )
    .expect("the takes are writable");
    println!("{}", presentation_census(app.world_mut()));
    println!(
        "[moveset-takes] {} take(s) -> {out}",
        bundle["takes"].as_array().map_or(0, Vec::len)
    );
}
