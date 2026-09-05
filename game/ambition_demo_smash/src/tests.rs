//! Extracted from `lib.rs` on 2026-08-30, unchanged.
//!
//! ⭐ THE MODULE-SIZE GATE COUNTS INLINE `#[cfg(test)]` TOWARD ITS FILE and
//! excludes a sibling `tests.rs` centrally, so a crate's own convention —
//! `#[cfg(test)] mod tests;` in a file of its own — is the sanctioned way to
//! bring a module back under the limit without moving a line of production
//! code. `lib.rs` was 5062 lines, of which 1640 were these two modules.

use super::*;
use ambition_platformer2d::engine_core::AabbExt;

/// ⭐ SWINGING SPENDS THE RESPAWN PROTECTION — AND ONLY IT, ON THE SAME
/// BODY.
///
/// ⛔⛔ THE FIRST VERSION OF THIS TEST PROVED THE WRONG THING. It gave two
/// DIFFERENT bodies an `Empowered` and checked that swinging on one left the
/// other alone — which is true of any implementation and says nothing about
/// ownership. The claim being made is about ONE body holding two grants, and
/// the implementation could not honour it: `Empowered` is a single component,
/// so respawn protection granted through it OVERWROTE whatever the body was
/// already carrying, and removing it took every semantic with it.
///
/// ⇒ this fixture puts both on the SAME fighter. A power-up that survives
/// the respawn beat, and survives the swing that ends it, is the property.
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
    // The body is ALREADY carrying a power-up when it comes back — the case
    // the borrowed-`Empowered` version silently destroyed.
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
    ambition_platformer2d::characters::moveset_authoring::strike(
        ambition_platformer2d::characters::moveset_authoring::Strike {
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

/// ⭐ THE PLATFORM LIVES EXACTLY AS LONG AS THE PROTECTION, AND NOT A TICK
/// LONGER.
///
/// A returning fighter used to appear in free air with nothing but an
/// invisible timer. The platform is that timer made readable — so it must
/// have no clock of its own, or the two disagree and a fighter stands on a
/// beat it has already spent.
///
/// ⛔⛔ THE THIRD ASSERTION IS THE ONE THAT MATTERS: an `Empowered` that
/// expires on its OWN clock must take the marker and the platform with it. A
/// latch cleared only by the swing would leave a platform standing for the
/// rest of the match, and the fighter that never swings is exactly the one
/// camping it.
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

    // The grant runs out — the platform must go with it, without anybody
    // having swung. ⭐ the grant owns its OWN clock now, so this is the
    // component leaving rather than a second timer being consulted.
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

/// ⭐ AND IT LEAVES EVERY OTHER PLATFORM ALONE. The stage's own platforms
/// share the resource; a respawn platform that cleared the Vec, or that was
/// retained by position rather than by its id prefix, would delete the stage.
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

/// THE STAGE OPENS A WINDOW FOR EVERY VERB IT GRANTS. ( slice 1b)
///
/// a granted verb whose tuning window is zero is a DEAD GRANT, and
/// it is invisible: nothing refuses it, nothing logs it, and the press
/// simply means nothing. [`MatchAbilities::is_coherent`] asks the same
/// question about the two ABILITY statements — *is everything granted also
/// permitted* — and this is the same question one layer down, against the
/// numbers the verbs run on.
///
/// the pairs are hand-listed and that is the point: adding a verb to
/// [`SMASH_FIGHTER_KIT`] whose window the engine defaults to zero is exactly
/// the mistake this catches, and only a list written against the KIT can
/// catch it. The air dodge is here because it was the one that bit; the
/// others are here because they are the rest of what the stage promises.
#[test]
fn the_stages_body_opens_a_window_for_every_verb_the_stage_grants() {
    // What a fighter that brought nothing of its own plays with here: the
    // stage's numbers over the engine's, which is the body twelve of the
    // fourteen grid fighters actually get.
    let body = SMASH_FIGHTER_BODY.over(ambition_platformer2d::engine_core::DEFAULT_TUNING);
    // ⭐ THE CEILING, NOT THE FLOOR. A verb the stage merely PERMITS still
    // needs a live window: a fighter that authored pogo and reaches a stage
    // whose body has no `pogo_speed` has a verb that does nothing, which is
    // the same defect one row down. Reading the floor here would have gone
    // quietly vacuous for pogo the moment it moved out of the grant.
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
    // NON-VACUITY, and it is the whole test. Every window above is
    // non-zero in `DEFAULT_TUNING` EXCEPT the air dodge, which the engine
    // holds at 0.0 deliberately — so a body that had stopped carrying the
    // stage's own numbers would still pass the loop above.
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

/// AND THE ROSTER IS WHERE IT SAYS SO. The test above measures the
/// constant; this measures that the stage actually declares it, which is the
/// half that can be deleted without breaking a compile.
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

/// Seat 0 is the human; everyone else is a CPU. The demo is playable with
/// one controller, which is the difference between a demo and a fixture.
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

/// Every seat is its own side, so a free-for-all actually resolves: a
/// four-way where everyone shares a team can never have a last side standing.
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

/// ⭐⭐ SUDDEN DEATH IS FOUGHT BY THE TIED SIDES, AND THE REST ARE OUT.
///
/// ⛔⛔ THE DEFECT THIS PINS, found by review 2026-08-24: the round put
/// every SURVIVOR on the starting damage. With three sides alive at a
/// timeout that means a side the clock had already put behind gets an even
/// restart against the two it was losing to.
///
/// ⛔ AND "LEAVE IT ALONE" IS WORSE THAN THE BUG, which is why the else arm
/// is an elimination and not a skip: a non-contender left standing keeps its
/// own low damage while the tied sides go to 150%, so the side that lost the
/// tiebreak would enter the round AHEAD.
///
/// ⭐ The retirement is the same `FighterEliminated` an exhausted fighter is
/// out with, so `take_eliminated_fighters_out_of_play` clears the body and
/// `last_side_standing` decides the round among the contenders — there is no
/// second notion of "out of the match" to keep in step with the first.
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

    // Three sides, seated the way the roster seats them, each carrying the
    // damage it had when the clock ran out.
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
                // ⛔⛔ TWO STOCKS EACH, AND THE FIXTURE HAD NONE AT ALL. A
                // genuine timed tie happens at whatever stock count the
                // fighters are on — this file's own tiebreak arms tie at
                // TWO — so a sudden-death fixture where nobody holds a stock
                // could not see the round failing to stage one.
                ambition_platformer2d::combat::components::FighterStocks::new(2),
                ambition_platformer2d::combat::components::ActiveCombatant,
            ))
            .id()
    };
    let tied_a = seat(&mut app, 0, 80);
    let tied_b = seat(&mut app, 1, 80);
    let behind = seat(&mut app, 2, 12);

    // The engine's own message, naming the sides the tiebreak found level.
    // ⭐ THE LABELS COME FROM `side_label`, which is what the fold that
    // named them used — a stage that spelled a side its own way would look
    // exactly like this defect.
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

    // ⛔⛔ ONE STOCK, WHICH IS WHAT MAKES IT SUDDEN DEATH. This staged the
    // damage and nothing else, so at a two-stock tie the first KO spent a
    // stock, eliminated nobody, and the ordinary respawn reset the very
    // damage this round had just set — the round simply continued. "Both at
    // 300%, one stock, first hit decides" was the stated rule and a third of
    // it was implemented.
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

    // ⛔ AND RETIREMENT IS BOTH HALVES. `spend_fighter_stocks` inserts the
    // marker AND removes `ActiveCombatant`, because a body stays standing
    // until a ruleset removes it — a marker alone leaves a corpse holding
    // attack state and a place on the anti-clump board. Command deferral
    // means cleanup cannot be relied on to close that gap.
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
/// the arrangement is symmetric about the centre and stays ON the
/// platform, which is the pair of properties that makes it a placement
/// rather than an offset: an eight-seat roster is still a fair start.
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

    // Symmetric about the centre: seat 0 and seat 1 straddle it evenly, so
    // no seat is handed the better return.
    assert!(
        ((seats[0].x - centre.x) + (seats[1].x - centre.x)).abs() < 0.01,
        "the first two seats are not symmetric about the stage centre"
    );

    // and every one of them is still over the platform, not past its lip —
    // an offset that grew without bound would respawn seat 7 into the blast
    // zone, which is a worse bug than the overlap it fixed.
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

/// A respawn is ABOVE the stage, not on it. A fighter that comes back on
/// the floor comes back inside the opponent who just knocked it off.
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

/// The stage is a platform surrounded by nothing, which is the one room
/// shape this engine had not loaded. Every other room is a box you cannot
/// leave.
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

/// A respawn lands above the PLATFORM, not above the stage's arbitrary
/// middle — the two coincide here and a future stage will separate them.
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
/// ⛔⛔ THIS FIXTURE USED TO WRITE `StocksMatchDecided` AND IT MEASURED
/// NOTHING. The card stopped reading that message when a speculative frame
/// was found writing it — a rolled-back verdict left NO CONTEST on screen
/// over a match still being fought — and the reader moved to the
/// `StocksMatchSettled` latch, which rewinds. The fixture kept driving the
/// message, so both arms asserted against a system that had not run.
///
/// ⭐ the repair is to model the PRODUCTION construction: a settled latch
/// stamped against the ACTIVE match, which is what makes the instance
/// comparison inside the reader answer at all. No `ConfirmedFrameBoundary`
/// is inserted on purpose — its own doc says an absent boundary confirms
/// everything, which is the eager host.
fn announced_outcome(outcome: ambition_platformer2d::actor::MatchVerdict) -> Option<String> {
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::presentation::HudReadouts>();
    let active = ambition_platformer2d::versus_match::ActiveMatch::for_test(2, None);
    let mut settled =
        ambition_platformer2d::actors::features::stocks_match::StocksMatchSettled::default();
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

/// The CARD says who won.
///
/// The plugin's half of the seam, driven through the message the engine
/// actually writes rather than by calling `victory_banner` directly — which
/// would test the string and not the wiring.
///
/// The claim is now made against the readout the stage declares and the HUD actually renders —
/// the same road as the fighter percents — which is strictly the stronger thing to assert.
///
/// the old test also guarded *"a ruleset that announces twice announces on
/// every frame after the match ends"*. That hazard is gone by construction
/// rather than by assertion: a readout is a map insert, so writing it twice
/// is writing it once.
#[test]
fn deciding_the_match_shows_a_card_naming_the_winner() {
    // the WORDING comes from `victory_banner`, which is where it is
    // decided; this fixture seats no bodies, so the card falls back to the
    // side label and that fallback is part of what is being asserted.
    use ambition_platformer2d::actor::MatchVerdict;
    let seat_two = MatchVerdict::Winner("seat 2".to_string());
    assert_eq!(
        announced_outcome(seat_two.clone()).as_deref(),
        Some(victory_banner(&seat_two, Some("seat 2")).as_str()),
        "the ending wrote no announce card, so the stage says nothing about \
         who won"
    );
}

/// THE TWO WINNER-LESS ENDINGS ARE STILL TWO, and only one of them gets a
/// card.
///
/// ⛔⛔ a draw and a no contest BOTH have no winner, so a card that
/// distinguished them by asking `winner.is_none()` would say the same thing
/// about a mutual ring-out and an abandoned match — which is the conflation
/// `MatchVerdict` exists to remove. The distinction is unchanged; what
/// changed is that the abandoned match no longer stops to show it. Jon,
/// 2026-08-26: *"skip the no contest presentation for now."* A mutual
/// ring-out is something the fighters achieved and nobody watching knows it
/// happened until the card says so; `Exit Match` is something the player
/// did on purpose one keypress ago.
///
/// ⭐ the WORDING for a no contest is still proven, by
/// `every_ending_has_its_own_words` over `victory_banner` — so restoring the
/// card is a one-line change and not a re-derivation.
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

/// The demo is something a player can ENTER.
///
/// Until this existed the crate was three correct pieces nobody could reach:
/// a roster, a stage and a ruleset, all unit-true and unassembled. That is
/// the shape this repo keeps catching — everything passes and nothing runs —
/// and a demo is the one kind of crate where it is indistinguishable from
/// working, because nobody notices a game they cannot start.
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

/// the stage declares a DI budget, and gives it back on the way out.
///
/// the DI law, its tuning field and the victim's live stick were all
/// wired, and this demo declared no combat rules at all — so `di_max_angle`
/// fell to the engine baseline of `0.0` and directional influence was OFF on
/// the one stage built to need it. Nothing failed; a launched fighter simply
/// had no say, and a knock-off was a coin flip instead of a read.
///
/// The release is the other half and the more dangerous one: left standing,
/// this budget follows the player into Ambition's PvE, which answers `0.0`
/// on purpose.
#[test]
fn the_stage_declares_its_di_budget_and_releases_it() {
    use ambition_platformer2d::game_shell::{MinimalShellPlugins, ShellExperienceScopes};
    use bevy::prelude::*;

    assert!(
        SMASH_DI_MAX_ANGLE > 0.0,
        "⛔ a zero budget makes `di_adjust` a no-op, so declaring the rules              at all would be theatre — DI would be off and every test still green"
    );
    // the same trap, one field over.
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
        "⛔ the stage's DI budget outlives its own experience and follows the              player into a game that authored none. Released: {released:?}"
    );
}

/// Run the preparation source as the SYSTEM it is, with one stage choice.
///
/// It grew a `Res` parameter when stage choice landed; the seam always took a
/// system (`install<S: IntoSystem<(), PreparedPlatformerSource, _>>`, whose doc
/// says the source *"may read the provider's own resources"*), so this is the
/// honest way to call it rather than a signature to work around.
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

/// THE CHOICE PICKS THE STAGE, AND BOTH STAGES STAY REACHABLE.
///
/// ⛔ A source that returned only the chosen room would look identical here on
/// the geometry assertion and quietly make the other stage unreachable to
/// anything that later wants to move between them — which is why the set is
/// asserted as well as the start.
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

    // ⭐ THE NARROW STAGE'S WHOLE POINT, asserted rather than described: LESS
    // GROUND, SAME ENVELOPE. Scaling the blast lines down with the width would
    // have produced the same stage smaller and changed no decision.
    let width = |w: &crate::ae::World| w.blocks[0].aabb.max.x - w.blocks[0].aabb.min.x;
    assert!(
        width(&narrow.geometry().0) < width(&flat.geometry().0),
        "the narrow stage is not narrower than the flat one"
    );

    // Same blast geometry across ALL THREE, so a comparison between stages is a
    // comparison of their geometry and not of their kill boundaries.
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

    // The default is still the stage every recorded measurement was taken on.
    assert_eq!(crate::SmashStageChoice::default(), crate::SmashStageChoice::Flat);
    // And the cycle a stage button would walk returns to where it started —
    // now three long, and asserted as a full lap rather than as a count, so
    // adding a fourth stage reddens this line instead of silently passing.
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

/// The prepared source carries the stage, not a default room.
///
/// The preparation seam takes a closure, and a closure that returns the
/// wrong room fails nowhere: the route prepares, the session starts, and the
/// player lands in somebody else's level.
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

/// The MATCH declares what 100% means, so a crossover fighter cannot bring
/// its own. ( found the number; found the owner,
/// )
///
/// A character that authors no vitals gets a ONE-HIT pool, and under
/// `DeathPolicy::Unbounded` the pool never kills — so nothing goes wrong
/// except the number, and the number is the entire user-facing output of the
/// stocks model. A 140-damage hit read as 14000%, with every test green:
/// the meter accumulated correctly and divided correctly, by a denominator
/// nobody had authored.
///
/// and the first fix was per-CHARACTER, which is why it held for a
/// fortnight and then failed on eleven fighters. This demo stamped the
/// reference onto the three ids it registers; every other name on
/// [`select::SMASH_ROSTER`] belongs to another game. Mary-O and Sanic author
/// `max_health: 1` — correct for a one-hit-kill platformer — and read 4200%
/// and 800% off ordinary melee damage on this stage.
///
/// so the assertion is about the ROSTER, not about a catalog row: the
/// character-side write is DELETED and re-adding it would not make this pass.
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
    // The reference is what makes a percent comparable across characters.
    // One would make every hit read in the thousands.
    assert!(
        SMASH_PERCENT_REFERENCE >= 50,
        "a percent reference of {SMASH_PERCENT_REFERENCE} makes a single hit \
         read in the hundreds, which is the 14000% bug in a smaller hat"
    );
}

/// The `duelist` preset resolves to the FIGHTER brain.
///
/// A preset name that does not resolve falls back to standing still, and a
/// fighter that stands still is indistinguishable from one whose brain was
/// never installed — which is what the match diagram printed for an hour
/// EVERY DIFFICULTY THIS DEMO CAN ASK FOR IS A PUBLISHED POLICY.
///
/// They are gone, and `smash_roster_at_levels` builds `duelist_l{level}` keys that now have
/// to resolve as `autonomous_profiles`.
///
/// what a miss looks like: `seat_brain_profile` finds nothing in either
/// authority and preparation REFUSES the seat — loud, not a fighter that
/// quietly stands still, which is how the same lookup failed twice before.
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
    // and the unlevelled name the roster's default seats use.
    assert!(
        profiles.contains_key(SMASH_DUELIST_BRAIN),
        "the bare `duelist` policy is gone, so an ordinary CPU seat is refused"
    );
}

/// (`travel: [0.0, 0.0]`) before anything said why.
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

/// Every verdict has words, and the winner's are a NAME rather than a side
/// label when the caller could resolve one.
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

/// ⛔⛔ THE PLATFORM MUST STAND STILL, and it did not — it was rebuilt under
/// the fighter every tick.
///
/// Its own comment calls it *"Stationary: a sweep of zero width at zero
/// speed"*, and the sweep IS zero. But the CENTRE was recomputed from
/// `kin.pos` on every tick, so from the outside it tracked the body exactly:
/// walk 200px and the platform walked with you.
///
/// ⭐ WHY IT MATTERS BEYOND LOOKING WRONG. A brain reads the floor it is
/// standing on to answer every ledge question, and a floor defined as
/// *"wherever I am"* makes those questions CIRCULAR — the perceived distance to
/// the edge is a constant 48px however far the body walks. Measured: with the
/// block made visible to perception, the fighter rollout judged every verb to
/// walk off it and vetoed all of them on every tick, which is queue row
/// ⭐⭐ **WALKING OFF THE PLATFORM ENDS THE PROTECTION — Jon's report, 2026-09-03:
/// *"in smash, if you move the platform disappears."***
///
/// ⚠ The sibling below already pins that the block does not FOLLOW. This pins
/// the half that was missing: one that outlives your step off it stays in play
/// for the whole grace window, which from the player's side is the same
/// complaint.
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

    // ⚠ A STEP, NOT A TELEPORT: just past the half-width, which is the boundary
    // the rule is about. Moving 200px would pass even if the rule were "leave
    // the stage".
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

/// ⭐ THE CONTROL ARM: falling toward the platform is not leaving it.
///
/// The rule is horizontal on purpose. A body descending onto its platform is
/// above it and has not left; a rule that read distance would take the grant
/// away during the fall the window exists to protect.
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

/// `D-BRAIN-PLATFORM-FLOOR`.
///
/// ⭐ AND IT IS THE GENRE'S ANSWER TOO: a respawn platform is somewhere you
/// LEAVE. One that follows cannot be left.
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

/// THE PLATFORMED STAGE IS A PLATFORM FIGHTER'S STAGE, structurally.
///
/// ⛔ The sign is the whole test. `y` grows DOWNWARD, so a tier ABOVE the stage
/// is at a SMALLER `y`; getting it backwards buries all three inside the main
/// block, where they are invisible, still solid to anything walking over them,
/// and break nothing that would say so.
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

    // And the original stage is untouched: one solid block, no tiers. Every
    // spacing/recovery number this project recorded was taken there.
    let flat = crate::smash_stage();
    assert_eq!(flat.world.blocks.len(), 1);
    assert!(!flat
        .world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::OneWay)));
}

/// EVERY TIER IS SOMEWHERE A FIGHTER CAN ACTUALLY GET TO.
///
/// ⛔⛔ THIS TEST EXISTS BECAUSE THE FIRST HEIGHTS I CHOSE WERE WRONG. Picked by
/// eye they were 132 and 250; the single-jump apex is 88.2px and the absolute
/// ceiling — an air jump taken exactly at the apex — is 148.3px, so the top tier
/// was 100px above anything the roster can reach. A platform nobody can stand on
/// fails no assertion and renders perfectly.
///
/// Recomputed from the ENGINE's constants rather than restating the numbers
/// above, so retuning gravity or either jump speed reddens this instead of
/// quietly stranding a tier.
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
