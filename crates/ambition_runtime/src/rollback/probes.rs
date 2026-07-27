//! **Per-component checksum localization across the save/load boundary.**
//!
//! A GGRS sync test reports ONE aggregate checksum per frame. When it disagrees it
//! can say "frames [149, 150, 151] differ" and nothing more, so every divergence
//! becomes a bisection: remove an entity class, re-walk a 150-frame route, guess
//! again. The triage doc for the equipment oracle
//! (`docs/planning/triage/rollback-equipment-oracle-divergence.md`) ends by naming
//! this module as the tool that would have answered it in minutes.
//!
//! ## What it measures, and why that is the right question
//!
//! Not "do two runs agree" — that reproduces the aggregate's blindness with more
//! steps. It measures the **restore** directly: for each registered rollback
//! component, take a census immediately before bevy_ggrs saves frame `F`, and take
//! it again immediately after bevy_ggrs loads frame `F`. A component whose census
//! changed across that boundary is a component the snapshot did not put back, and
//! it is named.
//!
//! That catches the failure class the aggregate hides best: state that IS
//! registered but whose restored value differs, which is where the equipment
//! divergence was narrowed to after registration gaps and coverage gaps were both
//! ruled out.
//!
//! ## Order independence
//!
//! bevy_ggrs destroys and recreates rollback entities, so entity ids and archetype
//! order both change across a load. A census that folded per-entity checksums in
//! iteration order would therefore report a difference for every component on
//! every load — all noise, no signal. Each probe combines its per-entity
//! checksums with a wrapping SUM plus a count, which is invariant under reordering
//! and still detects a changed value, a lost carrier, or a gained one.
//!
//! Addition rather than XOR because XOR annihilates equal pairs: a component held
//! with the same value by exactly two entities censused as `0x0` and could hide a
//! compensating change. A genuine value SWAP between two carriers still survives
//! either combiner, and is accepted — this is a localizer pointing at a component,
//! not a proof of equality. The oracle remains the guard.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

/// One registered component's order-independent census.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComponentCensus {
    /// How many entities carried this component.
    pub count: usize,
    /// Order-independent SUM of each carrier's checksum projection — the SAME
    /// projection the GGRS aggregate uses, so a difference here is a difference the
    /// session would see.
    ///
    /// Wrapping addition rather than XOR, learned the hard way: XOR makes two
    /// carriers with IDENTICAL values cancel to zero, so a component held equal by
    /// exactly two entities reported `0x0000000000000000` and any change to one of
    /// them could be masked by a matching change to the other. Addition has no such
    /// annihilating pair.
    pub xor: u64,
}

/// **How much of a component's state a probe can actually see.**
///
/// The distinction is load-bearing and was invisible until GPT 5.6 named it: the
/// forcing test that closed F3 compares TYPE NAMES, so a presence-only probe
/// satisfies "this registration owns a probe" while reporting nothing about the
/// value. `ProjectileOwner` is the case that matters — snapshotted by clone, remapped
/// as an entity reference, and probed by counting carriers. A restore that put back
/// the right NUMBER of owners and pointed one of them at the wrong body was
/// indistinguishable from a correct one, on exactly the state the equipment
/// divergence turned on.
///
/// So strength is recorded, [`RollbackChecksumProbes::presence_only_type_names`]
/// enumerates the weak ones, and the guard compares that enumeration against an
/// explicit list with stated reasons. A weakness that has to be written down is a
/// different thing from one that has to be noticed.
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
    /// Derived state is legitimately absent or stale immediately after a load —
    /// that is what "derived" means. Its promise is that the named system rebuilds
    /// it before anything reads it, and the boundary that tests THAT promise is
    /// resimulation, not restore. Comparing derived state across a restore reports
    /// the contract working as designed; the first version of this module did
    /// exactly that and accused `ProjectileView`, a presentation read model, of
    /// being a determinism defect.
    derived: bool,
    strength: ProbeStrength,
}

impl ChecksumProbe {
    pub fn census(&self, world: &mut World) -> ComponentCensus {
        (self.census)(world)
    }
}

/// Every registered component's probe.
///
/// Populated by the registration seam itself, so a component cannot be
/// rollback-registered and remain invisible to localization. That coupling is the
/// point: the two holes in the previous instrument were both "the sweep did not
/// know to look here".
///
/// ⚠ That sentence was written before it was true. `record_probe` was called from
/// five of the ten state-bearing registration arms, so the plain-clone and
/// custom-checksum arms installed GGRS machinery and no probe — `RoomSet`,
/// `LdtkRuntimeIndex`, `EncounterParticipants`, `PendingPlayerHitEvents` and
/// `ProjectileOwner` among them, the last being the very state the equipment
/// divergence turned on. The claim is now enforced rather than asserted, by
/// `rollback_exit_oracle::every_state_bearing_rollback_registration_owns_a_localization_probe`,
/// which compares `type_names` against every descriptor whose
/// `RollbackEntryKind::carries_state`. A comment is not a coupling.
///
/// ⚠ And "owns a probe" is still weaker than it reads, which is why
/// [`ProbeStrength`] exists. That test compares type NAMES, so a probe that counts
/// carriers satisfies it while seeing nothing of the value —
/// `every_presence_only_probe_is_named_with_its_reason` is the second half, and it
/// is what keeps "254 of 254 probed" from being read as "254 of 254 checked".
/// Today that is 112 value probes, 22 markers where presence IS the value, and 120
/// presence-only registrations each named with the reason it cannot do better.
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

/// **Census a component holding an ENTITY REFERENCE, through stable sim identity.**
///
/// A raw `Entity` cannot be folded into a census: bevy_ggrs destroys and recreates
/// rollback entities, so the index and generation both change across a load and a
/// value probe over them would report a difference on every restore — all noise. That
/// is the honest reason `ProjectileOwner` shipped with a presence-only probe.
///
/// The projection that DOES survive is the referenced entity's authored identity:
/// [`SimId`](ambition_platformer_primitives::sim_id::SimId) is a stable string minted
/// from a placement, a player slot, or a summoner, and a correct remap points at the
/// same one. So this folds `hash(SimId of the target)` per carrier, and a distinct
/// sentinel for a target that carries no `SimId` or no longer exists — which still
/// distinguishes "remapped to a body with no identity" from "remapped to nothing".
///
/// The census folds `(carrier identity, target identity)` PAIRS rather than targets
/// alone, so it catches a permutation as well as a redirect: swapping two bolts'
/// owners leaves the multiset of targets untouched and would have matched.
///
/// This is the strength difference in one function: a restore that puts back the right
/// NUMBER of references and points one of them at a different body changes this census
/// and does not change a presence count.
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
        // HASH THE PAIR. Summing target identities alone cannot see a permutation:
        // swap two bolts' owners and the multiset of targets is unchanged, so the
        // census matched while every association was wrong — the precise failure
        // this probe exists to catch (GPT 5.6, 2026-07-26).
        //
        // And it has to be a hash of the two together, not an arithmetic blend of
        // two hashes: any `a*K + b` mixture decomposes back into
        // `K*Σtargets + Σcarriers` under the outer sum, both of which are
        // permutation-invariant. (Measured — the first version of this did exactly
        // that and its swap test failed.) Hashing the concatenated bytes has no such
        // decomposition, while the outer wrapping SUM keeps the census independent
        // of iteration order, which the whole module depends on.
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
/// ⚠ A carrier with no `SimId` degrades the pairing above back toward target-only:
/// every such carrier contributes the same constant, so a permutation among
/// identity-less carriers is still invisible.
///
/// Projectiles are minted one by `mint_spawned_sim_ids` — but only when their OWNER
/// carries both a `SimId` and a `SimIdCounter`, which that system's query requires.
/// So the permutation sensitivity is real for the owned pool this was built for and
/// degrades to redirect-only for a bolt whose firer has no authored identity. Stated
/// rather than assumed, because the whole point of `ProbeStrength` is that an
/// instrument's reach should be written down and not inferred from its name.
///
/// Strike volumes USED to be the worst case here — `Hitbox`/`StrikeVolume`/
/// `HitboxHits` all ride on them, and they spawned anonymous, so every one folded
/// to the same constant and the pair projection collapsed for exactly the carriers
/// it was added for. They now derive `SimId::strike_volume(owner, move, window,
/// volume)` at spawn; a bare test body with no owner id still mints nothing.
fn stable_identity(world: &World, entity: Entity) -> u64 {
    match world.get::<ambition_platformer_primitives::sim_id::SimId>(entity) {
        Some(id) => super::checksum_bytes(id.as_str().as_bytes()),
        None if world.get_entity(entity).is_ok() => 0x1111_1111_1111_1111,
        None => 0xDEAD_BEEF_DEAD_BEEF,
    }
}

/// **Census a component holding a SET of entity references.**
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

/// Census PRESENCE only, for state registered with NO checksum projection at all.
///
/// Weaker on purpose, and worth having: population change is exactly the failure
/// `PlayerVisual` had — the tag was simply absent after bevy_ggrs recreated the
/// entity — and a count catches that without knowing anything about the value.
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
/// Resources were the blind spot the first version of this module shipped with:
/// the component census cleanly named `MovePlayback` as recomputed-differently, and
/// the input it reads that a rollback might not restore — `WorldTime` — is a
/// resource, so the tool could name the symptom and not the cause. `count` is 0 or
/// 1, which also distinguishes "absent after a load" from "present but different".
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
        // Keep the FIRST pass as the baseline: comparing every replay against the
        // original is what localizes the defect. Overwriting would compare replay
        // N against replay N-1 and go quiet once the error became consistent.
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
    use ambition_platformer_primitives::sim_id::SimId;

    #[derive(Component, Clone)]
    struct Owner(Entity);

    /// **A value probe over an entity reference actually sees a wrong target.**
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

    /// **H4: a PERMUTATION is a wrong restore too.**
    ///
    /// Two bolts, two owners, swapped. The carrier count is unchanged and so is the
    /// multiset of targets, so a census that summed target identities alone reported
    /// these two worlds as identical — which is exactly the "right number of owners,
    /// wrong bolt-to-owner association" case the probe was added for (GPT 5.6,
    /// 2026-07-26).
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
