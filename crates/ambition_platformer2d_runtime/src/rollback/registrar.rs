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
        self.record::<T>(owner, name, RollbackEntryKind::ComponentCanonical,
            "bevy_ggrs canonical codec snapshot + identical canonical checksum projection");
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ComponentCloneCursor,
            "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection");
        self
    }

    fn rollback_component_resolved<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotResolve,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ComponentCloneResolved,
            "bevy_ggrs clone snapshot + canonical authored-reference checksum projection");
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ComponentClone,
            "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection");
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
        self.record::<T>(owner, name, RollbackEntryKind::ComponentClone,
            "bevy_ggrs clone snapshot; entity handle remapped, probed through the target's stable sim identity");
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
        self.record::<T>(owner, name, RollbackEntryKind::ComponentClone,
            "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities");
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
        self.record::<T>(owner, name, RollbackEntryKind::ComponentClone,
            "bevy_ggrs clone snapshot; keyed entity MAP remapped, probed with each key folded against its target's stable sim identity");
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
        self.record::<T>(owner, name, RollbackEntryKind::ComponentClone,
            "bevy_ggrs clone snapshot; value-probed for localization, not in the session checksum");
        self
    }

    fn rollback_component_clone_state<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotState,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ComponentCloneCanonicalChecksum,
            "bevy_ggrs clone snapshot + canonical checksum; exact Entity/reference values are remapped after load");
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
        self.record::<T>(owner, name, RollbackEntryKind::ResourceCanonical,
            "bevy_ggrs canonical codec snapshot + identical canonical checksum projection");
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
        self.record::<T>(owner, name, RollbackEntryKind::ResourceCanonical,
            "bevy_ggrs canonical codec snapshot + presence-aware canonical checksum projection");
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ResourceClone,
            "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection");
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
        self.record::<T>(owner, name, RollbackEntryKind::ResourceClone,
            "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities");
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
        self.record::<T>(owner, name, RollbackEntryKind::ResourceClone,
            "bevy_ggrs clone snapshot; entity SET remapped and probed through the targets' stable sim identities, mixed with a projection of the value's non-entity fields");
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
        self.record::<T>(owner, name, RollbackEntryKind::EntityMapping,
            "bevy_ggrs LoadWorld entity-reference remapping");
        self
    }

    fn rollback_resource_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + MapEntities,
    {
        self.record::<T>(owner, name, RollbackEntryKind::ResourceEntityMapping,
            "bevy_ggrs LoadWorld resource entity-reference remapping");
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        self.record::<T>(owner, name, RollbackEntryKind::RequiredRollback,
            "component presence automatically installs bevy_ggrs::Rollback");
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        self.record::<T>(owner, name, RollbackEntryKind::MessageClear,
            "clear abandoned-future message buffer in LoadWorld::Mapping");
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
