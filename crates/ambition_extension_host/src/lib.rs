//! The Bevy host for procedural extension modules (fast-iteration I4).
//!
//! The host names no game algorithm. Domains install ports; games declare
//! modules; the host admits the modules against the installed ports, runs
//! their entries in one serial order, stores their body-attached state as
//! rollback state, and gives their requests to the domains that own the
//! facts. See `docs/planning/engine/extension-model.md` and
//! `docs/planning/engine/extension-domain-contracts.md`.
//!
//! ⭐ AN OFFER EXISTS ONLY WITH ITS ADAPTER. [`ExtensionAppExt`] installs a
//! port's offer and the system that answers it in ONE call. So a port key in
//! the admitted offers means that something answers it, and a module that
//! needs a port this composition did not install is refused at admission —
//! not given a silent no-op.
//!
//! Order of use in an App:
//!
//! 1. add [`ExtensionHostPlugin`] (before any port install — it fails closed);
//! 2. domains call `install_extension_*`;
//! 3. games call [`ExtensionAppExt::add_extension_module`], before or after 1;
//! 4. `Plugin::finish` admits. A refusal stops the composition.

mod admission;
mod exec;
pub mod reload;
mod store;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use ambition_extension_sdk::{ModuleDescriptor, Phase, Port, PortRole, API_VERSION};
use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

pub use admission::{
    admit, Admitted, AdmittedEntry, AdmittedSchema, DeclaredModule, EntryRunner, ModuleBackend,
    ModuleCode, PortOffer, Refusal,
};
pub use exec::{
    run_phase, ExtensionFaults, ExtensionInvocations, ExtensionOutbox, FaultRecord,
    InstalledPortCodecs, PendingInvocation, Submitted, Supplier,
};
pub use store::{register_rollback_state, BodyRecords, StoredRecord};

/// The three host-owned steps of one phase. The composition places them in
/// its schedule; the host chains them.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExtensionSet {
    /// The admission backstop. It runs before every phase and does nothing
    /// once the App is admitted.
    Admit,
    /// Trigger adapters queue invocations.
    Collect(Phase),
    /// The host runs the admitted entries.
    Invoke(Phase),
    /// Request adapters give submitted requests to their domains.
    Lower(Phase),
}

/// The sealed admission of this App.
#[derive(Resource, Clone)]
pub struct AdmittedExtensions(pub Arc<Admitted>);

/// What the admission was made from, kept so a reload can re-admit the whole
/// composition (see [`reload`]).
#[derive(Resource, Clone)]
pub struct ExtensionComposition {
    pub(crate) offers: Vec<PortOffer>,
    pub(crate) declared: Vec<DeclaredModule>,
}

/// The schedule the host and its adapters run in.
#[derive(Resource, Clone, Copy)]
struct ExtensionSchedule(Interned<dyn ScheduleLabel>);

/// Offers, suppliers and modules gathered during `Plugin::build`.
#[derive(Resource, Default)]
struct ExtensionInstallation {
    offers: Vec<PortOffer>,
    suppliers: HashMap<ambition_extension_sdk::PortKey, (Supplier, ambition_extension_sdk::EncodeFn)>,
    request_decoders: HashMap<ambition_extension_sdk::PortKey, ambition_extension_sdk::DecodeFn>,
    modules: Vec<DeclaredModule>,
    /// Phases whose systems are already in the schedule.
    phases: BTreeSet<Phase>,
}

/// Adds the extension host to the simulation schedule given here.
pub struct ExtensionHostPlugin {
    pub schedule: Interned<dyn ScheduleLabel>,
}

impl ExtensionHostPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Plugin for ExtensionHostPlugin {
    fn build(&self, app: &mut App) {
        // `init`, not `insert`: a game plugin may declare its modules before
        // the composition adds the host.
        app.insert_resource(ExtensionSchedule(self.schedule))
            .init_resource::<ExtensionInstallation>()
            .init_resource::<ExtensionInvocations>()
            .init_resource::<ExtensionOutbox>()
            .init_resource::<ExtensionFaults>()
            .add_systems(self.schedule, admit_world.in_set(ExtensionSet::Admit));
    }

    /// ⛔ `finish` is the FIRST admission point, not the only one. A runner
    /// calls it, but much of this repository drives `App::update` by hand and
    /// never does (`shared_tangle::app_finalization`), and a hand-finalized App
    /// can call it twice. So admission is idempotent, and the
    /// [`ExtensionSet::Admit`] system is the backstop on the first tick.
    fn finish(&self, app: &mut App) {
        admit_world(app.world_mut());
    }
}

/// Admit the gathered modules against the installed offers, once. A refusal
/// stops the composition.
pub fn admit_world(world: &mut World) {
    if world.contains_resource::<AdmittedExtensions>() {
        return;
    }
    let installation = world
        .remove_resource::<ExtensionInstallation>()
        .expect("ExtensionHostPlugin::build inserted the installation");
    let admitted = match admit(API_VERSION, &installation.offers, &installation.modules) {
        Ok(admitted) => admitted,
        Err(refusals) => {
            let lines: Vec<String> = refusals.iter().map(|r| format!("  - {r}")).collect();
            panic!(
                "extension admission refused this composition:\n{}",
                lines.join("\n")
            );
        }
    };
    if !admitted.replaced.is_empty() {
        info!("extension modules replaced: {:?}", admitted.replaced);
    }
    world.insert_resource(ExtensionComposition {
        offers: installation.offers.clone(),
        declared: installation.modules.clone(),
    });
    world.init_resource::<reload::StagedModuleReplacement>();
    world.insert_resource(InstalledPortCodecs {
        suppliers: Arc::new(installation.suppliers),
        request_decoders: Arc::new(installation.request_decoders),
    });
    world.insert_resource(AdmittedExtensions(Arc::new(admitted)));
}

/// Install ports and declare modules.
pub trait ExtensionAppExt {
    /// Install trigger port `P` in `phase`, with the adapter system that
    /// queues its invocations. The system runs in `ExtensionSet::Collect`.
    fn install_extension_trigger<P: Port, M>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        adapter: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self;

    /// Install observation port `P` in `phase`, with the domain projection
    /// that answers it.
    fn install_extension_observation<P: Port>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        supplier: fn(&World, Entity) -> Option<P::Value>,
    ) -> &mut Self;

    /// Install request port `P` in `phase`, with the adapter system that
    /// gives its submitted requests to the domain. The system runs in
    /// `ExtensionSet::Lower`.
    fn install_extension_request<P: Port, M>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        adapter: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self;

    /// Declare a module. Admission runs in `Plugin::finish`.
    fn add_extension_module(&mut self, module: ModuleDescriptor) -> &mut Self {
        self.add_declared_extension_module(module.into())
    }

    /// Declare the modules of a loaded file. With `replaces`, each one
    /// explicitly replaces a module of the same key that the game links (a
    /// developer's rebuilt module); admission records the replacement.
    fn add_loaded_extension_modules(
        &mut self,
        backend: Arc<dyn ModuleBackend>,
        modules: Vec<ModuleDescriptor>,
        replaces: bool,
    ) -> &mut Self {
        for (index, descriptor) in modules.into_iter().enumerate() {
            self.add_declared_extension_module(DeclaredModule {
                descriptor,
                code: ModuleCode::Loaded {
                    backend: backend.clone(),
                    module: index as u32,
                },
                replaces,
            });
        }
        self
    }

    /// Declare one module with its code.
    fn add_declared_extension_module(&mut self, module: DeclaredModule) -> &mut Self;
}

fn installation(app: &mut App) -> (Interned<dyn ScheduleLabel>, Mut<'_, ExtensionInstallation>) {
    let schedule = app
        .world()
        .get_resource::<ExtensionSchedule>()
        .map(|s| s.0)
        .expect("add ExtensionHostPlugin before installing an extension port or module");
    let installation = app
        .world_mut()
        .get_resource_mut::<ExtensionInstallation>()
        .expect("extension ports and modules are installed during Plugin::build, before admission");
    (schedule, installation)
}

/// The first offer in a phase adds the phase's chain and its executor.
fn ensure_phase(app: &mut App, schedule: Interned<dyn ScheduleLabel>, phase: &Phase) {
    let new = app
        .world_mut()
        .resource_mut::<ExtensionInstallation>()
        .phases
        .insert(phase.clone());
    if !new {
        return;
    }
    app.configure_sets(
        schedule,
        (
            ExtensionSet::Admit,
            ExtensionSet::Collect(phase.clone()),
            ExtensionSet::Invoke(phase.clone()),
            ExtensionSet::Lower(phase.clone()),
        )
            .chain(),
    );
    app.add_systems(
        schedule,
        run_phase(phase.clone()).in_set(ExtensionSet::Invoke(phase.clone())),
    );
}

fn offer<P: Port>(role: PortRole, phase: Phase, owner: &'static str) -> PortOffer {
    assert_eq!(
        P::ROLE,
        role,
        "port {} is a {:?} port, not a {role:?} port",
        P::KEY,
        P::ROLE
    );
    PortOffer {
        key: P::KEY,
        role,
        phase,
        owner,
    }
}

impl ExtensionAppExt for App {
    fn install_extension_trigger<P: Port, M>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        adapter: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self {
        let (schedule, mut installation) = installation(self);
        installation
            .offers
            .push(offer::<P>(PortRole::Trigger, phase.clone(), owner));
        ensure_phase(self, schedule, &phase);
        self.add_systems(schedule, adapter.in_set(ExtensionSet::Collect(phase)))
    }

    fn install_extension_observation<P: Port>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        supplier: fn(&World, Entity) -> Option<P::Value>,
    ) -> &mut Self {
        let (schedule, mut installation) = installation(self);
        installation
            .offers
            .push(offer::<P>(PortRole::Observation, phase.clone(), owner));
        ensure_phase(self, schedule, &phase);
        let mut installation = self.world_mut().resource_mut::<ExtensionInstallation>();
        let erased: Supplier = Arc::new(move |world: &World, scope: Entity| {
            supplier(world, scope).map(|v| Box::new(v) as Box<dyn std::any::Any + Send + Sync>)
        });
        installation
            .suppliers
            .insert(P::KEY, (erased, ambition_extension_sdk::encode_erased::<P>));
        self
    }

    fn install_extension_request<P: Port, M>(
        &mut self,
        phase: Phase,
        owner: &'static str,
        adapter: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self {
        let (schedule, mut installation) = installation(self);
        installation
            .offers
            .push(offer::<P>(PortRole::Request, phase.clone(), owner));
        installation
            .request_decoders
            .insert(P::KEY, ambition_extension_sdk::decode_erased::<P>);
        ensure_phase(self, schedule, &phase);
        self.add_systems(schedule, adapter.in_set(ExtensionSet::Lower(phase)))
    }

    /// A game plugin may run before or after the composition adds the host,
    /// so this needs no schedule. ⚠ An App that declares a module and never
    /// adds the host admits nothing; the module never runs.
    fn add_declared_extension_module(&mut self, module: DeclaredModule) -> &mut Self {
        assert!(
            !self.world().contains_resource::<AdmittedExtensions>(),
            "module {} declared after admission; declare it during Plugin::build",
            module.descriptor.key
        );
        self.world_mut()
            .get_resource_or_init::<ExtensionInstallation>()
            .modules
            .push(module);
        self
    }
}

#[cfg(test)]
mod tests;
