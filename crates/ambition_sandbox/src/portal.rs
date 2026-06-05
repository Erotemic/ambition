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
use crate::portal_pieces::{self as pp, PortalFrame};
use crate::GameWorld;

/// A portal's color. Portals are linked into PAIRS by complementary color (one
/// of each), so several independent pairs can exist at once: the gun fires the
/// **Blue↔Orange** pair, and authored test rooms place other pairs
/// (Purple↔Yellow, Teal↔Red, Green↔Magenta) so it's clear at a glance which two
/// portals are linked. [`partner`](Self::partner) gives the linked color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalColor {
    Blue,
    Orange,
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
}

impl PortalColor {
    /// The complementary color this portal is linked to (its pair partner).
    pub fn partner(self) -> Self {
        use PortalColor::*;
        match self {
            Blue => Orange,
            Orange => Blue,
            Purple => Yellow,
            Yellow => Purple,
            Teal => Red,
            Red => Teal,
            Green => Magenta,
            Magenta => Green,
            Cyan => Rose,
            Rose => Cyan,
        }
    }

    /// Back-compat alias for [`partner`](Self::partner) (the gun's blue↔orange toggle).
    pub fn other(self) -> Self {
        self.partner()
    }

    /// True for the gun's pair — the only one the portal gun fires / owns, so the
    /// only one that despawns when the gun is gone. Authored pairs persist.
    pub fn is_gun_pair(self) -> bool {
        matches!(self, PortalColor::Blue | PortalColor::Orange)
    }

    /// `(rim, core)` display colors for the portal bar — partners are visibly
    /// complementary so a linked pair reads as a pair.
    pub fn display(self) -> (Color, Color) {
        use PortalColor::*;
        match self {
            Blue => (Color::srgb(0.30, 0.62, 1.0), Color::srgb(0.74, 0.92, 1.0)),
            Orange => (Color::srgb(1.0, 0.55, 0.20), Color::srgb(1.0, 0.86, 0.55)),
            Purple => (Color::srgb(0.55, 0.30, 0.95), Color::srgb(0.82, 0.66, 1.0)),
            Yellow => (Color::srgb(0.95, 0.85, 0.18), Color::srgb(1.0, 0.96, 0.66)),
            Teal => (Color::srgb(0.13, 0.76, 0.70), Color::srgb(0.64, 0.96, 0.92)),
            Red => (Color::srgb(0.92, 0.22, 0.25), Color::srgb(1.0, 0.62, 0.62)),
            Green => (Color::srgb(0.28, 0.80, 0.35), Color::srgb(0.72, 0.96, 0.74)),
            Magenta => (Color::srgb(0.92, 0.25, 0.80), Color::srgb(1.0, 0.70, 0.95)),
            Cyan => (Color::srgb(0.18, 0.92, 0.95), Color::srgb(0.70, 0.99, 1.0)),
            Rose => (Color::srgb(1.0, 0.40, 0.62), Color::srgb(1.0, 0.74, 0.84)),
        }
    }

    /// Lowercase name, used in logs and as the LDtk authoring token.
    pub fn name(self) -> &'static str {
        use PortalColor::*;
        match self {
            Blue => "blue",
            Orange => "orange",
            Purple => "purple",
            Yellow => "yellow",
            Teal => "teal",
            Red => "red",
            Green => "green",
            Magenta => "magenta",
            Cyan => "cyan",
            Rose => "rose",
        }
    }

    /// Parse a color from its [`name`](Self::name) (LDtk authoring). Case-insensitive.
    pub fn from_name(s: &str) -> Option<Self> {
        use PortalColor::*;
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "blue" => Blue,
            "orange" => Orange,
            "purple" => Purple,
            "yellow" => Yellow,
            "teal" => Teal,
            "red" => Red,
            "green" => Green,
            "magenta" => Magenta,
            "cyan" => Cyan,
            "rose" => Rose,
            _ => return None,
        })
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

impl Portal {
    /// The pure-geometry frame this portal presents to [`crate::portal_pieces`]
    /// (the Core invariant math: piece decomposition, carve, portal map).
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            pos: self.pos,
            normal: self.normal,
            half_extent: self.half_extent,
        }
    }
}

/// The placed portal of `color`, if any.
fn find_portal<'a>(
    portals: impl IntoIterator<Item = &'a Portal>,
    color: PortalColor,
) -> Option<Portal> {
    portals.into_iter().find(|p| p.color == color).copied()
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
    include_one_way: bool,
) -> Option<(Vec2, Vec2)> {
    let dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut best_t = max_dist;
    let mut best_normal = Vec2::ZERO;
    for block in &world.blocks {
        // Portals adhere to one-way platforms too (#39); blink/dive pass through
        // them, so they leave `include_one_way` off.
        let hittable = matches!(block.kind, ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. })
            || (include_one_way && matches!(block.kind, ae::BlockKind::OneWay));
        if !hittable {
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

/// Recursive, portal-aware raycast: cast from `origin` along `dir`, and if the
/// ray crosses a portal face (entering from its front) before hitting a solid,
/// transform the remaining ray through the linked portal and continue — so line
/// of sight, beams, grapples, and aim traces "see through" a portal pair. The
/// returned `(hit, normal)` is in the chart where the ray finally lands. Bounded
/// by `max_depth` so two portals facing each other can't loop forever.
pub fn raycast_through_portals(
    world: &ae::World,
    portals: &[Portal],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    max_depth: u32,
) -> Option<(Vec2, Vec2)> {
    let mut origin = origin;
    let mut dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut budget = max_dist;
    for _ in 0..=max_depth {
        let solid = raycast_solids(world, origin, dir, budget, include_one_way);
        let solid_t = solid
            .map(|(hit, _)| (hit - origin).length())
            .unwrap_or(f32::INFINITY);
        // Nearest portal face the ray ENTERS (front side) before that solid —
        // across ALL placed pairs, each portal redirecting to its partner.
        let mut nearest: Option<(f32, Portal, Portal)> = None;
        for enter in portals {
            let Some(exit) = find_portal(portals, enter.color.partner()) else {
                continue;
            };
            // Only enter through the front of the face (moving into it).
            if dir.dot(enter.normal) >= 0.0 {
                continue;
            }
            if let Some((t, _)) =
                ray_aabb(origin, dir, ae::Aabb::new(enter.pos, enter.half_extent))
            {
                if t <= budget && t < solid_t && nearest.map_or(true, |(bt, _, _)| t < bt) {
                    nearest = Some((t, *enter, exit));
                }
            }
        }
        match nearest {
            Some((t, enter, exit)) => {
                let entry = origin + dir * t;
                // Emerge just out of the exit face, redirected through the pair.
                origin = pp::map_point(entry, &enter.frame(), &exit.frame()) + exit.normal;
                dir = pp::rotate(dir, pp::portal_rotation(enter.normal, exit.normal))
                    .normalize_or_zero();
                budget -= t;
                if budget <= 0.0 || dir == Vec2::ZERO {
                    return None;
                }
            }
            None => return solid,
        }
    }
    None
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

// The portal gun is now an LDtk-authored `PortalGunSpawn` entity (spawned at
// room load via `spawn_room_feature_entities`); the old debug near-player
// spawner is retired.

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
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
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
            // Reflect the portal gun into the 24-item catalog so the OoT menu
            // shows it as owned + equipped.
            if let Some(owned) = owned.as_deref_mut() {
                owned.grant(crate::items::Item::PortalGun, 1);
                owned.set_equipped(Some(crate::items::Item::PortalGun));
            }
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

/// Equip the portal gun onto the player from a non-pickup source (the inventory
/// menu): stash the action set, attach an active [`PortalGun`], and clear the
/// melee swing so `Attack` fires portals (the same replacement the world pickup
/// does). Mirrors [`pickup_portal_gun_system`] minus the ground entity.
pub fn equip_portal_gun(commands: &mut Commands, player: Entity, action_set: &mut ActionSet) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    commands.entity(player).insert(PortalGun {
        active: true,
        ..PortalGun::default()
    });
    action_set.melee = None;
}

/// Detach the portal gun and restore the stashed action set (inventory unequip).
pub fn unequip_portal_gun(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<PortalGun>();
    commands.entity(player).remove::<StashedActionSet>();
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
        if let Some((hit, normal)) = raycast_solids(&world.0, proj.pos, proj.vel, step, true) {
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
                Name::new(format!("Portal: {}", proj.color.name())),
                // Portals are per-room: a room transition despawns them, so they
                // don't linger and reappear when you leave and come back (#41).
                crate::presentation::rendering::RoomScopedEntity,
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

// The hub gravity flip is now an LDtk-authored `Switch` whose `action` is
// "FlipGravity" (handled in `encounter::systems::update_encounters_from_world`),
// so the old debug-spawned overlap column is gone. The `GravityFlipSwitch`
// component + `gravity_flip_switch_system` below remain only for the unit test
// + any future overlap-style gravity plate; nothing spawns one in-game.

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

// Localized gravity zones are now LDtk-authored `GravityZone` entities (spawned
// at room load via `spawn_room_feature_entities`), including the sliding column
// -- a `oscillate_amplitude > 0` field attaches an `OscillatingZone`. The old
// debug `spawn_debug_gravity_zone_once` is retired.

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
    gravity: crate::physics::GravityCtx,
    mut rolls: Query<(
        &mut ActorRoll,
        Option<&PlayerKinematics>,
        Option<&crate::features::ActorKinematics>,
        Option<&crate::features::BossKinematics>,
    )>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let max_step = ACTOR_ROLL_SPEED * dt;
    for (mut roll, pkin, akin, bkin) in &mut rolls {
        // Each body rights toward the gravity of the column IT is standing in
        // (localized): resolve from its own position, falling back to the
        // player's field when position is unavailable.
        let pos = pkin
            .map(|k| k.pos)
            .or_else(|| akin.map(|k| k.pos))
            .or_else(|| bkin.map(|k| k.pos));
        let gravity_dir = match pos {
            Some(p) => gravity.dir_at(p),
            None => gravity.field_dir(),
        };
        let target = gravity_upright_angle(gravity_dir);
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

/// Per-player transit state: the aperture latch / centroid-crossing machine
/// that replaces "touch = teleport". A body is mid-transit while any part of it
/// straddles a portal plane; the authoritative body transfers to the exit when
/// the CENTROID crosses, and transit ends (re-arming after a clear) once the
/// body fully clears the plane.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransit {
    /// Color of the portal whose plane the body currently straddles — the entry
    /// before the centroid crosses, the exit after.
    pub straddling: PortalColor,
    /// True once the centroid crossed the entry plane (authoritative body now
    /// on the exit side).
    pub crossed: bool,
}

/// Carve placed-portal apertures out of the host surface — but ONLY a portal a
/// transiting body currently occupies (its `PortalTransit.straddling`), so the
/// opening exists exactly while a body is passing through and re-seals the
/// instant it clears. A permanently-carved portal left a walk-in pocket in the
/// host wall (you could wiggle into the solid wall / ledge-grab the carved
/// edges); gating the carve on active transit closes that. Pair-gated — a lone
/// portal never carves.
pub fn publish_portal_carves(
    portals: Query<&Portal>,
    transits: Query<&PortalTransit>,
    mut overlay: ResMut<crate::features::FeatureEcsWorldOverlay>,
) {
    overlay.portal_carves.clear();
    let all: Vec<Portal> = portals.iter().copied().collect();
    // Carve each portal a body is actively transiting (deduped), but only if its
    // pair partner is placed — a lone portal must never open a bottomless hole.
    let mut carved: Vec<PortalColor> = Vec::new();
    for t in &transits {
        if carved.contains(&t.straddling) {
            continue;
        }
        let Some(enter) = find_portal(&all, t.straddling) else {
            continue;
        };
        if find_portal(&all, t.straddling.partner()).is_none() {
            continue;
        }
        overlay.portal_carves.push(pp::carve_hole(&enter.frame()));
        carved.push(t.straddling);
    }
}

/// Margin (px) added to a portal's thin face so a body resting against the
/// surface registers as "entering" before it has visibly sunk in (the carve
/// only opens once transit has begun, so begin must trigger on contact).
const TRANSIT_BEGIN_MARGIN: f32 = 6.0;

/// One step of the aperture / centroid-crossing transit machine for ANY body.
/// Pure: given the body's geometry + current transit/cooldown state + the portal
/// pair, it returns the action the caller applies. Shared by the player and
/// every non-player actor so they all cross a portal identically (the
/// unification the design calls for).
#[derive(Clone, Copy, Debug)]
pub enum TransitStep {
    /// Not touching a portal (or latched) — do nothing.
    Idle,
    /// Begin transit into this portal: insert [`PortalTransit`], play ENTER sfx.
    Begin { color: PortalColor, portal_pos: Vec2 },
    /// The centroid crossed: move the body to `pos`, set velocity `vel`, add
    /// `roll_delta` to its roll (the somersault), latch the cooldown, flip the
    /// straddled portal to `exit_color`, mark crossed, play EXIT sfx. `warp_rot`
    /// is the `(cos, sin)` portal map (same rotation applied to velocity) — the
    /// player layer warps the held movement input by it so the held direction
    /// keeps carrying the body OUT instead of fighting the warped velocity.
    Transfer {
        pos: Vec2,
        vel: Vec2,
        roll_delta: f32,
        warp_rot: (f32, f32),
        /// Mirror the body's horizontal facing (the wall↔wall "face out" rule).
        /// Also the gate for the held-input warp: it's exactly the case where
        /// warping held movement stays horizontally expressible.
        facing_flip: bool,
        /// Outward normal of the exit portal — the direction the body emerges.
        /// Used by emission protection so held input can't cancel the emergence.
        exit_normal: Vec2,
        exit_color: PortalColor,
        exit_pos: Vec2,
    },
    /// The body fully cleared the plane — remove [`PortalTransit`].
    Clear,
    /// Mid-transit, nothing to apply this frame.
    Continue,
}

/// The somersault roll a body picks up crossing a portal pair. It is the
/// on-screen turn from [`portal_transit_roll`] — EXCEPT a pure turn-around in
/// the gravity-perpendicular plane (wall↔wall under normal gravity) imparts NO
/// tumble: the body stays gravity-upright and just reverses facing, so it comes
/// out the far wall already correctly oriented. Crossing a floor / ceiling
/// (normal along gravity) keeps the genuine tumble (feet-in → reorient).
pub fn somersault_roll(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> f32 {
    let g = gravity_dir.normalize_or_zero();
    // A portal whose normal is perpendicular to gravity sits on a wall; the body
    // enters/leaves it moving horizontally, so the transit is a turn-around, not
    // a tumble.
    let in_wall = n_in.normalize_or_zero().dot(g).abs() < 0.5;
    let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
    if in_wall && out_wall {
        return 0.0;
    }
    portal_transit_roll(n_in, n_out)
}

/// Whether the body's horizontal FACING flips through this portal pair.
///
/// A 180° somersault rotation inherently mirrors the sprite left↔right. For a
/// wall↔wall turn-around we SUPPRESS that rotation (to keep the body upright —
/// see [`somersault_roll`]), which would lose the mirror and emerge the body
/// back-first ("face in, back out"). So in exactly that suppressed-180° case the
/// mirror is re-applied as a facing flip, giving the wanted "face in, face out"
/// (really: X-in, X-out). Every other case carries its orientation in the
/// rotation, so facing is left alone.
pub fn portal_facing_flips(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> bool {
    let g = gravity_dir.normalize_or_zero();
    let in_wall = n_in.normalize_or_zero().dot(g).abs() < 0.5;
    let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
    // Suppressed (both walls) AND the would-be turn is a ~180° flip (same-wall),
    // not a 0° straight-through (facing-each-other walls).
    in_wall && out_wall && portal_transit_roll(n_in, n_out).abs() > std::f32::consts::FRAC_PI_2
}

/// Compute the transit step for a body. See [`TransitStep`]. `cooldown` is the
/// body's post-jump latch (player gun cooldown / actor [`PortalCooldown`]);
/// `gravity_dir` selects whether a transit tumbles or just turns around.
pub fn transit_step(
    center: Vec2,
    size: Vec2,
    vel: Vec2,
    transit: Option<PortalTransit>,
    cooldown: f32,
    portals: &[Portal],
    gravity_dir: Vec2,
) -> TransitStep {
    let body = ae::Aabb::new(center, size * 0.5);
    // Resolve `(straddled, its linked exit)` for a color — both must be placed.
    let pair_for = |c: PortalColor| -> Option<(Portal, Portal)> {
        Some((find_portal(portals, c)?, find_portal(portals, c.partner())?))
    };
    match transit {
        None => {
            if cooldown > 0.0 {
                return TransitStep::Idle;
            }
            // Begin into the first portal (across ALL pairs) the body is entering.
            for enter in portals {
                // Need the partner placed, or there's no exit to transit to.
                if find_portal(portals, enter.color.partner()).is_none() {
                    continue;
                }
                if !portal_fits(size, enter) {
                    continue;
                }
                let frame = enter.frame();
                // Begin when the leading face reaches the opening, from the front
                // (centroid in front of the plane, or moving into it). The
                // capture box is the thin face plus a small margin; its
                // along-surface span is the opening, so this also gates laterally.
                let capture =
                    ae::Aabb::new(enter.pos, enter.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN));
                let entering =
                    pp::front_distance(center, &frame) > 0.0 || vel.dot(enter.normal) < 0.0;
                if entering && body.strict_intersects(capture) {
                    return TransitStep::Begin { color: enter.color, portal_pos: enter.pos };
                }
            }
            TransitStep::Idle
        }
        Some(t) => {
            // The straddled portal or its partner was removed → end transit.
            let Some((enter, exit)) = pair_for(t.straddling) else {
                return TransitStep::Clear;
            };
            let ef = enter.frame();
            // The CENTROID crossing the plane is the authoritative transfer —
            // the body jumps to the exit; gameplay sees no discontinuity because
            // every query uses the portal pieces.
            if !t.crossed && pp::front_distance(center, &ef) <= 0.0 {
                let xf = exit.frame();
                let mut vel_out = portal_transform_velocity(vel, enter.normal, exit.normal);
                // Floor the exit speed along the exit normal so a slow walk-in
                // still emerges instead of stalling in the opening.
                if vel_out.dot(exit.normal) < MIN_EXIT_SPEED {
                    let tangential = vel_out - vel_out.dot(exit.normal) * exit.normal;
                    vel_out = tangential + exit.normal * MIN_EXIT_SPEED;
                }
                return TransitStep::Transfer {
                    pos: pp::map_point(center, &ef, &xf),
                    vel: vel_out,
                    // The body picks up the on-screen turn it travels through
                    // (a tumble for floor/ceiling, nothing for a wall↔wall
                    // turn-around); `update_actor_roll` then eases it back to
                    // gravity-upright (feet-in → reorient).
                    roll_delta: somersault_roll(enter.normal, exit.normal, gravity_dir),
                    // Same rotation the velocity took — the held-input warp uses it.
                    warp_rot: pp::portal_rotation(enter.normal, exit.normal),
                    facing_flip: portal_facing_flips(enter.normal, exit.normal, gravity_dir),
                    exit_normal: exit.normal,
                    exit_color: exit.color,
                    exit_pos: exit.pos,
                };
            }
            // Stay engaged so the carve persists long enough to sink + cross —
            // clearing on "not straddling yet" would drop the carve every other
            // frame and the body would never sink in (it re-grounds on the solid
            // frame). Before the centroid crosses, stay while the body still
            // touches the opening (the capture box); after, stay while it still
            // straddles the exit plane (trailing edge not yet out). The cooldown
            // latch (set on transfer) stops a re-entry.
            let still_engaged = if t.crossed {
                pp::straddles(body, &enter.frame())
            } else {
                let capture = ae::Aabb::new(
                    enter.pos,
                    enter.half_extent + Vec2::splat(TRANSIT_BEGIN_MARGIN),
                );
                body.strict_intersects(capture)
            };
            if still_engaged {
                TransitStep::Continue
            } else {
                TransitStep::Clear
            }
        }
    }
}

/// Holds the player's real `ledge_grab` ability while it's suppressed during a
/// portal transit, so nothing is lost when transit ends.
#[derive(Component, Clone, Copy, Debug)]
pub struct LedgeGrabSuppressed(pub bool);

/// While the player is mid-transit, suppress ledge-grab so they don't latch onto
/// the carved aperture EDGES (the carve splits the host block, and those new
/// edges read as grabbable ledges — you'd grab "into" the portal and pop back out
/// the entry instead of crossing). The real ability is saved and restored, so the
/// player keeps ledge-grab everywhere else. Runs before the movement integration.
pub fn suppress_ledge_grab_during_transit(
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut crate::player::PlayerAbilities,
            Option<&PortalTransit>,
            Option<&LedgeGrabSuppressed>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    for (entity, mut abilities, transiting, saved) in &mut players {
        match (transiting.is_some(), saved) {
            (true, None) => {
                commands.entity(entity).insert(LedgeGrabSuppressed(abilities.abilities.ledge_grab));
                abilities.abilities.ledge_grab = false;
            }
            (false, Some(saved)) => {
                abilities.abilities.ledge_grab = saved.0;
                commands.entity(entity).remove::<LedgeGrabSuppressed>();
            }
            _ => {}
        }
    }
}

/// Drive the PLAYER through a portal as an **aperture**, not a trigger, via the
/// shared [`transit_step`] machine: the body physically sinks into the carved
/// opening (the movement integrator does that), transfers when the centroid
/// crosses (carrying momentum + a somersault roll), and clears on trailing-edge
/// out. The transfer's position snap is flagged as an intentional teleport so
/// the trace detector doesn't auto-dump on it.
pub fn portal_transit_system(
    time: Res<crate::WorldTime>,
    control: Option<Res<ControlFrame>>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut PlayerKinematics,
            // The gun is OPTIONAL: authored portals (the test lab) work without
            // ever picking it up. It only carries the anti-ping-pong cooldown; a
            // `PortalCooldown` component is the gun-less fallback latch.
            Option<&mut PortalGun>,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalCooldown>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    portals: Query<&Portal>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut intentional: Option<ResMut<IntentionalTeleport>>,
) {
    // Default to "no teleport this frame"; set true below on a centroid transfer.
    if let Some(flag) = intentional.as_deref_mut() {
        flag.0 = false;
    }
    let dt = time.sim_dt();
    let Ok((entity, mut kin, mut gun, mut transit, mut roll, cooldown)) = players.single_mut()
    else {
        return;
    };
    if let Some(gun) = gun.as_deref_mut() {
        if gun.teleport_cooldown > 0.0 {
            gun.teleport_cooldown = (gun.teleport_cooldown - dt).max(0.0);
        }
    }
    // Latch from the gun (if held) OR the fallback `PortalCooldown` component.
    let cooldown_now = gun
        .as_deref()
        .map_or(0.0, |g| g.teleport_cooldown)
        .max(cooldown.map_or(0.0, |c| c.0));
    let all: Vec<Portal> = portals.iter().copied().collect();
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
    let step = transit_step(
        kin.pos,
        kin.size,
        kin.vel,
        transit.as_deref().copied(),
        cooldown_now,
        &all,
        gravity_dir,
    );
    match step {
        TransitStep::Idle | TransitStep::Continue => {}
        TransitStep::Begin { color, portal_pos } => {
            commands.entity(entity).insert(PortalTransit { straddling: color, crossed: false });
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ENTER,
                pos: portal_pos,
            });
        }
        TransitStep::Transfer {
            pos, vel, roll_delta, warp_rot, facing_flip, exit_normal, exit_color, exit_pos,
        } => {
            kin.pos = pos;
            kin.vel = vel;
            if facing_flip {
                kin.facing = -kin.facing;
            }
            if let Some(roll) = roll.as_deref_mut() {
                roll.angle += roll_delta;
            }
            if let Some(gun) = gun.as_deref_mut() {
                gun.teleport_cooldown = TELEPORT_COOLDOWN_S;
            }
            // Always set the component latch too, so a gun-less player can't
            // ping-pong through an authored pair.
            commands.entity(entity).insert(PortalCooldown(TELEPORT_COOLDOWN_S));
            // Protect the emergence: for a short window the held input can't push
            // back INTO the exit wall (so physics carries the body out — Jon's
            // "don't let input cancel the portal emission").
            commands
                .entity(entity)
                .insert(PortalEmission { exit_normal, timer: PORTAL_EMISSION_TIME });
            // Warp the held input ONLY when the warped direction stays
            // horizontally expressible — i.e. the same-wall turn-around
            // (`facing_flip`). For a floor↔wall 90° turn the warp would rotate a
            // horizontal hold into "up", which the controller can't use, so we
            // skip it and let the emission guard + physics do the work.
            let held = control
                .as_deref()
                .map_or(Vec2::ZERO, |c| Vec2::new(c.axis_x, c.axis_y));
            if facing_flip && held.length() > PORTAL_INPUT_HELD_EPS {
                commands
                    .entity(entity)
                    .insert(PortalInputWarp { rot: warp_rot, anchor: held });
            }
            if let Some(t) = transit.as_deref_mut() {
                t.crossed = true;
                t.straddling = exit_color;
            }
            if let Some(flag) = intentional.as_deref_mut() {
                flag.0 = true;
            }
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_EXIT,
                pos: exit_pos,
            });
            bevy::log::info!(target: "ambition::portal", "transferred through the portal pair");
        }
        TransitStep::Clear => {
            commands.entity(entity).remove::<PortalTransit>();
        }
    }
}

/// Movement-axis magnitude above which input counts as "held" (stick deadzone).
const PORTAL_INPUT_HELD_EPS: f32 = 0.25;
/// While warped, the live raw input may drift this far (cosine) from the anchor
/// before it counts as a "clearly different" direction that drops the warp.
const PORTAL_INPUT_WARP_KEEP_COS: f32 = 0.5;

/// Seconds the [`PortalEmission`] guard protects a fresh exit. Long enough for
/// the floored exit velocity to carry the body clear of the opening.
const PORTAL_EMISSION_TIME: f32 = 0.18;

/// The input-layer fix for the same-wall ping-pong: after a portal crossing the
/// player's HELD movement input is warped by the same portal map as velocity, so
/// holding "right" into a left-facing pair keeps carrying you LEFT out the exit
/// instead of instantly fighting the warped velocity and pulling you back through.
/// Only set for the wall↔wall turn-around (where the warp stays horizontally
/// expressible). Soft, not a hard latch — see [`warp_portal_input`].
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalInputWarp {
    /// `(cos, sin)` portal-map rotation applied to the movement axis.
    pub rot: (f32, f32),
    /// Raw (un-warped) movement direction held when the warp was set; the warp
    /// drops once the live raw input releases or clearly diverges from this.
    pub anchor: Vec2,
}

/// Short-lived guard set on every crossing: for [`PORTAL_EMISSION_TIME`] the held
/// movement input cannot push back INTO the exit wall (against `exit_normal`), so
/// the floored exit velocity carries the body out instead of the input cancelling
/// the emergence. Gravity-general — it works off the exit normal vector, not a
/// hard-coded axis.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalEmission {
    /// Outward normal of the exit portal (the emergence direction).
    pub exit_normal: Vec2,
    /// Remaining protection time (s).
    pub timer: f32,
}

/// Apply the active portal input effects at the input layer (so the player brain
/// / movement see the adjusted `ControlFrame`): the same-wall held-input warp
/// (soft — drops on release or a clearly different direction) and the emergence
/// guard (held input can't push back into the exit wall while it's fresh). Both
/// are deliberately mild so portals never feel like a hard input latch.
pub fn warp_portal_input(
    time: Option<Res<crate::WorldTime>>,
    mut commands: Commands,
    mut control: ResMut<ControlFrame>,
    mut player: Query<
        (Entity, Option<&PortalInputWarp>, Option<&mut PortalEmission>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    let Ok((entity, warp, emission)) = player.single_mut() else {
        return;
    };

    // --- Same-wall held-input warp ---
    if let Some(warp) = warp {
        let raw = Vec2::new(control.axis_x, control.axis_y);
        if raw.length() < PORTAL_INPUT_HELD_EPS {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else if warp.anchor.length() > 0.01
            && raw.normalize_or_zero().dot(warp.anchor.normalize_or_zero())
                < PORTAL_INPUT_WARP_KEEP_COS
        {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else {
            let warped = pp::rotate(raw, warp.rot);
            control.axis_x = warped.x;
            control.axis_y = warped.y;
        }
    }

    // --- Emergence guard: strip any held input that pushes back into the wall ---
    if let Some(mut emission) = emission {
        emission.timer -= time.as_deref().map_or(0.0, |t| t.sim_dt());
        if emission.timer <= 0.0 {
            commands.entity(entity).remove::<PortalEmission>();
        } else {
            let raw = Vec2::new(control.axis_x, control.axis_y);
            let into = raw.dot(emission.exit_normal); // < 0 = pushing into the wall
            if into < 0.0 {
                let kept = raw - into * emission.exit_normal;
                control.axis_x = kept.x;
                control.axis_y = kept.y;
            }
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

/// The GUN's portals must not outlive the gun that made them: despawn the
/// gun-pair portals (blue/orange) + in-flight shots when **no** portal gun is
/// present in the room — neither held (`PortalGun`) nor lying as a
/// `PortalGunPickup`. This is the "gun is destroyed" case. Authored pairs (other
/// colors, e.g. a test room's portals) are NOT gun-owned, so they persist even
/// with no gun around. A merely *dropped* gun still exists as a pickup, so its
/// portals persist; leaving the room is handled by [`clear_portals_on_reset`].
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    guns: Query<(), With<PortalGun>>,
    pickups: Query<(), With<PortalGunPickup>>,
    portals: Query<(Entity, &Portal)>,
    shots: Query<Entity, With<PortalProjectile>>,
) {
    if !guns.is_empty() || !pickups.is_empty() {
        return;
    }
    for (entity, portal) in &portals {
        if portal.color.is_gun_pair() {
            commands.entity(entity).despawn();
        }
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
/// Send EVERY non-player actor (enemies / NPCs via `ActorKinematics`, bosses via
/// `BossKinematics`) through a portal with the SAME aperture / centroid-crossing
/// machine the player uses ([`transit_step`]) — the unification: a goblin or a
/// boss now sinks into the carved opening and transfers when its centroid
/// crosses, carrying momentum + a somersault roll, instead of instant-popping
/// out the far side. Size-gated (big bosses can't fit a small opening) and
/// latched by [`PortalCooldown`] against ping-pong.
pub fn portal_transit_actors(
    mut commands: Commands,
    portals: Query<&Portal>,
    mut actors: Query<
        (
            Entity,
            &mut crate::features::ActorKinematics,
            Option<&mut PortalTransit>,
            Option<&mut ActorRoll>,
            Option<&PortalCooldown>,
        ),
        // Exclude bosses so this query and the boss query below have disjoint
        // entity sets — both take `&mut ActorRoll` / `&mut PortalTransit`, so
        // Bevy needs them proven non-overlapping.
        Without<crate::features::BossKinematics>,
    >,
    mut bosses: Query<(
        Entity,
        &mut crate::features::BossKinematics,
        Option<&mut PortalTransit>,
        Option<&mut ActorRoll>,
        Option<&PortalCooldown>,
    )>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let all: Vec<Portal> = portals.iter().copied().collect();
    if all.is_empty() {
        return;
    }
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);

    for (entity, kin, mut transit, mut roll, cooldown) in &mut actors {
        let kin = kin.into_inner();
        let step = transit_step(
            kin.pos,
            kin.size,
            kin.vel,
            transit.as_deref().copied(),
            cooldown.map_or(0.0, |c| c.0),
            &all,
            gravity_dir,
        );
        apply_actor_transit(
            &mut commands,
            entity,
            &mut sfx,
            step,
            Some(&mut kin.pos),
            Some(&mut kin.vel),
            roll.as_deref_mut(),
            transit.as_deref_mut(),
        );
    }

    for (entity, kin, mut transit, mut roll, cooldown) in &mut bosses {
        // Bosses have no velocity field — feed ZERO and ignore the velocity the
        // step computes; they reposition + roll only.
        let kin = kin.into_inner();
        let step = transit_step(
            kin.pos,
            kin.size,
            Vec2::ZERO,
            transit.as_deref().copied(),
            cooldown.map_or(0.0, |c| c.0),
            &all,
            gravity_dir,
        );
        apply_actor_transit(
            &mut commands,
            entity,
            &mut sfx,
            step,
            Some(&mut kin.pos),
            None,
            roll.as_deref_mut(),
            transit.as_deref_mut(),
        );
    }
}

/// Apply a [`TransitStep`] to a non-player actor: the ECS-side of the shared
/// machine (insert / mutate / remove `PortalTransit`, latch `PortalCooldown`,
/// move the body, somersault its roll, play sfx). `vel` is `None` for bodies
/// without a velocity field (bosses).
#[allow(clippy::too_many_arguments)]
fn apply_actor_transit(
    commands: &mut Commands,
    entity: Entity,
    sfx: &mut MessageWriter<crate::audio::SfxMessage>,
    step: TransitStep,
    pos: Option<&mut Vec2>,
    vel: Option<&mut Vec2>,
    roll: Option<&mut ActorRoll>,
    transit: Option<&mut PortalTransit>,
) {
    match step {
        TransitStep::Idle | TransitStep::Continue => {}
        TransitStep::Begin { color, portal_pos } => {
            commands.entity(entity).insert(PortalTransit { straddling: color, crossed: false });
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_ENTER,
                pos: portal_pos,
            });
        }
        // Non-player actors have no held input to warp, so `warp_rot` is ignored.
        // (`facing_flip` is too — actor facing follows their own AI each tick.)
        TransitStep::Transfer { pos: new_pos, vel: new_vel, roll_delta, exit_color, exit_pos, .. } => {
            if let Some(pos) = pos {
                *pos = new_pos;
            }
            if let Some(vel) = vel {
                *vel = new_vel;
            }
            if let Some(roll) = roll {
                roll.angle += roll_delta;
            }
            if let Some(t) = transit {
                t.crossed = true;
                t.straddling = exit_color;
            }
            commands.entity(entity).insert(PortalCooldown(TELEPORT_COOLDOWN_S));
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_EXIT,
                pos: exit_pos,
            });
        }
        TransitStep::Clear => {
            commands.entity(entity).remove::<PortalTransit>();
        }
    }
}

// ---------------------------------------------------------------------------
// Presentation (visible build only — registered by the presentation plugin).

/// Marks a sprite entity that visualizes a [`Portal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

/// Marks a transient sprite drawing one portal-aware spatial piece of a body
/// mid-transit (the entry-side slice or the exit-side slice). Rebuilt each frame.
#[derive(Component)]
pub struct PortalBodyPiece;

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
            Sprite::from_color(core, Vec2::new(length * 0.86, PORTAL_VISUAL_THICKNESS * 0.42)),
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
            TextFont { font_size: 12.0, ..default() },
            TextColor(core),
            Transform::from_translation(label_translation),
            Name::new("Portal label"),
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
        let (hit, normal) = raycast_solids(&world, Vec2::new(200.0, 200.0), Vec2::new(-1.0, 0.0), 6000.0, false)
            .expect("ray should hit the left wall");
        assert!((hit.x - 20.0).abs() < 0.001, "hit x={}", hit.x);
        assert!(normal.x > 0.5 && normal.y.abs() < 0.001, "normal={normal:?}");
    }

    #[test]
    fn portals_adhere_to_one_way_platforms_but_blink_passes_through() {
        use crate::engine_core::world::{Block, World};
        let world = World {
            name: "one-way".to_string(),
            size: Vec2::new(400.0, 400.0),
            spawn: Vec2::new(200.0, 200.0),
            blocks: vec![Block::one_way(
                "ledge",
                Vec2::new(100.0, 300.0),
                Vec2::new(200.0, 12.0),
            )],
            climbable_regions: Vec::new(),
            water_regions: Vec::new(),
        };
        let from = Vec2::new(200.0, 100.0);
        let dir = Vec2::new(0.0, 1.0); // down toward the one-way's top (y=300)
        // A portal shot adheres to the one-way (#39).
        let portal_hit = raycast_solids(&world, from, dir, 6000.0, true);
        assert!(
            portal_hit.is_some_and(|(hit, n)| (hit.y - 300.0).abs() < 1.0 && n.y < -0.5),
            "a portal shot should adhere to a one-way's face (#39), got {portal_hit:?}"
        );
        // ...but blink / dive pass straight through one-ways.
        assert!(
            raycast_solids(&world, from, dir, 6000.0, false).is_none(),
            "blink/dive should pass through one-way platforms"
        );
    }

    #[test]
    fn raycast_sees_through_a_portal_pair_and_recurses() {
        // Only block: a left wall at x[0,20]. A ray cast straight DOWN hits no
        // solid — unless it transits the floor portal and emerges from the wall
        // portal heading left into that wall.
        let world = ae::World::new(
            "portal-los",
            Vec2::new(400.0, 400.0),
            Vec2::new(200.0, 200.0),
            vec![ae::Block::solid("left", Vec2::new(0.0, 0.0), Vec2::new(20.0, 400.0))],
        );
        let portals = vec![
            Portal {
                color: PortalColor::Blue,
                pos: Vec2::new(200.0, 380.0),
                normal: Vec2::new(0.0, -1.0),
                half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
            },
            Portal {
                color: PortalColor::Orange,
                pos: Vec2::new(380.0, 200.0),
                normal: Vec2::new(-1.0, 0.0),
                half_extent: portal_half_extent(Vec2::new(-1.0, 0.0)),
            },
        ];
        // Without portals, casting down hits nothing.
        assert!(raycast_solids(&world, Vec2::new(200.0, 300.0), Vec2::new(0.0, 1.0), 6000.0, false).is_none());
        // Through the portal pair, the ray emerges from the wall portal heading
        // left and lands on the left wall's right face (x≈20, normal +x).
        let hit = raycast_through_portals(
            &world,
            &portals,
            Vec2::new(200.0, 300.0),
            Vec2::new(0.0, 1.0),
            6000.0,
            false,
            2,
        );
        assert!(
            hit.is_some_and(|(p, n)| (p.x - 20.0).abs() < 1.0 && n.x > 0.5),
            "ray should recurse through the pair and hit the left wall, got {hit:?}"
        );
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
        app.add_systems(Update, portal_transit_actors);
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
        // Aperture transit: frame 1 begins (leading edge in the opening), frame 2
        // transfers (centroid already on the plane).
        app.update();
        app.update();
        let s = app.world().get::<ActorKinematics>(small).unwrap();
        assert!(
            s.pos.x > 250.0,
            "a fitting actor transits out the orange portal, pos={:?}",
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
    fn n_pairs_transit_routes_to_the_matching_partner() {
        let he = portal_half_extent(Vec2::new(0.0, -1.0));
        let floor = |color, x: f32| Portal {
            color,
            pos: Vec2::new(x, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: he,
        };
        // Two INDEPENDENT floor pairs placed at once.
        let portals = vec![
            floor(PortalColor::Blue, 100.0),
            floor(PortalColor::Orange, 200.0),
            floor(PortalColor::Purple, 400.0),
            floor(PortalColor::Yellow, 700.0),
        ];
        // A body whose centroid has crossed the PURPLE plane transfers to YELLOW
        // (its partner) — never to the unrelated orange portal.
        let step = transit_step(
            Vec2::new(400.0, 305.0),
            Vec2::new(24.0, 40.0),
            Vec2::new(0.0, 50.0),
            Some(PortalTransit { straddling: PortalColor::Purple, crossed: false }),
            0.0,
            &portals,
            Vec2::new(0.0, 1.0),
        );
        match step {
            TransitStep::Transfer { exit_color, pos, .. } => {
                assert_eq!(exit_color, PortalColor::Yellow, "purple links to yellow");
                assert!(pos.x > 600.0, "emerges at the yellow portal, got {pos:?}");
            }
            other => panic!("expected a transfer to yellow, got {other:?}"),
        }
    }

    #[test]
    fn facing_flips_only_for_a_same_wall_turn_around() {
        let g = Vec2::new(0.0, 1.0); // gravity down
        let up = Vec2::new(0.0, -1.0); // floor
        let down = Vec2::new(0.0, 1.0); // ceiling
        let left = Vec2::new(-1.0, 0.0); // right-wall normal
        let right = Vec2::new(1.0, 0.0); // left-wall normal
        // Same wall (both normals left) is the only "face in, back out" case →
        // facing mirrors so it comes out face-first.
        assert!(portal_facing_flips(left, left, g));
        // Walls facing EACH OTHER (portal_bridge) go straight through → no flip.
        assert!(!portal_facing_flips(right, left, g));
        // Floor/ceiling pairs carry orientation in the somersault rotation → no
        // separate facing flip (the 180° rotation already mirrors).
        assert!(!portal_facing_flips(up, up, g));
        assert!(!portal_facing_flips(down, down, g));
        assert!(!portal_facing_flips(up, left, g));
    }

    #[test]
    fn somersault_is_suppressed_for_a_wall_to_wall_turn_around() {
        use std::f32::consts::PI;
        let g = Vec2::new(0.0, 1.0); // gravity down
        let up = Vec2::new(0.0, -1.0); // floor normal
        let down = Vec2::new(0.0, 1.0); // ceiling normal
        let left = Vec2::new(-1.0, 0.0); // right-wall normal
        // Floor↔floor and ceiling↔ceiling KEEP the 180° tumble (feet-in → reorient).
        assert!((somersault_roll(up, up, g).abs() - PI).abs() < 1e-5);
        assert!((somersault_roll(down, down, g).abs() - PI).abs() < 1e-5);
        // Two portals on the SAME wall (both normals horizontal) impart NO tumble —
        // the body just turns around and comes out upright.
        assert!(somersault_roll(left, left, g).abs() < 1e-5);
        // A floor→wall pair still tumbles 90° (it genuinely reorients).
        assert!(somersault_roll(up, left, g).abs() > 1.0);
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
    fn actors_get_an_aerial_roll_through_portals() {
        use crate::features::ActorKinematics;
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_systems(Update, portal_transit_actors);
        // Floor portal (normal up) + right-wall portal (normal left): a
        // floor→wall pair, so transit imparts a -90° roll. Player and non-player
        // actors alike now tumble + reorient (the somersault is ported to the
        // aperture model and applied on the centroid transfer).
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
        // Frame 1 begins transit; frame 2 transfers (centroid on the plane) and
        // imparts the somersault roll.
        app.update();
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
        app.add_systems(Update, portal_transit_system);
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
        // Give the player a pre-set roll so we can prove the portal leaves the
        // player's orientation alone (#47 — no upside-down flip).
        app.world_mut()
            .entity_mut(player)
            .insert(ActorRoll { angle: 0.5 });
        // Frame 1 begins transit (leading edge in the aperture); frame 2 sees the
        // centroid already across the plane and transfers the authoritative body.
        app.update();
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
        let roll = app.world().get::<ActorRoll>(player).unwrap().angle;
        assert!(
            (roll - 0.5).abs() < 1e-5,
            "player keeps its orientation through the portal (#47 — no flip), got {roll}"
        );
    }

    #[test]
    fn portal_input_warp_transforms_held_input_then_clears() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, warp_portal_input);
        // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PortalInputWarp {
                    rot: pp::portal_rotation(Vec2::new(-1.0, 0.0), Vec2::new(-1.0, 0.0)),
                    anchor: Vec2::new(1.0, 0.0),
                },
            ))
            .id();

        // Still holding right → input is warped to LEFT (keeps you moving out).
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x < -0.5,
            "held right is warped to left while the warp is active"
        );
        assert!(app.world().get::<PortalInputWarp>(player).is_some(), "warp persists while held");

        // Release movement → warp drops, input passes through untouched next frame.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 0.0;
        app.update();
        assert!(app.world().get::<PortalInputWarp>(player).is_none(), "release drops the warp");

        // Re-arm, then press a clearly different direction (left) → warp drops.
        app.world_mut().entity_mut(player).insert(PortalInputWarp {
            rot: pp::portal_rotation(Vec2::new(-1.0, 0.0), Vec2::new(-1.0, 0.0)),
            anchor: Vec2::new(1.0, 0.0),
        });
        app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
        app.update();
        assert!(
            app.world().get::<PortalInputWarp>(player).is_none(),
            "a clearly different direction drops the warp"
        );
    }

    #[test]
    fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, warp_portal_input);
        // Emerging from a right-wall portal — exit_normal points LEFT (into room).
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PortalEmission { exit_normal: Vec2::new(-1.0, 0.0), timer: 1.0 },
            ))
            .id();
        // Holding RIGHT (back into the wall) is stripped so physics carries you out.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x.abs() < 0.01,
            "input pushing back into the exit wall is stripped during emergence"
        );
        // Holding LEFT (the emergence direction) passes through untouched.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x < -0.5,
            "input in the emergence direction is preserved"
        );
        let _ = player;
    }

    #[test]
    fn a_gunless_player_transits_an_authored_pair() {
        // The portal_lab scenario: pre-placed portals, player has NOT picked up
        // the gun. Transit must still work (the gun only carries the cooldown).
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(crate::WorldTime::default());
        app.add_systems(Update, portal_transit_system);
        let he = portal_half_extent(Vec2::new(0.0, -1.0));
        app.world_mut().spawn(Portal {
            color: PortalColor::Purple,
            pos: Vec2::new(200.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: he,
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Yellow,
            pos: Vec2::new(600.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: he,
        });
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: Vec2::new(200.0, 285.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                // No PortalGun on purpose.
            ))
            .id();
        // Frame 1 begins transit (standing on the purple floor portal).
        app.update();
        assert!(
            app.world().get::<PortalTransit>(player).is_some(),
            "a gun-less player standing on an authored portal begins transit"
        );
        // Sink the centroid past the plane → transfer to the yellow partner.
        app.world_mut().get_mut::<PlayerKinematics>(player).unwrap().pos.y = 305.0;
        app.update();
        let pos = app.world().get::<PlayerKinematics>(player).unwrap().pos;
        assert!(pos.x > 550.0, "transfers to the yellow portal without a gun, got {pos:?}");
    }

    #[test]
    fn transit_is_gradual_centroid_crossing_flags_the_teleport_then_clears() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(crate::WorldTime::default());
        app.init_resource::<IntentionalTeleport>();
        app.add_systems(Update, portal_transit_system);
        // Two FLOOR portals (normal up): blue at x=200, orange at x=600.
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(200.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(600.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        // Player straddling the blue floor: feet (max.y=305) below the plane,
        // centroid (285) still above it.
        let player = spawn_player(&mut app, Vec2::new(200.0, 285.0), 1.0);

        // Frame 1: leading edge in the aperture → transit BEGINS, no transfer.
        app.update();
        assert!(
            app.world().get::<PortalTransit>(player).is_some_and(|t| !t.crossed),
            "transit begins without an instant teleport"
        );
        assert!(app.world().get::<PlayerKinematics>(player).unwrap().pos.x < 250.0, "still entry-side");
        assert!(!app.world().resource::<IntentionalTeleport>().0, "no teleport flag yet");

        // Push the centroid across the plane (as the integrator would as the body
        // sinks into the carved opening).
        app.world_mut().get_mut::<PlayerKinematics>(player).unwrap().pos.y = 305.0;
        app.update();
        assert!(
            app.world().get::<PortalTransit>(player).is_some_and(|t| t.crossed),
            "centroid crossing transfers the authoritative body"
        );
        let pos = app.world().get::<PlayerKinematics>(player).unwrap().pos;
        assert!(pos.x > 550.0, "authoritative body is now exit-side, got {pos:?}");
        assert!(
            app.world().resource::<IntentionalTeleport>().0,
            "the centroid transfer flags an intentional teleport (suppresses the trace auto-dump)"
        );

        // Move clear of the exit plane → transit ends (re-armed via cooldown).
        app.world_mut().get_mut::<PlayerKinematics>(player).unwrap().pos.y = 270.0;
        app.update();
        assert!(
            app.world().get::<PortalTransit>(player).is_none(),
            "transit clears once the body fully clears the plane"
        );
        assert!(
            !app.world().resource::<IntentionalTeleport>().0,
            "the teleport flag is a single frame"
        );
    }

    #[test]
    fn partial_render_keeps_the_sprite_and_masks_the_through_slice() {
        use crate::presentation::rendering::PlayerVisual;
        let mut app = App::new();
        app.insert_resource(world_with_two_walls());
        app.add_systems(Update, sync_portal_body_pieces);
        // Floor pair so a body standing on the blue portal straddles its plane.
        app.world_mut().spawn(Portal {
            color: PortalColor::Blue,
            pos: Vec2::new(200.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(300.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        // Body whose feet have dipped below the floor plane (y 275..315, plane 300).
        let player = app
            .world_mut()
            .spawn((
                PlayerVisual,
                PlayerKinematics {
                    pos: Vec2::new(200.0, 295.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                Sprite::from_color(Color::WHITE, Vec2::new(24.0, 40.0)),
                Visibility::Inherited,
                PortalTransit { straddling: PortalColor::Blue, crossed: false },
            ))
            .id();
        app.update();
        // The real sprite stays visible — only the through-wall slice is masked.
        assert_eq!(
            *app.world().get::<Visibility>(player).unwrap(),
            Visibility::Inherited,
            "the real character sprite is NOT hidden; the box masks the invisible part"
        );
        // An exit copy of the sprite + a mask over each invisible slice (entry
        // through-wall + exit not-yet-emerged) = three transient pieces.
        let pieces = {
            let mut q = app.world_mut().query::<&PortalBodyPiece>();
            q.iter(app.world()).count()
        };
        assert_eq!(pieces, 3, "exit sprite copy + entry mask + exit mask");
    }

    #[test]
    fn portal_carve_is_transient_and_pair_gated() {
        let mut app = App::new();
        app.init_resource::<crate::features::FeatureEcsWorldOverlay>();
        app.add_systems(Update, publish_portal_carves);
        // A lone portal must NOT carve (no exit → no bottomless hole).
        let blue = app
            .world_mut()
            .spawn(Portal {
                color: PortalColor::Blue,
                pos: Vec2::new(200.0, 300.0),
                normal: Vec2::new(0.0, -1.0),
                half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
            })
            .id();
        app.update();
        assert!(
            app.world().resource::<crate::features::FeatureEcsWorldOverlay>().portal_carves.is_empty(),
            "a lone portal does not carve"
        );
        // Complete the pair — but with NO body transiting, still nothing carves
        // (so you can't wiggle into a wall pocket between crossings).
        app.world_mut().spawn(Portal {
            color: PortalColor::Orange,
            pos: Vec2::new(600.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        });
        app.update();
        assert!(
            app.world().resource::<crate::features::FeatureEcsWorldOverlay>().portal_carves.is_empty(),
            "a placed pair with no body transiting stays solid (no walk-in pocket)"
        );
        // A body transiting the blue portal carves EXACTLY that portal.
        let _ = blue;
        app.world_mut().spawn(PortalTransit { straddling: PortalColor::Blue, crossed: false });
        app.update();
        assert_eq!(
            app.world().resource::<crate::features::FeatureEcsWorldOverlay>().portal_carves.len(),
            1,
            "only the portal a body is passing through is carved"
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
        // The opened portal is room-scoped, so a room transition despawns it —
        // no lingering portals that reappear when you leave and come back (#41).
        let scoped = {
            let mut q = app.world_mut().query_filtered::<
                (),
                (With<Portal>, With<crate::presentation::rendering::RoomScopedEntity>),
            >();
            q.iter(app.world()).count()
        };
        assert!(scoped >= 1, "an opened portal must be RoomScopedEntity (#41)");
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
