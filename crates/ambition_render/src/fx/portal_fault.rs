//! The Floating-Point Exception a portal shows when it refuses an operation
//! for a numeric reason: tiny letters spelling IEEE 754 and the fault's
//! symbols, breathed out of the mouth that refused.
//!
//! The simulation writes the refusal every tick it stands, and a rollback
//! writes it again; one standing refusal is one operation, so it is drawn
//! once. A refusal that ends and comes back is a new operation.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomOf, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_vfx::FaultGlyph;
use bevy::prelude::*;

/// A refusal not written for this long has ended; written again, it is a new
/// operation. Longer than a rollback's re-emission, shorter than a player can
/// let go and push again.
const CONTINUING_GAP_SECONDS: f32 = 0.25;
/// Refusals remembered at once; beyond this the oldest is forgotten.
const EPISODES_HELD: usize = 64;
const GLYPH_SECONDS: f32 = 0.9;
const HEADER_SECONDS: f32 = 1.15;
const GLYPH_PX: f32 = 7.0;
const HEADER_PX: f32 = 8.0;

/// The refusals being drawn, by episode, and when each was last written.
#[derive(Default)]
pub struct FaultEpisodes(Vec<(u64, f32)>);

impl FaultEpisodes {
    /// Whether `episode`, written at `now`, starts a new operation.
    pub fn starts(&mut self, episode: u64, now: f32) -> bool {
        self.0.retain(|&(_, seen)| now - seen <= CONTINUING_GAP_SECONDS && seen <= now);
        if let Some(entry) = self.0.iter_mut().find(|(key, _)| *key == episode) {
            entry.1 = now;
            return false;
        }
        if self.0.len() >= EPISODES_HELD {
            self.0.remove(0);
        }
        self.0.push((episode, now));
        true
    }
}

/// One letter group drifting off a refusing mouth.
#[derive(Component)]
pub struct PortalFaultGlyphVisual {
    pos: ae::Vec2,
    vel: ae::Vec2,
    age: f32,
    lifetime: f32,
    rgb: [f32; 3],
}

/// The header and symbols one fault shows. The header names the standard
/// except for a policy refusal, which is not an IEEE fault.
fn words(glyph: FaultGlyph, seed: u64) -> (&'static str, Vec<String>) {
    let fragment = |shift: u32| format!("{:04X}", (seed >> shift) & 0xFFFF);
    match glyph {
        FaultGlyph::NotANumber => ("IEEE 754", vec!["NaN".into(), "NaN".into(), "7FF8".into()]),
        FaultGlyph::Infinity => ("IEEE 754", vec!["+Inf".into(), "-Inf".into(), "7FF0".into()]),
        FaultGlyph::SignedZero => ("IEEE 754", vec!["+0".into(), "-0".into(), "1p-1074".into()]),
        FaultGlyph::BitPattern => ("IEEE 754", vec![fragment(0), fragment(16), fragment(32), fragment(48)]),
        FaultGlyph::ExponentBias => ("EXPONENT BIAS", vec!["1023".into(), "2^e".into()]),
    }
}

fn colour(glyph: FaultGlyph) -> [f32; 3] {
    match glyph {
        // A policy refusal is amber: the arithmetic was fine.
        FaultGlyph::ExponentBias => [1.0, 0.78, 0.35],
        _ => [0.62, 0.95, 1.0],
    }
}

/// A uniform draw in [0, 1) from a splitmix64 stream, so a replayed fault
/// lays its letters out the same way.
fn draw(state: &mut u64) -> f32 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// Draw one fault at a mouth: the header over its middle and each symbol at
/// its own place along the face, all drifting out along the normal.
pub fn spawn_portal_fault(
    commands: &mut Commands,
    session_scope: Option<SessionSpawnScope>,
    world: &ae::World,
    font: &TextFont,
    pos: ae::Vec2,
    normal: ae::Vec2,
    half_length: f32,
    glyph: FaultGlyph,
    episode: u64,
) {
    let Some(session_scope) = session_scope else {
        return;
    };
    let normal = normal.try_normalize().unwrap_or(ae::Vec2::Y);
    let tangent = ae::Vec2::new(-normal.y, normal.x);
    let reach = half_length.max(4.0) * 0.8;
    let rgb = colour(glyph);
    let (header, symbols) = words(glyph, episode);
    let mut stream = episode;
    let spawn = |commands: &mut Commands, text: String, at: ae::Vec2, vel: ae::Vec2, lifetime: f32, px: f32| {
        commands.spawn_session_scoped(
            session_scope,
            (
                Text2d::new(text.clone()),
                TextFont { font_size: FontSize::Px(px), ..font.clone() },
                TextColor(Color::srgba(rgb[0], rgb[1], rgb[2], 0.0)),
                Transform::from_translation(world_to_bevy(world, at, WORLD_Z_FX + 2.0)),
                PortalFaultGlyphVisual { pos: at, vel, age: 0.0, lifetime, rgb },
                Name::new(format!("Portal fault: {text}")),
            ),
        );
    };
    spawn(commands, header.to_string(), pos + normal * 6.0, normal * 14.0, HEADER_SECONDS, HEADER_PX);
    for symbol in symbols {
        let along = (draw(&mut stream) * 2.0 - 1.0) * reach;
        let out = 18.0 + 16.0 * draw(&mut stream);
        let drift = (draw(&mut stream) * 2.0 - 1.0) * 6.0;
        let at = pos + tangent * along + normal * 3.0;
        spawn(commands, symbol, at, normal * out + tangent * drift, GLYPH_SECONDS, GLYPH_PX);
    }
}

/// Drift, fade and retire each fault's letters.
pub fn update_portal_fault_glyphs(
    mut commands: Commands,
    time: Res<Time>,
    world: LiveRoomOf<ae::RoomGeometry>,
    mut query: Query<(Entity, &mut PortalFaultGlyphVisual, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut glyph, mut transform, mut colour) in &mut query {
        glyph.age += dt;
        if glyph.age >= glyph.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let velocity = glyph.vel;
        glyph.pos += velocity * dt;
        glyph.vel *= (1.0 - 2.5 * dt).clamp(0.0, 1.0);
        let t = glyph.age / glyph.lifetime;
        // In over the first tenth, out over the rest.
        let alpha = (t * 10.0).min(1.0) * (1.0 - t);
        colour.0 = Color::srgba(glyph.rgb[0], glyph.rgb[1], glyph.rgb[2], alpha);
        if let Some(world) = world.of(entity) {
            transform.translation = world_to_bevy(&world.0, glyph.pos, WORLD_Z_FX + 2.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A refusal written every tick is one operation; one that stops and is
    /// written again later is two.
    #[test]
    fn a_standing_refusal_is_one_operation() {
        let mut episodes = FaultEpisodes::default();
        let starts: Vec<bool> = [0.0, 1.0 / 60.0, 2.0 / 60.0, 0.1, 0.2]
            .into_iter()
            .map(|now| episodes.starts(7, now))
            .collect();
        assert_eq!(starts, [true, false, false, false, false]);
        assert!(episodes.starts(7, 0.2 + CONTINUING_GAP_SECONDS + 0.05), "the refusal ended and came back");
        assert!(episodes.starts(8, 0.6), "another operation starts on its own");
    }
}
