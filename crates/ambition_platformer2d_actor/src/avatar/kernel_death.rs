//! Death publication for the exploration home body.

use bevy::prelude::*;
use ambition_platformer2d_core as ae;
use crate::body::BodyFrameOutput;

/// Publish the home body's movement reset as a death fact.
/// This runs after integration and before participant death policy.
pub fn publish_kernel_reset_death(
    mut died: MessageWriter<ambition_combat::death_rules::ActorDiedMessage>,
    bodies: Query<
        (Entity, &BodyFrameOutput),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    for (victim, frame_out) in &bodies {
        let Some(reset) = frame_out.reset else {
            continue;
        };
        died.write(ambition_combat::death_rules::ActorDiedMessage {
            victim,
            pos: reset.origin,
            // The kernel gate now says WHICH world killed her, so this reports
            // it instead of apologizing for not knowing. No entity claims any
            // of these kills — `attacker` stays `None` for all of them.
            cause: ambition_combat::death_rules::DeathCause {
                source: death_source_of(reset.cause),
                attacker: None,
            },
        });
    }
}

/// The killing category a kernel reset belongs to.
///
/// A voluntary reset is still a death: the run ended, the lives counter should
/// spend one, and a player who presses the restart verb in a platformer expects
/// the attempt to be over. It is charged to `Hazard` — the same anonymous
/// world-killed-you category the spikes use — because no vocabulary exists for
/// "you asked", and inventing one would only be honest if something read it.
fn death_source_of(cause: ae::ResetCause) -> ambition_combat::HitSource {
    match cause {
        ae::ResetCause::LeftTheWorld => ambition_combat::HitSource::LeftTheWorld,
        ae::ResetCause::Hazard | ae::ResetCause::Drowned | ae::ResetCause::Requested => {
            ambition_combat::HitSource::Hazard
        }
    }
}

#[cfg(test)]
mod kernel_reset_death_tests {
    use super::*;
    use bevy::prelude::App;

    #[test]
    fn kernel_reset_death_reports_the_pre_respawn_impact_position() {
        let mut app = App::new();
        app.add_message::<ambition_combat::death_rules::ActorDiedMessage>();
        app.add_systems(Update, publish_kernel_reset_death);

        let impact = ambition_platformer2d_core::Vec2::new(321.0, -45.0);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            crate::body::BodyFrameOutput {
                reset: Some(crate::body::BodyReset {
                    cause: ambition_platformer2d_core::ResetCause::Hazard,
                    origin: impact,
                }),
                ..default()
            },
        ));

        app.update();

        let deaths: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ambition_combat::death_rules::ActorDiedMessage>>()
            .drain()
            .collect();
        assert_eq!(deaths.len(), 1);
        assert_eq!(
            deaths[0].pos, impact,
            "the death fact must preserve where the hazard struck, not the spawn destination"
        );
    }
}
