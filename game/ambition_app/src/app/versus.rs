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

/// The two fighters.
///
/// They used to be `mary_o` and `sanic`, chosen because a match between two
/// demo providers' casts is the crossover the character seam was built for.
/// That was the right cast for proving the seam and the wrong one for a fight:
/// neither authors a move list, so couch versus was two people walking into
/// each other, and giving either one attacks would have been authoring against
/// their design to make a different mode work (Sanic's row says "no combat
/// moveset" in as many words; Mary-O has to play like SMB1).
///
/// The arena has its own fighters now — see [`super::versus_fighters`]. The
/// crossover is not lost: they SHARE the demos' art by id, so the stage still
/// draws one cast against the other, and it is still the only composition where
/// both sheets exist.
const FIGHTERS: [&str; 2] = ["arena_duelist_long", "arena_duelist_close"];

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

/// The roster this stage seats: the player, and either a friend or a CPU.
///
/// Seat 0 is HUMAN and is the body the session already spawned wearing
/// `FIGHTERS[0]` — seating adopts it rather than spawning beside it. That is not
/// a detail: the first version of this stage made both seats CPU while the
/// session had already spawned a player wearing the same character, and the
/// arena held two Mary-Os. The test passed because it asserted both fighters were
/// present, which is the assertion you write when you have not looked at the
/// screen.
///
/// Seat 1 is a HUMAN when a second controller is plugged in, and a CPU
/// otherwise. That is the whole of Jon's ordering (cpu vs cpu, then player vs
/// cpu, then local couch, and only then netcode) reaching its third step, and it
/// needs no menu: the presence of a second controller is an unambiguous
/// statement that a second person is here, and a stage that asked the question
/// in a lobby screen would be a lobby screen standing between a stranger and the
/// thing they came to see.
///
/// Deliberately decided at STAGE ENTRY, not per frame. A pad unplugged mid-match
/// must not silently hand player two's fighter to the AI, and a pad plugged in
/// mid-match must not spawn a third body into a running fight; both are
/// mid-match roster edits, which is a rule change, and this stage has no rules.
pub fn versus_roster(local_players: usize) -> MatchParticipantRoster {
    let opponent = if local_players > 1 {
        ControllerBinding::Human { device_slot: 1 }
    } else {
        ControllerBinding::Cpu {
            brain_profile: Some("medium_striker".into()),
        }
    };
    MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new(FIGHTERS[0])
                .driven_by(ControllerBinding::Human { device_slot: 0 })
                .on_team("blue"),
            MatchParticipant::new(FIGHTERS[1])
                .driven_by(opponent)
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
    devices: Res<ambition::input::LocalDeviceOrder>,
    roster: Option<Res<MatchParticipantRoster>>,
    mut demand: ResMut<ambition::actors::character_runtime::CharacterLoadDemand>,
    mut seated: ResMut<ambition::actors::character_runtime::MatchSeated>,
    mut friendly_fire: ResMut<ambition::combat::targeting::FriendlyFire>,
    mut match_state: ResMut<super::versus_rules::VersusMatch>,
) {
    let on_versus = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == VERSUS_GAMEPLAY_ROUTE);
    match (on_versus, roster.is_some()) {
        (true, false) => {
            // One local player per connected controller, capped by the stage's
            // two seats. Zero controllers is one seat: the keyboard is player
            // one, which is how every other route in the shell already plays.
            let roster = versus_roster(devices.devices().len().max(1));
            // Demand the art before the bodies exist: a fighter seated with no
            // decoded sheet draws a placeholder, and the whole point of a visible
            // slice is that it looks like the two characters it says it is.
            roster.project_demand(&mut demand);
            commands.insert_resource(roster);
            seated.0 = false;
            // NO global free-for-all. The fighters are on declared TEAMS
            // (`blue` / `red`), and `MatchTeam` outranks faction for "may this
            // land" — which is what the roster's teams were for since §7.8 and
            // what nothing read until 2026-07-27.
            //
            // The stage DID switch on global `FriendlyFire` for a day, because
            // `effective_faction` maps any player-brained body to Player and two
            // humans are therefore always the same faction. That works for a
            // free-for-all and is wrong the moment a 2v2 exists: it makes
            // teammates hittable too, and it is a world-wide rule change made by
            // one stage. Teams say the same thing locally and correctly.
            friendly_fire.enabled = false;
            // A FRESH match. `VersusMatch` is a long-lived resource, so without
            // this, leaving mid-round and coming back resumes the old score —
            // and a KO or match-over countdown resumes with it, which reads as
            // the stage opening onto somebody else's game (GPT 5.6,
            // 2026-07-27). Reset on ENTRY rather than exit so a crash or a
            // route change that skips the teardown still starts clean.
            *match_state = super::versus_rules::VersusMatch::default();
        }
        (false, true) => {
            commands.remove_resource::<MatchParticipantRoster>();
            seated.0 = false;
            // Off again on the way out, for the same reason the roster is
            // removed: a match rule that outlives its match is a rule the next
            // game silently inherits, and "your allies can now shoot you" is a
            // bad surprise to bring into a co-op level.
            friendly_fire.enabled = false;
        }
        _ => {}
    }
}

/// Register the versus experience: a launcher entry, a route, and the stage.
pub fn compose_versus_experience(app: &mut App) {
    // The ROW (who they are) and the DEFINITION (what they do), in that order.
    // Preparation refuses an experience whose starting character has no catalog
    // row, and the hand-authored moveset only exists on the definition.
    {
        use ambition::characters::actor::character_catalog::{
            CharacterCatalogAppExt, CharacterCatalogFragment,
        };
        app.register_character_catalog_fragment(
            CharacterCatalogFragment::from_ron(
                VERSUS_EXPERIENCE,
                Some(FIGHTERS[0]),
                super::versus_fighters::VERSUS_CATALOG_RON,
            )
            .expect("the versus fighter catalog is valid"),
        );
    }
    use ambition::actors::character_runtime::CharacterDefinitionAppExt;
    for fighter in super::versus_fighters::duelists() {
        app.register_character(fighter);
    }

    PlatformerExperienceAuthoring::new(
        VERSUS_EXPERIENCE,
        VERSUS_GAMEPLAY_ROUTE,
        "Versus",
        "Two characters from two providers, fighting",
        "Prepare Versus",
        AuthoredCatalogFragments::new(FIGHTERS[0], VERSUS_EXPERIENCE),
    )
    // The scoreboard. Three declared readouts; the engine never learns what a
    // ROUND is — `publish_versus_hud` writes the words.
    .with_hud(
        ambition::presentation::HudDeclaration::new()
            .slot(
                ambition::presentation::HudSlotSpec::new(super::versus_rules::HEALTH_HUD_SLOT)
                    .with_region(ambition::presentation::SurroundRegion::Top)
                    .with_font_size(22.0)
                    .with_color([1.0, 0.95, 0.9, 1.0]),
            )
            .slot(
                ambition::presentation::HudSlotSpec::new(super::versus_rules::ROUNDS_HUD_SLOT)
                    .with_region(ambition::presentation::SurroundRegion::Top)
                    .with_font_size(16.0)
                    .with_color([0.75, 0.8, 0.95, 1.0]),
            )
            // The KO / match card. Published only while a round is over, so it
            // needs no hide path — an unpublished slot draws nothing.
            .slot(
                ambition::presentation::HudSlotSpec::new(super::versus_rules::ANNOUNCE_HUD_SLOT)
                    .centered()
                    .with_font_size(34.0)
                    .with_color([1.0, 0.85, 0.3, 1.0]),
            ),
    )
    .install(app, versus_prepared_session_world);

    app.init_resource::<super::versus_rules::VersusMatch>();
    // The scoreboard is SIMULATION state: a rewind that restores the fighters
    // and not the score leaves the two disagreeing about what round it is.
    {
        use ambition::runtime::rollback::AmbitionRollbackApp;
        app.rollback_resource_clone::<super::versus_rules::VersusMatch>(
            VERSUS_EXPERIENCE,
            "resource.versus_match",
        );
    }

    // The AUTHORITATIVE half on the sim schedule, after damage has resolved —
    // it reads this tick's health outcome and writes body position. `Settle` is
    // the phase for exactly that: "everything that reads this tick's damage
    // outcome rather than producing it".
    {
        use ambition::platformer::schedule::{CombatSet, SimScheduleExt};
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            super::versus_rules::settle_versus_round.in_set(CombatSet::Settle),
        );
    }

    // The PRESENTATION half in `Update`, on the render clock. It holds the KO
    // card and asks the engine to freeze the sim while it is up; counting that
    // hold on the clock it just zeroed would hold forever.
    app.add_systems(
        Update,
        (
            super::versus_rules::advance_versus_hold,
            super::versus_rules::publish_versus_hud,
        )
            .chain(),
    );

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

    // The stage READS the device order, so the stage guarantees it exists.
    // `ambition_host` owns the systems that maintain it, and a composition
    // without the host input plugin (the lifecycle fixtures) still routes
    // through here — an `Option<Res>` would turn "this host has no device
    // layer" into "zero controllers", which is exactly the answer that decides
    // player two is a CPU. `init_resource` is a no-op when the host already
    // installed it, so there is still one writer.
    app.init_resource::<ambition::input::LocalDeviceOrder>();

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
