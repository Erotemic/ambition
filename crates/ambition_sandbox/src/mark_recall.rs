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
//!   [`crate::item_pickup::throw_held_item_system`] path.
//!
//! The held spec has no melee/ranged verb, so the throw system would normally
//! treat it as a "pure throwable" and throw it on a plain `Attack`. Like the
//! puppy-slug gun, it opts out via that system's `use_on_attack` id check, which
//! leaves `Attack` free to set the mark.
//!
//! One mark per player, stored as a [`PlayerMark`] **component** (not a resource)
//! so each player keeps an independent mark once the multiplayer split lands.
//! The mark has no persistent on-screen beacon yet — set/recall emit a VFX burst
//! and an SFX cue; a beacon sprite is a follow-up (it needs an authored asset,
//! same as the boss-sprite wiring).

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::HeldItem;
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

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
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut PlayerKinematics,
            &HeldItem,
            Option<&mut PlayerMark>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
) {
    let Ok((player, mut kin, held, mut mark)) = players.single_mut() else {
        return;
    };
    if held.spec.id != MARK_RECALL_ID {
        return;
    }

    // Plain Attack drops / moves the mark. Shield+Attack is the generic "throw
    // the item away", so a marked frame must not be a shielded one.
    if control.attack_pressed && !control.shield_held {
        let pos = kin.pos;
        match mark.as_deref_mut() {
            Some(existing) => existing.pos = Some(pos),
            None => {
                commands
                    .entity(player)
                    .insert(PlayerMark { pos: Some(pos) });
            }
        }
        sfx.write(crate::audio::SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_DASH,
            pos,
        });
        vfx.write(crate::presentation::fx::VfxMessage::Explosion {
            pos,
            kind: crate::presentation::fx::ExplosionKind::ClassicBurst,
            scale: 0.4,
        });
        return;
    }

    // Blink recalls to the mark, if one is set.
    if control.blink_pressed {
        if let Some(target) = mark.and_then(|m| m.pos) {
            kin.pos = target;
            // Recall-strike: a player-side shockwave at the mark, so you can mark a
            // spot, lure enemies onto it, and recall in to hit them (mirrors Blink).
            hits.write(crate::features::HitEvent {
                volume: ae::Aabb::new(target, ae::Vec2::splat(RECALL_SHOCKWAVE_HALF)),
                damage: RECALL_SHOCKWAVE_DAMAGE,
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
            vfx.write(crate::presentation::fx::VfxMessage::Explosion {
                pos: target,
                kind: crate::presentation::fx::ExplosionKind::ClassicBurst,
                scale: 0.6,
            });
        }
    }
}

/// Spawn one Mark/Recall ground item near the player on the first frame a player
/// exists (debug convenience until authored placement lands), mirroring the
/// puppy-slug gun's debug drop.
pub fn spawn_debug_mark_recall_once(
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
    let Some(spec) = crate::brain::held_item_by_id(MARK_RECALL_ID) else {
        return;
    };
    *done = true;
    commands.spawn((
        crate::item_pickup::GroundItem {
            spec,
            pos: kin.pos + ae::Vec2::new(-120.0, 0.0),
            vel: ae::Vec2::ZERO,
            half_extent: ae::Vec2::splat(18.0),
        },
        Name::new("Ground item: mark-recall beacon"),
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
        app.add_message::<crate::features::HitEvent>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, mark_recall_system);
        app
    }

    fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
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
                    facing: 1.0,
                },
                ActionSet::default(),
                HeldItem::new(spec),
            ))
            .id()
    }

    fn press(app: &mut App, attack: bool, blink: bool) {
        let mut cf = app.world_mut().resource_mut::<ControlFrame>();
        cf.attack_pressed = attack;
        cf.blink_pressed = blink;
        cf.shield_held = false;
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
    fn recall_emits_a_player_side_shockwave_at_the_mark() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(bevy::prelude::Update, capture_hits.after(mark_recall_system));
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(200.0, 80.0));
        press(&mut app, true, false); // mark at (200,80) — no hit yet
        app.update();
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(900.0, 50.0);
        press(&mut app, false, true); // recall -> shockwave at the mark
        app.update();
        let hits = &app.world().resource::<CapturedHits>().0;
        assert_eq!(hits.len(), 1, "one shockwave on recall");
        let cx = (hits[0].volume.min.x + hits[0].volume.max.x) * 0.5;
        assert!((cx - 200.0).abs() < 1.0, "shockwave centered on the mark");
        assert!(
            matches!(hits[0].source, crate::features::HitSource::PlayerSlash { .. }),
            "player-side so it spares the player",
        );
    }

    #[test]
    fn attack_sets_a_mark_then_blink_recalls_to_it() {
        let mut app = test_app();
        let player = spawn_player_holding(&mut app, MARK_RECALL_ID, ae::Vec2::new(100.0, 100.0));
        // Drop a mark where we stand.
        press(&mut app, true, false);
        app.update();
        assert_eq!(
            app.world().get::<PlayerMark>(player).and_then(|m| m.pos),
            Some(ae::Vec2::new(100.0, 100.0)),
            "Attack stored a mark at the player's position",
        );
        // Wander far away, then recall.
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(900.0, 50.0);
        press(&mut app, false, true);
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
        press(&mut app, true, false);
        app.update(); // mark at (10,10)
        app.world_mut()
            .get_mut::<PlayerKinematics>(player)
            .unwrap()
            .pos = ae::Vec2::new(400.0, 20.0);
        press(&mut app, true, false);
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
        press(&mut app, false, true);
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
        press(&mut app, true, false);
        app.update();
        assert!(
            app.world().get::<PlayerMark>(player).is_none(),
            "Attack while holding a different item sets no mark",
        );
    }
}
