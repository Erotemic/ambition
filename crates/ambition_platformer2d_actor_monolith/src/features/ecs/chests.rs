//! Player → static-chest open path on the ECS feature side.

// ⭐ NAMED, NOT GLOBBED. This was `use super::*` over the whole
// `features/ecs` module — a channel no `crate::` grep sees. Measured by
// deleting it: bevy's prelude, `ambition_vfx`'s two message types and
// `RoomVisual`, which is `shared_tangle`'s. No monolith vocabulary.
use ambition_combat::components::{
    CenteredAabb, ChestFeature, FallingChest, FeatureId, FeatureName, Opened,
};
use ambition_combat::events::{GameplayBanner, SetFlagRequested};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};
use bevy::prelude::*;

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    // ⭐ EVERY DRIVEN BODY. The gesture half was already per-body —
    // `ActingParticipant` keys the buffered interact off the body's OWN driving
    // slot — and only the subject was singular, so a couch's second seat could
    // stand on a chest and press interact forever.
    driven: ambition_held_items::DrivenBodies,
    // the buffered interact belongs to the SEAT DRIVING THE ACTING BODY.
    // Slot 0 was the wrong source the moment a body other than the home avatar
    // can be driven: a possessed actor's chest open spent the home seat's press.
    mut acting: crate::control::ActingParticipant,
    // The startup-frame fallback subject, and nothing else.
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    // Presentation anim for whichever body opened the chest.
    mut anims: Query<&mut ambition_characters::actor::BodyAnimFacts>,
    bodies: Query<&ambition_platformer2d_core::BodyKinematics>,
    // `&ChestFeature`, not `With<ChestFeature>` — that one word is the whole of "authored
    // chest rewards are never granted". The payload was filled by all three chest authors and
    // read by nobody: this system knew a chest was there and never asked what was IN it.
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &ChestFeature,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        With<FeatureSimEntity>,
    >,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
    // The grant, routed to the body that opened it — the same three writers the
    // walk-over pickup hands to `grant_pickup`.
    mut heals: MessageWriter<crate::avatar::PlayerHealRequested>,
    mut wallets: Query<&mut ambition_characters::actor::BodyWallet>,
    mut owned: Option<ResMut<ambition_items::OwnedItems>>,
) {
    // Iterate every player so each player's own buffered interact
    // can open a chest the player is overlapping. Per-player interact
    // state is independent (each player has their own
    // `PlayerInteractionState`); the chest is shared (a future co-op
    // build can still gate "first-come gets the open" by inserting
    // the `Opened` marker, which keeps subsequent attempts no-ops).
    // OVERNIGHT-TODO #17.6/#17.8 — preserve single-player behavior
    // because the iterator has one entity today.
    // Same hold time as the NPC / switch interact gesture. Kept in
    // sync with `interact_ecs_actors_and_switches::INTERACT_ANIM_HOLD_SECS`
    // so the player's reach-and-open animation feels uniform across
    // every interactable kind.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else: before a seat is
    // attached there is no driven body at all, and the primary avatar is the
    // subject every single-player fixture expects.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.iter().next());
    }
    for subject in subjects {
        if !acting.buffered_interact(subject) {
            continue;
        }
        let Ok(subject_kin) = bodies.get(subject) else {
            continue;
        };
        let reach_aabb = subject_kin.aabb();
        for (entity, id, name, aabb, chest, opened, falling) in &chests {
            if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(reach_aabb) {
                continue;
            }
            commands.entity(entity).insert(Opened);
            acting.consume_interact(subject);
            super::interact::pose_interact(&mut anims, subject, INTERACT_ANIM_HOLD_SECS);
            banner.show(format!("opened {}", name.0.as_str()), 2.6);
            // THE CHEST DOES NOT KNOW HOW TO GRANT ANYTHING, and that is
            // the point: its reward is a `PickupKind`, and there is exactly one
            // authority that turns one of those into health, money, an ability
            // or a story flag. Teaching the chest a second copy would be four
            // payload kinds to keep in agreement forever.
            if let Some(reward) = chest.reward() {
                super::pickups::grant_pickup(
                    reward,
                    subject,
                    &mut heals,
                    &mut wallets,
                    &mut set_flag,
                    owned.as_deref_mut(),
                );
            }
            let pos = aabb.center;
            vfx.write(VfxMessage::Burst {
                pos,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_TREASURE_CHEST_OPEN,
                pos,
            });
            if let Some(encounter_id) = id.as_str().strip_prefix("encounter_chest_") {
                set_flag.write(SetFlagRequested {
                    id: format!("encounter_{encounter_id}_reward_dropped"),
                    on: true,
                });
            }
            break;
        }
    }
}

#[cfg(test)]
mod chest_tests;
