use ambition_sprite_sheet::character::rigged::ComposedBodyDemand;
use bevy::prelude::*;

const FADE_SECONDS: f32 = 0.22;
const FIRST_FRAME_OPACITY: f32 = 0.18;

#[derive(Component, Default)]
pub(super) struct ActorEntranceFade {
    elapsed: f32,
}

/// Apply the fade after the animator and passage set the body's tint.
pub(super) fn present_actor_entrances(
    mut commands: Commands,
    time: ambition_time::PresentationTime,
    mut actors: Query<(Entity, &mut Sprite, &mut ActorEntranceFade)>,
    mut demand: ResMut<ComposedBodyDemand>,
) {
    for (body, mut sprite, mut fade) in &mut actors {
        if !fade.is_added() {
            fade.elapsed += time.scaled_dt();
        }
        if fade.elapsed >= FADE_SECONDS {
            commands.entity(body).try_remove::<ActorEntranceFade>();
            continue;
        }

        let progress = fade.elapsed / FADE_SECONDS;
        let eased = progress * progress * (3.0 - 2.0 * progress);
        // The body can take part in combat from its first frame.
        let opacity = FIRST_FRAME_OPACITY + (1.0 - FIRST_FRAME_OPACITY) * eased;
        let alpha = sprite.color.alpha() * opacity;
        sprite.color.set_alpha(alpha);
        demand.declare(body);
    }
}
