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

use crate::engine_core::{self as ae, AabbExt};
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

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

/// `Attack` while holding the Blink ability teleports the player up to
/// [`BLINK_DISTANCE`] along the aim direction, stopping a body-half short of the
/// first solid wall so the teleport never lands inside geometry.
pub fn blink_system(
    control: Res<ControlFrame>,
    world: Option<Res<crate::GameWorld>>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut PlayerKinematics,
            &HeldItem,
            Option<&mut crate::ability_cooldown::AbilityCooldown>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
) {
    // Plain Attack blinks; Shield+Attack is the generic "throw the item away".
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((player, mut kin, held, mut cooldown)) = players.single_mut() else {
        return;
    };
    if held.spec.id != BLINK_ID {
        return;
    }
    // Aim exactly like the ranged held items (right-stick aim, else movement,
    // else facing), so the blink goes where the player is pointing.
    let dir = crate::item_pickup::held_shot_aim(&control, kin.facing).normalize_or_zero();
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
    // Stop a body-half short of the wall so the player doesn't embed in it. The
    // pull-back must use the body's extent IN THE BLINK DIRECTION -- a vertical
    // blink needs half-HEIGHT, not half-width (the player is ~40 tall, ~24 wide),
    // or an up/down/diagonal blink (common while flying + aiming) embeds in the
    // floor/ceiling and trips the inside-solid OOB detector.
    let half = kin.size * 0.5;
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    let mut target = match world.as_ref().and_then(|w| {
        crate::platformer_runtime::collision::raycast_solids(
            &w.0,
            from,
            dir,
            BLINK_DISTANCE + margin,
            false,
        )
    }) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * BLINK_DISTANCE,
    };
    // Safety net: the center-ray can miss a wall the body's *perpendicular* extent
    // would clip (corners, grazing). If the landing box still overlaps a solid,
    // fall back to the start so a blink never lands the player inside geometry.
    if let Some(w) = world.as_ref() {
        let landing = ae::Aabb::new(target, half);
        let embeds = w.0.blocks.iter().any(|b| {
            matches!(
                b.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && landing.strict_intersects(b.aabb)
        });
        if embeds {
            target = from;
        }
    }
    kin.pos = target;
    // Offensive blink: a small player-side shockwave at the arrival point, so you
    // can blink *into* enemies to strike them (and the PlayerSlash source spares
    // the player). Composes nicely with a gravity well — blink in, sweep them up.
    hits.write(crate::features::HitEvent {
        volume: ae::Aabb::new(target, ae::Vec2::splat(BLINK_SHOCKWAVE_HALF)),
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
    vfx.write(crate::presentation::fx::VfxMessage::Explosion {
        pos: from,
        kind: crate::presentation::fx::ExplosionKind::ClassicBurst,
        scale: 0.35,
    });
    vfx.write(crate::presentation::fx::VfxMessage::Explosion {
        pos: target,
        kind: crate::presentation::fx::ExplosionKind::ClassicBurst,
        scale: 0.5,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::presentation::fx::VfxMessage>();
        app.add_message::<crate::features::HitEvent>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, blink_system);
        app
    }

    fn spawn_player_holding(app: &mut App, id: &str, facing: f32) -> Entity {
        let spec = crate::brain::held_item_by_id(id).unwrap();
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: ae::Vec2::new(300.0, 300.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
                    base_size: ae::Vec2::new(24.0, 40.0),
                    facing,
                },
                ActionSet::default(),
                HeldItem::new(spec),
            ))
            .id()
    }

    fn player_pos(app: &App, player: Entity) -> ae::Vec2 {
        app.world().get::<PlayerKinematics>(player).unwrap().pos
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
        let _player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        let hits = &app.world().resource::<CapturedHits>().0;
        assert_eq!(hits.len(), 1, "one shockwave on arrival");
        // Centered at the arrival point (300 + BLINK_DISTANCE along facing).
        let center_x = (hits[0].volume.min.x + hits[0].volume.max.x) * 0.5;
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
        // No GameWorld inserted → the no-clamp branch teleports the full distance.
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.world_mut()
            .resource_mut::<ControlFrame>()
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
        app.insert_resource(crate::GameWorld(ae::World::new(
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
            let mut c = app.world_mut().resource_mut::<ControlFrame>();
            c.attack_pressed = true;
            c.aim_y = 1.0; // aim straight down
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
            .resource_mut::<ControlFrame>()
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
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app2.update();
        assert_eq!(player_pos(&app2, player2), ae::Vec2::new(300.0, 300.0));
    }
}
