//! **A versus stage you can pick from the launcher.** (C4 slice 2)
//!
//! Slice 1 gave the engine the verb — `seat_match_participants` turns a
//! [`MatchParticipantRoster`] into bodies. Nothing called it. C4's complaint was
//! never that the fight did not work; it was that the fight existed only where a
//! test could see it, and "a stranger can run it and watch" is the whole
//! difference between an engine demo and an engine.
//!
//! So this is a route. It appears in the launcher beside Ambition, Sanic and
//! Mary-O, it prepares a flat arena, and it seats two CPU fighters facing each
//! other. Nobody has to read a test to know whether two characters can fight.
//!
//! ## Why it lives in the app and not in a provider crate
//!
//! The fighters are `mary_o` and `sanic`, registered by two DIFFERENT provider
//! plugins. The only composition where both casts exist is the multi-game shell
//! host, so the app is the only honest home for a match between them — and a
//! crossover is exactly what the multi-provider character work has been building
//! toward. A provider that wanted its own single-cast versus mode would author
//! the same thing inside itself.
//!
//! ## What it deliberately does not do
//!
//! No win condition, no rounds, no HUD. Those are match RULES, and a stage that
//! has to be right before rules can be written is the thing that was missing;
//! inventing a ruleset here would make the slice bigger and the seam no more
//! proven. Slice 3 (player vs CPU) needs the human seat, not rules.

use bevy::prelude::*;

use ambition::actors::character_runtime::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster, StagesCharacters,
};
use ambition::engine_core as ae;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;
use ambition::world::rooms::RoomSpec;

pub const VERSUS_EXPERIENCE: &str = "ambition_versus";
pub const VERSUS_GAMEPLAY_ROUTE: &str = "versus_gameplay";
pub const VERSUS_ROOM_ID: &str = "versus_arena";

/// The two fighters. Deliberately one from each demo provider: a match between
/// characters whose art, cues and movesets come from different packages is the
/// case every seam in the character work was built for, and the case a
/// single-provider roster would never exercise.
const FIGHTERS: [&str; 2] = ["mary_o", "sanic"];

/// A flat arena with walls. Nothing else: the stage exists so two bodies have
/// somewhere to stand and something to be stopped by, and every feature it does
/// not have is one that cannot be blamed when a fight looks wrong.
fn versus_arena() -> RoomSpec {
    let size = ae::Vec2::new(960.0, 540.0);
    let floor_top = 460.0;
    let world = ae::World::new(
        "Versus Arena",
        size,
        // Spawn at the centre of the floor: seating places fighters symmetrically
        // about this point, so it IS the middle of the stage.
        ae::Vec2::new(size.x * 0.5, floor_top - 24.0),
        vec![
            ae::Block::solid(
                "arena_floor",
                ae::Vec2::new(0.0, floor_top),
                ae::Vec2::new(size.x, size.y - floor_top),
            ),
            ae::Block::solid(
                "arena_wall_left",
                ae::Vec2::ZERO,
                ae::Vec2::new(16.0, size.y),
            ),
            ae::Block::solid(
                "arena_wall_right",
                ae::Vec2::new(size.x - 16.0, 0.0),
                ae::Vec2::new(16.0, size.y),
            ),
        ],
    );
    let mut room = RoomSpec::new(VERSUS_ROOM_ID, world);
    room.metadata.mode = Some(VERSUS_EXPERIENCE.to_owned());
    room
}

fn versus_prepared_session_world() -> PreparedPlatformerSource {
    let room = versus_arena();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    PreparedPlatformerSource::new(
        VERSUS_EXPERIENCE,
        RoomSet::from_parts(VERSUS_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        // The primary player wears the first fighter. It is the camera's subject
        // and, in slice 3, the body a human seat takes over — so the stage does
        // not change shape when the human arrives.
        StartingCharacter::new(FIGHTERS[0]),
        LdtkRuntimeIndex::default(),
    )
}

/// The roster this stage seats: the player, and one CPU opponent.
///
/// Seat 0 is HUMAN and is the body the session already spawned wearing
/// `FIGHTERS[0]` — seating adopts it rather than spawning beside it. That is not
/// a detail: the first version of this stage made both seats CPU while the
/// session had already spawned a player wearing the same character, and the
/// arena held two Mary-Os. The test passed because it asserted both fighters were
/// present, which is the assertion you write when you have not looked at the
/// screen.
///
/// Player-versus-CPU is also the second step of Jon's order (cpu vs cpu, then
/// player vs cpu, then local couch, and only then netcode). Step one is what the
/// seating tests prove; this is the one a stranger can pick up a controller and
/// play.
pub fn versus_roster() -> MatchParticipantRoster {
    MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new(FIGHTERS[0])
                .driven_by(ControllerBinding::Human { device_slot: 0 })
                .on_team("blue"),
            MatchParticipant::new(FIGHTERS[1])
                .driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("medium_striker".into()),
                })
                .on_team("red"),
        ],
    }
}

/// Install the roster while the versus route is active, and take it away when it
/// is not.
///
/// Scoped rather than global on purpose: `MatchParticipantRoster` is what seating
/// reads, so leaving it installed would seat two fighters into Mary-O's level the
/// next time somebody played it. Removing it on exit also resets the seating
/// latch, so returning to the stage seats a fresh match rather than an empty one.
fn track_versus_roster(
    mut commands: Commands,
    router: Res<ambition::game_shell::ShellRouter>,
    roster: Option<Res<MatchParticipantRoster>>,
    mut demand: ResMut<ambition::actors::character_runtime::CharacterLoadDemand>,
    mut seated: ResMut<ambition::actors::character_runtime::MatchSeated>,
) {
    let on_versus = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == VERSUS_GAMEPLAY_ROUTE);
    match (on_versus, roster.is_some()) {
        (true, false) => {
            let roster = versus_roster();
            // Demand the art before the bodies exist: a fighter seated with no
            // decoded sheet draws a placeholder, and the whole point of a visible
            // slice is that it looks like the two characters it says it is.
            roster.project_demand(&mut demand);
            commands.insert_resource(roster);
            seated.0 = false;
        }
        (false, true) => {
            commands.remove_resource::<MatchParticipantRoster>();
            seated.0 = false;
        }
        _ => {}
    }
}

/// Register the versus experience: a launcher entry, a route, and the stage.
pub fn compose_versus_experience(app: &mut App) {
    PlatformerExperienceAuthoring::new(
        VERSUS_EXPERIENCE,
        VERSUS_GAMEPLAY_ROUTE,
        "Versus",
        "Two characters from two providers, fighting",
        "Prepare Versus",
        AuthoredCatalogFragments::new(FIGHTERS[0], VERSUS_EXPERIENCE),
    )
    .install(app, versus_prepared_session_world);

    // DELIBERATE SILENCE, declared. Preparation refuses an experience whose
    // provider registered no explicit audio fragment — a good refusal, and one
    // the external-consumer fixture hit first. The stage authors no cues of its
    // own: the fighters bring theirs, attributed to THEIR providers, which is the
    // whole point of a crossover.
    {
        use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
        app.register_audio_catalog_fragment(
            AudioCatalogFragment::new(VERSUS_EXPERIENCE, None, None)
                .expect("the silent versus audio fragment is valid"),
        );
    }

    // `Update`, NOT the sim schedule.
    //
    // The first draft put this in `SandboxSet::PlayerInput`, and its own test
    // caught the consequence: every `SandboxSet` is nested inside
    // `GameplaySimulationRoot`, which carries the session gate, so leaving
    // gameplay switches OFF the system whose job is to clean up after leaving
    // gameplay. The roster survived, and the next game the player picked would
    // have had two fighters seated into it — silently, because the bodies look
    // like they belong to the level.
    //
    // A teardown that lives inside the thing it tears down can only ever run
    // while it is not needed. This reads the shell router and writes resources;
    // it is shell lifecycle, and the shell clock is where it belongs.
    app.add_systems(Update, track_versus_roster);
}
