//! The GGRS side of the domain-owned registration seam.
//!
//! Domains own the concrete state they need rewound and speak only the
//! backend-neutral [`RollbackRegistrar`] vocabulary. This module owns the other
//! half: translating that vocabulary into the existing `bevy_ggrs` registration,
//! checksum, mapping, probe, and load-clear machinery.
//!
//! the split is deliberate. Generic registration over `T` requires a
//! monomorphizing call site; it never required a netcode crate to own the list of
//! `T`s. A domain invokes a generic trait method, this wrapper performs the GGRS
//! operation, and no gameplay crate gains a `bevy_ggrs` dependency.
//!
//! a wrapper around `&mut App` is required by the orphan rule. The trait
//! lives in the floor and `bevy_app::App` is foreign, so the runtime cannot
//! implement the trait directly for `App`.
//!
//! this file must never grow a list of domains or concrete gameplay types.
//! It owns HOW a rollback declaration is installed, never WHAT the declarations
//! are.

use bevy::ecs::component::Mutable;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::message::Message;
use bevy::prelude::{App, Component, Entity, Resource};

use ambition_platformer2d_core::snapshot::{
    RollbackRegistrar, SnapshotCursor, SnapshotResolve, SnapshotState,
};

use crate::registration::AmbitionRollbackApp as _;

/// A `bevy_ggrs`-backed [`RollbackRegistrar`], borrowed from the host's `App` for
/// the duration of one registration pass.
pub struct GgrsRollbackRegistrar<'a> {
    app: &'a mut App,
}

impl<'a> GgrsRollbackRegistrar<'a> {
    /// Borrow `app` as a registrar a domain can register itself against.
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }
}

impl RollbackRegistrar for GgrsRollbackRegistrar<'_> {
    fn rollback_component_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + SnapshotState,
    {
        self.app.rollback_component_canonical::<T>(owner, name);
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        self.app.rollback_component_cursor::<T>(owner, name);
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
        self.app.rollback_component_resolved::<T>(owner, name);
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app.rollback_component_clone::<T>(owner, name);
        self
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_component_clone_entity_ref::<T>(owner, name, referenced);
        self
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_component_clone_entity_set::<T>(owner, name, referenced);
        self
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<(u64, Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_component_clone_entity_map::<T>(owner, name, referenced);
        self
    }

    fn rollback_component_clone_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_component_clone_probed::<T>(owner, name, projection);
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
        self.app.rollback_component_clone_state::<T>(owner, name);
        self
    }

    fn rollback_component_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        crate::registration::install_component_clone_checksum::<T>(
            self.app,
            owner,
            name,
            format!("bevy_ggrs clone snapshot + {projection}"),
            checksum,
        );
        self
    }

    fn rollback_component_clone_checksum_with_schema_detail<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_component_clone_checksum::<T>(owner, name, detail, checksum);
        self
    }

    fn rollback_resource_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + SnapshotState,
    {
        self.app.rollback_resource_canonical::<T>(owner, name);
        self
    }

    fn rollback_resource_optional_canonical<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + SnapshotState,
    {
        self.app
            .rollback_resource_optional_canonical::<T>(owner, name);
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone,
    {
        self.app.rollback_resource_clone::<T>(owner, name);
        self
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<Entity>,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_resource_clone_entity_set::<T>(owner, name, referenced);
        self
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<Entity>,
        facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_resource_clone_entity_set_probed::<T>(owner, name, referenced, facts);
        self
    }

    fn rollback_resource_clone_checksum<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone,
    {
        crate::registration::install_resource_clone_checksum::<T>(
            self.app,
            owner,
            name,
            format!("bevy_ggrs clone snapshot + {projection}"),
            checksum,
        );
        self
    }

    fn rollback_resource_clone_checksum_with_schema_detail<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        detail: &'static str,
        checksum: for<'a> fn(&'a T) -> u64,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + Clone,
    {
        self.app
            .rollback_resource_clone_checksum::<T>(owner, name, detail, checksum);
        self
    }

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + MapEntities,
    {
        self.app.rollback_map_entities::<T>(owner, name);
        self
    }

    fn rollback_resource_map_entities<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
    ) -> &mut Self
    where
        T: Resource<Mutability = Mutable> + MapEntities,
    {
        self.app.rollback_resource_map_entities::<T>(owner, name);
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        self.app.require_rollback::<T>(owner, name);
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        self.app.clear_message_on_rollback::<T>(owner, name);
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
        self.app
            .declare_rollback_derived_component::<T>(owner, name, reason);
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
        self.app
            .declare_rollback_derived_component_state::<T>(owner, name, reason);
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
        self.app
            .declare_rollback_derived_resource::<T>(owner, name, reason);
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
        self.app
            .declare_rollback_derived_resource_state::<T>(owner, name, reason);
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
        self.app.declare_dynamic_anchor::<T>(owner, name, detail);
        self
    }
}
