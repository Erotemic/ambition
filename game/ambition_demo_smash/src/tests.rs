//! Unit tests for the crate root, kept in a sibling file.
//!
//! The module-size gate counts inline `#[cfg(test)]` code toward its file but
//! excludes a sibling `tests.rs`.

use super::*;
use ambition_platformer2d::engine_core::AabbExt;

/// Swinging spends the respawn grant, and only that grant, on one body that
/// holds two.
///
/// Both grants are on the same fighter: the power-up it already had must
/// survive the respawn beat and the swing that ends it. (Two separate bodies
/// would pass for any implementation.)
#[test]
fn a_swing_spends_only_the_respawn_grant_on_a_body_that_holds_two() {
    use ambition_platformer2d::actors::features::empowerment::{Empowered, Empowerment};
    use ambition_platformer2d::characters::actor::{BodyHealth, Health, Invulnerability};

    let mut app = bevy::prelude::App::new();
    app.add_systems(bevy::prelude::Update, a_swing_spends_the_respawn_protection);
    app.add_observer(ambition_platformer2d::actor::retract_respawn_grace_on_removal);
    let mut health = BodyHealth::new(Health {
        current: 100,
        max: 100,
        invulnerable: Default::default(),
    });
    // The body already carries a power-up when it comes back.
    health
        .health
        .invulnerable
        .set(Invulnerability::EMPOWERED, true);
    health
        .health
        .invulnerable
        .set(Invulnerability::RESPAWN, true);
    let fighter = app
        .world_mut()
        .spawn((
            health,
            Empowered::held(Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT)),
            ambition_platformer2d::actor::RespawnGrace { remaining: 2.0 },
        ))
        .id();

    app.world_mut().entity_mut(fighter).insert(
        ambition_platformer2d::combat::moveset::MovePlayback::new(test_move(), 1.0),
    );
    app.update();

    let body = app.world().entity(fighter);
    assert!(
        body.get::<ambition_platformer2d::actor::RespawnGrace>()
            .is_none(),
        "the respawn grant survived the swing that spends it"
    );
    let invuln = body
        .get::<BodyHealth>()
        .expect("still a body")
        .health
        .invulnerable;
    assert!(
        !invuln.holds(Invulnerability::RESPAWN),
        "the respawn REASON outlived the grant that published it"
    );
    assert!(
        invuln.holds(Invulnerability::EMPOWERED),
        "swinging stripped a power-up this ruleset never granted — the \
         respawn beat is borrowing ownership it does not have"
    );
    assert!(
        body.get::<Empowered>().is_some(),
        "the power-up's whole component was removed with the respawn grant, \
         so every other trait it carried went with it"
    );
}

/// A minimal move for the test above: only its existence matters.
fn test_move() -> ambition_platformer2d::entity_catalog::MoveSpec {
    ambition_entity_catalog::authoring::strike(
        ambition_entity_catalog::authoring::Strike {
            id: "test_swing",
            clip: "attack",
            startup_s: 0.05,
            active_s: 0.05,
            recover_s: 0.10,
            offset: (10.0, 0.0),
            half_extents: (10.0, 10.0),
            damage: 1,
            knockback: 10.0,
            knockback_growth: 0.0,
            launch_dir: None,
            on_hit: None,
        },
    )
}

/// The platform lives exactly as long as the protection.
///
/// It has no clock of its own. The third assertion matters most: a grant that
/// expires on its own clock must take the platform with it, or a fighter that
/// never swings keeps the platform all match.
#[test]
fn the_respawn_platform_lives_exactly_as_long_as_the_grant() {
    use ambition_platformer2d::world::collision::MovingPlatformSet;

    let mut app = bevy::prelude::App::new();
    app.init_resource::<MovingPlatformSet>();
    app.add_systems(bevy::prelude::Update, hold_the_respawn_platforms);
    let fighter = app
        .world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(1),
            ambition_platformer2d::engine_core::BodyKinematics {
                pos: Vec2::new(120.0, 40.0),
                ..Default::default()
            },
            ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            },
        ))
        .id();

    app.update();
    let set = app.world().resource::<MovingPlatformSet>();
    assert_eq!(set.0.len(), 1, "a protected fighter got no platform");
    assert_eq!(set.0[0].id, respawn_platform_id(1));
    assert!(
        set.0[0].pos.y > 40.0,
        "the platform is above the fighter instead of under its feet: {:?}",
        set.0[0].pos
    );

    // The grant runs out with no swing: the component leaves, and the
    // platform must go with it.
    app.world_mut()
        .entity_mut(fighter)
        .remove::<ambition_platformer2d::actor::RespawnGrace>();
    app.update();
    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "the platform outlived the protection — the fighter that never \
         swings is exactly the one camping it"
    );
}

/// It leaves every other platform alone. The stage's platforms share the
/// resource, so clearing the Vec or retaining by position would delete them.
#[test]
fn holding_a_respawn_platform_does_not_touch_the_stages_own() {
    use ambition_platformer2d::world::collision::MovingPlatformSet;
    use ambition_platformer2d::world::platforms::MovingPlatformState;

    let mut app = bevy::prelude::App::new();
    app.insert_resource(MovingPlatformSet(vec![MovingPlatformState::from_sweep(
        "stage_lift",
        "Stage Lift",
        Vec2::new(0.0, 0.0),
        Vec2::new(64.0, 8.0),
        120.0,
        40.0,
    )]));
    app.add_systems(bevy::prelude::Update, hold_the_respawn_platforms);
    app.update();
    let set = app.world().resource::<MovingPlatformSet>();
    assert_eq!(set.0.len(), 1, "the stage's own platform was deleted");
    assert_eq!(set.0[0].id, "stage_lift");
}

/// The stage opens a window for every verb it grants.
///
/// A granted verb whose tuning window is zero is a dead grant: nothing refuses
/// or logs it, and the press does nothing. [`MatchAbilities::is_coherent`] asks
/// the same about the two ability statements; this asks it against the numbers
/// the verbs run on. The pairs are hand-listed against the kit on purpose.
#[test]
fn the_stages_body_opens_a_window_for_every_verb_the_stage_grants() {
    // The body a fighter that brings nothing of its own plays with: the
    // stage's numbers over the engine's.
    let body = SMASH_FIGHTER_BODY.over(ambition_platformer2d::engine_core::DEFAULT_TUNING);
    // The ceiling, not the floor: a verb the stage only permits (pogo) still
    // needs a live window.
    let kit = SMASH_FIGHTER_CEILING;
    let dead: Vec<&str> = [
        // (granted?, the number without which the verb does nothing, name)
        (kit.dodge, body.air_dodge_time, "dodge (in the air)"),
        (kit.dodge, body.dodge_roll_time, "dodge (on the ground)"),
        (kit.dodge, body.dodge_roll_speed, "dodge (on the ground)"),
        (kit.double_jump, f32::from(body.air_jumps), "double_jump"),
        (kit.fast_fall, body.fast_fall_speed, "fast_fall"),
        (kit.shield, body.parry_window_time, "shield (the parry)"),
        (
            kit.ledge_grab,
            body.ledge_momentum.window,
            "ledge_grab (the momentum carry)",
        ),
        (kit.pogo, body.pogo_speed, "pogo"),
    ]
    .into_iter()
    .filter(|(granted, window, _)| *granted && *window <= 0.0)
    .map(|(_, _, verb)| verb)
    .collect();
    assert!(
        dead.is_empty(),
        "the stage ALLOWS {dead:?} and supplies a body in which the verb \
         does nothing — see `MatchParticipantRoster::fighter_body`"
    );
    // Non-vacuity: every window above is non-zero in `DEFAULT_TUNING` except
    // the air dodge (0.0 by design), so this proves the stage's numbers apply.
    assert_eq!(
        ambition_platformer2d::engine_core::DEFAULT_TUNING.air_dodge_time,
        0.0,
        "the engine opened an air-dodge window by default, which changes \
         every exploration body in the game and makes this test vacuous"
    );
    assert!(
        body.air_dodge_time > 0.0,
        "the stage's body no longer opens the one window the engine \
         deliberately leaves shut"
    );
}

/// The roster declares the body. The test above measures the constant; this
/// checks the stage declares it, which a deletion would not break at compile.
#[test]
fn the_roster_supplies_the_fighters_body() {
    let roster = smash_roster(["player_robot_v3", "player_robot_v2"]);
    assert_eq!(
        roster.rules.body,
        Some(SMASH_FIGHTER_BODY),
        "the stage grants a platform fighter's verbs and supplies no body \
         to run them on"
    );
}

/// A stocks roster declares the pair the engine insists on.
#[test]
fn the_roster_declares_stocks_for_every_seat() {
    let roster = smash_roster(["player_robot_v3", "player_robot_v2"]);
    assert_eq!(roster.rules.stocks, Some(STARTING_STOCKS));
    assert_eq!(roster.participants.len(), 2);
    assert!(
        roster.rules.opens_suspended,
        "a fighter that can act during the countdown gets a free hit"
    );
}

/// Seat 0 is the human; everyone else is a CPU, so the demo plays with one
/// controller.
#[test]
fn the_first_seat_is_the_player_and_the_rest_are_cpus() {
    let roster = smash_roster(["a", "b", "c"]);
    assert!(matches!(
        roster.participants[0].controller,
        ControllerBinding::Human {
            source: ambition_platformer2d::actor::LocalInputSource::Pad(0)
        }
    ));
    for participant in &roster.participants[1..] {
        assert!(matches!(
            participant.controller,
            ControllerBinding::Cpu { .. }
        ));
    }
}

/// Every seat is its own side, so a free-for-all can have a last side
/// standing.
#[test]
fn every_seat_is_its_own_side() {
    let roster = smash_roster(["a", "b", "c", "d"]);
    let sides: std::collections::BTreeSet<_> = roster
        .participants
        .iter()
        .filter_map(|participant| participant.team.clone())
        .collect();
    assert_eq!(
        sides.len(),
        4,
        "seats share a side, so this match cannot end: the last-side-standing \
         rule never sees fewer than two"
    );
}

/// Sudden death is fought by the tied sides; the rest are out.
///
/// A non-contender must not get an even restart, and it must not stay at its
/// own low damage either, or the side that lost the tiebreak would enter the
/// round ahead. So it is eliminated with the same `FighterEliminated` an
/// exhausted fighter gets; `take_eliminated_fighters_out_of_play` clears it,
/// and `last_side_standing` decides among the contenders.
#[test]
fn only_the_tied_sides_are_carried_into_the_sudden_death_round() {
    use ambition_platformer2d::actors::features::stocks_match::SuddenDeathBegan;
    use ambition_platformer2d::characters::actor::{BodyHealth, Health};
    use ambition_platformer2d::combat::stocks::FighterEliminated;
    use ambition_platformer2d::combat::targeting::MatchTeam;

    let mut app = bevy::prelude::App::new();
    app.add_message::<SuddenDeathBegan>();
    app.init_resource::<ambition_platformer2d::presentation::HudReadouts>();
    app.add_systems(bevy::prelude::Update, open_the_sudden_death_round);

    // Three sides, seated as the roster seats them, each with its damage at
    // the timeout.
    let seat = |app: &mut bevy::prelude::App, index: usize, damage: i32| {
        let mut health = BodyHealth::new(Health {
            current: 100,
            max: 100,
            invulnerable: Default::default(),
        });
        health.set_damage_taken(damage);
        app.world_mut()
            .spawn((
                ambition_platformer2d::versus_match::MatchSeat(index),
                MatchTeam(format!("seat{index}")),
                health,
                // Two stocks each: a timed tie happens at any stock count
                // (the tiebreak arms tie at two), so the round must set one.
                ambition_platformer2d::combat::components::FighterStocks::new(2),
                ambition_platformer2d::combat::components::ActiveCombatant,
            ))
            .id()
    };
    let tied_a = seat(&mut app, 0, 80);
    let tied_b = seat(&mut app, 1, 80);
    let behind = seat(&mut app, 2, 12);

    // The engine's message naming the level sides. The labels come from
    // `side_label`, as in the fold that named them.
    app.world_mut().write_message(SuddenDeathBegan {
        starting_damage: 150,
        contenders: vec![
            ambition_platformer2d::combat::stocks::side_label(
                0,
                Some(&MatchTeam("seat0".to_string())),
            ),
            ambition_platformer2d::combat::stocks::side_label(
                1,
                Some(&MatchTeam("seat1".to_string())),
            ),
        ],
    });
    app.update();

    let damage_on = |app: &bevy::prelude::App, body| {
        app.world()
            .get::<BodyHealth>(body)
            .expect("the fighter still has a body")
            .damage_taken()
    };
    assert_eq!(
        (damage_on(&app, tied_a), damage_on(&app, tied_b)),
        (150, 150),
        "a tied side did not go to the authored sudden-death damage"
    );
    assert!(
        app.world().get::<FighterEliminated>(tied_a).is_none()
            && app.world().get::<FighterEliminated>(tied_b).is_none(),
        "a side that was TIED was retired from the round it is the point of"
    );

    assert!(
        app.world().get::<FighterEliminated>(behind).is_some(),
        "the side the clock had already put behind is still in the match, so \
         it gets to fight for a win the timeout had denied it"
    );
    assert_eq!(
        damage_on(&app, behind),
        12,
        "a retired side was ALSO put on the starting damage — the two arms \
         are exclusive, and doing both would leave a body that is out of the \
         match carrying the round's percent"
    );

    // One stock each: that is what makes it sudden death. Otherwise the first
    // KO spends a stock, eliminates nobody, and the respawn resets the damage.
    let stocks_of = |app: &bevy::prelude::App, body: bevy::prelude::Entity| {
        app.world()
            .get::<ambition_platformer2d::combat::components::FighterStocks>(body)
            .map(|s| s.remaining)
    };
    assert_eq!(
        (stocks_of(&app, tied_a), stocks_of(&app, tied_b)),
        (Some(1), Some(1)),
        "a contender entered sudden death still holding the stocks it had, so \
         the first knockout costs a stock instead of the round"
    );

    // Retirement is both halves, as in `spend_fighter_stocks`: the marker and
    // the removal of `ActiveCombatant`. A marker alone leaves a body with
    // attack state and a place on the anti-clump board.
    assert!(
        app.world()
            .get::<ambition_platformer2d::combat::components::ActiveCombatant>(behind)
            .is_none(),
        "a side retired by the timeout is still an ActiveCombatant — that is a \
         SECOND, weaker definition of leaving the match than the one stock \
         exhaustion uses"
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d::combat::components::ActiveCombatant>(tied_a)
            .is_some(),
        "a CONTENDER stopped being an active combatant, so the round has \
         nobody left to fight it"
    );
}

/// Two fighters do not come back to the same point.
///
/// The layout is symmetric about the centre and stays on the platform, so an
/// eight-seat roster is still a fair start.
#[test]
fn every_seat_comes_back_to_its_own_point_on_the_platform() {
    let centre = stage_centre();
    let seats: Vec<Vec2> = (0..8).map(|seat| respawn_placement(centre, seat)).collect();

    for (a, first) in seats.iter().enumerate() {
        for second in seats.iter().skip(a + 1) {
            assert!(
                (first.x - second.x).abs() >= RESPAWN_SEAT_SPACING_PX - 0.01,
                "two seats respawn within {RESPAWN_SEAT_SPACING_PX}px of each \
                 other, which is narrower than a standing body: {first:?} vs {second:?}"
            );
        }
    }

    // Symmetric about the centre: seats 0 and 1 straddle it evenly.
    assert!(
        ((seats[0].x - centre.x) + (seats[1].x - centre.x)).abs() < 0.01,
        "the first two seats are not symmetric about the stage centre"
    );

    // Every seat is over the platform, not past its lip.
    let half = PLATFORM_WIDTH / 2.0;
    for (seat, at) in seats.iter().enumerate() {
        assert!(
            (at.x - centre.x).abs() < half,
            "seat {seat} respawns {:.0}px from centre, past the {half:.0}px platform edge",
            (at.x - centre.x).abs()
        );
        assert!(
            at.y < centre.y,
            "seat {seat} respawns at or below the stage"
        );
    }
}

/// A respawn is above the stage, not on it: a fighter that comes back on the
/// floor comes back inside the opponent.
///
/// The height is this test's subject; the column belongs to
/// `every_seat_comes_back_to_its_own_point_on_the_platform`.
#[test]
fn a_respawn_is_above_the_stage_centre() {
    let centre = Vec2::new(400.0, 300.0);
    let respawn = respawn_placement(centre, 0);
    assert!(
        (respawn.x - centre.x).abs() <= RESPAWN_SEAT_SPACING_PX,
        "a respawn is within a seat spacing of the centre, not off across the stage"
    );
    assert!(
        respawn.y < centre.y,
        "the respawn is at or below the stage floor, so a returning fighter \
         materialises inside whatever is standing there"
    );
}

/// The stage is a platform surrounded by nothing; every other room is a box
/// you cannot leave.
#[test]
fn the_stage_is_a_platform_you_can_be_knocked_off() {
    let room = smash_stage();
    assert_eq!(room.id, SMASH_STAGE_ROOM_ID);
    assert_eq!(
        room.world.blocks.len(),
        1,
        "a fighter stage with walls is a room, and a body knocked into one \
         comes back — the emptiness IS the mechanic"
    );
    let platform = room.world.blocks[0].aabb;
    assert!(
        platform.width() < room.world.size.x,
        "the platform spans the stage, so there is no off to be knocked"
    );
}

/// The blast envelope is authored from the fighting platform.
///
/// The room rectangle is an implementation seam, not the thing whose size
/// should determine knockout timing. Pin the normalized Final Destination
/// proportions directly so a future room resize cannot silently move the
/// death lines relative to the ledges.
#[test]
fn the_stage_and_blast_envelope_keep_their_authored_proportions() {
    let room = smash_stage();
    let world = &room.world;
    let platform = world.blocks[0].aabb;
    let side_margin = world
        .edges
        .side
        .expect("the smash stage authors side blast lines");
    let ceiling_margin = world
        .edges
        .rise
        .expect("the smash stage authors a ceiling blast line");

    let left_ledge_to_blast = platform.left() + side_margin;
    let right_ledge_to_blast = (world.size.x - platform.right()) + side_margin;
    let surface_to_ceiling_blast = platform.top() + ceiling_margin;
    let surface_to_fall_blast = (world.size.y - platform.top()) + world.edges.fall;

    assert_eq!(platform.width(), PLATFORM_WIDTH);
    assert_eq!(left_ledge_to_blast, PLATFORM_WIDTH);
    assert_eq!(right_ledge_to_blast, PLATFORM_WIDTH);
    assert_eq!(surface_to_ceiling_blast, PLATFORM_WIDTH * 1.125);
    assert_eq!(surface_to_fall_blast, PLATFORM_WIDTH * 0.875);
    assert_eq!(world.size.x + side_margin * 2.0, PLATFORM_WIDTH * 3.0);
    assert_eq!(
        world.size.y + ceiling_margin + world.edges.fall,
        PLATFORM_WIDTH * 2.0
    );
}

/// The room carries the demo's MODE, so its rules sleep everywhere else.
#[test]
fn the_stage_carries_the_smash_mode() {
    assert_eq!(smash_stage().metadata.mode.as_deref(), Some(SMASH_MODE));
}

/// A respawn lands above the platform, not the stage's middle. They coincide
/// here; a future stage may separate them.
#[test]
fn a_respawn_lands_over_the_platform() {
    let room = smash_stage();
    let platform = room.world.blocks[0].aabb;
    let respawn = respawn_placement(stage_centre(), 0);
    assert!(
        respawn.x >= platform.left() && respawn.x <= platform.right(),
        "a respawning fighter is dropped past the edge of the platform it is \
         supposed to come back to"
    );
    assert!(
        respawn.y < platform.top(),
        "the respawn is not above the stage"
    );
}

/// Run `announce_the_winner` over one settled match and hand back the
/// announce slot's text, or `None` if nothing was written to it.
///
/// It models production: a `StocksMatchSettled` latch stamped against the
/// active match, which the reader compares. Writing `StocksMatchDecided` would
/// test nothing, because the card does not read it. No
/// `ConfirmedFrameBoundary` is inserted: an absent boundary confirms
/// everything (the eager host).
fn announced_outcome(outcome: ambition_platformer2d::actor::MatchVerdict) -> Option<String> {
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::presentation::HudReadouts>();
    let active = ambition_platformer2d::versus_match::ActiveMatch::for_test(2, None);
    let mut settled =
        ambition_platformer2d::versus_match::StocksMatchSettled::default();
    settled.settle(&active, outcome);
    app.insert_resource(active);
    app.insert_resource(settled);
    app.add_systems(Update, announce_the_winner);
    app.update();

    app.world()
        .resource::<ambition_platformer2d::presentation::HudReadouts>()
        .get(&SMASH_ANNOUNCE_HUD_SLOT.into())
        .map(ambition_platformer2d::presentation::HudReadout::text)
}

/// The card says who won.
///
/// Asserted against the readout the stage declares and the HUD renders, not
/// by calling `victory_banner`, so it tests the wiring. A readout is a map
/// insert, so writing it twice is harmless.
#[test]
fn deciding_the_match_shows_a_card_naming_the_winner() {
    // The wording comes from `victory_banner`. No bodies are seated, so the
    // card falls back to the side label; that fallback is part of the claim.
    use ambition_platformer2d::actor::MatchVerdict;
    let seat_two = MatchVerdict::Winner("seat 2".to_string());
    assert_eq!(
        announced_outcome(seat_two.clone()).as_deref(),
        Some(victory_banner(&seat_two, Some("seat 2")).as_str()),
        "the ending wrote no announce card, so the stage says nothing about \
         who won"
    );
}

/// The two winnerless endings stay distinct, and only one gets a card.
///
/// A draw and a no contest both have no winner, so `winner.is_none()` cannot
/// tell them apart; `MatchVerdict` does. A mutual ring-out gets a card; an
/// abandoned match does not (Jon's call). The no-contest wording is still
/// proven by `every_ending_has_its_own_words`, so restoring the card is one
/// line.
#[test]
fn a_draw_is_announced_and_an_abandoned_match_is_not() {
    use ambition_platformer2d::actor::MatchVerdict;
    let drawn = announced_outcome(MatchVerdict::Draw).expect("a draw is still an ending");
    assert!(
        drawn.contains("Draw"),
        "a draw was announced as something else: {drawn}"
    );
    assert_eq!(
        announced_outcome(MatchVerdict::NoContest),
        None,
        "an abandoned match still put a result card up, so the press to leave \
         is still behind a readout the player has to sit through"
    );
}

/// The demo is something a player can enter: a roster, a stage and a ruleset,
/// assembled and routable.
#[test]
fn a_host_composing_this_plugin_can_route_to_the_stage() {
    use ambition_platformer2d::game_shell::{
        MinimalShellPlugins, ShellExperienceId, ShellExperienceRegistry, ShellRouteCatalog,
        ShellRouteId,
    };
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(MinimalShellPlugins);
    app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
    app.add_plugins(SmashExperiencePlugin);

    let registration = app
        .world()
        .resource::<ShellExperienceRegistry>()
        .get(&ShellExperienceId::new(SMASH_EXPERIENCE))
        .expect("a host that composed this plugin lists the smash experience");
    assert_eq!(
        registration.launch_route.as_str(),
        SMASH_SELECT_ROUTE,
        "a launcher row for this demo opens CHARACTER SELECT; entering at the \
         stage would seat whoever the host happened to have lying around"
    );
    let select = app
        .world()
        .resource::<ShellRouteCatalog>()
        .get(&ShellRouteId::new(SMASH_SELECT_ROUTE))
        .expect("the select screen is a registered route, not an app's home only");
    assert_eq!(
        select.experience.as_str(),
        SMASH_SELECT_EXPERIENCE,
        "the screen is a frontend experience of its own: under the gameplay id \
         the shell would try to activate a session that has nothing prepared"
    );
    assert!(
        select.preparation.is_none(),
        "nothing is loading on a character select"
    );

    let route = app
        .world()
        .resource::<ShellRouteCatalog>()
        .get(&ShellRouteId::new(SMASH_GAMEPLAY_ROUTE))
        .expect("the session route is registered");
    assert!(
        route.preparation.is_some(),
        "the route has no preparation, so entering it would drop a player into \
         a stage whose content was never prepared"
    );

    let authored = app
        .world()
        .resource::<ambition_platformer2d::provider::PlatformerAuthoredCatalogRegistry>()
        .get(SMASH_EXPERIENCE)
        .expect("the host sees this demo's authored catalogs");
    assert_eq!(authored.starting_character, SMASH_CHARACTER_ID);
}

/// The stage declares a DI budget and gives it back on the way out.
///
/// Without a declaration `di_max_angle` falls to the engine baseline `0.0`,
/// and a launched fighter has no influence. The release matters as much: left
/// standing, the budget would follow the player into Ambition's PvE.
#[test]
fn the_stage_declares_its_di_budget_and_releases_it() {
    use ambition_platformer2d::game_shell::{MinimalShellPlugins, ShellExperienceScopes};
    use bevy::prelude::*;

    assert!(
        SMASH_DI_MAX_ANGLE > 0.0,
        "⛔ a zero budget makes `di_adjust` a no-op, so declaring the rules at all would be \
         theatre — DI would be off and every test still green"
    );
    // The same trap, one field over.
    assert!(
        SMASH_KNOCKBACK_GROWTH > 0.0,
        "⛔ a platform fighter whose launch does not grow with percent is a \
         fighting game with no comeback and no kill: every basic swing here \
         is prefab-derived and authors `knockback_growth: 0.0`, so this declaration \
         is the ONLY thing that makes a worn opponent fly"
    );

    let mut app = App::new();
    app.add_plugins(MinimalShellPlugins);
    app.add_plugins(ambition_platformer2d::load::AmbitionLoadPlugin);
    app.add_plugins(SmashExperiencePlugin);

    let rules = std::any::type_name::<ambition_platformer2d::combat::rules::DeclaredCombatRules>();
    let released: Vec<&str> = app
        .world()
        .resource::<ShellExperienceScopes>()
        .iter()
        .filter(|scope| scope.owner().as_str() == SMASH_EXPERIENCE)
        .flat_map(|scope| scope.released_state())
        .collect();
    assert!(
        released.contains(&rules),
        "⛔ the stage's DI budget outlives its own experience and follows the player into a \
         game that authored none. Released: {released:?}"
    );
}

/// Run the preparation source as the system it is, with one stage choice.
/// The seam takes a system that may read the provider's resources.
fn prepared_world_for(
    choice: crate::SmashStageChoice,
) -> ambition_platformer2d::runtime::PreparedPlatformerSource {
    use bevy::ecs::system::RunSystemOnce as _;
    let mut world = bevy::prelude::World::new();
    world.insert_resource(choice);
    world
        .run_system_once(crate::smash_prepared_session_world)
        .expect("the preparation source runs as a system")
}

/// The choice picks the starting stage, and every stage stays in the set.
/// A source that returned only the chosen room would pass the geometry
/// assertion and make the others unreachable.
#[test]
fn the_stage_choice_decides_which_stage_the_match_prepares() {
    let flat = prepared_world_for(crate::SmashStageChoice::Flat);
    let platforms = prepared_world_for(crate::SmashStageChoice::Platforms);

    assert_eq!(
        flat.geometry().0.blocks.len(),
        1,
        "the flat choice did not prepare the single-surface stage"
    );
    assert_eq!(
        platforms.geometry().0.blocks.len(),
        4,
        "the platforms choice prepared a stage without its three tiers"
    );
    let narrow = prepared_world_for(crate::SmashStageChoice::Narrow);
    assert_eq!(
        narrow.geometry().0.blocks.len(),
        1,
        "the narrow choice is a single surface like the flat one, only shorter"
    );

    // The narrow stage: less ground, same envelope.
    let width = |w: &crate::ae::World| w.blocks[0].aabb.max.x - w.blocks[0].aabb.min.x;
    assert!(
        width(&narrow.geometry().0) < width(&flat.geometry().0),
        "the narrow stage is not narrower than the flat one"
    );

    // The same blast geometry for all three. `smash_stage_room` makes this
    // structural; the check still catches a future stage built by hand with
    // the public `RoomSpec::new` instead of the builder.
    for (name, other) in [("platforms", &platforms), ("narrow", &narrow)] {
        assert_eq!(
            flat.geometry().0.edges.side,
            other.geometry().0.edges.side,
            "{name} moved the side blast line, so it cannot be compared with flat"
        );
        assert_eq!(
            flat.geometry().0.edges.fall,
            other.geometry().0.edges.fall,
            "{name} moved the fall blast line, so it cannot be compared with flat"
        );
    }

    // The default is still the stage recorded measurements used.
    assert_eq!(crate::SmashStageChoice::default(), crate::SmashStageChoice::Flat);
    // The stage button's cycle returns to its start after a full lap; a
    // fourth stage breaks this line.
    assert_eq!(
        crate::SmashStageChoice::Flat.next().next().next(),
        crate::SmashStageChoice::Flat,
        "the stage cycle does not return to its start in three steps"
    );
    assert_ne!(
        crate::SmashStageChoice::Flat.next().next(),
        crate::SmashStageChoice::Flat,
        "the cycle closed early — a stage is unreachable from the button"
    );
}

/// Every stage is reachable by the cycle and by its own name.
///
/// `next()` and `ladder_rig --stage` both read `ALL`, so a variant missing from
/// `ALL` is authored and unreachable. The compiler catches a new variant in
/// `room_id` and `label` (exhaustive matches) but not a missing `ALL` entry.
/// An associated-const compile-time check does not work: the block is never
/// evaluated. So this is a runtime count: the exhaustive `match` forces a new
/// variant to be considered, and the length comparison forces it into `ALL`.
#[test]
fn all_lists_every_variant() {
    // Exhaustive: a new variant fails to compile here, in the test that says
    // to add it to `ALL`.
    fn counted(stage: crate::SmashStageChoice) -> usize {
        match stage {
            crate::SmashStageChoice::Flat
            | crate::SmashStageChoice::Platforms
            | crate::SmashStageChoice::Narrow => 1,
        }
    }
    let total: usize = [
        crate::SmashStageChoice::Flat,
        crate::SmashStageChoice::Platforms,
        crate::SmashStageChoice::Narrow,
    ]
    .into_iter()
    .map(counted)
    .sum();
    assert_eq!(
        total,
        crate::SmashStageChoice::ALL.len(),
        "a stage is matched above but missing from ALL — `next()` and `--stage` \
         both read ALL, so it would be authored and unreachable"
    );

    fn counted_stocks(count: crate::SmashStockChoice) -> usize {
        match count {
            crate::SmashStockChoice::One
            | crate::SmashStockChoice::Three
            | crate::SmashStockChoice::Five => 1,
        }
    }
    let stock_total: usize = [
        crate::SmashStockChoice::One,
        crate::SmashStockChoice::Three,
        crate::SmashStockChoice::Five,
    ]
    .into_iter()
    .map(counted_stocks)
    .sum();
    assert_eq!(
        stock_total,
        crate::SmashStockChoice::ALL.len(),
        "a stock count is matched above but missing from ALL"
    );
}

#[test]
fn every_stage_is_reachable_by_the_cycle_and_names_itself_uniquely() {
    use std::collections::BTreeSet;

    let all = crate::SmashStageChoice::ALL;
    assert!(all.len() >= 3, "the stage-breadth checkpoint wants several");

    // Distinct labels, or `--stage <name>` is ambiguous; distinct room ids,
    // or two stages are the same room.
    let labels: BTreeSet<String> =
        all.iter().map(|s| s.label().to_ascii_lowercase()).collect();
    assert_eq!(labels.len(), all.len(), "two stages share a label: {labels:?}");
    let rooms: BTreeSet<&str> = all.iter().map(|s| s.room_id()).collect();
    assert_eq!(rooms.len(), all.len(), "two stages share a room id: {rooms:?}");

    // Walking `next()` from the default visits every stage and comes back,
    // as the select screen's single button does.
    let mut seen = BTreeSet::new();
    let mut walk = crate::SmashStageChoice::default();
    for _ in 0..all.len() {
        seen.insert(walk.room_id());
        walk = walk.next();
    }
    assert_eq!(
        walk,
        crate::SmashStageChoice::default(),
        "the cycle does not return to its start after one lap"
    );
    assert_eq!(seen, rooms, "the button cannot reach every authored stage");
}

/// The prepared source carries the stage, not a default room.
///
/// A closure that returns the wrong room fails nowhere: the player just lands
/// in another level.
#[test]
fn the_prepared_session_is_the_smash_stage() {
    let prepared = prepared_world_for(crate::SmashStageChoice::default());
    assert_eq!(
        prepared.starting_character().character_id.as_str(),
        SMASH_CHARACTER_ID
    );
    assert_eq!(
        prepared.geometry().0.blocks.len(),
        1,
        "the prepared geometry is not the one-platform stage"
    );
    assert_eq!(
        prepared.geometry().0.edges.side,
        Some(SIDE_BLAST_MARGIN_PX),
        "the prepared geometry lost the stage's blast margins, so a fighter \
         knocked off would drift instead of dying"
    );
}

/// The match declares what 100% means, so a crossover fighter cannot bring its
/// own.
///
/// A character with no vitals gets a one-hit pool. Under `DeathPolicy::
/// Unbounded` it never kills, so only the percent is wrong (a 140-damage hit
/// read 14000%). A per-character fix missed fighters from other games (Mary-O
/// and Sanic author `max_health: 1`). So this asserts on the roster; the
/// character-side write is deleted.
#[test]
fn the_match_declares_the_pool_every_fighters_percent_is_read_against() {
    let mut roster = ambition_platformer2d::actor::MatchParticipantRoster::of(["mary_o", "sanic"]);
    apply_smash_match_rules(&mut roster, STARTING_STOCKS);
    assert_eq!(
        roster.rules.health_pool,
        Some(SMASH_PERCENT_REFERENCE),
        "a stocks match that does not declare its own pool reads each seat's \
         percent against whatever that character's HOME GAME authored"
    );
    // The reference makes percent comparable across characters; one would
    // make every hit read in the thousands.
    assert!(
        SMASH_PERCENT_REFERENCE >= 50,
        "a percent reference of {SMASH_PERCENT_REFERENCE} makes a single hit \
         read in the hundreds, which is the 14000% bug in a smaller hat"
    );
}

/// Every difficulty this demo can ask for is a published policy.
///
/// `smash_roster_at_levels` builds `duelist_l{level}` keys that must resolve as
/// `autonomous_profiles`. On a miss, `seat_brain_profile` finds nothing and
/// preparation refuses the seat loudly.
#[test]
fn every_authored_difficulty_is_a_published_controller_policy() {
    use ambition_platformer2d::characters::actor::character_catalog::{
        parse_catalog, CharacterCatalog,
    };

    let catalog = CharacterCatalog::from_data(parse_catalog(SMASH_CATALOG_RON));
    let profiles = &catalog.data().autonomous_profiles;
    for level in [1u8, 3, 5, 6, 9] {
        let key = format!("{SMASH_DUELIST_BRAIN}_l{level}");
        let profile = profiles.get(&key).unwrap_or_else(|| {
            panic!(
                "`smash_roster_at_levels` builds the key `{key}`, and no policy \
                 publishes it — that seat is refused. Published: {:?}",
                profiles.keys().collect::<Vec<_>>()
            )
        });
        assert_eq!(
            profile.fighter_level, level,
            "`{key}` publishes level {} — the ladder was the ONLY thing the \
             six deleted archetype rows differed in, so getting it wrong \
             loses the entire content of that deletion",
            profile.fighter_level
        );
        assert_eq!(
            profile.template,
            ambition_platformer2d::characters::brain::CharacterBrainTemplate::Fighter,
            "`{key}` is not a Fighter, so this seat is not a fighter"
        );
    }
    // And the unlevelled name the default seats use.
    assert!(
        profiles.contains_key(SMASH_DUELIST_BRAIN),
        "the bare `duelist` policy is gone, so an ordinary CPU seat is refused"
    );
}

/// The `duelist` preset resolves to the fighter brain. An unresolved preset
/// stands still, which looks the same as a brain that was never installed.
#[test]
fn the_duelist_preset_is_a_fighter_brain() {
    use ambition_platformer2d::characters::actor::character_catalog::{
        parse_catalog, CharacterCatalog,
    };

    let catalog = CharacterCatalog::from_data(parse_catalog(SMASH_CATALOG_RON));
    assert!(
        catalog.has_brain_preset("duelist"),
        "the catalog does not know the `duelist` preset at all, so every \
         fighter asking for it silently stands still"
    );
    let brain = catalog
        .build_brain_from_preset(
            "duelist",
            &ambition_platformer2d::characters::actor::character_catalog::BrainBuildContext::at(
                0.0,
            ),
        )
        .expect("the `duelist` preset builds a brain");
    assert_eq!(
        brain.label(),
        "fighter",
        "`duelist` resolved to `{}` — a preset that does not resolve falls \
         back to standing still, and a fighter that stands still looks \
         exactly like one with no brain at all",
        brain.label()
    );
}

/// Every verdict has words, and the winner's are a name, not a side label,
/// when the caller can resolve one.
#[test]
fn every_ending_has_its_own_words() {
    use ambition_platformer2d::actor::MatchVerdict;
    let seat = MatchVerdict::Winner("seat 2".to_string());
    assert_eq!(
        victory_banner(&seat, Some("Robot v3")),
        "WINNER: Robot v3",
        "the card printed the SIDE when a name was available, which is what \
         Jon was looking at when he asked about `seat 2 wins`"
    );
    assert_eq!(
        victory_banner(&seat, None),
        "WINNER: seat 2",
        "with no name resolved, the side is the honest answer"
    );
    assert!(victory_banner(&MatchVerdict::Draw, None).contains("Draw"));
    assert!(victory_banner(&MatchVerdict::NoContest, None).contains("NO CONTEST"));
}

/// Walking off the respawn platform ends the protection, as in Smash.
///
/// The sibling below pins that the block does not follow the body. This pins
/// that a platform you stepped off does not stay for the whole grace window.
#[test]
fn walking_off_the_respawn_platform_ends_the_protection() {
    use ambition_platformer2d::world::collision::MovingPlatformSet;

    let mut app = bevy::prelude::App::new();
    app.init_resource::<MovingPlatformSet>();
    use bevy::prelude::IntoScheduleConfigs as _;
    app.add_systems(
        bevy::prelude::Update,
        (
            hold_the_respawn_platforms,
            leaving_the_platform_spends_the_respawn_protection,
        )
            .chain(),
    );
    let fighter = app
        .world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(1),
            ambition_platformer2d::engine_core::BodyKinematics {
                pos: Vec2::new(120.0, 40.0),
                ..Default::default()
            },
            ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            },
        ))
        .id();

    app.update();
    assert!(
        app.world()
            .entity(fighter)
            .contains::<ambition_platformer2d::actor::RespawnGrace>(),
        "precondition: standing on the platform keeps the grant",
    );

    // A step just past the half-width, the boundary the rule is about.
    // Moving 200px would pass even for a "leave the stage" rule.
    app.world_mut()
        .entity_mut(fighter)
        .get_mut::<ambition_platformer2d::engine_core::BodyKinematics>()
        .unwrap()
        .pos = Vec2::new(120.0 + RESPAWN_PLATFORM_SIZE.x * 0.5 + 1.0, 40.0);
    app.update();

    assert!(
        !app.world()
            .entity(fighter)
            .contains::<ambition_platformer2d::actor::RespawnGrace>(),
        "a fighter that walked off its respawn platform keeps the protection, so \
         the platform stays in play for the whole window — which is what 'the \
         platform moves with you' looks like from the outside",
    );
}

/// The control arm: falling toward the platform is not leaving it. The rule
/// is horizontal, so the grant survives the fall it exists to protect.
#[test]
fn falling_toward_the_respawn_platform_keeps_the_protection() {
    use ambition_platformer2d::world::collision::MovingPlatformSet;

    let mut app = bevy::prelude::App::new();
    app.init_resource::<MovingPlatformSet>();
    use bevy::prelude::IntoScheduleConfigs as _;
    app.add_systems(
        bevy::prelude::Update,
        (
            hold_the_respawn_platforms,
            leaving_the_platform_spends_the_respawn_protection,
        )
            .chain(),
    );
    let fighter = app
        .world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(1),
            ambition_platformer2d::engine_core::BodyKinematics {
                pos: Vec2::new(120.0, 40.0),
                ..Default::default()
            },
            ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            },
        ))
        .id();
    app.update();

    // Straight down a long way, same column.
    app.world_mut()
        .entity_mut(fighter)
        .get_mut::<ambition_platformer2d::engine_core::BodyKinematics>()
        .unwrap()
        .pos = Vec2::new(120.0, 400.0);
    app.update();

    assert!(
        app.world()
            .entity(fighter)
            .contains::<ambition_platformer2d::actor::RespawnGrace>(),
        "descending onto your own platform is not leaving it",
    );
}

/// The respawn platform stays where it was placed.
///
/// If its centre followed `kin.pos`, a brain's distance to the edge would be a
/// constant 48px and it would veto every verb (`D-BRAIN-PLATFORM-FLOOR`). A
/// respawn platform is somewhere you leave.
#[test]
fn the_respawn_platform_stays_where_it_was_placed() {
    use ambition_platformer2d::world::collision::MovingPlatformSet;

    let mut app = bevy::prelude::App::new();
    app.init_resource::<MovingPlatformSet>();
    app.add_systems(bevy::prelude::Update, hold_the_respawn_platforms);
    let fighter = app
        .world_mut()
        .spawn((
            ambition_platformer2d::actor::MatchSeat(1),
            ambition_platformer2d::engine_core::BodyKinematics {
                pos: Vec2::new(120.0, 40.0),
                ..Default::default()
            },
            ambition_platformer2d::actor::RespawnGrace {
                remaining: RESPAWN_PROTECTION_SECONDS,
            },
        ))
        .id();

    app.update();
    let placed = app.world().resource::<MovingPlatformSet>().0[0].pos;

    // The fighter walks off it.
    app.world_mut()
        .entity_mut(fighter)
        .get_mut::<ambition_platformer2d::engine_core::BodyKinematics>()
        .unwrap()
        .pos = Vec2::new(320.0, 40.0);
    app.update();

    let set = app.world().resource::<MovingPlatformSet>();
    assert_eq!(set.0.len(), 1, "the platform vanished or was duplicated");
    assert_eq!(
        set.0[0].pos, placed,
        "the platform followed the fighter 200px. A floor defined as `wherever \
         I am` can never be walked off, and every ledge question asked against \
         it answers the same thing forever"
    );
}

/// The platformed stage has three one-way tiers above its solid floor.
///
/// The sign matters: `y` grows downward, so a tier above the stage has a
/// smaller `y`. Backwards, the tiers are buried and invisible in the floor.
#[test]
fn the_platform_stage_puts_three_one_way_tiers_above_its_solid_floor() {
    use ambition_platformer2d::engine_core::BlockKind;

    let stage = crate::smash_platform_stage();
    let blocks = &stage.world.blocks;

    let solid: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Solid { .. }))
        .collect();
    let one_way: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::OneWay))
        .collect();
    assert_eq!(solid.len(), 1, "the main stage surface is one solid block");
    assert_eq!(one_way.len(), 3, "three drop-through tiers");

    let floor_top = solid[0].aabb.min.y;
    for tier in &one_way {
        assert!(
            tier.aabb.max.y < floor_top,
            "tier `{}` is at y {}..{}, not ABOVE the floor at {floor_top} — y \
             grows downward, so a platform above the stage has the SMALLER y",
            tier.name,
            tier.aabb.min.y,
            tier.aabb.max.y
        );
    }

    // The original stage is untouched: one solid block, no tiers.
    let flat = crate::smash_stage();
    assert_eq!(flat.world.blocks.len(), 1);
    assert!(!flat
        .world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::OneWay)));
}

/// Every tier is reachable.
///
/// The single-jump apex is 88.2px and the ceiling (an air jump at the apex)
/// is 148.3px. A platform nobody can reach fails no assertion and renders
/// fine. Recomputed from the engine's constants, so retuning gravity or jump
/// speed breaks this.
#[test]
fn the_tiers_sit_inside_the_fighters_measured_jump_arc() {
    use ambition_platformer2d::engine_core::{DOUBLE_JUMP_SPEED, GRAVITY, JUMP_SPEED};

    // The engine's own formula, from `FighterBodyAuthoring::jump_speed`.
    let apex = |v: f32| v * v / (2.0 * GRAVITY);
    let single = apex(JUMP_SPEED);
    let with_air_jump = single + apex(DOUBLE_JUMP_SPEED);

    assert!(
        crate::SOFT_PLATFORM_LOW_RISE < single,
        "the low tier at {}px is above the {single:.1}px single-jump apex, so \
         reaching it needs an air jump — that is the TOP tier's job",
        crate::SOFT_PLATFORM_LOW_RISE
    );
    assert!(
        crate::SOFT_PLATFORM_HIGH_RISE > single,
        "the top tier at {}px is inside a single jump, so the two tiers ask the \
         same question of the player",
        crate::SOFT_PLATFORM_HIGH_RISE
    );
    assert!(
        crate::SOFT_PLATFORM_HIGH_RISE < with_air_jump,
        "the top tier at {}px is above the {with_air_jump:.1}px ceiling — it is \
         scenery, not a platform",
        crate::SOFT_PLATFORM_HIGH_RISE
    );
}
