//! Paralysis, burial and freeze: strikes that take a body's control away.
//!
//! The strike is the module `ambition_demo_smash_modules::paralyze`, which
//! asks `ambition.combat.paralyze` to hold the struck body; the hold is the
//! engine's (`BodyCombat::paralysis_timer`), and so is a burial's
//! (`ambition_demo_smash_modules::bury`, `BodyCombat::bury_timer`) and a
//! freeze's (`ambition_demo_smash_modules::freeze`, `BodyCombat::freeze_timer`). What stays
//! here is this game's rule of how a held fighter buys time back.

use bevy::prelude::*;

/// A paralyzed fighter's press buys back time: one credit per tick, not per
/// button, read from the raw `ActorControl` as the sleeper's mash is
/// ([`crate::sing::mash_out_of_sleep`]). Its own rate: a paralysis is short
/// and a sleep is long, so one rate would make one of them meaningless.
///
/// Rollback: nothing new. `paralysis_timer` is on `BodyCombat`, which
/// snapshots, and `ActorControl` is re-derived from rolled-back input.
pub fn mash_out_of_paralysis(
    mut held: Query<(
        &mut ambition_platformer2d::characters::actor::BodyCombat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
) {
    let mash = crate::rules::smash_rules().paralysis_mash_seconds;
    for (mut combat, control) in &mut held {
        // Read before write: a free body takes no change-detection write.
        if combat.paralysis_timer <= 0.0 {
            continue;
        }
        if pressed_any(control) {
            combat.paralysis_timer = (combat.paralysis_timer - mash).max(0.0);
        }
    }
}

/// A buried fighter's press digs it out: the same one credit a tick, at the
/// burial's own rate.
pub fn mash_out_of_burial(
    mut buried: Query<(
        &mut ambition_platformer2d::characters::actor::BodyCombat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
) {
    let mash = crate::rules::smash_rules().bury_mash_seconds;
    for (mut combat, control) in &mut buried {
        if combat.bury_timer <= 0.0 {
            continue;
        }
        if pressed_any(control) {
            combat.bury_timer = (combat.bury_timer - mash).max(0.0);
        }
    }
}

/// A frozen fighter's press thaws it: the same one credit a tick, at the
/// freeze's own rate.
pub fn mash_out_of_freeze(
    mut frozen: Query<(
        &mut ambition_platformer2d::characters::actor::BodyCombat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
) {
    let mash = crate::rules::smash_rules().freeze_mash_seconds;
    for (mut combat, control) in &mut frozen {
        if combat.freeze_timer <= 0.0 {
            continue;
        }
        if pressed_any(control) {
            combat.freeze_timer = (combat.freeze_timer - mash).max(0.0);
        }
    }
}

/// Any action press this tick: a specific button would be a control-scheme
/// choice, and mashing the "wrong" one would look broken.
fn pressed_any(control: &ambition_platformer2d::characters::control::ActorControl) -> bool {
    let frame = &control.0;
    frame.melee_pressed
        || frame.jump_pressed
        || frame.burst_pressed
        || frame.special_pressed
        || frame.grab_pressed
        || frame.projectile_pressed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_road::{runner_of, Road};
    use ambition_platformer2d::characters::actor::BodyCombat;
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::entity_catalog::smash_paralyze::{ParalyzeParams, PARALYZE};
    use ambition_platformer2d::extension::sdk::phases::HIT_REACTION;
    use ambition_platformer2d::extension::ExtensionSet;

    fn app(road: Road) -> App {
        let mut app = crate::module_road::app_with(
            road,
            &[
                ambition_demo_smash_modules::paralyze::module,
                ambition_demo_smash_modules::bury::module,
                ambition_demo_smash_modules::freeze::module,
                ambition_demo_smash_modules::crumple::module,
            ],
        );
        app.add_systems(Update, (mash_out_of_paralysis, mash_out_of_burial, mash_out_of_freeze).after(ExtensionSet::Lower(HIT_REACTION)));
        app
    }

    fn body(app: &mut App) -> Entity {
        app.world_mut().spawn((ae::BodyKinematics::default(), BodyCombat::default(), ActorControl(Default::default()))).id()
    }

    fn strike(app: &mut App, owner: Entity, victim: Entity, duration_s: f32) {
        strike_with(app, owner, victim, PARALYZE, duration_s);
    }

    fn strike_with(app: &mut App, owner: Entity, victim: Entity, key: &str, duration_s: f32) {
        app.world_mut().write_message(OnHitEffectMessage {
            owner,
            victim,
            volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
            contact: ae::Vec2::ZERO,
            effect: ambition_platformer2d::entity_catalog::EffectRef {
                key: key.to_string(),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&ParalyzeParams { duration_s }).unwrap(),
            },
        });
    }

    fn held(app: &App, body: Entity) -> f32 {
        app.world().get::<BodyCombat>(body).unwrap().paralysis_timer
    }

    /// The strike holds the body it struck for the move's time, on either
    /// road; a shorter strike does not shorten a longer hold, and the striker
    /// is not held.
    #[test]
    fn a_paralyzing_strike_holds_its_victim_on_both_roads() {
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            assert_eq!(runner_of(&app, PARALYZE).starts_with("Loaded"), road == Road::Loaded);
            let (striker, victim) = (body(&mut app), body(&mut app));
            strike(&mut app, striker, victim, 0.6);
            app.update();
            assert_eq!(held(&app, victim), 0.6, "{road:?}: the victim is held for the move's time");
            assert_eq!(held(&app, striker), 0.0, "{road:?}: the striker was held");
            strike(&mut app, striker, victim, 0.1);
            app.update();
            assert_eq!(held(&app, victim), 0.6, "{road:?}: a shorter strike shortened the hold");
        }
    }

    /// A press buys back the rules' rate, one credit a tick; an unpressed
    /// tick buys nothing.
    #[test]
    fn a_press_buys_back_the_rules_rate() {
        let mut app = app(Road::Linked);
        let victim = body(&mut app);
        app.world_mut().get_mut::<BodyCombat>(victim).unwrap().paralysis_timer = 0.6;
        app.update();
        assert_eq!(held(&app, victim), 0.6, "an unpressed tick bought time");
        let frame = &mut app.world_mut().get_mut::<ActorControl>(victim).unwrap().0;
        frame.melee_pressed = true;
        frame.jump_pressed = true;
        app.update();
        let rule = crate::rules::smash_rules().paralysis_mash_seconds;
        assert!((held(&app, victim) - (0.6 - rule)).abs() < 1e-6, "a chord bought {} s", 0.6 - held(&app, victim));
    }

    /// A body struck standing is buried: held, and the hit's launch, velocity
    /// and stun are dropped, because the burial replaces them. A body struck
    /// in the air is not buried and keeps its hit. On both roads.
    #[test]
    fn a_burying_strike_buries_a_standing_body_and_spares_one_in_the_air() {
        use ambition_platformer2d::entity_catalog::smash_bury::BURY;
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let striker = body(&mut app);
            let launched = |app: &mut App, on_ground: bool| {
                let victim = body(app);
                let mut flight = ae::BodyFlightState::default();
                flight.stage_launch(ae::Vec2::new(0.0, 300.0), false, Default::default());
                let ground = ae::BodyGroundState { on_ground, ..Default::default() };
                let mut combat = BodyCombat::default();
                combat.hitstun_timer = 0.4;
                app.world_mut().entity_mut(victim).insert((flight, ground, combat));
                app.world_mut().get_mut::<ae::BodyKinematics>(victim).unwrap().vel = ae::Vec2::new(0.0, 300.0);
                victim
            };
            let standing = launched(&mut app, true);
            let flying = launched(&mut app, false);
            strike_with(&mut app, striker, standing, BURY, 1.0);
            strike_with(&mut app, striker, flying, BURY, 1.0);
            app.update();
            let combat = *app.world().get::<BodyCombat>(standing).unwrap();
            assert!(combat.is_buried() && combat.bury_timer == 1.0, "{road:?}: {combat:?}");
            assert_eq!(combat.hitstun_timer, 0.0, "{road:?}: the burial replaces the stun");
            assert!(app.world().get::<ae::BodyFlightState>(standing).unwrap().pending_launch_state().is_empty(), "{road:?}: the launch was not dropped");
            assert_eq!(app.world().get::<ae::BodyKinematics>(standing).unwrap().vel, ae::Vec2::ZERO);
            let spared = *app.world().get::<BodyCombat>(flying).unwrap();
            assert!(!spared.is_buried() && spared.hitstun_timer == 0.4, "{road:?}: a body in the air was buried: {spared:?}");
            assert!(!app.world().get::<ae::BodyFlightState>(flying).unwrap().pending_launch_state().is_empty(), "{road:?}: the spike was dropped");
        }
    }

    /// A buried fighter digs out at the burial's own rate.
    #[test]
    fn a_press_digs_out_at_the_burial_rate() {
        let mut app = app(Road::Linked);
        let victim = body(&mut app);
        app.world_mut().get_mut::<BodyCombat>(victim).unwrap().bury_timer = 1.0;
        app.world_mut().get_mut::<ActorControl>(victim).unwrap().0.melee_pressed = true;
        app.update();
        let rule = crate::rules::smash_rules().bury_mash_seconds;
        let left = app.world().get::<BodyCombat>(victim).unwrap().bury_timer;
        assert!((left - (1.0 - rule)).abs() < 1e-6, "one press dug {} s", 1.0 - left);
    }

    /// A freezing strike encases its victim and leaves its launch alone, on
    /// both roads; a press thaws at the freeze's own rate.
    #[test]
    fn a_freezing_strike_encases_its_victim_and_keeps_its_launch() {
        use ambition_platformer2d::entity_catalog::smash_freeze::FREEZE;
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let striker = body(&mut app);
            let victim = body(&mut app);
            let mut flight = ae::BodyFlightState::default();
            flight.stage_launch(ae::Vec2::new(200.0, -100.0), false, Default::default());
            app.world_mut().entity_mut(victim).insert(flight);
            strike_with(&mut app, striker, victim, FREEZE, 1.2);
            app.update();
            let combat = *app.world().get::<BodyCombat>(victim).unwrap();
            assert!(combat.is_frozen() && combat.freeze_timer == 1.2, "{road:?}: {combat:?}");
            assert!(!app.world().get::<ae::BodyFlightState>(victim).unwrap().pending_launch_state().is_empty(), "{road:?}: the freeze dropped the launch");
        }
        let mut app = app(Road::Linked);
        let victim = body(&mut app);
        app.world_mut().get_mut::<BodyCombat>(victim).unwrap().freeze_timer = 1.2;
        app.world_mut().get_mut::<ActorControl>(victim).unwrap().0.special_pressed = true;
        app.update();
        let rule = crate::rules::smash_rules().freeze_mash_seconds;
        assert!((app.world().get::<BodyCombat>(victim).unwrap().freeze_timer - (1.2 - rule)).abs() < 1e-6);
    }

    /// A crumpling strike replaces a standing body's hit (no launch, no stun)
    /// and leaves it owing the collapse; a body in the air keeps its hit. On
    /// both roads.
    #[test]
    fn a_crumpling_strike_owes_a_standing_body_its_collapse() {
        use ambition_platformer2d::entity_catalog::smash_crumple::CRUMPLE;
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let striker = body(&mut app);
            let struck = |app: &mut App, on_ground: bool| {
                let victim = body(app);
                let mut flight = ae::BodyFlightState::default();
                flight.stage_launch(ae::Vec2::new(120.0, -200.0), false, Default::default());
                let mut combat = BodyCombat::default();
                combat.hitstun_timer = 0.4;
                app.world_mut().entity_mut(victim).insert((flight, ae::BodyGroundState { on_ground, ..Default::default() }, combat));
                victim
            };
            let (standing, flying) = (struck(&mut app, true), struck(&mut app, false));
            strike_with(&mut app, striker, standing, CRUMPLE, 0.7);
            strike_with(&mut app, striker, flying, CRUMPLE, 0.7);
            app.update();
            let combat = *app.world().get::<BodyCombat>(standing).unwrap();
            assert!(combat.is_crumpling() && combat.crumple_collapse_owed && combat.hitstun_timer == 0.0, "{road:?}: {combat:?}");
            assert!(app.world().get::<ae::BodyFlightState>(standing).unwrap().pending_launch_state().is_empty(), "{road:?}");
            let spared = *app.world().get::<BodyCombat>(flying).unwrap();
            assert!(!spared.is_crumpling() && !spared.crumple_collapse_owed && spared.hitstun_timer == 0.4, "{road:?}: {spared:?}");
        }
    }
}
