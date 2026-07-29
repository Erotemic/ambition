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

/// A stage with EDGES. The first version was a closed box — a floor and two
/// walls — which is a room, not a stage: a fighter could be pushed to the wall
/// and no further, so the only way to lose was to run out of health, and the
/// verb the whole genre is built on (put them somewhere there is no floor) had
/// nowhere to happen.
///
/// So: a main platform floating in open space, two side platforms to recover
/// onto, no walls, and a blast margin the stage authors itself. Past that
/// margin the engine's out-of-bounds gate reports
/// `ResetCause::LeftTheWorld` — a lethal hit against exactly that fighter,
/// which zeroes their health, which is the condition `round_result` already
/// scores. A knock-off ends a round through the rule that was already there.
///
/// The margins are deliberately generous relative to the drop: a fighter has the
/// whole 140px below the platform plus 96px past the world before they are
/// gone, which is long enough to see the mistake and long enough to jump back
/// from. A tight blast zone turns every trade near the edge into a coin flip.
/// No CEILING zone: nothing in the current moveset launches hard enough upward
/// for one to be anything but a surprise.
fn versus_arena() -> RoomSpec {
    let size = ae::Vec2::new(960.0, 540.0);
    let platform_top = 400.0;
    // Seating spreads fighters up to ±144px about the centre, so a 480-wide
    // main platform holds a full four-seat 2v2 with room either side.
    let main_half_width = 240.0;
    let world = ae::World::new(
        "Versus Arena",
        size,
        // Spawn at the centre of the main platform: seating places fighters
        // symmetrically about this point, so it IS the middle of the stage.
        ae::Vec2::new(size.x * 0.5, platform_top - 24.0),
        vec![
            ae::Block::solid(
                "arena_platform_main",
                ae::Vec2::new(size.x * 0.5 - main_half_width, platform_top),
                ae::Vec2::new(main_half_width * 2.0, 40.0),
            ),
            // Recovery ledges. Solid rather than one-way: a fighter thrown out
            // sideways needs something to catch, and a platform they can fall
            // straight back through is not a second chance.
            ae::Block::solid(
                "arena_platform_left",
                ae::Vec2::new(96.0, platform_top - 96.0),
                ae::Vec2::new(128.0, 20.0),
            ),
            ae::Block::solid(
                "arena_platform_right",
                ae::Vec2::new(size.x - 224.0, platform_top - 96.0),
                ae::Vec2::new(128.0, 20.0),
            ),
        ],
    )
    .with_blast_margin(96.0)
    // The SIDE blast zone, which is where a platform fighter actually loses
    // most of its stocks. Without it a fighter thrown off the left edge only
    // dies once its arc happens to carry it below the stage, which reads as the
    // throw not having worked. 160px is deliberately looser than the 96px
    // floor: you are given further to recover from a horizontal launch than
    // from a straight drop, because a horizontal launch is the one you can act
    // on.
    .with_side_blast_margin(160.0);
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
/// Up to four seats: 1v1 with two controllers, 2v2 with four.
///
/// The seat count follows the controllers because that is the least ceremonious
/// honest signal — four pads is four people, and a lobby screen asking how many
/// players there are would stand between a stranger and the thing they came to
/// see.
///
/// TEAMS FOLLOW SIDES. `seat_for` alternates left/right by index, so evens stand
/// together and odds stand together; pairing the teams the same way puts
/// partners on the same side of the arena rather than opposite each other.
///
/// It is also the case teams were built for. With four HUMAN fighters,
/// `effective_faction` maps every one of them to `ActorFaction::Player` — so
/// faction distinguishes nobody and `MatchTeam` is the only thing that decides
/// who may hit whom. A 2v2 is not a bigger 1v1; it is the first arrangement
/// where the relation is load-bearing.
pub fn versus_roster(local_players: usize) -> MatchParticipantRoster {
    versus_roster_from(local_players, None)
}

/// The same, recording which frozen topology decided the seat count.
pub fn versus_roster_from(
    local_players: usize,
    seat_topology: Option<u64>,
) -> MatchParticipantRoster {
    // Two seats minimum (there is always an opponent, human or not) and four
    // maximum (the arena is one screen wide, and `SlotControls` holds four).
    let seats = local_players.clamp(2, MAX_VERSUS_SEATS);
    let participants = (0..seats)
        .map(|seat| {
            let controller = if seat < local_players {
                ControllerBinding::Human {
                    device_slot: seat as u8,
                }
            } else {
                ControllerBinding::Cpu {
                    brain_profile: Some("medium_striker".into()),
                }
            };
            MatchParticipant::new(FIGHTERS[seat % FIGHTERS.len()])
                .driven_by(controller)
                .on_team(if seat % 2 == 0 { "blue" } else { "red" })
        })
        .collect();
    MatchParticipantRoster {
        participants,
        // A round opens on a COUNTDOWN, so nobody acts until it ends. Declared on
        // the roster rather than applied after seating: a fighter that seats on
        // the tick the countdown begins would otherwise get one simulation step
        // first — a CPU decision, or a held direction — before the suspension
        // insert lands (GPT 5.6, 2026-07-29). The `Starting` arm reaching zero is
        // what takes it off, which is already the one place a round goes live.
        opens_suspended: true,
        // **A FAIR FIGHT.** Seat 0 is the ADOPTED primary player and arrives
        // carrying whatever the session granted it — in the shipped host, the
        // sandbox dev kit (blink, fly, shield). Every other seat is spawned with
        // the basic run-and-jump floor. So player one could teleport and fly and
        // the opponent could not, and the control legend on screen said so.
        //
        // Stated by the match rather than assumed by seating: a stage that wants
        // asymmetric fighters says something else here.
        fighter_abilities: Some(ae::AbilitySet::basic()),
        seat_topology,
    }
}

/// One screen, four fighters, four `SlotControls` slots.
pub const MAX_VERSUS_SEATS: usize = 4;

/// Install the roster while the versus route is active, and take it away when it
/// is not.
///
/// Scoped rather than global on purpose: `MatchParticipantRoster` is what seating
/// reads, so leaving it installed would seat two fighters into Mary-O's level the
/// next time somebody played it. Removing it on exit also resets the seating
/// latch, so returning to the stage seats a fresh match rather than an empty one.
/// **The roster and the session must agree about who is playing, and now they
/// can be asked.** (queue Y′9)
///
/// A route is entered before its rollback session starts, so the roster is built
/// from LIVE device discovery and the topology is frozen later. A controller
/// connecting in that gap leaves the roster seating N fighters into a session
/// with M handles, with both citing "the connected controllers".
///
/// Before seating, a roster is only an INTENTION — nothing has a body yet — so
/// the honest response is to rebuild it against the topology that won. After
/// seating it is bodies on a stage, and silently reseating mid-match would be a
/// worse bug than the disagreement; that case is reported instead.
///
/// ⚠ this is detection plus a safe early repair, not the full fix. The full fix
/// is match ACTIVATION — validate every participant, activate the roster
/// atomically, publish it, and start the countdown from THAT — which is written
/// up in `character-preparation-finalization-plan.md` under "Related,
/// deliberately after". Doing it here would mean building that seam inside a
/// change about a counter.
fn reconcile_roster_with_frozen_topology(
    mut commands: Commands,
    topology: Option<Res<ambition::input::LocalSeatTopology>>,
    roster: Option<ResMut<MatchParticipantRoster>>,
    active_match: Option<ResMut<ambition::actors::character_runtime::ActiveMatch>>,
    mut demand: ResMut<ambition::actors::character_runtime::CharacterLoadDemand>,
) {
    let (Some(topology), Some(mut roster)) = (topology, roster) else {
        return;
    };
    if !topology.is_frozen() || roster.seat_topology == Some(topology.generation()) {
        return;
    }
    // What the frozen topology WOULD seat. Before activation this replaces the
    // roster; after it, it is the thing the live roster is compared against.
    let rebuilt = versus_roster_from(topology.players(), Some(topology.generation()));
    if let Some(mut active) = active_match {
        // The match is LIVE and these are its bodies. Reseating them underneath
        // the round is worse than the disagreement — so the question is whether
        // there IS one.
        //
        // ⚠ **and the obvious comparison is wrong.** A one-player topology seats
        // TWO participants (there is always an opponent, human or CPU), so
        // comparing `participants.len()` against `topology.players()` reports a
        // disagreement in the ordinary single-player case. Compare the FIGHTERS —
        // the roster the frozen topology would build against the one that is
        // live — because that is what "the same match" actually means.
        if rebuilt.participants == roster.participants {
            // AGREEMENT ABOUT WHO, disagreement only about the paperwork.
            //
            // Ordinary, not contrived: the route builds its roster from live
            // device discovery on entry, and the rollback session freezes its
            // topology afterwards, so a roster stamped `None` meets a frozen
            // generation the moment the session starts. Nothing about the stage
            // is wrong — the same fighters, driven by the same devices — and the
            // only stale thing is the record of which decision produced them.
            //
            // Correcting a record is a repair. This is the half of Y′9 that was
            // left as "reported rather than repaired", and it turns the warning
            // below into a signal instead of startup noise.
            roster.seat_topology = Some(topology.generation());
            active.adopt_seat_topology(topology.generation());
            return;
        }
        bevy::log::warn_once!(
            "the versus roster seats {} fighter(s) from seat topology {:?}, but the session \
             froze topology {} with {} player(s), which would seat {} different fighter(s) \
             across {} body/bodies already on the stage. The match is already seated, so it \
             is left alone — reseating bodies mid-match would be the worse bug.",
            roster.participants.len(),
            roster.seat_topology,
            topology.generation(),
            topology.players(),
            rebuilt.participants.len(),
            active.seats(),
        );
        return;
    }
    rebuilt.project_demand(&mut demand);
    commands.insert_resource(rebuilt);
}

fn track_versus_roster(
    mut commands: Commands,
    router: Res<ambition::game_shell::ShellRouter>,
    devices: Res<ambition::input::LocalDeviceOrder>,
    // The seating a rollback session froze, when one is running.
    topology: Option<Res<ambition::input::LocalSeatTopology>>,
    roster: Option<Res<MatchParticipantRoster>>,
    mut demand: ResMut<ambition::actors::character_runtime::CharacterLoadDemand>,
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
            // THE SESSION'S FROZEN SEATING when one exists, the live order
            // otherwise. A rollback session decides its handle count once, at
            // session start; a roster that re-sampled the live device order
            // could seat a fighter the session has no handle for, and both
            // would be citing "the connected controllers" (GPT 5.6,
            // 2026-07-28). Without a session — the shell's non-rollback
            // routes — the live order IS the answer.
            let frozen = topology.as_ref().filter(|topology| topology.is_frozen());
            let roster = versus_roster_from(
                frozen
                    .map(|topology| topology.players())
                    .unwrap_or_else(|| devices.devices().len().max(1)),
                frozen.map(|topology| topology.generation()),
            );
            // Demand the art before the bodies exist: a fighter seated with no
            // decoded sheet draws a placeholder, and the whole point of a visible
            // slice is that it looks like the two characters it says it is.
            roster.project_demand(&mut demand);
            commands.insert_resource(roster);
            // A NEW roster is not yet a match. Activation is seating's to
            // publish, once every participant has a body.
            commands.remove_resource::<ambition::actors::character_runtime::ActiveMatch>();
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
            *match_state = super::versus_rules::VersusMatch::opening();
        }
        (false, true) => {
            commands.remove_resource::<MatchParticipantRoster>();
            // The match ends WITH its route. An activation that outlives its
            // match is the next game inheriting somebody else's fighters.
            commands.remove_resource::<ambition::actors::character_runtime::ActiveMatch>();
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
    // The scoreboard. One health gauge PER SEAT plus two text readouts; the
    // engine never learns what a ROUND is — `publish_versus_hud` writes the
    // words.
    //
    // Four gauges are declared and a 1v1 fills two, because a HUD declaration is
    // a statement about the STAGE and the stage seats four. Declaring two and
    // making the third fighter share one is what the first version did.
    .with_hud({
        let mut hud = ambition::presentation::HudDeclaration::new();
        for (seat, slot) in super::versus_rules::HEALTH_HUD_SLOTS.iter().enumerate() {
            hud = hud.slot(
                ambition::presentation::HudSlotSpec::new(*slot)
                    .with_region(ambition::presentation::SurroundRegion::Top)
                    .with_font_size(22.0)
                    // The gauge's full extent is the slot's declared minimum
                    // width, so a stage sizes its bar by saying how much room it
                    // wants rather than by knowing anything about the renderer.
                    .with_min_px(ambition::engine_core::Vec2::new(220.0, 30.0))
                    // Coloured by SIDE, matching the roster's seat-parity teams,
                    // so a partner's bar reads as a partner's at a glance.
                    .with_color(if seat % 2 == 0 {
                        [0.55, 0.85, 1.0, 1.0]
                    } else {
                        [1.0, 0.6, 0.55, 1.0]
                    }),
            );
        }
        hud.slot(
            ambition::presentation::HudSlotSpec::new(super::versus_rules::ROUNDS_HUD_SLOT)
                .with_region(ambition::presentation::SurroundRegion::Top)
                .with_font_size(16.0)
                .with_color([0.75, 0.8, 0.95, 1.0]),
        )
        // The announce line: the round-start COUNTDOWN ("ROUND 2", then "FIGHT")
        // and the KO / match card. One slot for both because they are the same
        // thing — the words the stage says between exchanges — and a second slot
        // would let them overlap.
        //
        // ⚠ this said "published only while a round is over, so it needs no hide
        // path". True until the countdown landed (2026-07-28) and false after:
        // the count publishes every tick of a LIVE-adjacent phase, so
        // `publish_versus_hud` clears the slot explicitly on `Fighting`. Without
        // that the word FIGHT would sit over the whole round.
        .slot(
            ambition::presentation::HudSlotSpec::new(super::versus_rules::ANNOUNCE_HUD_SLOT)
                .centered()
                .with_font_size(34.0)
                .with_color([1.0, 0.85, 0.3, 1.0]),
        )
    })
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

    // The RULES on the sim schedule, after damage has resolved — they read this
    // tick's health outcome and write body position. `Settle` is the phase for
    // exactly that: "everything that reads this tick's damage outcome rather
    // than producing it".
    //
    // ALL of them. There was a second system in `Update` holding the KO card on
    // the render clock, and it mutated `VersusMatch` — which is rollback state,
    // so the restored score depended on presentation-frame history that
    // resimulation does not replay. Calling a system "the presentation half"
    // does not make the resource it writes presentational.
    {
        use ambition::platformer::schedule::{CombatSet, SimScheduleExt};
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            super::versus_rules::settle_versus_round.in_set(CombatSet::Settle),
        );
    }

    // Publishing the scoreboard IS presentation: it reads `VersusMatch` and
    // writes HUD text, and it writes nothing the simulation reads back.
    app.add_systems(Update, super::versus_rules::publish_versus_hud);

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
    app.add_systems(
        Update,
        (
            track_versus_roster,
            // AFTER, and in the same tick: the roster this built may have been
            // decided before the session froze its seating.
            reconcile_roster_with_frozen_topology,
        )
            .chain(),
    );
}

#[cfg(test)]
mod roster_topology_tests {
    use super::*;
    use ambition::actors::character_runtime::{ActiveMatch, CharacterLoadDemand};
    use ambition::input::{LocalDeviceOrder, LocalSeatTopology};
    use bevy::prelude::*;

    fn topology_of(pads: usize) -> LocalSeatTopology {
        let mut topology = LocalSeatTopology::default();
        topology.capture(&LocalDeviceOrder::from_devices(
            (0..pads as u32).filter_map(Entity::from_raw_u32).collect(),
        ));
        topology
    }

    fn app_with(roster: MatchParticipantRoster, pads: usize, active: bool) -> App {
        let mut app = App::new();
        app.insert_resource(roster);
        app.insert_resource(topology_of(pads));
        if active {
            // An ACTIVE match: seating published it, so its bodies are on the
            // stage and rebuilding the roster would reseat underneath them.
            app.insert_resource(ActiveMatch::for_test(0, None));
        }
        app.init_resource::<CharacterLoadDemand>();
        app.add_systems(Update, reconcile_roster_with_frozen_topology);
        app
    }

    /// **A roster decided before the session froze its seating is rebuilt.**
    ///
    /// The real sequence, not a contrived one: a route is entered — and its
    /// roster built from live device discovery — before its rollback session
    /// exists. A controller connecting in that gap left the roster seating two
    /// fighters into a session with three handles, both citing "the connected
    /// controllers". Nothing has a body yet at that point, so the roster is only
    /// an intention and rebuilding it costs nothing.
    #[test]
    fn a_roster_built_before_the_freeze_is_rebuilt_against_it() {
        let mut app = app_with(versus_roster_from(2, None), 3, false);
        app.update();
        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants.len(),
            3,
            "the roster kept the seat count it guessed from live devices while \
             the session had already frozen a different one"
        );
        assert_eq!(
            roster.seat_topology,
            Some(app.world().resource::<LocalSeatTopology>().generation()),
            "a rebuilt roster must record WHICH topology it agreed with, or the \
             next tick rebuilds it again forever"
        );
    }

    /// A roster already built from this topology is left completely alone —
    /// otherwise the reconciler rebuilds it every tick and the demand set grows
    /// without end.
    #[test]
    fn an_agreeing_roster_is_not_touched() {
        let topology = topology_of(2);
        let mut app = app_with(versus_roster_from(2, Some(topology.generation())), 2, false);
        let before = app.world().resource::<MatchParticipantRoster>().clone();
        app.update();
        assert_eq!(*app.world().resource::<MatchParticipantRoster>(), before);
    }

    /// **A topology that has decided nothing does not get to overrule live
    /// discovery.**
    ///
    /// `generation == 0` means never captured — the resource exists, no session
    /// has frozen its seating. Treating that as an answer would rebuild every
    /// roster down to the two-seat floor. Written because removing the
    /// `is_frozen` guard left the other three tests green: an unverified guard
    /// is one that gets deleted by the next person who reads it as noise.
    #[test]
    fn an_unfrozen_topology_does_not_overrule_the_roster() {
        let mut app = App::new();
        app.insert_resource(versus_roster_from(4, None));
        app.insert_resource(LocalSeatTopology::default());
        app.init_resource::<CharacterLoadDemand>();
        app.add_systems(Update, reconcile_roster_with_frozen_topology);
        app.update();
        assert_eq!(
            app.world()
                .resource::<MatchParticipantRoster>()
                .participants
                .len(),
            4,
            "a topology that has never been captured overruled a four-player \
             roster and cut it to the two-seat floor"
        );
    }

    /// **The stamp is repaired when the match and the session agree about WHO.**
    ///
    /// The ordinary sequence, and the one that made this worth building: the
    /// route builds its roster from live device discovery on entry, stamped
    /// `None` because no session has frozen anything yet; the fighters seat; the
    /// rollback session starts and freezes its topology. Now the roster
    /// disagrees with the session about which decision produced it while
    /// agreeing completely about who is fighting.
    ///
    /// Before this, that warned — on a stage where nothing was wrong. Y′9 left
    /// the post-seating case "reported rather than repaired"; repairing the
    /// repairable half is what makes the remaining report mean something.
    #[test]
    fn a_live_match_that_agrees_with_its_session_adopts_its_topology() {
        let mut app = app_with(versus_roster_from(1, None), 1, true);
        let generation = app.world().resource::<LocalSeatTopology>().generation();
        let fighters = app
            .world()
            .resource::<MatchParticipantRoster>()
            .participants
            .clone();

        app.update();

        let roster = app.world().resource::<MatchParticipantRoster>();
        assert_eq!(
            roster.participants, fighters,
            "the repair changed WHO is fighting; it may only correct the record \
             of which topology decided them"
        );
        assert_eq!(
            roster.seat_topology,
            Some(generation),
            "the roster and the session agree about the fighters and the roster \
             still records no topology, so the reconciler warns about a stage \
             where nothing is wrong — every tick, forever"
        );
        assert_eq!(
            app.world().resource::<ActiveMatch>().seat_topology(),
            Some(generation),
            "the ACTIVATION kept the stale stamp, so the next comparison asks \
             the same question again and the repair has to be redone"
        );
    }

    /// **After seating it is bodies on a stage, and it is left alone.**
    ///
    /// Reseating mid-match would be a worse bug than the disagreement it fixes.
    /// The disagreement is reported instead — this is the case match ACTIVATION
    /// exists to make impossible, and pretending a rebuild is safe here would
    /// hide the reason that work is still open.
    #[test]
    fn a_seated_match_is_never_reseated_underneath_itself() {
        let mut app = app_with(versus_roster_from(2, None), 3, true);
        app.update();
        assert_eq!(
            app.world()
                .resource::<MatchParticipantRoster>()
                .participants
                .len(),
            2,
            "a seated match was rebuilt underneath its own bodies"
        );
    }
}
