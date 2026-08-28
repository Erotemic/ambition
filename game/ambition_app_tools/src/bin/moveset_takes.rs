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

use ambition_platformer2d::engine_core::ControlFrame;

/// Ticks recorded per take. Long enough for a five-second shark ride to show its
/// shape without every take carrying the tail of an idle stage.
const TAKE_TICKS: usize = 150;
/// The longest a take will wait for the stage to go quiet before the next press.
///
/// ⛔⛔ A FIXED SETTLE IS NOT A SETTLE. Forty-five ticks was less than the
/// admiral's forward smash owes, so `smash_up`, `smash_down` and `special_up`
/// each landed inside the previous move's recovery, were dropped, and were
/// reported as moves that produced nothing — three false findings from one
/// constant. The condition is "the body is idle and standing", which the world
/// already publishes, so the wait ASKS instead of counting.
/// ⛔ ABOVE THE LONGEST RIDE. A 240-tick limit is four seconds and the shark
/// carries its rider for five, so the take after the up-B started while the
/// admiral was still airborne on a mount and reported two moves as producing
/// nothing. A settle that gives up before the previous take finishes is a
/// settle that manufactures findings.
const SETTLE_LIMIT: usize = 480;

/// One press, as the genre spells it.
struct Verb {
    /// The repertoire verb this drives, which is the key the UI files it under.
    verb: &'static str,
    label: &'static str,
    axis_x: f32,
    axis_y: f32,
    button: Button,
    /// Jump first, and wait for the apex. An aerial pressed from the ground
    /// reaches the grounded chain instead, and reports the wrong move.
    airborne: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Attack,
    Smash,
    Special,
    Grab,
    Taunt,
}

/// The full press table. Directions follow the engine's own convention:
/// `axis_y = -1` is UP (the same value `shark_ride_probe` drives for the up-B).
const VERBS: &[Verb] = &[
    Verb {
        verb: "attack",
        label: "Jab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_forward",
        label: "F-tilt",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_up",
        label: "U-tilt",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_down",
        label: "D-tilt",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "smash_forward",
        label: "F-smash",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_up",
        label: "U-smash",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_down",
        label: "D-smash",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "attack_air",
        label: "N-air",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_forward",
        label: "F-air",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_back",
        label: "B-air",
        axis_x: -1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_up",
        label: "U-air",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_down",
        label: "D-air",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "special",
        label: "Neutral B",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_forward",
        label: "Side B",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    // ⭐ THE UP-B IS RECORDED FROM THE AIR, which is the only place it is the
    // move Jon is asking about. A grounded up-B answers the same press and shows
    // none of the recovery.
    Verb {
        verb: "special_up",
        label: "Up B (airborne)",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "special_down",
        label: "Down B",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_air_down",
        label: "Down B (air)",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "grab",
        label: "Grab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Grab,
        airborne: false,
    },
    Verb {
        verb: "taunt",
        label: "Taunt",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Taunt,
        airborne: false,
    },
];

/// How far the stick goes for a TILT.
///
/// ⛔⛔ THIS IS THE DIFFERENCE BETWEEN A TILT AND A SMASH, and driving `1.0`
/// silently recorded the smash for every directional tilt — four takes that
/// looked like working data. `resolve_attack_gesture` arms a flick at
/// `flick_threshold` 0.8 and calls a press that matches a recent flick a SMASH;
/// a magnitude above `directional_deadzone` 0.5 and below 0.8 is directional and
/// never arms one, which is exactly what a tilt input is.
const TILT_AXIS: f32 = 0.65;

/// The press, aimed relative to the body's CURRENT facing.
///
/// ⛔⛔ `axis_x` IS "FORWARD", NOT "RIGHT", and driving it as a world direction
/// recorded the forward air for every back-air take. `attack_dir_from_axis`
/// resolves a press against `BodyKinematics::facing`, so a fighter that happened
/// to be pointing left answered a left press with FORWARD — correctly, and the
/// take reported it as a back-air. The facing is a fact the world publishes;
/// this reads it rather than assuming the body starts pointing right.
fn press(v: &Verb, edge: bool, facing: f32) -> ControlFrame {
    let reach = if v.button == Button::Smash {
        1.0
    } else {
        TILT_AXIS
    };
    let mut frame = ControlFrame {
        axis_x: v.axis_x * reach * facing.signum(),
        axis_y: v.axis_y * reach,
        ..Default::default()
    };
    match v.button {
        Button::Attack => {
            frame.attack_pressed = edge;
            frame.attack_held = true;
        }
        Button::Smash => {
            frame.attack_pressed = edge;
            frame.attack_held = true;
            // The gesture that tells a tilt from a smash. Without it every
            // "smash" take records the tilt, which looks like working data.
            frame.attack_strong_hint = true;
        }
        Button::Special => {
            frame.special_pressed = edge;
            frame.special_held = true;
        }
        Button::Grab => frame.grab_pressed = edge,
        Button::Taunt => frame.taunt_pressed = edge,
    }
    frame
}

/// Everything one recorded tick says.
#[derive(Default)]
struct Frame {
    bodies: Vec<serde_json::Value>,
    hitboxes: Vec<serde_json::Value>,
    projectiles: Vec<serde_json::Value>,
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
}

const USAGE: &str = "\
moveset_takes — drive real control frames through the engine and record what it did.

USAGE:
    moveset_takes [--characters ID,ID] [--out PATH]

OPTIONS:
    --characters ID,ID   comma-separated catalog ids to record
                         `grid` (or `all`) records every fighter on the smash
                         grid: 21 fighters at ~1m17 each is about 27 MINUTES
                         (measured 2026-08-27, after the settle stopped
                         serialising a frame to read three booleans)
                         [default: npc_pirate_admiral]
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
fn sample(world: &mut World, subject_seat: usize) -> Frame {
    let mut frame = Frame::default();

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
    let sheet_keys = SHEET_KEYS.get_or_init(|| world
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
        .unwrap_or_default());

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

    let mut owner_pos = std::collections::HashMap::new();
    for (entity, pos, ..) in &rows {
        owner_pos.insert(*entity, *pos);
    }
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
    let subject_entity = rows
        .iter()
        .find(|(_, _, _, _, _, seat, ..)| *seat == Some(subject_seat))
        .map(|(entity, ..)| *entity);

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
        let subject = *seat == Some(subject_seat);
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
        frame.bodies.push(serde_json::json!({
            "pos": [pos.0, pos.1],
            "half": [half.0, half.1],
            "seat": seat,
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
        }));
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
        frame.projectiles.push(serde_json::json!({
            "id": sim_id,
            "pos": [pos.x, pos.y],
            "vel": [vel.x, vel.y],
            "half": [size.x * 0.5, size.y * 0.5],
            "damage": damage,
            // A shot with no owner belongs to nobody in particular — a hazard,
            // a stage emitter — and is not the subject's either way.
            "subject_owned": owner.is_some() && owner == subject_entity,
        }));
    }

    // ⭐⭐ A HITBOX HAS ITS OWN IDENTITY. `advance_move_playback` inserts
    // `SimId::strike_volume(owner, move, window, volume)` on the volume entity
    // itself, and `StrikeRank { window, volume }` beside it. Sorting by owner +
    // position + damage still tied whenever one owner's two volumes shared them
    // — a multi-hit's mirrored pair does — and a tie falls back to ECS order,
    // which is the thing being canonicalised.
    let mut hitboxes = world.query::<(
        &ambition_platformer2d::combat::strike::Hitbox,
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
        Option<&ambition_platformer2d::combat::moveset::StrikeRank>,
    )>();
    let boxes: Vec<_> = hitboxes
        .iter(world)
        .map(|(hitbox, id, rank)| {
            (
                hitbox.clone(),
                id.map(|id| id.as_str().to_string()),
                rank.map(|r| (r.window, r.volume)),
            )
        })
        .collect();
    for (hitbox, strike_id, rank) in boxes {
        let anchor = owner_pos.get(&hitbox.owner).copied().unwrap_or((0.0, 0.0));
        // The SAME resolution the combat runtime uses, so a recorded box is the
        // box that could hit somebody rather than a redrawn approximation.
        let at = ambition_platformer2d::engine_core::Vec2::new(anchor.0, anchor.1);
        let aabb = hitbox.world_aabb(at);
        // ⛔⛔ THE REAL SHAPE, not the box around it. `world_aabb` sits directly
        // beside `world_volume` and I reached for the wrong one, so a rotated
        // box, a disc and a convex arc were all recorded as the axis-aligned
        // rectangle that CONTAINS them — which for a sweeping arc is a great
        // deal larger than the thing that can actually hit you, and is the
        // difference between a diagram and a decoration.
        //
        // ⭐ THE AABB STAYS BESIDE IT. It is the broad phase the engine itself
        // uses, a viewer can draw it without knowing any shape, and keeping both
        // means an old take still renders.
        let shape = match hitbox.world_volume(at) {
            ambition_platformer2d::engine_core::CombatVolume::Aabb(_) => {
                serde_json::json!({ "kind": "aabb" })
            }
            ambition_platformer2d::engine_core::CombatVolume::Obb {
                center,
                half,
                rotation,
            } => serde_json::json!({
                "kind": "obb",
                "center": [center.x, center.y],
                "half": [half.x, half.y],
                "rotation": rotation,
            }),
            ambition_platformer2d::engine_core::CombatVolume::Circle { center, radius } => {
                serde_json::json!({
                    "kind": "circle",
                    "center": [center.x, center.y],
                    "radius": radius,
                })
            }
            ambition_platformer2d::engine_core::CombatVolume::Convex { points, .. } => {
                serde_json::json!({
                    "kind": "convex",
                    "points": points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
                })
            }
        };
        frame.hitboxes.push(serde_json::json!({
            "pos": [(aabb.min.x + aabb.max.x) * 0.5, (aabb.min.y + aabb.max.y) * 0.5],
            "half": [(aabb.max.x - aabb.min.x) * 0.5, (aabb.max.y - aabb.min.y) * 0.5],
            "shape": shape,
            "damage": hitbox.damage,
            // Already read for the anchor above and then thrown away, which is
            // how the opponent's swings got counted as the subject's.
            "subject_owned": Some(hitbox.owner) == subject_entity,
            // ⛔ WHOSE STRIKE, by stable identity. A hitbox has no `SimId` of its
            // own — it is a volume, not a body — so it is keyed by the body that
            // threw it, which is what makes the sort canonical rather than
            // merely usually-stable.
            // THE VOLUME'S OWN identity. ⛔ THE FALLBACK IS OWNER-QUALIFIED: a
            // bare `strike(window 1, volume 0)` names one volume of EVERY
            // unidentified owner, and an `id` that identifies two different
            // things is worse than an absent one.
            "id": strike_id.clone().or_else(|| {
                let owner = rows
                    .iter()
                    .find(|(e, ..)| *e == hitbox.owner)
                    .and_then(|row| row.14.clone())?;
                rank.map(|(window, volume)| format!("{owner}/strike/w{window}/v{volume}"))
            }),
            // Provenance, not identity: whose swing this is.
            "owner_id": rows
                .iter()
                .find(|(e, ..)| *e == hitbox.owner)
                .and_then(|row| row.14.clone()),
        }));
    }

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
    // ⛔⛔ POSITION AND DAMAGE ARE NOT AN IDENTITY. Two volumes of one move can
    // share both — a multi-hit's mirrored pair does — and ties then fall back to
    // ECS query order, which is the thing being canonicalised. The owner's
    // stable id leads the key, and geometry only breaks ties within one owner.
    // ⭐ BY THE VOLUME'S OWN ID. Geometry only breaks ties among strikes that
    // carry no derived identity at all.
    frame.hitboxes.sort_by(|a, b| {
        let key = |v: &serde_json::Value| {
            (
                v["id"].as_str().unwrap_or("").to_string(),
                v["owner_id"].as_str().unwrap_or("").to_string(),
                v["pos"][0].as_f64().unwrap_or_default().to_bits(),
                v["pos"][1].as_f64().unwrap_or_default().to_bits(),
                v["damage"].as_i64().unwrap_or_default(),
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

/// Put a clean match on the stage.
///
/// ⛔⛔ A TAKE THAT STARTS FROM A CORPSE MEASURES NOTHING. Two takes reported
/// their move as producing nothing because the previous one had knocked the
/// admiral off the stage: the recording showed a body frozen below the floor
/// with `grounded: false` forever, and the press went to somebody who was not
/// there. The settle can detect that state; only a re-seat can fix it.
fn reseat(app: &mut App, character: &str) {
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([character, character]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..240 {
        app.update();
    }
}

/// `verb -> move id` for one character, read from the composed host.
fn verb_table(app: &mut App, character: &str) -> std::collections::BTreeMap<String, String> {
    moveset_of(app, character).map_or_else(Default::default, |set| set.verbs)
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

/// The move a verb is bound to, or `None` when the fighter binds nothing there
/// (an unbound slot is answered by the directional chain, and whatever it
/// reaches is the right answer rather than a mismatch).
fn intended_move<'a>(
    bound: &'a std::collections::BTreeMap<String, String>,
    verb: &str,
) -> Option<&'a str> {
    bound.get(verb).map(String::as_str)
}

fn drive(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
    app.update();
}

/// Jump, and wait until the world says the body has left the ground.
///
/// ⛔ ASK, DO NOT COUNT. A fixed wait recorded `attack_air_down` as `smash_down`
/// and `attack_air_up` as nothing at all, because the press landed on a body the
/// engine still called grounded and the directional chain walked past every
/// aerial. The ground state is a fact the world publishes.
fn ensure_airborne(app: &mut App) -> bool {
    if sample(app.world_mut(), 0).grounded == Some(false) {
        return true;
    }
    drive(
        app,
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    for _ in 0..40 {
        drive(
            app,
            ControlFrame {
                jump_held: true,
                ..Default::default()
            },
        );
        if sample(app.world_mut(), 0).grounded == Some(false) {
            return true;
        }
    }
    false
}

/// Wait for a body that is idle, not riding anything, and standing IF IT CAN.
///
/// ⛔⛔ NOT EVERY FIGHTER CAN STAND. `player_robot_v3` can FLY, and a flying body
/// is never grounded by construction (`integration.rs`: *"a flying body is never
/// grounded — the collision sweep can still find support under a hovering
/// flyer"*). A settle that demanded `grounded` therefore never succeeded for it:
/// every take re-seated, timed out again, and started from whatever the last one
/// left behind — which is why the robot's grounded forward tilt was recorded as
/// its FORWARD AIR and both of its specials as producing nothing. Three findings
/// from one condition that could not hold.
///
/// So the ground is required only of a body that has been seen standing. A
/// fighter that never reports support settles on "idle and unencumbered", which
/// is the strongest true statement available about it.
/// The three facts a settle waits on, without building a frame to read them.
///
/// ⛔⛤ `settle` USED TO CALL `sample`, which queries every body, every hitbox and
/// every projectile and serialises all of it to JSON -- then threw the JSON away
/// to look at three booleans. At up to 480 iterations per take and 19 takes per
/// character that is the dominant cost of a recording, and none of it was the
/// simulation.
fn settle_facts(world: &mut World) -> (bool, bool, bool) {
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        Option<&ambition_platformer2d::engine_core::BodyGroundState>,
        Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
        Option<&ambition_platformer2d::mount::RidingOn>,
    )>();
    for (seat, ground, playing, riding) in q.iter(world) {
        if seat.0 != 0 {
            continue;
        }
        return (
            ground.is_some_and(|g| g.on_ground),
            playing.is_some(),
            riding.is_some(),
        );
    }
    (false, false, false)
}

fn settle(app: &mut App) -> bool {
    let mut ever_stood = false;
    for _ in 0..SETTLE_LIMIT {
        drive(app, ControlFrame::default());
        let (grounded, playing, riding) = settle_facts(app.world_mut());
        ever_stood |= grounded;
        if playing || riding {
            continue;
        }
        if grounded || !ever_stood {
            return true;
        }
    }
    false
}

/// Sample the world and append it to a take.
fn record(app: &mut App, frames: &mut Vec<serde_json::Value>) {
    let frame = sample(app.world_mut(), 0);
    frames.push(serde_json::json!({
        "bodies": frame.bodies,
        "hitboxes": frame.hitboxes,
        "projectiles": frame.projectiles,
        "move": frame.move_id,
        "grounded": frame.grounded,
        "subject_pos": frame.subject_pos.map(|p| vec![p.0, p.1]),
        "subject_vel": frame.subject_vel.map(|v| vec![v.0, v.1]),
        "facing": frame.facing,
        "gesture": frame.gesture,
        "riding": frame.riding,
    }));
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
        .find(|a| *a != "--out" && *a != "--characters")
    {
        eprintln!("moveset_takes: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    let arg = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    let out = arg("--out")
        .unwrap_or_else(|| "tools/ambition_moveset_inspector/data/takes/takes.json".to_string());
    let asked = arg("--characters");

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
        reseat(&mut app, character);
        let stage = platforms(&mut app);
        // The fighter's own verb table, so a take can say which move the press
        // was SUPPOSED to reach rather than only which one came out.
        let bound = verb_table(&mut app, character);
        // And its whole repertoire, so a take can ask what the move it played
        // AUTHORS and check its own recording against that. See
        // `authors_offense`.
        let repertoire = moveset_of(&mut app, character);

        for verb in VERBS {
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
            reseat(&mut app, character);
            let settled = settle(&mut app);
            if !settled {
                println!(
                    "[take] {character:<24} {:<16} WARNING - the stage would not go quiet \
                     even after a re-seat; read this take with that in mind",
                    verb.verb
                );
            }
            if verb.airborne {
                // ⛔⛔ AIRBORNE AT THE PRESS, CHECKED AT THE PRESS. Jumping and
                // then doing anything else is not the same claim: an aim settle,
                // a fast-fall, or simply a short hop meant the body was standing
                // again by the time the button went down, and the take recorded
                // the grounded move under the aerial's name. So the takeoff and
                // the aim settle are one loop that ends only when the body is
                // both airborne and pointing the right way.
                let mut ready = false;
                for _attempt in 0..3 {
                    if !ensure_airborne(&mut app) {
                        continue;
                    }
                    // Settle a HORIZONTAL aim only: a back-air driven the tick
                    // the stick reversed resolves as FORWARD, because
                    // `resolve_attack_gestures` reads `-facing` while a
                    // turnaround runs (the pivot rule, which is correct and is
                    // why a pivot grab needs no move of its own). Holding DOWN
                    // for the same settle would fast-fall back to the floor.
                    if verb.axis_x != 0.0 {
                        for _ in 0..8 {
                            let aim = sample(app.world_mut(), 0).facing.unwrap_or(1.0);
                            drive(
                                &mut app,
                                ControlFrame {
                                    axis_x: verb.axis_x * TILT_AXIS * aim.signum(),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    if sample(app.world_mut(), 0).grounded == Some(false) {
                        ready = true;
                        break;
                    }
                }
                if !ready {
                    println!(
                        "[take] {character:<24} {:<16} WARNING - could not be airborne at the \
                         press; this take records the GROUNDED answer to that button",
                        verb.verb
                    );
                }
            }
            let mut frames: Vec<serde_json::Value> = Vec::new();
            let facing = sample(app.world_mut(), 0).facing.unwrap_or(1.0);
            drive(&mut app, press(verb, true, facing));
            // ⛔ THE PRESS TICK IS FRAME ZERO. `ResolvedAttackGesture::pressed`
            // is set on the press tick and cleared after, so a recording that
            // started one tick later showed `gesture: null` on every frame of
            // every take — the one field that says what the engine understood
            // the input to be, absent from all of them.
            record(&mut app, &mut frames);
            for tick in 1..TAKE_TICKS {
                // A charge move releases when the button comes up. Half the take
                // held, half released, so both the hold and the payoff are on
                // the recording.
                let held = tick < TAKE_TICKS / 4;
                drive(
                    &mut app,
                    if held {
                        press(verb, false, facing)
                    } else {
                        ControlFrame::default()
                    },
                );
                record(&mut app, &mut frames);
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

            // ⛔⛔ THE TAKE SAYS WHETHER IT REACHED THE MOVE IT DROVE. Eighteen
            // of nineteen verbs do; `attack_air_back` does not, and that is an
            // ENGINE finding rather than a driver one — the recorded gesture
            // reads `Forward/Tilt/Airborne`, so the fighter turned to face the
            // back input before the press was read, and the back air is
            // unreachable for every fighter in the cast. Reporting it as "the
            // forward air, under the back air's name" is what a tool that
            // silently relabels its own failures does.
            let intended = intended_move(&bound, verb.verb);
            let reached = intended.is_none_or(|id| moves.contains(id));
            println!(
                "[take] {character:<24} {:<16} moves={:?} hitboxes<={live} shots<={shots} rode={rode}{}",
                verb.verb,
                moves,
                if reached {
                    String::new()
                } else {
                    format!(
                        " MISMATCH: drove {} but the engine played {:?}",
                        intended.unwrap_or("?"),
                        moves
                    )
                }
            );
            takes.push(serde_json::json!({
                "character": character,
                "verb": verb.verb,
                "label": verb.label,
                "seat": 0,
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
                "frames": frames,
            }));
        }
    }

    let bundle = serde_json::json!({
        "schema": "ambition.moveset_takes.v1",
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
