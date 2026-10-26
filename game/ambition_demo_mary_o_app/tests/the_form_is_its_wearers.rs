//! The form is its wearer's (review A6): each body in Eternal Cosmic Mary-O
//! has what the form gives, and a hit she lands in it stops her less and
//! does not stop the world.

use bevy::prelude::*;

use ambition_demo_mary_o::cosmic::COSMIC_CHARACTER_ID;
use ambition_platformer2d::actors::transform_beat::TransformBeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth, Invulnerability, WornCharacter};
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::impact_hitstop::ImpactHitstop;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>().0 = frame;
    app.update();
}

fn idle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        step(app, ControlFrame::default());
    }
}

fn press_utility(app: &mut App) {
    step(app, ControlFrame { fly_toggle_pressed: true, ..Default::default() });
}

/// The demo in 1-1, with her seated, in the form, and its transformation
/// done.
fn in_the_form() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    let body = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next()
        })
        .expect("Mary-O takes a seat");
    idle(&mut app, 30);
    press_utility(&mut app);
    idle(&mut app, 4);
    press_utility(&mut app);
    idle(&mut app, 8);
    let worn = app.world().get::<WornCharacter>(body).unwrap().id().to_string();
    assert_eq!(worn, COSMIC_CHARACTER_ID, "the premise: she is in the form");
    assert!(app.world().get::<TransformBeat>(body).is_none(), "the premise: her transformation is done");
    (app, body)
}

fn lent(app: &App, body: Entity) -> bool {
    app.world()
        .get::<ae::AbilityContributions>(body)
        .is_some_and(|contributions| contributions.get("mary_o_cosmic").is_some())
}

/// ⭐ A6: a body in the form that is not the first player has the jump in
/// the air the form lends. The form gave it only to `PrimaryPlayer`.
#[test]
fn a_body_in_the_form_that_is_not_the_first_player_has_what_it_gives() {
    let (mut app, body) = in_the_form();
    assert!(lent(&app, body), "the control: the first player in the form has the jump");
    app.world_mut().entity_mut(body).remove::<PrimaryPlayer>();
    app.world_mut().get_mut::<ae::AbilityContributions>(body).unwrap().clear("mary_o_cosmic");
    idle(&mut app, 2);
    assert!(lent(&app, body), "a second player in the form has the jump too");
}

/// ⭐ A6, the measurement: a hit she lands in the form is a contact hit. It
/// stops her for no more than the form's stop, and it does not stop the
/// world.
#[test]
fn a_hit_in_the_form_stops_her_less_and_not_the_world() {
    let (mut app, body) = in_the_form();
    let enemy = {
        let world = app.world_mut();
        let at = world.get::<ae::BodyKinematics>(body).unwrap().pos;
        let mut q = world.query_filtered::<(Entity, &ae::BodyKinematics, &BodyHealth), (With<ActorConfig>, Without<PlayerEntity>)>();
        q.iter(world)
            .filter(|(_, _, health)| health.alive())
            .min_by(|a, b| a.1.pos.distance(at).total_cmp(&b.1.pos.distance(at)))
            .map(|(entity, ..)| entity)
            .expect("1-1 has an enemy")
    };
    let mut hit = false;
    let mut most_stop = 0.0f32;
    let mut world_stopped = false;
    for _ in 0..120 {
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(body) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        let to = app.world().get::<ae::BodyKinematics>(enemy).map(|kin| kin.pos);
        if let (Some(to), false) = (to, hit) {
            let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
            kin.pos = to;
            kin.vel = Vec2::ZERO;
        }
        idle(&mut app, 1);
        let alive = app.world().get::<BodyHealth>(enemy).is_some_and(|health| health.alive());
        hit |= !alive;
        most_stop = most_stop.max(app.world().get::<BodyCombat>(body).unwrap().hitstop_timer);
        let now = *app.world().resource::<ambition_platformer2d::time::SimTick>();
        world_stopped |= app.world().resource::<ImpactHitstop>().is_freezing(now);
    }
    assert!(hit, "the premise: her touch in the form beat the enemy");
    assert!(most_stop > 0.0, "the premise: the hit stopped her a little ({most_stop})");
    assert!(most_stop <= 0.02 + 1e-4, "the form's stop is short: {most_stop} s");
    assert!(!world_stopped, "a contact hit does not stop the world");
}
