//! Seat a real Smash match and take the brain off: the setup every move probe
//! needs.
//!
//! A `#[path]` module, not a lib. This package has no lib on purpose (see its
//! `Cargo.toml`), because a lib relinks every binary when any of them
//! changes. Binaries include this file directly.
//!
//! It handles four pitfalls:
//!
//! * `VisibleRenderMode::NoWindow` sets `backends: None` and omits the render
//!   app, so every presentation count under it is zero.
//! * A hand-stepped `app.update()` does not wait for the wgpu device that
//!   `app.run()` waits for, and panics inside `no_automatic_skin_batching`
//!   with *"Res\<RenderDevice\> ... Resource does not exist"*.
//! * The two hosts announce the round differently; waiting on the app host's
//!   condition in the demo shell panics on a live match.
//! * A seated fighter's `Brain` writes its own `ControlFrame` every tick, so
//!   a frame delivered from outside is overwritten before the kernel reads it.

#![allow(dead_code)]

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::engine_core::BodyKinematics;
use bevy::prelude::*;

/// Which shell to measure in.
pub struct StageRequest<'a> {
    /// The two fighters to seat, by character id.
    pub cast: [&'a str; 2],
    /// `run_game.sh smash` launches `ambition_demo_smash_app`, not
    /// `ambition_app`; that shell composes its own catalogs.
    pub demo_host: bool,
    /// Give the probe a presentation layer. Without it there is no render app.
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
        // Mirror `capture_scene`: a bare `build_visible_app(OffscreenGpu, ..)`
        // panics in `no_automatic_skin_batching`. Use
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
    // Wait for the plugins, which `app.run()` does and `app.update()` does
    // not. `OffscreenGpu` initialises its wgpu device asynchronously during
    // plugin finish.
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
    // An untimed match. A probe runs far longer than
    // `SMASH_TIME_LIMIT_TICKS` (`8 * 60 * 60` = 28_800). When the clock runs
    // out, the cast is despawned, handles dangle, and every later trial
    // refuses with no message.
    //
    // `0` means untimed, not expire at once: `MatchRules::time_remaining` is
    // `(self.time_limit_ticks > 0).then(..)`, and `time_expired` is false for
    // an untimed match.
    //
    // Set it on the roster. `PreparedMatch` keeps `rules` private, so the
    // plan cannot change after preparation. `MatchParticipantRoster::rules`
    // is public for this; `smash_roster` sets the clock the same way.
    let mut roster = ambition_demo_smash::smash_roster(cast);
    roster.rules.time_limit_ticks = 0;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // The two hosts announce the round differently. The app host holds its
    // cast under `ControlHolds` through the ceremony, and the release is
    // observable. The demo shell boots straight into `smash_stage` and never
    // meets that condition, so wait on the roster's own countdown there.
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
                    With<ambition_platformer2d::characters::control::ControlHolds>,
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
    // Take the brain off, or every frame delivered from outside is
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

/// Stand `body` at `pos`, at rest, through the movement authority.
///
/// Not `kin.pos.x = `. A real fighter has contacts, an attachment, and a
/// resolved frame; `transit_body` reconciles them, and a bare field write
/// keeps stale ones.
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

/// Stand `body` at `pos`, at rest, without disturbing its current contacts.
/// This is the counterpart to [`place`]; they are not interchangeable.
///
/// This uses `constrain_body_pose`, not `transit_body`. [`place`] is the
/// discrete-transit authority: `reconcile_transit` clears ground and wall
/// contact by design. That suits a teleport, but a fixture that stands a body
/// on a floor and asserts it is standing needs the contacts kept (with
/// [`place`], the body does not re-ground). The pin "does not fabricate or
/// clear contact facts", and it is the authority
/// `engine.pose-writes-are-authority-only` asks for.
///
/// Use this when the body is already supported and slides along its surface.
/// Use [`place`] when the body teleports and should lose its contacts.
pub fn pin_grounded_at_rest(app: &mut App, body: Entity, pos: Vec2) {
    let Some(mut kin) = app.world_mut().get_mut::<BodyKinematics>(body) else {
        return;
    };
    ambition_platformer2d::engine_core::movement::constrain_body_pose(
        &mut kin,
        // A fixture placement is not travel, so the sample ends here and
        // readers fall back to the live pose.
        None,
        pos,
        Vec2::ZERO,
    );
}

pub fn playing_move(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<ambition_platformer2d::combat::moveset::MovePlayback>(body)
        .map(|p| format!("{}@{:.2}", p.spec.id, p.t))
}

/// How many bodies the presentation layer has built.
///
/// The instrument proves itself first. A presentation layer that was never
/// installed also answers zero for every presentation count.
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
