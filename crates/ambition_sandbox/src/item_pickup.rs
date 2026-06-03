//! Pick up / throw held items (vertical slice).
//!
//! A `GroundItem` sits in the world; an empty-handed player presses `Attack`
//! while overlapping it to pick it up — the item's `HeldItemSpec` is overlaid
//! onto the player's `ActionSet` (so e.g. the axe grants its swing) and a
//! `HeldItem` component is attached. `Shield + Attack` throws the held item
//! back onto the ground ahead of the player, restoring the player's original
//! action set.
//!
//! Decisions baked in (see TODO "Pick-up / throw held items"):
//! - one held item at a time (the `Without<HeldItem>` pickup filter),
//! - `Attack` picks up / uses; `Shield + Attack` throws (Smash grab-throw).
//!
//! Handoff / not-yet-built:
//! - thrown items arc under gravity and settle on contact (no slide/bounce),
//! - placement is a single debug-spawned axe; authored ground items come later,
//! - a held-in-hand sprite on the player; held-item gating of the portal gun.

use bevy::prelude::*;

use crate::brain::{ActionSet, HeldItemSpec, MeleeActionSpec, SwipeSpec};
use crate::engine_core::{self as ae, AabbExt};
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};
use crate::portal::PortalGun;

const PICKUP_HALF: f32 = 18.0;
const THROW_AHEAD: f32 = 48.0;

/// A held item resting in the world, pick-up-able with `Attack` when the
/// player is empty-handed. Thrown items carry a `vel` and arc under gravity
/// until they settle on a surface (`vel == ZERO` means resting).
#[derive(Component, Clone, Debug)]
pub struct GroundItem {
    pub spec: HeldItemSpec,
    pub pos: Vec2,
    pub vel: Vec2,
    pub half_extent: Vec2,
}

const GROUND_ITEM_GRAVITY: f32 = 1400.0;
const THROW_SPEED_X: f32 = 320.0;
const THROW_SPEED_UP: f32 = 260.0;

/// Integrate thrown ground items under gravity (y-down world) and settle them
/// when they'd enter a solid / one-way surface. Resting items (`vel == ZERO`)
/// are skipped, so pickup-able items stay put.
pub fn ground_item_physics(
    time: Res<crate::WorldTime>,
    world: Res<crate::GameWorld>,
    gravity: Option<Res<crate::physics::GravityField>>,
    mut grounds: Query<&mut GroundItem>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Thrown / dropped items are free bodies, so they integrate through the
    // shared world-forces seam — a gravity flip (or any future global force)
    // moves them without touching this system.
    let gravity = gravity.as_deref().copied().unwrap_or_default();
    for mut item in &mut grounds {
        if item.vel == Vec2::ZERO {
            continue;
        }
        crate::physics::apply_world_forces(&mut item.vel, GROUND_ITEM_GRAVITY, &gravity, dt);
        let next = item.pos + item.vel * dt;
        let next_aabb = ae::Aabb::new(next, item.half_extent);
        let blocked = world.0.blocks.iter().any(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            ) && next_aabb.strict_intersects(block.aabb)
        });
        let below_world = next.y > world.0.size.y + 200.0 || next.y < -200.0;
        if blocked {
            // Settle in place (simple — no slide).
            item.vel = Vec2::ZERO;
        } else if below_world {
            item.vel = Vec2::ZERO;
        } else {
            item.pos = next;
        }
    }
}

/// The player's pre-pickup `ActionSet`, restored when the held item is thrown.
#[derive(Component, Clone)]
pub struct StashedActionSet(pub ActionSet);

/// Authored axe held item: a keep-on-use heavy melee swing (placeholder tuning).
pub fn axe_spec() -> HeldItemSpec {
    HeldItemSpec {
        id: "axe".into(),
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.22,
            active_s: 0.12,
            recover_s: 0.30,
            damage: 3,
            reach_px: 64.0,
        })),
        ranged: None,
    }
}

/// Authored javelin held item: a *pure throwable* (no melee/ranged verb), so
/// using it (`Attack` while holding) throws it — the `ThrowOnUse` behavior.
pub fn javelin_spec() -> HeldItemSpec {
    HeldItemSpec {
        id: "javelin".into(),
        melee: None,
        ranged: None,
    }
}

/// The laser gun-sword as a *player* held item — the same authored `gun_sword`
/// the pirates carry (`crate::brain::held_item_by_id`). Picking it up replaces
/// the player's melee swing with the item's *ranged* verb, so `Attack` fires a
/// laser bolt instead of swinging — the unification the pirates will share once
/// their dedicated sniper mode is dropped (see TODO).
pub fn gunsword_spec() -> HeldItemSpec {
    crate::brain::held_item_by_id("gun_sword").expect("gun_sword is a built-in held item")
}

/// Resolve a catalog [`crate::items::Item`]'s held-item spec, for equipping from
/// a non-pickup source (the inventory menu). The three wired weapons each have a
/// spec; everything else returns `None`.
pub fn held_spec_for_item(item: crate::items::Item) -> Option<HeldItemSpec> {
    use crate::items::Item;
    match item {
        Item::Axe => Some(axe_spec()),
        Item::Javelin => Some(javelin_spec()),
        Item::GunSword => Some(gunsword_spec()),
        _ => item
            .held_item_id()
            .and_then(crate::brain::held_item_by_id),
    }
}

/// Equip a held-item spec onto the player from a non-pickup source (e.g. the
/// inventory menu): stash the current action set, overlay the item's verbs, and
/// attach [`HeldItem`]. Mirrors [`pickup_held_item_system`] minus the ground
/// entity so the menu and the world pickup share one equip contract.
pub fn equip_held_spec(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    spec: HeldItemSpec,
) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    let held = HeldItem::new(spec.clone());
    // The held item *replaces* the player's attack verbs (move-style/special
    // are kept), exactly as the world pickup does.
    action_set.melee = spec.melee;
    action_set.ranged = spec.ranged;
    commands.entity(player).insert(held);
}

/// Detach the currently held item and restore the stashed action set. Mirrors
/// the throw path's restore without dropping a ground item.
pub fn unequip_held(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<HeldItem>();
    commands.entity(player).remove::<StashedActionSet>();
}

/// Spawn one axe ground item near the player on the first frame a player
/// exists (debug convenience until authored placement lands).
pub fn spawn_debug_axe_once(
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
        GroundItem {
            spec: axe_spec(),
            pos: kin.pos + Vec2::new(80.0, 0.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(PICKUP_HALF),
        },
        Name::new("Ground item: axe"),
    ));
}

/// Spawn one gun-sword ground item near the player on the first frame a player
/// exists (debug convenience until authored placement lands). Picking it up
/// makes `Attack` fire laser bolts.
pub fn spawn_debug_gunsword_once(
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
        GroundItem {
            spec: gunsword_spec(),
            pos: kin.pos + Vec2::new(160.0, 0.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(PICKUP_HALF),
        },
        Name::new("Ground item: gun-sword"),
    ));
}

/// `Attack` while empty-handed and overlapping a `GroundItem` picks it up:
/// stash the current action set, overlay the item's verbs, attach `HeldItem`.
pub fn pickup_held_item_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &PlayerKinematics, &mut ActionSet),
        (
            With<PlayerEntity>,
            With<PrimaryPlayer>,
            // One item at a time (Smash-style): can't grab a ground item while
            // already holding one, or while holding the portal gun.
            Without<HeldItem>,
            Without<PortalGun>,
        ),
    >,
    grounds: Query<(Entity, &GroundItem)>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    if !control.attack_pressed {
        return;
    }
    let Ok((player, kin, mut action_set)) = players.single_mut() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (ground_entity, ground) in &grounds {
        let ground_aabb = ae::Aabb::new(ground.pos, ground.half_extent);
        if player_aabb.strict_intersects(ground_aabb) {
            commands
                .entity(player)
                .insert(StashedActionSet(action_set.clone()));
            // The held item *replaces* the player's attack verbs (not a merge):
            // the axe sets melee + clears ranged; the gun-sword clears melee +
            // sets ranged so `Attack` fires the laser instead of swinging. Move
            // style / special are kept from the player's own set.
            action_set.melee = ground.spec.melee;
            action_set.ranged = ground.spec.ranged;
            commands
                .entity(player)
                .insert(HeldItem::new(ground.spec.clone()));
            // Reflect into the 24-item catalog: picking a held item up grants its
            // slot and marks it equipped, so the OoT menu shows it as held.
            if let Some(owned) = owned.as_deref_mut() {
                if let Some(item) = crate::items::Item::from_held_item_id(&ground.spec.id) {
                    owned.grant(item, 1);
                    owned.set_equipped(Some(item));
                }
            }
            commands.entity(ground_entity).despawn();
            break;
        }
    }
}

/// A "pure throwable" held item has no melee/ranged verb of its own, so its
/// *use* is to be thrown (the javelin's `ThrowOnUse` behavior): a plain
/// `Attack` while holding it throws it. Items with a verb (the axe) keep
/// their swing on `Attack` and only throw on the explicit `Shield + Attack`.
fn is_pure_throwable(spec: &HeldItemSpec) -> bool {
    spec.melee.is_none() && spec.ranged.is_none()
}

/// Throw the held item: restore the stashed action set, detach `HeldItem`,
/// and drop a `GroundItem` ahead of the player. Fires on `Shield + Attack`
/// for any item, or on a plain `Attack` for a pure throwable (throw-on-use).
pub fn throw_held_item_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &PlayerKinematics,
            &mut ActionSet,
            &HeldItem,
            Option<&StashedActionSet>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    if !control.attack_pressed {
        return;
    }
    let Ok((player, kin, mut action_set, held, stashed)) = players.single_mut() else {
        return;
    };
    // Shield+Attack throws anything; a plain Attack only throws a pure throwable.
    if !(control.shield_held || is_pure_throwable(&held.spec)) {
        return;
    }
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    // Throwing stows the held weapon: clear the equipped slot (the player keeps
    // catalog ownership and can re-equip from the menu).
    if let Some(owned) = owned.as_deref_mut() {
        owned.set_equipped(None);
    }
    let spec = held.spec.clone();
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    let throw_pos = kin.pos + Vec2::new(facing * THROW_AHEAD, 0.0);
    commands.entity(player).remove::<HeldItem>();
    commands.entity(player).remove::<StashedActionSet>();
    commands.spawn((
        GroundItem {
            spec,
            pos: throw_pos,
            // Arc forward + up (y-down world, so up is -y).
            vel: Vec2::new(facing * THROW_SPEED_X, -THROW_SPEED_UP),
            half_extent: Vec2::splat(PICKUP_HALF),
        },
        Name::new("Ground item: thrown"),
    ));
}

// ---------------------------------------------------------------------------
// Held *ranged* items (the gun-sword): `Attack` fires a traveling laser bolt.
//
// Self-contained like the portal shot — a `HeldProjectile` travels each tick,
// damages the first enemy / boss / breakable it overlaps (reusing the shared
// feature-damage `HitEvent` channel), and expires on a solid wall or past max
// range. This is the player end of the held-gun-sword unification: the same
// `RangedActionSpec` the pirates fire, driven by the player's `Attack`.

/// An in-flight laser bolt fired from a held ranged item (gun-sword).
#[derive(Component, Clone, Copy, Debug)]
pub struct HeldProjectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: i32,
    pub traveled: f32,
}

const HELD_SHOT_MAX_RANGE: f32 = 1600.0;
const HELD_SHOT_HALF: Vec2 = Vec2::new(12.0, 9.0);

/// Aim a held ranged shot the way the pirates aim their gun-sword: right-stick
/// aim if pushed, else the movement axis (so holding Up / Down / a diagonal
/// aims there), else straight ahead along facing.
fn held_shot_aim(control: &ControlFrame, facing: f32) -> Vec2 {
    let aim = Vec2::new(control.aim_x, control.aim_y);
    if aim.length() > 0.3 {
        return aim.normalize_or_zero();
    }
    let mv = Vec2::new(control.axis_x, control.axis_y);
    if mv.length() > 0.3 {
        return mv.normalize_or_zero();
    }
    Vec2::new(if facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
}

/// `Attack` while holding a *ranged* item fires a laser bolt along the aim
/// direction. `Shield + Attack` is the throw/drop gesture, so don't fire on it.
pub fn fire_held_ranged_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    players: Query<(&PlayerKinematics, &HeldItem), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, held)) = players.single() else {
        return;
    };
    let Some(ranged) = held.spec.ranged else {
        return;
    };
    let dir = held_shot_aim(&control, kin.facing);
    if dir == Vec2::ZERO {
        return;
    }
    let origin = kin.pos + dir * (kin.size.x * 0.5 + 8.0) - Vec2::new(0.0, kin.size.y * 0.12);
    commands.spawn((
        HeldProjectile {
            pos: origin,
            vel: dir * ranged.speed(),
            damage: ranged.damage(),
            traveled: 0.0,
        },
        Name::new("Held ranged shot"),
    ));
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::SfxId::from_static("weapon.lasersword.fire"),
        pos: origin,
    });
}

/// Advance held ranged shots; damage the first feature they overlap, or expire
/// on a solid wall / past max range.
#[allow(clippy::too_many_arguments)]
pub fn held_projectile_step(
    time: Res<crate::WorldTime>,
    world: Res<crate::GameWorld>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut HeldProjectile)>,
    player: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    ecs_breakables: Query<
        (
            &crate::features::FeatureId,
            &crate::features::FeatureAabb,
            &crate::features::BreakableFeature,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    ecs_actors: Query<
        (
            &crate::features::FeatureId,
            &crate::features::FeatureAabb,
            &crate::features::ActorDisposition,
            &crate::features::ActorCombatState,
        ),
        (
            With<crate::features::FeatureSimEntity>,
            Without<crate::features::BossConfig>,
        ),
    >,
    ecs_bosses: Query<
        (
            &crate::features::FeatureId,
            &crate::features::FeatureAabb,
            crate::features::BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    mut feature_damage: MessageWriter<crate::features::HitEvent>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let attacker = player.single().ok();
    for (entity, mut proj) in &mut projectiles {
        // Damage check against actors / bosses / breakables via the shared
        // attacker-side channel. `PlayerProjectile` broadcasts to features.
        let hit_event = crate::features::HitEvent {
            volume: ae::Aabb::new(proj.pos, HELD_SHOT_HALF),
            damage: proj.damage,
            source: crate::features::HitSource::PlayerProjectile {
                kind: crate::projectile::ProjectileKind::Fireball,
            },
            attacker,
            target: crate::features::HitTarget::Volume,
            mode: crate::features::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        };
        let hit = crate::features::ecs_hit_event_hits_breakable(&hit_event, &ecs_breakables)
            || crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors)
            || crate::features::ecs_hit_event_hits_boss(&hit_event, &ecs_bosses);
        if hit {
            feature_damage.write(hit_event);
            sfx.write(crate::audio::SfxMessage::Hit { pos: proj.pos });
            commands.entity(entity).despawn();
            continue;
        }
        // Solid wall in this step → impact + expire.
        let step = (proj.vel * dt).length().max(1.0);
        if let Some((hit_pos, _normal)) = crate::portal::raycast_solids(&world.0, proj.pos, proj.vel, step) {
            vfx.write(crate::presentation::fx::VfxMessage::Impact { pos: hit_pos });
            commands.entity(entity).despawn();
            continue;
        }
        let delta = proj.vel * dt;
        proj.pos += delta;
        proj.traveled += delta.length();
        let oob = proj.pos.x < -64.0
            || proj.pos.y < -64.0
            || proj.pos.x > world.0.size.x + 64.0
            || proj.pos.y > world.0.size.y + 64.0;
        if proj.traveled > HELD_SHOT_MAX_RANGE || oob {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Presentation (visible build only).

/// Marks a sprite entity visualizing a [`GroundItem`].
#[derive(Component)]
pub struct GroundItemVisual;

/// Colored quad per ground item so it's visible. Clear-and-rebuild (few items).
/// Loaded held-item art (axe / javelin sprites). Visible build only.
#[derive(Resource)]
pub struct ItemArt {
    pub axe: Handle<Image>,
    pub javelin: Handle<Image>,
    pub gunsword: Handle<Image>,
}

/// Load the held-item sprites at startup.
pub fn load_item_art(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ItemArt {
        axe: assets.load("sprites/props/axe.png"),
        javelin: assets.load("sprites/props/javelin.png"),
        gunsword: assets.load("sprites/props/gunsword.png"),
    });
}

/// `(image, display size)` for a held-item spec id, if it has authored art.
/// Display sizes keep each prop's native aspect ratio (axe 173×76, javelin
/// 236×29, lasersword 169×44).
fn item_sprite(art: &ItemArt, spec_id: &str) -> Option<(Handle<Image>, Vec2)> {
    match spec_id {
        "axe" => Some((art.axe.clone(), Vec2::new(40.0, 18.0))),
        "javelin" => Some((art.javelin.clone(), Vec2::new(58.0, 7.0))),
        // Same `lasersword_with_guns` proportions the pirates hold (177×46).
        "gun_sword" | "gun_sword_heavy" => Some((art.gunsword.clone(), Vec2::new(54.0, 14.0))),
        _ => None,
    }
}

pub fn sync_ground_item_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    art: Option<Res<ItemArt>>,
    visuals: Query<Entity, With<GroundItemVisual>>,
    grounds: Query<&GroundItem>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for ground in &grounds {
        let translation = crate::config::world_to_bevy(&world.0, ground.pos, 8.0);
        let sprite = art
            .as_ref()
            .and_then(|a| item_sprite(a, ground.spec.id.as_str()))
            .map(|(image, size)| Sprite {
                image,
                custom_size: Some(size),
                ..default()
            })
            .unwrap_or_else(|| {
                Sprite::from_color(Color::srgb(0.72, 0.52, 0.30), ground.half_extent * 2.0)
            });
        commands.spawn((
            GroundItemVisual,
            sprite,
            Transform::from_translation(translation),
            Name::new("Ground item visual"),
        ));
    }
}

/// Marks the sprite shown in the player's hand for the currently held item.
#[derive(Component)]
pub struct HeldItemVisual;

/// Draw a small quad in the player's hand for whatever they're holding, tinted
/// per item (axe / javelin). Clear-and-rebuild each frame.
pub fn sync_held_item_visual(
    mut commands: Commands,
    control: Res<ControlFrame>,
    world: Res<crate::GameWorld>,
    art: Option<Res<ItemArt>>,
    visuals: Query<Entity, With<HeldItemVisual>>,
    players: Query<(&PlayerKinematics, &HeldItem), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Ok((kin, held)) = players.single() else {
        return;
    };
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the player's hand: just in front at hand height (y-down → small +y).
    let hand = kin.pos + Vec2::new(facing * (kin.size.x * 0.45 + 4.0), kin.size.y * 0.06);
    let translation = crate::config::world_to_bevy(&world.0, hand, 12.0);

    // A ranged held item (the gun-sword) points where you're AIMING — the same
    // direction it fires — just like the pirates' wielded gun-sword. Melee /
    // thrown items keep the simple facing flip.
    let (rotation, flip_x, flip_y) = if held.spec.ranged.is_some() {
        let aim = held_shot_aim(&control, kin.facing);
        // World is y-down, render space y-up. Aiming left flips vertically so
        // the gun stays upright instead of rotating upside-down.
        let angle = (-aim.y).atan2(aim.x);
        (Quat::from_rotation_z(angle), false, aim.x < 0.0)
    } else {
        (Quat::IDENTITY, facing < 0.0, false)
    };

    let sprite = art
        .as_ref()
        .and_then(|a| item_sprite(a, held.spec.id.as_str()))
        .map(|(image, size)| Sprite {
            image,
            custom_size: Some(size),
            flip_x,
            flip_y,
            ..default()
        })
        .unwrap_or_else(|| {
            let color = match held.spec.id.as_str() {
                "axe" => Color::srgb(0.72, 0.52, 0.30),
                "javelin" => Color::srgb(0.86, 0.84, 0.62),
                _ => Color::srgb(0.82, 0.82, 0.82),
            };
            Sprite::from_color(color, Vec2::new(14.0, 28.0))
        });
    commands.spawn((
        HeldItemVisual,
        sprite,
        Transform::from_translation(translation).with_rotation(rotation),
        Name::new("Held item visual"),
    ));
}

/// Marks the streak sprite for an in-flight [`HeldProjectile`] (laser bolt).
#[derive(Component)]
pub struct HeldProjectileVisual;

/// Render each in-flight gun-sword shot as the **same** spinning lasersword
/// sprite the pirates fire (`enemy_projectile::lasersword_projectile_sprite`),
/// rotated along its travel. Clear-and-rebuild each frame (few shots).
pub fn sync_held_projectile_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<crate::GameWorld>,
    visuals: Query<Entity, With<HeldProjectileVisual>>,
    projectiles: Query<&HeldProjectile>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let texture = asset_server.load(crate::enemy_projectile::LASERSWORD_SHEET);
    for proj in &projectiles {
        let translation = crate::config::world_to_bevy(&world.0, proj.pos, 9.5);
        let (sprite, anchor, rotation) =
            crate::enemy_projectile::lasersword_projectile_sprite(texture.clone(), proj.vel);
        commands.spawn((
            HeldProjectileVisual,
            sprite,
            anchor,
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            Name::new("Gun-sword laser shot"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_player(app: &mut App, pos: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ActionSet::default(),
            ))
            .id()
    }

    fn set_control(app: &mut App, attack: bool, shield: bool) {
        let mut cf = app.world_mut().resource_mut::<ControlFrame>();
        cf.attack_pressed = attack;
        cf.shield_held = shield;
    }

    #[test]
    fn attack_picks_up_axe_and_grants_its_swing_then_throw_restores() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
        // An axe on the ground, overlapping the player.
        app.world_mut().spawn(GroundItem {
            spec: axe_spec(),
            pos: Vec2::new(100.0, 100.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(PICKUP_HALF),
        });
        // Player starts with no melee.
        assert!(app.world().get::<ActionSet>(player).unwrap().melee.is_none());

        // Attack (no shield) → pick up the axe.
        set_control(&mut app, true, false);
        app.update();
        assert!(
            app.world().get::<HeldItem>(player).is_some(),
            "player should be holding the axe"
        );
        assert!(
            app.world().get::<ActionSet>(player).unwrap().melee.is_some(),
            "the axe should grant its melee swing"
        );
        let remaining_ground = {
            let mut q = app.world_mut().query::<&GroundItem>();
            q.iter(app.world()).count()
        };
        assert_eq!(remaining_ground, 0, "the picked-up axe should leave the ground");

        // Shield + Attack → throw it back onto the ground.
        set_control(&mut app, true, true);
        app.update();
        assert!(
            app.world().get::<HeldItem>(player).is_none(),
            "throwing should empty the player's hands"
        );
        assert!(
            app.world().get::<ActionSet>(player).unwrap().melee.is_none(),
            "throwing should restore the original (empty) action set"
        );
        let thrown = {
            let mut q = app.world_mut().query::<&GroundItem>();
            q.iter(app.world()).count()
        };
        assert_eq!(thrown, 1, "the thrown axe should be back on the ground");
    }

    #[test]
    fn gunsword_pickup_swaps_to_ranged_and_attack_fires_a_bolt() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, (pickup_held_item_system, fire_held_ranged_system));
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
        // Give the player a default melee swing so we can see it get cleared.
        app.world_mut().get_mut::<ActionSet>(player).unwrap().melee =
            Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.1,
                active_s: 0.1,
                recover_s: 0.1,
                damage: 1,
                reach_px: 32.0,
            }));
        app.world_mut().spawn(GroundItem {
            spec: gunsword_spec(),
            pos: Vec2::new(100.0, 100.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(PICKUP_HALF),
        });

        // Attack picks up the gun-sword (commands flush after the tick, so the
        // fire system can't also fire on this same press).
        set_control(&mut app, true, false);
        app.update();
        let actions = app.world().get::<ActionSet>(player).unwrap();
        assert!(
            actions.melee.is_none(),
            "the gun-sword should REPLACE (clear) the player's melee swing"
        );
        assert!(
            actions.ranged.is_some(),
            "the gun-sword should grant its ranged bolt"
        );

        // A second Attack while holding it fires exactly one laser bolt.
        set_control(&mut app, true, false);
        app.update();
        let bolts = {
            let mut q = app.world_mut().query::<&HeldProjectile>();
            q.iter(app.world()).count()
        };
        assert_eq!(bolts, 1, "Attack while holding the gun-sword fires one laser bolt");
    }

    #[test]
    fn thrown_item_arcs_and_settles_on_the_floor() {
        let mut app = App::new();
        let blocks = vec![ae::Block::solid(
            "floor",
            Vec2::new(0.0, 380.0),
            Vec2::new(400.0, 20.0),
        )];
        app.insert_resource(crate::GameWorld(ae::World::new(
            "phys",
            Vec2::new(400.0, 400.0),
            Vec2::new(200.0, 360.0),
            blocks,
        )));
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_systems(Update, ground_item_physics);
        let item = app
            .world_mut()
            .spawn(GroundItem {
                spec: axe_spec(),
                pos: Vec2::new(200.0, 200.0),
                vel: Vec2::new(120.0, -200.0), // forward + up
                half_extent: Vec2::splat(PICKUP_HALF),
            })
            .id();
        for _ in 0..120 {
            app.update();
        }
        let g = app.world().get::<GroundItem>(item).unwrap();
        assert_eq!(g.vel, Vec2::ZERO, "thrown item should settle, vel={:?}", g.vel);
        assert!(
            g.pos.y < 380.0 && g.pos.y > 300.0 && g.pos.x > 200.0,
            "settled near the floor and moved forward, pos={:?}",
            g.pos
        );
    }

    #[test]
    fn javelin_is_thrown_on_plain_attack_use() {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
        app.world_mut().spawn(GroundItem {
            spec: javelin_spec(),
            pos: Vec2::new(100.0, 100.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(PICKUP_HALF),
        });

        // First Attack picks up the javelin (commands flush after the tick, so
        // the throw system can't also fire this frame).
        set_control(&mut app, true, false);
        app.update();
        assert!(
            app.world().get::<HeldItem>(player).is_some(),
            "javelin should be picked up first"
        );

        // A second plain Attack (no shield) *uses* the javelin — which throws
        // it, since it has no melee/ranged verb of its own.
        set_control(&mut app, true, false);
        app.update();
        assert!(
            app.world().get::<HeldItem>(player).is_none(),
            "using the javelin should throw it and empty the hands"
        );
        let on_ground = {
            let mut q = app.world_mut().query::<&GroundItem>();
            q.iter(app.world()).count()
        };
        assert_eq!(on_ground, 1, "the thrown javelin should be on the ground");
    }
}
