//! Record what the real simulation does when a fighter performs each move.
//!
//! A frame-data table reports what a move declares. A take reports what the
//! engine did with it: where the body went, which hitboxes were live, what the
//! move spawned, and whether the fighter rode it. This lets the inspector show
//! that a move (for example the pirate's up-B shark ride) works in the real
//! engine.
//!
//! One App, one process: the tracing subscriber is process-global (see
//! `shark_ride_probe`), so this builds one App and seats every take in it.
//!
//! A move that does not come out is still recorded. An empty `move` field
//! means the press did not reach the move (a posture gate, a spent recovery,
//! a shield). Dropping those would show only the moves that already work.

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
    /// What connected with what, as the runtime says, not as geometry
    /// suggests. See `CombatObservation::contacts`.
    contacts: Vec<serde_json::Value>,
    move_id: Option<String>,
    /// Which use of the move. The id alone cannot show a cancel into the same
    /// move: it reads as one unbroken run. `MovePlayback::instance` tells the
    /// two apart.
    move_instance: Option<u32>,
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
    /// Recorded bodies with no `SimId`, by display label.
    ///
    /// The take's ordering and joins use `SimId`, so a body without one is
    /// counted here and the take is refused.
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
    --chain VERB         after the first verb, request this one too
    --chain-at TICK      the action tick the chained verb is REQUESTED on
                         [default: 37, the tick the first verb's own schedule
                         releases]
                         ⭐ SWEEP IT. The engine decides whether a cancel window
                         is open; the gap between the request and the acceptance
                         is the measurement.
    --target ID          who the subject performs the move AGAINST
                         [default: sandbag_infinite, the immortal training
                         dummy — ONE target for every subject, so a grid stays
                         comparable fighter to fighter]
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

/// Count the presentation components the real animation path needs, once.
///
/// The viewer's frame cursor reimplements `CharacterAnimator`, because the
/// real one is not present in this headless app. This census shows which
/// link is missing. `PlayerVisual` gates the pose read-model
/// (`rebuild_body_pose_views`); `CharacterAnimator` is the cursor;
/// `BodyPoseView` is the published result.
///
/// Known blockers:
///
///  1. `CharacterAnimator` is built by the render layer from a loaded
///     `CharacterSpriteAsset`. `NoWindow` sets `backends: None`, which omits
///     the render app by design.
///  2. `OffscreenGpu` has a render app (`capture_scene` uses it), but switching
///     this tool's mode alone panics in `bevy_pbr`'s skin batching:
///     `capture_scene` also sets up its own camera and render target through
///     `build_visible_app_with`. Doing that would let this tool read
///     `CharacterAnimator::frame` directly.
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

/// Which sheet row this body is drawn from, as `(sheet key, row index)`.
///
/// Clip first, then pose, in the renderer's order
/// (`CharacterAnimator::drawn_row`). A move that authors a clip its sheet has
/// is drawn from that row; other bodies fall back to the semantic pose.
///
/// `None` when the character names no sheet, the sheet is not in this build,
/// or the sheet has no row for the pose. The viewer then draws only the box.
fn drawn_row_of(
    sheet_keys: &std::collections::HashMap<String, String>,
    worn: Option<&str>,
    playing: Option<&ambition_entity_catalog::MoveSpec>,
    on_ground: Option<bool>,
) -> Option<(String, u32, bool)> {
    use ambition_platformer2d::sprite_sheet::character::sheets::{
        try_load_spec_for_target, SheetTuning,
    };
    let key = sheet_keys.get(worn?)?.clone();
    let spec = try_load_spec_for_target(&key, &SheetTuning::default())?;
    // The move's own clip chain, as the renderer resolves it.
    // `first_bound_row` stops at the first row the sheet has, so a missing
    // `smash_forward` row falls back as it does in game.
    if let Some(spec_move) = playing {
        let chain: Vec<&str> = std::iter::once(spec_move.clip.clip.as_str())
            .chain(spec_move.clip.fallbacks.iter().map(String::as_str))
            .collect();
        if let Some(slot) = spec.clip_slot(chain) {
            // A move's clip plays once and holds its last frame
            // (`CharacterAnimator::tick_slot` sets `clip_held`). Looping it
            // would show the move restarting mid-move.
            return Some((key, slot as u32, true));
        }
    }
    // A resting body still has art; otherwise the view shows a bare box
    // whenever no move plays.
    let resting = if on_ground == Some(false) {
        "jump"
    } else {
        "idle"
    };
    // A resting pose loops.
    spec.clip_slot([resting, "idle"])
        .map(|slot| (key, slot as u32, false))
}

/// Read the world once. Nothing is mutated, so a take cannot make a run
/// diverge.
///
/// Combat geometry comes from `CombatGeometryView` through
/// `combat_observation`. This function does not resolve volumes itself.
fn sample(world: &mut World, scenario: &ScenarioRoles) -> Frame {
    let mut frame = Frame::default();
    // Resolved every sample. The fighters are fixed, but what they own is
    // discovered each tick, because the move itself spawns the summon.
    let roles = scenario.resolve(world);

    // Character id -> sheet key, from the catalog the host loaded. The
    // catalog stores `sprites/<name>_spritesheet.png`; the baked sheet index
    // is keyed by the bare name. Built once: the catalog does not change
    // during a run, and `settle` calls `sample` up to 480 times per take.
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
        // What the engine understood the press to be. A take that drove back
        // and played the forward air is unreadable without it: direction is
        // resolved against facing, and a turnaround flips facing.
        Option<&ambition_platformer2d::characters::actor::attack_gesture::ResolvedAttackGesture>,
        // Which picture is on screen: sheet, row, and frame let a viewer blit
        // the same sub-rect the engine drew.
        //
        // Read `BodyPoseView`, not `CharacterAnimator`. The render layer
        // inserts the animator only after a sprite asset loads, which never
        // happens under `NoWindow`. The sim publishes `BodyPoseView` every
        // tick with the semantic pose and the move's clip. Every granted
        // character body carries `PosedBody`, so seated fighters have it.
        Option<&ambition_platformer2d::sim_view::BodyPoseView>,
        // A raw entity id is not an identity: it depends on every earlier
        // spawn and despawn, so two runs label the same body differently.
        // `SimId` is the engine's stable identity, which rollback remaps.
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
    )>();
    let rows: Vec<_> = bodies
        .iter(world)
        .map(
            |(e, kin, seat, worn, play, riding, slot, ground, gesture, pose, sim_id)| {
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
                    // The row, not the pose name. A clip draws from a row the
                    // pose does not name, so the pose gives the wrong picture
                    // while a move plays.
                    drawn_row_of(
                        &sheet_keys,
                        worn.map(|w| w.id()),
                        play.map(|p| &*p.spec),
                        ground.map(|g| g.on_ground),
                    ),
                    pose.is_some(),
                    sim_id.map(|id| id.as_str().to_string()),
                    // Appended at the tail: this tuple is also read by position
                    // below (`row.14`), so a middle insert renames later indices.
                    play.map(|p| p.instance),
                )
            },
        )
        .collect();

    // Who each output belongs to. The take seats a real CPU opponent, and
    // that opponent swings and fires. Everything is still recorded for the
    // viewer, but each item carries its owner, and the move's statistics
    // count only the subject's. Ownership is a role (`ScenarioRoles` names
    // all five), not a boolean.

    // The combat half of every row, from the semantic view. Geometry, move
    // clock, and tuning are read once here and merged onto the identity rows
    // below. Nothing in this file resolves a volume.
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
        instance,
    ) in &rows
    {
        let role = roles.role_of(*entity);
        let subject = role == ambition_sim_harness::ScenarioRole::Subject;
        if subject {
            frame.subject_pos = Some(*pos);
            frame.subject_vel = Some(*vel);
            frame.move_id = playing.clone();
            frame.move_instance = *instance;
            frame.grounded = *on_ground;
            frame.facing = Some(*facing);
            frame.gesture = gesture.clone();
            // The mount's label, not its worn character: a summoned mount
            // wears no catalog character. The ride is `RidingOn` existing;
            // the label is extra.
            frame.riding = riding.map(|mount| {
                rows.iter()
                    .find(|(e, ..)| *e == mount)
                    .and_then(|row| row.6.clone())
                    // The mount's stable id, never its entity index.
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
            // The kinematic box: position and size. The combat envelope and
            // hit volumes come with the observation, under `collision` and
            // `hurtboxes`.
            "pos": [pos.0, pos.1],
            "half": [half.0, half.1],
            "seat": seat,
            // The body's role in the scenario, so a reader never infers it
            // from a seat index or a colour.
            "role": role.as_str(),
            // Identity and appearance are separate fields. The take can seat
            // two fighters wearing the same character, so the character name
            // cannot identify a body. `SimId` is the deterministic identity,
            // independent of Bevy entity allocation.
            "id": sim_id.clone(),
            "character": worn.clone(),
            // What a reader recognises, which is allowed to be ambiguous
            // because it is not what anything joins on.
            "label": worn
                .clone()
                .or_else(|| sim_id.clone())
                .unwrap_or_else(|| "<unidentified body>".to_string()),
            // A summoned mount is neither a seat nor scenery; the viewer must
            // not draw the shark as a fighter.
            "kind": if *is_mount { "summon" } else if seat.is_some() { "fighter" } else { "body" },
            "move": playing.clone(),
            // Which way the art is mirrored. Sheets face one way and the engine
            // flips them.
            "facing": facing,
            // `[sheet_key, row_index, holds_last_frame]`, or absent when the
            // body has no sheet or the sheet has no row for its pose. A move's
            // clip plays once and holds; a resting pose loops. The frame index
            // is not stored: the viewer counts consecutive ticks on the same
            // row, which gives exact playback timing from the recording.
            "art": drawn
                .as_ref()
                .map(|(sheet, row, holds)| serde_json::json!([sheet, row, holds])),
            // Why there is no art: no pose published, no sheet joined, or a
            // sheet with no row for this pose. These look the same in a viewer.
            "has_pose": has_pose,
            "sheet": drawn.as_ref().map(|(sheet, ..)| sheet),
        });
        // The combat half, merged onto the identity half. A body the combat
        // view does not carry (a summoned mount is not a `BodyCombat`) keeps
        // its identity row with no combat fields, not a row of zeroes.
        if let (Some(facts), Some(object)) = (combat_facts.get(entity), body.as_object_mut()) {
            if let Some(facts) = facts.as_object() {
                for (key, value) in facts {
                    object.insert(key.clone(), value.clone());
                }
            }
        }
        frame.bodies.push(body);
    }

    // A ranged move's damage is its projectile. Projectiles are excluded from
    // actor-generic queries (`ProjectileGameplay` keeps them out), so query
    // them by name.
    let mut shots = world.query::<(
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::platformer::projectile::ProjectileGameplay,
        Option<&ambition_platformer2d::projectiles::ProjectileOwner>,
        // The stable identity: arrival order is ECS query order, and
        // recordings must compare byte-for-byte.
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
        // Whose shot, in the same vocabulary as bodies and strikes. An
        // unowned shot is a hazard; it does not belong to the target.
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
            // Kept for readers written before roles existed. The role is the
            // authority; this is its projection.
            "subject_owned": role.is_subjects(),
        }));
    }

    // The strikes, from the semantic view. `CombatGeometryView` puts every
    // live strike into world space with the same `Hitbox::world_volume` the
    // resolver uses, and `combat_observation` writes the row. This file does
    // not resolve a volume; `check_absence_contracts.py` enforces that.
    frame.hitboxes = observation.strikes.clone();
    frame.contacts = observation.contacts.clone();

    // Sort by stable identity before writing. Row order is Bevy query order
    // (archetype order), which changes with component composition, so
    // byte-stable output needs a sort.
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
/// A scenario parameter: a move against a fighter that walks into it is a
/// different measurement from one against a fighter that stands still. The
/// take records which one was used.
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
/// A take that starts after the previous take knocked a fighter off the
/// stage measures nothing: the press goes to a body that is not there. Only a
/// re-seat fixes that. The target is an argument, so the two bodies can be
/// different characters.
fn reseat(app: &mut App, character: &str, target: &str, behavior: TargetBehavior) -> bool {
    let previous_scope = app
        .world()
        .resource::<ambition_platformer2d::actor::ActiveSessionScope>()
        .current();
    let roster = match behavior {
        // The stand-still brain is a real driver. A CPU seat with no brain
        // profile is refused at preparation, so name the policy.
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
    // Staging is a postcondition, not a duration. 240 updates is the
    // ceiling; the loop ends when the new match is ready.
    //
    // The session is the identity, not the name. The outgoing cast is still
    // standing when a new roster publishes, and a re-seat of the same fighter
    // matches the same name. So wait for `ActiveSessionScope` to be present
    // and different from the one seen on entry, and for both seats to hold
    // the requested ids.
    for _ in 0..240 {
        app.update();
        let scope = app
            .world()
            .resource::<ambition_platformer2d::actor::ActiveSessionScope>()
            .current();
        if scope.is_some()
            && scope != previous_scope
            && move_exercise::seat_character(app, 0).as_deref() == Some(character)
            && move_exercise::seat_character(app, 1).as_deref() == Some(target)
        {
            return true;
        }
    }
    false
}

/// This fighter's repertoire, as the host prepared it.
fn moveset_of(app: &mut App, character: &str) -> Option<ambition_entity_catalog::MovesetContract> {
    app.world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .and_then(|registry| registry.get(character))
        .and_then(|prepared| prepared.kit.projectable_moveset())
        .cloned()
}

/// Does this move author offence: a strike volume, or an event that fires?
///
/// This reads the authoring, not the recording, so it can check the
/// recording.
///
/// A move can fire through `MoveEventKind::Effect` (a technique key with
/// params, like `smash.steered_bolt` and every throw) as well as `Ranged`.
/// This tool cannot interpret a technique, so the predicate abstains (returns
/// true) on an `Effect`. The refusal it feeds claims a move cannot produce
/// offence, and that claim is unprovable for a technique. `TechniqueOffer`
/// says where a technique is delivered, not whether it deals damage.
fn authors_offense(spec: &ambition_entity_catalog::MoveSpec) -> bool {
    spec.windows.iter().any(|w| !w.volumes.is_empty())
        || spec.events.iter().any(|e| {
            matches!(
                e.kind,
                ambition_entity_catalog::MoveEventKind::Ranged
                    | ambition_entity_catalog::MoveEventKind::Effect(_)
            )
        })
}

/// The absolute simulation tick, which causal facts are stamped with.
///
/// This is the join key between a take's frames (an index into one exercise)
/// and causal facts.
fn sim_tick(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

/// The causal inspector, when this build carries it.
///
/// The engine already announces why a hit resolved as it did (ignored,
/// blocked, armored, wallet-shielded, damaged), and the monolith turns those
/// into facts with a cause chain. This installs that inspector instead of
/// inventing hit events.
///
/// Off without the `causal` feature. A plain build records geometry and
/// consequences, and `AVAILABLE` says the causal array is unavailable.
#[cfg(feature = "causal")]
mod causal_trace {
    /// This build installs the causal recorder, so an empty `causal` array
    /// means it ran and matched nothing — not that it was missing.
    pub const AVAILABLE: bool = true;

    use bevy::prelude::App;

    pub fn arm(app: &mut App) {
        use ambition_platformer2d::causal::{domains, CausalRecording, RecordingPolicy};
        app.add_plugins(ambition_platformer2d::causal::CausalPlugin);
        if let Some(mut log) = app.world_mut().get_resource_mut::<CausalRecording>() {
            // DAMAGE is the resolution vocabulary; MOVESET is what the move
            // was doing. Other domains would bury both.
            log.set_policy(RecordingPolicy::only([domains::DAMAGE, domains::MOVESET]));
        }
    }

    /// Forget everything before this take. The log is a ring buffer shared by
    /// the whole run, so old facts would be credited to the wrong move.
    pub fn clear(app: &mut App) {
        use ambition_platformer2d::causal::CausalRecording;
        if let Some(mut log) = app.world_mut().get_resource_mut::<CausalRecording>() {
            log.clear();
        }
    }

    /// Every fact the log holds, in the shape the artifact writes.
    pub fn drain(app: &mut App) -> Vec<serde_json::Value> {
        use ambition_platformer2d::causal::CausalRecording;
        let Some(log) = app.world().get_resource::<CausalRecording>() else {
            return Vec::new();
        };
        log.facts()
            .map(|fact| {
                serde_json::json!({
                    // The absolute tick, which the frames also carry.
                    "sim_tick": fact.tick,
                    "domain": fact.domain.0,
                    "kind": fact.detail.kind,
                    "summary": fact.detail.summary,
                    // A stable id where the publisher had one. `entity:N`
                    // means the domain has no stable id yet; do not join on it.
                    "subject": fact.subject.as_ref().map(|s| s.to_string()),
                    "participant": fact.participant,
                    // What this fact followed from: this makes the log a chain.
                    "cause": fact.cause.map(|id| id.0),
                    "id": fact.id.0,
                    // A resimulated tick is not its original. Rollback can run a
                    // tick more than once with different facts; keep them apart.
                    "execution": fact.execution.to_string(),
                    "attempt": fact.attempt,
                    "fields": fact
                        .detail
                        .fields
                        .iter()
                        .map(|(name, value)| (name.to_string(), value.to_string()))
                        .collect::<std::collections::BTreeMap<_, _>>(),
                })
            })
            .collect()
    }
}

/// Without the feature there is no inspector to install.
///
/// The take still carries `causal: []`, so [`AVAILABLE`](causal_trace::AVAILABLE)
/// is written beside it to say whether the recorder existed.
#[cfg(not(feature = "causal"))]
mod causal_trace {
    use bevy::prelude::App;

    /// This build installs no causal recorder, so `causal: []` means unavailable.
    pub const AVAILABLE: bool = false;

    pub fn arm(_: &mut App) {}
    pub fn clear(_: &mut App) {}
    pub fn drain(_: &mut App) -> Vec<serde_json::Value> {
        Vec::new()
    }
}
/// Whether the subject's own strikes were inside the target, and how close the
/// nearest one came.
///
/// The verdict is the engine's `overlaps`: each strike row lists
/// `CombatVolume::intersects` against the target's hurtboxes, the call
/// gameplay uses. Do not derive it from AABBs: a bounds comparison can claim
/// contact the engine denied, and the body box is the wrong subject.
///
/// The bounds gap is a separate diagnostic: it answers "how far short did it
/// fall", which `overlaps` does not.
fn target_reach(frames: &[serde_json::Value]) -> (bool, Option<(f32, f32)>) {
    let num = |v: &serde_json::Value, i: usize| v[i].as_f64().map(|f| f as f32);
    let mut best: Option<(f32, f32)> = None;
    let mut overlapped = false;
    for frame in frames {
        let target = frame["bodies"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|b| b["role"].as_str() == Some("target"));
        let Some(target) = target else { continue };
        let (Some(tx), Some(ty), Some(thx), Some(thy)) = (
            num(&target["pos"], 0),
            num(&target["pos"], 1),
            num(&target["half"], 0),
            num(&target["half"], 1),
        ) else {
            continue;
        };
        for hit in frame["hitboxes"].as_array().into_iter().flatten() {
            if hit["subject_owned"].as_bool() != Some(true) {
                continue;
            }
            let (Some(hx), Some(hy), Some(hhx), Some(hhy)) = (
                num(&hit["pos"], 0),
                num(&hit["pos"], 1),
                num(&hit["half"], 0),
                num(&hit["half"], 1),
            ) else {
                continue;
            };
            // Signed, not clamped: negative is how far the bounds overlap,
            // positive is how far short they fell.
            let gap = ((hx - tx).abs() - (hhx + thx), (hy - ty).abs() - (hhy + thy));
            // The verdict is the engine's `overlaps` (against hurtboxes), not
            // this arithmetic. The gap is a diagnostic only: a convex blade
            // whose bounding box overlaps the target's may not touch it.
            if hit["overlaps"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|victim| victim.as_str() == target["id"].as_str())
            {
                overlapped = true;
            }
            // Closest by the axis that is furthest out: 2 px short in x and
            // 60 px short in y is a vertical miss.
            let worst = |g: (f32, f32)| g.0.max(g.1);
            if best.is_none_or(|b| worst(gap) < worst(b)) {
                best = Some(gap);
            }
        }
    }
    (overlapped, best)
}

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
        // The absolute tick, so a causal fact can be placed beside this frame.
        // The frame index is how far into the exercise; a reader needs both.
        "sim_tick": sim_tick(app),
        "bodies": frame.bodies,
        "hitboxes": frame.hitboxes,
        "projectiles": frame.projectiles,
        "contacts": frame.contacts,
        "move": frame.move_id,
        "move_instance": frame.move_instance,
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
    // Handle `--help` before the app boots.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    // An unknown flag is a refusal. Otherwise a typo like `--character`
    // silently records the default fighter.
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
                    | "--chain"
                    | "--chain-at"
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
    // A mirror match is the default: it keeps geometry comparable across the
    // grid. Name a target to make the two bodies visibly different.
    let target = arg("--target");
    // A name that matches nothing is a refusal, as for `--verbs`.
    let chain = arg("--chain").map(|name| match move_exercise::verb_named(&name) {
        Some(verb) => verb,
        None => {
            eprintln!(
                "moveset_takes: '{name}' is not a verb this exercise can perform.\n\
                 known: {}",
                VERBS.iter().map(|v| v.verb).collect::<Vec<_>>().join(", ")
            );
            std::process::exit(2);
        }
    });
    let chain_at: usize = match arg("--chain-at") {
        None => move_exercise::HOLD_TICKS,
        Some(word) => match word.parse() {
            Ok(tick) => tick,
            Err(_) => {
                eprintln!("moveset_takes: --chain-at wants a tick number");
                std::process::exit(2);
            }
        },
    };
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
    // A name that matches nothing is a refusal, not an empty run.
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

    // In the compose hook: a `NoWindow` build finishes its plugins before
    // returning, and Bevy 0.19 panics on `add_plugins` after that. See
    // `build_visible_app_with`.
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |app| {
            app.add_plugins(bevy::log::LogPlugin::default());
        },
    );
    // This tool owns the clock from here. Otherwise the rollback host
    // advances from a wall-clock accumulator, so one `app.update()` runs zero,
    // one, or several sim ticks, and runs do not repeat. This uses the
    // engine's manual-step contract (`manual_step_period`): the two sim hosts
    // need periods one nanosecond apart. The app's `SimulationHost` resource
    // says which host it has.
    ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    // The inspector, when this build has one. Installed before the first
    // update so every recorded tick has its frame stamp.
    causal_trace::arm(&mut app);
    for _ in 0..30 {
        app.update();
    }

    // `--characters grid` records the whole Smash grid: the set a player can
    // pick, which is what a balance view is about. The registry holds 48
    // prepared characters, but most are NPCs with no moveset.
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
    // Say how long this takes. Each take settles a real match, so a grid run
    // takes tens of minutes.
    eprintln!(
        "[moveset-takes] recording {} character(s): {}",
        who.len(),
        who.join(", ")
    );

    let mut takes = Vec::new();

    for character in &who {
        // A partner, so contact rules and targeting behave as in a match.
        // The target is resolved per subject. Without `--target`, every
        // subject faces the same immortal training dummy, which keeps a grid
        // recording comparable; a mirror match would vary the target too.
        let target = target
            .clone()
            .unwrap_or_else(|| ambition_demo_smash::INSPECTION_TARGET.to_string());
        reseat(&mut app, character, &target, behavior);
        let stage = platforms(&mut app);
        // The fighter's whole repertoire, so a take can check its recording
        // against what the move authors. See `authors_offense`.
        let repertoire = moveset_of(&mut app, character);

        for verb in VERBS.iter().filter(|verb| {
            only.as_ref()
                .is_none_or(|only| only.iter().any(|v| v == verb.verb))
        }) {
            // A fresh match for every take, not only when the settle fails.
            // Otherwise each take depends on the one before: for example
            // `afford_recovery` refuses an up-B whose airtime already spent a
            // recovery. It costs 240 ticks per take.
            //
            // An empty stage is a failure, not a quiet stage: a route that did
            // not come up would record a whole moveset against nobody.
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
            // The scenario comes from the move. One geometry (walk toward,
            // jump, press) cannot measure a move that points elsewhere. See
            // `move_exercise::Staging`.
            let staging = move_exercise::staging_for(verb);
            // Spacing before posture: walk to close the gap on the ground,
            // then an aerial verb takes off from there.
            let (closed, asked) = move_exercise::stage_for_press(&mut app, verb, spacing);
            if closed == Some(false) {
                println!(
                    "[take] {character:<24} {:<16} WARNING - could not close to {} px ({} staging); \
                     this take records the gap it reached",
                    verb.verb,
                    asked.unwrap_or_default(),
                    staging.as_str()
                );
            }
            // One preparation, shared with `moveset_render`. See
            // `move_exercise::prepare`.
            let prepared = move_exercise::prepare(&mut app, verb);
            if !prepared {
                println!(
                    "[take] {character:<24} {:<16} WARNING - could not be airborne at the \
                     press; this take records the GROUNDED answer to that button",
                    verb.verb
                );
            }
            // Then descend into range. `prepare` presses near the apex of a
            // jump, which is over a grounded target's head. A short-hop aerial
            // that meets a grounded opponent on the way down is the shipped use.
            // See `move_exercise::descend_to_meet`. The renderer does not do
            // this: it wants a clean pose, and the apex is where the pose reads.
            if prepared && !move_exercise::stage_airborne(&mut app, verb) {
                println!(
                    "[take] {character:<24} {:<16} WARNING - could not fall into range of the \
                     target before the press; this take records the press from where she was",
                    verb.verb
                );
            }
            // After the re-seat and before the press. Roles are entity
            // identities, and every re-seat spawns new bodies.
            let roles = ScenarioRoles::from_seats(app.world_mut(), 0, 1);
            // Measure at the press, as the name says. A connect launches the
            // target, so the gap after the take is larger.
            let spacing_at_press = move_exercise::gap_to_seat(&mut app, 1).map(f32::abs);
            // The log is a ring buffer shared by the whole run; clear it so
            // one move's consequences are not credited to another.
            causal_trace::clear(&mut app);
            let mut frames: Vec<serde_json::Value> = Vec::new();
            // Every body this take could not identify, collected across its ticks.
            let mut unidentified: std::collections::BTreeSet<String> = Default::default();
            let facing = move_exercise::facing_of(&mut app);
            // The schedule is `move_exercise::action_frame`; nothing else
            // decides what the player does. `chained_frame` equals
            // `action_frame` before the hand-off, so a chain presses exactly
            // what a single take presses up to that tick.
            let frame_at = |tick: usize| match chain {
                Some(second) => move_exercise::chained_frame(verb, second, chain_at, tick, facing),
                None => move_exercise::action_frame(verb, tick, facing),
            };
            step(&mut app, frame_at(0));
            // The press tick is frame zero. `ResolvedAttackGesture::pressed`
            // is set only on the press tick, so a later start would lose the
            // gesture on every frame.
            unidentified.extend(record(&mut app, &roles, &mut frames));
            for tick in 1..TAKE_TICKS {
                step(&mut app, frame_at(tick));
                unidentified.extend(record(&mut app, &roles, &mut frames));
            }

            // A take with an unidentified body is not canonical, so it is not
            // written: its row order would be query order, and the bundle joins
            // on `SimId`. The message names the body, so fix its spawn site.
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
            // How many moves started. A move cancelled into itself records one
            // name and one unbroken run, so key on `(move, instance)`.
            let move_starts = frames
                .windows(2)
                .filter(|w| {
                    let key = |f: &serde_json::Value| {
                        (
                            f["move"].as_str().map(str::to_string),
                            f["move_instance"].as_u64(),
                        )
                    };
                    let (prev, here) = (key(&w[0]), key(&w[1]));
                    here.0.is_some() && here != prev
                })
                .count()
                + usize::from(frames.first().is_some_and(|f| !f["move"].is_null()));
            let rode = frames.iter().any(|f| !f["riding"].is_null());
            // The subject's own output. The frame keeps everything for the
            // viewer; the move is credited only with its owner's output.
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
            // The take checks its own ownership. The CPU opponent swings and
            // fires, so without ownership its offence would count as the
            // subject's.
            //
            // The independent check is the authoring: a move with no volumes
            // and no firing events cannot produce offence, so a nonzero count
            // means another body's output was credited. This refuses instead
            // of warning, because people tune fighters from these numbers.
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
            // Per take, not per frame, so scrubbing does not make a rising
            // fighter look still while the world slides.
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

            // The take says whether it reached the move it drove. The
            // horizontal aim settle in `move_exercise::prepare` makes a back
            // input produce a back aerial, not a turnaround.
            //
            // One vocabulary with the renderer: `Outcome` has four answers,
            // and none collapses into another (an unbound verb is not a
            // success). Otherwise this panel and `moveset_render` can disagree.
            let intended = move_exercise::intended_move(&mut app, character, verb.verb);
            let verdict = move_exercise::outcome(prepared, intended.as_deref(), &moves);
            let reached = verdict.reached();
            // Why a miss missed (see `target_reach`). Without this, live shapes
            // with no contact read as a hitbox that is too small.
            let (overlapped, gap) = target_reach(&frames);
            let landed = frames.iter().any(|f| {
                f["contacts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|c| c["victim_role"].as_str() == Some("target"))
            });
            println!(
                "[take] {character:<24} {:<16} moves={:?} starts={move_starts} hitboxes<={live} shots<={shots} rode={rode}{}{}",
                verb.verb,
                moves,
                // Say it on the line, not only in the file: "no contact"
                // without a reason sends the reader to the hitbox.
                match (live > 0, landed, overlapped, gap) {
                    (true, false, false, Some((x, y))) => format!(
                        " OUT OF REACH: shapes never overlapped the target (closest {:.0}x, {:.0}y px) -- the SCENARIO, not the hitbox",
                        x.max(0.0),
                        y.max(0.0)
                    ),
                    (true, false, true, _) =>
                        " NO CONTACT THOUGH THE SHAPES OVERLAPPED -- the engine, not the scenario".to_string(),
                    _ => String::new(),
                },
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
            // Who was in this scenario, by name and identity, so a frame or
            // screenshot does not need a seat convention.
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
                // The premise: a passive target produces no offence by
                // construction, so `opponent_output: 0` proves nothing then.
                "target_behavior": behavior.as_str(),
                // The scheduling policy is simulation-defining input. Publish
                // its name so reports and cache identity need not infer it.
                "hold_policy": "move_exercise_default",
                // The scenario's staging. Takes staged differently do not
                // compare, and a reader needs the geometry behind a miss.
                "staging": staging.as_str(),
                // What was requested beside what the engine did, so the take
                // shows whether the engine took the second verb early, late,
                // or not at all.
                "chain": chain.map(|second| serde_json::json!({
                    "verb": second.verb,
                    "label": second.label,
                    "at": chain_at,
                })),
                "requested_spacing": spacing,
                "spacing_at_press": spacing_at_press,
                "view": [x0, y0, x1, y1],
                "platforms": stage,
                // What the engine did. A take that reached no move says so.
                "moves_seen": moves.iter().cloned().collect::<Vec<_>>(),
                // Two starts with one name is a cancel into the same move.
                "move_starts": move_starts,
                "rode_a_mount": rode,
                "max_live_hitboxes": live,
                // The premise behind a miss. `overlapped_target: false` is a
                // fact about the scenario; `true` with no contact is a fact
                // about the engine.
                "reach": {
                    // The engine's answer: `CombatVolume::intersects` against
                    // the target's hurtboxes, from each strike's `overlaps`.
                    "overlapped_target": overlapped,
                    // Bounds, signed, diagnostic only. Negative is overlap
                    // depth, positive is shortfall. It does not decide contact.
                    "closest_bounds_gap_px": gap.map(|(x, y)| vec![x, y]),
                    "contacted_target": landed,
                },
                "max_live_projectiles": shots,
                // The premise: with zero here, nothing else was on the stage to
                // be miscredited, so the take could not detect contamination.
                "opponent_output": opponent_output,
                "intended_move": intended,
                "reached_intended_move": reached,
                // Which kind of "no": an unbound verb, a press that reached
                // another move, or a posture that could not be set up.
                "outcome": verdict.as_str(),
                "prepared": prepared,
                // `causal: []` with `causal_resolution: false` means no
                // recorder in this build; with `true` it means the recorder
                // ran and matched nothing.
                "causal": causal_trace::drain(&mut app),
                "capabilities": serde_json::json!({
                    "causal_resolution": causal_trace::AVAILABLE,
                }),
                "frames": frames,
            }));
        }
    }

    let bundle = serde_json::json!({
        // v2: every body, strike, and shot has a scenario role; bodies carry
        // runtime hurtboxes and a move clock; the take names its subject and
        // target. The additions are additive, so a v1 reader still draws, but
        // a reader that needs roles must check the schema.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame: a target at the origin and one subject-owned strike, with the
    /// engine's `overlaps` verdict supplied by the caller.
    fn frame(
        strike_pos: (f32, f32),
        strike_half: (f32, f32),
        overlaps: &[&str],
    ) -> serde_json::Value {
        serde_json::json!({
            "bodies": [{
                "role": "target", "id": "the_target",
                "pos": [0.0, 0.0], "half": [10.0, 10.0],
            }],
            "hitboxes": [{
                "subject_owned": true,
                "pos": [strike_pos.0, strike_pos.1],
                "half": [strike_half.0, strike_half.1],
                "overlaps": overlaps,
            }],
        })
    }

    /// The shipped author table, parsed the way the pack parses it.
    ///
    /// Real content, not a hand-built `MoveSpec`: the predicate is a claim about
    /// the authored population.
    fn shipped_author_moves() -> Vec<ambition_entity_catalog::MoveSpec> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ambition_content/assets/data/movesets/director.ron"
        );
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|why| panic!("the shipped author table is readable ({path}): {why}"));
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(&text)
            .expect("the shipped author table parses");
        doc.entities
            .into_iter()
            .filter_map(|entity| entity.contracts.moveset)
            .flat_map(|moveset| moveset.moves)
            .collect()
    }

    fn shipped(id: &str) -> ambition_entity_catalog::MoveSpec {
        shipped_author_moves()
            .into_iter()
            .find(|spec| spec.id == id)
            .unwrap_or_else(|| panic!("the shipped author table no longer authors `{id}`"))
    }

    /// A move that fires through a technique authors offence.
    ///
    /// `director_train_of_thought` has no volumes and one
    /// `Effect("smash.steered_bolt")`; it must not be classified as hitless.
    #[test]
    fn a_move_that_fires_a_technique_effect_authors_offense() {
        let spec = shipped("director_train_of_thought");
        // The premise: otherwise this arm is about a volume and proves
        // nothing about the `Effect` path.
        assert!(
            spec.windows.iter().all(|w| w.volumes.is_empty()),
            "`director_train_of_thought` now authors a strike volume, so it no \
             longer exercises the effect-only case this arm exists for"
        );
        assert!(
            !spec
                .events
                .iter()
                .any(|e| matches!(e.kind, ambition_entity_catalog::MoveEventKind::Ranged)),
            "`director_train_of_thought` now carries a `Ranged` event, so the OLD \
             predicate would already classify it and this arm is vacuous"
        );
        assert!(
            spec.events
                .iter()
                .any(|e| matches!(e.kind, ambition_entity_catalog::MoveEventKind::Effect(_))),
            "`director_train_of_thought` no longer fires a technique effect"
        );

        assert!(
            authors_offense(&spec),
            "a move whose only output is a technique effect was called hitless, \
             which makes the recorder refuse a take it cannot adjudicate"
        );
    }

    /// The controls. The refusal asserts a move cannot produce offence; if
    /// every move authors offence, the refusal is dead.
    #[test]
    fn the_widened_predicate_still_separates_the_population() {
        let moves = shipped_author_moves();
        assert!(moves.len() > 10, "only {} author moves", moves.len());

        let with_volume = moves
            .iter()
            .find(|spec| spec.windows.iter().any(|w| !w.volumes.is_empty()))
            .expect("some author move authors a strike volume");
        assert!(authors_offense(with_volume), "{}", with_volume.id);

        let silent = moves.iter().find(|spec| {
            spec.windows.iter().all(|w| w.volumes.is_empty()) && spec.events.is_empty()
        });
        match silent {
            Some(spec) => assert!(
                !authors_offense(spec),
                "`{}` authors no volume and no event at all and was still called \
                 offensive — the predicate now says yes to everything and the \
                 refusal it feeds can never fire",
                spec.id
            ),
            // If no author move is silent, this control certifies nothing.
            // Say so instead of passing.
            None => panic!(
                "no move in the shipped author table authors nothing at all, so \
                 this control has no subject — point it at a table that does, or \
                 the widened predicate has no witnessed negative case"
            ),
        }
    }

    /// The regression: a rotated blade's bounding box sits over the target while
    /// the hull misses entirely, and a bounds comparison calls that a contact the
    /// engine refused.
    #[test]
    fn deeply_penetrating_bounds_are_not_a_contact_when_the_engine_says_no() {
        // Bounds overlap by 16 px on each axis — far past any tolerance.
        let (overlapped, gap) = target_reach(&[frame((2.0, 2.0), (12.0, 12.0), &[])]);
        assert!(
            !overlapped,
            "the engine listed no overlapping victim, so this is a MISS however \
             deep the bounding boxes sit inside each other (gap {gap:?})"
        );
        let gap = gap.expect("the bounds diagnostic is still reported");
        assert!(
            gap.0 < -1.0 && gap.1 < -1.0,
            "the premise: the BOUNDS really do penetrate, or this test would pass \
             for the uninteresting reason — {gap:?}"
        );
    }

    /// And the other direction, so the verdict is not always false.
    #[test]
    fn the_engines_overlap_verdict_is_what_reports_a_contact() {
        let (overlapped, _) = target_reach(&[frame((2.0, 2.0), (12.0, 12.0), &["the_target"])]);
        assert!(overlapped, "the engine named the target as overlapped");
    }

    /// A strike inside another body is not inside the target. The take seats
    /// a real opponent, so `overlaps` can name a body this is not about.
    #[test]
    fn overlapping_a_different_body_is_not_overlapping_the_target() {
        let (overlapped, _) = target_reach(&[frame((2.0, 2.0), (12.0, 12.0), &["somebody_else"])]);
        assert!(!overlapped);
    }

    #[test]
    fn repeated_character_takes_start_in_new_sessions_and_accept_attacks() {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        ambition_platformer2d::sim::enable_manual_stepping(&mut app);
        for _ in 0..30 {
            app.update();
        }
        let verb = move_exercise::verb_named("attack_forward").unwrap();
        for _ in 0..3 {
            let previous = app
                .world()
                .resource::<ambition_platformer2d::actor::ActiveSessionScope>()
                .current();
            assert!(reseat(
                &mut app,
                "performer",
                "sandbag_infinite",
                TargetBehavior::Passive
            ));
            let current = app
                .world()
                .resource::<ambition_platformer2d::actor::ActiveSessionScope>()
                .current();
            assert_ne!(previous, current);
            assert!(settle(&mut app));
            assert!(move_exercise::prepare(&mut app, verb));
            let facing = move_exercise::facing_of(&mut app);
            let mut played = false;
            for tick in 0..30 {
                step(&mut app, move_exercise::action_frame(verb, tick, facing));
                played |= move_exercise::playing_move(&mut app).as_deref()
                    == Some("performer_tilt_forward");
            }
            assert!(played, "the new match must accept the attack");
        }
    }

    /// An aerial take must meet a grounded target.
    ///
    /// `prepare` presses near the apex, above a grounded target. Waiting until
    /// she is "low enough" returns at once, because she passes that band while
    /// rising. `descend_to_meet` requires her to be falling; this test tells
    /// the two apart.
    #[test]
    fn an_aerial_take_falls_into_range_of_a_grounded_target() {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        ambition_platformer2d::sim::enable_manual_stepping(&mut app);
        for _ in 0..30 {
            app.update();
        }
        let verb = move_exercise::verb_named("attack_air_down").unwrap();
        assert!(verb.airborne, "the premise: this verb is an aerial");
        assert!(reseat(
            &mut app,
            "performer",
            "sandbag_infinite",
            TargetBehavior::Passive
        ));
        assert!(settle(&mut app));
        move_exercise::approach(&mut app, 32.0);
        assert!(move_exercise::prepare(&mut app, verb));
        // The premise: she is above the target's head after `prepare`, so
        // this tests the descent, not a fixture that started in range.
        assert!(
            move_exercise::descend_to_meet(&mut app, 1, move_exercise::AERIAL_LEAD_PX),
            "she never fell into range of a body standing on the floor"
        );

        // Assert the state, not the return value: the boolean is also true
        // for a no-op that returned while she was still rising.
        let (mine, theirs, falling) =
            move_exercise::seat_spans(&mut app, 1).expect("both seats are filled");
        assert!(falling, "she was still RISING when the press was thrown");
        assert!(
            mine.1 >= theirs.0 - move_exercise::AERIAL_LEAD_PX,
            "her feet ({}) never reached the lead above the target's head ({})",
            mine.1,
            theirs.0 - move_exercise::AERIAL_LEAD_PX
        );
    }

    /// A directional aerial is staged where it points.
    ///
    /// A back air puts its box behind her, so walk-toward staging places the
    /// target on the wrong side. This asserts that after staging, the target
    /// is on the side the move covers, not only that `staging_for` returned a
    /// name.
    #[test]
    fn a_back_air_is_staged_with_the_target_behind_her() {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        ambition_platformer2d::sim::enable_manual_stepping(&mut app);
        for _ in 0..30 {
            app.update();
        }
        let verb = move_exercise::verb_named("attack_air_back").unwrap();
        assert_eq!(
            move_exercise::staging_for(verb),
            move_exercise::Staging::Behind,
            "the premise: this verb asks to be staged from behind"
        );
        assert!(reseat(
            &mut app,
            "performer",
            "sandbag_infinite",
            TargetBehavior::Passive
        ));
        assert!(settle(&mut app));

        let facing_before = move_exercise::facing_of(&mut app);
        assert!(
            move_exercise::approach_past(&mut app, 48.0),
            "she never got past it"
        );
        let gap = move_exercise::gap_to_seat(&mut app, 1).expect("both seats are filled");

        // Both halves make it a back air: the target is on the side her box
        // covers, and she still faces the same way. A body that turned round
        // performs a forward air under another name.
        assert_eq!(
            move_exercise::facing_of(&mut app),
            facing_before,
            "she turned round, so the press would place its box toward the target"
        );
        assert_eq!(
            gap.signum(),
            -facing_before.signum(),
            "the target is still in FRONT of her (gap {gap}, facing {facing_before})"
        );
    }
}
