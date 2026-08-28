//! D255/R9: the ponytail hits you going out and hits you coming back, and does
//! not machine-gun you in between.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ A RETURNING SHOT IS TWO CHANCES, AND EACH VICTIM GETS ONE OF EACH.
///
/// ⛔⛔ IT DESPAWNED ON ITS FIRST BODY CONTACT, which made every bit of the
/// landed return-flight work unobservable in combat: she threw it, it hit
/// somebody, and it never came back. ⛔ AND DELETING THE DESPAWN IS THE WRONG
/// FIX — a shot that survives contact overlaps its victim for as many ticks as
/// it takes to pass through, and would damage on every one.
///
/// ⭐ THE RULE IS THE PULSE RULE this codebase already states for multi-hit
/// moves: one continuous stretch of contact owns ONE per-victim answer, and a
/// gap starts a new one. A boomerang's legs are its two stretches.
///
/// ⛔ THREE CLAIMS, EACH SEPARATELY ASSERTED, because any two of them are
/// satisfied by a shot that is simply wrong in a different way: it damages ONCE
/// on the way out (not once per overlapping tick), it is STILL ALIVE after that
/// (not spent), and it damages AGAIN after the turnaround (the leg re-arms).
#[test]
fn a_returning_shot_hits_each_victim_once_per_leg() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::platformer::projectile::ProjectileGameplay;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            break;
        }
    }
    let seat = |app: &mut App, want: usize| -> Entity {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, s)| s.0 == want)
            .map(|(entity, _)| entity)
            .expect("the match seats this fighter")
    };
    // ⛔ THE VICTIM IS THE HUMAN SEAT, which this test can hold still. A CPU
    // walks out of a shot's path and the arm becomes about pathfinding.
    let victim = seat(&mut app, 0);

    let at = |app: &App, body: Entity| {
        app.world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
            .expect("a seated fighter has kinematics")
            .pos
    };
    let damage = |app: &App, body: Entity| {
        app.world()
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(body)
            .map_or(0, |h| h.damage_taken())
    };
    let shots_alive = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<ProjectileGameplay>>();
        q.iter(world).count()
    };
    assert_eq!(
        shots_alive(&mut app),
        0,
        "something was already in flight, so the counts below are not this shot's"
    );

    // ⛔⛔ THROWN FROM INSIDE ITS OWN REACH, and that is arithmetic rather than
    // taste: a shot launched at `v0` with the analytic return acceleration turns
    // around at `v0 · out_s / 2`, which is 153px here. The first version stood
    // the victim 200px away and measured a boomerang that came home without ever
    // arriving — which reads exactly like a shot that damages nobody.
    //
    // `out_s` is the ponytail's own, so this flies the shape content authors
    // rather than a shape invented for the test.
    const OUT_S: f32 = 0.34;
    let victim_pos = at(&app, victim);
    let origin = victim_pos - ambition_platformer2d::engine_core::Vec2::new(110.0, 0.0);
    app.world_mut().write_message(
        ambition_platformer2d::projectiles::ProjectileSpawnRequest::open(
            // ⛔ OWNERLESS, AND THAT IS NOT A SHORTCUT. Both seats carry faction
            // `Player` in a free-for-all, so a shot owned by the other seat reads
            // as friendly fire and is refused — the same rule the shark test
            // needs `HitSide::Enemy` for. An ownerless shot is a HAZARD and lands
            // on everyone, which is what makes this arm about the LEG rule rather
            // than about allegiance.
            bevy::prelude::Entity::PLACEHOLDER,
            ambition_platformer2d::projectiles::ProjectileSpawn {
                origin,
                dir: ambition_platformer2d::engine_core::Vec2::new(1.0, 0.0),
                speed: 900.0,
                damage: 3,
                // Long enough for both legs; the return is analytic at 2·out_s.
                max_lifetime: OUT_S * 2.0,
                half_extent: ambition_platformer2d::engine_core::Vec2::new(14.0, 14.0),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                boomerang_return_s: Some(OUT_S),
            },
            ambition_platformer2d::projectiles::ProjectileStart::StepThisTick,
        ),
    );

    app.update();
    assert_eq!(
        shots_alive(&mut app),
        1,
        "the spawn request materialized nothing, so this arm is about a shot that \
         was never in the air"
    );

    // Walk the whole flight, holding the victim still and sampling the three
    // facts each tick.
    let mut first_hit_tick = None;
    let mut damage_after_first_leg = 0;
    let mut alive_after_first_hit = None;
    let mut total = 0;
    let mut hit_ticks: Vec<(usize, i32, bool)> = Vec::new();
    let mut has_turned = false;
    let before = damage(&app, victim);
    for tick in 0..60 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
        let taken = damage(&app, victim) - before;
        if first_hit_tick.is_none() && taken > 0 {
            first_hit_tick = Some(tick);
            alive_after_first_hit = Some(shots_alive(&mut app));
        }
        // ⛔ SAMPLED AT THE TURNAROUND ITSELF, not at a frame number. The leg is
        // the sign of the shot's own travel; deriving the tick from `out_s`
        // instead is a second model of the same fact, and it is the one that
        // would drift if the integrator changed.
        let turned = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<
                &ambition_platformer2d::engine_core::BodyKinematics,
                With<ProjectileGameplay>,
            >();
            q.iter(world).next().is_some_and(|kin| kin.vel.x < 0.0)
        };
        // ⛔ LATCHED. `turned` reads the live shot, and a shot that has EXPIRED
        // answers "not turned" for the same reason a missing one does — so
        // without the latch the tail of the flight overwrote the outbound
        // reading with the whole-flight total, and the arm failed claiming the
        // outbound leg had done both legs' damage.
        if !turned && !has_turned {
            damage_after_first_leg = taken;
        }
        has_turned |= turned;
        if taken > 0 {
            hit_ticks.push((tick, taken, turned));
        }
        total = taken;
    }

    let first = first_hit_tick.expect(
        "the shot never reached the victim at all, so none of the three claims \
         below is being measured",
    );
    assert_eq!(
        alive_after_first_hit,
        Some(1),
        "the shot was gone the tick it connected (first hit at tick {first}) — a \
         boomerang that dies in the first body it touches never comes back, and \
         every bit of the return-flight work is unobservable in combat"
    );
    assert_eq!(
        damage_after_first_leg, 3,
        "the outbound leg did {damage_after_first_leg} damage where the shot \
         authors 3 — anything more is the shot damaging on every tick it \
         overlaps its victim, which is what surviving contact costs if each leg \
         does not own a per-victim ledger. Damage arrived at (tick, total, \
         turned) = {hit_ticks:?}"
    );
    assert_eq!(
        total, 6,
        "the whole flight did {total} where two legs of a 3-damage shot are 6 — \
         the turnaround must re-arm a victim the outbound leg already spent"
    );
}
