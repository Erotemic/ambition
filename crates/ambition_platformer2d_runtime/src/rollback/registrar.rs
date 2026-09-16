//! Backend-neutral recorder for domain-owned rollback declarations.
//!
//! This registrar records the exact schema row a concrete rollback backend would install, but
//! installs no snapshot/session machinery. Concrete hosts implement the same
//! [`RollbackRegistrar`] vocabulary in their own crates.

use bevy::ecs::component::Mutable;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::message::Message;
use bevy::prelude::{App, Component, Entity, Resource};

use ambition_platformer2d_core::snapshot::{
    RollbackRegistrar, SnapshotCursor, SnapshotResolve, SnapshotState,
};

use super::registry::{self, RollbackEntryKind};

//: ⭐ MOVED to `ambition_platformer2d_core::rollback_kind::detail` 2026-09-16
//: and re-exported here, in the same step as `RollbackEntryKind`. A default
//: trait body that names the kind but not the SENTENCE would close half of
//: ROLLBACK-KIND-SPELLING and reopen the other half one crate away.
pub use ambition_platformer2d_core::rollback_kind::detail;

//: ⭐ THE (kind, sentence) PAIR, SPELLED ONCE FOR BOTH ROADS
//: (ROLLBACK-KIND-SPELLING, 2026-09-16).
pub use ambition_platformer2d_core::rollback_kind::spelling;

/// A metadata-only [`RollbackRegistrar`] borrowed from the composition's app.
pub struct SchemaRollbackRegistrar<'a> {
    app: &'a mut App,
}

impl<'a> SchemaRollbackRegistrar<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    fn record<T: 'static>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        kind: RollbackEntryKind,
        detail: &'static str,
    ) {
        registry::record_descriptor(
            self.app,
            registry::descriptor::<T>(owner, name, kind, detail),
        );
    }

    fn record_owned<T: 'static>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        kind: RollbackEntryKind,
        detail: String,
    ) {
        registry::record_descriptor(
            self.app,
            registry::descriptor_owned::<T>(owner, name, kind, detail),
        );
    }
}

impl RollbackRegistrar for SchemaRollbackRegistrar<'_> {
    fn rollback_component_canonical<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CANONICAL_IDENTICAL_CHECKSUM.kind,
            spelling::COMPONENT_CANONICAL_IDENTICAL_CHECKSUM.detail);
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_CURSOR.kind,
            spelling::COMPONENT_CLONE_CURSOR.detail);
        self
    }

    fn rollback_component_resolved<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_RESOLVED.kind,
            spelling::COMPONENT_CLONE_RESOLVED.detail);
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_UNHASHED.kind,
            spelling::COMPONENT_CLONE_UNHASHED.detail);
        self
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_ENTITY_REF_REMAPPED.kind,
            spelling::COMPONENT_CLONE_ENTITY_REF_REMAPPED.detail);
        self
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_ENTITY_SET_REMAPPED.kind,
            spelling::COMPONENT_CLONE_ENTITY_SET_REMAPPED.detail);
        self
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<(u64, Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_ENTITY_MAP_REMAPPED.kind,
            spelling::COMPONENT_CLONE_ENTITY_MAP_REMAPPED.detail);
        self
    }

    fn rollback_component_clone_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_PROBED_FOR_LOCALIZATION.kind,
            spelling::COMPONENT_CLONE_PROBED_FOR_LOCALIZATION.detail);
        self
    }

    fn rollback_component_clone_state<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState,
    {
        self.record::<T>(owner, name, spelling::COMPONENT_CLONE_CANONICAL_CHECKSUM_REMAPPED.kind,
            spelling::COMPONENT_CLONE_CANONICAL_CHECKSUM_REMAPPED.detail);
        self
    }

    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record_owned::<T>(owner, name, RollbackEntryKind::ComponentCloneCustomChecksum,
            format!("bevy_ggrs clone snapshot + {projection}"));
        self
    }

    fn rollback_component_clone_checksum_with_schema_detail<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(
            owner,
            name,
            RollbackEntryKind::ComponentCloneCustomChecksum,
            detail,
        );
        self
    }

    fn rollback_resource_canonical<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_CANONICAL_IDENTICAL_CHECKSUM.kind,
            spelling::RESOURCE_CANONICAL_IDENTICAL_CHECKSUM.detail);
        self
    }

    fn rollback_resource_optional_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_CANONICAL_PRESENCE_AWARE_CHECKSUM.kind,
            spelling::RESOURCE_CANONICAL_PRESENCE_AWARE_CHECKSUM.detail);
        self
    }

    fn rollback_component_canonical_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Component + SnapshotState,
    {
        // ⛔ SECOND SPELLING OF THE SAME KIND. The backend registrar in
        // `ambition_platformer2d_rollback_ggrs` spells it too, and the registry
        // panics if the two disagree — see ROLLBACK-KIND-SPELLING.
        self.record::<T>(owner, name, RollbackEntryKind::ComponentCanonicalCustomChecksum, detail);
        self
    }

    fn rollback_resource_canonical_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ResourceCanonicalCustomChecksum, detail);
        self
    }

    /// Presence-aware canonical snapshot whose CHECKSUM is a stated projection
    /// rather than the whole encoded value.
    ///
    /// For a resource that must survive a rewind intact while holding a field
    /// two peers cannot agree on. `detail` names what the projection covers, so
    /// the schema baseline records the distinction rather than only the kind.
    fn rollback_resource_optional_canonical_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ResourceCanonicalCustomChecksum, detail);
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_CLONE_UNHASHED.kind,
            spelling::RESOURCE_CLONE_UNHASHED.detail);
        self
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<Entity>,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_CLONE_ENTITY_SET_REMAPPED.kind,
            spelling::RESOURCE_CLONE_ENTITY_SET_REMAPPED.detail);
        self
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<Entity>,
        _facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED.kind,
            spelling::RESOURCE_CLONE_ENTITY_SET_REMAPPED_AND_VALUE_PROBED.detail);
        self
    }

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record_owned::<T>(owner, name, RollbackEntryKind::ResourceCloneCustomChecksum,
            format!("bevy_ggrs clone snapshot + {projection}"));
        self
    }

    fn rollback_resource_clone_checksum_with_schema_detail<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record::<T>(
            owner,
            name,
            RollbackEntryKind::ResourceCloneCustomChecksum,
            detail,
        );
        self
    }

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + MapEntities,
    {
        self.record::<T>(owner, name, spelling::ENTITY_MAPPING.kind,
            spelling::ENTITY_MAPPING.detail);
        self
    }

    fn rollback_resource_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + MapEntities,
    {
        self.record::<T>(owner, name, spelling::RESOURCE_ENTITY_MAPPING.kind,
            spelling::RESOURCE_ENTITY_MAPPING.detail);
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        self.record::<T>(owner, name, spelling::REQUIRED_ROLLBACK.kind,
            spelling::REQUIRED_ROLLBACK.detail);
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        self.record::<T>(owner, name, spelling::MESSAGE_CLEAR.kind,
            spelling::MESSAGE_CLEAR.detail);
        self
    }

    fn declare_rollback_derived_component<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component,
    {
        self.record::<T>(owner, name, RollbackEntryKind::Derived, reason);
        self
    }

    fn declare_rollback_derived_component_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component + SnapshotState,
    {
        self.record::<T>(owner, name, RollbackEntryKind::Derived, reason);
        self
    }

    fn declare_rollback_derived_resource<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource,
    {
        self.record::<T>(owner, name, RollbackEntryKind::Derived, reason);
        self
    }

    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.record::<T>(owner, name, RollbackEntryKind::Derived, reason);
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
        self.record::<T>(owner, name, RollbackEntryKind::DynamicAnchor, detail);
        self
    }
}
