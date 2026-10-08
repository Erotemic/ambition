//! Durable description for runtime-minted item instances.
//!
//! Authored placements can be rebuilt from room content; dynamically minted items
//! cannot. [`MintedItemDescription`] stores the occurrence's dynamic [`SpawnOrigin`]
//! and authored item-spec id so checkpoint/save restoration can mint the same kind of
//! instance again. Custody or whereabouts decide whether/where it should exist; this
//! description only answers how to reconstruct it.

use std::collections::BTreeMap;

use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, MessageReader, Plugin, Query, Res, ResMut, Resource, With,
};

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{AmbitionGameSaveData, PersistedMintedItem};
use ambition_platformer2d_core::snapshot::RollbackRegistrar;

use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::{
    CheckpointCapture, CheckpointCommitted, RoomScopedEntity,
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
    mut baseline: ResMut<MintedItemBaseline>,
) {
    // Drained unconditionally, like every other reader of this channel: a commit
    // seen during a load must not be re-read against a world that has moved on.
    let committed = commits.read().count() > 0;
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
///
/// The purse is in it too: the bag and the primary body's wallet are one
/// decision, as a purchase moves both, so a death that put back only one of them
/// lost the goods or the coins. It is the PRIMARY body's balance, the one a
/// death restores and the one the save holds; another participant's wallet is
/// not rewound by this player's death (Q151).
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct OwnedItemsBaseline {
    bag: ambition_items::OwnedItems,
    purse: i32,
}

impl OwnedItemsBaseline {
    /// `None` is not expressible: a checkpoint always saw SOME bag, and an empty one is a real
    /// answer.
    pub fn remembered(&self) -> &ambition_items::OwnedItems {
        &self.bag
    }

    /// The primary body's wallet balance at the last committed checkpoint.
    pub fn purse(&self) -> i32 {
        self.purse
    }

    /// Adopt a bag as the baseline — the road a durable LOAD takes, mirroring
    /// `OccurrenceBaseline::adopt`.
    pub fn adopt(&mut self, owned: ambition_items::OwnedItems) {
        self.bag = owned;
    }

    /// Adopt a wallet balance as the baseline purse, on the same load road.
    pub fn adopt_purse(&mut self, balance: i32) {
        self.purse = balance;
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
        // The BUILT-IN catalog's ids: a checksum has no world to ask, and the
        // built-in table is the same in every process, so two peers hash the
        // same bag to the same bytes whatever content each installed.
        let rows = self.bag.to_persisted(ambition_items::builtin_item_catalog());
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.purse as u32 as u64);
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
    wallets: Query<
        &ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut baseline: ResMut<OwnedItemsBaseline>,
) {
    // Drained unconditionally, like every other reader of this channel.
    let committed = commits.read().count() > 0;
    let Some(owned) = owned else {
        return;
    };
    if !committed {
        return;
    }
    if baseline.bag != *owned {
        baseline.bag = owned.clone();
    }
    // No primary body means no purse to remember: keep the last one rather
    // than write a zero nobody had.
    if let Ok(wallet) = wallets.single() {
        if baseline.purse != wallet.balance {
            baseline.purse = wallet.balance;
        }
    }
}

/// Put the entitlements back on a reset — so a death that retracts a
/// minted-after-the-checkpoint instance restores the quantity it was minted
/// from, instead of annihilating it.
/// The item domain's pinned inputs for one committed restore.
///
/// ⛔ INSTALLED FOR THE DURATION OF `CheckpointDomainApply` AND REMOVED AFTER,
/// exactly as the lifecycle layer's are. It is a SIBLING of
/// `CheckpointRestoreInputs`, not a member of it: mint recipes and entitlement
/// quantities are this domain's, and putting them in the shared value would make
/// `shared_tangle` the checkpoint coordinator.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ItemCheckpointRestoreInputs {
    /// How to remake the runtime mints the restored custody rows name.
    pub minted: MintedItemBaseline,
    /// The stored quantities. The hand is not in it — custody is
    /// `restore_custody_to_checkpoint`'s.
    pub owned: OwnedItemsBaseline,
    /// The grants since the checkpoint that the restore keeps in `owned`, each
    /// owned only by the participants the restore spares (Q151). They stay
    /// recorded, so that the next restore of the same checkpoint keeps them
    /// again. A record the restore forgets is a reward the next death loses.
    pub grants: RewardGrantsSinceCheckpoint,
    /// The bag spends since the checkpoint that the restore keeps taken from
    /// `owned`, for the same reason. A spend the restore forgets is a
    /// quantity the next death puts back while its object stays.
    pub spends: ambition_held_items::BagSpendsSinceCheckpoint,
}

/// ⛔⛔ IT RUNS ONLY FROM THE COMMIT, AND READS ONLY WHAT THE COMMIT INSTALLED.
/// This once read `ResetToCheckpoint`, so a reset the lifecycle slot refused
/// still rolled the bag back — measured, and the entitlement was one of three
/// values a refused reset spent.
pub fn restore_owned_items_to_checkpoint(
    inputs: Option<Res<ItemCheckpointRestoreInputs>>,
    owned: Option<ResMut<ambition_items::OwnedItems>>,
    mut wallets: Query<
        &mut ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    let (Some(inputs), Some(mut owned)) = (inputs, owned) else {
        return;
    };
    // The bag and the purse the restore was accepted with: the checkpoint's,
    // and what the grants it keeps gave (Q151; pinned by
    // `resume_at_checkpoint_on_reset` with `kept_by_restore`), so the
    // verification of the bag reads the same value.
    reduce_owned_items_to_baseline(inputs.owned.remembered(), &mut owned);
    if let Ok(mut wallet) = wallets.single_mut() {
        let balance = inputs.owned.purse();
        if wallet.balance != balance {
            wallet.balance = balance;
        }
    }
}

/// The entitlement domain's reducer.
///
/// The bag only. The hand is not in it (I1): custody is restored by
/// `restore_custody_to_checkpoint`, which re-equips what the hand held, and the
/// bag no longer carries a field that could fight it.
pub fn reduce_owned_items_to_baseline(
    baseline: &ambition_items::OwnedItems,
    owned: &mut ambition_items::OwnedItems,
) {
    *owned = baseline.clone();
}

/// The item domain starts a new run. (fresh-run reducer)
///
/// The bag itself is put back by [`restore_owned_items_to_checkpoint`] from the
/// pinned starter bag, as a death puts back a checkpoint's bag. This adds what a
/// death does not do: the pinned values become the item baselines, so a later
/// death returns to the starter bag and not to the old run's checkpoint, and the
/// wallet goes back to zero.
///
/// ⚠ **FRESH-RUN, NOT EMPTY.** The wallet is `BodyWallet::default()`: no
/// character definition authors a starting balance (checked 2026-09-13), so
/// zero is that value. If one ever does, this owes the definition and not a
/// constant.
pub fn start_the_item_domain_fresh(
    fresh: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>>,
    inputs: Option<Res<ItemCheckpointRestoreInputs>>,
    mut minted: ResMut<MintedItemBaseline>,
    mut owned: ResMut<OwnedItemsBaseline>,
    mut wallets: Query<
        &mut ambition_characters::actor::BodyWallet,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
) {
    if fresh.is_none() {
        return;
    }
    if let Ok(mut wallet) = wallets.single_mut() {
        *wallet = ambition_characters::actor::BodyWallet::default();
    }
    let Some(inputs) = inputs else {
        return;
    };
    if *minted != inputs.minted {
        *minted = inputs.minted.clone();
    }
    if *owned != inputs.owned {
        *owned = inputs.owned.clone();
    }
}

/// What one granted pickup added: coins to its collector's wallet, and items
/// to the bag. Returned by `grant_pickup`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickupGranted {
    pub coins: i32,
    pub item: Option<(ambition_items::Item, u32)>,
}

impl PickupGranted {
    pub fn is_empty(&self) -> bool {
        self.coins == 0 && self.item.is_none()
    }
}

/// Where a granted reward came from, as a retracted boss defeat names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrantSource {
    /// A collected mint, by the occurrence it fell out of
    /// (`SpawnOrigin::Dynamic { parent }`).
    Mint { parent: SimId },
    /// An opened boss reward chest, by its boss placement.
    BossChest { placement: String },
    /// A placed pickup or an ordinary chest of a room, by the participants
    /// in its live room when it was taken (Q151): their horizons own the
    /// grant. A restore keeps it while one of them is spared, because the
    /// source then stays taken in a room the restore does not build again.
    Authored {
        owners: Vec<ambition_characters::control::PlayerSlot>,
    },
}

/// One grant of a collected pickup or an opened chest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewardGrant {
    pub source: GrantSource,
    /// The body that collected it.
    pub collector: SimId,
    pub granted: PickupGranted,
}

/// The grants of the mints collected and the boss reward chests opened since
/// the last committed checkpoint (BOSS-REPLAY-RETRACTION). A collected mint is
/// gone and an opened chest grants nothing again, so this is the only record of
/// what they gave, and a retracted boss defeat takes back the grants of its
/// mints and its chest.
///
/// Rollback state with a real value: a collection writes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct RewardGrantsSinceCheckpoint {
    grants: Vec<RewardGrant>,
}

impl RewardGrantsSinceCheckpoint {
    /// Record a grant. A grant that added nothing is not recorded.
    pub fn record(&mut self, grant: RewardGrant) {
        if !grant.granted.is_empty() {
            self.grants.push(grant);
        }
    }

    /// Take out the grants of the mints of `bosses` and of the reward chests
    /// of `placements`, in the order they were made.
    pub fn take_for(
        &mut self,
        bosses: &std::collections::BTreeSet<SimId>,
        placements: &std::collections::BTreeSet<String>,
    ) -> Vec<RewardGrant> {
        let retracted = |grant: &RewardGrant| match &grant.source {
            GrantSource::Mint { parent } => bosses.contains(parent),
            GrantSource::BossChest { placement } => placements.contains(placement),
            // A boss defeat does not own what a room authored.
            GrantSource::Authored { .. } => false,
        };
        if !self.grants.iter().any(retracted) {
            return Vec::new();
        }
        let (taken, kept) = std::mem::take(&mut self.grants).into_iter().partition(retracted);
        self.grants = kept;
        taken
    }

    /// The grants a checkpoint restore keeps (Q151): those of a defeat the
    /// restore does not retract (`bosses` and `placements` name the ones it
    /// does), and those of an authored source that a spared participant owns
    /// (`spared_participants`).
    ///
    /// An authored source with no spared owner was taken only by the dying
    /// participant. Its room is built again from the checkpoint, either at
    /// once or when somebody comes back, so the source is there again and its
    /// grant goes back with it.
    ///
    /// ⚠ This keeps a grant also when the restore could put its source back,
    /// so it relies on two measured facts (2026-10-04). A collected bag
    /// pickup mint has no ledger row (the warden's `markrecall` has no
    /// `SimId` and no minted save row), so no restore builds it again. An
    /// opened chest stays opened, because its looted flag is not rewound. If
    /// either becomes false, that grant must go back with its source, or the
    /// reward exists twice.
    pub fn kept_by_restore<'a>(
        &'a self,
        bosses: &'a std::collections::BTreeSet<SimId>,
        placements: &'a std::collections::BTreeSet<String>,
        spared_participants: &'a [ambition_characters::control::PlayerSlot],
    ) -> impl Iterator<Item = &'a RewardGrant> + 'a {
        self.grants.iter().filter(move |grant| match &grant.source {
            GrantSource::Mint { parent } => !bosses.contains(parent),
            GrantSource::BossChest { placement } => !placements.contains(placement),
            GrantSource::Authored { owners } => owners.iter().any(|owner| spared_participants.contains(owner)),
        })
    }

    /// The grants a checkpoint restore keeps ([`Self::kept_by_restore`]), as
    /// the record that stays after it. An authored source stays owned only by
    /// its spared owners: the restore rewound the horizons of the others, so
    /// they own the grant no longer. A boss defeat that a restore keeps
    /// shrinks its participants in the same way.
    pub fn after_restore(
        &self,
        bosses: &std::collections::BTreeSet<SimId>,
        placements: &std::collections::BTreeSet<String>,
        spared_participants: &[ambition_characters::control::PlayerSlot],
    ) -> Self {
        let grants = self
            .kept_by_restore(bosses, placements, spared_participants)
            .map(|grant| {
                let mut grant = grant.clone();
                if let GrantSource::Authored { owners } = &mut grant.source {
                    owners.retain(|owner| spared_participants.contains(owner));
                }
                grant
            })
            .collect();
        Self { grants }
    }

    /// Every grant, in the order they were made.
    pub fn grants(&self) -> &[RewardGrant] {
        &self.grants
    }

    /// Forget every grant: a checkpoint commit makes them part of the
    /// baseline. A restore keeps the ones it keeps
    /// ([`keep_the_bag_records_the_restore_keeps`]).
    pub fn forget_all(&mut self) {
        if !self.grants.is_empty() {
            self.grants.clear();
        }
    }

    /// Entity-free value projection: two peers that disagree about what a
    /// retraction would take back have diverged.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.grants.len() as u64);
        for grant in &self.grants {
            match &grant.source {
                GrantSource::Mint { parent } => {
                    put_u64(&mut bytes, 0);
                    put_str(&mut bytes, parent.as_str());
                }
                GrantSource::BossChest { placement } => {
                    put_u64(&mut bytes, 1);
                    put_str(&mut bytes, placement);
                }
                GrantSource::Authored { owners } => {
                    put_u64(&mut bytes, 2);
                    put_u64(&mut bytes, owners.len() as u64);
                    for owner in owners {
                        put_u64(&mut bytes, u64::from(owner.0));
                    }
                }
            }
            put_str(&mut bytes, grant.collector.as_str());
            put_u64(&mut bytes, u64::from(grant.granted.coins as u32));
            match grant.granted.item {
                Some((item, n)) => {
                    put_u64(&mut bytes, item.index() as u64 + 1);
                    put_u64(&mut bytes, u64::from(n));
                }
                None => put_u64(&mut bytes, 0),
            }
        }
        checksum_bytes(&bytes)
    }
}

/// A committed checkpoint makes every grant and every bag spend since the
/// last one part of the baseline.
pub fn forget_bag_records_at_checkpoint(
    mut commits: MessageReader<CheckpointCommitted>,
    mut grants: ResMut<RewardGrantsSinceCheckpoint>,
    mut spends: ResMut<ambition_held_items::BagSpendsSinceCheckpoint>,
) {
    // Drained unconditionally, like every other reader of this channel.
    if commits.read().count() > 0 {
        grants.forget_all();
        spends.forget_all();
    }
}

/// A checkpoint restore keeps recorded the grants and the bag spends that it
/// keeps (pinned by `resume_at_checkpoint_on_reset`), and forgets the others:
/// the restore put their quantities back. A fresh run keeps none.
/// (checkpoint reducer, in `CheckpointDomainApply`)
pub fn keep_the_bag_records_the_restore_keeps(
    inputs: Option<Res<ItemCheckpointRestoreInputs>>,
    mut grants: ResMut<RewardGrantsSinceCheckpoint>,
    mut spends: ResMut<ambition_held_items::BagSpendsSinceCheckpoint>,
) {
    let Some(inputs) = inputs else {
        return;
    };
    if *grants != inputs.grants {
        *grants = inputs.grants.clone();
    }
    if *spends != inputs.spends {
        *spends = inputs.spends.clone();
    }
}

/// BOSS-REPLAY-RETRACTION (Q51): the mints of a boss defeat that a replay
/// retracted go with it. "If you roll back to before the defeat, you do not
/// have the item."
///
/// A mint names the boss it fell out of (`SpawnOrigin::Dynamic { parent }`).
/// Every live mint of a retracted boss is despawned, and a held one leaves
/// its holder's hand first. The ledger rows of those mints, and of the
/// dormant mints the save describes in a room that is not live, are retracted,
/// so no room build puts one back and the save mirrors drop their rows.
///
/// What a collected mint or the opened reward chest gave is taken back too:
/// its coins leave the wallet of the body that collected them (down to zero,
/// if they were spent), and its item leaves the bag
/// ([`RewardGrantsSinceCheckpoint`]).
#[allow(clippy::too_many_arguments)]
pub fn retract_mints_of_retracted_boss_defeats(
    mut commands: Commands,
    mut retracted: bevy::prelude::MessageReader<ambition_boss_encounter::BossDefeatRetracted>,
    mints: Query<(Entity, &SimId, &SpawnOrigin, Option<&GroundItem>, Option<&ItemCustody>)>,
    mut hands: Query<ambition_combat::hand::RepertoireQuery>,
    save: Option<Res<AmbitionGameSave>>,
    occurrences: Option<ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    mut grants: ResMut<RewardGrantsSinceCheckpoint>,
    mut wallets: Query<(&SimId, &mut ambition_characters::actor::BodyWallet)>,
    owned: Option<ResMut<ambition_items::OwnedItems>>,
) {
    let (mut bosses, mut placements) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
    for retracted in retracted.read() {
        bosses.extend(retracted.boss.clone());
        placements.insert(retracted.placement.clone());
    }
    if placements.is_empty() {
        return;
    }
    let mut owned = owned;
    for grant in grants.take_for(&bosses, &placements) {
        if grant.granted.coins != 0 {
            if let Some((_, mut wallet)) = wallets.iter_mut().find(|(id, _)| **id == grant.collector) {
                wallet.add(-grant.granted.coins);
            }
        }
        if let (Some((item, n)), Some(owned)) = (grant.granted.item, owned.as_deref_mut()) {
            owned.take(item, n);
        }
    }
    let fell_out_of_a_retracted_boss =
        |origin: &SpawnOrigin| matches!(origin, SpawnOrigin::Dynamic { parent, .. } if bosses.contains(parent));
    let mut ids: std::collections::BTreeSet<SimId> = std::collections::BTreeSet::new();
    for (entity, sim_id, origin, ground, custody) in &mints {
        if !fell_out_of_a_retracted_boss(origin) {
            continue;
        }
        if let (Some(ItemCustody::Held { holder }), Some(ground)) = (custody, ground) {
            if let Ok(mut repertoire) = hands.get_mut(*holder) {
                if repertoire.held.is_some_and(|held| held.id() == ground.spec.id.as_str()) {
                    ambition_held_items::unequip_held(&mut commands, *holder, &mut repertoire);
                }
            }
        }
        ambition_platformer2d_shared_tangle::lifecycle::despawn_scoped_entity(&mut commands, entity);
        ids.insert(sim_id.clone());
    }
    if let Some(save) = save.as_deref() {
        ids.extend(
            save.data()
                .minted_items()
                .iter()
                .filter(|row| bosses.iter().any(|boss| boss.as_str() == row.parent))
                .map(|row| SimId::from_snapshot(row.occurrence.clone())),
        );
    }
    if let Some(mut occurrences) = occurrences {
        if ids.iter().any(|sim_id| occurrences.remembers(sim_id)) {
            let _ = occurrences.retract(&ids);
        }
    }
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
        // The mints of a boss defeat a replay retracted, in the replay
        // chain's content slot, after the boss road retracts the defeat. A
        // host can omit the boss plugin, so this plugin also registers the
        // message it reads (registration is idempotent).
        .add_message::<ambition_boss_encounter::BossDefeatRetracted>()
        .add_systems(
            sim,
            retract_mints_of_retracted_boss_defeats
                .in_set(crate::session::reset::ContentRoomReplayResetSet)
                .after(ambition_boss_encounter::BossDefeatRetraction),
        )
        .add_systems(
            ambition_combat::events::RestoreConsequences,
            retract_mints_of_retracted_boss_defeats
                .in_set(ambition_combat::events::RestoreConsequenceSet::ReplayContent)
                .after(ambition_boss_encounter::BossDefeatRetraction),
        )
        .init_resource::<RewardGrantsSinceCheckpoint>()
        .init_resource::<ambition_held_items::BagSpendsSinceCheckpoint>()
        .add_systems(sim, forget_bag_records_at_checkpoint)
        // ⭐ INTO THE COMMIT EXECUTOR'S SCHEDULE, not the simulation. Custody
        // materializes and despawns; doing that on a speculative frame for an
        // unconfirmed request is what the confirmed-frame lifecycle exists to
        // prevent. `.chain()` because the entitlement bag and the hand are one
        // decision: the bag is restored first, then custody re-equips out of it.
        //
        // The custody projection runs again after the custody restore. The
        // restore's verification reads `InCustodyOf`, and the restore writes
        // only `ItemCustody`; an object it builds again into a hand has no
        // `InCustodyOf` until the projection runs. In the simulation the
        // projection runs later in the tick, but the verification runs first.
        .add_systems(
            ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply,
            (
                restore_owned_items_to_checkpoint,
                super::restore_custody_to_checkpoint,
                ambition_held_items::project_custody_onto_residency,
                start_the_item_domain_fresh,
                keep_the_bag_records_the_restore_keeps,
            )
                .chain(),
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
/// The minted descriptions a candidate's construction needs, as a VALUE.
///
/// ⛔⛤ **REVIEW FINDING 2, 2026-09-15: CANDIDATE CONSTRUCTION WAS READING THE
/// LIVE SESSION'S MINTED BASELINE.** `OccurrenceContinuity` needs two descriptors
/// to rebuild a runtime-minted occurrence — the ledger row saying WHERE it is,
/// and the minted description saying WHAT it is. The candidate carried its own
/// ledger and then read A's `MintedItemBaseline` for the second half, so it
/// planned from a MIXED durable horizon: B's whereabouts with A's descriptions.
/// A row of B's that A has never seen cannot be described, so B's first room
/// verifies against a world that is not B's saved world — and the correction
/// arrives after A has been retired, which is post-publication recovery.
pub fn minted_baseline_from_save(data: &AmbitionGameSaveData) -> MintedItemBaseline {
    let mut baseline = MintedItemBaseline::default();
    adopt_checkpoint_baselines_from_save(data, &ambition_items::OwnedItems::default(), 0, Some(&mut baseline), None);
    baseline
}

pub fn adopt_checkpoint_baselines_from_save(
    data: &AmbitionGameSaveData,
    owned: &ambition_items::OwnedItems,
    // The primary body's balance after the load applied the save's.
    purse: i32,
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
        baseline.adopt_purse(purse);
    }
}

/// Mirror the descriptions of every runtime mint that exists into the durable
/// save: the live ones, and the dormant ones (OW3).
///
/// Occurrence/custody rows are persisted by the lifecycle-facing durable
/// adapter; the item domain owns this field because only it knows what a minted
/// item description means.
///
/// The save's field is the one record of a DORMANT mint's description. A mint
/// left lying in a room that is not live has no entity to describe it, and no
/// room authors a record to rebuild it from. Its row stays while the occurrence
/// ledger places it in a room (see [`with_dormant_mints`]). The room build
/// reads this field when that room becomes live again, and a load adopts it.
pub fn persist_minted_item_horizon_to_save(
    restored: Res<crate::session::durable_horizon::SaveRestored>,
    minted: Query<(&SimId, &SpawnOrigin, &GroundItem, &ItemCustody), With<RoomScopedEntity>>,
    ledger: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    mut save: ResMut<AmbitionGameSave>,
    // The inputs the rows were last mirrored with: a cache of live inputs,
    // not state.
    mut mirrored_with: bevy::prelude::Local<Option<MintedMirrorKey>>,
) {
    if !restored.0 {
        return;
    }
    // ⭐ THE SAVE'S MINTED ROWS ARE MIRRORED ONLY WHEN SOMETHING THEY DEPEND ON
    // CHANGED (FI9): the live mints, the ledger, or the save's minted rows.
    // Otherwise the walk over every dormant mint the save describes is
    // skipped. Measured before the gate: 24 ms a tick with 10,000 dormant
    // mints.
    //
    // ⛔ THE SAME VALUES, NOT "WRITTEN" (M2 cut C). A rollback load writes the
    // ledger and the save on every resimulated frame, so an `is_changed()`
    // gate opened on every frame. The ledger and the save's rows are shared
    // allocations, so a load of unchanged rows matches the key by allocation.
    let key_now = MintedMirrorKey {
        live: live_minted_descriptions(&minted),
        ledger: ledger.as_deref().cloned(),
        save_rows: save.data().minted_items_shared().clone(),
    };
    if mirrored_with.as_ref() == Some(&key_now) {
        return;
    }
    let live = key_now.live.clone();
    let described = with_dormant_mints(
        live,
        minted_baseline_from_save(save.data()).minted,
        ledger.as_deref(),
    );
    let minted_items: Vec<PersistedMintedItem> = described
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

    if save.data().minted_items() != minted_items {
        // ⛔ `data_mut()` ONLY PAST THE GUARD ABOVE. Reaching it derefs the
        // `ResMut`, which marks the resource changed whether or not the value
        // differs -- that is what the guard protects.
        save.data_mut().set_minted_items(minted_items);
    }
    // The key holds the save's rows as they are now, after the write.
    *mirrored_with = Some(MintedMirrorKey {
        save_rows: save.data().minted_items_shared().clone(),
        ..key_now
    });
}

/// The inputs the save's minted rows were last mirrored with: the live mint
/// descriptions, the occurrence ledger, and the save's minted rows. The ledger
/// and the save's rows are shared allocations, so a match is by allocation
/// first.
pub struct MintedMirrorKey {
    live: BTreeMap<SimId, MintedItemDescription>,
    ledger: Option<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
    save_rows: std::sync::Arc<Vec<PersistedMintedItem>>,
}

impl PartialEq for MintedMirrorKey {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.save_rows, &other.save_rows)
            && self.ledger == other.ledger
            && self.live == other.live
    }
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
        "entity-free stored-quantity and checkpoint purse checksum projection",
        OwnedItemsBaseline::checksum,
    );
    // A collection writes it on a tick, so a rewind across that tick takes
    // the grant back out of the record with the coins out of the wallet.
    registrar.rollback_resource_clone_checksum::<RewardGrantsSinceCheckpoint>(
        OWNER,
        "resource.reward_grants_since_checkpoint",
        "the grants of the pickups collected and the chests opened since the last checkpoint: a retracted boss defeat takes back a mint's and its reward chest's, and a death keeps a placed source's while one of its owners is spared",
        RewardGrantsSinceCheckpoint::checksum,
    );
    // A throw writes it on a tick, so a rewind across that tick takes the
    // spend back out of the record with the quantity back into the bag.
    registrar.rollback_resource_clone_checksum::<ambition_held_items::BagSpendsSinceCheckpoint>(
        OWNER,
        "resource.bag_spends_since_checkpoint",
        "the quantities of the shared bag that throws made into objects since the last checkpoint: a death keeps the spend of each object it keeps",
        ambition_held_items::BagSpendsSinceCheckpoint::checksum,
    );
}

/// The live descriptions, and each earlier description of a mint that is not
/// live but that the occurrence ledger places in a room: a dormant mint.
///
/// The ledger decides how long a dormant description lives. When it no longer
/// places the occurrence (taken up again, retracted by a checkpoint restore, or
/// gone), the description goes too. A composition with no ledger has no
/// dormant mints.
pub fn with_dormant_mints(
    live: BTreeMap<SimId, MintedItemDescription>,
    earlier: BTreeMap<SimId, MintedItemDescription>,
    ledger: Option<&ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
) -> BTreeMap<SimId, MintedItemDescription> {
    let mut described = live;
    let Some(ledger) = ledger else {
        return described;
    };
    for (occurrence, description) in earlier {
        let placed = matches!(
            ledger.whereabouts(&occurrence),
            Some(ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Placed { .. })
        );
        if placed {
            described.entry(occurrence).or_insert(description);
        }
    }
    described
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
        GroundItem::at_rest(
            ambition_characters::brain::HeldItemSpec {
                id: spec_id.into(),
                melee: None,
                ranged: None,
                use_behavior: ambition_characters::brain::HeldUseBehavior::ThrowOnUse,
            },
            Vec2::new(11.0, 22.0),
            Vec2::splat(18.0),
        )
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
