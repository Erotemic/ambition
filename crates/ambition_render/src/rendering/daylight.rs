//! The light of the time of day: a veil over the world of a sunlit room.
//!
//! One translucent sprite follows the gameplay camera above the world (the
//! play, the parallax foreground and the effects) and under the UI. Its
//! colour is the light of the hour: clear by day, gold in the morning, amber
//! and violet at dusk, deep blue at night. It is compositing, not drawing: a
//! translucent sprite, so it is the same with screen shaders off.
//!
//! A room is sunlit when its parallax theme is one the game names in its
//! [`DaylightStyle`]; elsewhere (under the ground, indoors) the veil is clear.
//! The hour comes from [`ambition_time::DayClock`] under the game's
//! [`ambition_time::DayCycle`]. A game with no style has no veil.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::PrimaryLiveRoom;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::ParallaxTheme;

/// Over the world and the parallax foreground (`FOREGROUND_PARALLAX_Z`), under the UI.
pub const DAYLIGHT_VEIL_Z: f32 = 60.0;
/// The veil is this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;

/// How a game's day looks: the light at given hours (the colour and alpha of
/// the veil, between which it goes smoothly round the clock), and which
/// themes are under the sky.
#[derive(Resource, Clone, Debug)]
pub struct DaylightStyle {
    /// `(hour, colour)` in any order; the veil at an hour between two is
    /// their mix, round midnight too.
    pub keys: Vec<(f32, Color)>,
    pub sunlit: Vec<ParallaxTheme>,
}

impl DaylightStyle {
    /// The light at `hour` (0 to 24).
    pub fn light_at(&self, hour: f32) -> Color {
        let mut keys = self.keys.clone();
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(&first) = keys.first() else {
            return Color::NONE;
        };
        let last = *keys.last().expect("not empty");
        let hour = hour.rem_euclid(24.0);
        // The keys before and after `hour`, round the clock.
        let (before, after) = match keys.iter().position(|(at, _)| *at > hour) {
            Some(0) | None => (last, (first.0 + 24.0, first.1)),
            Some(i) => (keys[i - 1], keys[i]),
        };
        let mut from = before.0;
        let mut at = hour;
        if from > at {
            from -= 24.0;
        }
        let span = (after.0 - from).max(1e-4);
        if at < from {
            at += 24.0;
        }
        let t = ((at - from) / span).clamp(0.0, 1.0);
        let a = before.1.to_linear();
        let b = after.1.to_linear();
        Color::LinearRgba(a.mix(&b, t))
    }
}

/// The one veil.
#[derive(Component, Clone, Copy, Debug)]
pub struct DaylightVeil;

/// Put the veil over the camera's view in the light of the hour.
#[allow(clippy::too_many_arguments)]
pub fn light_the_world(
    mut commands: Commands,
    style: Option<Res<DaylightStyle>>,
    clock: Option<Res<ambition_time::DayClock>>,
    cycle: Option<Res<ambition_time::DayCycle>>,
    rooms: Option<LiveRoomSpecs>,
    primary: Option<PrimaryLiveRoom>,
    cameras: Query<&Transform, (With<MainCamera>, Without<DaylightVeil>)>,
    mut veils: Query<(&mut Transform, &mut Sprite), With<DaylightVeil>>,
) {
    let Some(style) = style else {
        return;
    };
    let target = match (clock.as_deref(), cycle.as_deref(), rooms, primary) {
        (Some(clock), Some(cycle), Some(rooms), Some(primary)) if clock.day > 0 => {
            let sunlit = rooms.definition_named(primary.get()).is_some_and(|definition| {
                let theme = ParallaxTheme::from_room_metadata(&rooms.rooms().spec(definition).metadata);
                style.sunlit.contains(&theme)
            });
            if sunlit { style.light_at(clock.hour(cycle)) } else { Color::NONE }
        }
        _ => Color::NONE,
    };
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let at = camera.translation.truncate().extend(DAYLIGHT_VEIL_Z);
    let Ok((mut transform, mut sprite)) = veils.single_mut() else {
        commands.spawn((
            DaylightVeil,
            Sprite { color: target, custom_size: Some(Vec2::splat(VEIL_SIZE)), ..Default::default() },
            Transform::from_translation(at),
            // Not on the world layer: a portal capture draws that layer, and
            // its window, which the veil covers too, was lit two times. The
            // main cameras draw this layer and no capture does.
            bevy::camera::visibility::RenderLayers::layer(
                ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
            ),
            Name::new("Daylight veil"),
        ));
        return;
    };
    transform.translation = at;
    // The hour moves slowly, so the light needs no easing of its own; a change
    // of room is a cut behind the room transition's cover.
    sprite.color = target;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> DaylightStyle {
        DaylightStyle {
            keys: vec![
                (12.0, Color::srgba(0.0, 0.0, 0.0, 0.0)),
                (0.0, Color::srgba(0.0, 0.0, 0.5, 0.5)),
            ],
            sunlit: vec![ParallaxTheme::MathTown],
        }
    }

    #[test]
    fn the_light_is_each_key_at_its_hour_and_a_mix_between_round_the_clock() {
        let s = style();
        assert!((s.light_at(12.0).alpha() - 0.0).abs() < 1e-4);
        assert!((s.light_at(0.0).alpha() - 0.5).abs() < 1e-4);
        assert!((s.light_at(24.0).alpha() - 0.5).abs() < 1e-4);
        assert!((s.light_at(6.0).alpha() - 0.25).abs() < 1e-3, "half way from midnight to noon");
        assert!((s.light_at(18.0).alpha() - 0.25).abs() < 1e-3, "half way from noon to midnight");
    }
}
