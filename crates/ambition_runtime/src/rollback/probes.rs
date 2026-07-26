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

use std::collections::BTreeMap;

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

/// A type-erased census probe for one registered rollback component.
#[derive(Clone)]
pub struct ChecksumProbe {
    pub type_name: &'static str,
    census: fn(&mut World) -> ComponentCensus,
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

}

impl ChecksumProbe {
    pub const fn new(type_name: &'static str, census: fn(&mut World) -> ComponentCensus) -> Self {
        Self { type_name, census }
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
/// A plain generic function rather than a closure so the probe stays a bare `fn`
/// pointer: the projection is decided by the trait bound, not captured.
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

/// Census PRESENCE only, for components whose checksum projection is a
/// caller-supplied function that cannot be baked into a `fn` pointer.
///
/// Weaker on purpose, and worth having: population change is exactly the failure
/// `PlayerVisual` had — the tag was simply absent after bevy_ggrs recreated the
/// entity — and a count catches that without knowing anything about the value.
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
    let mut found = Vec::new();
    for (type_name, restored_census) in &restored {
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
