//! Paralysis, burial and freeze: strikes that take a body's control away.
//!
//! The status is hit data (`HitStatus` on the volume), installed by the
//! receiver as part of the hit's reaction (`apply_body_hit_reaction`), and the
//! hold is the engine's (`BodyCombat::paralysis_timer`, `bury_timer`,
//! `freeze_timer`). What stays here is this game's rule of how a held fighter
//! buys time back.

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
    packs: ambition_platformer2d::content::SelectedPacks,
) {
    let selected_generation = packs.of(&crate::smash_pack::PACK);
    let selected_pack: &ambition_platformer2d::content::PreparedContentPack = &selected_generation;
    let mash = crate::rules::smash_rules_in(selected_pack).paralysis_mash_seconds;
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
    packs: ambition_platformer2d::content::SelectedPacks,
) {
    let selected_generation = packs.of(&crate::smash_pack::PACK);
    let selected_pack: &ambition_platformer2d::content::PreparedContentPack = &selected_generation;
    let mash = crate::rules::smash_rules_in(selected_pack).bury_mash_seconds;
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
    packs: ambition_platformer2d::content::SelectedPacks,
) {
    let selected_generation = packs.of(&crate::smash_pack::PACK);
    let selected_pack: &ambition_platformer2d::content::PreparedContentPack = &selected_generation;
    let mash = crate::rules::smash_rules_in(selected_pack).freeze_mash_seconds;
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
    use crate::module_road::Road;
    use ambition_platformer2d::characters::actor::BodyCombat;
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::combat::on_hit::OnHitEffectMessage;
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::extension::sdk::phases::HIT_REACTION;
    use ambition_platformer2d::extension::ExtensionSet;

    fn app(road: Road) -> App {
        let mut app = crate::module_road::app_with(
            road,
            &[
                ambition_demo_smash_modules::lifesteal::module,
                ambition_demo_smash_modules::hit_grab::module,
                ambition_demo_smash_modules::recoil::module,
            ],
        );
        app.add_systems(Update, (mash_out_of_paralysis, mash_out_of_burial, mash_out_of_freeze).after(ExtensionSet::Lower(HIT_REACTION)));
        app
    }

    fn body(app: &mut App) -> Entity {
        app.world_mut().spawn((ae::BodyKinematics::default(), BodyCombat::default(), ActorControl(Default::default()))).id()
    }

    fn held(app: &App, body: Entity) -> f32 {
        app.world().get::<BodyCombat>(body).unwrap().paralysis_timer
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
        let rule = crate::rules::smash_rules_of(crate::smash_pack::PACK.prepared()).paralysis_mash_seconds;
        assert!((held(&app, victim) - (0.6 - rule)).abs() < 1e-6, "a chord bought {} s", 0.6 - held(&app, victim));
    }

    /// A buried fighter digs out at the burial's own rate.
    #[test]
    fn a_press_digs_out_at_the_burial_rate() {
        let mut app = app(Road::Linked);
        let victim = body(&mut app);
        app.world_mut().get_mut::<BodyCombat>(victim).unwrap().bury_timer = 1.0;
        app.world_mut().get_mut::<ActorControl>(victim).unwrap().0.melee_pressed = true;
        app.update();
        let rule = crate::rules::smash_rules_of(crate::smash_pack::PACK.prepared()).bury_mash_seconds;
        let left = app.world().get::<BodyCombat>(victim).unwrap().bury_timer;
        assert!((left - (1.0 - rule)).abs() < 1e-6, "one press dug {} s", 1.0 - left);
    }

    /// A frozen fighter thaws at the freeze's own rate.
    #[test]
    fn a_press_thaws_at_the_freeze_rate() {
        let mut app = app(Road::Linked);
        let victim = body(&mut app);
        app.world_mut().get_mut::<BodyCombat>(victim).unwrap().freeze_timer = 1.2;
        app.world_mut().get_mut::<ActorControl>(victim).unwrap().0.special_pressed = true;
        app.update();
        let rule = crate::rules::smash_rules_of(crate::smash_pack::PACK.prepared()).freeze_mash_seconds;
        assert!((app.world().get::<BodyCombat>(victim).unwrap().freeze_timer - (1.2 - rule)).abs() < 1e-6);
    }

    /// A lifesteal strike mends the body that struck, by its credit: its pool
    /// and its percent. The struck body is not mended, a striker with no
    /// identity cannot be named, and a striker out of play is not mended. On
    /// both roads.
    #[test]
    fn a_lifesteal_strike_mends_its_striker() {
        use ambition_platformer2d::characters::actor::{BodyHealth, DeathPolicy, Health};
        use ambition_platformer2d::entity_catalog::smash_lifesteal::{LifestealParams, LIFESTEAL};
        use ambition_platformer2d::platformer::sim_id::SimId;
        let hurt = || BodyHealth::restored(Health { current: 70, max: 100, invulnerable: Default::default() }, 30, DeathPolicy::Unbounded);
        let steal = |app: &mut App, owner: Entity, victim: Entity| {
            app.world_mut().write_message(OnHitEffectMessage {
                owner,
                victim,
                volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
                contact: ae::Vec2::ZERO,
                effect: ambition_platformer2d::entity_catalog::EffectRef {
                    key: LIFESTEAL.to_string(),
                    params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&LifestealParams { amount: 4 }).unwrap(),
                },
            });
        };
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let striker = body(&mut app);
            let victim = body(&mut app);
            let nameless = body(&mut app);
            let gone = body(&mut app);
            for (who, slot) in [(striker, 1), (victim, 2), (gone, 3)] {
                app.world_mut().entity_mut(who).insert((SimId::player_slot(slot), hurt()));
            }
            app.world_mut().entity_mut(nameless).insert(hurt());
            app.world_mut().entity_mut(gone).insert(ambition_platformer2d::combat::death_rules::OutOfPlay);
            steal(&mut app, striker, victim);
            steal(&mut app, nameless, victim);
            steal(&mut app, gone, victim);
            app.update();
            let health = |app: &App, who: Entity| {
                let health = app.world().get::<BodyHealth>(who).unwrap();
                (health.current(), health.damage_taken())
            };
            assert_eq!(health(&app, striker), (74, 26), "{road:?}: the striker was not mended its 4");
            assert_eq!(health(&app, victim), (70, 30), "{road:?}: the struck body was mended");
            assert_eq!(health(&app, nameless), (70, 30), "{road:?}: a striker with no identity was mended");
            assert_eq!(health(&app, gone), (70, 30), "{road:?}: a striker out of play was mended");
        }
    }

    /// A hit-grab asks the capture relation for the struck body by name, held
    /// where the move says, and drops the hit's launch and stun, because the
    /// hold replaces them. A body struck in the air keeps its hit and is not
    /// asked for. On both roads.
    #[test]
    fn a_hit_grab_asks_for_the_body_it_struck() {
        use ambition_platformer2d::combat::capture::CaptureAttemptRequested;
        use ambition_platformer2d::entity_catalog::smash_hit_grab::{HitGrabParams, HIT_GRAB};
        use ambition_platformer2d::platformer::sim_id::SimId;
        use bevy::ecs::message::Messages;
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let grounded = |app: &mut App, slot: u8, on_ground: bool| {
                let who = body(app);
                let mut flight = ae::BodyFlightState::default();
                flight.stage_launch(ae::Vec2::new(120.0, -200.0), false, Default::default());
                let mut combat = BodyCombat::default();
                combat.hitstun_timer = 0.4;
                app.world_mut().entity_mut(who).insert((
                    SimId::player_slot(slot),
                    flight,
                    ae::BodyGroundState { on_ground, ..Default::default() },
                    combat,
                ));
                who
            };
            let striker = grounded(&mut app, 1, true);
            let standing = grounded(&mut app, 2, true);
            let flying = grounded(&mut app, 3, false);
            for victim in [standing, flying] {
                app.world_mut().write_message(OnHitEffectMessage {
                    owner: striker,
                    victim,
                    volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
                    contact: ae::Vec2::ZERO,
                    effect: ambition_platformer2d::entity_catalog::EffectRef {
                        key: HIT_GRAB.to_string(),
                        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&HitGrabParams { hold_offset: (13.0, 3.0) }).unwrap(),
                    },
                });
            }
            app.update();
            let messages = app.world().resource::<Messages<CaptureAttemptRequested>>();
            let asked: Vec<_> = messages.get_cursor().read(messages).map(|a| (a.captor, a.target, a.hold_offset)).collect();
            assert_eq!(asked, vec![(striker, Some(standing), ae::Vec2::new(13.0, 3.0))], "{road:?}");
            let seized = *app.world().get::<BodyCombat>(standing).unwrap();
            assert!(seized.hitstun_timer == 0.0, "{road:?}: the seized body kept its stun");
            assert!(app.world().get::<ae::BodyFlightState>(standing).unwrap().pending_launch_state().is_empty(), "{road:?}");
            assert_eq!(app.world().get::<BodyCombat>(flying).unwrap().hitstun_timer, 0.4, "{road:?}: the body in the air lost its hit");
        }
    }

    /// A recoiling strike charges its striker as a price: the meter rises, the
    /// pool falls but not below 1, and the struck body does not pay. On both
    /// roads.
    #[test]
    fn a_recoiling_strike_charges_its_striker_a_price() {
        use ambition_platformer2d::characters::actor::{BodyHealth, DeathPolicy, Health};
        use ambition_platformer2d::entity_catalog::smash_recoil::{RecoilParams, RECOIL};
        use ambition_platformer2d::platformer::sim_id::SimId;
        let pool = |current: i32| BodyHealth::restored(Health { current, max: 100, invulnerable: Default::default() }, 100 - current, DeathPolicy::HpDepleted);
        for road in [Road::Linked, Road::Loaded] {
            let mut app = app(road);
            let striker = body(&mut app);
            let nearly_out = body(&mut app);
            let victim = body(&mut app);
            for (who, slot, current) in [(striker, 1, 70), (nearly_out, 2, 2), (victim, 3, 70)] {
                app.world_mut().entity_mut(who).insert((SimId::player_slot(slot), pool(current)));
            }
            for owner in [striker, nearly_out] {
                app.world_mut().write_message(OnHitEffectMessage {
                    owner,
                    victim,
                    volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
                    contact: ae::Vec2::ZERO,
                    effect: ambition_platformer2d::entity_catalog::EffectRef {
                        key: RECOIL.to_string(),
                        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&RecoilParams { amount: 3 }).unwrap(),
                    },
                });
            }
            app.update();
            let health = |app: &App, who: Entity| {
                let health = app.world().get::<BodyHealth>(who).unwrap();
                (health.current(), health.damage_taken())
            };
            assert_eq!(health(&app, striker), (67, 33), "{road:?}: the striker did not pay its 3");
            assert_eq!(health(&app, nearly_out), (1, 99), "{road:?}: a striker that could afford 1 paid other than 1");
            assert_eq!(health(&app, victim), (70, 30), "{road:?}: the struck body paid");
        }
    }
}
