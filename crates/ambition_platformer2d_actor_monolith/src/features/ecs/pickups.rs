//! Player → pickup collection on the ECS feature path.

use super::*;
use ambition_combat::components::{CenteredAabb, Collected, FeatureName, PickupFeature};
use ambition_combat::events::GameplayBanner;
use ambition_combat::events::SetFlagRequested;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::sim_selection::winner_by;
use ambition_sfx::{SfxMessage, SfxWriter};

/// Bodies eligible for passive touch collection, and the value check that goes
/// with it.
///
/// ⭐ BOTH MOVED TO [`ambition_platformer2d_shared_tangle::markers`] and are
/// re-exported here under the short names the three passes below read. They are
/// composed of nothing but `PlayerEntity` and `ControlClaims`, both of which
/// already live down there, and while they sat in this file they were the only
/// reason a world-item collect pass could not leave this crate.
/// ⛔ ONE DEFINITION, not a copy: the filter decides who a query RETURNS and
/// `body_collects_on_touch` decides whether a returned body actually collects,
/// so a second copy of either is a way for the two halves to disagree.
/// `PlayerEntity` remains sufficient — a player body whose brain is temporarily
/// absent still collects — and action-driven held-item pickup uses a separate
/// control path.
pub use ambition_platformer2d_shared_tangle::markers::{
    body_collects_on_touch, TouchCollectorFilter,
};

/// Attraction targets the nearest eligible touch collector.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct PickupMagnet {
    /// Distance within which this pickup starts drifting toward a collector.
    pub range: f32,
    /// How fast it closes (px/s).
    pub speed: f32,
}

impl PickupMagnet {
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
/// That works right up until something spawns a pickup at RUNTIME — Sanic's scattered rings — at
/// which point the spec is long gone and the pickup's art is unrecoverable: it simulates,
/// magnetizes and credits perfectly while drawing nothing at all.
///
/// Putting the id on the ENTITY makes a pickup self-describing, so the
/// dynamic-visual pass can bind the same spinning sheet the authored pass binds
/// without needing the room spec that no longer applies.
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct PickupArt(pub String);

/// The set [`magnetize_pickups`] runs in — loot is pulled here.
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
    // be pulled toward a body that is not allowed to pick it up. Now spelled
    // ONCE, in `TouchCollectorFilter`, instead of restated per system.
    collectors: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&ambition_platformer2d_shared_tangle::temporary_control::ControlClaims>,
            // ⭐ THE TIE-BREAK. Two collectors equidistant from a pickup is the
            // ordinary couch arrangement, and `min_by` on distance alone answered
            // it with whichever body the query happened to yield first — which is
            // archetype order, not a gameplay rule.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        TouchCollectorFilter,
    >,
    // A pickup is drawn only toward a body in its own live room (OW1 cut 4).
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut pickups: Query<
        (Entity, &mut CenteredAabb, &PickupMagnet),
        (
            With<PickupFeature>,
            Without<Collected>,
            // A tossed pickup mid-flight owns its own motion (its game's toss
            // system) and must not be reeled in until it settles.
            Without<PickupCollectLock>,
        ),
    >,
) {
    let dt = time.sim_dt();
    for (pickup, mut aabb, magnet) in &mut pickups {
        let room = rooms.of(pickup);
        // NEAREST collector, not the first one the query yields: iteration order
        // is not a gameplay fact, and on a couch "whoever the query happened to
        // return" would be a coin flip between two players.
        let Some((to_collector, dist, _)) =
            ambition_platformer2d_shared_tangle::sim_selection::winner_by(
                collectors
                    .iter()
                    .filter(|(collector, _, is_player, control, _)| {
                        body_collects_on_touch(*is_player, *control)
                            && rooms.of(*collector) == room
                    })
                    .map(|(_, body, _, _, id)| {
                        let delta = body.pos - aabb.center;
                        (delta, delta.length(), id)
                    }),
                |(_, dist, _)| *dist,
                |(_, _, id)| *id,
            )
        else {
            return;
        };
        if dist > 1.0 && dist < magnet.range {
            aabb.center += to_collector.normalize() * (magnet.speed * dt).min(dist);
        }
    }
}

/// The set [`collect_ecs_pickups`] runs in — loot is claimed here.
///
/// The closing half of the pickup window. Together with [`PickupMagnetize`] it
/// names the gap a game inserts custom loot motion into: after the magnet has
/// pulled, before collection claims. Sanic's scattered-ring burst owns each
/// ring's position during exactly that gap, so the magnet cannot reclaim it and
/// collect sees it out at its arc rather than on top of the knocked-back body.
///
/// ONE member. `apply_player_heal_requests` is chained after and is a
/// CONSUMER of what collection produced, not part of claiming it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupCollect;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    collectors: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&ambition_platformer2d_shared_tangle::temporary_control::ControlClaims>,
        ),
        TouchCollectorFilter,
    >,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &PickupFeature,
            Option<&Collected>,
            Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ),
        // A locked (mid-toss) pickup is not collectible yet, exactly as it is not
        // magnetizable — the two guards MUST agree or a ring the magnet ignores
        // could still be collected on overlap.
        (With<FeatureSimEntity>, Without<PickupCollectLock>),
    >,
    mut heals: MessageWriter<crate::avatar::PlayerHealRequested>,
    mut wallets: Query<&mut ambition_characters::actor::BodyWallet>,
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
    mut set_flag: MessageWriter<SetFlagRequested>,
    (mut owned, items): (Option<ResMut<ambition_items::OwnedItems>>, ambition_items::ItemCatalogRead),
    // The tie-break's authority. Read through a lookup rather than joined onto
    // the collector query so a body without one still competes on distance —
    // it just cannot win a tie, which is what `winner_by` documents.
    sim_ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    // A body collects only a pickup in its own live room (OW1 cut 4).
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    // What a collected pickup gave: a retracted boss defeat takes back a
    // mint's, and a restore keeps a placed pickup's while one of the
    // participants in its room is spared (Q151).
    mut reward_grants: Option<ResMut<crate::items::pickup::RewardGrantsSinceCheckpoint>>,
    participants: Query<(Entity, &ambition_characters::control::DrivingParticipant)>,
) {
    // With a population expressed as a filter plus a value test it would no longer mean "nobody can
    // collect" — `TouchCollectorFilter` matches every autonomous actor — and a system-wide return
    // on a population guess is exactly the shape that has switched whole subsystems off in this
    // repo. The per-pickup `find` below already yields nothing when nobody qualifies.
    for (entity, name, aabb, pickup, collected, origin) in &pickups {
        if collected.is_some() {
            continue;
        }
        // ⛔⛔ THE NEAREST OVERLAPPING COLLECTOR, NOT THE FIRST. This was
        // `collectors.iter().find(..)`, and its own comment said so — "find the
        // first overlapping collector" — which reads like a rule and is not one:
        // "first" is Bevy query order, i.e. archetype order, which a resimulated
        // tick can present differently. With two players standing on one ring
        // that decided who healed, who banked the currency, and who took the
        // flag, and it decided it unrepeatably.
        //
        // ⭐ NEAREST-CENTRE IS A RULE A PLAYER CAN SEE, and `SimId` is what makes
        // the answer the same on both peers when two bodies are equidistant. The
        // heal is still routed to that specific body via
        // `PlayerHealRequested::target`, so single-player behaviour is unchanged:
        // one candidate wins by being the only one.
        let Some((collector_entity, ..)) = winner_by(
            collectors.iter().filter(|(collector, kin, is_player, control)| {
                body_collects_on_touch(*is_player, *control)
                    && rooms.of(*collector) == rooms.of(entity)
                    && aabb.aabb().strict_intersects(kin.aabb())
            }),
            |(_, kin, _, _)| kin.pos.distance_squared(aabb.center),
            |(entity, _, _, _)| sim_ids.get(*entity).ok(),
        ) else {
            continue;
        };
        commands.entity(entity).insert(Collected);
        // Q152: a pickup authored to regrow counts its regrowth down, as a
        // broken breakable counts its respawn (`world_time_schedule`).
        if let ambition_entity_catalog::placements::HazardRespawn::AfterSeconds(seconds) =
            pickup.pickup.respawn
        {
            commands
                .entity(entity)
                .insert(ambition_combat::components::RespawnTimer(seconds));
        }
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        let granted = grant_pickup(
            &pickup.pickup.kind,
            collector_entity,
            &mut heals,
            &mut wallets,
            &mut set_flag,
            owned.as_deref_mut(),
            items.get(),
        );
        if let (Ok(collector), Some(reward_grants)) =
            (sim_ids.get(collector_entity), reward_grants.as_deref_mut())
        {
            let source = match origin {
                Some(ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic { parent, .. }) => {
                    crate::items::pickup::GrantSource::Mint { parent: parent.clone() }
                }
                _ => crate::items::pickup::GrantSource::Authored {
                    owners: super::world_time_schedule::owners_beside(entity, &rooms, &participants),
                },
            };
            reward_grants.record(crate::items::pickup::RewardGrant {
                source,
                collector: collector.clone(),
                granted,
            });
        }
        let pos = aabb.center;
        vfx.for_room(rooms.of(entity)).write(VfxMessage::Burst {
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

/// Give `collector` what this pickup is worth. THE grant authority for a
/// [`PickupKind`](ambition_interaction::PickupKind), for every road that hands
/// one to somebody.
///
/// it exists because a SECOND road had the payload and no way to spend
/// it. `ChestFeature::reward()` — an `Option<PickupKind>` filled by all three
/// chest authors (LDtk's `spawn_static`, the mob encounter's reward chest, and
/// the boss's `DropChest` profile) — had zero callers. Every chest in the
/// game opened, sparked, played its sound and announced *"opened X"*, and the
/// authored reward was parsed, lowered onto the live component, and never
/// granted to anybody.
///
/// Lifting the arm out is what makes "a chest's reward is a pickup" true in the code rather
/// than only in the data.
///
/// grant only — no banner, no spark, no sound. Those belong to the road
/// the reward arrived by, and a chest already has its own.
///
/// Returns what the grant added to the collector's wallet and to the bag,
/// which a retraction takes back (BOSS-REPLAY-RETRACTION). A unique item the
/// bag already holds adds nothing.
pub fn grant_pickup(
    kind: &ambition_interaction::PickupKind,
    collector: bevy::prelude::Entity,
    heals: &mut MessageWriter<crate::avatar::PlayerHealRequested>,
    wallets: &mut Query<&mut ambition_characters::actor::BodyWallet>,
    set_flag: &mut MessageWriter<SetFlagRequested>,
    mut owned: Option<&mut ambition_items::OwnedItems>,
    items: &ambition_items::ItemCatalog,
) -> crate::items::pickup::PickupGranted {
    let mut granted = crate::items::pickup::PickupGranted::default();
    match kind {
        ambition_interaction::PickupKind::Health { amount } => {
            heals.write(crate::avatar::PlayerHealRequested::for_target(
                *amount, collector,
            ));
        }
        ambition_interaction::PickupKind::Currency { amount } => {
            // Credit the collecting player's wallet (HUD money meter).
            if let Ok(mut wallet) = wallets.get_mut(collector) {
                let before = wallet.balance;
                wallet.add(*amount);
                granted.coins = wallet.balance - before;
            }
        }
        ambition_interaction::PickupKind::Ability { ability_id } => {
            // Grant the ability into the player's catalog so it shows up in
            // the OoT inventory and can be equipped (wired abilities) — the
            // Metroidvania "learn a power from a boss" beat.
            if let Some(owned) = owned.as_deref_mut() {
                if let Some(item) = items.item_by_dialog_id(ability_id) {
                    let before = owned.count(item);
                    owned.grant(items, item, 1);
                    granted.item = Some((item, owned.count(item) - before)).filter(|(_, n)| *n > 0);
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
        // A PAYLOAD THIS CANNOT SPEND IS A LOUD FAILURE, NOT A NO-OP. `PickupKind::Custom` is
        // an opaque authored string with no reader anywhere in the engine, so reaching here means
        // somebody authored a reward that is granted to nobody — and a silent `_ => {}` is how the
        // eight shipped boss chests came to be authored `Custom("pirate_hoard")`,
        // `Custom("gnu_scroll")` and six more relics whose ids appear in `boss_profiles.ron` and in
        // NO catalog, item table or flag.
        ambition_interaction::PickupKind::Custom(id) => {
            bevy::log::warn!(
                target: "ambition_platformer2d::pickups",
                "pickup payload `Custom({id})` reached the grant and the engine has no \
                 vocabulary for it, so {id} was awarded to nobody: author it as a \
                 health/currency/ability/flag reward, or teach the catalog what it is",
            );
        }
    }
    granted
}

#[cfg(test)]
mod tests;

/// Remember the collected pickups authored `Never` as gone for good (Q154):
/// a `Consumed` row in the occurrence ledger, so a room built again does not
/// build them, and the save keeps them gone.
///
/// Republished from live state every tick, as the ledger's other rows are.
/// A checkpoint restore replaces the ledger with the pinned one, and the
/// pickups still collected in a live room another participant holds are
/// written again on the next tick (Q151). The room the restore rebuilds
/// builds them uncollected, so nothing writes them again. A row of a room
/// that is not live has no pickup to write it again, so each new row is also
/// kept in [`ConsumedSinceCheckpoint`] with its owners.
///
/// Only an authored occurrence has a row: a dropped pickup has no record a
/// room could build again.
#[allow(clippy::type_complexity)]
pub fn record_consumed_pickups(
    pickups: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            &PickupFeature,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
        ),
        With<Collected>,
    >,
    // `Option`: a composition with no rooms still writes the ledger; it only
    // cannot name whose horizons own a row.
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    participants: Query<(Entity, &ambition_characters::control::DrivingParticipant)>,
    occurrences: Option<ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    since: Option<ResMut<ConsumedSinceCheckpoint>>,
) {
    let Some(mut occurrences) = occurrences else {
        return;
    };
    let consumed: Vec<_> = pickups
        .iter()
        .filter(|(_, sim_id, pickup, origin)| {
            pickup.pickup.respawn == ambition_entity_catalog::placements::HazardRespawn::Never
                && matches!(
                    origin,
                    ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { .. }
                )
                && occurrences.whereabouts(sim_id).is_none()
        })
        .map(|(entity, sim_id, ..)| (entity, sim_id.clone()))
        .collect();
    // Written only on a new row, so an unchanged ledger is not marked changed.
    if consumed.is_empty() {
        return;
    }
    if let (Some(rooms), Some(mut since)) = (rooms, since) {
        for (entity, sim_id) in &consumed {
            let Some(definition) = rooms.definition_of(*entity) else {
                continue;
            };
            since.record(
                sim_id.clone(),
                rooms.rooms().spec(definition).id.clone(),
                super::world_time_schedule::owners_beside(*entity, rooms.live(), &participants),
            );
        }
    }
    occurrences.consume(consumed.into_iter().map(|(_, sim_id)| sim_id));
}

/// The one-time pickups consumed since the last committed checkpoint: the room
/// each was in, and the participants whose bodies were there (Q151).
///
/// The ledger's `Consumed` row is the fact; this says whose horizons own it.
/// A checkpoint restore puts the pinned ledger back, which has no row for a
/// pickup consumed after the checkpoint, and a room that is not live has no
/// pickup to write the row again. So the restore's acceptance pins the rows
/// that a spared participant owns into the ledger it restores
/// (`resume_at_checkpoint_on_reset`), and its admission takes the dying
/// participant out of each record's owners
/// ([`disown_consumed_pickups_on_restore`]). A record with no owner left goes
/// back, so its room builds the pickup again.
///
/// Rollback state with a real value: a collection writes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsumedSinceCheckpoint {
    records: std::collections::BTreeMap<ambition_platformer2d_shared_tangle::sim_id::SimId, ConsumedRecord>,
}

/// One pickup consumed since the checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ConsumedRecord {
    /// The id of the room definition it was in.
    room: String,
    /// The participants whose bodies were in its live room, in seat order.
    owners: Vec<ambition_characters::control::PlayerSlot>,
}

impl ConsumedSinceCheckpoint {
    /// Remember that `occurrence` was consumed in room `room`, owned by `owners`.
    pub fn record(
        &mut self,
        occurrence: ambition_platformer2d_shared_tangle::sim_id::SimId,
        room: String,
        owners: Vec<ambition_characters::control::PlayerSlot>,
    ) {
        self.records.insert(occurrence, ConsumedRecord { room, owners });
    }

    /// Whose horizons own the consumption of `occurrence`, if it is recorded.
    pub fn owners(
        &self,
        occurrence: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Option<&[ambition_characters::control::PlayerSlot]> {
        self.records.get(occurrence).map(|record| record.owners.as_slice())
    }

    /// The pickups consumed since the checkpoint that one of `participants`
    /// owns, in id order.
    pub fn owned_by<'a>(
        &'a self,
        participants: &'a [ambition_characters::control::PlayerSlot],
    ) -> impl Iterator<Item = ambition_platformer2d_shared_tangle::sim_id::SimId> + 'a {
        self.records
            .iter()
            .filter(|(_, record)| record.owners.iter().any(|owner| participants.contains(owner)))
            .map(|(occurrence, _)| occurrence.clone())
    }

    /// Keep only the `kept` participants' horizons; forget a record with no
    /// owner left.
    pub fn keep_only_owners(&mut self, kept: &[ambition_characters::control::PlayerSlot]) {
        for record in self.records.values_mut() {
            record.owners.retain(|owner| kept.contains(owner));
        }
        self.records.retain(|_, record| !record.owners.is_empty());
    }

    /// Forget every record.
    pub fn forget_all(&mut self) {
        if !self.records.is_empty() {
            self.records.clear();
        }
    }

    /// Entity-free value projection.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.records.len() as u64);
        for (occurrence, record) in &self.records {
            put_str(&mut bytes, occurrence.as_str());
            put_str(&mut bytes, &record.room);
            put_u64(&mut bytes, record.owners.len() as u64);
            for owner in &record.owners {
                put_u64(&mut bytes, u64::from(owner.0));
            }
        }
        checksum_bytes(&bytes)
    }
}

/// A committed checkpoint makes every consumption since the last one part of
/// the baseline.
pub fn forget_consumed_pickups_at_checkpoint(
    mut commits: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>,
    mut since: ResMut<ConsumedSinceCheckpoint>,
) {
    // Drained unconditionally, like every other reader of this channel.
    if commits.read().count() > 0 {
        since.forget_all();
    }
}

/// An admitted checkpoint restore keeps only the horizons of the participants
/// it spares (Q151). At the admission, because it names who is spared.
pub fn disown_consumed_pickups_on_restore(
    mut replays: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut since: ResMut<ConsumedSinceCheckpoint>,
) {
    for replay in replays.read() {
        if replay.to_checkpoint {
            since.keep_only_owners(&replay.spared_participants);
        }
    }
}

