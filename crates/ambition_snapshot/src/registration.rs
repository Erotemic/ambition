//! Register Bevy state with a rollback backend.

use crate::snapshot::{SnapshotCursor, SnapshotResolve, SnapshotState};

/// Backend-neutral registrar for domain-owned rollback state.
///
/// Domains supply the concrete component type and checksum projection; the host
/// implements storage mechanics and may depend on the rollback backend. Generic
/// methods are intentionally not object-safe. The runtime uses an `App` wrapper
/// to satisfy dependency and orphan-rule boundaries without moving backend
/// dependencies into domain crates.
pub trait RollbackRegistrar {
    /// Defaults fail closed so a partial test registrar cannot silently omit newly
    /// requested rollback state.
    fn rollback_component_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_component_canonical for {name}")
    }

    /// Canonical snapshot whose CHECKSUM is a stated projection of the value.
    ///
    /// ⛔⛤ **THE COMPONENT HALF OF A FAMILY THAT WAS ASYMMETRIC.** Resources had
    /// `rollback_resource_canonical_checksum` and
    /// `rollback_resource_optional_canonical_checksum`; components had only the
    /// CLONE-strategy `rollback_component_clone_checksum`. So a component that
    /// snapshots through its own canonical codec had no way to state a
    /// projection at all, and the peer/local split was simply unavailable for
    /// it — which is not a decision anybody made, it is a gap. Measured
    /// 2026-09-15 while looking for the registrar `TransactionId` would need:
    /// its `{epoch}\t{room}\t{session}` string is `component-canonical`, two of
    /// its three terms are per-App counts, and there was no mechanism to compare
    /// less than all of it.
    ///
    /// ⚠ `SimId` is in the same position, and it is the type both provenance
    /// defects in this campaign travelled through.
    fn rollback_component_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support rollback_component_canonical_checksum for {name}"
        )
    }

    fn rollback_component_cursor<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotCursor,
    {
        panic!("RollbackRegistrar does not support rollback_component_cursor for {name}")
    }

    fn rollback_component_resolved<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotResolve,
    {
        panic!("RollbackRegistrar does not support rollback_component_resolved for {name}")
    }

    fn rollback_component_clone<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone for {name}")
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> bevy_ecs::entity::Entity,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_ref for {name}")
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_set for {name}")
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<(u64, bevy_ecs::entity::Entity)>,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_entity_map for {name}")
    }

    fn rollback_component_clone_probed<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_probed for {name}")
    }

    fn rollback_component_clone_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + Clone
            + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_state for {name}")
    }

    /// Clone-snapshot a component and checksum the domain projection.
    ///
    /// `projection` describes only what the checksum sees. The backend owns the
    /// storage half of the schema detail, so a domain never has to name GGRS.
    fn rollback_component_clone_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _projection: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_component_clone_checksum for {name}")
    }

    /// Clone-snapshot a component and checksum a domain projection while preserving
    /// an exact, domain-owned schema description.
    ///
    /// Unlike [`Self::rollback_component_clone_checksum`], `detail` is already the
    /// complete schema detail and is recorded verbatim. Use this when the stable
    /// schema identity intentionally owns its prose rather than composing a
    /// backend storage description with a projection description.
    fn rollback_component_clone_checksum_with_schema_detail<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!(
            "RollbackRegistrar does not support rollback_component_clone_checksum_with_schema_detail for {name}"
        )
    }

    fn rollback_resource_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_canonical for {name}")
    }

    fn rollback_resource_optional_canonical<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_optional_canonical for {name}")
    }

    /// Canonical snapshot whose CHECKSUM is a stated projection of the value.
    ///
    /// For an always-present resource that must rewind intact while holding a
    /// field two peers cannot agree on.
    fn rollback_resource_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!("RollbackRegistrar does not support rollback_resource_canonical_checksum for {name}")
    }

    /// Presence-aware canonical snapshot whose CHECKSUM is a stated projection
    /// of the value rather than the whole encoding — for a resource that must
    /// rewind intact while holding a field two peers cannot agree on.
    fn rollback_resource_optional_canonical_checksum<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support \
             rollback_resource_optional_canonical_checksum for {name}"
        )
    }

    fn rollback_resource_clone<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone for {name}")
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone_entity_set for {name}")
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _referenced: fn(&T) -> Vec<bevy_ecs::entity::Entity>,
        _facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!("RollbackRegistrar does not support rollback_resource_clone_entity_set_probed for {name}")
    }

    /// Clone-snapshot a resource and checksum the domain projection.
    ///
    /// `projection` describes only what the checksum sees. The backend owns the
    /// storage half of the schema detail, so a domain never has to name GGRS.
    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone;

    /// Resource twin of
    /// [`Self::rollback_component_clone_checksum_with_schema_detail`].
    fn rollback_resource_clone_checksum_with_schema_detail<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
        _checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
    {
        panic!(
            "RollbackRegistrar does not support rollback_resource_clone_checksum_with_schema_detail for {name}"
        )
    }

    fn rollback_map_entities<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component<Mutability = bevy_ecs::component::Mutable>
            + bevy_ecs::entity::MapEntities,
    {
        panic!("RollbackRegistrar does not support rollback_map_entities for {name}")
    }

    fn rollback_resource_map_entities<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable>
            + bevy_ecs::entity::MapEntities,
    {
        panic!("RollbackRegistrar does not support rollback_resource_map_entities for {name}")
    }

    fn require_rollback<T>(&mut self, _owner: &'static str, name: &'static str) -> &mut Self
    where
        T: bevy_ecs::component::Component,
    {
        panic!("RollbackRegistrar does not support require_rollback for {name}")
    }

    fn clear_message_on_rollback<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::message::Message,
    {
        panic!("RollbackRegistrar does not support clear_message_on_rollback for {name}")
    }

    /// Clear a message channel on rollback that feeds an INSTRUMENT, not the
    /// simulation.
    ///
    /// Same rewind behaviour as [`Self::clear_message_on_rollback`]; the
    /// difference is that the registration is not part of the schema identity
    /// two peers negotiate. See
    /// `RollbackEntryKind::MessageClearInstrument` for the measurement that
    /// made this a separate road.
    fn clear_instrument_message_on_rollback<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::message::Message,
    {
        panic!("RollbackRegistrar does not support clear_instrument_message_on_rollback for {name}")
    }

    fn declare_rollback_derived_component<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_component for {name}")
    }

    /// Declare derived component state with a canonical value projection for
    /// restore localization. The backend may use the projection for diagnostics
    /// without snapshotting the derived value itself.
    fn declare_rollback_derived_component_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::component::Component + SnapshotState,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_component_state for {name}")
    }

    fn declare_rollback_derived_resource<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource,
    {
        panic!("RollbackRegistrar does not support declare_rollback_derived_resource for {name}")
    }

    /// Resource twin of [`Self::declare_rollback_derived_component_state`].
    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _reason: &'static str,
    ) -> &mut Self
    where
        T: bevy_ecs::resource::Resource + SnapshotState,
    {
        panic!(
            "RollbackRegistrar does not support declare_rollback_derived_resource_state for {name}"
        )
    }

    fn declare_dynamic_anchor<T>(
        &mut self,
        _owner: &'static str,
        name: &'static str,
        _detail: &'static str,
    ) -> &mut Self
    where
        T: 'static,
    {
        panic!("RollbackRegistrar does not support declare_dynamic_anchor for {name}")
    }
}

#[cfg(test)]
mod rollback_registrar_default_method_tests {
    use super::RollbackRegistrar;

    // Bevy 0.19: `Resource: Component`, and only the derive can emit both.
    #[derive(Clone, bevy_ecs::resource::Resource)]
    struct DummyResource;

    struct CapturingRegistrar {
        called: bool,
    }

    impl RollbackRegistrar for CapturingRegistrar {
        fn rollback_resource_clone_checksum<T>(
            &mut self,
            _owner: &'static str,
            _name: &'static str,
            _projection: &'static str,
            _checksum: for<'a> fn(&'a T) -> u64,
        ) -> &mut Self
        where
            T: bevy_ecs::resource::Resource<Mutability = bevy_ecs::component::Mutable> + Clone,
        {
            self.called = true;
            self
        }
    }

    fn dummy_checksum(_: &DummyResource) -> u64 {
        0
    }

    #[test]
    fn a_narrow_registrar_only_implements_the_operation_it_captures() {
        let mut registrar = CapturingRegistrar { called: false };
        registrar.rollback_resource_clone_checksum::<DummyResource>(
            "test",
            "resource.dummy",
            "dummy projection",
            dummy_checksum,
        );
        assert!(registrar.called);
    }
}
