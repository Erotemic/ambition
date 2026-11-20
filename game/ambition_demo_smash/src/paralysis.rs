//! Paralysis: a strike that holds the body it struck.
//!
//! The strike is the module `ambition_demo_smash_modules::paralyze`, which
//! asks `ambition.combat.paralyze` to hold the struck body; the hold is the
//! engine's (`BodyCombat::paralysis_timer`). What stays here is this game's
//! rule of how a paralyzed fighter buys time back.

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
        let frame = &control.0;
        let pressed = frame.melee_pressed
            || frame.jump_pressed
            || frame.burst_pressed
            || frame.special_pressed
            || frame.grab_pressed
            || frame.projectile_pressed;
        if pressed {
            combat.paralysis_timer = (combat.paralysis_timer - mash).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_road::{app_on, runner_of, Road};
    use ambition_platformer2d::characters::actor::BodyCombat;
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::entity_catalog::smash_paralyze::{ParalyzeParams, PARALYZE};
    use ambition_platformer2d::extension::sdk::phases::HIT_REACTION;
    use ambition_platformer2d::extension::ExtensionSet;

    fn app(road: Road) -> App {
        let mut app = app_on(road, ambition_demo_smash_modules::paralyze::module);
        app.add_systems(Update, mash_out_of_paralysis.after(ExtensionSet::Lower(HIT_REACTION)));
        app
    }

    fn body(app: &mut App) -> Entity {
        app.world_mut().spawn((ae::BodyKinematics::default(), BodyCombat::default(), ActorControl(Default::default()))).id()
    }

    fn strike(app: &mut App, owner: Entity, victim: Entity, duration_s: f32) {
        app.world_mut().write_message(OnHitEffectMessage {
            owner,
            victim,
            volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
            contact: ae::Vec2::ZERO,
            effect: ambition_platformer2d::entity_catalog::EffectRef {
                key: PARALYZE.to_string(),
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
}
