//! Real-ECS headless fighter harness (architecture roadmap S0).
//!
//! The unified-control architecture
//! (`docs/planning/fighter-capability-and-motor-unification.md`) hangs on a
//! contract: *any controller drives any body through one input seam, and the
//! **body** enforces all physics.* That contract has to be proven against the
//! **real** simulation systems, not a proxy with its own kinematics. This module
//! is the seed of that harness: it builds a minimal but real headless `App` —
//! real messages, the real `emit_brain_action_messages` resolver and the real
//! `spawn_enemy_projectiles_from_brain_actions` body-enforcement system — drops a
//! body, lets a chosen controller drive the one input seam (`ActorControlFrame`),
//! and ticks.
//!
//! It deliberately drives the body through the **same** `ActorControlFrame` seam a
//! brain, a possessing human, or a future RL policy use, so a test controller is
//! substitutable for any of them (invariant I1). As later slices land (unified
//! motor, full capability parity, headless perception), this harness grows to
//! drop full bodies in a real room and tick the whole actor pipeline; today it
//! covers the first migrated intent — fire (S1) — and proves the body owns the
//! fire rate (invariant I3).
//!
//! This is test-support; it is compiled only under `cfg(test)`. The proxy arena
//! in `ambition_characters::brain::smash::arena` (own kinematics, no terrain) is
//! retired in favor of harnesses like this for "works in a fight" claims; the
//! brain's pure-stage unit tests stay for fast checks.

#![cfg(test)]

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::{
    action_set::ActionRequest, ActionSet, ActorActionMessage, ActorControl, RangedActionSpec,
};

use super::actor_clusters::ActorClusterSeed;
use super::brain_effects::spawn_enemy_projectiles_from_brain_actions;
use crate::enemy_projectile::test_support::enemy_projectile_bodies;
use crate::enemy_projectile::EnemyProjectileState;
use crate::projectile::ProjectileSeqCounter;

/// Fixed simulation step the harness ticks at (s). Matches the engine's nominal
/// 60 Hz so body cooldowns measured in seconds map cleanly to tick counts.
pub const HARNESS_DT: f32 = 1.0 / 60.0;

/// Tick the body's per-frame attack timers, exactly as the real integrator does
/// (`em.update()` → `BodyMelee::tick`). Isolated here so the harness
/// advances body cooldowns without standing up the full integration system; the
/// fire-rate enforcement under test reads the same `ranged_cooldown` this decays.
fn tick_body_cooldowns(mut q: Query<&mut crate::features::BodyMelee>) {
    for mut attack in &mut q {
        // Advances the melee swing (none armed on the fire path) and the
        // `ranged_cooldown` floor the fire-rate test reads.
        attack.tick(HARNESS_DT);
    }
}

/// A controller plugged into the one input seam: given the current tick index it
/// returns the `ActorControlFrame` it wants the body to attempt this tick. A
/// brain, a possessing human, and a future RL policy are all just functions of
/// this shape (invariant I1) — the harness does not care which it is.
pub type Controller = dyn Fn(u32) -> ActorControlFrame;

/// A headless fighter body driven by a controller over the real fire pipeline.
pub struct FighterHarness {
    app: App,
    body: Entity,
    tick: u32,
}

impl FighterHarness {
    /// Drop one ranged-capable hostile body at `pos` and wire the real fire
    /// pipeline: `emit_brain_action_messages` (resolve the seam into action
    /// requests) → `spawn_enemy_projectiles_from_brain_actions` (the body's
    /// fire-rate enforcement) → `apply_projectile_effects` (materialize shots),
    /// with `tick_body_cooldowns` decaying the body's weapon cooldown each frame.
    pub fn ranged_body_at(pos: ae::Vec2) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // The real pipeline, in order: tick body cooldowns, resolve the seam,
        // enforce + emit fire effects, then materialize the projectiles.
        app.add_systems(
            Update,
            (
                tick_body_cooldowns,
                ambition_characters::brain::emit_brain_action_messages,
                spawn_enemy_projectiles_from_brain_actions,
                crate::enemy_projectile::apply_projectile_effects,
            )
                .chain(),
        );

        let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
        let seed = ActorClusterSeed::new(
            "harness_ranged",
            "Harness Ranged Body",
            aabb,
            ambition_characters::actor::EnemyBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        let action_set = ActionSet {
            ranged: Some(RangedActionSpec::Rock {
                speed: 300.0,
                damage: 1,
            }),
            ..ActionSet::peaceful()
        };
        let body = app
            .world_mut()
            .spawn((
                crate::features::ActorDisposition::Hostile,
                seed.into_components(),
                action_set,
                ActorControl::default(),
                ActorPose::from_parts(pos, ae::Vec2::new(14.0, 23.0), 1.0),
            ))
            .id();
        Self { app, body, tick: 0 }
    }

    /// Run `ticks` simulation steps, driving the body each tick with the frame
    /// `controller` returns. Returns the number of projectiles the body actually
    /// launched over the window — the *body's* output rate, whatever the
    /// controller attempted.
    pub fn run(&mut self, ticks: u32, controller: &Controller) -> usize {
        for _ in 0..ticks {
            let frame = controller(self.tick);
            if let Some(mut ctrl) = self.app.world_mut().get_mut::<ActorControl>(self.body) {
                ctrl.0 = frame;
            }
            self.app.update();
            self.tick += 1;
        }
        enemy_projectile_bodies(&mut self.app).len()
    }
}

/// A controller that attempts a ranged shot on EVERY tick — the degenerate spam
/// line a learned policy is free to probe (invariant I4). Frame-agnostic world-
/// space aim to the right.
fn spam_fire(_tick: u32) -> ActorControlFrame {
    let mut f = ActorControlFrame::neutral();
    f.fire = Some(
        ambition_characters::actor::control::ActorFireRequest::world_space(
            ae::Vec2::new(1.0, 0.0),
            300.0,
        ),
    );
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spam-equivalence (invariant I3): a controller that attempts a shot every
    /// single tick does not out-fire the body's weapon rate. Over a 2.0 s window
    /// (120 ticks) with a 1.1 s body refire, the body launches exactly the shots
    /// its cooldown allows — at t=0 and t≈1.1 s — i.e. 2, not 120. The controller
    /// "spamming" is harmless; the body is the floor.
    #[test]
    fn spam_controller_fires_at_the_body_rate_not_the_tick_rate() {
        let mut h = FighterHarness::ranged_body_at(ae::Vec2::new(300.0, 300.0));
        let window_ticks = (2.0 / HARNESS_DT) as u32; // 120 ticks ≈ 2.0 s
        let shots = h.run(window_ticks, &spam_fire);
        assert_eq!(
            shots, 2,
            "a 60 Hz spam controller over 2.0 s with a 1.1 s body refire must \
             produce the body's rate (2 shots), not the tick rate (120)"
        );
    }

    /// The body rate is bounded by the *body*, not the attempt rate: a controller
    /// attempting every other tick (30 Hz) launches the same number of shots as
    /// the 60 Hz spammer over the same window — up to a single-tick phase
    /// quantization (a refire window can expire on a tick the slower controller
    /// skips, deferring that one shot by a frame). Both are far below the attempts
    /// they made. This is the property that makes an AI controller and a
    /// possessing human equivalent: doubling the attempt rate does not double the
    /// output.
    #[test]
    fn output_rate_is_bounded_by_the_body_not_the_attempt_rate() {
        let window_ticks = (3.4 / HARNESS_DT) as u32; // ≈ 3.4 s, ~4 refire windows

        let mut fast = FighterHarness::ranged_body_at(ae::Vec2::new(300.0, 300.0));
        let fast_shots = fast.run(window_ticks, &spam_fire);

        let mut slow = FighterHarness::ranged_body_at(ae::Vec2::new(300.0, 300.0));
        // Attempt every other tick — half the spam rate, still far above the body.
        let slow_shots = slow.run(window_ticks, &|tick| {
            if tick % 2 == 0 {
                spam_fire(tick)
            } else {
                ActorControlFrame::neutral()
            }
        });

        assert_eq!(
            fast_shots, 4,
            "~four refire windows fit in 3.4 s at 1.1 s each"
        );
        assert!(
            fast_shots.abs_diff(slow_shots) <= 1,
            "halving a still-superfast attempt rate must not change the body output \
             rate by more than one-tick phase quantization: fast={fast_shots}, slow={slow_shots}"
        );
        assert!(
            slow_shots * 10 < window_ticks as usize,
            "the body output rate stays far below the attempt rate (body-bound, not attempt-bound)"
        );
    }

    /// Sanity: a controller that never attempts a shot fires nothing — the body
    /// only ever resolves intents the controller actually emits (it is not the
    /// controller; it does not act on its own).
    #[test]
    fn idle_controller_never_fires() {
        let mut h = FighterHarness::ranged_body_at(ae::Vec2::new(300.0, 300.0));
        let shots = h.run(120, &|_| ActorControlFrame::neutral());
        assert_eq!(shots, 0, "no attempts → no shots");
    }

    /// The single Ranged-message path the harness exercises is the same one a
    /// brain drives in-engine: a `fire` frame + a `ranged` ActionSet resolves to
    /// exactly one `ActionRequest::Ranged`. Guards the seam the harness depends on.
    #[test]
    fn fire_frame_resolves_to_one_ranged_request() {
        let action_set = ActionSet {
            ranged: Some(RangedActionSpec::Rock {
                speed: 300.0,
                damage: 1,
            }),
            ..ActionSet::peaceful()
        };
        let requests = ambition_characters::brain::action_set::resolve(
            &action_set,
            &spam_fire(0),
            ae::Vec2::ZERO,
        );
        assert_eq!(requests.len(), 1);
        assert!(matches!(requests[0], ActionRequest::Ranged { .. }));
    }
}
