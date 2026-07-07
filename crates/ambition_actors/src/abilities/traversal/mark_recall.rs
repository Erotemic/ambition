//! Mark / Recall — a held item that drops a teleport mark and recalls to it.
//!
//! Canon ability ([`crate::items::Item::MarkRecall`]): Jon's design — a teleport
//! used for fast travel and combat repositioning (the "Recall" half of the
//! blink family). It's implemented as a **held item** so it reuses the whole
//! equip / stash / throw plumbing instead of inventing an ability-dispatch path:
//!
//! - Equip it (walk over the ground item, or equip the catalog slot), and while
//!   it's held a plain `Attack` **drops / moves the mark** at the player's feet.
//! - The `Blink` button **recalls** the player to the mark (instant teleport).
//! - `Shield + Attack` still throws the item away through the generic
//!   [`crate::items::pickup::throw_held_item_system`] path.
//!
//! The held spec has no melee/ranged verb, so the throw system would normally
//! treat it as a "pure throwable" and throw it on a plain `Attack`. Like the
//! puppy-slug gun, it opts out via that system's `use_on_attack` id check, which
//! leaves `Attack` free to set the mark.
//!
//! One mark per player, stored as a [`PlayerMark`] **component** (not a resource)
//! so each player keeps an independent mark once the multiplayer split lands.
//! A persistent [`MarkBeaconVisual`] glowing-crystal beacon stands at the mark
//! ([`sync_mark_beacon_visual`]) so the player can see where Blink will recall
//! them to; set/recall also emit a VFX burst + SFX cue.

use bevy::prelude::*;

use super::possession::ControlledSubject;
use crate::actor::BodyKinematics;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;

/// The held-item id the Mark/Recall ability grants (see `brain::action_set`
/// `HELD_ITEMS` and `items::Item::held_item_id`).
pub const MARK_RECALL_ID: &str = "mark_recall";

/// Half-extent of the recall-strike shockwave at the mark.
const RECALL_SHOCKWAVE_HALF: f32 = 36.0;
/// Recall-strike damage — modest, like Blink's arrival shockwave.
const RECALL_SHOCKWAVE_DAMAGE: i32 = 2;

/// The teleport mark a player has dropped with the Mark/Recall item, if any.
/// Per-player (a component, not a resource) so the future multiplayer split
/// keeps each player's mark independent.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerMark {
    /// World position of the dropped mark, or `None` until one is set.
    pub pos: Option<ae::Vec2>,
}

/// While holding the Mark/Recall item: a plain `Attack` drops or moves the mark
/// at the player's feet, and `Blink` recalls the player to the mark (if set). A
/// frame that drops a mark does not also recall, so a simultaneous press resolves
/// as "set the mark here" rather than "recall to where I just stood".
pub fn mark_recall_system(
    mut commands: Commands,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &mut BodyKinematics,
        &HeldItem,
        Option<&mut PlayerMark>,
    )>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((player, control, mut kin, held, mut mark)) = players.get_mut(subject) else {
        return;
    };
    let c = control.0;
    if held.spec.id != MARK_RECALL_ID {
        return;
    }

    // Plain Attack drops / moves the mark. Shield+Attack is the generic "throw
    // the item away", so a marked frame must not be a shielded one.
    if c.melee_pressed && !c.shield_held {
        let pos = kin.pos;
        match mark.as_deref_mut() {
            Some(existing) => existing.pos = Some(pos),
            None => {
                commands
                    .entity(player)
                    .insert(PlayerMark { pos: Some(pos) });
            }
        }
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_DASH,
            pos,
        });
        vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
            pos,
            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
            scale: 0.4,
        });
        return;
    }

    // Blink recalls to the mark, if one is set.
    if c.blink_pressed {
        if let Some(target) = mark.and_then(|m| m.pos) {
            kin.pos = target;
            // Recall-strike: a player-side shockwave at the mark, so you can mark a
            // spot, lure enemies onto it, and recall in to hit them (mirrors Blink).
            hits.write(crate::features::HitEvent {
                volume: ae::CombatVolume::circle(target, RECALL_SHOCKWAVE_HALF),
                damage: RECALL_SHOCKWAVE_DAMAGE,
                source: crate::features::HitSource::PlayerSlash { knock_x: 0.0 },
                attacker: Some(player),
                target: crate::features::HitTarget::Volume,
                mode: crate::features::HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            });
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PLAYER_BLINK,
                pos: target,
            });
            vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
                pos: target,
                kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
                scale: 0.6,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::SfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<crate::features::HitEvent>();
        app.add_systems(Update, mark_recall_system);
        app
    }

    fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        crate::abilities::test_support::spawn_primary_player_holding_at(app, id, pos, 1.0)
    }

    fn press(app: &mut App, player: Entity, attack: bool, blink: bool) {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = attack;
        control.0.blink_pressed = blink;
        control.0.shield_held = false;
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
    fn recall_emits_a_player_side_shockwave_at_the_mark() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(
            bevy::prelude::Update,
            capture_hits.after(mark_recall_system),
        );
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(200.0, 80.0));
        press(&mut app, player, true, false); // mark at (200,80) — no hit yet
        app.update();
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(900.0, 50.0);
        press(&mut app, player, false, true); // recall -> shockwave at the mark
        app.update();
        let hits = &app.world().resource::<CapturedHits>().0;
        assert_eq!(hits.len(), 1, "one shockwave on recall");
        let cx = (hits[0].volume.bounds().min.x + hits[0].volume.bounds().max.x) * 0.5;
        assert!((cx - 200.0).abs() < 1.0, "shockwave centered on the mark");
        assert!(
            matches!(
                hits[0].source,
                crate::features::HitSource::PlayerSlash { .. }
            ),
            "player-side so it spares the player",
        );
    }

    #[test]
    fn attack_sets_a_mark_then_blink_recalls_to_it() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(100.0, 100.0));
        // Drop a mark where we stand.
        press(&mut app, player, true, false);
        app.update();
        assert_eq!(
            app.world().get::<PlayerMark>(player).and_then(|m| m.pos),
            Some(ae::Vec2::new(100.0, 100.0)),
            "Attack stored a mark at the player's position",
        );
        // Wander far away, then recall.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(900.0, 50.0);
        press(&mut app, player, false, true);
        app.update();
        assert_eq!(
            player_pos(&app, player),
            ae::Vec2::new(100.0, 100.0),
            "Blink recalled the player to the mark",
        );
    }

    #[test]
    fn re_marking_moves_the_single_mark() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(10.0, 10.0));
        press(&mut app, player, true, false);
        app.update(); // mark at (10,10)
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(400.0, 20.0);
        press(&mut app, player, true, false);
        app.update(); // re-mark at (400,20) — should replace, not add a second
        assert_eq!(
            app.world().get::<PlayerMark>(player).and_then(|m| m.pos),
            Some(ae::Vec2::new(400.0, 20.0)),
            "the single mark moved to the newest drop",
        );
    }

    #[test]
    fn blink_without_a_mark_is_a_no_op() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(900.0, 50.0));
        press(&mut app, player, false, true);
        app.update();
        assert_eq!(
            player_pos(&app, player),
            ae::Vec2::new(900.0, 50.0),
            "no mark set → Blink does not teleport",
        );
    }

    #[test]
    fn a_different_held_item_never_marks() {
        // Holding the bomb (also a pure throwable) must not trip the mark logic.
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, "bomb", ae::Vec2::new(100.0, 100.0));
        press(&mut app, player, true, false);
        app.update();
        assert!(
            app.world().get::<PlayerMark>(player).is_none(),
            "Attack while holding a different item sets no mark",
        );
    }
}
