//! Mark / Recall: a held item that drops a teleport mark and recalls to it.
//!
//! - While it is held, a plain `Attack` drops or moves the mark at the
//!   player's feet.
//! - `Blink` recalls the player to the mark (instant teleport).
//! - `Shield + Attack` throws the item away through
//!   [`ambition_held_items::throw_held_item_system`].
//!
//! The held spec has no melee/ranged verb, so the throw system would treat it
//! as a pure throwable. Like the puppy-slug gun, it opts out through that
//! system's `use_on_attack` id check, which leaves `Attack` free to set the
//! mark.
//!
//! One mark per player, stored as a [`PlayerMark`] component (not a
//! resource) so each player has an independent mark. A [`MarkBeaconVisual`]
//! crystal stands at the mark ([`sync_mark_beacon_visual`]); set and recall
//! also emit a VFX burst and an SFX cue.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

/// The held-item id the Mark/Recall ability grants (see `brain::action_set`
/// `HELD_ITEMS` and `items::Item::held_item_id`).
pub const MARK_RECALL_ID: &str = "mark_recall";

/// Half-extent of the recall-strike shockwave at the mark.
const RECALL_SHOCKWAVE_HALF: f32 = 36.0;
/// Recall-strike damage: modest, like Blink's arrival shockwave.
const RECALL_SHOCKWAVE_DAMAGE: i32 = 2;

/// The teleport mark a player dropped with the Mark/Recall item, if any. A
/// component, not a resource, so each player's mark is independent.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerMark {
    /// World position of the dropped mark, or `None` until one is set.
    pub pos: Option<ae::Vec2>,
}

/// While holding the Mark/Recall item: a plain `Attack` drops or moves the
/// mark at the player's feet, and `Blink` recalls to the mark if set. A frame
/// that drops a mark does not also recall, so a simultaneous press means "set
/// the mark here".
pub fn mark_recall_system(
    mut commands: Commands,
    // Every driven body, not only the primary seat's `ControlledSubject`, so
    // a possessed body or a second seat can use it.
    driven: ambition_held_items::DrivenBodies,
    mut players: Query<(
        Entity,
        &ActorControl,
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d_core::movement::MotionModel,
        &HeldItem,
        Option<&mut PlayerMark>,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hits: MessageWriter<ambition_combat::events::HitEvent>,
    // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
    // recalls.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    for subject in driven.entities() {
        let Ok((player, control, mut cluster_item, mut motion_model, held, mut mark)) =
            players.get_mut(subject)
        else {
            continue;
        };
        let mut clusters = cluster_item.as_clusters_mut();
        let c = control.0;
        if held.spec.id != MARK_RECALL_ID {
            continue;
        }

        // Plain Attack drops or moves the mark. Shield+Attack throws the item
        // away, so a shielded frame does not mark.
        if c.melee_pressed && !c.shield_held {
            let pos = clusters.kinematics.pos;
            match mark.as_deref_mut() {
                Some(existing) => existing.pos = Some(pos),
                None => {
                    commands
                        .entity(player)
                        .insert(PlayerMark { pos: Some(pos) });
                }
            }
            sfx.write_for(
                player,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DASH,
                    pos,
                },
            );
            vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                pos,
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.4,
                pose: ambition_vfx::FxPose::UPRIGHT,
            });
            continue;
        }

        // Blink recalls to the mark, if one is set.
        if c.blink_pressed {
            if let Some(target) = mark.and_then(|m| m.pos) {
                // The discrete-transit authority: momentum kept, departure
                // contacts and attachment reconciled (ADR 0024).
                ae::movement::transit_body(
                    &mut motion_model,
                    &mut clusters,
                    target,
                    ae::movement::TransitVelocity::Keep,
                );
                // Class-B transit (`docs/concepts/movement-collision.md`):
                // the recall moves the body, so it is a scripted teleport.
                if let Some(log) = class_b.as_mut() {
                    log.record(player, ClassBRemap::ScriptedTeleport);
                }
                // Recall strike: a player-side shockwave at the mark, so you
                // can lure enemies onto it and recall in to hit them.
                hits.write(ambition_combat::events::HitEvent {
                    strike_sfx: None,
                    volume: ae::CombatVolume::circle(target, RECALL_SHOCKWAVE_HALF),
                    damage: RECALL_SHOCKWAVE_DAMAGE,
                    source: ambition_combat::events::HitSource::Melee,
                    attacker: Some(player),
                    target: ambition_combat::events::HitTarget::Volume,
                    mode: ambition_combat::events::HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                                    attacker_move_instance: None,
                });
                sfx.write_for(
                    player,
                    ambition_sfx::SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_BLINK,
                        pos: target,
                    },
                );
                vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                    pos: target,
                    fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                    scale: 0.6,
                    pose: ambition_vfx::FxPose::UPRIGHT,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
