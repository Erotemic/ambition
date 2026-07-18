//! Thin Ambition registration layer over `bevy_ggrs`.
//!
//! This is deliberately not a snapshot registry. `bevy_ggrs` owns storage,
//! history, entity reconciliation, save/load ordering, and checksum aggregation.
//! The registry here records the exact typed contract installed into GGRS so
//! prepared content and peers can reject incompatible binaries before play.

use std::collections::BTreeMap;
use std::fmt;

use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy_ggrs::{
    ComponentSnapshotPlugin, LoadWorld, LoadWorldSystems, ResourceSnapshotPlugin, RollbackApp,
};

use crate::content_identity::SnapshotSchemaFingerprint;

use super::{
    cursor_checksum, resolved_checksum, state_checksum, CanonicalCodecStrategy, SnapshotCursor,
    SnapshotResolve, SnapshotState,
};

/// Managed same-build schema version for Ambition's GGRS registration contract.
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 2;

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
    fn canonical_name(self) -> &'static str {
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
        }
    }
}

impl std::error::Error for RollbackRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationOutcome {
    Inserted,
    Idempotent,
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
            None => {
                self.entries.insert(descriptor.name.clone(), descriptor);
                Ok(RollbackRegistrationOutcome::Inserted)
            }
            Some(existing) if existing == &descriptor => {
                Ok(RollbackRegistrationOutcome::Idempotent)
            }
            Some(existing) => Err(RollbackRegistrationError::Conflict {
                name: descriptor.name.clone(),
                existing: existing.clone(),
                incoming: descriptor,
            }),
        }
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

    pub fn schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ambition.ggrs-rollback-schema\0");
        hasher.update(&GGRS_ROLLBACK_SCHEMA_VERSION.to_le_bytes());
        let dump = self.deterministic_dump();
        hasher.update(&(dump.len() as u64).to_le_bytes());
        hasher.update(dump.as_bytes());
        SnapshotSchemaFingerprint::from_bytes(*hasher.finalize().as_bytes())
    }
}

fn descriptor<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: &'static str,
) -> RollbackRegistrationDescriptor {
    RollbackRegistrationDescriptor {
        name: name.to_string(),
        owner: owner.to_string(),
        kind,
        type_name: std::any::type_name::<T>().to_string(),
        detail: detail.to_string(),
    }
}

fn register_app_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    app.init_resource::<RollbackRegistry>();
    app.world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"))
}

/// App-level typed registration vocabulary. Each method installs the real
/// `bevy_ggrs` plugin and records the exact managed schema identity once.
pub trait AmbitionRollbackApp {
    fn rollback_component_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState;

    fn rollback_component_cursor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor;

    fn rollback_component_resolved<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve;

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone the exact component for load/mapping, but checksum a canonical
    /// projection. Use this for state containing `Entity` handles or authored
    /// references that GGRS must preserve and remap rather than decode itself.
    fn rollback_component_clone_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState;

    /// Clone the exact component and include a domain-owned deterministic
    /// checksum projection. The detail string is part of the exact schema.
    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    fn rollback_resource_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState;

    fn rollback_resource_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone + SnapshotCursor;

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone;

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone;

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + bevy::ecs::entity::MapEntities;

    fn rollback_resource_map_entities<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + bevy::ecs::entity::MapEntities;

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component;

    fn clear_message_on_rollback<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Message;

    fn declare_rollback_derived<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: 'static;

    fn declare_dynamic_anchor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
    ) -> &mut Self
    where
        T: 'static;
}

impl AmbitionRollbackApp for App {
    fn rollback_component_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_plugins(ComponentSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_component(self, state_checksum::<T>);
        }
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, cursor_checksum::<T>);
        }
        self
    }

    fn rollback_component_resolved<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneResolved,
                "bevy_ggrs clone snapshot + canonical authored-reference checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, resolved_checksum::<T>);
        }
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
        }
        self
    }

    fn rollback_component_clone_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCanonicalChecksum,
                "bevy_ggrs clone snapshot + canonical checksum; exact Entity/reference values are remapped after load",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, state_checksum::<T>);
        }
        self
    }

    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCustomChecksum,
                detail,
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, checksum);
        }
        self
    }

    fn rollback_resource_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_plugins(ResourceSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_resource(self, state_checksum::<T>);
        }
        self
    }

    fn rollback_resource_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone + SnapshotCursor,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            RollbackApp::checksum_resource(self, cursor_checksum::<T>);
        }
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
        }
        self
    }

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCloneCustomChecksum,
                detail,
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            RollbackApp::checksum_resource(self, checksum);
        }
        self
    }

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + bevy::ecs::entity::MapEntities,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::EntityMapping,
                "bevy_ggrs LoadWorld entity-reference remapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::update_component_with_map_entities::<T>(self);
        }
        self
    }

    fn rollback_resource_map_entities<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + bevy::ecs::entity::MapEntities,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceEntityMapping,
                "bevy_ggrs LoadWorld resource entity-reference remapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::update_resource_with_map_entities::<T>(self);
        }
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::RequiredRollback,
                "component presence automatically installs bevy_ggrs::Rollback",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            RollbackApp::require_rollback::<T>(self);
        }
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        if register_app_descriptor(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::MessageClear,
                "clear abandoned-future message buffer in LoadWorld::Mapping",
            ),
        ) == RollbackRegistrationOutcome::Inserted
        {
            self.add_systems(
                LoadWorld,
                clear_message_channel::<T>.in_set(LoadWorldSystems::Mapping),
            );
        }
        self
    }

    fn declare_rollback_derived<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        register_app_descriptor(
            self,
            descriptor::<T>(owner, name, RollbackEntryKind::Derived, reason),
        );
        self
    }

    fn declare_dynamic_anchor<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        register_app_descriptor(
            self,
            descriptor::<T>(owner, name, RollbackEntryKind::DynamicAnchor, detail),
        );
        self
    }
}

fn clear_message_channel<T: Message>(messages: Option<ResMut<Messages<T>>>) {
    if let Some(mut messages) = messages {
        messages.clear();
    }
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
