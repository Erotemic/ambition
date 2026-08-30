//! GGRS-backed implementation of Ambition's typed rollback registration vocabulary.
//!
//! The neutral schema contract lives in `ambition_platformer2d_runtime`; this module is the
//! adapter that turns those same descriptors into `bevy_ggrs` snapshot/checksum/mapping
//! machinery.

use std::collections::BTreeSet;

use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy_ggrs::{
    ComponentSnapshotPlugin, LoadWorld, LoadWorldSystems, ResourceSnapshotPlugin, RollbackApp,
};

use ambition_platformer2d_core::snapshot::{
    cursor_checksum, resolved_checksum, state_checksum, SnapshotCursor, SnapshotResolve, SnapshotState,
};
use crate::CanonicalCodecStrategy;
use ambition_platformer2d_runtime::rollback::{
    descriptor, descriptor_owned, record_descriptor, RollbackEntryKind,
    RollbackRegistrationDescriptor,
};

#[derive(Resource, Default)]
struct GgrsInstalledRegistrations(BTreeSet<String>);

/// Record the neutral descriptor and independently decide whether this GGRS
/// backend still owes the corresponding snapshot/checksum installation.
fn should_install_backend(app: &mut App, descriptor: RollbackRegistrationDescriptor) -> bool {
    let stable_name = descriptor.name.clone();
    let _ = record_descriptor(app, descriptor);
    let rollback_host = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::SimulationHost>()
        .copied()
        == Some(ambition_platformer2d_runtime::SimulationHost::Rollback);
    if !rollback_host {
        return false;
    }
    app.world_mut()
        .get_resource_or_insert_with(GgrsInstalledRegistrations::default)
        .0
        .insert(stable_name)
}

fn record_probe(app: &mut App, probe: crate::ChecksumProbe) {
    app.world_mut()
        .get_resource_or_insert_with(crate::RollbackChecksumProbes::default)
        .register(probe);
}

pub(crate) fn install_component_clone_checksum<T>(
    app: &mut App,
    owner: &'static str,
    name: &'static str,
    detail: String,
    checksum: for<'a> fn(&'a T) -> u64,
) where
    T: Component<Mutability = Mutable> + Clone,
{
    if should_install_backend(
        app,
        descriptor_owned::<T>(
            owner,
            name,
            RollbackEntryKind::ComponentCloneCustomChecksum,
            detail,
        ),
    ) {
        RollbackApp::rollback_component_with_clone::<T>(app);
        RollbackApp::checksum_component(app, checksum);
        record_probe(
            app,
            crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                crate::census_with::<T>(world, checksum)
            }),
        );
    }
}

pub(crate) fn install_resource_clone_checksum<T>(
    app: &mut App,
    owner: &'static str,
    name: &'static str,
    detail: String,
    checksum: for<'a> fn(&'a T) -> u64,
) where
    T: Resource + Clone,
{
    if should_install_backend(
        app,
        descriptor_owned::<T>(
            owner,
            name,
            RollbackEntryKind::ResourceCloneCustomChecksum,
            detail,
        ),
    ) {
        RollbackApp::rollback_resource_with_clone::<T>(app);
        RollbackApp::checksum_resource(app, checksum);
        record_probe(
            app,
            crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                crate::census_resource_with::<T>(world, checksum)
            }),
        );
    }
}

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

    /// Clone-snapshot a component that holds an ENTITY REFERENCE, and probe it
    /// through the target's stable sim identity.
    ///
    /// The same snapshot contract as [`Self::rollback_component_clone`] — no GGRS
    /// checksum, because a raw entity id legitimately differs after a load and
    /// putting it in the aggregate would report a desync on every rewind. What it
    /// adds is a VALUE-sensitive localization probe: a restore that puts back the
    /// right number of references and points one of them at a different body changes
    /// this census, and does not change a presence count.
    ///
    /// `referenced` extracts the handle. Pair this with
    /// [`Self::rollback_map_entities`], which is what actually remaps it.
    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> bevy::prelude::Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot a component holding a SET of entity references, probed
    /// through their stable sim identities. The multi-handle twin of
    /// [`Self::rollback_component_clone_entity_ref`].
    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot a component holding a KEYED MAP of entity references,
    /// probed with the key folded in.
    ///
    /// Use this whenever the association between key and entity is itself the state.
    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<(u64, bevy::prelude::Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone;

    /// Clone-snapshot with a projection the LOCALIZER measures and the GGRS
    /// aggregate does not.
    ///
    /// The distinction matters and is easy to lose: `rollback_component_clone_checksum` hands the
    /// same projection to both, which makes any nondeterminism in it a session-wide desync report.
    /// This arm strengthens only the diagnostic.
    fn rollback_component_clone_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        projection: fn(&T) -> u64,
    ) -> &mut Self
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

    /// Canonical snapshot for a resource that legitimately COMES AND GOES.
    ///
    /// [`Self::rollback_resource_canonical`] cannot serve one: it installs
    /// `bevy_ggrs`'s `ResourceChecksumPlugin`, whose system takes `Res<T>` and
    /// therefore panics on any frame the resource is absent — *"Parameter
    /// `Res<'_, ActiveMatch>` failed validation: Resource does not exist"*. The
    /// SNAPSHOT half already handles absence correctly (`ResourceSnapshotPlugin`
    /// maps `(Some(_), None)` to `remove_resource`), so the gap was only ever in
    /// the checksum.
    ///
    /// This supplies a checksum over `Option<T>`: absence hashes to a distinct
    /// constant, so "the match had not activated yet" and "the match activated
    /// with these seats" are different checksums rather than one of them being
    /// unrepresentable. That distinction IS the state for a latch whose whole
    /// job is to exist (AA2 / AC2).
    fn rollback_resource_optional_canonical<T>(
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

    /// Clone-snapshot a RESOURCE holding entity references, probed through their
    /// stable sim identities. The resource twin of
    /// [`Self::rollback_component_clone_entity_set`].
    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Resource + Clone;

    /// The same, plus the fields the entity set cannot see.
    ///
    /// an entity-set probe is silent about everything that is not an
    /// entity, and for a resource that holds both it reports two divergent
    /// values as identical. `ActiveConversation` is the case that found it: the
    /// probe localized the two bodies faithfully while `input_owner` — which
    /// decides whose controls the conversation captures — could differ between
    /// peers with no signal at all.
    ///
    /// `facts` must NOT hash raw entity handles. Those differ across a load by
    /// design, which is the whole reason the entity half goes through stable sim
    /// identities; mixing them in would make every load look like a desync.
    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
        facts: fn(&T) -> u64,
    ) -> &mut Self
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

    /// Declare derived state that is a COMPONENT, and register a presence probe
    /// for it.
    ///
    /// A `declare_rollback_derived` is an assertion about behaviour: "the system
    /// named in `reason` rebuilds this every tick". Nothing checked that assertion,
    /// and one of them was false — `ProjectileOwner` named a healing system whose
    /// query could not see enemy projectiles at all, which cost a day of
    /// bisection and was the equipment oracle's whole divergence. A derived
    /// declaration that lies is worse than no declaration, because it satisfies
    /// the coverage sweep.
    ///
    /// ⛔ IT IS NOT THE WHOLE CONTRACT. A presence census sees a MISSING derived
    /// component; it cannot see one rebuilt with entirely wrong values on the right
    /// number of carriers, and for a singleton derived resource "present" is nearly a
    /// constant. `declare_rollback_derived_component_state` is the value-sensitive
    /// twin, and gameplay-significant derived state should use it. Which of these are
    /// still presence-only is enumerated by
    /// `rollback_exit_oracle::every_presence_only_probe_is_named_with_its_reason`.
    fn declare_rollback_derived_component<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component;

    fn declare_rollback_derived_component_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component + SnapshotState;

    /// Declare derived state that is a RESOURCE, and register a presence probe.
    ///
    /// Same contract and same reason as
    /// [`Self::declare_rollback_derived_component`]. Split only because
    /// `declare_rollback_derived` bounds `T: 'static` and a probe needs to know
    /// whether to look in the component store or the resource store.
    fn declare_rollback_derived_resource<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource;

    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState;

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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        )
        {
            self.add_plugins(ComponentSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_component(self, state_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone + SnapshotCursor,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, cursor_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_cursor::<T>,
                ),
            );
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneResolved,
                "bevy_ggrs clone snapshot + canonical authored-reference checksum projection",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, resolved_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_resolved::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            // PRESENCE only, because this arm's contract is "snapshotted here,
            // value checksummed by some other authoritative projection" — there is
            // no projection to measure. A count still catches a carrier that
            // bevy_ggrs did not put back, which is `PlayerVisual`'s exact failure.
            //
            // It is genuinely weaker, and G2 made that weakness enumerable rather
            // than implied: a presence probe satisfies the F3 coverage test, which
            // compares type NAMES, while saying nothing about the value. If the type
            // has any stable projection at all — including an entity reference's
            // target identity — reach for `rollback_component_clone_entity_ref` instead.
            record_probe(
                self,
                crate::ChecksumProbe::presence_for::<T>(
                    std::any::type_name::<T>(),
                    crate::census_presence::<T>,
                ),
            );
        }
        self
    }

    fn rollback_component_clone_entity_ref<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> bevy::prelude::Entity,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; entity handle remapped, probed through the target's stable sim identity",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            // No GGRS checksum, for the same reason as the plain clone arm: the raw
            // handle differs across a load by design. But the TARGET's identity does
            // not, so localization is not stuck at presence.
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::census_entity_reference::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::census_entity_set::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_component_clone_entity_map<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<(u64, bevy::prelude::Entity)>,
    ) -> &mut Self
    where
        T: Component<Mutability = Mutable> + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; keyed entity MAP remapped, probed with each key folded against its target's stable sim identity",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::census_entity_map::<T>(world, referenced)
                }),
            );
        }
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentClone,
                "bevy_ggrs clone snapshot; value-probed for localization, not in the session checksum",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::census_with::<T>(world, projection)
                }),
            );
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ComponentCloneCanonicalChecksum,
                "bevy_ggrs clone snapshot + canonical checksum; exact Entity/reference values are remapped after load",
            ),
        )
        {
            RollbackApp::rollback_component_with_clone::<T>(self);
            RollbackApp::checksum_component(self, state_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_state::<T>,
                ),
            );
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
        install_component_clone_checksum::<T>(
            self,
            owner,
            name,
            detail.to_string(),
            checksum,
        );
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCanonical,
                "bevy_ggrs canonical codec snapshot + identical canonical checksum projection",
            ),
        )
        {
            self.add_plugins(ResourceSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            RollbackApp::checksum_resource(self, state_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_resource_state::<T>,
                ),
            );
        }
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCanonical,
                "bevy_ggrs canonical codec snapshot + presence-aware canonical checksum projection",
            ),
        )
        {
            self.add_plugins(ResourceSnapshotPlugin::<CanonicalCodecStrategy<T>>::default());
            // Ambition's own checksum system rather than `RollbackApp::checksum_resource`,
            // which installs the `Res<T>` one.
            let update = move |mut commands: Commands,
                               resource: Option<Res<T>>,
                               mut checksum: Query<
                &mut bevy_ggrs::ChecksumPart,
                (
                    Without<bevy_ggrs::RollbackId>,
                    With<bevy_ggrs::ChecksumFlag<T>>,
                ),
            >| {
                const ABSENT: u128 = 0x4142_5345_4E54_u128;
                let part = bevy_ggrs::ChecksumPart(
                    resource.map_or(ABSENT, |value| state_checksum(value.as_ref()) as u128),
                );
                if let Ok(mut existing) = checksum.single_mut() {
                    *existing = part;
                } else {
                    commands.spawn((part, bevy_ggrs::ChecksumFlag::<T>::default()));
                }
            };
            self.add_systems(
                bevy_ggrs::SaveWorld,
                update.in_set(bevy_ggrs::SaveWorldSystems::Checksum),
            );
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_resource_state::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_cursor<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone + SnapshotCursor,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceCloneCursor,
                "bevy_ggrs clone snapshot + canonical mutable-cursor checksum projection",
            ),
        )
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            RollbackApp::checksum_resource(self, cursor_checksum::<T>);
            record_probe(
                self,
                crate::ChecksumProbe::new(
                    std::any::type_name::<T>(),
                    crate::census_resource_cursor::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_clone<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Resource + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; state checksum supplied by another authoritative projection",
            ),
        )
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // Presence only, for the same reason as the component arm: no
            // projection was supplied. 0-or-1 distinguishes "absent after a load"
            // from "present", and nothing else — for a singleton resource that is
            // almost always "present", which is the narrowest a probe gets.
            record_probe(
                self,
                crate::ChecksumProbe::presence_for::<T>(
                    std::any::type_name::<T>(),
                    crate::census_resource_presence::<T>,
                ),
            );
        }
        self
    }

    fn rollback_resource_clone_entity_set<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; entity SET remapped, probed through the targets' stable sim identities",
            ),
        )
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // No GGRS checksum: the raw handles differ across a load by design.
            // The TARGETS' identities do not, so localization is not stuck at
            // presence — which for a singleton resource is very nearly nothing.
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    crate::census_resource_entity_set::<T>(world, referenced)
                }),
            );
        }
        self
    }

    fn rollback_resource_clone_entity_set_probed<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        referenced: fn(&T) -> Vec<bevy::prelude::Entity>,
        facts: fn(&T) -> u64,
    ) -> &mut Self
    where
        T: Resource + Clone,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceClone,
                "bevy_ggrs clone snapshot; entity SET remapped and probed through the targets' stable sim identities, mixed with a projection of the value's non-entity fields",
            ),
        )
        {
            RollbackApp::rollback_resource_with_clone::<T>(self);
            // No GGRS checksum, for the same reason as the plain entity-set arm:
            // the raw handles differ across a load by design. The localization
            // probe carries both halves.
            record_probe(
                self,
                crate::ChecksumProbe::new(std::any::type_name::<T>(), move |world| {
                    let mut census =
                        crate::census_resource_entity_set::<T>(world, referenced);
                    if let Some(value) = world.get_resource::<T>() {
                        census.xor = census.xor.wrapping_add(facts(value));
                    }
                    census
                }),
            );
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
        install_resource_clone_checksum::<T>(self, owner, name, detail.to_string(), checksum);
        self
    }

    fn rollback_map_entities<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = Mutable> + bevy::ecs::entity::MapEntities,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::EntityMapping,
                "bevy_ggrs LoadWorld entity-reference remapping",
            ),
        )
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
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::ResourceEntityMapping,
                "bevy_ggrs LoadWorld resource entity-reference remapping",
            ),
        )
        {
            RollbackApp::update_resource_with_map_entities::<T>(self);
        }
        self
    }

    fn require_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Component,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::RequiredRollback,
                "component presence automatically installs bevy_ggrs::Rollback",
            ),
        )
        {
            RollbackApp::require_rollback::<T>(self);
        }
        self
    }

    fn clear_message_on_rollback<T>(&mut self, owner: &'static str, name: &'static str) -> &mut Self
    where
        T: Message,
    {
        if should_install_backend(
            self,
            descriptor::<T>(
                owner,
                name,
                RollbackEntryKind::MessageClear,
                "clear abandoned-future message buffer in LoadWorld::Mapping",
            ),
        )
        {
            self.add_systems(
                LoadWorld,
                clear_message_channel::<T>.in_set(LoadWorldSystems::Mapping),
            );
        }
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
        self.declare_rollback_derived::<T>(owner, name, reason);
        // PRESENCE, and for a singleton resource that means 0-or-1: it can catch a
        // derived resource that was never rebuilt, and cannot catch one rebuilt
        // WRONG. `declare_rollback_derived_resource_state` is the value-sensitive
        // twin, and gameplay-significant derived state should use it (G2).
        record_probe(
            self,
            crate::ChecksumProbe::derived_for::<T>(
                std::any::type_name::<T>(),
                crate::census_resource_presence::<T>,
            ),
        );
        self
    }

    /// Declare derived state that HAS a canonical projection, so the resimulation
    /// comparison sees a wrongly-rebuilt value and not merely a missing one.
    fn declare_rollback_derived_resource_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Resource + SnapshotState,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        record_probe(
            self,
            crate::ChecksumProbe::derived_value(
                std::any::type_name::<T>(),
                crate::census_resource_state::<T>,
            ),
        );
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
        self.declare_rollback_derived::<T>(owner, name, reason);
        // PRESENCE: catches a derived component nobody rebuilt (which is what
        // `ProjectileOwner`'s broken derived promise actually was), and cannot catch
        // a motion sample rebuilt with entirely wrong values on the right number of
        // entities. `declare_rollback_derived_component_state` is the strong twin.
        record_probe(
            self,
            crate::ChecksumProbe::derived_for::<T>(
                std::any::type_name::<T>(),
                crate::census_presence::<T>,
            ),
        );
        self
    }

    /// Declare a derived COMPONENT that has a canonical projection.
    fn declare_rollback_derived_component_state<T>(
        &mut self,
        owner: &'static str,
        name: &'static str,
        reason: &'static str,
    ) -> &mut Self
    where
        T: Component + SnapshotState,
    {
        self.declare_rollback_derived::<T>(owner, name, reason);
        record_probe(
            self,
            crate::ChecksumProbe::derived_value(
                std::any::type_name::<T>(),
                crate::census_state::<T>,
            ),
        );
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
        should_install_backend(
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
        should_install_backend(
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
