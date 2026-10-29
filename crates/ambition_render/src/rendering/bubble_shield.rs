// ---------------------------------------------------------------------------
// Bubble shield
// ---------------------------------------------------------------------------
//
// The guard is a field around the body: a translucent interior the fighter
// shows through, with a bright rim at its edge. It is drawn in front of the
// body, so the character is visibly inside it. Behind the sprite it would
// read as a halo; a hollow ring in front reads as a hoop.
//
// The picture is authored: `bubble_shield_field.png`, of the sprite
// submodule's `bubble_shield` target. It is white, and `Sprite.color` gives
// it its hue each frame, so parry, held, and near-break states need no image
// upload. The tint and size come from resolved sim facts on
// `ShieldRingsView` (integrity, parry state, shieldstun). This layer owns no
// shield policy.
//
// On a device with a budget for shaders a second layer is drawn over the
// sprite (`crate::bubble_shield_fx`). The sprite alone is the whole look
// where there is none.

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::prelude::*;

/// The pictures of the bubble (the sprite submodule's `bubble_shield`).
const FIELD_PICTURE: &str = "sprites/bubble_shield_field.png";
const LATTICE_PICTURE: &str = "sprites/bubble_shield_lattice.png";
const CRACKS_PICTURE: &str = "sprites/bubble_shield_cracks.png";

/// The authored pictures of the bubble. Loaded once at startup.
#[derive(Resource, Clone, Default)]
pub struct BubbleShieldSprite {
    /// The field: the picture of the sprite.
    pub handle: Handle<Image>,
    /// The honeycomb and the cracks, for the shader layer.
    pub lattice: Handle<Image>,
    pub cracks: Handle<Image>,
}

/// Marker on the shield sibling sprite.
#[derive(Component)]
pub struct BubbleShieldVisual;

/// What a bubble shows now, as its sync resolved it, for a layer that draws
/// on it (`crate::bubble_shield_fx`). Presentation state: no rule reads it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BubbleShieldLook {
    /// How whole the guard is, 1 to 0.
    pub integrity: f32,
    /// The pulse of a blow it took, 1 to 0.
    pub pulse: f32,
    pub parrying: bool,
    /// How strong a layer on the bubble is: less while the guard flickers.
    pub strength: f32,
    /// How far through the beat of a blocked blow the guard is, 0 to 1.
    pub blow_phase: Option<f32>,
    /// Where on the bubble that blow landed. The bubble is a circle of
    /// radius 1 here: x toward the body's right, y toward its feet.
    pub blow_at: bevy::math::Vec2,
}

impl Default for BubbleShieldLook {
    fn default() -> Self {
        Self {
            integrity: 1.0,
            pulse: 0.0,
            parrying: false,
            strength: 0.0,
            blow_phase: None,
            blow_at: bevy::math::Vec2::ZERO,
        }
    }
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
        BubbleShieldLook::default(),
        Name::new("Bubble Shield Visual"),
    )
}

/// In FRONT of the body — the fighter is inside the bubble, not behind it.
/// Still well under the hit-flash overlay's z bias, so a struck body's white
/// silhouette is never covered by its own guard.
const BUBBLE_Z: f32 = ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05;

/// Startup system: ask for the authored pictures and keep their handles.
pub fn build_bubble_shield_sprite(sprite_content: bevy::prelude::Res<ambition_sprite_sheet::SpriteContent>, mut commands: Commands, asset_server: Res<AssetServer>) {
    use ambition_sprite_sheet::game_assets::load_sheet_image;
    commands.insert_resource(BubbleShieldSprite {
        handle: load_sheet_image(&sprite_content, &asset_server, "bubble-shield", FIELD_PICTURE),
        lattice: load_sheet_image(&sprite_content, &asset_server, "bubble-shield", LATTICE_PICTURE),
        cracks: load_sheet_image(&sprite_content, &asset_server, "bubble-shield", CRACKS_PICTURE),
    });
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
/// presentation constant: the sim publishes the shieldstun timer, and the
/// flare length is not a shield rule.
const HIT_PULSE_SECONDS: f32 = 0.12;
/// How far the field swells, and how much brighter it gets, at the pulse's peak.
const HIT_PULSE_SWELL: f32 = 0.13;
const HIT_PULSE_FLARE: f32 = 0.45;

/// Below this integrity the guard flickers to show danger. A colour ramp
/// alone reads slowly in a busy match; the fighter must drop the shield
/// before it breaks.
const DANGER_INTEGRITY: f32 = 0.34;
/// The danger flicker's period in sim ticks and its depth at zero integrity.
/// Sim-derived, like the intangibility blink, so the flicker is one rate at
/// any refresh rate and a capture shows what the screen showed.
const DANGER_PERIOD_TICKS: u64 = 8;
const DANGER_FLICKER_DEPTH: f32 = 0.45;

/// How hard the guard is flickering right now: `0.0` while it is healthy,
/// rising as it approaches the break and oscillating on the sim clock.
///
/// Separate from the colour ramp: the ramp says how spent the guard is, this
/// says how urgent that is.
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
/// `pulse` flares the whole field on the beat a hit is absorbed, to separate
/// "my guard took that" from "that missed me". `flicker` is the near-break
/// read, and it dims instead of brightening, so a failing guard looks like
/// it is failing.
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
/// toward the body, as a platform fighter player expects.
fn shield_bubble_coverage(integrity: f32) -> f32 {
    0.55 + 0.45 * integrity.clamp(0.0, 1.0)
}

/// The hit pulse strength: `1.0` on the frame a hit lands, `0.0` once the
/// flare is spent. It normalizes the published shieldstun timer against a
/// presentation constant, like `hit_flash_secs` / `normalize_hit_flash` for
/// the body.
///
/// Shieldstun outlasts the flare on a heavy hit, so this saturates: the guard
/// flashes once per hit, not in proportion to the stun.
fn shield_hit_pulse(stun_secs: f32) -> f32 {
    (stun_secs / HIT_PULSE_SECONDS).clamp(0.0, 1.0)
}

/// The part of the beat of a blocked blow in which the bubble flares.
const BLOW_FLARE_PART: f32 = 0.4;
/// How far from the middle of the bubble the point of a blow is drawn, as a
/// part of its radius. Under 1: at the rim the bubble turns away from the
/// viewer, and the point would not show.
const BLOW_POINT_RADIUS: f32 = 0.8;

/// The flare of a blocked blow: `1.0` when it lands, `0.0` after
/// [`BLOW_FLARE_PART`] of its beat. The ring of the blow runs the whole beat.
fn shield_blow_flare(blow_phase: Option<f32>) -> f32 {
    blow_phase.map_or(0.0, |phase| (1.0 - phase / BLOW_FLARE_PART).clamp(0.0, 1.0))
}

/// Where on the bubble a blow shows, with the bubble as a circle of radius 1
/// (x toward the body's right, y toward its feet). The blow can land at any
/// distance from the body, so only its direction is used. A blow at the
/// middle shows at the middle.
fn shield_blow_point(
    blow_from: ambition_platformer2d_core::Vec2,
    gravity_dir: ambition_platformer2d_core::Vec2,
    bubble_size: bevy::math::Vec2,
) -> bevy::math::Vec2 {
    let right = ambition_platformer2d_core::Vec2::new(gravity_dir.y, -gravity_dir.x);
    let on_bubble = bevy::math::Vec2::new(
        blow_from.dot(right) / (bubble_size.x * 0.5).max(1.0),
        blow_from.dot(gravity_dir) / (bubble_size.y * 0.5).max(1.0),
    );
    on_bubble.try_normalize().map_or(bevy::math::Vec2::ZERO, |dir| dir * BLOW_POINT_RADIUS)
}

/// Show, hide, and tint a bubble around every body whose shield is up (the
/// player and any brain-controlled actor). One pooled bubble per active
/// shielder; unused sprites hide, and the pool grows on demand. Scale follows
/// each body's size.
/// Run condition of [`sync_bubble_shield_visual`]: a shield is raised, or a
/// pooled bubble is still shown. With neither, each bubble is already hidden
/// and the pass would only hide it again.
pub fn a_shield_is_raised_or_shown(
    active: Res<ambition_sim_view::ShieldRingsView>,
    bubbles: Query<&Visibility, With<BubbleShieldVisual>>,
) -> bool {
    !active.0.is_empty() || bubbles.iter().any(|visibility| *visibility != Visibility::Hidden)
}

pub fn sync_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    // Each drawable is placed by the geometry of its own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    // Every raised shield, resolved sim-side into the pooled read model.
    active: Res<ambition_sim_view::ShieldRingsView>,
    // The danger flicker's phase. Sim-derived: see `DANGER_PERIOD_TICKS`.
    tick: Res<ambition_time::SimTick>,
    mut bubbles: Query<
        (
            Entity,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
            &mut BubbleShieldLook,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        With<BubbleShieldVisual>,
    >,
) {
    // A guard whose live room cannot be told is not drawn: there is no
    // geometry to place it by.
    let active: Vec<_> = active
        .0
        .iter()
        .filter_map(|guard| Some((*guard, guard.room?, rooms.in_room(guard.room?)?)))
        .collect();
    let pool_size = bubbles.iter().count();
    let mut assigned = 0usize;
    for (bubble, mut transform, mut sprite, mut vis, mut look, stamp) in &mut bubbles {
        if let Some((guard, room, world)) = active.get(assigned).copied() {
            // A pooled bubble draws a different guard from frame to frame, so it
            // takes the room of the guard it draws now.
            if stamp.map(|stamp| stamp.0) != Some(room) {
                commands
                    .entity(bubble)
                    .try_insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(room));
            }
            transform.translation =
                ambition_platformer2d_core::config::world_to_bevy(&world.0, guard.pos, BUBBLE_Z);
            // The field is an ellipse that belongs to the body, so it rotates with
            // the body's frame. Otherwise a wall-walker's guard lies on its side.
            transform.rotation = Quat::from_rotation_z(
                ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle(
                    guard.gravity_dir,
                ),
            );
            let pulse = shield_hit_pulse(guard.stun_secs).max(shield_blow_flare(guard.blow_phase));
            // Generous overlap: the body is inside this, so the field must clear
            // the silhouette.
            let extent = shield_bubble_coverage(guard.integrity) * (1.0 + HIT_PULSE_SWELL * pulse);
            let bubble_size =
                bevy::math::Vec2::new(guard.size.x * 1.70 * extent, guard.size.y * 1.35 * extent);
            sprite.custom_size = Some(bubble_size);
            let flicker = shield_danger_flicker(guard.integrity, tick.0);
            sprite.color = shield_bubble_color(guard.parrying, guard.integrity, pulse, flicker);
            let shown = BubbleShieldLook {
                integrity: guard.integrity.clamp(0.0, 1.0),
                pulse,
                parrying: guard.parrying,
                strength: 1.0 - DANGER_FLICKER_DEPTH * flicker.clamp(0.0, 1.0),
                blow_phase: guard.blow_phase,
                blow_at: shield_blow_point(guard.blow_from, guard.gravity_dir, bubble_size),
            };
            if *look != shown {
                *look = shown;
            }
            vis.set_if_neq(Visibility::Visible);
            assigned += 1;
        } else {
            vis.set_if_neq(Visibility::Hidden);
        }
    }

    // More shielders than pool sprites: grow the pool (new ones are placed
    // next frame). The common 0-1 shielder case stays at one sprite.
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
    /// Each pooled bubble draws its guard in the guard's own live room (view
    /// half, cut V2g). Two live rooms of different sizes, a raised guard in
    /// each at one simulation position: each bubble is placed by its guard's
    /// room and carries that room's stamp, so V3's band draws it only in the
    /// views that frame that room. A guard whose room cannot be told is not
    /// drawn.
    #[test]
    fn each_bubble_is_drawn_in_its_guard_s_own_live_room() {
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        const AT: ae::Vec2 = ae::Vec2::new(100.0, 200.0);
        let world_of = |size: ae::Vec2| ae::World::new("shield room", size, ae::Vec2::new(40.0, 40.0), Vec::new());
        let (big, small) = (ae::Vec2::new(800.0, 600.0), ae::Vec2::new(400.0, 300.0));
        let guard = |room| ambition_sim_view::ShieldRingFact {
            pos: AT,
            size: ae::Vec2::new(20.0, 40.0),
            parrying: false,
            integrity: 1.0,
            stun_secs: 0.0,
            blow_phase: None,
            blow_from: ae::Vec2::ZERO,
            gravity_dir: ae::Vec2::new(0.0, 1.0),
            room,
        };
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), ae::RoomGeometry(world_of(big)));
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, ae::RoomGeometry(world_of(small)));
        app.init_resource::<ambition_time::SimTick>();
        app.insert_resource(ambition_sim_view::ShieldRingsView(vec![
            guard(Some(LiveRoomInstance::ACTIVATION)),
            guard(Some(second)),
            guard(None),
        ]));
        for _ in 0..3 {
            app.world_mut().spawn(new_bubble_sprite(Handle::default()));
        }
        app.add_systems(Update, sync_bubble_shield_visual);
        app.update();
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&Transform, &Visibility, Option<&InRoomInstance>)>();
        let mut drawn: Vec<(u32, (i32, i32))> = q
            .iter(world)
            .filter(|(_, visibility, _)| **visibility == Visibility::Visible)
            .map(|(transform, _, stamp)| {
                (
                    stamp.map_or(u32::MAX, |stamp| stamp.0.ordinal()),
                    (transform.translation.x as i32, transform.translation.y as i32),
                )
            })
            .collect();
        drawn.sort();
        let flipped = |size: ae::Vec2| ((AT.x - size.x * 0.5) as i32, (size.y * 0.5 - AT.y) as i32);
        assert_eq!(
            drawn,
            vec![(LiveRoomInstance::ACTIVATION.ordinal(), flipped(big)), (second.ordinal(), flipped(small))],
            "(room, position) of each drawn bubble: each must be placed by its guard's live room and stamped with it"
        );
    }

    use super::*;

    /// A spent guard covers less of the body. Monotone, because "smaller means
    /// weaker" is the read.
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

    /// A blocked hit flares the guard once, and the flare ends quickly. It must
    /// not last as long as the shieldstun.
    #[test]
    fn a_blocked_hit_flares_the_guard_and_the_flare_is_spent_before_the_stun() {
        assert_eq!(shield_hit_pulse(0.0), 0.0, "an unhit guard does not flare");
        // A blow shows on the side of the bubble it came from, for a body
        // that stands on a wall too (its feet toward +x: a blow from under
        // it in the world is at its right).
        use ambition_platformer2d_core as ae;
        let size = bevy::math::Vec2::new(40.0, 60.0);
        let from_the_left = shield_blow_point(ae::Vec2::new(-50.0, 0.0), ae::Vec2::new(0.0, 1.0), size);
        assert_eq!(from_the_left, bevy::math::Vec2::new(-BLOW_POINT_RADIUS, 0.0));
        let on_a_wall = shield_blow_point(ae::Vec2::new(0.0, 50.0), ae::Vec2::new(1.0, 0.0), size);
        assert!(on_a_wall.x < -0.7 && on_a_wall.y.abs() < 1e-4, "{on_a_wall:?}");
        assert_eq!(shield_blow_point(ae::Vec2::ZERO, ae::Vec2::new(0.0, 1.0), size), bevy::math::Vec2::ZERO);
        assert_eq!(shield_blow_flare(Some(0.0)), 1.0);
        assert_eq!(shield_blow_flare(Some(BLOW_FLARE_PART)), 0.0);
        assert_eq!(shield_blow_flare(None), 0.0);
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

    /// A healthy guard never flickers; one about to break does, and the flicker
    /// dims so a failing shield looks like it is failing.
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

    /// The field is oriented to the body, not the screen. Ordinary gravity
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
