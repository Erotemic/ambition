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
/// `RollbackRegistry::schema_fingerprint` (unlinked on purpose: it lives in
/// `ambition_platformer2d_runtime`, which sits ABOVE this crate and is not a
/// dependency of it), which is the snapshot schema's identity, and the two
/// roads would name the same
/// registration differently depending on which registrar ran.
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
}

pub mod spelling {
    use super::{RollbackEntryKind, Spelling};

    pub const COMPONENT_CANONICAL_IDENTICAL_CHECKSUM: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentCanonical,
        detail: super::detail::CANONICAL_IDENTICAL_CHECKSUM,
    };

    pub const COMPONENT_CLONE_CURSOR: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentCloneCursor,
        detail: super::detail::CLONE_CURSOR_CHECKSUM,
    };

    pub const COMPONENT_CLONE_RESOLVED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentCloneResolved,
        detail: super::detail::CLONE_RESOLVED_CHECKSUM,
    };

    pub const COMPONENT_CLONE_UNHASHED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentClone,
        detail: super::detail::CLONE_UNHASHED,
    };

    pub const COMPONENT_CLONE_ENTITY_REF_REMAPPED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentClone,
        detail: super::detail::CLONE_ENTITY_REF_REMAPPED,
    };

    pub const COMPONENT_CLONE_ENTITY_SET_REMAPPED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentClone,
        detail: super::detail::CLONE_ENTITY_SET_REMAPPED,
    };

    pub const COMPONENT_CLONE_ENTITY_MAP_REMAPPED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentClone,
        detail: super::detail::CLONE_ENTITY_MAP_REMAPPED,
    };

    pub const COMPONENT_CLONE_PROBED_FOR_LOCALIZATION: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentClone,
        detail: super::detail::CLONE_PROBED_FOR_LOCALIZATION,
    };

    pub const COMPONENT_CLONE_CANONICAL_CHECKSUM_REMAPPED: Spelling = Spelling {
        kind: RollbackEntryKind::ComponentCloneCanonicalChecksum,
        detail: super::detail::CLONE_CANONICAL_CHECKSUM_REMAPPED,
    };

    pub const RESOURCE_CANONICAL_IDENTICAL_CHECKSUM: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceCanonical,
        detail: super::detail::CANONICAL_IDENTICAL_CHECKSUM,
    };

    pub const RESOURCE_CANONICAL_PRESENCE_AWARE_CHECKSUM: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceCanonical,
        detail: super::detail::CANONICAL_PRESENCE_AWARE_CHECKSUM,
    };

    pub const RESOURCE_CLONE_UNHASHED: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceClone,
        detail: super::detail::CLONE_UNHASHED,
    };

    pub const RESOURCE_CLONE_ENTITY_SET_REMAPPED: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceClone,
        detail: super::detail::CLONE_ENTITY_SET_REMAPPED,
    };

    pub const RESOURCE_CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceClone,
        detail: super::detail::CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED,
    };

    pub const ENTITY_MAPPING: Spelling = Spelling {
        kind: RollbackEntryKind::EntityMapping,
        detail: super::detail::ENTITY_MAPPING,
    };

    pub const RESOURCE_ENTITY_MAPPING: Spelling = Spelling {
        kind: RollbackEntryKind::ResourceEntityMapping,
        detail: super::detail::RESOURCE_ENTITY_MAPPING,
    };

    pub const REQUIRED_ROLLBACK: Spelling = Spelling {
        kind: RollbackEntryKind::RequiredRollback,
        detail: super::detail::REQUIRED_ROLLBACK,
    };

    pub const MESSAGE_CLEAR: Spelling = Spelling {
        kind: RollbackEntryKind::MessageClear,
        detail: super::detail::MESSAGE_CLEAR,
    };

    pub const MESSAGE_CLEAR_INSTRUMENT: Spelling = Spelling {
        kind: RollbackEntryKind::MessageClearInstrument,
        detail: super::detail::MESSAGE_CLEAR_INSTRUMENT,
    };
}
