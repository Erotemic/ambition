//! The binding resolution boundary: authored references resolve ONCE, and what
//! fails to resolve is named out loud.
//!
//! Ambition is full of cross-layer references authored as strings — an anim row
//! (`"death"`), a world-item sprite id (`"spark_blossom"`), an sfx cue, a recipe,
//! a brain, a music track. Historically each was looked up at USE time, by the
//! consumer, through a fallible map:
//!
//! ```ignore
//! let Some(row) = sheet.row_index_of(name) else { return };   // and nothing draws
//! ```
//!
//! That shape has one failure mode and it is always the same one: the reference
//! misses, the consumer degrades to silence, and the defect ships. It cost this
//! project an unreachable death animation (the sheet spelled it `death`, the
//! policy said `dead`), invisible rings, a spark blossom that was never drawn,
//! and a character that shipped as a fully transparent sprite sheet.
//!
//! # The boundary
//!
//! 1. Content declares a [`Ref<N>`] — an id in a named [`Namespace`] — never a
//!    bare `String` that a consumer will later guess at.
//! 2. A [`Resolver<N>`], built once from the ids that actually exist, turns each
//!    `Ref` into a [`Bound<N>`]. `Bound` has no public constructor, so a consumer
//!    CANNOT hold one it did not resolve.
//! 3. Whatever fails lands in a [`BindingLedger`], which closes into one
//!    [`BindingReport`] naming the namespace, the id, WHO declared it, and what
//!    ids were actually available — with a did-you-mean when one is close.
//!
//! The point is not that resolution can never fail. Content has typos; that is
//! normal. The point is that a failure is a *value someone holds*, not an early
//! `return` nobody sees.
//!
//! # Draw blind, but say so
//!
//! A non-empty report does not mean "draw nothing". Presentation keeps its
//! visible fallback (the magenta placeholder quad) so a blind run still shows
//! that something is wrong on screen. The report is the other half: the run also
//! *says* what is wrong, in a form a headless test can assert on.
//!
//! # Determinism
//!
//! `Resolver` is a sorted `Vec` and the report is sorted by
//! `(namespace, declared_by, id)`, so two runs over the same content produce a
//! byte-identical report regardless of iteration order upstream (ADR 0023).

use std::marker::PhantomData;

/// A family of ids that resolve against one another — anim rows, item sprites,
/// sfx cues. Implemented by a zero-sized marker type per family.
///
/// `NAME` is what the report prints, so it reads as a noun phrase in a sentence
/// like "unknown anim row `dead`": prefer `"anim row"` over `"AnimRow"`.
pub trait Namespace: 'static {
    /// Human-readable name of this family, used in diagnostics.
    const NAME: &'static str;
}

/// An authored, NOT-yet-resolved reference into namespace `N`.
///
/// This is what content holds. It is deliberately inert: it has no lookup method,
/// because a reference that can look itself up is a reference that can silently
/// fail to. Ask a [`Resolver`].
// The std derives would demand `N: Debug + Clone + ...` on the marker type, which
// `PhantomData<fn() -> N>` does not actually need. Hand-written impls keep
// namespace markers bare unit structs.
pub struct Ref<N: Namespace> {
    id: String,
    _namespace: PhantomData<fn() -> N>,
}

impl<N: Namespace> std::fmt::Debug for Ref<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ref<{}>({})", N::NAME, self.id)
    }
}

impl<N: Namespace> Clone for Ref<N> {
    fn clone(&self) -> Self {
        Self::new(self.id.clone())
    }
}

impl<N: Namespace> PartialEq for Ref<N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<N: Namespace> Eq for Ref<N> {}

impl<N: Namespace> std::hash::Hash for Ref<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<N: Namespace> Ref<N> {
    /// Declare a reference to `id`. Call this at the authoring seam (RON load,
    /// LDtk field, provider registration) — not at the use site.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            _namespace: PhantomData,
        }
    }

    /// The authored id, for diagnostics and round-tripping back to content.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// A reference that HAS resolved: the id exists in the namespace, and [`slot`]
/// is the position it was DECLARED at — a sheet's row index, a manifest's entry
/// index — so a consumer indexes straight into the authored data.
///
/// There is no public constructor. A consumer holding a `Bound<N>` is holding
/// proof that resolution happened, which is the whole invariant this module
/// exists to create.
pub struct Bound<N: Namespace> {
    id: String,
    slot: usize,
    _namespace: PhantomData<fn() -> N>,
}

impl<N: Namespace> std::fmt::Debug for Bound<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bound<{}>({}#{})", N::NAME, self.id, self.slot)
    }
}

impl<N: Namespace> Clone for Bound<N> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            slot: self.slot,
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> PartialEq for Bound<N> {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.id == other.id
    }
}

impl<N: Namespace> Eq for Bound<N> {}

impl<N: Namespace> std::hash::Hash for Bound<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.slot.hash(state);
        self.id.hash(state);
    }
}

impl<N: Namespace> Bound<N> {
    /// The resolved id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The position this id was declared at: the sheet row, the manifest entry.
    /// Indexing the authored collection with it needs no bounds check in spirit —
    /// a `Bound` only exists because that entry does.
    pub fn slot(&self) -> usize {
        self.slot
    }
}

/// One reference that did not resolve, carrying everything needed to fix it
/// without opening a debugger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRef {
    /// [`Namespace::NAME`] of the family searched.
    pub namespace: &'static str,
    /// The id content asked for.
    pub id: String,
    /// Who declared it — a plan row, character id, catalog entry. Free-form, but
    /// it must let a reader find the authored line.
    pub declared_by: String,
    /// Every id that WAS available, sorted. This is the half that turns a puzzle
    /// into a typo: `dead` is obviously wrong once you can see `death` beside it.
    pub available: Vec<String>,
    /// The closest available id, when one is close enough to be worth naming.
    pub did_you_mean: Option<String>,
}

impl std::fmt::Display for UnresolvedRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown {} `{}` declared by `{}`",
            self.namespace, self.id, self.declared_by
        )?;
        if let Some(suggestion) = &self.did_you_mean {
            write!(f, " — did you mean `{suggestion}`?")?;
        }
        if self.available.is_empty() {
            write!(f, " (nothing is registered in this namespace)")
        } else {
            write!(f, " (available: {})", self.available.join(", "))
        }
    }
}

/// The ids that exist in namespace `N`, and the only thing that can mint a
/// [`Bound<N>`].
///
/// Build it once, from the authority for that family: the sheet's row list, the
/// unioned art manifest, the sfx bank's cue table.
pub struct Resolver<N: Namespace> {
    /// Sorted and deduplicated, so lookup is a binary search and `available` in
    /// a report reads alphabetically.
    ids: Vec<String>,
    /// `slots[i]` is the position `ids[i]` was DECLARED at upstream — the sheet
    /// row, the manifest entry. Parallel to `ids`, so sorting for lookup does not
    /// cost the consumer its index into the authored data.
    slots: Vec<usize>,
    _namespace: PhantomData<fn() -> N>,
}

impl<N: Namespace> std::fmt::Debug for Resolver<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Resolver<{}>{:?}", N::NAME, self.ids)
    }
}

impl<N: Namespace> Clone for Resolver<N> {
    fn clone(&self) -> Self {
        Self {
            ids: self.ids.clone(),
            slots: self.slots.clone(),
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> Default for Resolver<N> {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            slots: Vec::new(),
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> FromIterator<String> for Resolver<N> {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        // Declaration order is the payload, so pair each id with its position
        // BEFORE sorting for lookup. A duplicate id keeps its first slot: content
        // that declares `idle` twice means the first one.
        let mut pairs: Vec<(String, usize)> = iter.into_iter().zip(0usize..).collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        pairs.dedup_by(|a, b| a.0 == b.0);
        let (ids, slots) = pairs.into_iter().unzip();
        Self {
            ids,
            slots,
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> Resolver<N> {
    /// Build from anything string-ish — `&str`, `String`, a map's keys.
    pub fn new<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ids.into_iter().map(Into::into).collect()
    }

    /// Build where one entry answers to SEVERAL spellings: each `(alias, slot)`
    /// resolves to the same declaration.
    ///
    /// Kinematic paths are the case — a room's paths are addressable by authored
    /// id and by display name, and content legitimately references either. Every
    /// alias appears in a report's `available` list, because "the id you used is
    /// not one of the spellings this path answers to" is the useful message.
    pub fn with_aliases<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, usize)>,
        S: Into<String>,
    {
        let mut pairs: Vec<(String, usize)> = entries
            .into_iter()
            .map(|(alias, slot)| (alias.into(), slot))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        pairs.dedup_by(|a, b| a.0 == b.0);
        let (ids, slots) = pairs.into_iter().unzip();
        Self {
            ids,
            slots,
            _namespace: PhantomData,
        }
    }

    /// Every id that exists here, sorted.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// True when nothing was ever registered. Worth distinguishing in a report:
    /// "you spelled it wrong" and "the provider never registered anything" are
    /// different bugs with different fixes.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Resolve `reference`, or describe why it failed. `declared_by` names the
    /// authored thing that carried the reference.
    pub fn resolve(
        &self,
        reference: &Ref<N>,
        declared_by: impl Into<String>,
    ) -> Result<Bound<N>, UnresolvedRef> {
        match self
            .ids
            .binary_search_by(|id| id.as_str().cmp(reference.id()))
        {
            Ok(index) => Ok(Bound {
                id: self.ids[index].clone(),
                slot: self.slots[index],
                _namespace: PhantomData,
            }),
            Err(_) => Err(UnresolvedRef {
                namespace: N::NAME,
                id: reference.id().to_owned(),
                declared_by: declared_by.into(),
                available: self.ids.clone(),
                did_you_mean: closest(reference.id(), &self.ids),
            }),
        }
    }
}

/// The closest available id, when it is close enough to be a likely typo rather
/// than a coincidence.
///
/// Two edits, but never more than half the id — so `dead`→`death` is offered
/// (substitute `d`→`t`, insert `h`: distance 2, and 2 ≤ half of 4 rounded up)
/// while `run`→`jump` is not, and a long id like `spark_blossom` only ever
/// suggests a genuine near-miss rather than the least-distant unrelated entry.
fn closest(needle: &str, haystack: &[String]) -> Option<String> {
    let budget = 2.min(needle.len().div_ceil(2));
    let typo = haystack
        .iter()
        .map(|candidate| (edit_distance(needle, candidate), candidate))
        // Ties go to the first in sorted order, so the suggestion is deterministic.
        .filter(|(distance, _)| *distance <= budget)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate.clone());
    typo.or_else(|| affix_rename(needle, haystack))
}

/// The other common way a reference goes stale: not a typo but a RENAME that
/// added or dropped a suffix — `stair` when the zone became `stair_top`,
/// `patrol` when the path became `patrol_a`. Edit distance scores those as
/// far apart, and it is right to: they are not misspellings. They are still
/// almost always the id the author meant.
///
/// Only offered when exactly ONE candidate is an extension of the needle (or the
/// needle of it), so an ambiguous stem suggests nothing rather than guessing.
fn affix_rename(needle: &str, haystack: &[String]) -> Option<String> {
    const MIN_STEM: usize = 3;
    if needle.len() < MIN_STEM {
        return None;
    }
    let mut matches = haystack.iter().filter(|candidate| {
        candidate.len() >= MIN_STEM
            && (candidate.starts_with(needle) || needle.starts_with(candidate.as_str()))
    });
    let first = matches.next()?;
    matches.next().is_none().then(|| first.clone())
}

/// Levenshtein distance, two rows. Small inputs (ids), so the straightforward
/// implementation is the right one.
fn edit_distance(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b_chars.len()).collect();
    let mut current = vec![0usize; b_chars.len() + 1];

    for (i, a_char) in a.chars().enumerate() {
        current[0] = i + 1;
        for (j, &b_char) in b_chars.iter().enumerate() {
            let substitution = previous[j] + usize::from(a_char != b_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b_chars.len()]
}

/// Accumulates unresolved references across every namespace into ONE report.
///
/// The cross-namespace part is the point. A room's construction touches anim
/// rows, item sprites, cues, and recipes; a reader chasing "why is this room
/// wrong" should get one list, not four scattered warnings from four crates with
/// four different error types.
#[derive(Debug, Default, Clone)]
pub struct BindingLedger {
    unresolved: Vec<UnresolvedRef>,
}

impl BindingLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a failure.
    pub fn record(&mut self, unresolved: UnresolvedRef) {
        self.unresolved.push(unresolved);
    }

    /// Resolve through `resolver`, recording the failure and yielding `None`
    /// rather than short-circuiting — so ONE pass reports every bad reference
    /// instead of stopping at the first and hiding the rest.
    pub fn resolve<N: Namespace>(
        &mut self,
        resolver: &Resolver<N>,
        reference: &Ref<N>,
        declared_by: impl Into<String>,
    ) -> Option<Bound<N>> {
        match resolver.resolve(reference, declared_by) {
            Ok(bound) => Some(bound),
            Err(unresolved) => {
                self.record(unresolved);
                None
            }
        }
    }

    /// Merge another ledger in — for a construction that fans out across
    /// subsystems and collects their ledgers at the boundary.
    pub fn absorb(&mut self, other: BindingLedger) {
        self.unresolved.extend(other.unresolved);
    }

    /// Close the ledger into a sorted report.
    pub fn finish(self) -> BindingReport {
        let mut unresolved = self.unresolved;
        unresolved.sort_by(|a, b| {
            a.namespace
                .cmp(b.namespace)
                .then_with(|| a.declared_by.cmp(&b.declared_by))
                .then_with(|| a.id.cmp(&b.id))
        });
        unresolved.dedup();
        BindingReport { unresolved }
    }
}

/// What did not bind, after a construction (or a whole-content sweep) finished.
///
/// Empty means every authored reference in scope found its target. That is the
/// assertion a headless test makes, and the condition a room construction
/// requires before it publishes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BindingReport {
    unresolved: Vec<UnresolvedRef>,
}

impl BindingReport {
    /// Every reference that failed, sorted by `(namespace, declared_by, id)`.
    pub fn unresolved(&self) -> &[UnresolvedRef] {
        &self.unresolved
    }

    /// Nothing failed to bind.
    pub fn is_empty(&self) -> bool {
        self.unresolved.is_empty()
    }

    /// How many references failed.
    pub fn len(&self) -> usize {
        self.unresolved.len()
    }

    /// True when any failure was in namespace `N` — for a consumer that only
    /// cares about its own family.
    pub fn has<N: Namespace>(&self) -> bool {
        self.unresolved.iter().any(|u| u.namespace == N::NAME)
    }

    /// Merge another report in, keeping the sort order.
    ///
    /// A construction fans out — the room's own families, then the content staged
    /// into it — and the whole point is that a reader gets ONE list rather than
    /// one per pass.
    pub fn absorb(&mut self, other: BindingReport) {
        self.unresolved.extend(other.unresolved);
        self.unresolved.sort_by(|a, b| {
            a.namespace
                .cmp(b.namespace)
                .then_with(|| a.declared_by.cmp(&b.declared_by))
                .then_with(|| a.id.cmp(&b.id))
        });
        self.unresolved.dedup();
    }

    /// Say what did not bind, at `error` level, tagged with `context` (the room,
    /// the visual, the provider).
    ///
    /// This is the ONE sink for "content named something that does not exist", so
    /// the message reads the same everywhere and a consumer never has to invent
    /// its own `warn!`. A visible run gets it in the console; a headless run gets
    /// it in the captured log; a test asserts on the report itself.
    ///
    /// Empty reports say nothing — silence here means every reference bound,
    /// which is the one time silence is the honest answer.
    pub fn log(&self, context: &str) {
        for unresolved in &self.unresolved {
            tracing::error!("{context}: {unresolved}");
        }
    }
}

impl std::fmt::Display for BindingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.unresolved.is_empty() {
            return write!(f, "every authored reference resolved");
        }
        writeln!(f, "{} unresolved binding(s):", self.unresolved.len())?;
        for unresolved in &self.unresolved {
            writeln!(f, "  ▢ {unresolved}")?;
        }
        Ok(())
    }
}

/// Remembers which unresolved references have already been reported, so a
/// consumer that resolves EVERY FRAME says each one once.
///
/// Presentation is the case that needs this: the item-visual sync clears and
/// rebuilds its sprites each frame, so a single missing art id would otherwise
/// emit sixty identical lines a second and bury everything else. Keep one in a
/// `Local<ReportedOnce>` beside the system that resolves.
///
/// It deliberately does NOT suppress across contexts — the same missing sprite
/// reported by two different visuals is two different facts about the content.
#[derive(Debug, Default, Clone)]
pub struct ReportedOnce {
    seen: std::collections::BTreeSet<(&'static str, String, String)>,
}

impl ReportedOnce {
    /// Log whatever in `report` has not been logged before, at `context`.
    pub fn log_new(&mut self, report: &BindingReport, context: &str) {
        for unresolved in &report.unresolved {
            let key = (
                unresolved.namespace,
                unresolved.declared_by.clone(),
                unresolved.id.clone(),
            );
            if self.seen.insert(key) {
                tracing::error!("{context}: {unresolved}");
            }
        }
    }

    /// How many distinct references this has reported — the count a diagnostic
    /// overlay or a test can read without scraping the log.
    pub fn reported(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The anim rows of a sprite sheet.
    struct AnimRow;
    impl Namespace for AnimRow {
        const NAME: &'static str = "anim row";
    }

    /// The world-item art ids a provider registered.
    struct ItemSprite;
    impl Namespace for ItemSprite {
        const NAME: &'static str = "item sprite";
    }

    /// The load-bearing property: a construction that touches several namespaces
    /// produces ONE report, and each entry carries enough to fix the content
    /// without reading engine source — who declared it, and what existed.
    ///
    /// The two failures here are the real ones this boundary was built for: the
    /// anim row Mary-O's death was authored as (`dead`, sheet says `death`), and
    /// a world item whose art id no provider ever registered.
    #[test]
    fn unresolved_refs_land_in_one_report() {
        let rows: Resolver<AnimRow> = Resolver::new(["idle", "walk", "run", "jump", "death"]);
        let sprites: Resolver<ItemSprite> = Resolver::new(["milk", "ring"]);

        let mut ledger = BindingLedger::new();
        // Resolution keeps going after a failure, so one pass finds both.
        assert!(ledger
            .resolve(&rows, &Ref::new("walk"), "mary_o/anim")
            .is_some());
        assert!(ledger
            .resolve(&rows, &Ref::new("dead"), "mary_o/anim")
            .is_none());
        assert!(ledger
            .resolve(&sprites, &Ref::new("spark_blossom"), "level_1_2/item#12")
            .is_none());

        let report = ledger.finish();
        assert_eq!(report.len(), 2, "one report, both namespaces:\n{report}");
        assert!(report.has::<AnimRow>() && report.has::<ItemSprite>());

        // Sorted by namespace, so `anim row` precedes `item sprite`.
        let row = &report.unresolved()[0];
        assert_eq!(row.namespace, "anim row");
        assert_eq!(row.declared_by, "mary_o/anim", "the report names WHO");
        assert_eq!(
            row.did_you_mean.as_deref(),
            Some("death"),
            "`dead` is two edits from `death` — the real Mary-O typo"
        );
        assert!(
            row.available.contains(&"death".to_owned()),
            "the report names what WAS available"
        );

        let sprite = &report.unresolved()[1];
        assert_eq!(sprite.namespace, "item sprite");
        assert_eq!(sprite.id, "spark_blossom");
        assert_eq!(
            sprite.did_you_mean, None,
            "nothing registered is close; do not invent a suggestion"
        );
    }

    /// `slot()` is the position the id was DECLARED at, not its position in the
    /// sorted lookup table — that is what lets a consumer index straight into the
    /// authored data (a sheet's `rows[slot]`) after resolving by name.
    #[test]
    fn a_binding_carries_its_declaration_slot() {
        // Deliberately not alphabetical: sorting for lookup must not disturb it.
        let rows: Resolver<AnimRow> = Resolver::new(["idle", "walk", "death"]);
        assert_eq!(rows.ids(), ["death", "idle", "walk"], "sorted for lookup");

        for (declared_at, id) in ["idle", "walk", "death"].iter().enumerate() {
            let bound = rows.resolve(&Ref::new(*id), "sheet").expect("resolves");
            assert_eq!(bound.slot(), declared_at, "{id} keeps its authored row");
        }
    }

    /// An empty namespace says so, rather than offering a bare "not found" that
    /// reads like a typo. "The provider registered nothing" is a different bug.
    #[test]
    fn an_empty_namespace_reports_that_it_is_empty() {
        let sprites: Resolver<ItemSprite> = Resolver::default();
        let unresolved = sprites
            .resolve(&Ref::new("milk"), "level_1_1/item#3")
            .expect_err("nothing is registered");
        assert!(sprites.is_empty());
        assert!(
            unresolved.to_string().contains("nothing is registered"),
            "{unresolved}"
        );
    }
}
