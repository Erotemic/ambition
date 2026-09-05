//! Durable description for runtime-minted item instances.
//!
//! Authored placements can be rebuilt from room content; dynamically minted items
//! cannot. [`MintedItemDescription`] stores the occurrence's dynamic [`SpawnOrigin`]
//! and authored item-spec id so checkpoint/save restoration can mint the same kind of
//! instance again. Custody or whereabouts decide whether/where it should exist; this
//! description only answers how to reconstruct it.

use std::collections::BTreeMap;

use bevy::prelude::{
    App, IntoScheduleConfigs, MessageReader, Plugin, Query, Res, ResMut, Resource, With,
};

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{AmbitionGameSaveData, PersistedMintedItem};
use ambition_platformer2d_core::snapshot::RollbackRegistrar;

use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::{
    CheckpointCapture, CheckpointCommitted, CheckpointRestore, RoomScopedEntity,
};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use super::{GroundItem, ItemCustody};

/// Everything a restore needs to make one runtime-minted instance again, and
/// deliberately nothing more. See the module docs for why each of the two fields
/// is load-bearing and why there is no third.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MintedItemDescription {
    /// Where it came from — carried forward verbatim so the rebuilt occurrence
    /// states the same spawner the original did.
    pub origin: SpawnOrigin,
    /// The authored id of the item's spec: a reference into the item catalog,
    /// resolved by [`held_spec_by_id`](super::held_spec_by_id) at restore time.
    pub held_item: String,
}

/// How to rebuild each runtime-minted instance remembered at the last
/// committed checkpoint, keyed by the occurrence's own identity.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct MintedItemBaseline {
    /// occurrence → how to make it again.
    minted: BTreeMap<SimId, MintedItemDescription>,
}

impl MintedItemBaseline {
    /// How to rebuild `occurrence`, if the checkpoint saw it as a runtime mint.
    ///
    /// `None` is the ordinary answer for an authored occurrence, whose record is
    /// the thing that rebuilds it.
    pub fn description_of(&self, occurrence: &SimId) -> Option<&MintedItemDescription> {
        self.minted.get(occurrence)
    }

    pub fn is_empty(&self) -> bool {
        self.minted.is_empty()
    }

    pub fn len(&self) -> usize {
        self.minted.len()
    }

    /// Every description, in identity order — for the writer that puts this
    /// value on disk.
    pub fn rows(&self) -> impl Iterator<Item = (&SimId, &MintedItemDescription)> {
        self.minted.iter()
    }

    /// Adopt a set of descriptions — the one road that writes this outside a
    /// [`CheckpointCommitted`].
    ///
    /// its single caller is a durable LOAD. A fresh process has no
    /// checkpoint history, so what the save file described IS what a first death
    /// can rebuild; without this the shipped restore would find a custody row it
    /// has no recipe for and warn instead of putting the object back.
    ///
    /// whole-value, and still not a registry. Adopting a file's rows is
    /// the same snapshot semantics the capture has — the map is replaced, never
    /// accumulated.
    pub fn adopt(&mut self, minted: BTreeMap<SimId, MintedItemDescription>) {
        if self.minted != minted {
            self.minted = minted;
        }
    }

    /// The desync checksum — identities, a canonical provenance rendering,
    /// and an authored id. No `Entity` appears in any of them, so this is
    /// comparable between peers without a mapping pass.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.minted.len() as u64);
        // `BTreeMap`, so this walk is ordered by identity on every peer.
        for (occurrence, description) in &self.minted {
            put_str(&mut bytes, occurrence.as_str());
            // The compatibility rendering, not `Debug`: `canonical_summary` is
            // the spelling the plan dumps and snapshot blobs already use.
            put_str(&mut bytes, &description.origin.canonical_summary());
            put_str(&mut bytes, &description.held_item);
        }
        checksum_bytes(&bytes)
    }
}

/// Record how to remake what the simulation minted, at the instant a
/// checkpoint commits.
///
/// the population is `SpawnOrigin::Dynamic` AND in custody. Dynamic,
/// because an authored occurrence is rebuilt by its record and a second
/// description of it here would be a competing authority. In custody, because
/// that is the only question this value serves — a minted object lying in a
/// loaded room is answered by the object itself, and one in an unloaded room is
/// beyond what a checkpoint remembers at all.
///
/// it reads [`ItemCustody`], the item domain's own authority, rather than
/// the [`InCustodyOf`](ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf)
/// projection the custody capture reads. Each domain captures from what it
/// owns. The projection drops a row for a room-fixture hand, so this map can
/// carry a description the custody baseline has no row for; a surplus
/// description is never consulted, whereas a missing one would lose an object.
pub fn capture_minted_item_baseline(
    mut commits: MessageReader<CheckpointCommitted>,
    carried: Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>,
    baseline: Option<ResMut<MintedItemBaseline>>,
) {
    // Drained unconditionally, like every other reader of this channel: a commit
    // seen during a load must not be re-read against a world that has moved on.
    let committed = commits.read().count() > 0;
    let Some(mut baseline) = baseline else {
        return;
    };
    if !committed {
        return;
    }
    let minted = live_minted_descriptions(&carried);
    if baseline.minted != minted {
        baseline.minted = minted;
    }
}

/// WHAT THE PLAYER WAS ENTITLED TO AT THE LAST COMMITTED CHECKPOINT.
///
/// rollback state with a real VALUE, exactly like its three siblings.
/// Nothing republishes it, and a commit happens mid-frame at a shrine, so a
/// rewind across the commit must restore it or the world keeps an entitlement
/// from a future that was un-happened.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct OwnedItemsBaseline(ambition_items::OwnedItems);

impl OwnedItemsBaseline {
    /// `None` is not expressible: a checkpoint always saw SOME bag, and an empty one is a real
    /// answer.
    pub fn remembered(&self) -> &ambition_items::OwnedItems {
        &self.0
    }

    /// Adopt a bag as the baseline — the road a durable LOAD takes, mirroring
    /// `OccurrenceBaseline::adopt`.
    pub fn adopt(&mut self, owned: ambition_items::OwnedItems) {
        self.0 = owned;
    }

    /// Entity-free VALUE projection, like its three siblings: two peers that
    /// disagree about what the player was entitled to at the last checkpoint
    /// have diverged, and a checksum is how they find out.
    ///
    /// the EQUIPPED slot is deliberately outside it. The baseline restores
    /// stored quantities only — the hand is `restore_custody_to_checkpoint`'s —
    /// so hashing a field this resource does not own would make the projection
    /// disagree with what it actually puts back.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        // `to_persisted` rather than a private field walk: it is already THE
        // stored-quantity view — the durable save's own — and it excludes the
        // equipped projection for the same reason this checksum must. Reusing it
        // means the hash and the file can never come to disagree about what a
        // quantity is.
        let rows = self.0.to_persisted();
        let mut bytes = Vec::new();
        put_u64(&mut bytes, rows.len() as u64);
        for row in &rows {
            put_str(&mut bytes, &row.id);
            put_u64(&mut bytes, u64::from(row.count));
        }
        checksum_bytes(&bytes)
    }
}

pub fn capture_owned_items_baseline(
    mut commits: MessageReader<CheckpointCommitted>,
    owned: Option<Res<ambition_items::OwnedItems>>,
    baseline: Option<ResMut<OwnedItemsBaseline>>,
) {
    // Drained unconditionally, like every other reader of this channel.
    let committed = commits.read().count() > 0;
    let (Some(owned), Some(mut baseline)) = (owned, baseline) else {
        return;
    };
    if !committed {
        return;
    }
    if baseline.0 != *owned {
        baseline.0 = owned.clone();
    }
}

/// Put the entitlements back on a reset — so a death that retracts a
/// minted-after-the-checkpoint instance restores the quantity it was minted
/// from, instead of annihilating it.
pub fn restore_owned_items_to_checkpoint(
    mut resets: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint>,
    baseline: Option<Res<OwnedItemsBaseline>>,
    owned: Option<ResMut<ambition_items::OwnedItems>>,
) {
    // Drained unconditionally, like every other reader of this channel.
    let requested = resets.read().count() > 0;
    let (Some(baseline), Some(mut owned)) = (baseline, owned) else {
        return;
    };
    if !requested {
        return;
    }
    // The bag only. The hand is not in it (I1): custody is restored by
    // `restore_custody_to_checkpoint`, which re-equips what the hand held, and
    // the bag no longer carries a field that could fight it.
    *owned = baseline.remembered().clone();
}

/// The item domain's checkpoint contribution: its two private baseline values,
/// their captures, and the item-specific restore of the generic custody
/// relation.
///
/// The host composes this plugin without naming `MintedItemBaseline`,
/// `OwnedItemsBaseline`, or any of these systems. That is the migration's
/// deletion gate: the concrete item census lives with the item domain.
pub struct ItemCheckpointHorizonPlugin;

impl Plugin for ItemCheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Keeping the edge here makes the contribution carry its own scheduling obligation
        // instead of making the host know which item set produces the facts.
        app.configure_sets(
            sim,
            CheckpointCapture.after(super::ItemPickupSet::CoreHeldItems),
        )
        .init_resource::<MintedItemBaseline>()
        .init_resource::<OwnedItemsBaseline>()
        .add_systems(
            sim,
            (capture_minted_item_baseline, capture_owned_items_baseline).in_set(CheckpointCapture),
        )
        .add_systems(
            sim,
            (
                super::restore_custody_to_checkpoint,
                restore_owned_items_to_checkpoint,
            )
                .in_set(CheckpointRestore),
        );
    }
}

/// Adopt every item-domain checkpoint baseline from a loaded file, after
/// `OwnedItems` itself has been restored.
///
/// This is intentionally one function. `OwnedItemsBaseline` once joined capture,
/// restore and rollback but silently missed durable adoption; keeping the item
/// baselines together here makes that omission local to the domain rather than a
/// fifth cross-crate census.
pub fn adopt_checkpoint_baselines_from_save(
    data: &AmbitionGameSaveData,
    owned: &ambition_items::OwnedItems,
    minted_baseline: Option<&mut MintedItemBaseline>,
    owned_baseline: Option<&mut OwnedItemsBaseline>,
) {
    if let Some(baseline) = minted_baseline {
        let minted = data
            .minted_items()
            .iter()
            .map(|row| {
                (
                    SimId::from_snapshot(row.occurrence.clone()),
                    MintedItemDescription {
                        origin: SpawnOrigin::Dynamic {
                            parent: SimId::from_snapshot(row.parent.clone()),
                            sequence: row.sequence,
                        },
                        held_item: row.held_item.clone(),
                    },
                )
            })
            .collect();
        baseline.adopt(minted);
    }
    if let Some(baseline) = owned_baseline {
        baseline.adopt(owned.clone());
    }
}

/// Mirror the current runtime-minted item descriptions into the durable save.
///
/// Occurrence/custody rows are persisted by the lifecycle-facing durable
/// adapter; the item domain owns this field because only it knows what a minted
/// item description means.
pub fn persist_minted_item_horizon_to_save(
    restored: Res<crate::session::durable_horizon::SaveRestored>,
    minted: Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>,
    mut save: ResMut<AmbitionGameSave>,
) {
    if !restored.0 {
        return;
    }
    let minted_items: Vec<PersistedMintedItem> = live_minted_descriptions(&minted)
        .into_iter()
        .filter_map(|(occurrence, description)| {
            let SpawnOrigin::Dynamic { parent, sequence } = &description.origin else {
                return None;
            };
            Some(PersistedMintedItem {
                occurrence: occurrence.as_str().to_string(),
                parent: parent.as_str().to_string(),
                sequence: *sequence,
                held_item: description.held_item.clone(),
            })
        })
        .collect();

    if save.data().minted_items() == minted_items {
        return;
    }
    // ⛔ `data_mut()` ONLY PAST THE GUARD ABOVE. Reaching it derefs the
    // `ResMut`, which marks the resource changed whether or not the value
    // differs -- that is what the early return protects.
    save.data_mut().set_minted_items(minted_items);
}

/// Rollback facet of the item checkpoint contribution.
pub(crate) fn register_checkpoint_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    const OWNER: &str = env!("CARGO_PKG_NAME");

    registrar.rollback_resource_clone_checksum::<MintedItemBaseline>(
        OWNER,
        "resource.minted_item_baseline",
        "entity-free minted-instance-description checksum projection",
        MintedItemBaseline::checksum,
    );
    registrar.rollback_resource_clone_checksum::<OwnedItemsBaseline>(
        OWNER,
        "resource.owned_items_baseline",
        "entity-free stored-quantity checksum projection",
        OwnedItemsBaseline::checksum,
    );
}

/// How to remake every runtime mint that exists RIGHT NOW — in a hand or
/// lying where somebody dropped it.
///
/// The population rule — `SpawnOrigin::Dynamic` — is stated once, so a second describer cannot
/// start describing authored occurrences by accident.
///
/// the two halves are exact complements, which is why the filter read as
/// correct for a year. An in-custody mint is DESCRIBED and unplaced, because
/// the hand supplies where it is. An in-world mint is PLACED and, until now,
/// undescribed. Neither half ever covered the other's case.
///
/// this widens a POPULATION, not a format. `MintedItemDescription` is unchanged, so the
/// baseline's codec, the three rollback baselines and the save version are all untouched by it
/// — the reason recorded cause (*"the description remembers no position"*) mattered is that it
/// implied the opposite.
pub fn live_minted_descriptions(
    carried: &Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>,
) -> BTreeMap<SimId, MintedItemDescription> {
    carried
        .iter()
        .filter_map(|(occurrence, origin, ground, _)| {
            // the DISCRIMINATOR IS THE PROVENANCE COMPONENT, never the shape
            // of the id string. `SimId::as_str`'s doc is explicit that the
            // spelling is a legibility convenience and that nothing may recover
            // a fact from it — provenance is `SpawnOrigin` precisely so a change
            // to the id grammar cannot silently change reconstruction.
            matches!(origin, SpawnOrigin::Dynamic { .. }).then(|| {
                (
                    occurrence.clone(),
                    MintedItemDescription {
                        origin: origin.clone(),
                        held_item: ground.spec.id.clone(),
                    },
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn horizon_world() -> App {
        let mut app = App::new();
        app.add_message::<CheckpointCommitted>()
            .init_resource::<MintedItemBaseline>()
            .add_systems(Update, capture_minted_item_baseline);
        app
    }

    fn ground(spec_id: &str) -> GroundItem {
        GroundItem {
            spec: ambition_characters::brain::HeldItemSpec {
                id: spec_id.into(),
                melee: None,
                ranged: None,
                use_behavior: ambition_characters::brain::HeldUseBehavior::ThrowOnUse,
            },
            pos: Vec2::new(11.0, 22.0),
            vel: Vec2::ZERO,
            half_extent: Vec2::splat(18.0),
        }
    }

    fn carried(app: &mut App, occurrence: SimId, origin: SpawnOrigin, spec_id: &str) -> Entity {
        let holder = app.world_mut().spawn_empty().id();
        app.world_mut()
            .spawn((
                occurrence,
                origin,
                ground(spec_id),
                ItemCustody::Held { holder },
                RoomScopedEntity,
            ))
            .id()
    }

    /// A carried runtime mint is described; a carried AUTHORED occurrence is
    /// not.
    ///
    /// both terms are observed, because the failure that matters is a capture
    /// that describes everything: an authored occurrence with a row here would be
    /// rebuilt from a snapshot's copy of its spec instead of from the record that
    /// owns it, and a content edit would stop taking effect.
    #[test]
    fn the_capture_describes_the_minted_and_ignores_the_authored() {
        let mut app = horizon_world();
        let thrower = SimId::player_slot(0);
        let mint = SimId::spawned(&thrower, 0);
        carried(
            &mut app,
            mint.clone(),
            SpawnOrigin::Dynamic {
                parent: thrower.clone(),
                sequence: 0,
            },
            "javelin",
        );
        carried(
            &mut app,
            SimId::placement("ground_axe"),
            SpawnOrigin::Authored {
                source: "hub".into(),
                instance: "ground_axe".into(),
            },
            "axe",
        );

        app.world_mut().write_message(CheckpointCommitted);
        app.update();

        let baseline = app.world().resource::<MintedItemBaseline>();
        assert_eq!(
            baseline.len(),
            1,
            "only the runtime mint owes a description"
        );
        assert_eq!(
            baseline.description_of(&mint),
            Some(&MintedItemDescription {
                origin: SpawnOrigin::Dynamic {
                    parent: thrower,
                    sequence: 0,
                },
                held_item: "javelin".into(),
            }),
        );
        assert!(baseline
            .description_of(&SimId::placement("ground_axe"))
            .is_none());
    }

    /// A mint that appears AFTER the commit is not in the baseline.
    ///
    /// Such a map grows forever, and it would describe an object the checkpoint never saw.
    #[test]
    fn a_mint_after_the_commit_has_no_row() {
        let mut app = horizon_world();
        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(app.world().resource::<MintedItemBaseline>().is_empty());

        let thrower = SimId::player_slot(0);
        let late = SimId::spawned(&thrower, 0);
        carried(
            &mut app,
            late.clone(),
            SpawnOrigin::Dynamic {
                parent: thrower,
                sequence: 0,
            },
            "javelin",
        );
        app.update();

        assert!(
            app.world()
                .resource::<MintedItemBaseline>()
                .description_of(&late)
                .is_none(),
            "nothing was committed after the mint, so the checkpoint cannot know about it"
        );
    }

    /// A later commit with nothing minted overwrites an earlier one's rows.
    ///
    /// Dropping is now correctly a no-op for this row: the object still exists and still has to be
    /// describable. So the fixture states the case the test is actually about — the occurrence
    /// CEASING TO EXIST — and the claim it was written for is unchanged.
    #[test]
    fn committing_with_nothing_minted_clears_the_earlier_rows() {
        let mut app = horizon_world();
        let thrower = SimId::player_slot(0);
        let mint = SimId::spawned(&thrower, 0);
        let item = carried(
            &mut app,
            mint,
            SpawnOrigin::Dynamic {
                parent: thrower,
                sequence: 0,
            },
            "javelin",
        );
        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(!app.world().resource::<MintedItemBaseline>().is_empty());

        app.world_mut().entity_mut(item).despawn();
        app.world_mut().write_message(CheckpointCommitted);
        app.update();
        assert!(
            app.world().resource::<MintedItemBaseline>().is_empty(),
            "the second checkpoint saw no mint at all and must say so"
        );
    }

    /// A runtime mint lying in the world, dropped rather than carried.
    fn dropped(app: &mut App, occurrence: SimId, origin: SpawnOrigin, spec_id: &str) -> Entity {
        app.world_mut()
            .spawn((
                occurrence,
                origin,
                ground(spec_id),
                ItemCustody::InWorld,
                RoomScopedEntity,
            ))
            .id()
    }

    /// The capture refused anything `InWorld`, and no authored record can describe a thing the
    /// simulation invented, so a minted item put down in a room was lost at the save horizon.
    ///
    /// A unique dropped weapon must persist where it fell.
    #[test]
    fn the_capture_describes_a_dropped_mint_as_well_as_a_carried_one() {
        let mut app = horizon_world();
        let thrower = SimId::player_slot(0);
        let carried_mint = SimId::spawned(&thrower, 0);
        let dropped_mint = SimId::spawned(&thrower, 1);
        carried(
            &mut app,
            carried_mint.clone(),
            SpawnOrigin::Dynamic {
                parent: thrower.clone(),
                sequence: 0,
            },
            "spark_bomb",
        );
        dropped(
            &mut app,
            dropped_mint.clone(),
            SpawnOrigin::Dynamic {
                parent: thrower.clone(),
                sequence: 1,
            },
            "cinder_beacon",
        );
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<CheckpointCommitted>>()
            .write(CheckpointCommitted::default());
        app.update();

        let baseline = app.world().resource::<MintedItemBaseline>();
        assert!(
            baseline.description_of(&carried_mint).is_some(),
            "the carried mint stopped being described, so widening the population \
             traded one case for the other"
        );
        let dropped_row = baseline.description_of(&dropped_mint).expect(
            "a mint lying in a room is described by NOBODY else — no authored \
             record can describe what the simulation invented",
        );
        assert_eq!(dropped_row.held_item, "cinder_beacon");
    }
}
