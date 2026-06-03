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

use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

/// The held-item id the Blink ability grants.
pub const BLINK_ID: &str = "blink";

/// How far a blink carries the player along the aim direction, walls permitting.
const BLINK_DISTANCE: f32 = 150.0;

/// Cooldown between blinks, so it reads as a deliberate reposition (not spam).
const BLINK_COOLDOWN_S: f32 = 0.45;

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
    let dir = crate::item_pickup::held_shot_aim(&control, kin.facing);
    if dir == ae::Vec2::ZERO {
        return;
    }
    // Gate on the shared movement-ability cooldown (after confirming a real blink
    // so an aimless press doesn't burn it).
    if !crate::ability_cooldown::try_use_ability(&mut cooldown, &mut commands, player, BLINK_COOLDOWN_S)
    {
        return;
    }
    let from = kin.pos;
    // Stop a body-half short of the wall so the player doesn't embed in it.
    let margin = kin.size.x * 0.5 + 2.0;
    let target = match world
        .as_ref()
        .and_then(|w| crate::portal::raycast_solids(&w.0, from, dir, BLINK_DISTANCE + margin))
    {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * BLINK_DISTANCE,
    };
    kin.pos = target;
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

/// Spawn one Blink ground item near the player on the first frame a player exists
/// (debug convenience until authored placement lands), mirroring the puppy-slug
/// gun and Mark/Recall debug drops.
pub fn spawn_debug_blink_once(
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
    let Some(spec) = crate::brain::held_item_by_id(BLINK_ID) else {
        return;
    };
    *done = true;
    commands.spawn((
        crate::item_pickup::GroundItem {
            spec,
            pos: kin.pos + ae::Vec2::new(-160.0, 0.0),
            vel: ae::Vec2::ZERO,
            half_extent: ae::Vec2::splat(18.0),
        },
        Name::new("Ground item: blink"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::presentation::fx::VfxMessage>();
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

    #[test]
    fn attack_blinks_the_player_forward_along_facing() {
        // No GameWorld inserted → the no-clamp branch teleports the full distance.
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
        app.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
        app.update();
        assert_eq!(
            player_pos(&app, player),
            ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
            "blink carried the player one BLINK_DISTANCE along facing",
        );
    }

    #[test]
    fn blink_follows_facing_left() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, BLINK_ID, -1.0);
        app.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
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
        app2.world_mut().resource_mut::<ControlFrame>().attack_pressed = true;
        app2.update();
        assert_eq!(player_pos(&app2, player2), ae::Vec2::new(300.0, 300.0));
    }
}
