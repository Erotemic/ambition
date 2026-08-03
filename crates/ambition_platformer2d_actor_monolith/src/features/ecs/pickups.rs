//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::SetFlagRequested;
use ambition_sfx::{SfxMessage, SfxWriter};

/// **A pickup that comes to you.** Absent by default — a pickup with no
/// `PickupMagnet` sits where it landed and is collected by touching it.
///
/// ⛔ **this used to be an engine DEFAULT and two hardcoded constants**, applied
/// to every pickup in every game. Jon, 2026-08-03: *"Maryo coins and sanic rings
/// should not be magnetic to the player."* That is not a tuning request — the
/// engine had decided for all content, and the only way for a game to say "my
/// coins stay put" was to not use pickups.
///
/// ⚠ **and the old rule named the PLAYER** (`With<PrimaryPlayer>`), which is the
/// first core value inverted: on a couch, every coin in the room flew at seat
/// one. The attraction is toward the NEAREST collector now — the same population
/// `collect_ecs_pickups` already claims with — so four players work without this
/// module knowing what a protagonist is.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct PickupMagnet {
    /// Distance within which this pickup starts drifting toward a collector.
    pub range: f32,
    /// How fast it closes (px/s).
    pub speed: f32,
}

impl PickupMagnet {
    /// The behaviour every pickup used to get: 130px reach, 340px/s.
    ///
    /// Named rather than `Default` on purpose: a game asking for the classic
    /// loot magnet should have to say so, and "default" reads like "what a
    /// pickup is", which is exactly the assumption this component removes.
    pub fn classic() -> Self {
        Self {
            range: 130.0,
            speed: 340.0,
        }
    }
}

/// A pickup that is temporarily NEITHER magnetizable NOR collectible — a piece
/// of loot mid-toss that has not settled yet (Sanic's scattered rings burst from
/// the body and must not be reeled straight back or credited the instant they
/// spawn on top of the player). Both [`magnetize_pickups`] and
/// [`collect_ecs_pickups`] skip a pickup carrying this, so a game can throw loot
/// outward and make it collectible only once it removes the lock. It is
/// authoritative sim state (it changes whether a pickup is collected this frame),
/// so it is rollback-registered beside the other pickup components.
#[derive(bevy::prelude::Component, Debug, Clone, Copy, Default)]
pub struct PickupCollectLock;

/// The animated sheet a pickup is DRAWN with, carried on the sim entity.
///
/// The authored `PickupSpec.sprite` id used to be read only at room-load, off
/// the room spec, by the pass that spawns static visuals. That works right up
/// until something spawns a pickup at RUNTIME — Sanic's scattered rings — at
/// which point the spec is long gone and the pickup's art is unrecoverable: it
/// simulates, magnetizes and credits perfectly while drawing nothing at all.
///
/// Putting the id on the ENTITY makes a pickup self-describing, so the
/// dynamic-visual pass can bind the same spinning sheet the authored pass binds
/// without needing the room spec that no longer applies.
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct PickupArt(pub String);

/// **The set [`magnetize_pickups`] runs in — loot is pulled here.**
///
/// The opening half of the pickup window; see [`PickupCollect`] for what the
/// pair is for. ONE member: this is the whole of the attract step.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupMagnetize;

/// Pull nearby uncollected pickups toward the player. Runs before
/// [`collect_ecs_pickups`], which still does the actual overlap grant — a pickup
/// pulled into overlap is collected the same frame.
pub fn magnetize_pickups(
    time: Res<ambition_time::WorldTime>,
    // The SAME population `collect_ecs_pickups` claims with, so a pickup cannot
    // be pulled toward a body that is not allowed to pick it up.
    collectors: Query<
        &ambition_platformer2d_core::BodyKinematics,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut pickups: Query<
        (&mut CenteredAabb, &PickupMagnet),
        (
            With<PickupFeature>,
            Without<Collected>,
            // A tossed pickup mid-flight owns its own motion (its game's toss
            // system) and must not be reeled in until it settles.
            Without<PickupCollectLock>,
        ),
    >,
) {
    let dt = time.scaled_dt;
    for (mut aabb, magnet) in &mut pickups {
        // NEAREST collector, not the first one the query yields: iteration order
        // is not a gameplay fact, and on a couch "whoever the query happened to
        // return" would be a coin flip between two players.
        let Some((to_collector, dist)) = collectors
            .iter()
            .map(|body| {
                let delta = body.pos - aabb.center;
                (delta, delta.length())
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            return;
        };
        if dist > 1.0 && dist < magnet.range {
            aabb.center += to_collector.normalize() * (magnet.speed * dt).min(dist);
        }
    }
}

/// **The set [`collect_ecs_pickups`] runs in — loot is claimed here.**
///
/// The closing half of the pickup window. Together with [`PickupMagnetize`] it
/// names the gap a game inserts custom loot motion into: after the magnet has
/// pulled, before collection claims. Sanic's scattered-ring burst owns each
/// ring's position during exactly that gap, so the magnet cannot reclaim it and
/// collect sees it out at its arc rather than on top of the knocked-back body.
///
/// ⚠ ONE member. `apply_player_heal_requests` is chained after and is a
/// CONSUMER of what collection produced, not part of claiming it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupCollect;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<
        (Entity, &ambition_platformer2d_core::BodyKinematics),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        // A locked (mid-toss) pickup is not collectible yet, exactly as it is not
        // magnetizable — the two guards MUST agree or a ring the magnet ignores
        // could still be collected on overlap.
        (With<FeatureSimEntity>, Without<PickupCollectLock>),
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
