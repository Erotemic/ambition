//! Shared presentation/simulation beat for body transformations.
//!
//! During the authored wall-clock duration it pins an animation pose, publishes
//! a [`ClockScaleRequest`], and optionally makes the body untouchable. The beat
//! never writes global clock scale directly; active regime policy decides whether
//! a requested dilation is allowed. Wall time prevents the beat from stretching
//! its own duration when dilation is granted.

use bevy::prelude::*;

use ambition_characters::actor::BodyHealth;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};
use ambition_time::{ClockDomain, WorldTime};

use ambition_time::time_control::{ClockRequester, ClockScaleRequest};

/// What a transformation looks like for THIS body.
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
///
/// A component, deliberately, and not a message. The identity change and the
/// decision to celebrate it have to survive a rollback together or the pair is
/// not a transaction. Every producer necessarily runs LATER in the frame than
/// [`begin_requested_transform_beats`] consumes — Mary-O's grow must follow the
/// engine's `collect_world_items`, and the consumer must precede `Combat` so a
/// body transforming this frame is untouchable for this frame's hits — so a
/// request always waits a frame. As a message it was cleared by GGRS `LoadWorld`
/// in exactly that gap, which restored the transformed identity and lost its
/// beat permanently. As rollback-registered state it is restored with the
/// identity that caused it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TransformBeatRequested;

/// Start a beat for each requesting body, if it authored one.
///
/// Re-requesting during a beat RESTARTS it rather than stacking: two powerups
/// in half a second is one transformation as far as the player can see.
pub fn begin_requested_transform_beats(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            Option<&TransformBeatPolicy>,
            Option<&mut BodyHealth>,
        ),
        With<TransformBeatRequested>,
    >,
) {
    for (body, policy, health) in &mut bodies {
        // The request is consumed either way: an unauthored body transforms
        // instantly, which is what every body did before this seam existed, and
        // a request left behind would re-fire every frame.
        commands.entity(body).remove::<TransformBeatRequested>();
        let Some(policy) = policy else {
            continue;
        };
        // TAKE the transformation's own reason. Nothing is captured and
        // nothing is restored: a star burning through this transformation holds
        // `EMPOWERED` the whole time and is unaffected by us taking and releasing
        // `TRANSFORMING`, which is the entire reason invulnerability is a set.
        if policy.untouchable {
            if let Some(mut health) = health {
                health
                    .health
                    .invulnerable
                    .set(ambition_characters::actor::Invulnerability::TRANSFORMING, true);
            }
        }
        commands.entity(body).try_insert(TransformBeat {
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
    mut bodies: Query<(Entity, &mut TransformBeat, Option<&mut BodyHealth>)>,
) {
    // WALL time: a beat that slows the clock must not also slow its own timer,
    // or its duration becomes a function of its own effect.
    let dt = time.wall_dt();
    for (entity, mut beat, health) in &mut bodies {
        beat.remaining -= dt;
        let still_running = beat.remaining > 0.0;

        // EXACTLY ONE clock request per frame: the dilation while the beat runs,
        // the release on the frame it ends. Both on the ending frame would leave
        // the dilation in force for one extra frame, because
        // `apply_clock_scale_requests` keeps the strongest slow rather than the
        // last one written. Releasing by ASKING for 1.0 rather than writing the
        // scale keeps the regime in the loop on the way out too.
        if beat.policy.clock_scale != 1.0 {
            clock.write(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: if still_running {
                    beat.policy.clock_scale
                } else {
                    1.0
                },
                requester: ClockRequester::Engine,
                reason: if still_running {
                    "transform_beat"
                } else {
                    "transform_beat_end"
                },
            });
        }

        if still_running {
            commands
                .entity(entity)
                .try_insert(ActorAnimOverride(beat.policy.anim));
            continue;
        }

        // Done: release the pose, and release OUR reason only. A transformation
        // that happens to overlap a star leaves the star holding the body
        // untouchable, with nothing here having to know that.
        if beat.policy.untouchable {
            if let Some(mut health) = health {
                health
                    .health
                    .invulnerable
                    .set(ambition_characters::actor::Invulnerability::TRANSFORMING, false);
            }
        }
        commands
            .entity(entity)
            .remove::<ActorAnimOverride>()
            .remove::<TransformBeat>();
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
        use ambition_platformer2d_shared_tangle::schedule::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (begin_requested_transform_beats, run_transform_beats)
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );
    }
}

/// True while this body is mid-transformation and its policy says it cannot be
/// touched.
///
/// The ENFORCEMENT is not here: the beat sets the body's existing
/// `Health::invulnerable` for its duration and restores it after, so a
/// transformation is untouchable through the one seam every other invulnerable
/// state already uses — i-frames, Sanic's super form — rather than a second
/// intangibility mechanism the hit path would have to learn about. This
/// predicate is for readers that want to know WHY a body is untouchable.
pub fn body_is_transforming(beat: Option<&TransformBeat>) -> bool {
    beat.is_some_and(|beat| beat.policy.untouchable && beat.remaining > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
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
            .entity_mut(body)
            .insert(TransformBeatRequested);
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
            .entity_mut(body)
            .insert(TransformBeatRequested);

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
            .entity_mut(body)
            .insert(TransformBeatRequested);
        advance(&mut app, 0.05);

        let requests = app.world().resource::<Messages<ClockScaleRequest>>();
        let mut cursor = requests.get_cursor();
        let scales: Vec<f32> = cursor.read(requests).map(|request| request.scale).collect();
        assert!(
            scales.contains(&0.35),
            "the beat did not request its dilation ({scales:?})",
        );

        advance(&mut app, 0.3);
        let requests = app.world().resource::<Messages<ClockScaleRequest>>();
        let mut cursor = requests.get_cursor();
        let scales: Vec<f32> = cursor.read(requests).map(|request| request.scale).collect();
        assert!(
            scales.contains(&1.0),
            "the beat ended without releasing the clock ({scales:?})",
        );
    }

    /// The clause that makes `untouchable` real, and the trap that used to be
    /// in it: a body can ALREADY be untouchable for a reason of its own — a
    /// burning star, Sanic's super form — when a transformation starts.
    ///
    /// It takes and releases its own reason now, so both orderings hold for the same reason rather
    /// than one being handled and the other being an accident.
    #[test]
    fn the_beat_takes_its_own_reason_and_leaves_every_other_one_alone() {
        use ambition_characters::actor::Invulnerability;

        for star_first in [false, true] {
            let mut app = app();
            let mut health = BodyHealth::new(ambition_characters::actor::Health::new(3));
            if star_first {
                health.health.invulnerable.set(Invulnerability::EMPOWERED, true);
            }
            let body = app
                .world_mut()
                .spawn((
                    TransformBeatPolicy {
                        duration: 0.2,
                        ..Default::default()
                    },
                    health,
                ))
                .id();
            app.world_mut()
                .entity_mut(body)
                .insert(TransformBeatRequested);

            advance(&mut app, 0.05);
            let mid = app.world().get::<BodyHealth>(body).unwrap().health.invulnerable;
            assert!(
                mid.holds(Invulnerability::TRANSFORMING),
                "she cannot be hit out of her own transformation",
            );
            assert_eq!(
                mid.holds(Invulnerability::EMPOWERED),
                star_first,
                "and the beat neither invents nor forgets somebody else's reason",
            );

            // A star that begins DURING the beat — the ordering the old
            // save-and-restore could not survive, because the restore was
            // decided before this reason existed.
            if !star_first {
                app.world_mut()
                    .get_mut::<BodyHealth>(body)
                    .unwrap()
                    .health
                    .invulnerable
                    .set(Invulnerability::EMPOWERED, true);
            }

            advance(&mut app, 0.3);
            let after = app.world().get::<BodyHealth>(body).unwrap().health.invulnerable;
            assert!(
                !after.holds(Invulnerability::TRANSFORMING),
                "the beat released its own reason when it ended",
            );
            assert!(
                after.holds(Invulnerability::EMPOWERED),
                "and the star outlives it, whichever order they started in",
            );
            assert!(after.any(), "so the body is still untouchable");
        }
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
            .entity_mut(body)
            .insert(TransformBeatRequested);
        advance(&mut app, 0.2);
        app.world_mut()
            .entity_mut(body)
            .insert(TransformBeatRequested);
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
