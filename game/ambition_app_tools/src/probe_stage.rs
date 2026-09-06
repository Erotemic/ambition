//! SEATING A REAL SMASH MATCH AND TAKING THE BRAIN OFF — the setup every
//! move probe needs, and the part that was hard to get right.
//!
//! ⛔⛔ THIS IS A `#[path]` MODULE, NOT A LIB, and that is deliberate: this
//! package's `Cargo.toml` says in as many words that it has no lib on purpose,
//! because a lib would relink every binary in it whenever any of them changed.
//! Two binaries including one file share the code without buying that.
//!
//! ⭐⭐ IT EXISTS BECAUSE `trap_probe` PAID FOR ALL OF IT. Four separate
//! findings are baked into the sixty lines below, and every one of them was a
//! run that measured nothing:
//!
//! * `VisibleRenderMode::NoWindow` sets `backends: None` and omits the render
//!   app ENTIRELY, so every presentation number a probe reports under it is a
//!   zero that means nothing.
//! * A hand-stepped `app.update()` does NOT wait for the wgpu device that
//!   `app.run()` waits for, and panics inside `no_automatic_skin_batching` with
//!   *"Res\<RenderDevice\> ... Resource does not exist"*.
//! * The two hosts do not announce the round the same way, and waiting on the
//!   app host's condition against the demo shell panics on a live match.
//! * A seated fighter carries a `Brain` that writes its own `ControlFrame`
//!   every tick, so a frame delivered from outside is overwritten before the
//!   kernel reads it. Two probe runs measured nothing for exactly this.
//!
//! ⛔ A PROBE THAT REDISCOVERS ANY OF THESE HAS SPENT A DAY TO ARRIVE WHERE
//! THIS FILE STARTS.

#![allow(dead_code)]

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::engine_core::BodyKinematics;
use bevy::prelude::*;

/// Which shell to measure in.
pub struct StageRequest<'a> {
    /// The two fighters to seat, by character id.
    pub cast: [&'a str; 2],
    /// ⭐⭐ `run_game.sh smash` launches `ambition_demo_smash_app`, NOT
    /// `ambition_app` — a different shell composing its own catalogs. A probe
    /// that only ever measures the app host cannot see a defect that lives in
    /// the shell somebody plays.
    pub demo_host: bool,
    /// ⭐⭐ GIVE THIS PROBE A PRESENTATION LAYER. Without it there is no render
    /// app at all and no visual question is answerable.
    pub rendered: bool,
}

/// A seated, live match with seat 0's brain removed and its feet on the boards.
pub struct Staged {
    pub app: App,
    pub seat0: Entity,
    pub seat1: Entity,
}

/// Build the host, seat the roster, wait for the round to be live, and hand
/// seat 0's controller to the caller.
pub fn stage(request: StageRequest<'_>) -> Staged {
    let StageRequest {
        cast,
        demo_host,
        rendered,
    } = request;
    let mut app = if demo_host {
        ambition_demo_smash_app::build_demo_app()
    } else if rendered {
        // ⛔ MIRRORING `capture_scene` EXACTLY, because a bare
        // `build_visible_app(OffscreenGpu, ..)` panics here in
        // `no_automatic_skin_batching`. The working recipe is
        // `build_visible_app_with` plus a declared `HeadlessDisplaySurface`.
        let mut app = ambition_app::app::build_visible_app_with(
            ambition_app::app::VisibleRenderMode::OffscreenGpu,
            true,
            |_| {},
        );
        app.insert_resource(
            ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
                ambition_platformer2d::engine_core::Vec2::new(960.0, 540.0),
            ),
        );
        app
    } else {
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true)
    };
    // ⛔⛔ WAIT FOR THE PLUGINS, WHICH `app.run()` DOES FOR YOU AND
    // `app.update()` DOES NOT. `OffscreenGpu` initialises its wgpu device
    // ASYNCHRONOUSLY during plugin finish.
    if rendered {
        while app.plugins_state() != bevy::app::PluginsState::Ready {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
    }
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster(cast));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // ⛔⛔ THE TWO HOSTS DO NOT ANNOUNCE THE ROUND THE SAME WAY. The app host
    // holds its cast under `ScriptedControl` through the ceremony and releasing
    // it is observable; the demo shell boots straight into `smash_stage` and
    // never satisfies that condition, so waiting on it panics against a match
    // that is perfectly live. Wait on the ROSTER'S OWN countdown there.
    if demo_host {
        let countdown = ambition_demo_smash::smash_roster(cast)
            .rules
            .opening_countdown_ticks;
        for _ in 0..(countdown as usize + 60) {
            app.update();
        }
    } else {
        let mut live = false;
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
                live = true;
                break;
            }
        }
        assert!(live, "the opening ceremony never released the cast");
    }

    let (seat0, seat1) = seats(&mut app);
    // ⛔⛔ TAKE THE BRAIN OFF, or every frame delivered from outside is
    // overwritten before the kernel reads it.
    app.world_mut()
        .entity_mut(seat0)
        .remove::<ambition_platformer2d::characters::brain::Brain>();
    app.update();
    Staged { app, seat0, seat1 }
}

pub fn seats(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &MatchSeat)>();
    let mut rows: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
    rows.sort_by_key(|(seat, _)| *seat);
    (
        rows.first().expect("the match seats a first fighter").1,
        rows.get(1).expect("the match seats a second fighter").1,
    )
}

pub fn kin(app: &App, body: Entity) -> (Vec2, Vec2) {
    let k = app
        .world()
        .get::<BodyKinematics>(body)
        .expect("the fighter still has a body");
    (Vec2::new(k.pos.x, k.pos.y), Vec2::new(k.vel.x, k.vel.y))
}

/// Stand `body` at `pos`, at rest, THROUGH THE MOVEMENT AUTHORITY.
///
/// ⛔ NOT `kin.pos.x = `. A probe places a real fighter in a real match, so the
/// body it moves has contacts, an attachment and a resolved frame — the things
/// `transit_body` reconciles and a bare field write silently keeps. A probe
/// whose fixture leaves a body standing on a surface it is no longer touching
/// measures the fixture.
///
/// `officer_probe` needed this to stand the second fighter out of the firing
/// lane; every later probe gets the seam instead of the field.
pub fn place(app: &mut App, body: Entity, pos: Vec2) {
    let world = app.world_mut();
    let mut q = world.query::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    )>();
    let Ok((mut cluster_item, mut model)) = q.get_mut(world, body) else {
        return;
    };
    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut model,
        &mut clusters,
        pos,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

pub fn playing_move(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<ambition_platformer2d::combat::moveset::MovePlayback>(body)
        .map(|p| format!("{}@{:.2}", p.spec.id, p.t))
}

/// How many bodies the presentation layer has built.
///
/// ⛔⛔ THE INSTRUMENT PROVES ITSELF FIRST. Every presentation count a probe
/// takes queries a component, and a presentation layer that was never installed
/// answers zero for the same reason a missing visual does.
pub fn player_visuals(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::platformer::lifecycle::PlayerVisual>();
    q.iter(world).count()
}

/// Live hitboxes this body owns.
pub fn hitbox_count(app: &mut App, owner: Entity) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ambition_platformer2d::combat::strike::Hitbox>();
    q.iter(world).filter(|hb| hb.owner == owner).count()
}
