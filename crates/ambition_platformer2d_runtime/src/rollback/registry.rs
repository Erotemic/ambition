//! Backend-neutral rollback schema metadata.
//!
//! Gameplay domains declare typed rollback obligations through
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar`. This module records the exact
//! managed schema those declarations describe; concrete rollback hosts install storage/checksum
//! machinery separately.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::*;

use crate::content_identity::SnapshotSchemaFingerprint;

/// Managed same-build version of the rollback schema contract.
///
/// Bump when the registered state set, wire type identity, encoded payload, or
/// checksum projection changes incompatibly. Peers with different versions must
/// not treat their snapshots as compatible.
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 115;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RollbackEntryKind {
    ComponentCanonical,
    ComponentCloneCursor,
    ComponentCloneResolved,
    ComponentClone,
    ComponentCloneCanonicalChecksum,
    ComponentCloneCustomChecksum,
    ResourceCanonical,
    ResourceCloneCursor,
    ResourceClone,
    ResourceCloneCustomChecksum,
    MessageClear,
    EntityMapping,
    ResourceEntityMapping,
    RequiredRollback,
    Derived,
    DynamicAnchor,
}

impl RollbackEntryKind {
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
            | Self::ResourceCanonical
            | Self::ResourceCloneCursor
            | Self::ResourceClone
            | Self::ResourceCloneCustomChecksum
            | Self::Derived => true,
            Self::MessageClear
            | Self::EntityMapping
            | Self::ResourceEntityMapping
            | Self::RequiredRollback
            | Self::DynamicAnchor => false,
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
            Self::ResourceCanonical => "resource-canonical",
            Self::ResourceCloneCursor => "resource-clone-cursor",
            Self::ResourceClone => "resource-clone",
            Self::ResourceCloneCustomChecksum => "resource-clone-custom-checksum",
            Self::MessageClear => "message-clear",
            Self::EntityMapping => "entity-mapping",
            Self::ResourceEntityMapping => "resource-entity-mapping",
            Self::RequiredRollback => "required-rollback",
            Self::Derived => "derived",
            Self::DynamicAnchor => "dynamic-anchor",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RollbackRegistrationDescriptor {
    pub name: String,
    pub owner: String,
    pub kind: RollbackEntryKind,
    pub type_name: String,
    pub detail: String,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RollbackRegistry {
    entries: BTreeMap<String, RollbackRegistrationDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationError {
    EmptyName,
    EmptyOwner,
    Conflict {
        name: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
    /// Two DIFFERENT Rust types reduce to the same [`wire_type_identity`].
    ///
    /// This is what keeps v20's narrower identity sound. The fingerprint hashes
    /// the type's final segment so that relocating a type is not a wire-format
    /// change — and that is only truthful while final segments are unique. Two
    /// crates each registering a `Cooldown` would hash equal, and a peer that
    /// had them the other way round would be declared compatible.
    ///
    ///  registering ONE type under several stable names is not this. The whole
    /// point of a stable name is that it identifies the registration; 39 of the
    /// live rows do exactly that, and they carry identical type names.
    TypeIdentityCollision {
        identity: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
}

impl fmt::Display for RollbackRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "rollback registration name must not be empty"),
            Self::EmptyOwner => write!(f, "rollback registration owner must not be empty"),
            Self::Conflict {
                name,
                existing,
                incoming,
            } => write!(
                f,
                "conflicting rollback registration '{name}': existing {existing:?}, incoming {incoming:?}"
            ),
            Self::TypeIdentityCollision {
                identity,
                existing,
                incoming,
            } => write!(
                f,
                "two different types share the rollback wire identity '{identity}', which the \
                 schema fingerprint cannot tell apart: existing {existing:?}, incoming {incoming:?}. \
                 Since v20 the fingerprint hashes a type's FINAL SEGMENT so that moving a type \
                 between crates or modules is not a wire-format change, and that stays sound only \
                 while final segments are unique. Rename one of the two types."
            ),
        }
    }
}

impl std::error::Error for RollbackRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationOutcome {
    /// The descriptor was inserted and the active GGRS host should install its
    /// runtime snapshot/checksum machinery.
    Inserted,
    /// The exact descriptor was already present.
    Idempotent,
    /// The descriptor was inserted for schema/content identity, but this host
    /// does not run GGRS and therefore must not install rollback machinery.
    RecordedOnly,
}

/// The part of a type's name that a CARVE leaves alone.
///
///  the final segment, and not the module path below the crate, which is what the answer
/// was until the diff it cited was read.
///
/// Every path INSIDE the name is shortened, not only the outermost one, so a
/// generic keeps its constructor: `Vec<foo::Bar>` is `Vec<Bar>` and not `Bar>`.
/// No registration is generic today; taking a single `rsplit` would quietly give
/// `Vec<X>` and `VecDeque<X>` one identity, and this is a hash whose entire job
/// is telling wire formats apart.
fn wire_type_identity(type_name: &str) -> String {
    let mut out = String::with_capacity(type_name.len());
    let mut path = String::new();
    for ch in type_name.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' {
            path.push(ch);
        } else {
            out.push_str(final_segment(&path));
            path.clear();
            out.push(ch);
        }
    }
    out.push_str(final_segment(&path));
    out
}

fn final_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

impl RollbackRegistry {
    pub fn try_register(
        &mut self,
        descriptor: RollbackRegistrationDescriptor,
    ) -> Result<RollbackRegistrationOutcome, RollbackRegistrationError> {
        if descriptor.name.trim().is_empty() {
            return Err(RollbackRegistrationError::EmptyName);
        }
        if descriptor.owner.trim().is_empty() {
            return Err(RollbackRegistrationError::EmptyOwner);
        }
        match self.entries.get(&descriptor.name) {
            Some(existing) if existing == &descriptor => {
                return Ok(RollbackRegistrationOutcome::Idempotent);
            }
            Some(existing) => {
                return Err(RollbackRegistrationError::Conflict {
                    name: descriptor.name.clone(),
                    existing: existing.clone(),
                    incoming: descriptor,
                });
            }
            None => {}
        }
        // What keeps v20's narrower identity sound. The fingerprint hashes
        // [`wire_type_identity`] so that relocating a type is not a wire-format
        // change; two crates each registering a `Cooldown` would then hash equal,
        // and a peer that had the two the other way round would be declared
        // compatible with this one. The duplicate-NAME refusal above does not
        // reach it — these arrive under different stable names, which is exactly
        // the case that looks legitimate.
        let identity = wire_type_identity(&descriptor.type_name);
        let collision = self
            .entries
            .values()
            .find(|existing| {
                existing.type_name != descriptor.type_name
                    && wire_type_identity(&existing.type_name) == identity
            })
            .cloned();
        if let Some(existing) = collision {
            return Err(RollbackRegistrationError::TypeIdentityCollision {
                identity,
                existing,
                incoming: descriptor,
            });
        }
        self.entries.insert(descriptor.name.clone(), descriptor);
        Ok(RollbackRegistrationOutcome::Inserted)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &RollbackRegistrationDescriptor> {
        self.entries.values()
    }

    /// Stable human-readable representation; byte-identical under equivalent
    /// plugin/registration insertion orders.
    pub fn deterministic_dump(&self) -> String {
        let mut out = format!("ggrs-rollback-schema-v{}\n", GGRS_ROLLBACK_SCHEMA_VERSION);
        for entry in self.entries.values() {
            use std::fmt::Write as _;
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                entry.name,
                entry.owner,
                entry.kind.canonical_name(),
                entry.type_name,
                entry.detail
            );
        }
        out
    }

    /// What the schema actually IS, with every organisational label removed.
    ///
    /// [`Self::deterministic_dump`] carries `owner` and the type's full path because a human
    /// reading a conflict wants to know which module registered a thing and where the type
    /// lives.
    ///
    /// Both moves require the schema fingerprint to stay unchanged — which was impossible while
    /// the fingerprint hashed who registered a row. `owner` left in v5; [`wire_type_identity`]
    /// is the second half of that decision, in v20.
    pub fn schema_dump(&self) -> String {
        let mut out = format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}\n");
        for entry in self.entries.values() {
            use std::fmt::Write as _;
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}",
                entry.name,
                entry.kind.canonical_name(),
                wire_type_identity(&entry.type_name),
                entry.detail
            );
        }
        out
    }

    /// Which of these requirements is NOT installed.
    ///
    /// A capability offers its rollback state and the composition installs it,
    /// which keeps the capability's dependency closure to foundations. The hole
    /// that leaves is that nothing forces the composition to accept the offer —
    /// and a skipped registration is a DESYNC, not a missing feature.
    ///
    /// This closes it the way the content compiler closes the same shape: the
    /// obligation is declared next to the thing that has it
    /// ([`ambition_platformer2d_core::snapshot::RequiredRollbackState`]) and the
    /// assembler can refuse when it is unmet.
    ///
    ///  it checks the OWNER too. A name registered by somebody else is not
    /// this capability's state — two capabilities may reasonably both want a
    /// `cooldown`, and only the owner distinguishes them.
    pub fn missing_required_state<'a>(
        &self,
        required: &'a [ambition_platformer2d_core::snapshot::RequiredRollbackState],
    ) -> Vec<&'a ambition_platformer2d_core::snapshot::RequiredRollbackState> {
        required
            .iter()
            .filter(|req| {
                !self
                    .entries
                    .values()
                    .any(|entry| entry.name == req.name && entry.owner == req.owner)
            })
            .collect()
    }

    pub fn schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ambition.ggrs-rollback-schema\0");
        hasher.update(&GGRS_ROLLBACK_SCHEMA_VERSION.to_le_bytes());
        let dump = self.schema_dump();
        hasher.update(&(dump.len() as u64).to_le_bytes());
        hasher.update(dump.as_bytes());
        SnapshotSchemaFingerprint::from_bytes(*hasher.finalize().as_bytes())
    }
}

pub fn descriptor<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: &'static str,
) -> RollbackRegistrationDescriptor {
    descriptor_owned::<T>(owner, name, kind, detail.to_string())
}

/// [`descriptor`] for a detail this crate COMPOSES rather than quotes.
///
/// The recorded `detail` is two halves: how the value is stored (the backend's
/// half — "bevy_ggrs clone snapshot") and what the checksum sees (the domain's
/// half). A domain that registers its own state through
/// [`ambition_platformer2d_core::snapshot::RollbackRegistrar`] supplies only the
/// second half, precisely so a crate with no `bevy_ggrs` dependency never has to
/// write the word; this joins them back into the exact string the schema baseline
/// records.
pub fn descriptor_owned<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: String,
) -> RollbackRegistrationDescriptor {
    RollbackRegistrationDescriptor {
        name: name.to_string(),
        owner: owner.to_string(),
        kind,
        type_name: std::any::type_name::<T>().to_string(),
        detail,
    }
}

/// Record one schema descriptor on an app, independent of the active rollback backend.
///
/// Backend installation deliberately has a separate idempotence authority: a row may
/// already have been recorded by a capability plugin before a concrete rollback host
/// installs its typed snapshot machinery. Therefore callers must not interpret an
/// `Idempotent` schema row as evidence that a backend registration already exists.
pub fn record_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    app.init_resource::<RollbackRegistry>();
    app.world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, owner: &str, detail: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: owner.to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: "test::Type".to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn schema_is_insertion_order_independent() {
        let mut a = RollbackRegistry::default();
        a.try_register(entry("z", "provider-b", "second")).unwrap();
        a.try_register(entry("a", "provider-a", "first")).unwrap();

        let mut b = RollbackRegistry::default();
        b.try_register(entry("a", "provider-a", "first")).unwrap();
        b.try_register(entry("z", "provider-b", "second")).unwrap();

        assert_eq!(a.deterministic_dump(), b.deterministic_dump());
        assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
    }

    #[test]
    fn identical_registration_is_idempotent() {
        let descriptor = entry("same", "provider", "same");
        let mut registry = RollbackRegistry::default();
        assert_eq!(
            registry.try_register(descriptor.clone()).unwrap(),
            RollbackRegistrationOutcome::Inserted
        );
        assert_eq!(
            registry.try_register(descriptor).unwrap(),
            RollbackRegistrationOutcome::Idempotent
        );
        assert_eq!(registry.descriptors().count(), 1);
    }

    fn typed_entry(name: &str, type_name: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: "test-owner".to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: type_name.to_owned(),
            detail: "test-only descriptor".to_owned(),
        }
    }

    fn registry_of(rows: &[(&str, &str)]) -> RollbackRegistry {
        let mut registry = RollbackRegistry::default();
        for (name, type_name) in rows {
            registry.try_register(typed_entry(name, type_name)).unwrap();
        }
        registry
    }

    /// Where a type LIVES is not part of the wire format (v20).
    ///
    /// Only the final segment survived either move.
    #[test]
    fn relocating_a_type_leaves_the_fingerprint_alone() {
        let before = registry_of(&[
            (
                "actor.anim_override",
                "ambition_platformer2d_actor_monolith::features::ecs::actor_clusters::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_actor_monolith::avatar::components::PlayerBlinkCameraState",
            ),
        ]);
        let after = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState",
            ),
        ]);
        assert_eq!(
            before.schema_fingerprint(),
            after.schema_fingerprint(),
            "moving a rollback-registered type to another crate and another \
             module moved the schema fingerprint. Nothing a peer can observe \
             changed, so two peers running byte-identical snapshot logic would \
             refuse to agree — which makes every carve in the decomposition \
             campaign a netplay compatibility break."
        );

        // POISON. Without it this test is equally green for a fingerprint that
        // hashes nothing about the type at all, and dropping `type_name` from
        // the dump entirely was a real alternative — it costs the last signal
        // that a DIFFERENT Rust type got registered under an existing name.
        let renamed = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkEaseState",
            ),
        ]);
        assert_ne!(
            after.schema_fingerprint(),
            renamed.schema_fingerprint(),
            "a stable name that changed which TYPE it registers left the \
             fingerprint alone, so the dump is no longer hashing the type in \
             any form."
        );
    }

    /// What makes the narrower identity sound.
    ///
    /// Two `Cooldown`s in two crates hash equal once the final segment is the
    /// identity, so a peer holding them the other way round would be declared
    /// compatible. The second half asserts the guard is not merely strict: one
    /// type registered under two stable names is the ordinary case, and 39 of
    /// the live rows are it.
    #[test]
    fn two_types_sharing_a_final_segment_are_rejected_and_one_type_twice_is_not() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(typed_entry(
                "ability.cooldown",
                "ambition_combat::ability::Cooldown",
            ))
            .unwrap();

        let error = registry
            .try_register(typed_entry(
                "weapon.cooldown",
                "ambition_projectiles::weapon::Cooldown",
            ))
            .unwrap_err();
        assert!(
            matches!(
                error,
                RollbackRegistrationError::TypeIdentityCollision { .. }
            ),
            "two different types whose names end in `Cooldown` were accepted, \
             and the fingerprint cannot tell them apart: {error}"
        );

        registry
            .try_register(typed_entry(
                "ability.cooldown_mirror",
                "ambition_combat::ability::Cooldown",
            ))
            .expect(
                "registering ONE type under a second stable name is not a \
                 collision — the stable name is what identifies a registration, \
                 and refusing this would reject 39 of the live rows",
            );
    }

    #[test]
    fn conflicting_registration_is_transactional() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("same", "provider-a", "old"))
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .try_register(entry("same", "provider-b", "new"))
            .unwrap_err();
        assert!(matches!(error, RollbackRegistrationError::Conflict { .. }));
        assert_eq!(registry.deterministic_dump(), before);
    }
}
