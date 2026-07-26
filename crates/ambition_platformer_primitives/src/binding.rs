//! The binding resolution boundary: authored references resolve through the
//! authority that knows, and what fails to resolve is named out loud.
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
//! policy said `dead`), invisible rings, and a character that shipped as a fully
//! transparent sprite sheet.
//!
//! The spark blossom is NOT on that list, though it was cited here for a while.
//! Its id was registered correctly; the PNG behind it did not exist. That is a
//! different bug, and this module does not catch it — see "What this is NOT".
//!
//! # The boundary
//!
//! 1. A [`Resolver<N>`], built from the ids that actually exist, is the only
//!    thing that can mint a [`Bound<N>`]. `Bound` has no public constructor, so
//!    a consumer CANNOT hold one it did not resolve.
//! 2. Whatever fails lands in a [`BindingLedger`], which closes into a
//!    [`BindingReport`] naming the namespace, the id, WHO declared it, and what
//!    ids were actually available — with a did-you-mean when one is close.
//! 3. An id declared twice is reported too ([`AmbiguousRef`]), because "resolves
//!    to the first of two" is a silence of its own.
//!
//! The point is not that resolution can never fail. Content has typos; that is
//! normal. The point is that a failure is a *value someone holds*, not an early
//! `return` nobody sees.
//!
//! # What this is NOT
//!
//! Read this before treating a resolved reference as a guarantee.
//!
//! - **Most content still stores `String`.** [`Ref<N>`] is the authoring-seam
//!   vocabulary, and the sweeps construct one at the boundary rather than the
//!   authored types carrying it. Content that holds a `Ref` cannot be
//!   *addressed* without a resolver; content that holds a `String` merely tends
//!   to be resolved through one, by convention.
//! - **`Bound<N>` proves that SOME resolver had the id, not WHICH.** The
//!   namespace marker names a family (`anim row`), and two sprite sheets are two
//!   authorities in the same family. `sheet_b.row(&sheet_a_bound)` type-checks;
//!   it is caught by a runtime assertion in the consumer, not by construction.
//! - **Resolution is not always once.** Room construction resolves once, into a
//!   report carried on the plan. Presentation resolves per frame, because what
//!   it draws changes per frame — see [`ReportedOnce`], which makes that cheap
//!   AND quiet after the first complaint rather than pretending it is a
//!   construction-time question.
//! - **There is no single global report.** A report is per-pass: one for a
//!   room's construction, one per presentation consumer, and audio keeps its own
//!   vocabulary entirely. `absorb` unifies the passes that CAN be unified.
//! - **A non-empty report does not block publication.** A room with an
//!   unresolvable patrol path still constructs, on purpose: the alternative is a
//!   blind run that shows nothing at all. The report is what makes the
//!   degradation loud, not what prevents it. Where a defect genuinely should
//!   refuse — a ground item naming an unregistered held item — that refusal
//!   lives in construction and raises an [`UnresolvedRef`] of its own.
//! - **An id that binds says nothing about whether an ASSET exists.** A resolver
//!   proves content agrees with content. The Mary-O spark blossom was registered
//!   correctly and pointed at a PNG no generator produced — a failure this
//!   module cannot see. Whoever loads the file has to check the load, which is
//!   what `render::item_visuals::report_unloadable_item_art` is for.
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
use std::sync::Arc;

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
/// Deliberately inert: it has no lookup method, because a reference that can
/// look itself up is a reference that can silently fail to. Ask a [`Resolver`].
///
/// Where the authored type is still a `String` — which is most of them — the
/// sweep constructs one of these at the boundary. That is weaker than content
/// holding it directly, and the difference is real: a `String` field can be read
/// by a consumer that never asks anyone whether the id exists.
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
    /// Shared with the [`Resolver`] that minted it: presentation resolves every
    /// visible item every frame, and a `String` here meant a heap allocation per
    /// item per frame to carry a name the caller usually only reads.
    id: Arc<str>,
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
    ids: Vec<Arc<str>>,
    /// `slots[i]` is the position `ids[i]` was DECLARED at upstream — the sheet
    /// row, the manifest entry. Parallel to `ids`, so sorting for lookup does not
    /// cost the consumer its index into the authored data.
    slots: Vec<usize>,
    /// Ids declared more than once, in sorted order. Resolution picks the first
    /// declaration; keeping the collision means a consumer can SAY that the
    /// content is ambiguous instead of quietly picking for the author.
    duplicates: Vec<Arc<str>>,
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
            duplicates: self.duplicates.clone(),
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> Default for Resolver<N> {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            slots: Vec::new(),
            duplicates: Vec::new(),
            _namespace: PhantomData,
        }
    }
}

impl<N: Namespace> FromIterator<String> for Resolver<N> {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self::from_declarations(iter.into_iter().map(Arc::<str>::from).zip(0usize..))
    }
}

impl<N: Namespace> Resolver<N> {
    /// The one place declarations become a lookup table.
    ///
    /// Declaration order is the payload, so each id is paired with its position
    /// BEFORE sorting. A duplicate id keeps its FIRST slot — content that
    /// declares `idle` twice means the first one — and is remembered in
    /// [`Self::duplicates`] so the choice can be reported rather than assumed.
    fn from_declarations(entries: impl IntoIterator<Item = (Arc<str>, usize)>) -> Self {
        let mut pairs: Vec<(Arc<str>, usize)> = entries.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut duplicates: Vec<Arc<str>> = pairs
            .windows(2)
            // Repeating the same spelling for the SAME declaration is benign:
            // a path whose authored id equals its display name contributes the
            // alias twice, but there is still only one target. Ambiguity begins
            // only when one spelling reaches two distinct slots.
            .filter(|pair| pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1)
            .map(|pair| pair[0].0.clone())
            .collect();
        duplicates.dedup();
        pairs.dedup_by(|a, b| a.0 == b.0);
        let (ids, slots) = pairs.into_iter().unzip();
        Self {
            ids,
            slots,
            duplicates,
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
        Self::from_declarations(
            entries
                .into_iter()
                .map(|(alias, slot)| (Arc::from(alias.into()), slot)),
        )
    }

    /// Every id that exists here, sorted.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.ids.iter().map(Arc::as_ref)
    }

    /// Every id beside the slot it was DECLARED at.
    ///
    /// [`Self::ids`] is sorted for lookup, so zipping it against the authored
    /// collection pairs the wrong id with the wrong entry. Anything walking both
    /// sides wants this.
    pub fn declarations(&self) -> impl ExactSizeIterator<Item = (&str, usize)> {
        self.ids
            .iter()
            .map(Arc::as_ref)
            .zip(self.slots.iter().copied())
    }

    /// Ids that point at more than one declaration slot, sorted. Non-empty means
    /// the content is ambiguous: two sheet rows called `idle`, two distinct
    /// paths answering to the same alias. Repeating one alias for the same slot
    /// is harmless and is not reported. Resolution still succeeds — it takes
    /// the first declaration — so this is the only way anyone learns the second
    /// one is unreachable.
    pub fn duplicates(&self) -> impl ExactSizeIterator<Item = &str> {
        self.duplicates.iter().map(Arc::as_ref)
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
        self.resolve_str(reference.id(), || declared_by.into())
    }

    /// Resolve a borrowed id, building the declarer string ONLY if it fails.
    ///
    /// The lazy declarer is not a micro-optimization for its own sake: the item
    /// visuals resolve every id every frame, and an eager
    /// `format!("world item `{}`", row)` would allocate per item per frame to
    /// describe a failure that almost never happens.
    pub fn resolve_str(
        &self,
        id: &str,
        declared_by: impl FnOnce() -> String,
    ) -> Result<Bound<N>, UnresolvedRef> {
        self.bind(id).ok_or_else(|| self.explain(id, declared_by()))
    }

    /// Bind `id` if it exists, saying nothing if it does not.
    ///
    /// A binary search and a refcount bump — no allocation at all. This is the
    /// half a per-frame consumer wants, because [`Self::explain`] is where the
    /// cost lives and a consumer that has already reported a permanently missing
    /// id must not keep paying for the explanation nobody will read. Anything
    /// resolving ONCE should call [`Self::resolve`] instead and let the report
    /// carry the failure.
    pub fn bind(&self, id: &str) -> Option<Bound<N>> {
        let index = self
            .ids
            .binary_search_by(|known| known.as_ref().cmp(id))
            .ok()?;
        Some(Bound {
            id: self.ids[index].clone(),
            slot: self.slots[index],
            _namespace: PhantomData,
        })
    }

    /// Everything a reader needs to fix `id` — including a clone of every
    /// available id and a did-you-mean search over all of them.
    ///
    /// Deliberately separate from [`Self::bind`]: this is O(namespace) work with
    /// several allocations, worth every cent the first time and nothing at all
    /// the sixtieth time in a second. Call it once per distinct failure.
    pub fn explain(&self, id: &str, declared_by: impl Into<String>) -> UnresolvedRef {
        let available: Vec<String> = self.ids.iter().map(|id| id.as_ref().to_owned()).collect();
        UnresolvedRef {
            namespace: N::NAME,
            id: id.to_owned(),
            declared_by: declared_by.into(),
            did_you_mean: closest(id, &available),
            available,
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

/// An id declared more than once in one namespace.
///
/// Distinct from [`UnresolvedRef`] because the reference DOES resolve: the
/// first declaration wins, quietly, and the author never learns that the second
/// one they wrote is dead. That is the same silence this module exists to break,
/// so it is reported — but it does not fail a binding, and does not stop a room
/// from being published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousRef {
    /// [`Namespace::NAME`] of the family the collision is in.
    pub namespace: &'static str,
    /// The id declared more than once.
    pub id: String,
    /// Who owns the declarations — the sheet, the room, the manifest.
    pub declared_by: String,
}

impl std::fmt::Display for AmbiguousRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ambiguous {} `{}` in `{}` — declared more than once; the first \
             declaration wins and the rest are unreachable",
            self.namespace, self.id, self.declared_by
        )
    }
}

/// Accumulates the failures of ONE pass, across whatever namespaces that pass
/// touched, into one report.
///
/// The cross-namespace part is the point: a room's construction touches paths
/// and archetypes, and a reader chasing "why is this room wrong" should get one
/// list rather than scattered warnings from several crates with several error
/// types.
///
/// It is one report per PASS, not one per run. Presentation and audio have
/// their own; [`BindingReport::absorb`] joins the passes that belong together.
#[derive(Debug, Default, Clone)]
pub struct BindingLedger {
    unresolved: Vec<UnresolvedRef>,
    ambiguous: Vec<AmbiguousRef>,
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

    /// Record every id `resolver` was given twice, attributed to whoever owns
    /// the declarations. Costs one pass over a list that is empty in healthy
    /// content, so it is worth calling wherever a resolver is built from
    /// authored data.
    pub fn note_duplicates<N: Namespace>(
        &mut self,
        resolver: &Resolver<N>,
        declared_by: impl Into<String>,
    ) {
        if resolver.duplicates.is_empty() {
            return;
        }
        let declared_by = declared_by.into();
        self.ambiguous
            .extend(resolver.duplicates().map(|id| AmbiguousRef {
                namespace: N::NAME,
                id: id.to_owned(),
                declared_by: declared_by.clone(),
            }));
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
        let mut ambiguous = self.ambiguous;
        ambiguous.sort_by(|a, b| {
            a.namespace
                .cmp(b.namespace)
                .then_with(|| a.declared_by.cmp(&b.declared_by))
                .then_with(|| a.id.cmp(&b.id))
        });
        ambiguous.dedup();
        BindingReport {
            unresolved,
            ambiguous,
        }
    }
}

/// What did not bind, after a construction (or a whole-content sweep) finished.
///
/// Empty means every authored reference in scope found its target. That is the
/// assertion a headless test makes.
///
/// It is NOT a precondition for publishing a room. A room whose patrol path does
/// not resolve still constructs — the enemy goes passive, which is what it did
/// before this module existed, and a blind run showing a passive enemy beats a
/// blind run showing nothing. The report changes whether the degradation is
/// SAID, not whether it happens. Refusal is a separate decision, made by
/// whichever construction rule considers the defect fatal.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BindingReport {
    unresolved: Vec<UnresolvedRef>,
    ambiguous: Vec<AmbiguousRef>,
}

impl BindingReport {
    /// Every reference that failed, sorted by `(namespace, declared_by, id)`.
    pub fn unresolved(&self) -> &[UnresolvedRef] {
        &self.unresolved
    }

    /// Every id declared twice, sorted the same way. Reported, not fatal: these
    /// resolved, just not necessarily to the declaration the author meant.
    pub fn ambiguous(&self) -> &[AmbiguousRef] {
        &self.ambiguous
    }

    /// Nothing failed to bind.
    ///
    /// Ambiguity is deliberately not counted here. An id declared twice still
    /// resolves, so a room whose sheet has two `idle` rows is drawable and
    /// should be published — with a complaint in the log, which is what
    /// [`Self::log`] is for.
    pub fn is_empty(&self) -> bool {
        self.unresolved.is_empty()
    }

    /// How many references failed.
    pub fn len(&self) -> usize {
        self.unresolved.len()
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
        self.ambiguous.extend(other.ambiguous);
        self.ambiguous.sort_by(|a, b| {
            a.namespace
                .cmp(b.namespace)
                .then_with(|| a.declared_by.cmp(&b.declared_by))
                .then_with(|| a.id.cmp(&b.id))
        });
        self.ambiguous.dedup();
    }

    /// Say what did not bind, at `error` level, tagged with `context` (the room,
    /// the visual, the provider).
    ///
    /// Built on [`log_unresolved`], which is the one place the wording lives, so
    /// a consumer never invents its own `warn!` — including the per-frame ones
    /// that report a single failure at a time and never hold a whole report. A
    /// visible run gets it in the console; a headless run gets it in the captured
    /// log; a test asserts on the report itself.
    ///
    /// It is not the only place the ENGINE reports a missing reference. Audio
    /// speaks its own vocabulary (a cue is not a slot in a namespace), and
    /// construction refusals carry an [`UnresolvedRef`] inside a hard error
    /// rather than a report.
    ///
    /// Empty reports say nothing — silence here means every reference bound,
    /// which is the one time silence is the honest answer.
    pub fn log(&self, context: &str) {
        for unresolved in &self.unresolved {
            log_unresolved(context, unresolved);
        }
        // A warning, not an error: the content still works, it just does not
        // say what its author thinks it says.
        for ambiguous in &self.ambiguous {
            tracing::warn!("{context}: {ambiguous}");
        }
    }
}

/// Say that one reference did not bind, at `error` level, tagged with `context`.
///
/// The single sink [`BindingReport::log`] is built from. A per-frame consumer
/// that gates on [`ReportedOnce`] reports one failure at a time and must not
/// invent its own wording, so it calls this rather than collecting a whole
/// report it would immediately take apart again.
pub fn log_unresolved(context: &str, unresolved: &UnresolvedRef) {
    tracing::error!("{context}: {unresolved}");
}

impl std::fmt::Display for BindingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.unresolved.is_empty() && self.ambiguous.is_empty() {
            return write!(f, "every authored reference resolved");
        }
        if !self.unresolved.is_empty() {
            writeln!(f, "{} unresolved binding(s):", self.unresolved.len())?;
            for unresolved in &self.unresolved {
                writeln!(f, "  ▢ {unresolved}")?;
            }
        }
        if !self.ambiguous.is_empty() {
            writeln!(f, "{} ambiguous declaration(s):", self.ambiguous.len())?;
            for ambiguous in &self.ambiguous {
                writeln!(f, "  ▢ {ambiguous}")?;
            }
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
///
/// # Ask it BEFORE doing the work
///
/// [`Self::first_sight`] is a set probe. Explaining a failure — cloning every
/// available id, running a did-you-mean over all of them — is the expensive
/// part, and a permanently missing id would otherwise pay it every frame
/// forever just to have the log line thrown away. Gate first, explain second.
///
/// # It goes stale, so clear it
///
/// The memory is a `Local`, and it outlives the content it describes: a
/// manifest replaced by a room reload, a provider swapped by a shell
/// transition. A failure suppressed because "we already said that" about
/// content that no longer exists is a lie. Consumers call [`Self::clear`] when
/// the resource they resolve against changes.
#[derive(Debug, Default, Clone)]
pub struct ReportedOnce {
    /// A `Vec` rather than a set on purpose. What lands here is one entry per
    /// DISTINCT content defect, which is nought to a handful; a linear scan of
    /// borrowed `&str`s beats a set whose keys can only be probed by building
    /// them, and the repeat path — the one that runs every frame — then costs
    /// no allocation at all.
    seen: Vec<(&'static str, String, String)>,
}

impl ReportedOnce {
    /// True the FIRST time this exact failure is seen, and false after — so the
    /// caller can skip building a diagnostic nobody will read.
    pub fn first_sight(&mut self, namespace: &'static str, declared_by: &str, id: &str) -> bool {
        if self.seen.iter().any(|(seen_ns, seen_by, seen_id)| {
            *seen_ns == namespace && seen_by == declared_by && seen_id == id
        }) {
            return false;
        }
        self.seen
            .push((namespace, declared_by.to_owned(), id.to_owned()));
        true
    }

    /// Forget everything, because the content this described was replaced.
    pub fn clear(&mut self) {
        self.seen.clear();
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
        assert!(
            rows.ids().eq(["death", "idle", "walk"]),
            "sorted for lookup"
        );
        // The pairing anything walking both sides needs: sorted id, authored
        // slot. Zipping `ids()` against the authored collection would pair
        // `death` with row 0.
        assert!(
            rows.declarations()
                .eq([("death", 2), ("idle", 0), ("walk", 1)]),
            "each id keeps the slot it was declared at"
        );

        for (declared_at, id) in ["idle", "walk", "death"].iter().enumerate() {
            let bound = rows.resolve(&Ref::new(*id), "sheet").expect("resolves");
            assert_eq!(bound.slot(), declared_at, "{id} keeps its authored row");
        }
    }

    #[test]
    fn duplicate_alias_means_distinct_targets_not_repeated_input() {
        let one_target: Resolver<AnimRow> = Resolver::with_aliases([("idle", 0), ("idle", 0)]);
        assert!(
            one_target.duplicates().next().is_none(),
            "one declaration may contribute the same alias twice"
        );

        let two_targets: Resolver<AnimRow> = Resolver::with_aliases([("idle", 0), ("idle", 1)]);
        assert_eq!(two_targets.duplicates().collect::<Vec<_>>(), vec!["idle"]);
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
