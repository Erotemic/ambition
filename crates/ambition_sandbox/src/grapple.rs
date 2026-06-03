//! Grapple — a held item that yanks the player toward a grappled surface.
//!
//! Canon ability ([`crate::items::Item::Grapple`]): a traversal pull. Implemented
//! as a wired ability (a held item) like Blink / Mark/Recall / Fireball, so it
//! reuses the equip / OoT-menu / throw plumbing. While holding it, `Attack`
//! casts a line along the aim direction; if it lands on a solid wall within
//! [`GRAPPLE_RANGE`], the player is yanked toward the hit at [`GRAPPLE_PULL_SPEED`]
//! (a burst impulse — collision resolution then settles them at the surface).
//! A grapple into empty space fizzles.
//!
//! Stateless, so nothing to clear on reset; opts out of throw-on-attack like the
//! other pure-use abilities.

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

/// The held-item id the Grapple ability grants.
pub const GRAPPLE_ID: &str = "grapple";

/// How far the grapple line reaches for a solid surface.
const GRAPPLE_RANGE: f32 = 300.0;

/// Speed of the burst yank toward a grappled surface.
const GRAPPLE_PULL_SPEED: f32 = 620.0;

/// Cooldown between successful yanks, so grappling reads as deliberate.
const GRAPPLE_COOLDOWN_S: f32 = 0.55;

/// `Attack` while holding the Grapple ability casts along the aim direction; on
/// hitting a solid within [`GRAPPLE_RANGE`] it yanks the player toward the hit.
pub fn grapple_system(
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
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((player, mut kin, held, mut cooldown)) = players.single_mut() else {
        return;
    };
    if held.spec.id != GRAPPLE_ID {
        return;
    }
    let dir = crate::item_pickup::held_shot_aim(&control, kin.facing);
    if dir == ae::Vec2::ZERO {
        return;
    }
    let from = kin.pos;
    let Some((hit, _normal)) = world
        .as_ref()
        .and_then(|w| crate::portal::raycast_solids(&w.0, from, dir, GRAPPLE_RANGE))
    else {
        // Grapple into empty space: a dry fizzle, no pull (and no cooldown burned).
        sfx.write(crate::audio::SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_DASH,
            pos: from,
        });
        return;
    };
    // Only a successful latch is on cooldown — a miss above costs nothing.
    if !crate::ability_cooldown::try_use_ability(
        &mut cooldown,
        &mut commands,
        player,
        GRAPPLE_COOLDOWN_S,
    ) {
        return;
    }
    // Yank toward the latched surface (collision resolution settles the player at
    // it). A burst velocity, not a teleport, so the movement reads as a pull.
    let pull = (hit - from).normalize_or_zero();
    kin.vel = pull * GRAPPLE_PULL_SPEED;
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_DASH,
        pos: from,
    });
    vfx.write(crate::presentation::fx::VfxMessage::Impact { pos: hit });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;

    fn world_with_right_wall() -> crate::GameWorld {
        // A solid wall at x[380,400], y[0,400]; open space to its left.
        let blocks = vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(380.0, 0.0),
            ae::Vec2::new(20.0, 400.0),
        )];
        crate::GameWorld(ae::World::new(
            "grapple_test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(100.0, 200.0),
            blocks,
        ))
    }

    fn test_app(world: Option<crate::GameWorld>) -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::presentation::fx::VfxMessage>();
        app.insert_resource(ControlFrame::default());
        if let Some(w) = world {
            app.insert_resource(w);
        }
        app.add_systems(Update, grapple_system);
        app
    }

    fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2, facing: f32) -> Entity {
        let spec = crate::brain::held_item_by_id(id).unwrap();
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos,
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

    fn player_vel(app: &App, player: Entity) -> ae::Vec2 {
        app.world().get::<PlayerKinematics>(player).unwrap().vel
    }

    #[test]
    fn grapple_yanks_the_player_toward_a_grappled_wall() {
        let mut app = test_app(Some(world_with_right_wall()));
        // Player to the left of the wall, facing/aiming right.
        let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
        app.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
        app.update();
        let vel = player_vel(&app, player);
        assert!(vel.x > 0.0, "the yank velocity points toward the wall (right)");
        assert!(
            vel.x.abs() > vel.y.abs(),
            "a horizontal grapple yanks mostly horizontally ({vel:?})"
        );
        assert!(
            (vel.length() - GRAPPLE_PULL_SPEED).abs() < 1.0,
            "the yank is at the pull speed",
        );
    }

    #[test]
    fn grapple_into_empty_space_does_not_move_the_player() {
        // No world (or no wall in range) → fizzle, velocity untouched.
        let mut app = test_app(None);
        let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
        app.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
        app.update();
        assert_eq!(player_vel(&app, player), ae::Vec2::ZERO, "a dry grapple yanks nothing");
    }

    #[test]
    fn no_grapple_without_attack_or_with_a_different_item() {
        // Holding grapple but not attacking → no pull.
        let mut app = test_app(Some(world_with_right_wall()));
        let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
        app.update();
        assert_eq!(player_vel(&app, player), ae::Vec2::ZERO);
        // Holding the bomb + attacking → grapple_system ignores it.
        let mut app2 = test_app(Some(world_with_right_wall()));
        let player2 = spawn_player_holding(&mut app2, "bomb", ae::Vec2::new(100.0, 200.0), 1.0);
        app2.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
        app2.update();
        assert_eq!(player_vel(&app2, player2), ae::Vec2::ZERO);
    }
}
