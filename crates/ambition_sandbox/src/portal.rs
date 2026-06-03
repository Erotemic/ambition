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
//! blue↔orange. The gun is a single item picked up in the room
//! (`PortalGunPickup`); equipping it stashes the player's melee so `Attack`
//! fires portals instead of swinging (the same attack-replacement the held
//! gun-sword / axe use), and dropping it restores the swing.
//!
//! Portals are thin oriented doorways: a fixed-length opening
//! (`PORTAL_OPENING_HALF`) along the hit surface, thin through it
//! (`PORTAL_THICKNESS_HALF`), so the face is the same size on a wall, floor, or
//! ceiling and the warp happens right at the drawn face (`portal_half_extent`).

use bevy::prelude::*;

use crate::brain::ActionSet;
use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::item_pickup::StashedActionSet;
use crate::physics::{gravity_upright_angle, GravityField, GravityZone};
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

/// A portal opening is the SAME size in every orientation: a doorway
/// `PORTAL_OPENING_HALF * 2` long along the surface, and thin perpendicular to
/// it (we only see its side profile in 2D). Both the drawn face AND the capture
/// box that warps the player are built from these, so the warp happens right at
/// the visual face regardless of whether the portal is on a wall, floor, or
/// ceiling.
const PORTAL_OPENING_HALF: f32 = 46.0;
const PORTAL_THICKNESS_HALF: f32 = 9.0;
const PORTAL_MAX_RANGE: f32 = 6000.0;
/// Portal shot travel speed (px/s) — fast, but slow enough to see the streak.
const PORTAL_SHOT_SPEED: f32 = 1900.0;
const TELEPORT_COOLDOWN_S: f32 = 0.25;
/// Floor on exit speed so a slow walk into a portal still pops you out the
/// far side instead of stalling inside the exit portal.
const MIN_EXIT_SPEED: f32 = 220.0;
/// On-screen thickness of the thin portal doorway (side profile in 2D). The
/// bar's *length* comes from the portal opening; this is its narrow dimension,
/// matched to the capture box so the player warps right at the drawn face.
const PORTAL_VISUAL_THICKNESS: f32 = PORTAL_THICKNESS_HALF * 2.0;

/// Oriented half-extent for a portal on a surface with the given `normal`:
/// `PORTAL_OPENING_HALF` along the surface (perpendicular to the normal) and
/// `PORTAL_THICKNESS_HALF` through it. So the opening (face) is the same length
/// in every orientation and the box is thin in the normal direction. An
/// axis-aligned normal gives an exact thin box; a slanted normal gives the
/// axis-aligned box that bounds the tilted face (good enough until slanted
/// portals are real).
pub fn portal_half_extent(normal: Vec2) -> Vec2 {
    let n = normal.normalize_or_zero();
    let along = Vec2::new(-n.y, n.x);
    Vec2::new(
        along.x.abs() * PORTAL_OPENING_HALF + n.x.abs() * PORTAL_THICKNESS_HALF,
        along.y.abs() * PORTAL_OPENING_HALF + n.y.abs() * PORTAL_THICKNESS_HALF,
    )
}

/// How far out of the exit portal (along its normal) to pop a body so it clears
/// the thin portal face without immediately re-entering: the body's half-size
/// projected onto the normal, plus the portal's thickness and a hair of margin.
/// Pops the body out right next to the face — NOT the old over-large
/// `half_extent.length()` push that included the full opening length.
fn portal_exit_clearance(half_size: Vec2, exit_normal: Vec2) -> f32 {
    half_size.dot(exit_normal.abs()) + PORTAL_THICKNESS_HALF + 3.0
}

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
    mut players: Query<
        (Entity, &PlayerKinematics, &mut ActionSet, Option<&StashedActionSet>),
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
    let Ok((player, kin, mut action_set, stashed)) = players.single_mut() else {
        return;
    };
    commands.entity(player).remove::<PortalGun>();
    // Restore the swing the gun replaced (same path the held items use).
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<StashedActionSet>();
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
    mut players: Query<
        (Entity, &PlayerKinematics, &mut ActionSet),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
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
    let Ok((player, kin, mut action_set)) = players.single_mut() else {
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
            // Equipping the portal gun REPLACES the attack: stash the player's
            // ActionSet and clear the melee swing so Attack fires portals
            // instead of swinging (same StashedActionSet path the held axe /
            // gun-sword use — unified held-item attack replacement).
            commands
                .entity(player)
                .insert(StashedActionSet(action_set.clone()));
            action_set.melee = None;
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
                    half_extent: portal_half_extent(normal),
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

/// A sandbox gravity-flip switch: a tall pressure-plate column the player steps
/// into to flip [`GravityField`] up↔down. Tall so it's reachable from both the
/// floor and the ceiling (after a flip you're on the ceiling — walk back into
/// the column to flip again). `armed` latches so one entry = one flip.
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityFlipSwitch {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// True when the player is clear of the plate, so the next entry flips.
    pub armed: bool,
}

/// Spawn one gravity-flip switch column near the player the first frame a
/// player exists (debug convenience until authored placement lands).
pub fn spawn_debug_gravity_switch_once(
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
        GravityFlipSwitch {
            pos: kin.pos + Vec2::new(140.0, 0.0),
            // Tall column so it spans floor→ceiling and is reachable either way.
            half_extent: Vec2::new(16.0, 220.0),
            armed: true,
        },
        Name::new("Gravity flip switch"),
    ));
}

/// Flip the room's **ambient** gravity ([`crate::physics::BaseGravity`]) up↔down
/// when the player steps into a [`GravityFlipSwitch`] (rising-edge latched by
/// `armed`). Flipping the ambient (not the live `GravityField` directly) lets
/// gravity zones override locally while the switch sets the room default.
pub fn gravity_flip_switch_system(
    mut base: ResMut<crate::physics::BaseGravity>,
    players: Query<&PlayerKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut switches: Query<&mut GravityFlipSwitch>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let Ok(kin) = players.single() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for mut sw in &mut switches {
        let overlapping = player_aabb.strict_intersects(ae::Aabb::new(sw.pos, sw.half_extent));
        if overlapping && sw.armed {
            // Flip the vertical component of the ambient gravity.
            base.dir = Vec2::new(base.dir.x, -base.dir.y);
            sw.armed = false;
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition::portal", "ambient gravity flipped: dir = {:?}", base.dir);
        } else if !overlapping {
            sw.armed = true;
        }
    }
}

/// Spawn one **gravity-up zone** near the player the first frame a player exists
/// (debug convenience until LDtk-authored placement lands): a tall region to the
/// right where gravity points up, so walking in drops you onto the ceiling and
/// walking out returns you to the ambient gravity.
pub fn spawn_debug_gravity_zone_once(
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
    let center = kin.pos + Vec2::new(380.0, 0.0);
    commands.spawn((
        GravityZone {
            aabb: ae::Aabb::new(center, Vec2::new(140.0, 240.0)),
            dir: Vec2::new(0.0, -1.0), // up
        },
        Name::new("Gravity zone: up"),
    ));
}

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
    }
}

/// Reset gravity to the default (down) when the room resets, so a flipped /
/// zoned room doesn't carry over.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<crate::features::ResetRoomFeaturesEvent>,
    mut gravity: ResMut<GravityField>,
    mut base: ResMut<crate::physics::BaseGravity>,
) {
    if resets.read().next().is_none() {
        return;
    }
    *gravity = GravityField::default();
    *base = crate::physics::BaseGravity::default();
}

/// Visual roll (aerial orientation) of an actor — player OR any NPC / enemy /
/// boss — in render-space radians. Portal transit ADDS the rotation the velocity
/// underwent (general for ANY portal-pair angle — see [`portal_transit_roll`]),
/// so a body leaves a portal rotated consistently with how it entered; then
/// [`update_actor_roll`] eases the roll back toward "feet along gravity" so the
/// body rights itself.
///
/// This is the shared "which way is down" orientation: the SAME component,
/// righting system ([`update_actor_roll`]), and transit math drive the player
/// and every actor, so a goblin or shark somersaults through a portal exactly
/// like the player (the unification). The [`GravityField`] it orients to is the
/// gravity-room hook.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ActorRoll {
    /// Current render-space z-rotation applied to the body's sprite.
    pub angle: f32,
}

/// Reorientation rate easing `angle` toward gravity-upright (rad/s). Visible but
/// quick — a 180° portal flip rights itself in ~0.4s as the body arcs.
const ACTOR_ROLL_SPEED: f32 = 8.0;

/// Attach an [`ActorRoll`] to every body that can travel through a portal — the
/// player plus all non-player actors (enemies / NPCs via `ActorKinematics`,
/// bosses via `BossKinematics`) — lazily, so no bundle needs to know about the
/// portal module.
pub fn ensure_actor_roll(
    mut commands: Commands,
    player: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>, Without<ActorRoll>)>,
    actors: Query<Entity, (With<crate::features::ActorKinematics>, Without<ActorRoll>)>,
    bosses: Query<Entity, (With<crate::features::BossKinematics>, Without<ActorRoll>)>,
) {
    for entity in &player {
        commands.entity(entity).insert(ActorRoll::default());
    }
    for entity in &actors {
        commands.entity(entity).insert(ActorRoll::default());
    }
    for entity in &bosses {
        commands.entity(entity).insert(ActorRoll::default());
    }
}

/// Continuously ease EVERY actor's roll toward "feet along gravity" (the
/// orient-to-gravity reflex) — player and non-player alike. Runs whether
/// airborne or grounded, so after a portal rotates a body it visibly rights
/// itself toward the current [`GravityField`]; in a gravity room it settles to
/// that room's down.
pub fn update_actor_roll(
    time: Res<crate::WorldTime>,
    gravity: Option<Res<GravityField>>,
    mut rolls: Query<&mut ActorRoll>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
    let target = gravity_upright_angle(gravity_dir);
    let max_step = ACTOR_ROLL_SPEED * dt;
    for mut roll in &mut rolls {
        // Shortest signed difference, wrapped to (-π, π], so righting always
        // takes the short way around.
        let mut diff = (target - roll.angle).rem_euclid(std::f32::consts::TAU);
        if diff > std::f32::consts::PI {
            diff -= std::f32::consts::TAU;
        }
        if diff.abs() <= max_step {
            roll.angle = target;
        } else {
            roll.angle += max_step * diff.signum();
        }
        // Keep the stored angle bounded so repeated portals don't grow it.
        roll.angle = roll.angle.rem_euclid(std::f32::consts::TAU);
    }
}

/// The render-space roll a body picks up traveling through a portal pair: the
/// signed on-screen angle its motion turns through — from "into the entry"
/// (`-n_in`) to "out of the exit" (`n_out`), measured in RENDER space (y
/// flipped). Computing it as the render-space turn directly (rather than a
/// world rotation we then conjugate) keeps the sign unambiguous. Fully general
/// for ANY two portal angles: floor↔floor = ±π, floor↔wall = ±π/2, slanted
/// pairs = whatever the normals give. A body entering feet-first leaves
/// feet-first along its new velocity.
pub fn portal_transit_roll(n_in: Vec2, n_out: Vec2) -> f32 {
    // Approach direction (-n_in) and exit direction (n_out), each flipped into
    // render space; the body turns by the signed angle between them.
    let into_render = Vec2::new(-n_in.x, n_in.y);
    let out_render = Vec2::new(n_out.x, -n_out.y);
    let dot = into_render.dot(out_render);
    let cross = into_render.x * out_render.y - into_render.y * out_render.x;
    cross.atan2(dot)
}

/// Teleport the player between linked portals, carrying momentum. Requires
/// both portals to exist; a short cooldown after each jump prevents ping-pong.
pub fn portal_teleport_system(
    time: Res<crate::WorldTime>,
    mut players: Query<
        (&mut PlayerKinematics, &mut PortalGun, Option<&mut ActorRoll>),
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
    let Ok((mut kin, mut gun, mut roll)) = players.single_mut() else {
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
            let clearance = portal_exit_clearance(kin.size * 0.5, exit.normal);
            kin.pos = exit.pos + exit.normal * clearance;
            kin.vel = out_vel;
            gun.teleport_cooldown = TELEPORT_COOLDOWN_S;
            // Aerial reorientation: roll the body by the same rotation the
            // portal pair applies to the velocity, so you leave rotated
            // consistently with how you entered (general for any pair of portal
            // angles). update_actor_roll then rights you toward gravity.
            if let Some(roll) = roll.as_deref_mut() {
                roll.angle += portal_transit_roll(enter.normal, exit.normal);
            }
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

/// Portals must not outlive the gun that made them: despawn all portals (and
/// any in-flight shots) when **no** portal gun is present in the room — neither
/// held (`PortalGun`) nor lying on the ground as a `PortalGunPickup`. This is
/// the "gun is destroyed" case. A merely *dropped* gun still exists as a pickup
/// in the room, so its portals persist; leaving the room is handled separately
/// by [`clear_portals_on_reset`].
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    guns: Query<(), With<PortalGun>>,
    pickups: Query<(), With<PortalGunPickup>>,
    portals: Query<Entity, With<Portal>>,
    shots: Query<Entity, With<PortalProjectile>>,
) {
    if !guns.is_empty() || !pickups.is_empty() {
        return;
    }
    if portals.is_empty() && shots.is_empty() {
        return;
    }
    for entity in &portals {
        commands.entity(entity).despawn();
    }
    for entity in &shots {
        commands.entity(entity).despawn();
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
                let clearance = portal_exit_clearance(item.half_extent, exit.normal);
                item.pos = exit.pos + exit.normal * clearance;
                break;
            }
        }
    }
}

/// Per-actor cooldown after a portal jump, so an actor that pops out of the
/// exit doesn't immediately re-enter and ping-pong. Inserted on teleport and
/// ticked down by [`tick_portal_cooldowns`].
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalCooldown(pub f32);

/// Tick down (and clear) per-actor [`PortalCooldown`]s.
pub fn tick_portal_cooldowns(
    time: Res<crate::WorldTime>,
    mut commands: Commands,
    mut cooldowns: Query<(Entity, &mut PortalCooldown)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut cooldown) in &mut cooldowns {
        cooldown.0 -= dt;
        if cooldown.0 <= 0.0 {
            commands.entity(entity).remove::<PortalCooldown>();
        }
    }
}

/// Does an actor of `size` fit through `portal`? The opening the actor must
/// pass through is the portal extent **perpendicular to its normal**: a wall
/// portal (horizontal normal) is a vertical doorway, so the actor's *height*
/// must fit; a floor / ceiling portal (vertical normal) gates on *width*. This
/// keeps big bosses out of small portals while staying fully general — make a
/// huge portal (or shrink the boss) and it passes.
pub fn portal_fits(size: Vec2, portal: &Portal) -> bool {
    let normal_is_horizontal = portal.normal.x.abs() >= portal.normal.y.abs();
    let (opening, cross) = if normal_is_horizontal {
        (portal.half_extent.y * 2.0, size.y)
    } else {
        (portal.half_extent.x * 2.0, size.x)
    };
    cross <= opening
}

/// Teleport non-player actors (enemies / NPCs / bosses) through the portal
/// pair, **size-gated** so only actors that fit the opening pass. Enemies / NPCs
/// (`ActorKinematics`) carry their momentum through the rotation; bosses
/// (`BossKinematics`, no velocity field) are repositioned out the exit. A short
/// [`PortalCooldown`] after each jump prevents ping-pong.
pub fn portal_teleport_actors(
    mut commands: Commands,
    portals: Query<&Portal>,
    mut actors: Query<
        (
            Entity,
            &mut crate::features::ActorKinematics,
            Option<&mut ActorRoll>,
        ),
        // Exclude bosses so this query and the boss query below have disjoint
        // entity sets — both take `&mut ActorRoll`, so Bevy needs them proven
        // non-overlapping.
        (Without<PortalCooldown>, Without<crate::features::BossKinematics>),
    >,
    mut bosses: Query<
        (
            Entity,
            &mut crate::features::BossKinematics,
            Option<&mut ActorRoll>,
        ),
        Without<PortalCooldown>,
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let blue = portals.iter().find(|p| p.color == PortalColor::Blue).copied();
    let orange = portals
        .iter()
        .find(|p| p.color == PortalColor::Orange)
        .copied();
    let (Some(blue), Some(orange)) = (blue, orange) else {
        return;
    };
    let pair = [(blue, orange), (orange, blue)];

    for (entity, mut kin, mut roll) in &mut actors {
        let aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
        for (enter, exit) in pair {
            if !portal_fits(kin.size, &enter) {
                continue;
            }
            if aabb.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent + Vec2::splat(4.0)))
            {
                kin.vel = portal_transform_velocity(kin.vel, enter.normal, exit.normal);
                let clearance = portal_exit_clearance(kin.size * 0.5, exit.normal);
                kin.pos = exit.pos + exit.normal * clearance;
                // Same aerial roll as the player (the unification): the actor
                // leaves rotated and `update_actor_roll` rights it to gravity.
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += portal_transit_roll(enter.normal, exit.normal);
                }
                commands.entity(entity).insert(PortalCooldown(TELEPORT_COOLDOWN_S));
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_ENTER,
                    pos: enter.pos,
                });
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_EXIT,
                    pos: exit.pos,
                });
                break;
            }
        }
    }

    for (entity, mut kin, mut roll) in &mut bosses {
        let aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
        for (enter, exit) in pair {
            if !portal_fits(kin.size, &enter) {
                continue;
            }
            if aabb.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent + Vec2::splat(4.0)))
            {
                let clearance = portal_exit_clearance(kin.size * 0.5, exit.normal);
                kin.pos = exit.pos + exit.normal * clearance;
                if let Some(roll) = roll.as_deref_mut() {
                    roll.angle += portal_transit_roll(enter.normal, exit.normal);
                }
                commands.entity(entity).insert(PortalCooldown(TELEPORT_COOLDOWN_S));
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_ENTER,
                    pos: enter.pos,
                });
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_EXIT,
                    pos: exit.pos,
                });
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
        let (rim, core) = match portal.color {
            PortalColor::Blue => (Color::srgb(0.30, 0.62, 1.0), Color::srgb(0.74, 0.92, 1.0)),
            PortalColor::Orange => (Color::srgb(1.0, 0.55, 0.20), Color::srgb(1.0, 0.86, 0.55)),
        };
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
            Sprite::from_color(core, Vec2::new(length * 0.86, PORTAL_VISUAL_THICKNESS * 0.42)),
            Transform::from_translation(core_translation).with_rotation(rotation),
            Name::new("Portal visual (core)"),
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
                ActionSet::default(),
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
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
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
    fn portal_fit_gate_keys_on_the_opening_perpendicular_to_the_normal() {
        let wall = Portal {
            color: PortalColor::Blue,
            pos: Vec2::ZERO,
            normal: Vec2::new(1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        };
        // The opening is the SAME size in every orientation (2*46=92). A wall
        // portal gates on HEIGHT: a short actor fits, a 200-tall boss does not.
        assert!(portal_fits(Vec2::new(24.0, 40.0), &wall));
        assert!(!portal_fits(Vec2::new(80.0, 200.0), &wall));
        // A floor portal gates on WIDTH — same 92 opening, so the threshold
        // matches the wall's.
        let floor = Portal {
            color: PortalColor::Orange,
            pos: Vec2::ZERO,
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        };
        assert!(portal_fits(Vec2::new(40.0, 200.0), &floor));
        assert!(!portal_fits(Vec2::new(100.0, 20.0), &floor));
    }

    #[test]
    fn portals_teleport_a_fitting_actor_and_skip_an_oversized_one() {
        use crate::features::ActorKinematics;
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_systems(Update, portal_teleport_actors);
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(20.0, 200.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
        let small = app
            .world_mut()
            .spawn(ActorKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(24.0, 40.0),
                facing: -1.0,
            })
            .id();
        let big = app
            .world_mut()
            .spawn(ActorKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(80.0, 200.0),
                facing: -1.0,
            })
            .id();
        app.update();
        let s = app.world().get::<ActorKinematics>(small).unwrap();
        assert!(
            s.pos.x > 250.0,
            "a fitting actor teleports out the orange portal, pos={:?}",
            s.pos
        );
        let b = app.world().get::<ActorKinematics>(big).unwrap();
        assert!(
            b.pos.x < 100.0,
            "an oversized actor does not fit and stays put, pos={:?}",
            b.pos
        );
    }

    #[test]
    fn portal_transit_roll_is_general_and_matches_on_screen_turn() {
        use std::f32::consts::{FRAC_PI_2, PI};
        let up = Vec2::new(0.0, -1.0); // floor portal faces up (y-down world)
        let down = Vec2::new(0.0, 1.0); // ceiling portal faces down
        let left = Vec2::new(-1.0, 0.0); // right wall faces left
        let right = Vec2::new(1.0, 0.0); // left wall faces right

        // Floor↔floor flips 180° (you somersault).
        assert!((portal_transit_roll(up, up).abs() - PI).abs() < 1e-5);
        // A straight-through wall pair (enter into +x wall, exit -x wall) keeps
        // orientation — no turn.
        assert!(portal_transit_roll(right, left).abs() < 1e-5);
        // Floor→right-wall: falling in, you exit moving LEFT, so the body turns
        // -90° (render) — feet swing from down to left, leaving feet-first.
        assert!((portal_transit_roll(up, left) - (-FRAC_PI_2)).abs() < 1e-5);
        // The reverse pair turns the opposite way.
        assert!((portal_transit_roll(left, up) - FRAC_PI_2).abs() < 1e-5);
        // Ceiling↔ceiling also flips 180°.
        assert!((portal_transit_roll(down, down).abs() - PI).abs() < 1e-5);
    }

    #[test]
    fn roll_eases_back_to_gravity_upright_in_air() {
        let mut app = App::new();
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.init_resource::<GravityField>();
        app.add_systems(Update, update_actor_roll);
        // Start rolled 180° (just exited a floor↔floor portal), airborne.
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                ActorRoll {
                    angle: std::f32::consts::PI,
                },
            ))
            .id();
        // It rights itself toward gravity-upright (0) over time WITHOUT needing
        // to be grounded (the orient-to-gravity reflex).
        for _ in 0..120 {
            app.update();
        }
        let angle = app.world().get::<ActorRoll>(player).unwrap().angle;
        let from_upright = angle.min(std::f32::consts::TAU - angle); // distance to 0 mod 2π
        assert!(from_upright < 1e-2, "should right itself to gravity-up, got {angle}");
    }

    #[test]
    fn gravity_upright_angle_tracks_the_gravity_direction() {
        use std::f32::consts::FRAC_PI_2;
        // Default gravity (down, +Y world) → upright is 0.
        assert!(gravity_upright_angle(Vec2::new(0.0, 1.0)).abs() < 1e-5);
        // Gravity to the right (+X) → the body stands rotated +90° (render).
        assert!((gravity_upright_angle(Vec2::new(1.0, 0.0)) - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn actors_get_the_same_aerial_roll_through_portals_as_the_player() {
        use crate::features::ActorKinematics;
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_systems(Update, portal_teleport_actors);
        // Floor portal (normal up) + right-wall portal (normal left): a
        // floor→wall pair, so transit imparts a -90° roll — the SAME value
        // portal_transit_roll gives the player.
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(200.0, 380.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(-1.0, 0.0)),
        });
        let actor = app
            .world_mut()
            .spawn((
                ActorKinematics {
                    pos: Vec2::new(200.0, 380.0),
                    vel: Vec2::new(0.0, 100.0),
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ActorRoll::default(),
            ))
            .id();
        app.update();
        let roll = app.world().get::<ActorRoll>(actor).unwrap().angle;
        let expected = portal_transit_roll(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0))
            .rem_euclid(std::f32::consts::TAU);
        assert!(
            (roll.rem_euclid(std::f32::consts::TAU) - expected).abs() < 1e-4,
            "a teleported actor should pick up the same aerial roll as the player; got {roll}, expected {expected}"
        );
    }

    #[test]
    fn gravity_switch_flips_on_entry_and_rearms_on_exit() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.init_resource::<GravityField>();
        app.init_resource::<crate::physics::BaseGravity>();
        app.add_systems(Update, gravity_flip_switch_system);
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);
        app.world_mut().spawn(GravityFlipSwitch {
            pos: Vec2::new(400.0, 100.0),
            half_extent: Vec2::new(16.0, 220.0),
            armed: true,
        });

        // Not overlapping → gravity stays down.
        app.update();
        assert!(app.world().resource::<crate::physics::BaseGravity>().dir.y > 0.0, "starts down");

        // Step onto the switch → flips up.
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<crate::physics::BaseGravity>().dir.y < 0.0,
            "stepping on the switch flips ambient gravity up"
        );
        // Staying on it does not re-flip (latched).
        app.update();
        assert!(app.world().resource::<crate::physics::BaseGravity>().dir.y < 0.0, "stays flipped while on it");

        // Leave, then re-enter → flips back down.
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = Vec2::new(100.0, 100.0);
        app.update();
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<crate::physics::BaseGravity>().dir.y > 0.0,
            "re-entering flips ambient gravity back down"
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
                ActionSet::default(),
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
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(378.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
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
