//! Sanic-style demo content home.
//!
//! This crate intentionally depends only on the `ambition` facade crate. It is
//! the E9 engine-for-other-games ORACLE: a second platformer's content is
//! authored entirely through the umbrella surface, never by reaching into a
//! lower `ambition_*` crate or copying `game/ambition_app`'s dependency wall.
//! If authoring a room here needs a type the umbrella does not re-export, that
//! is a real engine leak — and it fails to compile HERE, which is the point.
//!
//! What lives here is the SHOWCASE GEOMETRY (a landmarked momentum speedway
//! with a rideable loop), plus the mode-local rules that make progress legible
//! through milestone SFX. The windowed shell loads the shared Ambition art tree
//! through the public facade, so generated Sanic art and existing parallax /
//! block assets light up without an `ambition_app` dependency.

use ambition::engine_core as ae;
use ambition::prelude::*;
use ambition::world::rooms::RoomSpec;

/// Stable room id for the momentum speedway.
pub const SPEEDWAY_ROOM_ID: &str = "sanic_speedway";

/// The game-MODE tag this demo's rooms carry (decomposition D-C).
///
/// Ambition hosts this demo by loading its rooms alongside its own; a Sanic
/// rules plugin gates its systems on `ambition::runtime::in_mode(SANIC_MODE)`
/// so they sleep everywhere else. [`SanicRulesPlugin`] is that ruleset, and its
/// `hosted()` / `global()` constructor flag is the D-C pattern made real.
pub const SANIC_MODE: &str = "sanic";

/// Authored soundtrack for the standalone Sanic demo. The rendered asset lives
/// in the shared engine asset tree beside the other generated music tracks.
pub const SANIC_MUSIC_ASSET_PATH: &str = "audio/music/generated/you_are_too_slow/full.ogg";

/// Number of segments in the loop body. The authored entry ramp is sampled
/// separately so its final tangent is exactly the loop's first tangent.
pub const LOOP_SEGMENTS: usize = 96;

/// Samples in the raised entry ramp. A smooth cubic here replaces the old
/// one-segment spur whose corner could strand a momentum body at the join.
pub const LOOP_RAMP_SEGMENTS: usize = 24;

/// Index of the loop's first arc point inside the combined ramp+loop chain.
pub const LOOP_ENTRY_POINT_INDEX: usize = LOOP_RAMP_SEGMENTS;

/// Index of the loop's open release endpoint.
pub const LOOP_EXIT_POINT_INDEX: usize = LOOP_ENTRY_POINT_INDEX + LOOP_SEGMENTS;

const LOOP_RADIUS: f32 = 180.0;
const LOOP_START_ANGLE: f32 = std::f32::consts::FRAC_PI_3;
const LOOP_SWEEP_ANGLE: f32 = std::f32::consts::TAU * 0.75;
const LOOP_RAMP_START_X: f32 = 2000.0;
const LOOP_RAMP_START_CLEARANCE: f32 = 92.0;

fn cubic_bezier(p0: ae::Vec2, p1: ae::Vec2, p2: ae::Vec2, p3: ae::Vec2, t: f32) -> ae::Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// Build one authored route for the raised on-ramp and loop arc.
///
/// The old course joined a single straight spur directly to a coarse arc. The
/// body's route was technically one chain, but the tangent discontinuity at the
/// join behaved like an edge: the follower could shed, project back to the lip,
/// and appear frozen. This cubic ends with the exact first-loop tangent, so the
/// ramp/loop boundary is an ordinary sampled curve rather than a collision seam.
///
/// The open 270-degree release remains intentionally separate from the ramp:
/// it sends the rider down and right to the floor *before* the raised ramp. The
/// body then passes underneath with standing-height clearance, preserving the
/// classic entry/exit layering without deleting the ramp or making it ghostly.
fn raised_ramp_loop_points(floor_top: f32) -> (Vec<ae::Vec2>, ae::Vec2) {
    let center = ae::Vec2::new(2000.0, floor_top - 312.0);
    let ramp_start = ae::Vec2::new(LOOP_RAMP_START_X, floor_top - LOOP_RAMP_START_CLEARANCE);
    let loop_start =
        center + ae::Vec2::new(LOOP_START_ANGLE.cos(), LOOP_START_ANGLE.sin()) * LOOP_RADIUS;
    // The authored loop winds with decreasing angle. Its travel tangent is
    // therefore (sin(theta), -cos(theta)); use it as the cubic's final handle.
    let loop_start_tangent = ae::Vec2::new(LOOP_START_ANGLE.sin(), -LOOP_START_ANGLE.cos());

    let mut points = Vec::with_capacity(1 + LOOP_RAMP_SEGMENTS + LOOP_SEGMENTS);
    points.push(ramp_start);
    let ramp_control_1 = ramp_start + ae::Vec2::new(25.0, 0.0);
    let ramp_control_2 = loop_start - loop_start_tangent * 55.0;
    for step in 1..=LOOP_RAMP_SEGMENTS {
        let t = step as f32 / LOOP_RAMP_SEGMENTS as f32;
        points.push(cubic_bezier(
            ramp_start,
            ramp_control_1,
            ramp_control_2,
            loop_start,
            t,
        ));
    }
    for step in 1..=LOOP_SEGMENTS {
        let t = step as f32 / LOOP_SEGMENTS as f32;
        let theta = LOOP_START_ANGLE - LOOP_SWEEP_ANGLE * t;
        points.push(center + ae::Vec2::new(theta.cos(), theta.sin()) * LOOP_RADIUS);
    }
    (points, center)
}

/// Canonical transform pair for the demo's semantic Utility action (D in the
/// classic arrows+Z/X/C preset).
pub const SANIC_CHARACTER_ID: &str = "sanic";
pub const SUPER_SANIC_CHARACTER_ID: &str = "super_sanic";

/// Visually authored distance markers. The floating marker platforms and the
/// one-shot milestone SFX share this table so the eye and ear measure the same
/// positions instead of drifting as the speedway changes.
pub const SPEED_MARKER_XS: [f32; 5] = [600.0, 1200.0, 1800.0, 2600.0, 3400.0];

/// Build the Sanic momentum showcase room through the `ambition` umbrella
/// surface ONLY. The tiled solid floor remains the ordinary run surface. A
/// rebound reaches one raised, continuously sampled ramp+loop chain; its open
/// release lands before the ramp and then runs underneath it.
pub fn sanic_speedway() -> RoomSpec {
    let width = 4000.0;
    let height = 720.0;
    let floor_top = height - 48.0;

    // Ambition's tiled block-art path supplies a readable ground fill. This is
    // a real solid floor: the raised loop route is reached by the existing
    // rebound, then releases back onto the same ordinary ground.
    let mut tiled_floor = ae::Block::solid(
        "speedway_floor",
        ae::Vec2::new(0.0, floor_top),
        ae::Vec2::new(width, 48.0),
    );
    tiled_floor.id = ae::GeoId::tile_layer("sanic_speedway_ground", 0);
    let mut blocks = vec![tiled_floor];
    blocks.push(ae::Block::one_way(
        "start_gantry",
        ae::Vec2::new(64.0, floor_top - 190.0),
        ae::Vec2::new(260.0, 18.0),
    ));
    for (index, x) in SPEED_MARKER_XS.into_iter().enumerate() {
        let lift = if index % 2 == 0 { 150.0 } else { 220.0 };
        blocks.push(ae::Block::one_way(
            format!("distance_marker_{}", index + 1),
            ae::Vec2::new(x - 52.0, floor_top - lift),
            ae::Vec2::new(104.0, 14.0),
        ));
    }
    blocks.push(ae::Block::rebound(
        "speed_booster",
        ae::Vec2::new(1640.0, floor_top - 22.0),
        ae::Vec2::new(72.0, 22.0),
        // Reach the raised ramp from below. The ramp and arc are one smooth
        // route, so the rebound is entry staging rather than a seam workaround.
        ae::Vec2::new(700.0, -550.0),
    ));
    blocks.push(ae::Block::hazard(
        "finish_warning_spikes",
        ae::Vec2::new(width - 220.0, floor_top - 20.0),
        ae::Vec2::new(116.0, 20.0),
    ));
    blocks.push(ae::Block::solid(
        "finish_tower",
        ae::Vec2::new(width - 72.0, floor_top - 250.0),
        ae::Vec2::new(32.0, 250.0),
    ));
    let spawn = ae::Vec2::new(160.0, floor_top - 64.0);

    let (ramp_loop_points, loop_center) = raised_ramp_loop_points(floor_top);
    let ramp_loop = ae::SurfaceChain::open("sanic_loop", ramp_loop_points);

    let world = ae::World::new(
        "Sanic Speedway",
        ae::Vec2::new(width, height),
        spawn,
        blocks,
    )
    .with_chains(vec![ramp_loop]);

    let mut room = RoomSpec::new(SPEEDWAY_ROOM_ID, world);
    room.metadata.mode = Some(SANIC_MODE.to_string());
    // Borrow Ambition's generated skybridge stack. The visible shell loads the
    // shared `GameAssets`; if those optional images are absent the renderer keeps
    // the deterministic clear-color + landmark geometry fallback.
    room.metadata.biome = Some("skybridge".to_string());
    room.metadata.visual_theme = Some("skybridge".to_string());
    room.metadata.visual_profile.id = Some("sanic_speedway".to_string());
    room.metadata.visual_profile.parallax_theme = Some("skybridge".to_string());

    // World-space labels turn the speedway into a ruler. They are ordinary room
    // debug labels rendered by the generic presentation face, not app-local UI.
    let mut labels = vec![
        (
            "start".to_string(),
            "START   Z: JUMP   DOWN+X: REV   RELEASE DOWN: DASH   D: SUPER".to_string(),
            ae::Vec2::new(300.0, floor_top - 230.0),
        ),
        (
            "loop".to_string(),
            "LOOP".to_string(),
            ae::Vec2::new(loop_center.x, loop_center.y - LOOP_RADIUS - 36.0),
        ),
        (
            "finish".to_string(),
            "FINISH".to_string(),
            ae::Vec2::new(width - 130.0, floor_top - 300.0),
        ),
    ];
    labels.extend(SPEED_MARKER_XS.into_iter().enumerate().map(|(index, x)| {
        (
            format!("marker_{}", index + 1),
            format!("{x:.0}"),
            ae::Vec2::new(x, floor_top - 280.0),
        )
    }));
    room.debug_labels = labels
        .into_iter()
        .map(|(id, text, position)| {
            ambition::world::rooms::Authored::new(
                format!("sanic_{id}"),
                text.clone(),
                ae::Aabb::new(position, ae::Vec2::splat(1.0)),
                ambition::world::debug_label::DebugLabel::new(
                    text,
                    position,
                    ambition::world::debug_label::DebugLabelKind::Custom,
                ),
            )
        })
        .collect();
    room
}

/// The demo's two-form catalog. Every demo installs its own roster; the engine
/// ships none (ADR 0017). The visible shell resolves both generated Sanic forms
/// through the shared Ambition asset catalog. Missing local artifacts remain a
/// loud, marked fallback rather than a second sprite path.
const SANIC_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        // A peaceful speedster: the momentum ride + ball dash ARE the kit; no
        // combat moveset. Referenced by the row below so the catalog is valid.
        "peaceful": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "sanic": (
            sprite_tuning: Some((collision_scale: 1.6, frame_sample_inset: 1)),
            display_name: "Sanic",
            spritesheet: "sprites/sanic_spritesheet.png",
            manifest: "sprites/sanic_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["player"],
            // The MOVEMENT identity that makes this a Sanic demo: the worn home
            // box opts into `MotionModel::SurfaceMomentum` (rides the speedway +
            // loop), which is also what `ball_dash` requires to charge/launch.
            // Without this the body is axis-swept and ball dash is inert — the
            // demo would be an Ambition player wearing the name "Sanic".
            momentum: Some((
                ground_accel: 900.0,
                top_speed: 1200.0,
                jump_speed: 700.0,
            )),
        ),
        "super_sanic": (
            sprite_tuning: Some((collision_scale: 1.6, frame_sample_inset: 1)),
            display_name: "Super Sanic",
            spritesheet: "sprites/super_sanic_spritesheet.png",
            manifest: "sprites/super_sanic_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: ["player", "super", "transformation"],
            // This slice is an identity/presentation transformation. It keeps
            // the same authored peaceful kit and momentum tuning so D cannot
            // accidentally become a second gameplay-authority path.
            momentum: Some((
                ground_accel: 900.0,
                top_speed: 1200.0,
                jump_speed: 700.0,
            )),
        ),
    },
)"#;

pub mod ball_dash;

/// Content plugin for the Sanic movement demo: installs the roster, the world,
/// and the engine's own sim-world setup. This is the shape
/// `crates/ambition_host/tests/demo_shell_smoke.rs` prescribes, built through the
/// `ambition` umbrella alone.
pub struct SanicDemoContentPlugin;

impl Plugin for SanicDemoContentPlugin {
    fn build(&self, app: &mut App) {
        use ambition::runtime::demo_fixture::{ActiveRoomMetadata, RoomSet};
        use bevy::prelude::IntoScheduleConfigs;

        ambition::runtime::demo_fixture::install_character_catalog(SANIC_CATALOG_RON);
        ambition::actors::session::data::install_music_registry(
            ambition::audio::spec::MusicRegistry {
                default_track: "you_are_too_slow".to_string(),
                tracks: vec![ambition::audio::spec::MusicTrack {
                    id: "you_are_too_slow".to_string(),
                    display_name: "You Are Too Slow".to_string(),
                    asset_path: Some(SANIC_MUSIC_ASSET_PATH.to_string()),
                }],
            },
        );
        // The packed Ambition bank supplies the actual typed cues. The registry
        // remains intentionally minimal but valid so the demo owns its audio
        // data seam instead of borrowing the full game's content registry.
        ambition::actors::session::data::install_sfx_registry(ambition::audio::spec::SfxRegistry {
            sample_rate: 44_100,
            sfx: Vec::new(),
        });
        // The demo's player is explicitly the speedster rather than relying on
        // whichever row happens to be the installed catalog default.
        app.insert_resource(ambition::runtime::demo_fixture::StartingCharacter::new(
            "sanic",
        ));
        let room = sanic_speedway();
        app.insert_resource(ae::RoomGeometry(room.world.clone()));
        app.insert_resource(ActiveRoomMetadata(room.metadata.clone()));
        app.insert_resource(RoomSet::from_parts(
            SPEEDWAY_ROOM_ID,
            vec![room],
            Vec::new(),
        ));
        app.add_systems(
            bevy::app::Startup,
            sanic_setup.in_set(ambition::runtime::demo_fixture::SimulationSetupSet),
        );
    }
}

/// The demo's world construction: the engine's `simulation_world` on the
/// speedway. Labeled `SimulationSetupSet` so the host's input attach orders after
/// the player body exists.
#[allow(clippy::too_many_arguments)]
fn sanic_setup(
    mut commands: bevy::prelude::Commands,
    world: bevy::prelude::Res<ae::RoomGeometry>,
    room_set: bevy::prelude::Res<ambition::runtime::demo_fixture::RoomSet>,
    ldtk_index: bevy::prelude::Res<ambition::runtime::demo_fixture::LdtkRuntimeIndex>,
    editable_abilities: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableAbilitySet>,
    editable_tuning: bevy::prelude::Res<ambition::runtime::demo_fixture::EditableMovementTuning>,
    starting_character: bevy::prelude::Res<ambition::runtime::demo_fixture::StartingCharacter>,
    asset_server: bevy::prelude::Res<bevy::asset::AssetServer>,
) {
    ambition::runtime::demo_fixture::simulation_world(
        &mut commands,
        ambition::runtime::demo_fixture::SimulationSetup {
            world: &world,
            room_set: &room_set,
            ldtk_index: &ldtk_index,
            editable_abilities: &editable_abilities,
            editable_tuning: &editable_tuning,
            starting_character: &starting_character,
            sandbox_data_asset: None,
            sandbox_asset_collection: None,
            asset_server: &asset_server,
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The RULES plugin — the D-C mode-scope seam, used for real.
// ─────────────────────────────────────────────────────────────────────────────

/// The act's live state, owned by the mode. It rides a `ModeScopedEntity`, so
/// leaving the Sanic rooms tears it down through the engine's lifetime-scope
/// vocabulary rather than a bespoke reset.
#[derive(bevy::prelude::Component, Default, Debug)]
pub struct SanicActState {
    /// Seconds the act has been running (sim clock, so bullet-time slows it).
    pub elapsed: f32,
    /// Next index in [`SPEED_MARKER_XS`] that should emit its one-shot progress
    /// cue. Mode-scoped with the act, so leaving and re-entering the demo resets
    /// the audible ruler without a global resource leak.
    pub next_milestone: usize,
}

/// Sanic's level rules. **ONE system list; a constructor flag decides its gating**
/// — [`SanicRulesPlugin::hosted`] when Ambition hosts the demo alongside its own
/// rooms, [`SanicRulesPlugin::global`] when the demo IS the game. That is the D-C
/// pattern (`docs/planning/engine/decomposition.md` §Phase D-C), and this is its
/// first real consumer: before this, `in_mode` had no ruleset to gate.
pub struct SanicRulesPlugin {
    hosted: bool,
}

impl SanicRulesPlugin {
    /// Ambition hosts this demo: every rule sleeps outside the Sanic rooms.
    pub fn hosted() -> Self {
        Self { hosted: true }
    }

    /// The demo IS the game: the rules run unconditionally.
    pub fn global() -> Self {
        Self { hosted: false }
    }
}

impl Plugin for SanicRulesPlugin {
    fn build(&self, app: &mut App) {
        use bevy::prelude::IntoScheduleConfigs;
        let sim = ambition::platformer::schedule::SimScheduleExt::sim_schedule(app);
        app.init_resource::<ball_dash::BallDashTuning>();
        // Attach the mode-local state and consume Sanic's semantic input verbs
        // before the generic peaceful-kit gate erases combat intent. Chaining
        // provides an apply-deferred seam, so the first eligible X edge cannot be
        // lost on a newly controlled body. Utility is D in the classic preset;
        // the transform system consumes that edge so it cannot also toggle a
        // host-code flight ability inherited by the control box.
        let sanic_input_rules = (
            ball_dash::attach_ball_dash,
            ball_dash::capture_ball_dash_input,
            toggle_sanic_form,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::PlayerInput)
            .after(ambition::actors::avatar::tick_player_brains)
            .before(ambition::actors::avatar::gate_worn_player_control);
        if self.hosted {
            app.add_systems(
                sim,
                sanic_input_rules.run_if(ambition::runtime::in_mode(SANIC_MODE)),
            );
        } else {
            app.add_systems(sim, sanic_input_rules);
        }

        // The ball dash is a RULE, not world content: it exists while the Sanic
        // mode is live and nowhere else, exactly like the act clock. Effects run
        // after PlayerInput captured the technique and before later presentation.
        // `tick_ball_dash` precedes `tick_rolling`, so a launch cannot un-ball in
        // the same frame even if tuning changes.
        let rules = (
            spawn_sanic_mode_owner,
            tick_sanic_act,
            ball_dash::tick_ball_dash,
            ball_dash::tick_rolling,
        )
            .chain()
            .in_set(ambition::platformer::schedule::SandboxSet::GameplayEffects);
        let milestone_sfx = emit_sanic_milestone_sfx
            .in_set(ambition::platformer::schedule::SandboxSet::GameplayEffects);
        if self.hosted {
            app.add_systems(sim, rules.run_if(ambition::runtime::in_mode(SANIC_MODE)));
            app.add_systems(
                sim,
                milestone_sfx.run_if(ambition::runtime::in_mode(SANIC_MODE)),
            );
        } else {
            app.add_systems(sim, rules);
            app.add_systems(sim, milestone_sfx);
        }
    }
}

/// Toggle the controlled body between the two catalog-authored Sanic forms.
///
/// This consumes the already-semantic Utility edge (`D` in the demo's classic
/// keyboard preset), never a raw key. Both rows carry the same movement and
/// peaceful action profile, so `WornCharacter` remains the single gameplay +
/// presentation authority and the transformation cannot fork a second kit path.
fn toggle_sanic_form(
    subject: Option<bevy::prelude::Res<ambition::platformer::markers::ControlledSubject>>,
    mut bodies: bevy::prelude::Query<(
        &mut ambition::characters::brain::ActorControl,
        &mut ambition::characters::actor::WornCharacter,
        &ae::BodyKinematics,
    )>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    let Some(entity) = subject.and_then(|subject| subject.0) else {
        return;
    };
    let Ok((mut control, mut worn, kinematics)) = bodies.get_mut(entity) else {
        return;
    };
    if !control.0.fly_toggle_pressed {
        return;
    }

    // Utility belongs to this mode-local transformation. Consume the edge before
    // lower movement layers can interpret it as the generic fly toggle.
    control.0.fly_toggle_pressed = false;
    let next = match worn.id() {
        SANIC_CHARACTER_ID => SUPER_SANIC_CHARACTER_ID,
        SUPER_SANIC_CHARACTER_ID => SANIC_CHARACTER_ID,
        _ => return,
    };
    *worn = ambition::characters::actor::WornCharacter::new(next);
    sfx.write(ambition::sfx::SfxMessage::Dash {
        pos: kinematics.pos,
    });
}

/// Bring the act state into being the first frame the mode is live. Spawned
/// `spawn_mode_scoped`, so the engine despawns it when the active room's mode
/// changes — no teardown code here.
fn spawn_sanic_mode_owner(
    mut commands: bevy::prelude::Commands,
    existing: bevy::prelude::Query<(), bevy::prelude::With<SanicActState>>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    use ambition::platformer::lifecycle::SpawnScopedExt;
    if existing.iter().next().is_none() {
        commands.spawn_mode_scoped(SANIC_MODE, SanicActState::default());
        // Audible confirmation that the standalone shell is draining the
        // standard SfxMessage seam. Distance markers emit alternating cues as
        // the player advances, so this one also proves the bank at room entry.
        sfx.write(ambition::sfx::SfxMessage::Dash {
            pos: ae::Vec2::ZERO,
        });
    }
}

/// The act timer runs on the SIM clock (`scaled_dt`), so bullet-time and pause
/// slow it exactly as they slow everything else — `WorldTime`, never `Res<Time>`.
fn tick_sanic_act(
    time: bevy::prelude::Res<ambition::time::WorldTime>,
    mut act: bevy::prelude::Query<&mut SanicActState>,
) {
    for mut state in &mut act {
        state.elapsed += time.scaled_dt;
    }
}

/// Emit a small, existing Ambition cue when the primary body crosses each
/// visible distance marker. These are deliberately simple diagnostic sounds:
/// the demo is proving that its shell drains the standard [`ambition::sfx::SfxMessage`] seam,
/// not inventing a parallel Sanic audio stack.
fn emit_sanic_milestone_sfx(
    player: bevy::prelude::Query<
        &ae::BodyKinematics,
        bevy::prelude::With<ambition::actors::actor::PrimaryPlayer>,
    >,
    mut act: bevy::prelude::Query<&mut SanicActState>,
    mut sfx: bevy::prelude::MessageWriter<ambition::sfx::SfxMessage>,
) {
    let Ok(kin) = player.single() else {
        return;
    };
    for mut state in &mut act {
        while let Some(&marker_x) = SPEED_MARKER_XS.get(state.next_milestone) {
            if kin.pos.x < marker_x {
                break;
            }
            let message = if state.next_milestone % 2 == 0 {
                ambition::sfx::SfxMessage::Dash { pos: kin.pos }
            } else {
                ambition::sfx::SfxMessage::Jump { pos: kin.pos }
            };
            sfx.write(message);
            state.next_milestone += 1;
        }
    }
}

/// Install the Sanic demo content layer into an engine app.
pub fn add_demo_content(app: &mut App) {
    app.add_plugins(SanicDemoContentPlugin);
}

#[cfg(test)]
mod tests;
