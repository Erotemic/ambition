//! Mark / Recall — a held item that drops a teleport mark and recalls to it.
//!
//! - Equip it (walk over the ground item, or equip the catalog slot), and while
//!   it's held a plain `Attack` drops / moves the mark at the player's feet.
//! - The `Blink` button recalls the player to the mark (instant teleport).
//! - `Shield + Attack` still throws the item away through the generic
//!   [`crate::items::pickup::throw_held_item_system`] path.
//!
//! The held spec has no melee/ranged verb, so the throw system would normally
//! treat it as a "pure throwable" and throw it on a plain `Attack`. Like the
//! puppy-slug gun, it opts out via that system's `use_on_attack` id check, which
//! leaves `Attack` free to set the mark.
//!
//! One mark per player, stored as a [`PlayerMark`] component (not a resource)
//! so each player keeps an independent mark once the multiplayer split lands.
//! A persistent [`MarkBeaconVisual`] glowing-crystal beacon stands at the mark
//! ([`sync_mark_beacon_visual`]) so the player can see where Blink will recall
//! them to; set/recall also emit a VFX burst + SFX cue.

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
    // ⭐ EVERY DRIVEN BODY, not the one the primary seat happens to hold.
    // `ControlledSubject` is singular by construction, so a possessed body or a
    // second seat holding the same item simply never acted.
    driven: crate::items::pickup::DrivenBodies,
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
    // Optional: the diagnostic-only Class-B ledger (§3.2). A minimal test app
    // that never added the engine's schedule plugin still recalls.
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

        // Plain Attack drops / moves the mark. Shield+Attack is the generic "throw
        // the item away", so a marked frame must not be a shielded one.
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
                // THE discrete-transit authority: momentum kept, departure contacts
                // and any attachment reconciled (ADR 0024 authority model).
                ae::movement::transit_body(
                    &mut motion_model,
                    &mut clusters,
                    target,
                    ae::movement::TransitVelocity::Keep,
                );
                // Class-B transit authority (`docs/concepts/movement-collision.md`): the
                // recall JUMPS the body, so it is a scripted teleport.
                if let Some(log) = class_b.as_mut() {
                    log.record(player, ClassBRemap::ScriptedTeleport);
                }
                // Recall-strike: a player-side shockwave at the mark, so you can mark a
                // spot, lure enemies onto it, and recall in to hit them (mirrors Blink).
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
