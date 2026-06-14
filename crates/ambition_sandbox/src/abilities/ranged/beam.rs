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

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::{ActorFaction, HeldItem};
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerMana, PrimaryPlayer};

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
    control: Res<ControlFrame>,
    mut players: Query<
        (Entity, &HeldItem, &BodyKinematics, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((entity, held, kin, mut mana)) = players.single_mut() else {
        return;
    };
    if held.spec.id != BEAM_ID {
        return;
    }
    // Costs mana — out of mana, no beam (the sandbox's fast regen tops it back up).
    if !mana.meter.try_spend(BEAM_MANA_COST) {
        return;
    }
    let aim = crate::items::pickup::held_shot_aim(&control, kin.facing);
    let (offset, half_extent) = beam_geometry(aim, kin.facing);
    effects.write(crate::effects::EffectRequest {
        owner: entity,
        effect: crate::effects::Effect::DamageBox(crate::effects::DamageBoxEffect {
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
    use crate::features::{Hitbox, HitboxAnchor};
    use crate::abilities::test_support::spawn_primary_player_holding;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.insert_resource(ControlFrame::default());
        // fire_beam emits Effect::DamageBox; apply_effects spawns the hitbox.
        app.add_systems(
            Update,
            (fire_beam_system, crate::effects::apply_effects).chain(),
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
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
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
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
        app.update();
        assert_eq!(hitboxes(&mut app).len(), 0, "no beam when mana < cost");

        app.world_mut()
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .current = 100.0;
        app.update();
        assert_eq!(hitboxes(&mut app).len(), 1, "fires once there's mana");
        let mana = app.world().get::<PlayerMana>(player).unwrap().meter.current;
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
}
