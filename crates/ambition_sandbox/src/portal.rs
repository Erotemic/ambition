//! Portal gun — the flagship player ability (vertical slice).
//!
//! Fire a blue/orange portal pair onto solid surfaces, then travel between
//! them carrying your momentum (Portal-style). This module is the
//! self-contained mechanic: components + the three systems (fire / toggle /
//! teleport) + a pure ray-vs-solids helper. It is deterministic (no RNG, no
//! per-frame allocation in the hot path) so it runs identically in the
//! headless sim.
//!
//! Controls (per the design decision): when the portal gun is `active`,
//! `Attack` fires/places a portal of the current color and `Interact` toggles
//! blue↔orange. Today the player is granted an always-active portal gun by
//! `grant_portal_gun`; once held-item equip lands (see TODO "Grid inventory")
//! the `active` flag is driven by equipping the portal-gun item instead.
//!
//! Handoff / feel notes (left intentionally untuned):
//! - exit velocity shoots straight out of the exit portal at the incoming
//!   speed; a fuller transform (reflect the velocity through the portal pair)
//!   would preserve diagonal entry — easy follow-up.
//! - vertical aiming uses `aim`/`axis` as-is; confirm the y-sign feels right.
//! - portals are a fixed 56px square; orient/size them to the hit surface later.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::GameWorld;

/// Which of the two linked portals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalColor {
    Blue,
    Orange,
}

impl PortalColor {
    pub fn other(self) -> Self {
        match self {
            PortalColor::Blue => PortalColor::Orange,
            PortalColor::Orange => PortalColor::Blue,
        }
    }
}

/// Player-held portal gun state.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGun {
    /// When false the gun ignores input (stand-in for "not equipped" until
    /// held-item equip exists).
    pub active: bool,
    /// Color the next `Attack` will place.
    pub next_color: PortalColor,
    /// Seconds before another teleport is allowed (prevents ping-pong).
    pub teleport_cooldown: f32,
}

impl Default for PortalGun {
    fn default() -> Self {
        Self {
            active: true,
            next_color: PortalColor::Blue,
            teleport_cooldown: 0.0,
        }
    }
}

/// One placed portal. The pair is linked implicitly by `color` (one Blue +
/// one Orange exist at most).
#[derive(Component, Clone, Copy, Debug)]
pub struct Portal {
    pub color: PortalColor,
    /// World-space center (on the hit surface).
    pub pos: Vec2,
    /// Unit surface normal, pointing out of the wall into the room.
    pub normal: Vec2,
    /// Half-extent of the portal's overlap region.
    pub half_extent: Vec2,
}

const PORTAL_HALF: f32 = 28.0;
/// Portals are tall doorways (taller than wide) so you don't miss them
/// vertically when approaching at a different height than you fired them.
const PORTAL_HALF_H: f32 = 46.0;
const PORTAL_MAX_RANGE: f32 = 6000.0;
/// Portal shot travel speed (px/s) — fast, but slow enough to see the streak.
const PORTAL_SHOT_SPEED: f32 = 1900.0;
const TELEPORT_COOLDOWN_S: f32 = 0.25;
/// Floor on exit speed so a slow walk into a portal still pops you out the
/// far side instead of stalling inside the exit portal.
const MIN_EXIT_SPEED: f32 = 220.0;

// ---------------------------------------------------------------------------
// Pure geometry — ray vs solid AABBs (slab method).

/// Nearest solid surface hit by a ray from `origin` along `dir`. Returns the
/// hit point and the outward face normal (pointing back toward the ray).
pub fn raycast_solids(
    world: &ae::World,
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
) -> Option<(Vec2, Vec2)> {
    let dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut best_t = max_dist;
    let mut best_normal = Vec2::ZERO;
    for block in &world.blocks {
        if !matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) {
            continue;
        }
        if let Some((t, n)) = ray_aabb(origin, dir, block.aabb) {
            if t < best_t {
                best_t = t;
                best_normal = n;
            }
        }
    }
    if best_normal == Vec2::ZERO {
        None
    } else {
        Some((origin + dir * best_t, best_normal))
    }
}

/// Ray-vs-AABB. Returns `(t_near, face_normal)` for a forward hit (`t >= 0`).
fn ray_aabb(origin: Vec2, dir: Vec2, aabb: ae::Aabb) -> Option<(f32, Vec2)> {
    // 1/0 → ±inf is the intended slab-method behavior for axis-parallel rays.
    let inv = Vec2::new(1.0 / dir.x, 1.0 / dir.y);
    let tx1 = (aabb.min.x - origin.x) * inv.x;
    let tx2 = (aabb.max.x - origin.x) * inv.x;
    let ty1 = (aabb.min.y - origin.y) * inv.y;
    let ty2 = (aabb.max.y - origin.y) * inv.y;
    let tminx = tx1.min(tx2);
    let tmaxx = tx1.max(tx2);
    let tminy = ty1.min(ty2);
    let tmaxy = ty1.max(ty2);
    let t_near = tminx.max(tminy);
    let t_far = tmaxx.min(tmaxy);
    if t_near > t_far || t_far < 0.0 {
        return None;
    }
    // The axis that produced t_near is the face we hit; its normal opposes
    // the ray's travel on that axis.
    let normal = if tminx > tminy {
        Vec2::new(-dir.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, -dir.y.signum())
    };
    Some((t_near.max(0.0), normal))
}

/// Transform a velocity through a portal pair: the rotation that maps the
/// "into the entry portal" direction (`-n_in`) onto the "out of the exit
/// portal" direction (`n_out`), applied to `v`. This preserves the player's
/// sideways momentum through the pair (Portal-style) instead of always
/// shooting straight out.
pub fn portal_transform_velocity(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    let u = -n_in; // direction the player was traveling into the entry portal
    let cos = u.dot(n_out);
    let sin = u.x * n_out.y - u.y * n_out.x; // 2D cross (z component)
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// Aim direction for a fired portal: right-stick aim, else movement axis,
/// else straight ahead along facing.
fn pick_aim(control: &ControlFrame, facing: f32) -> Vec2 {
    let aim = Vec2::new(control.aim_x, control.aim_y);
    if aim.length() > 0.2 {
        return aim;
    }
    let mv = Vec2::new(control.axis_x, control.axis_y);
    if mv.length() > 0.2 {
        return mv;
    }
    Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
}

// ---------------------------------------------------------------------------
// Systems.

/// A portal gun resting in the world. Walking onto it and pressing `Attack`
/// activates the player's (inactive) portal gun — "pick up the portal gun in
/// a room". Kept distinct from `item_pickup::GroundItem` because the portal
/// gun's ability is the `PortalGun` component, not a `HeldItemSpec` verb.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGunPickup {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Seconds before this pickup can be grabbed. A *just-dropped* gun arms
    /// after a short delay so the same `Attack` press that dropped it (and the
    /// next overlapping frame) can't immediately re-grab it. World-placed
    /// pickups spawn already armed (`0.0`).
    pub arm_timer: f32,
}

/// Spawn one portal-gun pickup near the player on the first frame a player
/// exists (debug convenience until authored placement lands).
pub fn spawn_debug_portal_gun_pickup_once(
    mut commands: Commands,
    mut done: Local<bool>,
    players: Query<&PlayerKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    if *done {
        return;
    }
    let Ok(kin) = players.single() else {
        return;
    };
    *done = true;
    commands.spawn((
        PortalGunPickup {
            pos: kin.pos + Vec2::new(-80.0, 0.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.0,
        },
        Name::new("Portal gun pickup"),
    ));
}

/// Tick down each pickup's [`PortalGunPickup::arm_timer`] so a just-dropped gun
/// becomes grabbable after the short delay. Always runs (cheap; at most a
/// couple of pickups).
pub fn arm_portal_pickups(
    time: Res<crate::WorldTime>,
    mut pickups: Query<&mut PortalGunPickup>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for mut pickup in &mut pickups {
        if pickup.arm_timer > 0.0 {
            pickup.arm_timer = (pickup.arm_timer - dt).max(0.0);
        }
    }
}

/// `Shield + Attack` drops the held portal gun: removes the `PortalGun` (so
/// `Attack` stops firing portals) and leaves a `PortalGunPickup` at the
/// player's feet to grab again. Only when not also holding a throwable item
/// (that throw takes precedence).
pub fn drop_portal_gun_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    players: Query<
        (Entity, &PlayerKinematics),
        (
            With<PlayerEntity>,
            With<PrimaryPlayer>,
            With<PortalGun>,
            Without<crate::features::HeldItem>,
        ),
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !(control.shield_held && control.attack_pressed) {
        return;
    }
    let Ok((player, kin)) = players.single() else {
        return;
    };
    commands.entity(player).remove::<PortalGun>();
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    commands.spawn((
        PortalGunPickup {
            // Drop it a bit ahead and arm it after a short delay so this same
            // Attack press (and the immediately-overlapping next frame) can't
            // re-grab it — that was the "can't drop the portal gun" bug.
            pos: kin.pos + Vec2::new(facing * 44.0, 0.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.35,
        },
        Name::new("Portal gun pickup"),
    ));
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIZZLE,
        pos: kin.pos,
    });
}

/// `Attack` while overlapping the [`PortalGunPickup`] grants the player an
/// (active) `PortalGun` and consumes the pickup. The gun is a **single item**:
/// it doesn't exist until you pick it up (no separate granted-but-inactive
/// component) — picking up the one world item *is* getting the portal gun.
pub fn pickup_portal_gun_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    players: Query<(Entity, &PlayerKinematics), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    already_have: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>, With<PortalGun>)>,
    // One item at a time (Smash-style): can't grab the portal gun while holding
    // a ground item (axe / gun-sword / javelin).
    holding_item: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>, With<crate::features::HeldItem>)>,
    pickups: Query<(Entity, &PortalGunPickup)>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || !already_have.is_empty() || !holding_item.is_empty() {
        return;
    }
    let Ok((player, kin)) = players.single() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (entity, pickup) in &pickups {
        if pickup.arm_timer > 0.0 {
            continue;
        }
        if player_aabb.strict_intersects(ae::Aabb::new(pickup.pos, pickup.half_extent)) {
            commands.entity(player).insert(PortalGun {
                active: true,
                ..PortalGun::default()
            });
            commands.entity(entity).despawn();
            // Rising sci-fi charge-up as the device wakes.
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::portal", "picked up the portal gun");
            break;
        }
    }
}

/// An in-flight portal shot streaking toward a surface. On contact with a
/// solid it opens a portal of `color`; if it travels too far / leaves the
/// world it fizzles.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalProjectile {
    pub color: PortalColor,
    pub pos: Vec2,
    pub vel: Vec2,
    pub traveled: f32,
}

/// `Attack` fires a portal *shot* of the gun's current color along the aim
/// direction. The shot travels (see `portal_projectile_step`) so the player
/// sees its path before it lands and opens a portal.
pub fn portal_fire_system(
    control: Res<ControlFrame>,
    players: Query<(&PlayerKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut commands: Commands,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    // Shield+Attack is the "drop the gun" gesture — don't fire on it.
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let aim = pick_aim(&control, kin.facing).normalize_or_zero();
    if aim == Vec2::ZERO {
        return;
    }
    // Punchy fire blast + the airy travel whizz.
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_FIRE,
        pos: kin.pos,
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PORTAL_TRAVEL,
        pos: kin.pos,
    });
    commands.spawn((
        PortalProjectile {
            color: gun.next_color,
            pos: kin.pos,
            vel: aim * PORTAL_SHOT_SPEED,
            traveled: 0.0,
        },
        Name::new("Portal shot"),
    ));
}

/// Advance portal shots; open a portal on solid contact (the bright warping
/// whoosh) or fizzle past max range / out of bounds (the rejection buzz).
pub fn portal_projectile_step(
    time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut PortalProjectile)>,
    portals: Query<(Entity, &Portal)>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (proj_entity, mut proj) in &mut projectiles {
        let step = (proj.vel * dt).length().max(1.0);
        if let Some((hit, normal)) = raycast_solids(&world.0, proj.pos, proj.vel, step) {
            // Hit a wall — open (or replace) the portal of this color.
            for (entity, portal) in &portals {
                if portal.color == proj.color {
                    commands.entity(entity).despawn();
                    sfx.write(crate::audio::SfxMessage::Play {
                        id: ambition_sfx::ids::PORTAL_CLOSE,
                        pos: hit,
                    });
                }
            }
            commands.spawn((
                Portal {
                    color: proj.color,
                    pos: hit + normal * 2.0,
                    normal,
                    half_extent: Vec2::new(PORTAL_HALF, PORTAL_HALF_H),
                },
                Name::new(match proj.color {
                    PortalColor::Blue => "Portal: blue",
                    PortalColor::Orange => "Portal: orange",
                }),
            ));
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ATTACH,
                pos: hit,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }
        let delta = proj.vel * dt;
        proj.pos += delta;
        proj.traveled += step;
        let oob = proj.pos.x < -64.0
            || proj.pos.y < -64.0
            || proj.pos.x > world.0.size.x + 64.0
            || proj.pos.y > world.0.size.y + 64.0;
        if proj.traveled > PORTAL_MAX_RANGE || oob {
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_INVALID,
                pos: proj.pos,
            });
            commands.entity(proj_entity).despawn();
        }
    }
}

/// `Interact` toggles which color the next `Attack` will place.
pub fn portal_toggle_system(
    control: Res<ControlFrame>,
    mut players: Query<&mut PortalGun, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    nearest: Option<Res<crate::player::affordances::NearestInteractable>>,
) {
    if !control.interact_pressed {
        return;
    }
    // A genuine interactable (door / NPC / switch) claims the Interact press,
    // matching the HUD label — only toggle portal mode when there's none.
    if let Some(nearest) = nearest.as_deref() {
        if !matches!(
            nearest.0,
            crate::player::affordances::InteractVariant::None
        ) {
            return;
        }
    }
    let Ok(mut gun) = players.single_mut() else {
        return;
    };
    if gun.active {
        gun.next_color = gun.next_color.other();
    }
}


/// One-frame flag: set true the frame the player teleports through a portal,
/// so the trace position-delta detector treats it as an *intentional* teleport
/// and doesn't auto-dump. Read + cleared by the gameplay-trace system.
#[derive(Resource, Default)]
pub struct IntentionalTeleport(pub bool);

/// Teleport the player between linked portals, carrying momentum. Requires
/// both portals to exist; a short cooldown after each jump prevents ping-pong.
pub fn portal_teleport_system(
    time: Res<crate::WorldTime>,
    mut players: Query<
        (&mut PlayerKinematics, &mut PortalGun),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    portals: Query<&Portal>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut intentional: Option<ResMut<IntentionalTeleport>>,
) {
    // Default to "no teleport this frame"; set true below if one happens.
    if let Some(flag) = intentional.as_deref_mut() {
        flag.0 = false;
    }
    let dt = time.sim_dt();
    let Ok((mut kin, mut gun)) = players.single_mut() else {
        return;
    };
    if gun.teleport_cooldown > 0.0 {
        gun.teleport_cooldown = (gun.teleport_cooldown - dt).max(0.0);
        return;
    }
    let blue = portals.iter().find(|p| p.color == PortalColor::Blue).copied();
    let orange = portals
        .iter()
        .find(|p| p.color == PortalColor::Orange)
        .copied();
    let (Some(blue), Some(orange)) = (blue, orange) else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (enter, exit) in [(blue, orange), (orange, blue)] {
        // A small margin makes the entry forgiving — the player only reaches
        // the wall surface, not the portal's center.
        let portal_aabb = ae::Aabb::new(enter.pos, enter.half_extent + Vec2::splat(6.0));
        if player_aabb.strict_intersects(portal_aabb) {
            // Carry momentum through the pair's rotation; floor the speed so a
            // slow walk-in still pops out the far side instead of stalling.
            let mut out_vel = portal_transform_velocity(kin.vel, enter.normal, exit.normal);
            if out_vel.length() < MIN_EXIT_SPEED {
                out_vel = exit.normal * MIN_EXIT_SPEED;
            }
            // Pop out just clear of the exit portal so we don't re-trigger it.
            let clearance = kin.size.length() * 0.5 + exit.half_extent.length() + 4.0;
            kin.pos = exit.pos + exit.normal * clearance;
            kin.vel = out_vel;
            gun.teleport_cooldown = TELEPORT_COOLDOWN_S;
            // Suction warp going in, soft pop-out coming back into normal space.
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ENTER,
                pos: enter.pos,
            });
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_EXIT,
                pos: exit.pos,
            });
            bevy::log::info!(target: "ambition::portal", "teleported through the portal pair");
            break;
        }
    }
}

/// Despawn all portals when the room resets / transitions, and clear the
/// gun's teleport cooldown — portals are per-room, so stale ones from a
/// previous room must not linger and teleport the player unexpectedly.
pub fn clear_portals_on_reset(
    mut commands: Commands,
    mut resets: MessageReader<crate::features::ResetRoomFeaturesEvent>,
    portals: Query<Entity, With<Portal>>,
    mut guns: Query<&mut PortalGun>,
) {
    if resets.read().next().is_none() {
        return;
    }
    for entity in &portals {
        commands.entity(entity).despawn();
    }
    for mut gun in &mut guns {
        gun.teleport_cooldown = 0.0;
    }
}

/// Dev off-switch: `F7` toggles the portal gun active/inactive so the
/// always-on slice gun doesn't fire portals on every Attack while testing
/// other sandbox mechanics. (Visible build only.) Final gating is via
/// held-item equip; this is a developer convenience until then.
pub fn portal_dev_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut guns: Query<&mut PortalGun>,
) {
    if !keys.just_pressed(KeyCode::F7) {
        return;
    }
    for mut gun in &mut guns {
        gun.active = !gun.active;
        bevy::log::info!(target: "ambition::portal", "portal gun active = {}", gun.active);
    }
}

/// In-flight ground items (thrown axes / javelins) also travel through the
/// portal pair, carrying momentum through the rotation — throw a javelin into
/// the blue portal and it flies out of the orange one. Resting items are
/// ignored (only `vel != ZERO` items teleport), and a teleported item pops out
/// clear of the exit portal so it doesn't immediately re-enter.
pub fn portal_teleport_ground_items(
    portals: Query<&Portal>,
    mut items: Query<&mut crate::item_pickup::GroundItem>,
) {
    let blue = portals.iter().find(|p| p.color == PortalColor::Blue).copied();
    let orange = portals
        .iter()
        .find(|p| p.color == PortalColor::Orange)
        .copied();
    let (Some(blue), Some(orange)) = (blue, orange) else {
        return;
    };
    for mut item in &mut items {
        if item.vel == Vec2::ZERO {
            continue;
        }
        let item_aabb = ae::Aabb::new(item.pos, item.half_extent);
        for (enter, exit) in [(blue, orange), (orange, blue)] {
            if item_aabb.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent)) {
                // Rotation preserves speed, so momentum carries through.
                item.vel = portal_transform_velocity(item.vel, enter.normal, exit.normal);
                let clearance = item.half_extent.length() + exit.half_extent.length() + 4.0;
                item.pos = exit.pos + exit.normal * clearance;
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Presentation (visible build only — registered by the presentation plugin).

/// Marks a sprite entity that visualizes a [`Portal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

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

/// Draw the portal-gun sprite **in the player's hand** (in front, at hand
/// height, flipped to face), tinted to the active mode color so toggling
/// (Interact) visibly swaps blue↔orange.
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
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
    let image = match gun.next_color {
        PortalColor::Blue => art.blue.clone(),
        PortalColor::Orange => art.orange.clone(),
    };
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the player's hand: just in front of the body at roughly hand height
    // (y-down world, so a small +y is slightly below centre). z=12 keeps it
    // in front of the player sprite.
    let pos = kin.pos + Vec2::new(facing * (kin.size.x * 0.45 + 6.0), kin.size.y * 0.06);
    let translation = crate::config::world_to_bevy(&world.0, pos, 12.0);
    commands.spawn((
        PortalModeIndicator,
        Sprite {
            image,
            custom_size: Some(Vec2::new(38.0, 20.0)),
            flip_x: facing < 0.0,
            ..default()
        },
        Transform::from_translation(translation),
        Name::new("Held portal gun"),
    ));
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
        let color = match proj.color {
            PortalColor::Blue => Color::srgb(0.55, 0.85, 1.0),
            PortalColor::Orange => Color::srgb(1.0, 0.72, 0.35),
        };
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
                custom_size: Some(Vec2::new(52.0, 28.0)),
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
        let color = match portal.color {
            PortalColor::Blue => Color::srgb(0.30, 0.62, 1.0),
            PortalColor::Orange => Color::srgb(1.0, 0.55, 0.20),
        };
        let translation = crate::config::world_to_bevy(&world.0, portal.pos, 9.0);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(color, portal.half_extent * 2.0),
            Transform::from_translation(translation),
            Name::new("Portal visual"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_two_walls() -> GameWorld {
        // Left wall x[0,20], right wall x[380,400], both y[0,400].
        let blocks = vec![
            ae::Block::solid("left", Vec2::new(0.0, 0.0), Vec2::new(20.0, 400.0)),
            ae::Block::solid("right", Vec2::new(380.0, 0.0), Vec2::new(20.0, 400.0)),
        ];
        GameWorld(ae::World::new(
            "portal_test",
            Vec2::new(400.0, 400.0),
            Vec2::new(200.0, 360.0),
            blocks,
        ))
    }

    fn spawn_player(app: &mut App, pos: Vec2, facing: f32) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing,
                },
                PortalGun::default(),
            ))
            .id()
    }

    fn find_portal(app: &mut App, color: PortalColor) -> Option<Portal> {
        let mut q = app.world_mut().query::<&Portal>();
        let world = app.world();
        q.iter(world).find(|p| p.color == color).copied()
    }

    fn set_control(app: &mut App, attack: bool, interact: bool) {
        let mut cf = app.world_mut().resource_mut::<ControlFrame>();
        cf.attack_pressed = attack;
        cf.interact_pressed = interact;
    }

    #[test]
    fn raycast_hits_nearest_solid_face_with_outward_normal() {
        let world = world_with_two_walls().0;
        // Fire left from mid-room: hit the left wall's right face at x=20,
        // normal pointing back toward the shooter (+x).
        let (hit, normal) = raycast_solids(&world, Vec2::new(200.0, 200.0), Vec2::new(-1.0, 0.0), 6000.0)
            .expect("ray should hit the left wall");
        assert!((hit.x - 20.0).abs() < 0.001, "hit x={}", hit.x);
        assert!(normal.x > 0.5 && normal.y.abs() < 0.001, "normal={normal:?}");
    }

    #[test]
    fn velocity_transform_rotates_through_perpendicular_portals() {
        // Entry: a floor portal whose normal points up (in y-down world, up = -y).
        // Exit: a right-wall portal whose normal points left (-x).
        // The player falls in moving down (+y) and should emerge moving left
        // (out of the exit portal) at the same speed — a 90° turn.
        let out = portal_transform_velocity(
            Vec2::new(0.0, 100.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        );
        assert!(
            (out.x + 100.0).abs() < 0.01 && out.y.abs() < 0.01,
            "fall-in should exit left at the same speed, got {out:?}"
        );
    }

    #[test]
    fn in_flight_ground_item_travels_through_the_portal_pair() {
        use crate::item_pickup::GroundItem;
        let mut app = App::new();
        app.add_systems(Update, portal_teleport_ground_items);
        // Blue portal facing right at x=20, orange facing left at x=380.
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(20.0, 200.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: Vec2::splat(PORTAL_HALF),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::splat(PORTAL_HALF),
        });
        // A thrown item flying into the blue portal.
        let item = app
            .world_mut()
            .spawn(GroundItem {
                spec: crate::item_pickup::axe_spec(),
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-300.0, 0.0),
                half_extent: Vec2::splat(12.0),
            })
            .id();
        app.update();
        let g = app.world().get::<GroundItem>(item).unwrap();
        assert!(
            g.pos.x > 250.0,
            "item should have come out of the orange (right) portal, pos={:?}",
            g.pos
        );
        assert!(
            (g.vel.length() - 300.0).abs() < 1.0,
            "momentum carries through the portal, vel={:?}",
            g.vel
        );
    }

    #[test]
    fn picking_up_the_portal_gun_activates_it() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, pickup_portal_gun_system);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: Vec2::new(50.0, 50.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                // No PortalGun yet — the single pickup item grants it.
            ))
            .id();
        app.world_mut().spawn(PortalGunPickup {
            pos: Vec2::new(50.0, 50.0),
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.0,
        });
        assert!(app.world().get::<PortalGun>(player).is_none());

        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        assert!(
            app.world()
                .get::<PortalGun>(player)
                .is_some_and(|g| g.active),
            "walking onto the pickup and pressing Attack grants the active gun"
        );
        let remaining = {
            let mut q = app.world_mut().query::<&PortalGunPickup>();
            q.iter(app.world()).count()
        };
        assert_eq!(remaining, 0, "the pickup is consumed");
    }

    #[test]
    fn dropped_portal_gun_arms_before_it_can_be_regrabbed() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_systems(
            Update,
            (
                drop_portal_gun_system,
                arm_portal_pickups,
                pickup_portal_gun_system,
            )
                .chain(),
        );
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);

        // Shield + Attack drops the gun.
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = true;
            cf.shield_held = true;
        }
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_none(),
            "Shield+Attack should drop the portal gun"
        );

        // Move the player directly onto the dropped pickup so only the arm
        // timer (not distance) guards against a re-grab.
        let pickup_pos = {
            let mut q = app.world_mut().query::<&PortalGunPickup>();
            q.iter(app.world()).next().expect("a pickup was dropped").pos
        };
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = pickup_pos;

        // Immediately press Attack again while overlapping — the freshly-dropped
        // pickup is still arming, so it must NOT be re-grabbed (the bug).
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = true;
            cf.shield_held = false;
        }
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_none(),
            "an armed (just-dropped) pickup can't be re-grabbed on the next Attack"
        );

        // Let it disarm, then Attack picks it back up.
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = false;
        }
        for _ in 0..30 {
            app.update();
        }
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = true;
        }
        app.update();
        assert!(
            app.world().get::<PortalGun>(player).is_some(),
            "once disarmed, Attack while overlapping re-grabs the gun"
        );
    }

    #[test]
    fn portal_pair_teleports_player_carrying_momentum() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(crate::WorldTime::default());
        app.add_systems(Update, portal_teleport_system);
        // Blue on the left (facing right), orange on the right (facing left).
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(22.0, 200.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: Vec2::new(PORTAL_HALF, PORTAL_HALF_H),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(378.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(PORTAL_HALF, PORTAL_HALF_H),
        });
        let player = spawn_player(&mut app, Vec2::new(22.0, 200.0), 1.0);
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .vel = Vec2::new(-100.0, 0.0);
        app.update();
        let kin = *app.world().get::<PlayerKinematics>(player).unwrap();
        assert!(
            kin.pos.x > 250.0,
            "player should have teleported to the orange (right) side, got {:?}",
            kin.pos
        );
        assert!(
            kin.vel.length() >= MIN_EXIT_SPEED - 1.0,
            "exit should carry momentum (>= min exit speed), got {:?}",
            kin.vel
        );
    }

    #[test]
    fn portal_shot_travels_and_opens_a_portal_on_a_wall() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(world_with_two_walls());
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, (portal_fire_system, portal_projectile_step).chain());
        // Player mid-room facing left.
        spawn_player(&mut app, Vec2::new(200.0, 200.0), -1.0);

        // One Attack pulse fires a single shot.
        set_control(&mut app, true, false);
        app.update();
        assert_eq!(
            {
                let mut q = app.world_mut().query::<&PortalProjectile>();
                q.iter(app.world()).count()
            },
            1,
            "firing spawns a traveling portal shot"
        );
        // No portal yet — it has to travel there.
        assert!(find_portal(&mut app, PortalColor::Blue).is_none());

        // Let the shot fly into the left wall.
        set_control(&mut app, false, false);
        for _ in 0..40 {
            app.update();
        }
        let blue = find_portal(&mut app, PortalColor::Blue);
        assert!(
            blue.is_some_and(|p| p.pos.x < 60.0 && p.normal.x > 0.5),
            "the shot should open a blue portal on the left wall, got {blue:?}"
        );
        assert_eq!(
            {
                let mut q = app.world_mut().query::<&PortalProjectile>();
                q.iter(app.world()).count()
            },
            0,
            "the shot is consumed when it lands"
        );
    }
}
