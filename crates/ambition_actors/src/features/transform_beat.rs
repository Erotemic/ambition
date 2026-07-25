//! **The transformation moment** — the beat a body plays when it becomes
//! something else.
//!
//! Jon asked for this twice, once per demo: *"in mary-o we need a 'growing'
//! animation when she grows or transforms. In single player this might request
//! that time around the transforming character slows down as an effect, but in a
//! multi-player setting the time slow needs to be agreed upon by all players"*,
//! and *"similarly to mary-o sanic needs the transform animation"*.
//!
//! Two demos asking for the same thing is the shape of an engine capability, not
//! of two demo hacks. So the beat is one thing here, and what each game owns is
//! only the DECISION that a transformation happened plus the numbers it wants.
//!
//! ## The multiplayer clause is structural, not promised
//!
//! The beat never touches `ClockState::time_scale`. It writes a
//! [`ClockScaleRequest`], and `apply_clock_scale_requests` consults the active
//! [`RegimePolicy`] before granting one — `Regime::Solo` grants, and a regime
//! that cannot afford one participant bending everyone's clock denies. That is
//! the agreement Jon is describing: a transformation ASKS for time to slow, and
//! the regime in force answers. A demo that dilated time by writing the scale
//! itself would be correct in single-player and wrong the moment a second
//! participant existed, silently.
//!
//! ## What it does
//!
//! For its duration: pins the shown pose (`ActorAnimOverride`), asks for the
//! authored clock scale every frame, and — if the policy says so — makes the
//! body untouchable, because a transformation you can be hit out of is a
//! punishment for collecting a powerup.
//!
//! It ticks on WALL time, not sim time. A beat that dilates the clock and then
//! measures itself with the dilated clock stretches itself by its own effect.

use bevy::prelude::*;

use ambition_sprite_sheet::character::CharacterAnim;
use ambition_time::{ClockDomain, WorldTime};

use super::ecs::actor_clusters::ActorAnimOverride;
use crate::time::time_control::{ClockRequester, ClockScaleRequest};

/// What a transformation looks like for THIS body. Authored per character by
/// the game; absent means transformations are instant, which is a legitimate
/// choice and what every body did before this existed.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TransformBeatPolicy {
    /// Wall-clock seconds the beat holds.
    pub duration: f32,
    /// The pose held while it runs.
    pub anim: CharacterAnim,
    /// Clock scale to REQUEST while it runs. `1.0` asks for nothing.
    pub clock_scale: f32,
    /// Whether the body is untouchable for the duration.
    pub untouchable: bool,
}

impl Default for TransformBeatPolicy {
    fn default() -> Self {
        Self {
            duration: 0.45,
            anim: CharacterAnim::Idle,
            clock_scale: 1.0,
            untouchable: true,
        }
    }
}

/// A transformation beat in progress.
///
/// Inserted by [`begin_requested_transform_beats`] from the body's policy, so a
/// game never states the duration twice. Registered snapshot state: it gates
/// whether the body can be hit, and anything that can cause an input or a hit to
/// be IGNORED is simulation state regardless of which struct it lives on.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TransformBeat {
    pub remaining: f32,
    pub policy: TransformBeatPolicy,
}

/// "This body just became something else." Written by whoever owns the meaning
/// of that — Mary-O's power tiers, Sanic's super form — because only the game
/// knows which identity changes are a transformation and which are a downgrade.
#[derive(Message, Clone, Copy, Debug)]
pub struct TransformBeatRequested {
    pub body: Entity,
}

/// Start a beat for each request, if the body authored one.
///
/// Re-requesting during a beat RESTARTS it rather than stacking: two powerups
/// in half a second is one transformation as far as the player can see.
pub fn begin_requested_transform_beats(
    mut requests: MessageReader<TransformBeatRequested>,
    mut commands: Commands,
    bodies: Query<&TransformBeatPolicy>,
) {
    for request in requests.read() {
        let Ok(policy) = bodies.get(request.body) else {
            // No authored beat: the transformation is instant. Not an error —
            // it is what every body did before this seam existed.
            continue;
        };
        commands.entity(request.body).try_insert(TransformBeat {
            remaining: policy.duration,
            policy: *policy,
        });
    }
}

/// Hold the pose, ask for the time dilation, and end the beat.
pub fn run_transform_beats(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut clock: MessageWriter<ClockScaleRequest>,
    mut bodies: Query<(Entity, &mut TransformBeat)>,
) {
    // WALL time: a beat that slows the clock must not also slow its own timer,
    // or its duration becomes a function of its own effect.
    let dt = time.wall_dt();
    for (entity, mut beat) in &mut bodies {
        if beat.policy.clock_scale != 1.0 {
            clock.write(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: beat.policy.clock_scale,
                requester: ClockRequester::Engine,
                reason: "transform_beat",
            });
        }
        commands
            .entity(entity)
            .try_insert(ActorAnimOverride(beat.policy.anim));

        beat.remaining -= dt;
        if beat.remaining > 0.0 {
            continue;
        }
        // Done: release the pose and hand the clock back. Releasing by asking
        // for 1.0 rather than by writing the scale keeps the regime in the loop
        // on the way out too.
        commands
            .entity(entity)
            .remove::<ActorAnimOverride>()
            .remove::<TransformBeat>();
        if beat.policy.clock_scale != 1.0 {
            clock.write(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: 1.0,
                requester: ClockRequester::Engine,
                reason: "transform_beat_end",
            });
        }
    }
}

/// Register the beat.
///
/// `PlayerSimulation` because the beat is body state that gates being hit, and
/// the Combat phase runs after it: a body that begins transforming this frame is
/// untouchable for the hits resolved this frame, not the next one.
pub struct TransformBeatPlugin;

impl Plugin for TransformBeatPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer_primitives::schedule::{SandboxSet, SimScheduleExt};
        app.add_message::<TransformBeatRequested>();
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (begin_requested_transform_beats, run_transform_beats)
                .chain()
                .in_set(SandboxSet::PlayerSimulation),
        );
    }
}

/// True while this body is mid-transformation and its policy says it cannot be
/// touched. Consulted by the hit path the same way `body_is_corpse` is.
pub fn body_is_transforming(beat: Option<&TransformBeat>) -> bool {
    beat.is_some_and(|beat| beat.policy.untouchable && beat.remaining > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<TransformBeatRequested>();
        app.add_message::<ClockScaleRequest>();
        app.init_resource::<WorldTime>();
        app.add_systems(
            Update,
            (begin_requested_transform_beats, run_transform_beats).chain(),
        );
        app
    }

    fn advance(app: &mut App, seconds: f32) {
        app.world_mut().resource_mut::<WorldTime>().raw_dt = seconds;
        app.update();
    }

    #[test]
    fn a_body_with_no_policy_transforms_instantly() {
        let mut app = app();
        let body = app.world_mut().spawn(()).id();
        app.world_mut()
            .write_message(TransformBeatRequested { body });
        advance(&mut app, 0.016);
        assert!(
            app.world().get::<TransformBeat>(body).is_none(),
            "a body that authored no beat should not have acquired one",
        );
    }

    #[test]
    fn the_beat_holds_its_pose_and_then_gives_it_back() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn(TransformBeatPolicy {
                duration: 0.3,
                anim: CharacterAnim::Taunt,
                clock_scale: 1.0,
                untouchable: true,
            })
            .id();
        app.world_mut()
            .write_message(TransformBeatRequested { body });

        advance(&mut app, 0.1);
        assert_eq!(
            app.world().get::<ActorAnimOverride>(body).map(|o| o.0),
            Some(CharacterAnim::Taunt),
            "the beat did not pin the pose it authored",
        );
        assert!(body_is_transforming(app.world().get::<TransformBeat>(body)));

        advance(&mut app, 0.3);
        assert!(
            app.world().get::<TransformBeat>(body).is_none(),
            "the beat outlived its duration",
        );
        assert!(
            app.world().get::<ActorAnimOverride>(body).is_none(),
            "the beat kept the pose pinned after it ended — the body is now \
             frozen in its transformation pose forever",
        );
    }

    /// The clause that makes the multiplayer story true: the beat ASKS, through
    /// the channel a regime can refuse. If this ever becomes a direct write to
    /// `time_scale`, single-player still looks right and multiplayer is broken.
    #[test]
    fn the_dilation_is_a_request_not_a_write() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn(TransformBeatPolicy {
                duration: 0.2,
                anim: CharacterAnim::Idle,
                clock_scale: 0.35,
                untouchable: true,
            })
            .id();
        app.world_mut()
            .write_message(TransformBeatRequested { body });
        advance(&mut app, 0.05);

        let requests = app.world().resource::<Messages<ClockScaleRequest>>();
        let mut cursor = requests.get_cursor();
        let scales: Vec<f32> = cursor.read(requests).map(|request| request.scale).collect();
        assert!(
            scales.contains(&0.35),
            "the beat did not request its dilation ({scales:?})",
        );

        // And it hands the clock back rather than leaving the world in slow
        // motion — the failure mode where one powerup slows the rest of the run.
        advance(&mut app, 0.3);
        let requests = app.world().resource::<Messages<ClockScaleRequest>>();
        let mut cursor = requests.get_cursor();
        let scales: Vec<f32> = cursor.read(requests).map(|request| request.scale).collect();
        assert!(
            scales.contains(&1.0),
            "the beat ended without releasing the clock ({scales:?})",
        );
    }

    #[test]
    fn a_second_transformation_restarts_the_beat_instead_of_stacking() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn(TransformBeatPolicy {
                duration: 0.3,
                ..Default::default()
            })
            .id();
        app.world_mut()
            .write_message(TransformBeatRequested { body });
        advance(&mut app, 0.2);
        app.world_mut()
            .write_message(TransformBeatRequested { body });
        advance(&mut app, 0.05);

        let remaining = app
            .world()
            .get::<TransformBeat>(body)
            .expect("the beat restarted")
            .remaining;
        assert!(
            remaining > 0.2,
            "the second transformation did not restart the beat ({remaining})",
        );
    }
}
