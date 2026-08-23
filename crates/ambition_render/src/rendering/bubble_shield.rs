// ---------------------------------------------------------------------------
// Bubble shield (procedural)
// ---------------------------------------------------------------------------
//
// The guard is a soft filled FIELD around the body: a translucent interior the
// fighter reads through, gathering into a bright rim at its edge. It is drawn
// in FRONT of the body, because a bubble a platform fighter recognises is one
// the character is visibly inside — a field drawn behind the sprite is a
// halo, and a hollow ring in front is a hoop.
//
// The texture is one white field generated at startup and tinted by
// `Sprite.color` each frame, so the parry / held / near-break reads cost no
// image upload. Everything the tint and the size are derived from is a
// resolved simulation fact on `ShieldRingsView` — integrity, parry state and
// shieldstun. This layer owns no shield policy: it does not know what spends
// the guard, what breaks it, or how long the dizzy lasts.

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Procedural field texture. Built once at startup.
#[derive(Resource, Clone, Default)]
pub struct BubbleShieldSprite {
    pub handle: Handle<Image>,
}

/// Marker on the shield sibling sprite.
#[derive(Component)]
pub struct BubbleShieldVisual;

/// 128 rather than 64: the field is mostly a smooth gradient now, and a
/// gradient stretched from 64px over a fighter shows its steps.
const SHIELD_TEXTURE_SIZE: u32 = 128;

/// Where the field ends, as a fraction of the texture's half-width.
const BUBBLE_EDGE: f32 = 0.94;
/// Where the rim is brightest, and how tightly it is gathered there.
const RIM_CENTRE: f32 = 0.87;
const RIM_SPREAD: f32 = 0.075;
/// Peak alpha the rim contributes on top of the interior.
const RIM_GAIN: f32 = 1.0;
/// The interior's alpha at the very centre, and at the rim's inner shoulder.
/// The fighter has to stay readable THROUGH this.
const INTERIOR_ALPHA_CENTRE: f32 = 0.32;
const INTERIOR_ALPHA_EDGE: f32 = 0.58;

/// Generate the filled field: translucent inside, bright at the rim,
/// transparent outside. White, so `Sprite.color` supplies the hue and the
/// global alpha.
pub fn build_bubble_shield_image() -> Image {
    let size = SHIELD_TEXTURE_SIZE;
    let centre = (size as f32 - 1.0) * 0.5;
    // One texel expressed in the normalized radius the profile is written in,
    // so the outer edge gets the same ~2px anti-alias band at any resolution.
    let texel = 1.0 / centre.max(1.0);

    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - centre;
            let dy = y as f32 - centre;
            // Normalized radius: 1.0 is the texture's edge.
            let r = (dx * dx + dy * dy).sqrt() / centre.max(1.0);
            let alpha = field_alpha(r, texel * 2.0);
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The field's alpha profile at normalized radius `r`, anti-aliased over `aa`.
///
/// Written as a pure function of the radius so the shape is testable without
/// an image: a filled interior that gathers outward, plus a gaussian rim, cut
/// off at [`BUBBLE_EDGE`].
fn field_alpha(r: f32, aa: f32) -> f32 {
    let outer_fade = ((BUBBLE_EDGE - r) / aa.max(f32::EPSILON)).clamp(0.0, 1.0);
    if outer_fade <= 0.0 {
        return 0.0;
    }
    let toward_edge = (r / BUBBLE_EDGE).clamp(0.0, 1.0);
    let interior =
        INTERIOR_ALPHA_CENTRE + (INTERIOR_ALPHA_EDGE - INTERIOR_ALPHA_CENTRE) * toward_edge;
    let from_rim = (r - RIM_CENTRE) / RIM_SPREAD;
    let rim = RIM_GAIN * (-(from_rim * from_rim)).exp();
    outer_fade * (interior + rim).clamp(0.0, 1.0)
}

/// One pooled bubble sprite (hidden until `sync` assigns it to a shielder).
fn new_bubble_sprite(handle: Handle<Image>) -> impl Bundle {
    (
        Sprite {
            image: handle,
            custom_size: Some(bevy::math::Vec2::new(48.0, 64.0)),
            color: Color::srgba(0.5, 0.8, 1.0, 0.0),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, BUBBLE_Z),
        Visibility::Hidden,
        BubbleShieldVisual,
        Name::new("Bubble Shield Visual"),
    )
}

/// In FRONT of the body — the fighter is inside the bubble, not behind it.
/// Still well under the hit-flash overlay's z bias, so a struck body's white
/// silhouette is never covered by its own guard.
const BUBBLE_Z: f32 = ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05;

/// Startup system: build the procedural field image and stash its handle.
pub fn build_bubble_shield_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_bubble_shield_image());
    commands.insert_resource(BubbleShieldSprite { handle });
}

/// Seed the pool with one bubble. `sync` grows it on demand when several
/// bodies shield at once.
pub fn spawn_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(), With<BubbleShieldVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(sprite) = sprite else { return };
    if sprite.handle == Handle::default() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    commands.spawn_session_scoped(session_scope, new_bubble_sprite(sprite.handle.clone()));
}

/// How long a blocked hit stays visible on the guard, in seconds. A
/// presentation constant: the SIM publishes the shieldstun timer, and how long
/// a flare should last is not the shield rules' business.
const HIT_PULSE_SECONDS: f32 = 0.12;
/// How far the field swells, and how much brighter it gets, at the pulse's peak.
const HIT_PULSE_SWELL: f32 = 0.13;
const HIT_PULSE_FLARE: f32 = 0.45;

/// Below this integrity the guard is in DANGER and says so by flickering. A
/// colour ramp alone is a slow read in a busy match: the fighter needs to know
/// to drop the shield before the break, not after it.
const DANGER_INTEGRITY: f32 = 0.34;
/// The danger flicker's period in sim ticks and its depth at zero integrity.
/// Sim-derived, like the intangibility blink, so the flicker is one rate at
/// any refresh rate and a capture shows what the screen showed.
const DANGER_PERIOD_TICKS: u64 = 8;
const DANGER_FLICKER_DEPTH: f32 = 0.45;

/// How hard the guard is flickering right now: `0.0` while it is healthy,
/// rising as it approaches the break and oscillating on the sim clock.
///
/// Separate from the colour ramp rather than folded into it, because the two
/// answer different questions — the ramp says how spent the guard is, this says
/// how urgent that has become.
fn shield_danger_flicker(integrity: f32, tick: u64) -> f32 {
    let urgency =
        ((DANGER_INTEGRITY - integrity.clamp(0.0, 1.0)) / DANGER_INTEGRITY).clamp(0.0, 1.0);
    if urgency <= 0.0 {
        return 0.0;
    }
    let phase = (tick % DANGER_PERIOD_TICKS) as f32 / DANGER_PERIOD_TICKS as f32;
    // Triangle, like the body's intangibility blink: a square wave at this rate
    // reads as a dropped frame.
    urgency * (1.0 - (2.0 * phase - 1.0).abs())
}

/// Parry window: gold glow. Held but expired: cyan that reddens as the guard is
/// spent, so "this shield is about to break" is readable without a meter.
///
/// `pulse` flares the whole field on the beat a hit is absorbed — the read that
/// separates "my guard took that" from "that missed me". `flicker` is the
/// near-break danger read, and it dims rather than brightens: a guard about to
/// shatter should look like it is failing.
///
/// A parry ignores the flicker. It lasts a handful of frames and is the one
/// beat that must never be mistaken for anything else.
fn shield_bubble_color(parrying: bool, integrity: f32, pulse: f32, flicker: f32) -> Color {
    let flare = 1.0 + HIT_PULSE_FLARE * pulse.clamp(0.0, 1.0);
    if parrying {
        return Color::srgba(1.0, 0.95, 0.40, (0.80 * flare).min(1.0));
    }
    let spent = 1.0 - integrity.clamp(0.0, 1.0);
    let fade = 1.0 - DANGER_FLICKER_DEPTH * flicker.clamp(0.0, 1.0);
    Color::srgba(
        0.50 + 0.50 * spent,
        0.80 - 0.55 * spent,
        1.0 - 0.75 * spent,
        (0.62 * flare * fade).min(1.0),
    )
}

/// How much of the body the guard still covers. A spent shield shrinks
/// toward the body, which is the read a platform fighter expects and the shape a
/// later poke rule measures against.
fn shield_bubble_coverage(integrity: f32) -> f32 {
    0.55 + 0.45 * integrity.clamp(0.0, 1.0)
}

/// The hit pulse's strength, `1.0` on the frame a hit lands and `0.0` once the
/// flare is spent. Reads the published shieldstun timer and normalizes it
/// against a presentation constant — the same split
/// `hit_flash_secs` / `normalize_hit_flash` uses for the body.
///
/// Shieldstun outlasts the flare on a heavy hit, which is why this saturates
/// rather than scaling: the guard flashes once per hit, not proportionally to
/// how stunned it is.
fn shield_hit_pulse(stun_secs: f32) -> f32 {
    (stun_secs / HIT_PULSE_SECONDS).clamp(0.0, 1.0)
}

/// Show / hide + tint a bubble around EVERY body whose shield is up — the player AND any
/// brain-controlled actor (the duel fighters). One pooled bubble per active shielder; unused
/// sprites hide, and the pool grows on demand. Scale tracks each body's size.
pub fn sync_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    // Every raised shield, resolved sim-side into the pooled read-model
    // (E4): render positions bubbles, it no longer queries the live clusters.
    active: Res<ambition_sim_view::ShieldRingsView>,
    // The danger flicker's phase. Sim-derived: see `DANGER_PERIOD_TICKS`.
    tick: Res<ambition_time::SimTick>,
    mut bubbles: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<BubbleShieldVisual>>,
) {
    let active = &active.0;
    let pool_size = bubbles.iter().count();
    let mut assigned = 0usize;
    for (mut transform, mut sprite, mut vis) in &mut bubbles {
        if let Some(guard) = active.get(assigned).copied() {
            transform.translation =
                ambition_platformer2d_core::config::world_to_bevy(&world.0, guard.pos, BUBBLE_Z);
            // The field is an ELLIPSE and the ellipse belongs to the body, so
            // it rotates with the body's own frame. Without this a wall-walker's
            // guard lies on its side — the same screen-axis assumption the
            // sprite pass already refuses through this exact helper.
            transform.rotation = Quat::from_rotation_z(
                ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle(
                    guard.gravity_dir,
                ),
            );
            let pulse = shield_hit_pulse(guard.stun_secs);
            // Generous overlap: the body is INSIDE this, so the field has to
            // clear the silhouette rather than trace it.
            let extent = shield_bubble_coverage(guard.integrity) * (1.0 + HIT_PULSE_SWELL * pulse);
            sprite.custom_size = Some(bevy::math::Vec2::new(
                guard.size.x * 1.70 * extent,
                guard.size.y * 1.35 * extent,
            ));
            sprite.color = shield_bubble_color(
                guard.parrying,
                guard.integrity,
                pulse,
                shield_danger_flicker(guard.integrity, tick.0),
            );
            *vis = Visibility::Visible;
            assigned += 1;
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // More bodies shielding than sprites in the pool → grow it (the new ones get
    // positioned next frame). Spawn-on-demand keeps the common 0-1 shielder case at
    // a single sprite.
    if active.len() > pool_size {
        if let Some(sprite) = sprite {
            if sprite.handle != Handle::default() {
                let Some(session_scope) =
                    SessionSpawnScope::for_optional_active_session(active_session.as_deref())
                else {
                    return;
                };
                for _ in pool_size..active.len() {
                    commands.spawn_session_scoped(
                        session_scope,
                        new_bubble_sprite(sprite.handle.clone()),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(img: &Image, x: usize, y: usize) -> u8 {
        let size = SHIELD_TEXTURE_SIZE as usize;
        img.data.as_ref().expect("image data")[(y * size + x) * 4 + 3]
    }

    #[test]
    fn build_bubble_shield_image_is_correct_size() {
        let img = build_bubble_shield_image();
        assert_eq!(img.width(), SHIELD_TEXTURE_SIZE);
        assert_eq!(img.height(), SHIELD_TEXTURE_SIZE);
    }

    /// THE SLICE: the guard is a filled field, not a hoop. The centre used to
    /// be a hole — a fighter behind it was standing in a doorway rather than
    /// inside a bubble.
    #[test]
    fn the_field_is_filled_and_the_body_reads_through_it() {
        let img = build_bubble_shield_image();
        let mid = SHIELD_TEXTURE_SIZE as usize / 2;
        let centre = alpha_at(&img, mid, mid);
        assert!(centre > 40, "the interior must be present, got {centre}");
        assert!(
            centre < 130,
            "the interior must stay translucent enough to read the fighter \
             through, got {centre}"
        );
    }

    /// The rim is the brightest part of the field, and it is a band rather
    /// than the whole disc.
    #[test]
    fn the_rim_is_the_brightest_band() {
        let img = build_bubble_shield_image();
        let mid = SHIELD_TEXTURE_SIZE as usize / 2;
        let half = (SHIELD_TEXTURE_SIZE as f32 - 1.0) * 0.5;
        let sample = |r: f32| alpha_at(&img, mid + (half * r) as usize, mid);

        let centre = sample(0.0);
        let rim = sample(RIM_CENTRE);
        let outside = sample(1.0);
        assert!(rim > centre, "rim {rim} must outshine interior {centre}");
        assert!(rim > 200, "the rim must read as an edge, got {rim}");
        assert_eq!(outside, 0, "outside the field is transparent");
    }

    /// A spent guard covers less of the body. Monotone, because "smaller means
    /// weaker" is the read, and a non-monotone curve would lie halfway.
    #[test]
    fn coverage_shrinks_monotonically_as_the_guard_is_spent() {
        let whole = shield_bubble_coverage(1.0);
        let half = shield_bubble_coverage(0.5);
        let spent = shield_bubble_coverage(0.0);
        assert!(whole > half && half > spent, "{whole} {half} {spent}");
        assert!(spent > 0.0, "a guard about to break still draws something");
        // Out-of-range integrity cannot invert the read.
        assert_eq!(shield_bubble_coverage(4.0), whole);
        assert_eq!(shield_bubble_coverage(-4.0), spent);
    }

    /// A whole guard is cool, a spent one is red, and a parry is neither.
    #[test]
    fn the_field_reddens_toward_a_break_and_a_parry_is_gold() {
        let whole = shield_bubble_color(false, 1.0, 0.0, 0.0).to_srgba();
        let breaking = shield_bubble_color(false, 0.0, 0.0, 0.0).to_srgba();
        assert!(breaking.red > whole.red, "a spent guard reddens");
        assert!(breaking.blue < whole.blue);

        let parry = shield_bubble_color(true, 1.0, 0.0, 0.0).to_srgba();
        assert!(
            parry.red > whole.red && parry.green > whole.green && parry.blue < whole.blue,
            "the parry window is its own read, not a brighter held shield"
        );
    }

    /// A blocked hit flares the guard once and the flare is over quickly —
    /// it must not simply mirror however long the shieldstun runs.
    #[test]
    fn a_blocked_hit_flares_the_guard_and_the_flare_is_spent_before_the_stun() {
        assert_eq!(shield_hit_pulse(0.0), 0.0, "an unhit guard does not flare");
        assert_eq!(shield_hit_pulse(HIT_PULSE_SECONDS), 1.0);
        // A heavy hit stuns far longer than the flare lasts; the guard flashes
        // once per hit rather than staying lit for the whole stun.
        assert_eq!(shield_hit_pulse(HIT_PULSE_SECONDS * 20.0), 1.0);
        let decaying = shield_hit_pulse(HIT_PULSE_SECONDS * 0.5);
        assert!(decaying > 0.0 && decaying < 1.0, "{decaying}");

        // And the flare is visible: brighter, at both reads.
        for parrying in [false, true] {
            let calm = shield_bubble_color(parrying, 1.0, 0.0, 0.0).to_srgba();
            let struck = shield_bubble_color(parrying, 1.0, 1.0, 0.0).to_srgba();
            assert!(struck.alpha > calm.alpha, "parrying={parrying}");
            assert!(struck.alpha <= 1.0);
        }
    }

    /// A healthy guard never flickers; one about to break does, and the
    /// flicker dims rather than brightens so a failing shield looks like it is
    /// failing.
    #[test]
    fn a_guard_near_breaking_flickers_and_a_healthy_one_never_does() {
        for tick in 0..DANGER_PERIOD_TICKS * 3 {
            assert_eq!(shield_danger_flicker(1.0, tick), 0.0, "whole guard");
            assert_eq!(
                shield_danger_flicker(DANGER_INTEGRITY, tick),
                0.0,
                "the threshold itself is not yet danger"
            );
        }
        let mut peak: f32 = 0.0;
        let mut trough = f32::MAX;
        for tick in 0..DANGER_PERIOD_TICKS {
            let f = shield_danger_flicker(0.0, tick);
            peak = peak.max(f);
            trough = trough.min(f);
        }
        assert!(peak > trough, "a flicker that never varies is a tint");
        assert!(peak <= 1.0);

        // Urgency rises as the guard is spent.
        let mid = DANGER_PERIOD_TICKS / 2;
        assert!(
            shield_danger_flicker(0.0, mid) > shield_danger_flicker(DANGER_INTEGRITY * 0.5, mid)
        );

        // The field dims at the flicker's peak, and never disappears.
        let calm = shield_bubble_color(false, 0.05, 0.0, 0.0).to_srgba();
        let flickering = shield_bubble_color(false, 0.05, 0.0, 1.0).to_srgba();
        assert!(flickering.alpha < calm.alpha, "danger DIMS the guard");
        assert!(flickering.alpha > 0.0, "a guard still up is still drawn");
    }

    /// The field is oriented to the BODY, not the screen. Ordinary gravity
    /// leaves it upright; a wall-walker's guard turns with it.
    #[test]
    fn the_field_turns_with_the_body_not_the_screen() {
        use ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle;
        // Engine coords are y-down, so ordinary gravity points +Y.
        let ordinary = gravity_upright_angle(bevy::math::Vec2::new(0.0, 1.0));
        assert!(ordinary.abs() < 1e-6, "ordinary gravity draws upright");

        // Every other gravity turns the field, and a half turn is a half turn.
        let sideways = gravity_upright_angle(bevy::math::Vec2::new(1.0, 0.0));
        let flipped = gravity_upright_angle(bevy::math::Vec2::new(0.0, -1.0));
        assert!(
            sideways.abs() > 1e-3,
            "a wall-walker's guard is not upright"
        );
        assert!(
            (flipped.abs() - std::f32::consts::PI).abs() < 1e-3,
            "flipped gravity is a half turn, got {flipped}"
        );
    }

    /// The parry read is never diluted by anything else on the field.
    #[test]
    fn a_parry_ignores_the_danger_flicker() {
        let calm = shield_bubble_color(true, 0.02, 0.0, 0.0);
        let flickering = shield_bubble_color(true, 0.02, 0.0, 1.0);
        assert_eq!(calm, flickering);
    }
}
