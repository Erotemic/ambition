//! What a rollback registration IS, and the sentence that describes it.
//!
//! ⛔⛤ **THESE LIVE HERE BECAUSE THE TRAIT DOES.** `RollbackRegistrar` is
//! declared in this crate's `snapshot` module, and the two roads that implement
//! it -- the metadata recorder in `ambition_platformer2d_runtime` and the
//! installing registrar in `ambition_platformer2d_rollback_ggrs` -- each spelled
//! every method's `RollbackEntryKind` at its own call site. ROLLBACK-KIND-SPELLING
//! asks that a method's kind be named where the method is DECLARED, and a default
//! trait body cannot name a type that lives in a crate ABOVE the trait.
//!
//! ⚠ The `detail` sentences move for the same reason and in the same step. A
//! default body that names the kind but not the sentence closes half the row and
//! reopens the other half one crate away.
//!
//! ⛔ THE RELOCATION IS BYTE-EXACT BY CONSTRUCTION, AND THERE IS AN ORACLE FOR
//! IT: `canonical_name()` is unchanged, `schema_dump()` emits the same four
//! columns, and `compute_schema_fingerprint` hashes that dump including `detail`.
//! So `rollback_schema_baseline` passing with a 0-line diff IS the proof, not a
//! corroboration. A peer measured its sensitivity: pluralising ONE word in
//! `detail::MESSAGE_CLEAR` reddens it with 166 diff lines.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RollbackEntryKind {
    ComponentCanonical,
    ComponentCloneCursor,
    ComponentCloneResolved,
    ComponentClone,
    ComponentCloneCanonicalChecksum,
    ComponentCloneCustomChecksum,
    /// Canonical snapshot, but the CHECKSUM is a stated projection of the value
    /// rather than the whole value — the component half of
    /// `ResourceCanonicalCustomChecksum`.
    ///
    /// ⛔⛤ THE FAMILY WAS ASYMMETRIC AND NOBODY DECIDED IT. Resources had both
    /// canonical-with-projection kinds; components had only the clone-strategy
    /// one, so a component snapshotting through its own canonical codec could
    /// not state a projection at all. `TransactionId` and `SimId` are both in
    /// that position, and `SimId` is the type both provenance defects in this
    /// campaign travelled through.
    ComponentCanonicalCustomChecksum,
    ResourceCanonical,
    /// Canonical snapshot, but the CHECKSUM is a stated projection of the
    /// value rather than the whole value. The kind is what tells a guard
    /// that the two differ: `ResourceCanonical` means "every field is
    /// compared between peers", and that claim is false here.
    ResourceCanonicalCustomChecksum,
    ResourceCloneCursor,
    ResourceClone,
    ResourceCloneCustomChecksum,
    MessageClear,
    /// A message channel cleared on rollback that feeds an INSTRUMENT, not the
    /// simulation — the causal recorder's channels are the only members today.
    ///
    /// ⛔⛤ **THIS EXISTS BECAUSE A DEBUGGING FEATURE WAS CHANGING THE PEER
    /// IDENTITY.** Building the same harness with and without
    /// `--features causal` gave 494 vs 497 schema rows and two different
    /// `schema_fingerprint()` values, for simulations that are mechanically
    /// identical: these rows carry no value of their own and the channels feed a
    /// recorder, so both peers compute the same snapshots and the same
    /// checksums — and would then refuse to play each other.
    ///
    /// ⚠ IT IS NOT A SECOND SPELLING OF `MessageClear`. A rewind clears both the
    /// same way; what differs is whether the channel is part of the schema two
    /// peers negotiate, and `MessageClear` cannot answer that because ordinary
    /// message channels ARE part of it.
    ///
    /// ⇒ The repository had already decided this, in
    /// `rollback_schema_baseline.rs`, which filtered these rows out of its
    /// comparison by NAME PREFIX with the reason written beside it. A decision
    /// stated in a test and not in the authority is a decision the authority
    /// does not make — and that filter was also what hid the defect, by keeping
    /// the lane green in both configurations.
    MessageClearInstrument,
    EntityMapping,
    ResourceEntityMapping,
    RequiredRollback,
    Derived,
    DynamicAnchor,
}

impl RollbackEntryKind {
    /// Does this registration contribute to the checksum TWO PEERS COMPARE?
    ///
    /// ⛔⛤ **THIS LIVED IN A TEST AS A LIST OF STRINGS AND HID 25 OF 29
    /// REGISTRATIONS.** `id_peer_audit.rs` kept its own `CHECKSUMMED_KINDS`
    /// naming six variants; every `*CustomChecksum` kind was absent, so the
    /// three checkpoint resources — which write a raw `SessionScopeId` into
    /// their projection — were never examined by a guard whose whole subject is
    /// host-local identity reaching a peer comparison.
    ///
    /// ⇒ The question belongs HERE, where the variant is added. A new kind
    /// cannot be written without answering it.
    ///
    /// ⚠ **"FEEDS" IS NOT "IS COMPARED WHOLE".** A custom-checksum kind answers
    /// TRUE and may still compare only part of the value — that is the point of
    /// it. The kind cannot say WHICH fields; only the projection's own test can.
    pub fn feeds_peer_checksum(self) -> bool {
        match self {
            Self::ComponentCanonical
            | Self::ComponentCloneCursor
            | Self::ComponentCloneResolved
            | Self::ComponentCloneCanonicalChecksum
            | Self::ComponentCloneCustomChecksum
            | Self::ComponentCanonicalCustomChecksum
            | Self::ResourceCanonical
            | Self::ResourceCanonicalCustomChecksum
            | Self::ResourceCloneCursor
            | Self::ResourceCloneCustomChecksum => true,
            // Snapshotted but not hashed: a rewind restores them, no peer reads them.
            Self::ComponentClone
            | Self::ResourceClone
            // These carry no value of their own.
            | Self::MessageClear
            | Self::MessageClearInstrument
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::Derived
            | Self::DynamicAnchor => false,
        }
    }

    /// Does this registration compare a REVIEWED PROJECTION rather than the
    /// whole value?
    ///
    /// ⛔⛤ **THE THIRD STRING LIST THIS ENUM HAS ABSORBED, AND IT HID A GREEN
    /// EXEMPTION.** `id_peer_audit.rs` kept `PROJECTED_CHECKSUM_KINDS` — and it
    /// omitted `ComponentCanonicalCustomChecksum` from the day that variant was
    /// added. Worse, the audit's staleness rule asked only whether a recorded
    /// divergence still FEEDS the checksum: a type correctly migrated from a
    /// whole-value comparison to a reviewed projection still feeds it, so its
    /// exemption stayed green after the leak it recorded was closed. An
    /// exception outliving its defect is a hole with a comment over it.
    ///
    /// ⇒ The question belongs where the variant is written, beside
    /// [`Self::feeds_peer_checksum`]. Answering TRUE is a claim that a projection
    /// EXISTS, never a claim about what it compares — the kind cannot know that,
    /// and the projection's own value-level arm is what holds it.
    pub fn uses_peer_projection(self) -> bool {
        match self {
            Self::ComponentCloneCustomChecksum
            | Self::ComponentCanonicalCustomChecksum
            | Self::ResourceCanonicalCustomChecksum
            | Self::ResourceCloneCustomChecksum => true,
            // Compared WHOLE: every field reaches the peer checksum.
            Self::ComponentCanonical
            | Self::ComponentCloneCursor
            | Self::ComponentCloneResolved
            | Self::ComponentCloneCanonicalChecksum
            | Self::ResourceCanonical
            | Self::ResourceCloneCursor
            // Not compared at all.
            | Self::ComponentClone
            | Self::ResourceClone
            | Self::MessageClear
            | Self::MessageClearInstrument
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::Derived
            | Self::DynamicAnchor => false,
        }
    }

    /// Whether this registration carries or reconstructs a value that rollback
    /// localization must observe. `Derived` counts because its reconstruction
    /// contract must also be checked across a resimulation boundary; message-clear,
    /// remapping helpers, required markers, and dynamic anchors do not carry values.
    pub fn carries_state(self) -> bool {
        match self {
            Self::ComponentCanonical
            | Self::ComponentCloneCursor
            | Self::ComponentCloneResolved
            | Self::ComponentClone
            | Self::ComponentCloneCanonicalChecksum
            | Self::ComponentCloneCustomChecksum
            | Self::ComponentCanonicalCustomChecksum
            | Self::ResourceCanonical
            | Self::ResourceCanonicalCustomChecksum
            | Self::ResourceCloneCursor
            | Self::ResourceClone
            | Self::ResourceCloneCustomChecksum
            | Self::Derived => true,
            Self::MessageClear
            | Self::MessageClearInstrument
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::DynamicAnchor => false,
        }
    }

    /// Is this registration part of the schema identity two peers negotiate?
    ///
    /// ⛔⛤ **ASKED HERE BECAUSE THE ONLY OTHER PLACE IT WAS ASKED WAS A NAME
    /// PREFIX IN A TEST.** `rollback_schema_baseline.rs` filtered
    /// `message.causal_*` from both sides of its comparison, which is the right
    /// decision recorded in a place that cannot enforce it: the fingerprint is
    /// computed from `schema_dump()`, which had no such rule, so the instrument
    /// moved the identity while the lane stayed green in both configurations.
    ///
    /// ⚠ ANSWERING FALSE IS A CLAIM THAT A PEER CANNOT OBSERVE THIS
    /// REGISTRATION AT ALL — not that it is unhashed, which
    /// [`Self::feeds_peer_checksum`] already covers, and not that it carries no
    /// value, which [`Self::carries_state`] covers. An unhashed row still
    /// changes what a rewind restores; a row outside the schema does not exist
    /// as far as the other peer is concerned.
    pub fn in_peer_schema_identity(self) -> bool {
        match self {
            // The instrument is the only thing a peer cannot observe: its
            // channels are compiled in by a local feature, cleared like any
            // other message, and read by nothing the simulation consults.
            Self::MessageClearInstrument => false,
            Self::ComponentCanonical
            | Self::ComponentCloneCursor
            | Self::ComponentCloneResolved
            | Self::ComponentClone
            | Self::ComponentCloneCanonicalChecksum
            | Self::ComponentCloneCustomChecksum
            | Self::ComponentCanonicalCustomChecksum
            | Self::ResourceCanonical
            | Self::ResourceCanonicalCustomChecksum
            | Self::ResourceCloneCursor
            | Self::ResourceClone
            | Self::ResourceCloneCustomChecksum
            | Self::MessageClear
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::Derived
            | Self::DynamicAnchor => true,
        }
    }

    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::ComponentCanonical => "component-canonical",
            Self::ComponentCloneCursor => "component-clone-cursor",
            Self::ComponentCloneResolved => "component-clone-resolved",
            Self::ComponentClone => "component-clone",
            Self::ComponentCloneCanonicalChecksum => "component-clone-canonical-checksum",
            Self::ComponentCloneCustomChecksum => "component-clone-custom-checksum",
            Self::ComponentCanonicalCustomChecksum => "component-canonical-custom-checksum",
            Self::ResourceCanonical => "resource-canonical",
            Self::ResourceCanonicalCustomChecksum => "resource-canonical-custom-checksum",
            Self::ResourceCloneCursor => "resource-clone-cursor",
            Self::ResourceClone => "resource-clone",
            Self::ResourceCloneCustomChecksum => "resource-clone-custom-checksum",
            Self::MessageClear => "message-clear",
            Self::MessageClearInstrument => "message-clear-instrument",
            Self::EntityMapping => "entity-mapping",
            Self::ResourceEntityMapping => "resource-entity-mapping",
            Self::RequiredRollback => "required-rollback",
            Self::Derived => "derived",
            Self::DynamicAnchor => "dynamic-anchor",
        }
    }
}

/// The `detail` sentence each registration road records.
///
/// ⛔⛤ **EIGHTEEN CALL SITES SPELLED FIFTEEN SENTENCES TWICE, IN TWO CRATES,
/// AND NOTHING COMPARED THEM.** `SchemaRollbackRegistrar` (the metadata
/// recorder) and `rollback_ggrs`'s installing registrar each wrote their own
/// copy of every string. They agreed — measured 2026-09-16, byte for byte — but
/// only because nobody had reworded one. A drift in either copy would move
/// `RollbackRegistry::schema_dump` (unlinked on purpose: it lives in
/// `ambition_platformer2d_runtime`, which sits ABOVE this crate and is not a
/// dependency of it), and the two roads would name the same
/// registration differently depending on which registrar ran. (Since v303 the
/// schema fingerprint hashes each road's `mechanism` token, not its sentence.)
///
/// ⇒ One sentence, one owner. Both registrars reference these; neither spells
/// one. The `rollback_schema_baseline` test is the proof the collapse was
/// faithful: this commit leaves the dump byte-identical, so the baseline does
/// not move and no schema version is owed.
///
/// ⚠ THE SENTENCES THEMSELVES ARE NOT ALL VERIFIED, and centralising them does
/// not make them so — see `Q122`. Collapsing the copies is what made
/// [`detail::CLONE_UNHASHED`] a one-place edit when its coverage claim came
/// out.
pub mod detail {
    pub const CANONICAL_IDENTICAL_CHECKSUM: &str =
        "bevy_ggrs canonical codec snapshot + identical canonical checksum projection";

    pub const CANONICAL_PRESENCE_AWARE_CHECKSUM: &str =
        "bevy_ggrs canonical codec snapshot + presence-aware canonical checksum projection";

    pub const CLONE_CURSOR_CHECKSUM: &str =
        "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection";

    pub const CLONE_RESOLVED_CHECKSUM: &str =
        "bevy_ggrs clone snapshot + canonical authored-reference checksum projection";

    pub const CLONE_CANONICAL_CHECKSUM_REMAPPED: &str =
        "bevy_ggrs clone snapshot + canonical checksum; exact Entity/reference values are remapped after load";

    /// ⛔⛤ THIS SENTENCE USED TO CLAIM COVERAGE IT COULD NOT ESTABLISH. It read
    /// *"state checksum supplied by another authoritative projection"* on 99
    /// rows, emitted by `rollback_component_clone` / `rollback_resource_clone`,
    /// whose only bound is `T: Clone`. Whether some OTHER registration projects
    /// a type's state is a property of the type; the snapshot strategy cannot
    /// know it. It now states what IS true by construction, which is
    /// `RollbackEntryKind::feeds_peer_checksum() == false`.
    pub const CLONE_UNHASHED: &str =
        "bevy_ggrs clone snapshot; not in the session checksum";

    pub const CLONE_ENTITY_REF_REMAPPED: &str =
        "bevy_ggrs clone snapshot; entity handle remapped, probed through the target's stable sim identity";

    pub const CLONE_ENTITY_SET_REMAPPED: &str =
        "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities";

    pub const CLONE_ENTITY_MAP_REMAPPED: &str =
        "bevy_ggrs clone snapshot; keyed entity MAP remapped, probed with each key folded against its target's stable sim identity";

    pub const CLONE_PROBED_FOR_LOCALIZATION: &str =
        "bevy_ggrs clone snapshot; value-probed for localization, not in the session checksum";

    pub const CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED: &str =
        "bevy_ggrs clone snapshot; entity SET remapped and probed through the targets' stable sim identities, mixed with a projection of the value's non-entity fields";

    pub const ENTITY_MAPPING: &str =
        "bevy_ggrs LoadWorld entity-reference remapping";

    pub const RESOURCE_ENTITY_MAPPING: &str =
        "bevy_ggrs LoadWorld resource entity-reference remapping";

    pub const REQUIRED_ROLLBACK: &str =
        "component presence automatically installs bevy_ggrs::Rollback";

    pub const MESSAGE_CLEAR: &str =
        "clear abandoned-future message buffer in LoadWorld::Mapping";

    /// ⚠ This sentence is never hashed and never compared, because the kind it
    /// belongs to is excluded from `schema_dump()` — but it is written to the
    /// same standard anyway, since `deterministic_dump()` still carries it and
    /// a reader meets it there.
    pub const MESSAGE_CLEAR_INSTRUMENT: &str =
        "clear abandoned-future INSTRUMENT message buffer in LoadWorld::Mapping; \
         outside the peer schema identity";
}

/// A registrar method's (kind, sentence) pair, spelled ONCE.
///
/// ⛔⛤ **EVERY PAIR BELOW WAS WRITTEN TWICE UNTIL 2026-09-16 — once on the
/// RECORDING road (`ambition_platformer2d_runtime`'s registrar, which writes the
/// descriptor the schema baseline and every census read) and once on the
/// INSTALLING road (`ambition_platformer2d_rollback_ggrs`, which adds the
/// snapshot plugin). Nothing derived one from the other.** Splitting
/// `resource-canonical-custom-checksum` out of `resource-canonical` on
/// 2026-09-15 changed the recording road only, and one registration arrived
/// under two different kinds. It was caught by `RollbackRegistry`'s
/// conflicting-registration check — accidental cross-evidence, not a designed
/// guard, and it covers only names BOTH roads reach.
///
/// ⭐ **MEASURED, WHICH IS WHY THIS TABLE IS KEYED ON THE PAIR AND NOT ON THE
/// METHOD.** Across both roads there are exactly 18 distinct literal (kind,
/// detail) pairs and each occurs EXACTLY TWICE — a perfect 1:1 between the
/// roads, with zero disagreements. A method-keyed table would have needed the
/// method attribution that two separate parsers of mine got wrong; the pair
/// needs none. The remaining methods take a caller-supplied `detail` and so have
/// no literal to collapse.
///
/// ⛔ **THE COSTED DESIGN THIS REPLACES DOES NOT TYPECHECK, AND THAT IS
/// MEASURED, NOT REASONED.** The queue row proposed ONE required
/// `install<T>(owner, name, kind, detail, ops)` primitive with 24 default
/// bodies. The methods' `T` bounds are DISJOINT — `SnapshotState` vs
/// `SnapshotCursor` vs `SnapshotResolve` vs `MapEntities`, and `Component` vs
/// `Resource` — so `install`'s own `where` clause would have to be their UNION,
/// and every default body fails `E0277` at the call. Compiled against `rustc` to
/// confirm rather than argued. The shapes that do typecheck (a per-op trait, or
/// fn-pointers carrying the work) either reintroduce ~21 op types — the cost the
/// row already rejected — or require this crate to name the host's `App`, which
/// is exactly the dependency the split exists to prevent.
///
/// ⇒ So the pair moves to the declaration side WITHOUT trait surgery. Changing a
/// method's kind here changes both roads, because neither road spells a kind
/// literal any more.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spelling {
    pub kind: RollbackEntryKind,
    pub detail: &'static str,
    /// The mechanical fact `detail` explains, as a stable token: which road
    /// within `kind` the row takes. The schema fingerprint hashes this and not
    /// `detail` (`Q122`), so rewording the sentence leaves peer identity alone
    /// and moving a row to another road changes it.
    pub mechanism: &'static str,
}

/// The mechanism token of a row: its road's token when `(kind, detail)` is one
/// of the [`spelling::ALL`] pairs, else [`DESCRIBED`]. A row whose `detail` a
/// caller wrote (a custom checksum's description, a derived row's reason) has
/// no road beyond its kind, and its prose is not a mechanical fact.
pub fn mechanism_of(kind: RollbackEntryKind, detail: &str) -> &'static str {
    spelling::ALL
        .iter()
        .find(|road| road.kind == kind && road.detail == detail)
        .map_or(DESCRIBED, |road| road.mechanism)
}

/// The mechanism token of a row whose `detail` its caller wrote.
pub const DESCRIBED: &str = "described";

pub mod spelling {
    use super::{RollbackEntryKind, Spelling};

    /// Declares each road once and lists it in [`ALL`], so a road cannot be
    /// spelled and left out of the lookup the fingerprint uses.
    macro_rules! roads {
        ($($name:ident = ($kind:ident, $detail:ident, $mechanism:literal);)*) => {
            $(
                pub const $name: Spelling = Spelling {
                    kind: RollbackEntryKind::$kind,
                    detail: super::detail::$detail,
                    mechanism: $mechanism,
                };
            )*
            /// Every road, in declaration order.
            pub const ALL: &[Spelling] = &[$($name),*];
        };
    }

    roads! {
        COMPONENT_CANONICAL_IDENTICAL_CHECKSUM = (ComponentCanonical, CANONICAL_IDENTICAL_CHECKSUM, "canonical-identical-checksum");
        COMPONENT_CLONE_CURSOR = (ComponentCloneCursor, CLONE_CURSOR_CHECKSUM, "cursor-checksum");
        COMPONENT_CLONE_RESOLVED = (ComponentCloneResolved, CLONE_RESOLVED_CHECKSUM, "resolved-reference-checksum");
        COMPONENT_CLONE_UNHASHED = (ComponentClone, CLONE_UNHASHED, "unhashed");
        COMPONENT_CLONE_ENTITY_REF_REMAPPED = (ComponentClone, CLONE_ENTITY_REF_REMAPPED, "entity-ref-remapped-probed");
        COMPONENT_CLONE_ENTITY_SET_REMAPPED = (ComponentClone, CLONE_ENTITY_SET_REMAPPED, "entity-set-remapped-probed");
        COMPONENT_CLONE_ENTITY_MAP_REMAPPED = (ComponentClone, CLONE_ENTITY_MAP_REMAPPED, "entity-map-remapped-probed");
        COMPONENT_CLONE_PROBED_FOR_LOCALIZATION = (ComponentClone, CLONE_PROBED_FOR_LOCALIZATION, "value-probed-unhashed");
        COMPONENT_CLONE_CANONICAL_CHECKSUM_REMAPPED = (ComponentCloneCanonicalChecksum, CLONE_CANONICAL_CHECKSUM_REMAPPED, "canonical-checksum-remapped");
        RESOURCE_CANONICAL_IDENTICAL_CHECKSUM = (ResourceCanonical, CANONICAL_IDENTICAL_CHECKSUM, "canonical-identical-checksum");
        RESOURCE_CANONICAL_PRESENCE_AWARE_CHECKSUM = (ResourceCanonical, CANONICAL_PRESENCE_AWARE_CHECKSUM, "canonical-presence-aware-checksum");
        RESOURCE_CLONE_UNHASHED = (ResourceClone, CLONE_UNHASHED, "unhashed");
        RESOURCE_CLONE_ENTITY_SET_REMAPPED = (ResourceClone, CLONE_ENTITY_SET_REMAPPED, "entity-set-remapped-probed");
        RESOURCE_CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED = (ResourceClone, CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED, "entity-set-remapped-and-value-probed");
        ENTITY_MAPPING = (EntityMapping, ENTITY_MAPPING, "entity-reference-remapping");
        RESOURCE_ENTITY_MAPPING = (ResourceEntityMapping, RESOURCE_ENTITY_MAPPING, "resource-entity-reference-remapping");
        REQUIRED_ROLLBACK = (RequiredRollback, REQUIRED_ROLLBACK, "rollback-marker-on-presence");
        MESSAGE_CLEAR = (MessageClear, MESSAGE_CLEAR, "clear-on-load");
        MESSAGE_CLEAR_INSTRUMENT = (MessageClearInstrument, MESSAGE_CLEAR_INSTRUMENT, "clear-on-load-instrument");
    }
}

#[cfg(test)]
mod tests {
    use super::{mechanism_of, spelling, DESCRIBED};

    /// Each road is one pair, and within a kind each road has its own token,
    /// or the fingerprint could not tell two roads of one kind apart.
    #[test]
    fn each_road_has_its_own_pair_and_token() {
        for (at, road) in spelling::ALL.iter().enumerate() {
            for other in &spelling::ALL[at + 1..] {
                assert!(
                    (road.kind, road.detail) != (other.kind, other.detail),
                    "two roads share a pair: {road:?}"
                );
                assert!(
                    (road.kind, road.mechanism) != (other.kind, other.mechanism),
                    "two roads of one kind share a token: {road:?} {other:?}"
                );
            }
            assert_ne!(road.mechanism, DESCRIBED, "a road uses the described token");
            assert_eq!(mechanism_of(road.kind, road.detail), road.mechanism);
        }
    }

    /// ⛔ THE TOKENS ARE PEER IDENTITY. The schema fingerprint hashes them, and
    /// no baseline row shows them, so this list is their record: a renamed
    /// token, or a road moved to another one, moves the fingerprint. If you
    /// change this list, bump `GGRS_ROLLBACK_SCHEMA_VERSION` and say why.
    #[test]
    fn the_mechanism_tokens_are_recorded() {
        let live: Vec<(&str, &str)> = spelling::ALL
            .iter()
            .map(|road| (road.kind.canonical_name(), road.mechanism))
            .collect();
        assert_eq!(
            live,
            [
                ("component-canonical", "canonical-identical-checksum"),
                ("component-clone-cursor", "cursor-checksum"),
                ("component-clone-resolved", "resolved-reference-checksum"),
                ("component-clone", "unhashed"),
                ("component-clone", "entity-ref-remapped-probed"),
                ("component-clone", "entity-set-remapped-probed"),
                ("component-clone", "entity-map-remapped-probed"),
                ("component-clone", "value-probed-unhashed"),
                ("component-clone-canonical-checksum", "canonical-checksum-remapped"),
                ("resource-canonical", "canonical-identical-checksum"),
                ("resource-canonical", "canonical-presence-aware-checksum"),
                ("resource-clone", "unhashed"),
                ("resource-clone", "entity-set-remapped-probed"),
                ("resource-clone", "entity-set-remapped-and-value-probed"),
                ("entity-mapping", "entity-reference-remapping"),
                ("resource-entity-mapping", "resource-entity-reference-remapping"),
                ("required-rollback", "rollback-marker-on-presence"),
                ("message-clear", "clear-on-load"),
                ("message-clear-instrument", "clear-on-load-instrument"),
            ]
        );
    }
}
