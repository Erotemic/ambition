//! Per-component checksum localization across rollback save/load.
//!
//! Each registered component is censused immediately around a restore. Entity
//! ids and iteration order may change, so probes use carrier count plus wrapping
//! sums of value projections. Presence-only probes are tracked explicitly; the
//! aggregate rollback checksum remains the equality oracle.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

/// One registered component's order-independent census.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComponentCensus {
    /// How many entities carried this component.
    pub count: usize,
    /// Order-independent wrapping sum of carrier checksum projections. XOR is
    /// not used because equal carrier values can cancel in pairs.
    pub xor: u64,
}

/// How much component state a probe observes. Presence-only value-bearing
/// components are enumerated so weak coverage stays explicit; zero-sized marker
/// components are complete when counted by carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeStrength {
    /// Counts carriers. Detects a lost or gained carrier; blind to the value.
    Presence,
    /// Counts carriers of a ZERO-SIZED type, where presence IS the whole value.
    ///
    /// Mechanically distinguished (`size_of::<T>() == 0`) rather than listed, and
    /// that distinction carries most of the weight: a marker like `Collected`,
    /// `PrimaryPlayer`, or `SwitchOn` has no state a value projection could examine,
    /// so a carrier count is not a weaker measurement of it — it is the measurement.
    /// Filing those under "presence-only, reason unstated" would have buried the
    /// handful of genuinely under-probed types in eighty-odd markers.
    Complete,
    /// Folds a projection of each carrier's VALUE. Detects a changed value as well
    /// as a changed population.
    Value,
}

/// A type-erased census probe for one registered rollback component.
///
/// The census is boxed rather than a bare `fn` pointer because two registration
/// arms take the checksum projection as a PARAMETER
/// (`rollback_component_clone_checksum` and its resource twin). Those are the arms
/// that shipped with no probe at all, and a probe shape that could not hold a
/// caller-supplied projection is why: the only thing left to offer them would have
/// been presence-only, for types that have a perfectly good value projection right
/// there in the call.
#[derive(Clone)]
pub struct ChecksumProbe {
    pub type_name: &'static str,
    census: std::sync::Arc<dyn Fn(&mut World) -> ComponentCensus + Send + Sync>,
    /// True for state declared DERIVED rather than snapshotted.
    ///
    /// Derived state is legitimately absent or stale immediately after a load — that is what
    /// "derived" means.
    derived: bool,
    strength: ProbeStrength,
}

impl ChecksumProbe {
    pub fn census(&self, world: &mut World) -> ComponentCensus {
        (self.census)(world)
    }
}

/// Localization probes for rollback-registered components.
///
/// Registration populates this collection so state-bearing rollback types cannot
/// be invisible to localization. [`ProbeStrength`] distinguishes value probes from
/// marker/presence-only probes; policy tests require weak probes to be named and
/// justified explicitly.
#[derive(Resource, Default, Clone)]
pub struct RollbackChecksumProbes {
    probes: Vec<ChecksumProbe>,
}

impl RollbackChecksumProbes {
    pub fn len(&self) -> usize {
        self.probes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    pub fn probes(&self) -> impl Iterator<Item = &ChecksumProbe> {
        self.probes.iter()
    }

    /// Census every registered component, keyed by type name.
    pub fn census_all(&self, world: &mut World) -> BTreeMap<&'static str, ComponentCensus> {
        self.probes
            .iter()
            .map(|probe| (probe.type_name, probe.census(world)))
            .collect()
    }

    /// Every probed type name. What a coverage guard compares the rollback
    /// registry's state-bearing descriptors against.
    pub fn type_names(&self) -> BTreeSet<&'static str> {
        self.probes.iter().map(|probe| probe.type_name).collect()
    }

    /// Every type whose probe can see only PRESENCE, not value.
    ///
    /// What the coverage guard enumerates. A registration in this set is snapshotted
    /// and remapped correctly as far as the localizer can tell, which is not the same
    /// claim as "restored identically".
    pub fn presence_only_type_names(&self) -> BTreeSet<&'static str> {
        self.probes
            .iter()
            .filter(|probe| probe.strength == ProbeStrength::Presence)
            .map(|probe| probe.type_name)
            .collect()
    }

    /// Count by strength: `(complete, value, presence_only)`.
    pub fn strength_tally(&self) -> (usize, usize, usize) {
        let mut tally = (0, 0, 0);
        for probe in &self.probes {
            match probe.strength {
                ProbeStrength::Complete => tally.0 += 1,
                ProbeStrength::Value => tally.1 += 1,
                ProbeStrength::Presence => tally.2 += 1,
            }
        }
        tally
    }

    /// The subset that is SNAPSHOT state — the only thing a restore must reproduce.
    pub fn snapshot_type_names(&self) -> BTreeSet<&'static str> {
        self.probes
            .iter()
            .filter(|probe| !probe.derived)
            .map(|probe| probe.type_name)
            .collect()
    }
}

impl ChecksumProbe {
    /// A VALUE probe: the census folds a projection of each carrier's state.
    pub fn new(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name,
            census: std::sync::Arc::new(census),
            derived: false,
            strength: ProbeStrength::Value,
        }
    }

    /// A PRESENCE-only probe, for a registration that supplied no projection to
    /// measure. Named separately from [`Self::new`] so that choosing the weaker one
    /// is a decision at the call site rather than a property of which census
    /// function happened to be passed.
    ///
    /// The strength recorded is [`ProbeStrength::Complete`] for a ZERO-SIZED type,
    /// where there is no value beyond the carrier: a marker's presence is not a
    /// partial view of its state, it is all of it.
    pub fn presence(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name,
            census: std::sync::Arc::new(census),
            derived: false,
            strength: ProbeStrength::Presence,
        }
    }

    /// Presence for a zero-sized type — [`ProbeStrength::Complete`].
    pub fn marker(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name,
            census: std::sync::Arc::new(census),
            derived: false,
            strength: ProbeStrength::Complete,
        }
    }

    /// A presence census whose strength is decided by whether the type has any state
    /// at all. The registration arms that supply no projection call this.
    pub fn presence_for<T: 'static>(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        if std::mem::size_of::<T>() == 0 {
            Self::marker(type_name, census)
        } else {
            Self::presence(type_name, census)
        }
    }

    /// A probe for DERIVED state: compared across resimulation, not across restore.
    /// Strength follows the same zero-sized rule as [`Self::presence_for`].
    pub fn derived_for<T: 'static>(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        let mut probe = if std::mem::size_of::<T>() == 0 {
            Self::marker(type_name, census)
        } else {
            Self::presence(type_name, census)
        };
        probe.derived = true;
        probe
    }

    pub fn derived(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name,
            census: std::sync::Arc::new(census),
            derived: true,
            strength: ProbeStrength::Presence,
        }
    }

    /// A derived declaration that DID supply a value projection, so the
    /// resimulation comparison can see a wrongly-REBUILT value and not merely a
    /// missing one.
    pub fn derived_value(
        type_name: &'static str,
        census: impl Fn(&mut World) -> ComponentCensus + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name,
            census: std::sync::Arc::new(census),
            derived: true,
            strength: ProbeStrength::Value,
        }
    }

    pub fn is_derived(&self) -> bool {
        self.derived
    }

    pub fn strength(&self) -> ProbeStrength {
        self.strength
    }
}

impl RollbackChecksumProbes {
    pub fn register(&mut self, probe: ChecksumProbe) {
        if self
            .probes
            .iter()
            .any(|existing| existing.type_name == probe.type_name)
        {
            return;
        }
        self.probes.push(probe);
    }
}

/// Census a component through the canonical STATE projection.
///
/// A plain generic function rather than a closure: the projection is decided by the
/// trait bound, not captured, so the registration arm names nothing twice.
pub fn census_state<T>(world: &mut World) -> ComponentCensus
where
    T: Component + super::SnapshotState,
{
    fold(world, super::state_checksum::<T>)
}

/// Census a component through the mutable-CURSOR projection.
pub fn census_cursor<T>(world: &mut World) -> ComponentCensus
where
    T: Component + super::SnapshotCursor,
{
    fold(world, super::cursor_checksum::<T>)
}

/// Census a component through the authored-REFERENCE projection.
pub fn census_resolved<T>(world: &mut World) -> ComponentCensus
where
    T: Component + super::SnapshotResolve,
{
    fold(world, super::resolved_checksum::<T>)
}

/// Census a component through a CALLER-SUPPLIED checksum projection.
///
/// The registration arms that take `checksum: fn(&T) -> u64` hand the same function
/// to GGRS and to this, so the probe measures byte-for-byte what the session's
/// aggregate measures. That is the strongest census available and it costs nothing
/// extra — the projection is already at the call site.
pub fn census_with<T>(world: &mut World, projection: fn(&T) -> u64) -> ComponentCensus
where
    T: Component,
{
    fold(world, projection)
}

/// Resource twin of [`census_with`].
pub fn census_resource_with<T>(world: &mut World, projection: fn(&T) -> u64) -> ComponentCensus
where
    T: Resource,
{
    fold_resource(world, projection)
}

/// Census entity references through stable [`SimId`](ambition_platformer2d_shared_tangle::sim_id::SimId) identity.
///
/// Raw Bevy entity ids change across rollback restores. Folding carrier/target
/// identity pairs detects redirects and permutations while remaining stable across
/// entity recreation; missing or unidentified targets use distinct sentinels.
pub fn census_entity_reference<T>(
    world: &mut World,
    referenced: fn(&T) -> Entity,
) -> ComponentCensus
where
    T: Component,
{
    let pairs: Vec<(Entity, Entity)> = {
        let mut query = world.query::<(Entity, &T)>();
        query
            .iter(world)
            .map(|(carrier, value)| (carrier, referenced(value)))
            .collect()
    };
    let count = pairs.len();
    let mut sum: u64 = 0;
    for (carrier, target) in pairs {
        // HASH THE PAIR.
        //
        // Hash the pair together rather than arithmetically blending independent
        // hashes; linear mixtures decompose under the outer sum and cannot detect
        // carrier/target swaps. The outer wrapping sum still keeps iteration order
        // irrelevant.
        let mut pair = [0u8; 16];
        pair[..8].copy_from_slice(&stable_identity(world, carrier).to_le_bytes());
        pair[8..].copy_from_slice(&stable_identity(world, target).to_le_bytes());
        sum = sum.wrapping_add(super::checksum_bytes(&pair));
    }
    ComponentCensus { count, xor: sum }
}

/// An entity's rollback-stable identity, folded into a probe.
///
/// `SimId` when it has one — the authored string a correct remap preserves. The two
/// fallbacks are deliberately DIFFERENT constants because they are different facts:
/// a live body that carries no authored identity, and a reference into nothing.
///
/// A carrier with no `SimId` degrades the pairing above back toward target-only:
/// every such carrier contributes the same constant, so a permutation among
/// identity-less carriers is still invisible.
///
/// Projectiles are minted one by `mint_spawned_sim_ids` — but only when their OWNER
/// carries both a `SimId` and a `SimIdCounter`, which that system's query requires.
/// So the permutation sensitivity is real for the owned pool this was built for and
/// degrades to redirect-only for a bolt whose firer has no authored identity. Stated
/// rather than assumed, because the whole point of `ProbeStrength` is that an
/// instrument's reach should be written down and not inferred from its name.
fn stable_identity(world: &World, entity: Entity) -> u64 {
    match world.get::<ambition_platformer2d_shared_tangle::sim_id::SimId>(entity) {
        Some(id) => super::checksum_bytes(id.as_str().as_bytes()),
        None if world.get_entity(entity).is_ok() => 0x1111_1111_1111_1111,
        None => 0xDEAD_BEEF_DEAD_BEEF,
    }
}

/// Census a component holding a SET of entity references.
///
/// The multi-handle twin of [`census_entity_reference`]: `HitboxHits` holds the
/// victims a strike has already hit, and a restore that put back the right NUMBER
/// of hit-sets while losing one victim from one of them is the difference between a
/// sustained overlap re-hitting a body and not.
///
/// Per carrier the targets are folded with a wrapping SUM — a `HashSet` has no
/// stable iteration order, so anything order-sensitive would report noise — and the
/// resulting digest is then hashed together with the CARRIER's identity, so two
/// strikes swapping their victim lists is a different census. Same reasoning as the
/// pair projection, one level up.
pub fn census_entity_set<T>(world: &mut World, referenced: fn(&T) -> Vec<Entity>) -> ComponentCensus
where
    T: Component,
{
    let rows: Vec<(Entity, Vec<Entity>)> = {
        let mut query = world.query::<(Entity, &T)>();
        query
            .iter(world)
            .map(|(carrier, value)| (carrier, referenced(value)))
            .collect()
    };
    let count = rows.len();
    let mut sum: u64 = 0;
    for (carrier, targets) in rows {
        let mut digest: u64 = targets.len() as u64;
        for target in targets {
            digest = digest.wrapping_add(stable_identity(world, target));
        }
        let mut pair = [0u8; 16];
        pair[..8].copy_from_slice(&stable_identity(world, carrier).to_le_bytes());
        pair[8..].copy_from_slice(&digest.to_le_bytes());
        sum = sum.wrapping_add(super::checksum_bytes(&pair));
    }
    ComponentCensus { count, xor: sum }
}

/// Census a component holding a KEYED MAP of entity references.
///
/// ```text
/// hand_left → limb A          hand_left → limb B
/// hand_right → limb B         hand_right → limb A
/// ```
///
/// So the KEY is folded into each entry's hash, not merely its target: an entry
/// digests `hash(key ++ target identity)`, and a swap changes both entries'
/// bytes. Entries are still summed, which keeps the census independent of map
/// iteration order, and the per-carrier digest is hashed with the carrier's own
/// identity for the same reason [`census_entity_set`] does it.
///
/// The key is whatever stable `u64` the caller derives from its own key type —
/// an enum discriminant, a hashed name. It only has to agree across the peers
/// comparing checksums, which run the same binary, so a discriminant is enough.
pub fn census_entity_map<T>(
    world: &mut World,
    referenced: fn(&T) -> Vec<(u64, Entity)>,
) -> ComponentCensus
where
    T: Component,
{
    let rows: Vec<(Entity, Vec<(u64, Entity)>)> = {
        let mut query = world.query::<(Entity, &T)>();
        query
            .iter(world)
            .map(|(carrier, value)| (carrier, referenced(value)))
            .collect()
    };
    let count = rows.len();
    let mut sum: u64 = 0;
    for (carrier, entries) in rows {
        let mut digest: u64 = entries.len() as u64;
        for (key, target) in entries {
            let mut entry = [0u8; 16];
            entry[..8].copy_from_slice(&key.to_le_bytes());
            entry[8..].copy_from_slice(&stable_identity(world, target).to_le_bytes());
            digest = digest.wrapping_add(super::checksum_bytes(&entry));
        }
        let mut pair = [0u8; 16];
        pair[..8].copy_from_slice(&stable_identity(world, carrier).to_le_bytes());
        pair[8..].copy_from_slice(&digest.to_le_bytes());
        sum = sum.wrapping_add(super::checksum_bytes(&pair));
    }
    ComponentCensus { count, xor: sum }
}

/// Census a RESOURCE holding entity references, through stable sim identity.
///
/// The resource twin of [`census_entity_set`]. A resource has no carrier entity
/// to pair against, so the ORDER of the projected handles is the pairing: the
/// index is folded in with each identity, which is what distinguishes
/// `PossessionState { possessed: A, home: B }` from the same pair swapped — a
/// restore that exchanged the possessed body and the home avatar would otherwise
/// fold identically while inverting the whole possession.
pub fn census_resource_entity_set<T>(
    world: &mut World,
    referenced: fn(&T) -> Vec<Entity>,
) -> ComponentCensus
where
    T: Resource,
{
    let Some(targets) = world.get_resource::<T>().map(referenced) else {
        return ComponentCensus { count: 0, xor: 0 };
    };
    let mut sum: u64 = 0;
    for (index, target) in targets.iter().enumerate() {
        let mut pair = [0u8; 16];
        pair[..8].copy_from_slice(&(index as u64).to_le_bytes());
        pair[8..].copy_from_slice(&stable_identity(world, *target).to_le_bytes());
        sum = sum.wrapping_add(super::checksum_bytes(&pair));
    }
    ComponentCensus {
        count: targets.len(),
        xor: sum,
    }
}

/// Census PRESENCE only, for state registered with NO checksum projection at all.
///
/// This is the honest answer for `rollback_component_clone`, whose whole contract is
/// "snapshotted here, checksummed by some other authoritative projection". A probe
/// that cannot see the value can still see the carrier disappear, and `ProjectileOwner`
/// — registered exactly this way — is the state the equipment divergence turned on.
pub fn census_presence<T>(world: &mut World) -> ComponentCensus
where
    T: Component,
{
    let mut query = world.query::<&T>();
    ComponentCensus {
        count: query.iter(world).count(),
        xor: 0,
    }
}

/// Census a rollback-registered RESOURCE through the canonical state projection.
///
/// `count` is 0 or 1, which also distinguishes "absent after a load" from "present but
/// different".
pub fn census_resource_state<T>(world: &mut World) -> ComponentCensus
where
    T: Resource + super::SnapshotState,
{
    fold_resource(world, super::state_checksum::<T>)
}

pub fn census_resource_cursor<T>(world: &mut World) -> ComponentCensus
where
    T: Resource + super::SnapshotCursor,
{
    fold_resource(world, super::cursor_checksum::<T>)
}

/// Presence-only resource census, for caller-supplied checksum projections.
pub fn census_resource_presence<T>(world: &mut World) -> ComponentCensus
where
    T: Resource,
{
    ComponentCensus {
        count: usize::from(world.get_resource::<T>().is_some()),
        xor: 0,
    }
}

fn fold_resource<T: Resource>(world: &mut World, projection: fn(&T) -> u64) -> ComponentCensus {
    match world.get_resource::<T>() {
        Some(value) => ComponentCensus {
            count: 1,
            xor: projection(value),
        },
        None => ComponentCensus::default(),
    }
}

fn fold<T: Component>(world: &mut World, projection: fn(&T) -> u64) -> ComponentCensus {
    let mut query = world.query::<&T>();
    let mut out = ComponentCensus::default();
    for value in query.iter(world) {
        out.count += 1;
        out.xor = out.xor.wrapping_add(projection(value));
    }
    out
}

/// WHERE a component's value stopped agreeing with itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivergenceBoundary {
    /// Saved at frame `F`, and different immediately after loading frame `F`.
    /// The snapshot did not put the value back.
    Restore,
    /// Saved at frame `F` on the first pass, and different when frame `F` was
    /// reached AGAIN by resimulation. The restore was faithful and the REPLAY
    /// produced something else, which means the system that writes this component
    /// read something the rollback does not restore — an unregistered resource, a
    /// `Local<T>`, an asset, or an order-dependent accumulation.
    Resimulation,
}

impl DivergenceBoundary {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Restore => "did not survive its own snapshot",
            Self::Resimulation => "was recomputed differently on replay",
        }
    }
}

/// One component that changed across a save → load of the SAME frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreDivergence {
    pub frame: i32,
    pub boundary: DivergenceBoundary,
    pub type_name: &'static str,
    pub saved: ComponentCensus,
    pub restored: ComponentCensus,
}

impl std::fmt::Display for RestoreDivergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "frame {}: `{}` {} — first {} entities (xor {:#018x}), then {} entities \
             (xor {:#018x})",
            self.frame,
            self.type_name,
            self.boundary.as_str(),
            self.saved.count,
            self.saved.xor,
            self.restored.count,
            self.restored.xor,
        )
    }
}

/// Localization state: what each frame's snapshot contained, and what came back.
#[derive(Resource, Default)]
pub struct RollbackRestoreAudit {
    /// Enabled explicitly. Censusing every registered component on every save and
    /// every load is far too expensive for a shipping frame, and a diagnostic that
    /// is always on becomes a diagnostic nobody can afford to leave on.
    pub enabled: bool,
    saved: BTreeMap<i32, BTreeMap<&'static str, ComponentCensus>>,
    /// Every component that failed to survive its own snapshot, in discovery order.
    pub divergences: Vec<RestoreDivergence>,
    /// How many frames were censused at save time.
    pub saves: usize,
    /// How many loads were observed.
    pub loads: usize,
    /// How many saves were REPEAT saves of a frame already seen — i.e. how many
    /// resimulation comparisons were possible.
    pub resimulations: usize,
    /// How many loads were actually COMPARED against a saved census.
    ///
    /// The difference between this and `loads` is the whole reason it exists: a
    /// localizer that reports "no divergence" while never comparing anything is
    /// worse than no localizer, because it launders an absence of evidence into
    /// evidence of absence. A caller must assert this is non-zero.
    pub comparisons: usize,
}

impl RollbackRestoreAudit {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// The first divergence, which is the one to fix: later ones are usually
    /// consequences of it.
    pub fn first(&self) -> Option<&RestoreDivergence> {
        self.divergences.first()
    }

    /// Distinct component type names that ever diverged, deterministically ordered.
    pub fn diverging_types(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self
            .divergences
            .iter()
            .map(|divergence| divergence.type_name)
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// What the audit actually observed, so a green result can be believed.
    pub fn coverage(&self) -> String {
        format!(
            "{} save(s) censused ({} of them repeats of an already-seen frame, so \
             replay-comparable), {} load(s) observed, {} compared against a saved \
             baseline",
            self.saves, self.resimulations, self.loads, self.comparisons
        )
    }

    pub fn report(&self) -> String {
        if self.divergences.is_empty() {
            return "no component changed across a save/load of the same frame".to_owned();
        }
        let mut out = format!(
            "{} component(s) disagreed with themselves:\n",
            self.diverging_types().len()
        );
        for divergence in &self.divergences {
            out.push_str(&format!("  {divergence}\n"));
        }
        out
    }
}

/// Record what frame `F`'s snapshot contained — and if this frame has been saved
/// before, COMPARE, because a repeat save of the same frame is a resimulation.
///
/// GGRS saves after every advance, so a rolled-back frame is saved once on the
/// first pass and again on each replay. That makes this the cheapest possible
/// place to answer "the aggregate says frames 149-151 differ — differ in WHAT":
/// no second run, no bisection, just the census already being taken.
pub fn record_saved_census(world: &mut World) {
    if !world
        .get_resource::<RollbackRestoreAudit>()
        .is_some_and(|audit| audit.enabled)
    {
        return;
    }
    let Some(frame) = current_rollback_frame(world) else {
        return;
    };
    let Some(probes) = world.get_resource::<RollbackChecksumProbes>().cloned() else {
        return;
    };
    let census = probes.census_all(world);
    let previous = world
        .get_resource::<RollbackRestoreAudit>()
        .and_then(|audit| audit.saved.get(&frame).cloned());
    let mut found = Vec::new();
    if let Some(previous) = previous.as_ref() {
        for (type_name, now) in &census {
            let before = previous.get(type_name).copied().unwrap_or_default();
            if before != *now {
                found.push(RestoreDivergence {
                    frame,
                    boundary: DivergenceBoundary::Resimulation,
                    type_name,
                    saved: before,
                    restored: *now,
                });
            }
        }
    }
    if let Some(mut audit) = world.get_resource_mut::<RollbackRestoreAudit>() {
        audit.saves += 1;
        if previous.is_some() {
            audit.resimulations += 1;
        }
        audit.divergences.extend(found);
        // Overwriting would compare replay N against replay N-1 and go quiet once the error became
        // consistent.
        audit.saved.entry(frame).or_insert(census);
    }
}

/// Compare frame `F` against what its snapshot claimed to hold, at load time.
pub fn compare_restored_census(world: &mut World) {
    if !world
        .get_resource::<RollbackRestoreAudit>()
        .is_some_and(|audit| audit.enabled)
    {
        return;
    }
    let Some(frame) = current_rollback_frame(world) else {
        return;
    };
    let Some(probes) = world.get_resource::<RollbackChecksumProbes>().cloned() else {
        return;
    };
    let restored = probes.census_all(world);
    if let Some(mut audit) = world.get_resource_mut::<RollbackRestoreAudit>() {
        audit.loads += 1;
    }
    let Some(saved) = world
        .get_resource::<RollbackRestoreAudit>()
        .and_then(|audit| audit.saved.get(&frame).cloned())
    else {
        // Loading a frame this run never saved: nothing to compare against, which
        // is not a defect (the very first load of a session-restored frame).
        return;
    };
    let snapshot_only = probes.snapshot_type_names();
    let mut found = Vec::new();
    for (type_name, restored_census) in &restored {
        // Derived state is expected to be absent or stale here; its contract is
        // tested at the resimulation boundary instead.
        if !snapshot_only.contains(type_name) {
            continue;
        }
        let saved_census = saved.get(type_name).copied().unwrap_or_default();
        if saved_census != *restored_census {
            found.push(RestoreDivergence {
                frame,
                boundary: DivergenceBoundary::Restore,
                type_name,
                saved: saved_census,
                restored: *restored_census,
            });
        }
    }
    if let Some(mut audit) = world.get_resource_mut::<RollbackRestoreAudit>() {
        audit.comparisons += 1;
        audit.divergences.extend(found);
    }
}

/// The frame bevy_ggrs is currently saving or loading.
fn current_rollback_frame(world: &World) -> Option<i32> {
    world
        .get_resource::<bevy_ggrs::RollbackFrameCount>()
        .map(|count| count.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    #[derive(Component, Clone)]
    struct Owner(Entity);

    /// A value probe over an entity reference actually sees a wrong target.
    ///
    /// The claim `ProbeStrength::Value` makes is falsifiable only if a mis-mapped
    /// reference changes the census, so this asserts it directly rather than trusting
    /// the label. Same carrier count both times — which is exactly what the presence
    /// probe this replaced could measure, and all it could measure.
    #[test]
    fn remapping_a_reference_to_a_different_body_changes_the_census() {
        let mut world = World::new();
        let alpha = world.spawn(SimId::placement("alpha")).id();
        let beta = world.spawn(SimId::placement("beta")).id();
        let bolt = world.spawn(Owner(alpha)).id();

        let correct = census_entity_reference::<Owner>(&mut world, |owner| owner.0);
        world.entity_mut(bolt).insert(Owner(beta));
        let wrong = census_entity_reference::<Owner>(&mut world, |owner| owner.0);

        assert_eq!(
            (correct.count, wrong.count),
            (1, 1),
            "the population is unchanged, so a presence probe reports these two \
             worlds as identical — that is the blindness this projection removes"
        );
        assert_ne!(
            correct.xor, wrong.xor,
            "pointing the bolt at a different body must change the census"
        );
    }

    /// H4: a PERMUTATION is a wrong restore too.
    #[test]
    fn swapping_two_carriers_owners_changes_the_census() {
        let mut world = World::new();
        let alpha = world.spawn(SimId::placement("alpha")).id();
        let beta = world.spawn(SimId::placement("beta")).id();
        let bolt_a = world.spawn((SimId::placement("bolt_a"), Owner(alpha))).id();
        let bolt_b = world.spawn((SimId::placement("bolt_b"), Owner(beta))).id();

        let correct = census_entity_reference::<Owner>(&mut world, |owner| owner.0);
        world.entity_mut(bolt_a).insert(Owner(beta));
        world.entity_mut(bolt_b).insert(Owner(alpha));
        let swapped = census_entity_reference::<Owner>(&mut world, |owner| owner.0);

        assert_eq!((correct.count, swapped.count), (2, 2));
        assert_ne!(
            correct.xor, swapped.xor,
            "the same two owners, attached to the other bolts, is a different world"
        );
    }

    /// And it is still order-independent, which is the property the whole module
    /// depends on: bevy_ggrs recreates entities, so archetype order changes on every
    /// load and a census that noticed would report a difference every time.
    ///
    /// Rebuilding the same associations in the opposite order must census the same.
    #[test]
    fn the_pairing_is_still_independent_of_iteration_order() {
        fn build(reversed: bool) -> u64 {
            let mut world = World::new();
            let alpha = world.spawn(SimId::placement("alpha")).id();
            let beta = world.spawn(SimId::placement("beta")).id();
            let pairs = [("bolt_a", alpha), ("bolt_b", beta)];
            let ordered: Vec<_> = if reversed {
                pairs.iter().rev().copied().collect()
            } else {
                pairs.to_vec()
            };
            for (name, owner) in ordered {
                world.spawn((SimId::placement(name), Owner(owner)));
            }
            census_entity_reference::<Owner>(&mut world, |owner| owner.0).xor
        }
        assert_eq!(build(false), build(true));
    }

    #[derive(Component, Clone)]
    struct Hits(Vec<Entity>);

    /// A hit-set that lost one victim is a strike that will hit that body again.
    /// The carrier count is unchanged, so presence could not see it.
    #[test]
    fn losing_one_victim_from_a_hit_set_changes_the_census() {
        let mut world = World::new();
        let alpha = world.spawn(SimId::placement("alpha")).id();
        let beta = world.spawn(SimId::placement("beta")).id();
        let strike = world
            .spawn((SimId::placement("strike"), Hits(vec![alpha, beta])))
            .id();

        let both = census_entity_set::<Hits>(&mut world, |hits| hits.0.clone());
        world.entity_mut(strike).insert(Hits(vec![alpha]));
        let one = census_entity_set::<Hits>(&mut world, |hits| hits.0.clone());

        assert_eq!((both.count, one.count), (1, 1));
        assert_ne!(both.xor, one.xor);
    }

    /// And two strikes exchanging their victim lists is a different world, for the
    /// same reason a permutation of owners is.
    #[test]
    fn two_strikes_swapping_hit_sets_changes_the_census() {
        let mut world = World::new();
        let alpha = world.spawn(SimId::placement("alpha")).id();
        let beta = world.spawn(SimId::placement("beta")).id();
        let first = world
            .spawn((SimId::placement("first"), Hits(vec![alpha])))
            .id();
        let second = world
            .spawn((SimId::placement("second"), Hits(vec![beta])))
            .id();

        let correct = census_entity_set::<Hits>(&mut world, |hits| hits.0.clone());
        world.entity_mut(first).insert(Hits(vec![beta]));
        world.entity_mut(second).insert(Hits(vec![alpha]));
        let swapped = census_entity_set::<Hits>(&mut world, |hits| hits.0.clone());
        assert_ne!(correct.xor, swapped.xor);
    }

    #[derive(Component, Clone)]
    struct Rig(Vec<(u64, Entity)>);

    /// The `LimbRig` failure, in miniature: two SLOTS exchange their limbs. The
    /// set census cannot see this — the targets are the same multiset and its
    /// fold is a sum — which is why the map census exists and why this test
    /// asserts both halves rather than only the one that passes.
    #[test]
    fn two_slots_exchanging_their_limbs_changes_the_map_census() {
        let mut world = World::new();
        let left = world.spawn(SimId::placement("limb_left")).id();
        let right = world.spawn(SimId::placement("limb_right")).id();
        let host = world
            .spawn((SimId::placement("host"), Rig(vec![(0, left), (1, right)])))
            .id();

        let correct = census_entity_map::<Rig>(&mut world, |rig| rig.0.clone());
        let correct_as_set =
            census_entity_set::<Rig>(&mut world, |rig| rig.0.iter().map(|(_, e)| *e).collect());
        world
            .entity_mut(host)
            .insert(Rig(vec![(0, right), (1, left)]));
        let swapped = census_entity_map::<Rig>(&mut world, |rig| rig.0.clone());
        let swapped_as_set =
            census_entity_set::<Rig>(&mut world, |rig| rig.0.iter().map(|(_, e)| *e).collect());

        assert_ne!(correct.xor, swapped.xor, "the map census must see the swap");
        assert_eq!(
            correct_as_set.xor, swapped_as_set.xor,
            "and the set census must NOT — that blindness is the whole reason \
             this projection is a different helper, so if this ever starts \
             failing the two can be merged"
        );
    }

    /// A limb moving to a slot the rig did not have is not the same rig, even
    /// though the same limbs are attached.
    #[test]
    fn rekeying_a_limb_changes_the_map_census() {
        let mut world = World::new();
        let limb = world.spawn(SimId::placement("limb")).id();
        let host = world
            .spawn((SimId::placement("host"), Rig(vec![(0, limb)])))
            .id();

        let at_zero = census_entity_map::<Rig>(&mut world, |rig| rig.0.clone());
        world.entity_mut(host).insert(Rig(vec![(1, limb)]));
        let at_one = census_entity_map::<Rig>(&mut world, |rig| rig.0.clone());
        assert_ne!(at_zero.xor, at_one.xor);
    }

    /// A reference into nothing is distinguishable from a reference to an
    /// identity-less body: they are different failures and want different answers.
    #[test]
    fn a_dangling_reference_and_an_unidentified_target_are_different_censuses() {
        let mut world = World::new();
        let anonymous = world.spawn_empty().id();
        let bolt = world.spawn(Owner(anonymous)).id();
        let unidentified = census_entity_reference::<Owner>(&mut world, |owner| owner.0);

        world.entity_mut(anonymous).despawn();
        let dangling = census_entity_reference::<Owner>(&mut world, |owner| owner.0);
        assert_ne!(unidentified.xor, dangling.xor);

        // And a body WITH an identity is a third answer, not folded in with either.
        let identified = world.spawn(SimId::placement("gamma")).id();
        world.entity_mut(bolt).insert(Owner(identified));
        let named = census_entity_reference::<Owner>(&mut world, |owner| owner.0);
        assert_ne!(named.xor, unidentified.xor);
        assert_ne!(named.xor, dangling.xor);
    }
}
