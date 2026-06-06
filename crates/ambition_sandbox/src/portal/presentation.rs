//! Portal presentation (visible build only — registered by the presentation
//! plugin): portal quads + labels, the held / pickup gun sprite, the body-piece
//! decomposition mid-transit, the disorientation indicator, and the gravity
//! zone / switch visuals.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::physics::{GravityField, GravityZone};
use crate::platformer_runtime::orientation::ActorRoll;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::portal_pieces as pp;
use crate::GameWorld;

use super::color::PortalColor;
use super::gun::PortalGun;
use super::lifecycle::GravityFlipSwitch;
use super::pickup::PortalGunPickup;
use super::placement::{pick_aim, portal_facing_flips, somersault_roll};
use super::shot::PortalProjectile;
use super::transit::{PortalInputWarp, PortalTransit};
use super::types::{find_portal, Portal, PORTAL_VISUAL_THICKNESS};

/// Marks the visual for a [`GravityZone`].
#[derive(Component)]
pub struct GravityZoneVisual;

/// Draw each gravity zone as a translucent tinted region so the player can see
/// where gravity changes (violet = up, teal = down/other).
pub fn sync_gravity_zone_visual(
    mut commands: Commands,
    world: Res<GameWorld>,
    visuals: Query<Entity, With<GravityZoneVisual>>,
    zones: Query<&GravityZone>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for zone in &zones {
        let color = if zone.dir.y < 0.0 {
            Color::srgba(0.62, 0.40, 0.95, 0.16) // up = violet
        } else {
            Color::srgba(0.30, 0.80, 0.80, 0.16) // else teal
        };
        let center = (zone.aabb.min + zone.aabb.max) * 0.5;
        let size = zone.aabb.max - zone.aabb.min;
        let translation = crate::config::world_to_bevy(&world.0, center, 7.5);
        commands.spawn((
            GravityZoneVisual,
            Sprite::from_color(color, size),
            Transform::from_translation(translation),
            Name::new("Gravity zone visual"),
        ));
        // A brighter band on the edge gravity pulls TOWARD (the "down" edge under
        // this zone's gravity), so the zone reads as a DIRECTION — you can see
        // which way you'll fall before stepping in, not just that something
        // changes here.
        let band_color = if zone.dir.y < 0.0 {
            Color::srgba(0.62, 0.40, 0.95, 0.55) // up = violet
        } else {
            Color::srgba(0.30, 0.80, 0.80, 0.55) // else teal
        };
        let half_along = (size.x * zone.dir.x.abs() + size.y * zone.dir.y.abs()) * 0.5;
        let thickness = 10.0_f32.min(half_along * 0.8);
        let band_center = center + zone.dir * (half_along - thickness * 0.5);
        let band_size = ae::Vec2::new(
            if zone.dir.x != 0.0 { thickness } else { size.x },
            if zone.dir.y != 0.0 { thickness } else { size.y },
        );
        let band_translation = crate::config::world_to_bevy(&world.0, band_center, 7.6);
        commands.spawn((
            GravityZoneVisual,
            Sprite::from_color(band_color, band_size),
            Transform::from_translation(band_translation),
            Name::new("Gravity zone direction band"),
        ));
    }
}

/// Marks a sprite entity that visualizes a [`Portal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

/// Marks a transient sprite drawing one portal-aware spatial piece of a body
/// mid-transit (the entry-side slice or the exit-side slice). Rebuilt each frame.
#[derive(Component)]
pub struct PortalBodyPiece;

/// Marks the transient "portal disorientation" indicator above the player —
/// visible exactly while the held movement input is portal-warped.
#[derive(Component)]
pub struct PortalDisorientIndicator;

/// Show a small indicator over the player whenever their movement input is
/// portal-warped ([`PortalInputWarp`]) — so the "I'm holding left but moving
/// right" state is legible, and it disappears the instant the warp drops (on
/// release / redirect). Placeholder dot+glyph for now; a nicer effect (incl. on
/// the joystick visual) can replace it later.
pub fn sync_portal_disorientation_indicator(
    mut commands: Commands,
    world: Res<GameWorld>,
    existing: Query<Entity, With<PortalDisorientIndicator>>,
    player: Query<
        (&PlayerKinematics, Has<PortalInputWarp>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Ok((kin, warped)) = player.single() else {
        return;
    };
    if !warped {
        return;
    }
    // A little spinning-arrow glyph just above the head.
    let pos = kin.pos + Vec2::new(0.0, -(kin.size.y * 0.5 + 16.0));
    let translation =
        crate::config::world_to_bevy(&world.0, pos, crate::config::WORLD_Z_PLAYER + 9.0);
    commands.spawn((
        PortalDisorientIndicator,
        Text2d::new("\u{21BB}"), // ↻ clockwise open circle arrow
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.74, 0.92, 1.0)),
        Transform::from_translation(translation),
        Name::new("Portal disorientation indicator"),
    ));
}

/// Render the player mid-transit as the body in BOTH charts: the real sprite at
/// the entry, a second copy of the sprite emerging from the exit (rotated by the
/// somersault the body is taking), and an opaque box over the **invisible /
/// intangible** slice of each — the part of the entry sprite that has sunk
/// through the portal plane (into the wall), and the part of the exit copy that
/// has not yet emerged. So the visible part of each shows the real character art
/// and the through-the-wall part is masked off ("feet in, feet out"). Drawing
/// the sprite twice + masking sidesteps texture clipping until we tune visuals.
pub fn sync_portal_body_pieces(
    mut commands: Commands,
    world: Res<GameWorld>,
    pieces: Query<Entity, With<PortalBodyPiece>>,
    portals: Query<&Portal>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut player: Query<
        (
            &PlayerKinematics,
            Option<&PortalTransit>,
            Option<&ActorRoll>,
            &Sprite,
            &mut Visibility,
        ),
        With<crate::presentation::rendering::PlayerVisual>,
    >,
) {
    for entity in &pieces {
        commands.entity(entity).despawn();
    }
    let Ok((kin, transit, roll, sprite, mut visibility)) = player.single_mut() else {
        return;
    };
    // The real character sprite always shows now (no hiding) — the masks below
    // cover only the parts that have passed through a portal.
    *visibility = Visibility::Inherited;
    // The body is transiting exactly one portal — decompose against that pair.
    let Some(transit) = transit else {
        return;
    };
    let all: Vec<Portal> = portals.iter().copied().collect();
    let (Some(enter_portal), Some(exit_portal)) = (
        find_portal(&all, transit.straddling),
        find_portal(&all, transit.straddling.partner()),
    ) else {
        return;
    };
    let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
    // Decompose via the tested Core-invariant function so these slices can never
    // drift from the collision / gameplay decomposition.
    let pieces = pp::compute_body_pieces(body, Some((enter_portal.frame(), exit_portal.frame())));
    let Some(through) = pieces.through else {
        // Touching a portal but nothing has crossed the plane yet.
        return;
    };
    let (enter, exit) = (through.enter, through.exit);
    let base_roll = roll.map_or(0.0, |r| r.angle);
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
    // Opaque mask over the invisible/intangible slice (per Jon's note: the box
    // belongs over the part you should NOT see).
    let mask_color = Color::srgb(0.80, 0.95, 1.0);
    let mask_z = crate::config::WORLD_Z_PLAYER + 1.0;

    // Exit copy: the whole sprite emerging from the exit, mapped + rotated by the
    // somersault it is taking (none for a wall↔wall turn-around). For a suppressed
    // wall↔wall turn-around the sprite stays upright but its FACING mirrors, so it
    // comes out face-first (X-in, X-out) instead of back-first.
    let exit_center = pp::map_point(kin.pos, &enter, &exit);
    let exit_roll = base_roll + somersault_roll(enter.normal, exit.normal, gravity_dir);
    let mut exit_sprite = sprite.clone();
    if portal_facing_flips(enter.normal, exit.normal, gravity_dir) {
        exit_sprite.flip_x = !exit_sprite.flip_x;
    }
    let exit_translation =
        crate::config::world_to_bevy(&world.0, exit_center, crate::config::WORLD_Z_PLAYER);
    commands.spawn((
        PortalBodyPiece,
        exit_sprite,
        Transform::from_translation(exit_translation)
            .with_rotation(Quat::from_rotation_z(exit_roll)),
        Name::new("Portal body copy (exit)"),
    ));

    // Entry mask: the slice of the real sprite that has sunk THROUGH the entry
    // plane (into the wall) — invisible on this side.
    if let Some(hidden) = pp::clip_halfspace(body, enter.pos, -enter.normal) {
        let translation = crate::config::world_to_bevy(&world.0, hidden.center(), mask_z);
        commands.spawn((
            PortalBodyPiece,
            Sprite::from_color(mask_color, hidden.half_size() * 2.0),
            Transform::from_translation(translation),
            Name::new("Portal mask (entry, through-wall)"),
        ));
    }
    // Exit mask: the slice of the exit copy that has NOT yet emerged (still behind
    // the exit plane) — invisible until it comes out.
    let exit_body = pp::map_aabb(body, &enter, &exit);
    if let Some(hidden) = pp::clip_halfspace(exit_body, exit.pos, -exit.normal) {
        let translation = crate::config::world_to_bevy(&world.0, hidden.center(), mask_z);
        commands.spawn((
            PortalBodyPiece,
            Sprite::from_color(mask_color, hidden.half_size() * 2.0),
            Transform::from_translation(translation),
            Name::new("Portal mask (exit, not-yet-emerged)"),
        ));
    }
}

/// Loaded portal-gun art: the blue / orange mode sprites. Visible build only.
#[derive(Resource)]
pub struct PortalGunArt {
    pub blue: Handle<Image>,
    pub orange: Handle<Image>,
}

/// Load the portal-gun mode sprites at startup.
pub fn load_portal_gun_art(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(PortalGunArt {
        blue: assets.load("sprites/props/portal_gun_blue.png"),
        orange: assets.load("sprites/props/portal_gun_orange.png"),
    });
}

/// Marks the held portal-gun sprite carried in the player's hand.
#[derive(Component)]
pub struct PortalModeIndicator;

/// On-screen size of the portal-gun sprite, used for BOTH the held gun and the
/// ground pickup so it doesn't change size when you pick it up (keeps the
/// 140×64 sprite aspect ≈ 2.19).
const PORTAL_GUN_DISPLAY: Vec2 = Vec2::new(52.0, 24.0);

/// Draw the portal-gun sprite **in the player's hand**, rotated to point where
/// you're AIMING (the same direction `Attack` fires the portal — like the
/// pirates' wielded weapon), tinted to the active mode color so toggling
/// (Interact) visibly swaps blue↔orange.
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
    control: Res<ControlFrame>,
    world: Res<GameWorld>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalModeIndicator>>,
    players: Query<(&PlayerKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let Some(art) = art else {
        return;
    };
    // The gun only ever fires its blue↔orange pair; orange art for orange, blue
    // for everything else.
    let image = match gun.next_color {
        PortalColor::Orange => art.orange.clone(),
        _ => art.blue.clone(),
    };
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the player's hand: just in front of the body at roughly hand height
    // (y-down world, so a small +y is slightly below centre). z=12 keeps it
    // in front of the player sprite.
    let pos = kin.pos + Vec2::new(facing * (kin.size.x * 0.45 + 6.0), kin.size.y * 0.06);
    let translation = crate::config::world_to_bevy(&world.0, pos, 12.0);
    // Aim the barrel where the shot will go (same aim as `portal_fire_system`).
    // World y-down → render y-up; aiming left flips vertically so the gun stays
    // upright rather than upside-down.
    let aim = pick_aim(&control, kin.facing).normalize_or_zero();
    let angle = (-aim.y).atan2(aim.x);
    commands.spawn((
        PortalModeIndicator,
        Sprite {
            image,
            custom_size: Some(PORTAL_GUN_DISPLAY),
            flip_y: aim.x < 0.0,
            ..default()
        },
        Transform::from_translation(translation).with_rotation(Quat::from_rotation_z(angle)),
        Name::new("Held portal gun"),
    ));
}

/// Marks the visual for a [`GravityFlipSwitch`].
#[derive(Component)]
pub struct GravitySwitchVisual;

/// Draw the gravity-flip switch column, tinted green when gravity is normal and
/// orange when it's flipped, so the player can see the current gravity state.
pub fn sync_gravity_switch_visual(
    mut commands: Commands,
    world: Res<GameWorld>,
    gravity: Option<Res<GravityField>>,
    visuals: Query<Entity, With<GravitySwitchVisual>>,
    switches: Query<&GravityFlipSwitch>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let flipped = gravity.as_deref().is_some_and(|g| g.dir.y < 0.0);
    let color = if flipped {
        Color::srgba(0.95, 0.55, 0.20, 0.65)
    } else {
        Color::srgba(0.40, 0.90, 0.60, 0.65)
    };
    for sw in &switches {
        let translation = crate::config::world_to_bevy(&world.0, sw.pos, 8.5);
        commands.spawn((
            GravitySwitchVisual,
            Sprite::from_color(color, sw.half_extent * 2.0),
            Transform::from_translation(translation),
            Name::new("Gravity switch visual"),
        ));
    }
}

/// Colored quad per portal so the player can actually see them. Clear-and-
/// rebuild each frame — there are at most two portals, so the churn is
/// negligible and the visuals can never desync from the sim entities.
pub fn sync_portal_visuals(
    mut commands: Commands,
    world: Res<GameWorld>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalVisual>>,
    portals: Query<&Portal>,
    pickups: Query<&PortalGunPickup>,
    projectiles: Query<&PortalProjectile>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    // In-flight portal shots: a small bright streak in the shot's color.
    for proj in &projectiles {
        let color = proj.color.display().1;
        let translation = crate::config::world_to_bevy(&world.0, proj.pos, 9.5);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(color, Vec2::new(16.0, 8.0)),
            Transform::from_translation(translation),
            Name::new("Portal shot visual"),
        ));
    }
    // Uncollected portal-gun pickup: a purple marker quad.
    for pickup in &pickups {
        let translation = crate::config::world_to_bevy(&world.0, pickup.pos, 9.0);
        // The world pickup shows the actual gun sprite (blue mode by default);
        // falls back to a marker quad before the art has loaded.
        let sprite = match art.as_ref() {
            Some(art) => Sprite {
                image: art.blue.clone(),
                // Same display size as the held gun so it doesn't visibly
                // resize when you pick it up.
                custom_size: Some(PORTAL_GUN_DISPLAY),
                ..default()
            },
            None => Sprite::from_color(Color::srgb(0.66, 0.36, 0.92), pickup.half_extent * 2.0),
        };
        commands.spawn((
            PortalVisual,
            sprite,
            Transform::from_translation(translation),
            Name::new("Portal gun pickup visual"),
        ));
    }
    for portal in &portals {
        let (rim, core) = portal.color.display();
        // A portal is a thin doorway seen in side profile (2D): a bar lying
        // ALONG the wall (perpendicular to the surface normal), thin in the
        // normal direction. `along` rotates with the normal, so a slanted
        // surface yields a slanted portal for free.
        let n = portal.normal.normalize_or_zero();
        let along = Vec2::new(-n.y, n.x);
        // Opening half-length = the portal extent projected onto the wall
        // direction: a wall portal (horizontal normal) shows its full height,
        // a floor / ceiling portal shows its width.
        let opening_half =
            along.x.abs() * portal.half_extent.x + along.y.abs() * portal.half_extent.y;
        let length = (opening_half * 2.0).max(PORTAL_VISUAL_THICKNESS);
        // World is y-down, render space is y-up — flip y to get the on-screen
        // direction of the bar's long axis, then rotate the sprite to match.
        let angle = (-along.y).atan2(along.x);
        let rotation = Quat::from_rotation_z(angle);
        // Rim (outer) + brighter thin core, both oriented along the wall.
        let rim_translation = crate::config::world_to_bevy(&world.0, portal.pos, 9.0);
        let core_translation = crate::config::world_to_bevy(&world.0, portal.pos, 9.1);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(rim, Vec2::new(length, PORTAL_VISUAL_THICKNESS)),
            Transform::from_translation(rim_translation).with_rotation(rotation),
            Name::new("Portal visual (rim)"),
        ));
        commands.spawn((
            PortalVisual,
            Sprite::from_color(
                core,
                Vec2::new(length * 0.86, PORTAL_VISUAL_THICKNESS * 0.42),
            ),
            Transform::from_translation(core_translation).with_rotation(rotation),
            Name::new("Portal visual (core)"),
        ));
        // A small color-name label just out in front of the face, so portals can
        // be referred to precisely (each linked pair is a distinct complementary
        // color: purple↔yellow, teal↔red, …). The color name IS the identifier.
        let label_pos = portal.pos + n * 24.0;
        let label_translation = crate::config::world_to_bevy(&world.0, label_pos, 9.2);
        commands.spawn((
            PortalVisual,
            Text2d::new(portal.color.name()),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(core),
            Transform::from_translation(label_translation),
            Name::new("Portal label"),
        ));
    }
}
