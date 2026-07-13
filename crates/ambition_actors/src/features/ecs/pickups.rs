//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::SetFlagRequested;
use ambition_sfx::{SfxMessage, SfxWriter};

/// Range within which dropped loot drifts toward the player — a coin/loot magnet,
/// so the coins + hearts this run drops come to you instead of needing a pixel-
/// perfect walk-over.
const PICKUP_MAGNET_RANGE: f32 = 130.0;
/// How fast magnetized pickups close on the player (px/s).
const PICKUP_MAGNET_SPEED: f32 = 340.0;

/// Pull nearby uncollected pickups toward the player. Runs before
/// [`collect_ecs_pickups`], which still does the actual overlap grant — a pickup
/// pulled into overlap is collected the same frame.
pub fn magnetize_pickups(
    time: Res<ambition_time::WorldTime>,
    players: Query<
        &ambition_engine_core::BodyKinematics,
        With<ambition_platformer_primitives::markers::PrimaryPlayer>,
    >,
    mut pickups: Query<&mut CenteredAabb, (With<PickupFeature>, Without<Collected>)>,
) {
    let dt = time.scaled_dt;
    let Ok(player) = players.single() else {
        return;
    };
    for mut aabb in &mut pickups {
        let to_player = player.pos - aabb.center;
        let dist = to_player.length();
        if dist > 1.0 && dist < PICKUP_MAGNET_RANGE {
            aabb.center += to_player.normalize() * (PICKUP_MAGNET_SPEED * dt).min(dist);
        }
    }
}

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<
        (Entity, &ambition_engine_core::BodyKinematics),
        With<ambition_platformer_primitives::markers::PlayerEntity>,
    >,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        With<FeatureSimEntity>,
    >,
    mut heals: MessageWriter<crate::avatar::PlayerHealRequested>,
    mut wallets: Query<&mut ambition_characters::actor::BodyWallet>,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    if player.is_empty() {
        return;
    }
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() {
            continue;
        }
        // Find the first overlapping player. The heal is then routed
        // to that specific player via `PlayerHealRequested::target` so
        // a non-primary collector still actually heals themselves
        // (OVERNIGHT-TODO #17.6 bridge). Single-player behavior is
        // unchanged: the iterator has one entity, and the target ==
        // primary fallback path lands the heal on the same player.
        let Some((collector_entity, _)) = player
            .iter()
            .find(|(_, kin)| aabb.aabb().strict_intersects(kin.aabb()))
        else {
            continue;
        };
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        match &pickup.pickup.kind {
            ambition_interaction::PickupKind::Health { amount } => {
                heals.write(crate::avatar::PlayerHealRequested::for_target(
                    *amount,
                    collector_entity,
                ));
            }
            ambition_interaction::PickupKind::Currency { amount } => {
                // Credit the collecting player's wallet (HUD money meter).
                if let Ok(mut wallet) = wallets.get_mut(collector_entity) {
                    wallet.add(*amount);
                }
            }
            ambition_interaction::PickupKind::Ability { ability_id } => {
                // Grant the ability into the player's catalog so it shows up in
                // the OoT inventory and can be equipped (wired abilities) — the
                // Metroidvania "learn a power from a boss" beat.
                if let Some(owned) = owned.as_deref_mut() {
                    if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                        owned.grant(item, 1);
                    }
                }
            }
            ambition_interaction::PickupKind::StoryFlag { flag } => {
                // PickupSpawn entities with `kind: "flag:<id>"` set
                // the named flag in the save layer and emit a
                // QuestAdvanceEvent::FlagSet via apply_flag_effects.
                // Mirrors the LockWall/Switch flag-setting pattern so
                // intro-v1 cartography pickups and similar narrative
                // story-flag drops just work without per-pickup wiring.
                set_flag.write(SetFlagRequested {
                    id: flag.clone(),
                    on: true,
                });
            }
            _ => {}
        }
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        let id = match &pickup.pickup.kind {
            ambition_interaction::PickupKind::Health { .. } => {
                ambition_sfx::ids::WORLD_HEALTH_COLLECT
            }
            ambition_interaction::PickupKind::Currency { .. } => {
                ambition_sfx::ids::WORLD_COIN_PICKUP
            }
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}

#[cfg(test)]
mod tests;
