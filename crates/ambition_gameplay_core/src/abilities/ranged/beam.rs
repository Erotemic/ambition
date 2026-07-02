//! Focus Beam — a player-wielded **directional line attack**: a long, thin,
//! aimed [`Hitbox`] that damages every enemy along its length.
//!
//! This is the third wielded boss-style attack, alongside [`crate::abilities::ranged::shockwave`]
//! (a centered AOE) and [`crate::abilities::ranged::volley`] (a ranged fan). Where the shockwave
//! slams a compact box at the player's feet, the beam reaches *forward* along
//! the aim as a long thin box — a single readable lance that skewers a line of
//! enemies. It is the **smirking_behemoth** (the eye-beam boss) signature
//! gauntlet: defeat the boss whose tell is a focused eye beam, wield the beam
//! yourself ("every boss a failed objective function, learn its attack").
//!
//! Mechanically it rides the same faction-tagged [`Hitbox`] primitive the
//! shockwave uses — a `Player`-faction box damages enemies/bosses through the
//! `apply_hitbox_damage` player branch, not the player. The box is axis-aligned
//! (the `Hitbox` primitive carries no rotation), so the aim snaps to its
//! dominant axis: a mostly-horizontal aim fires a wide horizontal lance, a
//! mostly-vertical aim fires a tall vertical one. Diagonal aim resolves to
//! whichever axis dominates — good enough for a first pass; a rotated beam is a
//! feel/visual follow-up.

use ambition_characters::brain::ActorControl;
use bevy::prelude::*;

use crate::abilities::traversal::possession::ControlledSubject;
use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::features::{ActorFaction, HeldItem};
use ambition_engine_core as ae;

/// Held-item id of the focus-beam gauntlet.
pub const BEAM_ID: &str = "beam";

/// Mana the beam spends per zap (out of 100). The priciest of the three wielded
/// attacks — it's a strong, long-reach, line-clearing hit, so it's gated harder.
const BEAM_MANA_COST: f32 = 30.0;

/// Beam length (px) along the aim axis — how far forward it reaches.
const BEAM_LENGTH: f32 = 300.0;
/// Beam thickness (px) across the aim axis.
const BEAM_WIDTH: f32 = 30.0;
const BEAM_DAMAGE: i32 = 5;
const BEAM_LIFETIME_S: f32 = 0.12;
const BEAM_KNOCKBACK: f32 = 1.1;

/// Resolve the beam's axis-aligned geometry from an aim vector. Snaps to the
/// dominant axis and returns `(center_offset_from_player, half_extent)` so the
/// box reaches `BEAM_LENGTH` forward along that axis. A zero aim falls back to
/// `facing` (a forward horizontal lance), so a plain Attack with no directional
/// hold still fires.
fn beam_geometry(aim: ae::Vec2, facing: f32) -> (ae::Vec2, ae::Vec2) {
    let half_len = BEAM_LENGTH * 0.5;
    let half_wid = BEAM_WIDTH * 0.5;
    // Pick the dominant axis; default to horizontal-facing on a null aim.
    let horizontal = if aim == ae::Vec2::ZERO {
        true
    } else {
        aim.x.abs() >= aim.y.abs()
    };
    if horizontal {
        let dir = if aim.x.abs() > 0.001 {
            aim.x.signum()
        } else {
            facing.signum()
        };
        (
            ae::Vec2::new(dir * half_len, 0.0),
            ae::Vec2::new(half_len, half_wid),
        )
    } else {
        let dir = aim.y.signum();
        (
            ae::Vec2::new(0.0, dir * half_len),
            ae::Vec2::new(half_wid, half_len),
        )
    }
}

/// `Attack` while holding the beam gauntlet fires an aimed line [`Hitbox`] of
/// **Player** faction along the dominant aim axis. Plain Attack only —
/// `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_beam_system(
    gravity: crate::physics::GravityCtx,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &mut BodyMana,
    )>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((entity, control, held, kin, mut mana)) = players.get_mut(subject) else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != BEAM_ID {
        return;
    }
    // Costs mana — out of mana, no beam (the sandbox's fast regen tops it back up).
    if !mana.meter.try_spend(BEAM_MANA_COST) {
        return;
    }
    let gravity_dir = gravity.dir_at(kin.pos);
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let aim = crate::items::pickup::ability_aim_local(&c, kin.facing);
    let (offset_local, half_local) = beam_geometry(aim, kin.facing);
    let offset = frame.to_world(offset_local);
    let half_extent = frame.to_world_half(half_local);
    effects.write(ambition_vfx::EffectRequest {
        owner: entity,
        effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
            center: kin.pos + offset,
            faction: ActorFaction::Player,
            half_extent,
            damage: BEAM_DAMAGE,
            knockback: BEAM_KNOCKBACK,
            lifetime_s: BEAM_LIFETIME_S,
            name: Some("Focus Beam"),
        }),
    });
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;
    use crate::features::{Hitbox, HitboxAnchor};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<ambition_vfx::EffectRequest>();
        // fire_beam emits Effect::DamageBox; apply_effects spawns the hitbox.
        app.add_systems(
            Update,
            (fire_beam_system, ambition_vfx::apply_effects).chain(),
        );
        app
    }

    fn hitboxes(app: &mut App) -> Vec<Hitbox> {
        app.world_mut()
            .query::<&Hitbox>()
            .iter(app.world())
            .cloned()
            .collect()
    }

    #[test]
    fn attack_with_the_beam_spawns_one_player_faction_line_hitbox() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, BEAM_ID);
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        let boxes = hitboxes(&mut app);
        assert_eq!(boxes.len(), 1, "one beam hitbox spawned");
        assert_eq!(
            boxes[0].source,
            ActorFaction::Player,
            "beam carries the player's faction so it damages enemies, not the player"
        );
        assert_eq!(boxes[0].owner, player);
        // Default facing (+x), no directional hold → a forward HORIZONTAL lance:
        // long along x, thin along y, offset forward of the player.
        assert!(
            boxes[0].half_extent.x > boxes[0].half_extent.y,
            "horizontal beam is long along x; got {:?}",
            boxes[0].half_extent
        );
        if let HitboxAnchor::World { center } = boxes[0].anchor {
            assert!(center.x > 100.0, "beam reaches forward (+x) of the player");
        } else {
            panic!("beam should be world-anchored");
        }
    }

    #[test]
    fn no_beam_without_attack_or_item() {
        let mut app = test_app();
        spawn_primary_player_holding(&mut app, BEAM_ID);
        app.update(); // no attack pressed
        assert_eq!(hitboxes(&mut app).len(), 0);
    }

    #[test]
    fn beam_costs_mana_and_is_blocked_when_empty() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, BEAM_ID);
        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        assert_eq!(hitboxes(&mut app).len(), 0, "no beam when mana < cost");

        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 100.0;
        app.update();
        assert_eq!(hitboxes(&mut app).len(), 1, "fires once there's mana");
        let mana = app.world().get::<BodyMana>(player).unwrap().meter.current;
        assert!(
            (mana - (100.0 - BEAM_MANA_COST)).abs() < 0.01,
            "mana dropped by the cost: {mana}"
        );
    }

    #[test]
    fn vertical_aim_makes_a_tall_thin_beam() {
        // Aim straight up → a tall vertical lance (long along y, thin along x),
        // offset above the player. Engine y grows downward, so "up" is -y.
        let (offset, half) = beam_geometry(ae::Vec2::new(0.0, -1.0), 1.0);
        assert!(
            half.y > half.x,
            "vertical beam is long along y; got {half:?}"
        );
        assert!(offset.y < 0.0, "an up-aimed beam reaches above the player");
    }

    #[test]
    fn beam_geometry_is_c4_equivariant_for_local_aim() {
        let local_aim = ae::Vec2::new(0.0, -1.0);
        let (offset_local, half_local) = beam_geometry(local_aim, 1.0);
        for gravity_dir in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let offset_world = frame.to_world(offset_local);
            let half_world = frame.to_world_half(half_local);
            assert!(
                (frame.to_local(offset_world) - offset_local).length() < 0.001,
                "beam offset should round-trip through gravity {gravity_dir:?}"
            );
            if gravity_dir.x.abs() > gravity_dir.y.abs() {
                assert!(
                    half_world.x > half_world.y,
                    "local vertical beam should become world-horizontal under sideways gravity; got {half_world:?}"
                );
            } else {
                assert!(
                    half_world.y > half_world.x,
                    "local vertical beam should stay world-vertical under vertical gravity; got {half_world:?}"
                );
            }
        }
    }
}
