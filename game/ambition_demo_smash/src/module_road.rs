//! The test harness for a special that is a module of
//! `ambition_demo_smash_modules`: the module on the road a game runs it.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::extension::sdk::{phases, ModuleDescriptor};
use ambition_platformer2d::extension::{ExtensionAppExt as _, ExtensionHostPlugin, ExtensionSet};

/// Where a special comes from: linked into the game, or its file built for
/// `wasm32-unknown-unknown` and loaded, the road a developer edits on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Road {
    Linked,
    Loaded,
}

/// An app that runs `module`'s specials on `road`, with the special and
/// hit-reaction adapters installed and a world clock of 1/60 s. The app is
/// finished; add the game's own systems after.
pub fn app_on(road: Road, module: fn() -> ModuleDescriptor) -> App {
    app_with(road, &[module])
}

/// [`app_on`] for several modules, as a join of two specials runs.
pub fn app_with(road: Road, modules: &[fn() -> ModuleDescriptor]) -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_message::<ambition_platformer2d::vfx::vfx::VfxInRoom>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::combat::hitbox::ParriedBodyHit>();
    app.add_message::<ambition_platformer2d::combat::guard::DeflectedBodyHit>();
    app.add_plugins(ExtensionHostPlugin::new(Update))
        .init_resource::<ambition_platformer2d::time::SimTick>()
        .insert_resource(ambition_platformer2d::time::WorldTime::new(0.0, 1.0 / 60.0));
    ambition_platformer2d::combat::extension::special::install_for_specials(&mut app);
    ambition_platformer2d::abilities::extension::install_for_technique_execution(&mut app);
    ambition_platformer2d::combat::extension::install_for_hit_reaction(&mut app);
    ambition_platformer2d::abilities::extension::install_module_entities(&mut app);
    ambition_platformer2d::combat::extension::blast::install_for_module_entity_tick(&mut app);
    // The shipped order: the module entities are ticked with the items, in
    // the player phase, before the specials of the combat phase; the hits of
    // a tick are resolved after its specials, so an answer pressed in
    // `hit_reaction` is read on the next tick.
    app.configure_sets(
        Update,
        (
            ExtensionSet::Collect(phases::TECHNIQUE_EXECUTION).after(ExtensionSet::Lower(phases::MODULE_ENTITY_TICK)),
            ExtensionSet::Collect(phases::HIT_REACTION).after(ExtensionSet::Lower(phases::TECHNIQUE_EXECUTION)),
        ),
    );
    match road {
        Road::Linked => {
            for module in modules {
                app.add_extension_module(module());
            }
        }
        Road::Loaded => {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let file = ambition_platformer2d::extension::build_module_crate(&root, "ambition_demo_smash_modules")
                .expect("the Smash module crate builds for wasm32-unknown-unknown");
            let (backend, mut loaded) =
                ambition_platformer2d::extension::WasmModules::load(&std::fs::read(file).unwrap())
                    .expect("the module file loads");
            let keys: Vec<_> = modules.iter().map(|module| module().key).collect();
            loaded.retain(|module| keys.contains(&module.key));
            app.add_loaded_extension_modules("ambition_demo_smash_modules.wasm", backend, loaded, false);
        }
    }
    app.finish();
    app
}

/// The runner of the admitted entry `entry`, as its debug text: a loaded
/// file's starts with `Loaded`.
pub fn runner_of(app: &App, entry: &str) -> String {
    let admitted = app.world().resource::<ambition_platformer2d::extension::AdmittedExtensions>();
    let found = admitted.0.entries.iter().find(|admitted| admitted.descriptor.key.as_ref() == entry);
    format!("{:?}", found.unwrap_or_else(|| panic!("the entry `{entry}` is admitted")).runner)
}
