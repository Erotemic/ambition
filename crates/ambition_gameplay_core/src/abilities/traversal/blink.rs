//! Blink — a held item granting a short-range directional teleport.
//!
//! Canon ability ([`crate::items::Item::Blink`]): Jon's note — "Short-range
//! teleport. Your favorite, and high-skill." Implemented as a wired ability (a
//! held item) like Mark/Recall and Fireball, so it reuses the equip / OoT-menu /
//! throw plumbing. While holding it, `Attack` blinks the player a fixed distance
//! along the aim direction, **collision-clamped**: a `raycast_solids` stops the
//! teleport just short of the first wall so you can't blink through or embed in a
//! solid (the "collision safety policy" the blink design calls for).
//!
//! Stateless (no mark to store), so there's nothing to clear on reset. Like the
//! other pure-use held items it has no melee/ranged verb and opts out of
//! throw-on-attack via `throw_held_item_system`'s `use_on_attack` id check.

use bevy::prelude::*;

use super::possession::ControlledSubject;
use crate::actor::BodyKinematics;
use crate::actor::PlayerEntity;
use crate::features::HeldItem;
use crate::player::PlayerInputFrame;
use ambition_engine_core::{self as ae, AabbExt};

/// The held-item id the Blink ability grants.
pub const BLINK_ID: &str = "blink";

/// How far a blink carries the player along the aim direction, walls permitting.
const BLINK_DISTANCE: f32 = 150.0;

/// Cooldown between blinks, so it reads as a deliberate reposition (not spam).
const BLINK_COOLDOWN_S: f32 = 0.45;

/// Half-extent of the arrival shockwave that lets you blink offensively into a
/// cluster of enemies.
const BLINK_SHOCKWAVE_HALF: f32 = 36.0;
/// Shockwave damage — modest; Blink is mobility first, a light strike second.
const BLINK_SHOCKWAVE_DAMAGE: i32 = 2;

/// Resolve a blink destination over `world`: teleport up to `distance` along the
/// unit `dir`, stopping a body-half (`half`, measured in the blink direction)
/// short of the first solid so the body never embeds, with a safety net that
/// falls back to `from` if the landing box would still overlap a solid.
///
/// This is the **one teleport rule** shared by every controller: the player's
/// held-item blink and any actor body that resolves a `blink` intent from its
/// `ActorControlFrame` call the same function (invariants I2/I7 — a possessed or
/// AI body blinks exactly as the player does, against the same collision world it
/// physically occupies).
pub fn blink_target(
    world: &ae::World,
    from: ae::Vec2,
    dir: ae::Vec2,
    distance: f32,
    half: ae::Vec2,
) -> ae::Vec2 {
    // Pull-back must use the body's extent IN the blink direction — a vertical
    // blink needs half-height, not half-width — or a diagonal blink embeds.
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    let mut target = match crate::platformer_runtime::collision::raycast_solids(
        world,
        from,
        dir,
        distance + margin,
        false,
    ) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * distance,
    };
    // Safety net: the center-ray can miss a wall the body's perpendicular extent
    // would clip (corners, grazing). If the landing box still overlaps a solid,
    // fall back to the start so a blink never lands inside geometry.
    let landing = ae::Aabb::new(target, half);
    let embeds = world.blocks.iter().any(|b| {
        matches!(
            b.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) && landing.strict_intersects(b.aabb)
    });
    if embeds {
        target = from;
    }
    target
}

/// `Attack` while holding the Blink ability teleports the player up to
/// [`BLINK_DISTANCE`] along the aim direction, stopping a body-half short of the
/// first solid wall so the teleport never lands inside geometry.
pub fn blink_system(
    gravity: crate::physics::GravityCtx,
    user_settings: Option<Res<crate::persistence::settings::UserSettings>>,
    world: crate::features::CollisionWorld,
    mut commands: Commands,
    // Ability ORIGIN = the controlled subject (the body carrying
    // `Brain::Player(PRIMARY)`), not a `PrimaryPlayer` filter. Blinks the body you
    // are DRIVING; a vacated home avatar (not the subject) is skipped, and a
    // possessed body only blinks its own held item (it usually holds none).
    controlled: Res<ControlledSubject>,
    mut players: Query<
        (
            Entity,
            &PlayerInputFrame,
            &mut BodyKinematics,
            &HeldItem,
            Option<&mut crate::ability_cooldown::AbilityCooldown>,
        ),
        With<PlayerEntity>,
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((player, input, mut kin, held, mut cooldown)) = players.get_mut(subject) else {
        return;
    };
    // Plain Attack blinks; Shield+Attack is the generic "throw the item away".
    if !input.frame.attack_pressed || input.frame.shield_held {
        return;
    }
    if held.spec.id != BLINK_ID {
        return;
    }
    // Aim in the controlled body's input frame, then resolve to world-space for
    // the raycast/teleport. The gameplay move is body-relative; the raycast is
    // naturally world-space.
    let gravity_dir = gravity.dir_at(kin.pos);
    let modes = crate::items::pickup::control_frame_modes_from_settings(user_settings.as_deref());
    let dir =
        crate::items::pickup::held_shot_aim_world(&input.frame, kin.facing, gravity_dir, modes)
            .normalize_or_zero();
    if dir == ae::Vec2::ZERO {
        return;
    }
    // Gate on the shared movement-ability cooldown (after confirming a real blink
    // so an aimless press doesn't burn it).
    if !crate::ability_cooldown::try_use_ability(
        &mut cooldown,
        &mut commands,
        player,
        BLINK_COOLDOWN_S,
    ) {
        return;
    }
    let from = kin.pos;
    let half = kin.size * 0.5;
    // One composited collision view (moving platforms + ECS solids included),
    // shared by the clamp raycast and the embed safety net inside `blink_target`.
    let collision = world.solids();
    let target = match collision.as_ref() {
        Some(w) => blink_target(&**w, from, dir, BLINK_DISTANCE, half),
        // No collision world (tests / degenerate) — blink the full distance.
        None => from + dir * BLINK_DISTANCE,
    };
    kin.pos = target;
    // Offensive blink: a small player-side shockwave at the arrival point, so you
    // can blink *into* enemies to strike them (and the PlayerSlash source spares
    // the player). Composes nicely with a gravity well — blink in, sweep them up.
    hits.write(crate::features::HitEvent {
        volume: ae::CombatVolume::circle(target, BLINK_SHOCKWAVE_HALF),
        damage: BLINK_SHOCKWAVE_DAMAGE,
        source: crate::features::HitSource::PlayerSlash { knock_x: 0.0 },
        attacker: Some(player),
        target: crate::features::HitTarget::Volume,
        mode: crate::features::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_BLINK,
        pos: target,
    });
    // A wisp where you left, a flash where you arrive.
    vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
        pos: from,
        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
        scale: 0.35,
    });
    vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
        pos: target,
        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
        scale: 0.5,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared teleport rule (used by both the player blink and any actor
    /// body): full distance over open space, clamped a body-half short of a wall,
    /// never embedding. This is the single invariant both controllers inherit.
    #[test]
    fn blink_target_travels_full_distance_then_clamps_at_a_wall() {
        let half = ae::Vec2::new(12.0, 20.0);
        // Open world (no blocks): blink the full distance to the right.
        let empty = ae::World::new("t", ae::Vec2::new(2000.0, 600.0), ae::Vec2::ZERO, vec![]);
        let from = ae::Vec2::new(0.0, 0.0);
        let open = blink_target(&empty, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
        assert!(
            (open.x - 150.0).abs() < 1e-3,
            "open blink travels full distance: {open:?}"
        );

        // A wall whose left face is at x=100 (Block::solid takes the MIN corner):
        // the body stops a half-width (+margin) short of it, never crossing in.
        let walled = ae::World::new(
            "t",
            ae::Vec2::new(2000.0, 600.0),
            ae::Vec2::ZERO,
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(100.0, -300.0),
                ae::Vec2::new(120.0, 600.0),
            )],
        );
        let clamped = blink_target(&walled, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
        assert!(
            clamped.x + half.x <= 100.0 + 1e-3,
            "clamped blink must not cross the wall's left face at x=100: right edge={}",
            clamped.x + half.x
        );
        assert!(
            clamped.x > 0.0,
            "but it should still carry toward the wall: {clamped:?}"
        );
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<crate::features::HitEvent>();
        app.add_systems(Update, blink_system);
        app
    }

    fn spawn_player_holding(app: &mut App, id: &str, facing: f32) -> Entity {
        crate::abilities::test_support::spawn_primary_player_holding_at(
            app,
            id,
            ae::Vec2::new(300.0, 300.0),
            facing,
        )
    }

    fn player_pos(app: &App, player: Entity) -> ae::Vec2 {
        app.world().get::<BodyKinematics>(player).unwrap().pos
    }

    #[derive(bevy::prelude::Resource, Default)]
    struct CapturedHits(Vec<crate::features::HitEvent>);

    fn capture_hits(
        mut reader: bevy::prelude::MessageReader<crate::features::HitEvent>,
        mut out: bevy::prelude::ResMut<CapturedHits>,
    ) {
        out.0.extend(reader.read().cloned());
    }

    #[test]
    fn blink_emits_a_player_side_shockwave_at_arrival() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(bevy::prelude::Update, capture_hits.after(blink_system));
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.world_mut()
            .get_mut::<PlayerInputFrame>(player)
            .unwrap()
            .frame
            .attack_pressed = true;
        app.update();
        let hits = &app.world().resource::<CapturedHits>().0;
        assert_eq!(hits.len(), 1, "one shockwave on arrival");
        // Centered at the arrival point (300 + BLINK_DISTANCE along facing).
        let center_x = (hits[0].volume.bounds().min.x + hits[0].volume.bounds().max.x) * 0.5;
        assert!(
            (center_x - (300.0 + BLINK_DISTANCE)).abs() < 1.0,
            "shockwave is at the arrival point",
        );
        assert_eq!(hits[0].damage, BLINK_SHOCKWAVE_DAMAGE);
        assert!(
            matches!(
                hits[0].source,
                crate::features::HitSource::PlayerSlash { .. }
            ),
            "player-side source so it spares the player",
        );
    }

    #[test]
    fn attack_blinks_the_player_forward_along_facing() {
        // No RoomGeometry inserted → the no-clamp branch teleports the full distance.
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.world_mut()
            .get_mut::<PlayerInputFrame>(player)
            .unwrap()
            .frame
            .attack_pressed = true;
        app.update();
        assert_eq!(
            player_pos(&app, player),
            ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
            "blink carried the player one BLINK_DISTANCE along facing",
        );
    }

    #[test]
    fn downward_blink_does_not_embed_in_the_floor() {
        // Regression: a vertical blink must pull back by the body's half-HEIGHT,
        // not half-width, or the 40-tall body embeds in the floor and trips the
        // inside-solid OOB detector (the fly + aim-down blink case).
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0); // (300,300), 24x40
                                                                    // Solid floor whose top edge is at y=350, just below the player.
        app.insert_resource(crate::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(300.0, 300.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 350.0),
                ae::Vec2::new(600.0, 250.0),
            )],
        )));
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame.attack_pressed = true;
            input.frame.aim_y = 1.0; // aim straight down
        }
        app.update();
        let pos = player_pos(&app, player);
        let half_h = 20.0;
        assert!(
            pos.y + half_h <= 350.0 + 1e-3,
            "downward blink embedded the body in the floor: bottom={}, floor top=350",
            pos.y + half_h,
        );
        assert!(
            pos.y > 300.0,
            "the blink should still carry the player toward the floor (got y={})",
            pos.y,
        );
    }

    #[test]
    fn blink_follows_facing_left() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, -1.0);
        app.world_mut()
            .get_mut::<PlayerInputFrame>(player)
            .unwrap()
            .frame
            .attack_pressed = true;
        app.update();
        assert_eq!(
            player_pos(&app, player),
            ae::Vec2::new(300.0 - BLINK_DISTANCE, 300.0),
            "a left-facing blink goes left",
        );
    }

    #[test]
    fn no_blink_without_attack_or_with_a_different_item() {
        // Holding blink but not attacking → stays put.
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.update();
        assert_eq!(player_pos(&app, player), ae::Vec2::new(300.0, 300.0));
        // Holding the bomb + attacking → blink_system ignores it.
        let mut app2 = test_app();
        let player2 = spawn_player_holding(&mut app2, "bomb", 1.0);
        app2.world_mut()
            .get_mut::<PlayerInputFrame>(player2)
            .unwrap()
            .frame
            .attack_pressed = true;
        app2.update();
        assert_eq!(player_pos(&app2, player2), ae::Vec2::new(300.0, 300.0));
    }
}
