//! Pick up / throw held items (vertical slice).
//!
//! A `GroundItem` sits in the world; an empty-handed player presses `Attack`
//! while overlapping it to pick it up — the item's `HeldItemSpec` is overlaid
//! onto the player's `ActionSet` (so e.g. the axe grants its swing) and a
//! `HeldItem` component is attached. `Shield + Attack` throws the held item
//! back onto the ground ahead of the player, restoring the player's original
//! action set.
//!
//! One held item at a time; `Attack` picks up / uses and `Shield + Attack`
//! throws.

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::features::HeldItem;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use crate::player::PlayerInputFrame;
#[cfg(feature = "portal")]
use crate::portal::PortalGun;
use ambition_characters::brain::ActorControl;
use ambition_characters::brain::{
    ActionSet, HeldItemSpec, HeldUseBehavior, MeleeActionSpec, SwipeSpec,
};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_input::ControlFrame;

/// Public schedule labels for held-item and ground-item simulation.
///
/// Other modules should order against these sets rather than concrete system
/// functions. That keeps cross-subsystem dependencies stable while item pickup
/// continues moving out of `app/plugins.rs`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ItemPickupSet {
    /// Held-item pickup/use/throw plus ground-item physics.
    CoreHeldItems,
    /// Bombs, gravity grenades, and other effects armed by thrown items.
    ThrownItemEffects,
    /// Wielded movement/combat abilities and ability cooldown maintenance.
    WieldedAbilities,
}

/// Module-local plugin for held-item, pickup, thrown-item, and wielded-item
/// simulation systems.
///
/// The app installs this plugin, but the item module owns the registration and
/// ordering details for item behavior.
pub struct ItemPickupSimulationPlugin;

impl Plugin for ItemPickupSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                ItemPickupSet::CoreHeldItems,
                ItemPickupSet::ThrownItemEffects,
                ItemPickupSet::WieldedAbilities,
            )
                .chain()
                .in_set(crate::schedule::SandboxSet::PlayerSimulation),
        );

        app.add_systems(
            Update,
            (
                // Held-items, the portal gun, the heal/save shrine, and localized
                // gravity zones are LDtk-authored room entities.
                crate::shrine::heal_save_shrine_system.run_if(crate::gameplay_allowed),
                // Resolve the live GravityField from zones + ambient after the
                // FlipGravity switch and before ground_item_physics reads it.
                crate::physics::resolve_active_gravity,
                pickup_held_item_system.run_if(crate::gameplay_allowed),
                fire_held_ranged_system.run_if(crate::gameplay_allowed),
                held_projectile_step.run_if(crate::gameplay_allowed),
                crate::abilities::thrown::puppy_slug_gun::fire_puppy_slug_gun_system
                    .run_if(crate::gameplay_allowed),
                throw_held_item_system.run_if(crate::gameplay_allowed),
                ground_item_physics.run_if(crate::gameplay_allowed),
            )
                .chain()
                // `ItemPickupSet::CoreHeldItems` is configured
                // `.in_set(PlayerSimulation)` above, so the parent placement is
                // already implied — a direct `.in_set(PlayerSimulation)` here
                // would be a redundant hierarchy edge.
                .in_set(ItemPickupSet::CoreHeldItems),
        );

        // Portal-gun ground pickups: arm the LDtk-authored pickup here; the
        // Ambition inventory grant (`pickup_portal_gun_system`) is registered
        // by the content layer (`AmbitionPortalAdaptersPlugin`), ordered
        // `.after(arm_portal_pickups)` inside this same set so the chain edge
        // is preserved without this generic module naming content.
        #[cfg(feature = "portal")]
        app.add_systems(
            Update,
            crate::portal::arm_portal_pickups
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::CoreHeldItems` (configured above).
                .in_set(ItemPickupSet::CoreHeldItems),
        );

        // Bombs and gravity grenades run after the held-item throw/physics group.
        app.add_systems(
            Update,
            (
                crate::abilities::ranged::bomb::arm_thrown_bombs.run_if(crate::gameplay_allowed),
                crate::abilities::ranged::bomb::tick_bomb_fuses.run_if(crate::gameplay_allowed),
                crate::abilities::thrown::gravity_grenade::arm_thrown_gravity_grenades
                    .run_if(crate::gameplay_allowed),
                crate::abilities::thrown::gravity_grenade::tick_gravity_grenade_fuses
                    .run_if(crate::gameplay_allowed),
                crate::physics::tick_temporary_zones.run_if(crate::gameplay_allowed),
            )
                .chain()
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::ThrownItemEffects` (configured above).
                .in_set(ItemPickupSet::ThrownItemEffects),
        );

        // Wielded movement/combat items live in their own group to avoid the
        // chained tuple arity cap in the core held-item group.
        app.add_systems(
            Update,
            (
                crate::abilities::traversal::mark_recall::mark_recall_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::traversal::blink::blink_system.run_if(crate::gameplay_allowed),
                crate::abilities::traversal::grapple::grapple_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::shockwave::fire_shockwave_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::volley::fire_volley_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::beam::fire_beam_system.run_if(crate::gameplay_allowed),
                crate::abilities::ranged::vortex::fire_vortex_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::vortex::update_vortex_wells
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::sentry::fire_sentry_system
                    .run_if(crate::gameplay_allowed),
                crate::abilities::ranged::sentry::update_sentries.run_if(crate::gameplay_allowed),
                crate::abilities::traversal::dive::fire_dive_system.run_if(crate::gameplay_allowed),
                crate::abilities::ranged::meteor::fire_meteor_system
                    .run_if(crate::gameplay_allowed),
                crate::ability_cooldown::tick_ability_cooldown,
            )
                .chain()
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::WieldedAbilities` (configured above).
                .in_set(ItemPickupSet::WieldedAbilities),
        );
    }
}

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
    time: Res<ambition_time::WorldTime>,
    world: crate::features::CollisionWorld,
    gravity: crate::physics::GravityCtx,
    mut grounds: Query<&mut GroundItem>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Thrown / dropped items settle on the composited collision world, so a
    // moving platform / ECS solid catches them the same as authored geometry.
    let Some(world) = world.solids() else {
        return;
    };
    // Thrown / dropped items are free bodies that integrate through the shared
    // world-forces seam. Gravity is resolved per item by position, so an item
    // thrown into a gravity column falls the column's way (localized).
    for mut item in &mut grounds {
        if item.vel == Vec2::ZERO {
            continue;
        }
        let local = crate::physics::GravityField {
            dir: gravity.dir_at(item.pos),
        };
        crate::physics::apply_world_forces(&mut item.vel, GROUND_ITEM_GRAVITY, &local, dt);
        let next = item.pos + item.vel * dt;
        let next_aabb = ae::Aabb::new(next, item.half_extent);
        let blocked = world.blocks.iter().any(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            ) && next_aabb.strict_intersects(block.aabb)
        });
        let below_world = next.y > world.size.y + 200.0 || next.y < -200.0;
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
        // Has a melee verb → Auto keeps it on use (swing, don't throw).
        use_behavior: HeldUseBehavior::Auto,
    }
}

/// Authored javelin held item: a *pure throwable* (no melee/ranged verb), so
/// using it (`Attack` while holding) throws it — the `ThrowOnUse` behavior.
pub fn javelin_spec() -> HeldItemSpec {
    HeldItemSpec {
        id: "javelin".into(),
        melee: None,
        ranged: None,
        // The canonical thrown item: using it (plain Attack) throws it.
        use_behavior: HeldUseBehavior::ThrowOnUse,
    }
}

/// The laser gun-sword as a *player* held item — the same authored `gun_sword`
/// the pirates carry (`ambition_characters::brain::held_item_by_id`). Picking it up replaces
/// the player's melee swing with the item's *ranged* verb, so `Attack` fires a
/// laser bolt instead of swinging — the unification the pirates will share once
/// their dedicated sniper mode is dropped (see TODO).
pub fn gunsword_spec() -> HeldItemSpec {
    ambition_characters::brain::held_item_by_id("gun_sword")
        .expect("gun_sword is a built-in held item")
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
            .and_then(ambition_characters::brain::held_item_by_id),
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

/// Equip the portal gun onto the player from a non-pickup source (the inventory
/// menu): stash the action set, attach an active [`PortalGun`], and clear the
/// melee swing so `Attack` fires portals (the same replacement the world pickup
/// does). Mirrors the pickup grant minus the ground entity — the portal-gun
/// twin of [`equip_held_spec`], so the menu and the world pickup share one
/// equip contract.
#[cfg(feature = "portal")]
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

/// Detach the portal gun and restore the stashed action set (inventory
/// unequip). The portal-gun twin of [`unequip_held`].
#[cfg(feature = "portal")]
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

/// `Attack` while empty-handed and overlapping a `GroundItem` picks it up:
/// stash the current action set, overlay the item's verbs, attach `HeldItem`.
///
/// SUBJECT-GENERIC (like `fire_held_ranged_system`): it acts on the
/// [`ControlledSubject`](crate::abilities::traversal::possession::ControlledSubject)
/// — the body you are DRIVING physically grabs the item — reading that body's own
/// `ActorControl` (brain output), NOT `PlayerInputFrame` + `PrimaryPlayer`. The
/// held item is EXPLICITLY owned by the controlled body; the catalog grant lands
/// on the global `OwnedItems` home inventory. One item at a time: a body already
/// holding an item (or the portal gun) can't grab another.
pub fn pickup_held_item_system(
    mut commands: Commands,
    controlled: Res<crate::abilities::traversal::possession::ControlledSubject>,
    mut bodies: Query<(
        &mut ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        Option<&HeldItem>,
        Option<&mut PlayerInputFrame>,
    )>,
    // Holding the portal gun blocks a pickup (portal builds only).
    #[cfg(feature = "portal")] portal_guns: Query<&PortalGun>,
    grounds: Query<(Entity, &GroundItem)>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    let Some(player) = controlled.0 else {
        return;
    };
    let Ok((mut control, kin, mut action_set, held, input)) = bodies.get_mut(player) else {
        return;
    };
    // One item at a time: already holding a physical item, or the portal gun.
    if held.is_some() {
        return;
    }
    #[cfg(feature = "portal")]
    if portal_guns.get(player).is_ok() {
        return;
    }
    // Gameplay authority is the body's brain-resolved `ActorControl`, not the
    // `PlayerInputFrame` compat mirror.
    if !control.0.melee_pressed {
        return;
    }
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    let mut input = input;
    for (ground_entity, ground) in &grounds {
        let ground_aabb = ae::Aabb::new(ground.pos, ground.half_extent);
        // AMBITION_REVIEW(discrete_ok): CC2 §3.3 GroundItem pickup — gated on a
        // deliberate `melee_pressed` while overlapping (the button-press branch
        // above), not a path-dependent auto-collect. You cannot fly THROUGH and
        // grab it, so there is no tunnel to sweep. An auto-collect (touch-to-grab
        // ring/coin) would instead route through `cast::aabb_path_contacts`.
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
            // The Attack press is *consumed* by the pickup so the same press
            // doesn't also fire the just-equipped item this frame. Clear the
            // brain-resolved `ActorControl` (the subject-generic held-item / ability
            // systems — blink/grapple/gun — read `melee_pressed` there) AND, if this
            // body carries one, the `PlayerInputFrame` compat mirror (the portal-gun
            // gesture adapter still reads it).
            control.0.melee_pressed = false;
            if let Some(input) = input.as_deref_mut() {
                input.frame.attack_pressed = false;
            }
            commands.entity(ground_entity).despawn();
            break;
        }
    }
}

/// Throw the held item: restore the stashed action set, detach `HeldItem`,
/// and drop a `GroundItem` ahead of the player. Fires on `Shield + Attack`
/// for any item, or on a plain `Attack` for a pure throwable (throw-on-use).
///
/// SUBJECT-GENERIC: acts on the
/// [`ControlledSubject`](crate::abilities::traversal::possession::ControlledSubject)
/// — the body you drive throws the item it holds — reading that body's own
/// `ActorControl`, not `PlayerInputFrame` + `PrimaryPlayer`.
pub fn throw_held_item_system(
    mut commands: Commands,
    controlled: Res<crate::abilities::traversal::possession::ControlledSubject>,
    mut bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        &HeldItem,
        Option<&StashedActionSet>,
    )>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    let Some(player) = controlled.0 else {
        return;
    };
    let Ok((control, kin, mut action_set, held, stashed)) = bodies.get_mut(player) else {
        return;
    };
    if !control.0.melee_pressed {
        return;
    }
    // Shield+Attack throws anything; plain Attack throws only items whose
    // authored `use_behavior` opts in, leaving `UseSystem` abilities to their
    // own systems.
    if !(control.0.shield_held || held.spec.throws_on_plain_attack()) {
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
    commands.spawn_room_scoped((
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

/// Held-shot-specific gameplay for an in-flight ranged item. Position and
/// velocity live in shared [`BodyKinematics`]; this component carries damage,
/// range traveled, and optional splash radius.
#[derive(Component, Clone, Copy, Debug)]
pub struct HeldProjectile {
    pub damage: i32,
    pub traveled: f32,
    /// Half-extent of an explosion this shot triggers when it hits something.
    /// `0.0` for a plain bolt (the gun-sword); a Fireball sets it so the impact
    /// deals splash damage to everything in the box, not just the first body.
    pub explode_half: f32,
}

const HELD_SHOT_MAX_RANGE: f32 = 1600.0;
const HELD_SHOT_HALF: Vec2 = Vec2::new(12.0, 9.0);

impl HeldProjectile {
    /// The box that actually registers a hit on a body this tick, centered on the
    /// body's current `pos`. ONE source of truth shared by the collision system
    /// (`held_projectile_step`) and the debug overlay so the drawn box can never
    /// drift from the box that hits — the cause of the "fireball hits before it
    /// touches the visible box" report was that this contact box was never drawn.
    pub fn contact_aabb(pos: Vec2) -> ae::Aabb {
        ae::Aabb::new(pos, HELD_SHOT_HALF)
    }

    /// The splash box a Fireball detonates with on contact (`None` for a plain
    /// bolt). Drawn faintly around an in-flight fireball so the player can see
    /// the whole area-of-effect that will trigger, not just the thin bolt.
    pub fn splash_aabb(&self, pos: Vec2) -> Option<ae::Aabb> {
        (self.explode_half > 0.0).then(|| ae::Aabb::new(pos, Vec2::splat(self.explode_half)))
    }
}

/// Held-item id of the Fireball ability — a ranged held item whose shot
/// explodes on contact (see [`fire_held_ranged_system`]).
pub const FIREBALL_ID: &str = "fireball";

/// Splash half-extent a Fireball shot detonates with on contact.
const FIREBALL_EXPLODE_HALF: f32 = 56.0;

/// Detonate a Fireball shot at `pos`: a boxed splash `HitEvent` (damages every
/// body in the box, not just the first), an explosion VFX, and a boom SFX. A
/// free fn (not a closure) so it can borrow the loop's writers at each call site
/// without holding them across the projectile loop.
fn emit_fireball_explosion(
    pos: Vec2,
    damage: i32,
    half: f32,
    attacker: Option<Entity>,
    feature_damage: &mut MessageWriter<crate::features::HitEvent>,
    sfx: &mut MessageWriter<ambition_sfx::SfxMessage>,
    vfx: &mut MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    feature_damage.write(crate::features::HitEvent {
        volume: ae::Aabb::new(pos, Vec2::splat(half)).into(),
        damage,
        source: crate::features::HitSource::PlayerProjectile {
            kind: crate::projectile::ProjectileKind::Fireball,
        },
        attacker,
        target: crate::features::HitTarget::Volume,
        mode: crate::features::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos,
    });
    vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
        pos,
        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
        scale: 1.0,
    });
}

/// Legacy screen/raw aim helper. Prefer [`held_shot_aim_local`] or
/// [`held_shot_aim_world`] in gameplay systems so aiming crosses the input seam
/// once and then lives in the controlled body's frame.
pub fn held_shot_aim(control: &ControlFrame, facing: f32) -> Vec2 {
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

/// Resolve held-item aim into the controlled body's local frame, choosing the
/// frame policy by INPUT SOURCE per [`ae::ControlFrameModes`]: the precision-aim
/// stick wins (precision aiming → `modes.aim`), then the movement stick
/// (locomotion → `modes.movement`), then local facing. Thin `ControlFrame`
/// adapter over [`ae::AccelerationFrame::resolve_aim_local`].
pub fn held_shot_aim_local(
    control: &ControlFrame,
    facing: f32,
    frame: ae::AccelerationFrame,
    modes: ae::ControlFrameModes,
) -> Vec2 {
    frame.resolve_aim_local(
        modes,
        Vec2::new(control.aim_x, control.aim_y),
        Vec2::new(control.axis_x, control.axis_y),
        facing,
    )
}

/// Resolve held-item aim into world space after crossing the input seam through
/// [`held_shot_aim_local`].
pub fn held_shot_aim_world(
    control: &ControlFrame,
    facing: f32,
    gravity_dir: Vec2,
    modes: ae::ControlFrameModes,
) -> Vec2 {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    frame.to_world(held_shot_aim_local(control, facing, frame, modes))
}

/// Body-generic ability aim in the CONTROLLED BODY'S LOCAL frame, taken from the
/// brain-resolved [`ActorControlFrame::aim`] (the brain already crossed the input
/// seam via the aim frame mode), falling back to local facing when neutral. This
/// is the subject-generic counterpart to [`held_shot_aim_local`]: it reads the
/// body's `ActorControl` (present on ANY controlled body — player or possessed
/// actor) rather than a player-only `ControlFrame`, so an ability fires from
/// whichever body is being driven.
pub fn ability_aim_local(
    control: &ambition_characters::actor::control::ActorControlFrame,
    facing: f32,
) -> Vec2 {
    // Match `held_shot_aim_local`'s fallback chain, but off the brain-resolved
    // frame: the aim stick, else the movement stick (`locomotion` — so you can
    // steer a held-item cast with the direction you're moving), else facing.
    if control.aim.length() > 0.1 {
        control.aim
    } else if control.locomotion.length() > 0.1 {
        control.locomotion
    } else {
        Vec2::new(facing, 0.0)
    }
}

/// [`ability_aim_local`] rotated into world space for the body's gravity frame.
pub fn ability_aim_world(
    control: &ambition_characters::actor::control::ActorControlFrame,
    facing: f32,
    gravity_dir: Vec2,
) -> Vec2 {
    ae::AccelerationFrame::new(gravity_dir).to_world(ability_aim_local(control, facing))
}

// Pending wiring point for the OPEN input reference-frame design (gravity-relative
// vs screen-relative joystick mapping — see frame-of-reference.md): reads the user's
// control-frame preference, but nothing applies it YET. Kept (not deleted) as the
// seam the reference-frame slice wires in; `allow(dead_code)` so the -D-warnings CI
// build stays clean until then. See code_smells 2026-07-03.
#[allow(dead_code)]
pub(crate) fn control_frame_modes_from_settings(
    settings: Option<&crate::persistence::settings::UserSettings>,
) -> ae::ControlFrameModes {
    settings.map_or(ae::ControlFrameModes::default(), |s| {
        s.gameplay.control_frame_modes()
    })
}

fn gravity_dir_at(gravity: &crate::physics::GravityCtx, pos: Vec2) -> Vec2 {
    gravity.dir_at(pos)
}

/// `Attack` while holding a *ranged* item fires a laser bolt along the aim
/// direction. `Shield + Attack` is the throw/drop gesture, so don't fire on it.
pub fn fire_held_ranged_system(
    gravity: crate::physics::GravityCtx,
    mut commands: Commands,
    // SUBJECT-GENERIC held-weapon fire: acts on the `ControlledSubject`, reading
    // that body's OWN `ActorControl` (brain output) + `HeldItem`. No
    // `With<PlayerEntity>` filter, no `PlayerInputFrame` — a possessed body firing
    // its held gun works exactly like the home avatar.
    controlled: Res<crate::abilities::traversal::possession::ControlledSubject>,
    bodies: Query<(&ActorControl, &BodyKinematics, &HeldItem)>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((control, kin, held)) = bodies.get(subject) else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    let Some(ranged) = held.spec.ranged else {
        return;
    };
    let gravity_dir = gravity_dir_at(&gravity, kin.pos);
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let local_dir = ability_aim_local(&c, kin.facing);
    let dir = frame.to_world(local_dir).normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    let muzzle_side = if local_dir.x.abs() > 0.001 {
        local_dir.x.signum()
    } else {
        kin.facing.signum()
    };
    let muzzle = frame.to_world(Vec2::new(
        muzzle_side * (kin.size.x * 0.5 + 8.0),
        -kin.size.y * 0.12,
    ));
    let origin = kin.pos + muzzle;
    // A Fireball shot explodes on contact; every other ranged held item fires a
    // plain single-target bolt (`explode_half` 0).
    let explode_half = if held.spec.id == FIREBALL_ID {
        FIREBALL_EXPLODE_HALF
    } else {
        0.0
    };
    #[allow(unused_mut)]
    let mut shot = commands.spawn_room_scoped((
        // Position + velocity live in the shared body; size matches contact.
        BodyKinematics {
            pos: origin,
            vel: dir * ranged.speed(),
            size: HELD_SHOT_HALF * 2.0,
            facing: if dir.x >= 0.0 { 1.0 } else { -1.0 },
        },
        // The projectile *marker*: excludes the bolt from actor-generic queries
        // (auto-righting, actor portal tagging). Its kinematics are driven by
        // `held_projectile_step` (keyed on `HeldProjectile`), not the ECS
        // projectile step (keyed on `PlayerProjectile`), so this marker never
        // double-steps the bolt.
        crate::projectile::ProjectileGameplay {
            age: 0.0,
            max_lifetime: f32::MAX,
            gravity: 0.0,
            damage: ranged.damage(),
            bounces_remaining: 0,
            // Stepped by `held_projectile_step` (keyed on `HeldProjectile`), not
            // the ECS projectile world-collision path, so this is inert here; a
            // detonate-on-contact bolt is `ExpireOnContact` in spirit.
            world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
        },
        HeldProjectile {
            damage: ranged.damage(),
            traveled: 0.0,
            explode_half,
        },
        Name::new("Held ranged shot"),
    ));
    // Opt the bolt into the ONE generic portal transit AT SPAWN (not via the
    // deferred `ensure_projectile_portal_bodies`), so the host-surface carve opens
    // the SAME frame even for a point-blank shot at a portal — otherwise the bolt
    // would detonate on the still-solid surface one frame before it gets tagged.
    // `reorient: false, carry_velocity: true` is the free-flying projectile policy.
    #[cfg(feature = "portal")]
    shot.insert((
        crate::portal::PortalBody,
        crate::portal::PortalPolicy {
            reorient: false,
            carry_velocity: true,
        },
    ));
    let _ = &shot;
    // Fireball currently reuses the dash whoosh instead of the gun-sword zap.
    let fire_sfx = if held.spec.id == FIREBALL_ID {
        ambition_sfx::ids::PLAYER_DASH
    } else {
        ambition_sfx::SfxId::from_static("weapon.lasersword.fire")
    };
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: fire_sfx,
        pos: origin,
    });
}

/// Advance held ranged shots; damage the first feature they overlap, or expire
/// on a solid wall / past max range.
#[allow(clippy::too_many_arguments)]
pub fn held_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
    overlay: Res<crate::features::FeatureEcsWorldOverlay>,
    mut commands: Commands,
    // `Without<FeatureSimEntity>` keeps this `&mut BodyKinematics` disjoint from
    // the boss cluster query below (which reads `BodyKinematics` via
    // `BossClusterRef`) — a held bolt is never a feature-sim entity (B0001).
    mut projectiles: Query<
        (Entity, &mut BodyKinematics, &mut HeldProjectile),
        Without<crate::features::FeatureSimEntity>,
    >,
    player: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    ecs_breakables: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            &crate::features::BreakableFeature,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    ecs_actors: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            &crate::features::ActorDisposition,
            &ambition_characters::actor::BodyCombat,
        ),
        (
            With<crate::features::FeatureSimEntity>,
            Without<crate::features::BossConfig>,
        ),
    >,
    ecs_bosses: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            crate::features::BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &ambition_characters::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    mut feature_damage: MessageWriter<crate::features::HitEvent>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Collide against the room world with ONLY the portal apertures carved out: a
    // portal punched through a wall leaves the opening non-solid, so a bolt fired
    // at a wall portal flies INTO the opening instead of detonating on the wall —
    // and `portal_transit` (which already moves this bolt's `BodyKinematics`)
    // carries it out the far portal. Carves-only preserves the bolt's historical
    // raw-world collision (it passes through moving platforms).
    let collision_world =
        crate::features::world_with_portal_carves(&world.0, &overlay.portal_carves);
    let attacker = player.single().ok();
    for (entity, mut kin, mut proj) in &mut projectiles {
        let pos = kin.pos;
        let vel = kin.vel;
        // Damage check against actors / bosses / breakables via the shared
        // attacker-side channel. `PlayerProjectile` broadcasts to features.
        let hit_event = crate::features::HitEvent {
            volume: HeldProjectile::contact_aabb(pos).into(),
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
            if proj.explode_half > 0.0 {
                // Fireball: the splash box covers the body we hit plus anything
                // around it, so skip the single-target write and detonate.
                emit_fireball_explosion(
                    pos,
                    proj.damage,
                    proj.explode_half,
                    attacker,
                    &mut feature_damage,
                    &mut sfx,
                    &mut vfx,
                );
            } else {
                feature_damage.write(hit_event);
                sfx.write(ambition_sfx::SfxMessage::Hit { pos });
            }
            commands.entity(entity).despawn();
            continue;
        }
        // Solid wall in this step → impact + expire (Fireball detonates here too).
        // Uses the carved world, so a portal opening is NOT a wall.
        let step = (vel * dt).length().max(1.0);
        if let Some((hit_pos, _normal)) = crate::platformer_runtime::collision::raycast_solids(
            &*collision_world,
            pos,
            vel,
            step,
            false,
        ) {
            if proj.explode_half > 0.0 {
                emit_fireball_explosion(
                    hit_pos,
                    proj.damage,
                    proj.explode_half,
                    attacker,
                    &mut feature_damage,
                    &mut sfx,
                    &mut vfx,
                );
            } else {
                vfx.write(ambition_vfx::vfx::VfxMessage::Impact { pos: hit_pos });
            }
            commands.entity(entity).despawn();
            continue;
        }
        let delta = vel * dt;
        kin.pos += delta;
        proj.traveled += delta.length();
        let oob = kin.pos.x < -64.0
            || kin.pos.y < -64.0
            || kin.pos.x > world.0.size.x + 64.0
            || kin.pos.y > world.0.size.y + 64.0;
        if proj.traveled > HELD_SHOT_MAX_RANGE || oob {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
