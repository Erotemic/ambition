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
    mut grounds: Query<&mut GroundItem>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for mut item in &mut grounds {
        if item.vel == Vec2::ZERO {
            continue;
        }
        item.vel.y += GROUND_ITEM_GRAVITY * dt;
        let next = item.pos + item.vel * dt;
        let next_aabb = ae::Aabb::new(next, item.half_extent);
        let blocked = world.0.blocks.iter().any(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            ) && next_aabb.strict_intersects(block.aabb)
        });
        let below_world = next.y > world.0.size.y + 200.0;
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

/// `Attack` while empty-handed and overlapping a `GroundItem` picks it up:
/// stash the current action set, overlay the item's verbs, attach `HeldItem`.
pub fn pickup_held_item_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &PlayerKinematics, &mut ActionSet),
        (With<PlayerEntity>, With<PrimaryPlayer>, Without<HeldItem>),
    >,
    grounds: Query<(Entity, &GroundItem)>,
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
            ground.spec.apply_to_action_set(&mut action_set);
            commands
                .entity(player)
                .insert(HeldItem::new(ground.spec.clone()));
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
// Presentation (visible build only).

/// Marks a sprite entity visualizing a [`GroundItem`].
#[derive(Component)]
pub struct GroundItemVisual;

/// Colored quad per ground item so it's visible. Clear-and-rebuild (few items).
pub fn sync_ground_item_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    visuals: Query<Entity, With<GroundItemVisual>>,
    grounds: Query<&GroundItem>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for ground in &grounds {
        let translation = crate::config::world_to_bevy(&world.0, ground.pos, 8.0);
        commands.spawn((
            GroundItemVisual,
            Sprite::from_color(Color::srgb(0.72, 0.52, 0.30), ground.half_extent * 2.0),
            Transform::from_translation(translation),
            Name::new("Ground item visual"),
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
