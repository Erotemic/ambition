//! The test harness for a special that is a module of
//! `ambition_demo_smash_modules`: the module on the road a game runs it.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::extension::sdk::ModuleDescriptor;
use ambition_platformer2d::extension::{ExtensionAppExt as _, ExtensionHostPlugin};

/// Where a special comes from: linked into the game, or its file built for
/// `wasm32-unknown-unknown` and loaded, the road a developer edits on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Road {
    Linked,
    Loaded,
}

/// An app that runs `module`'s specials on `road`, with the special adapters
/// installed and a world clock of 1/60 s. The app is finished; add the
/// game's own systems after.
pub fn app_on(road: Road, module: fn() -> ModuleDescriptor) -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_plugins(ExtensionHostPlugin::new(Update))
        .init_resource::<ambition_platformer2d::time::SimTick>()
        .insert_resource(ambition_platformer2d::time::WorldTime::new(0.0, 1.0 / 60.0));
    ambition_platformer2d::combat::extension::special::install_for_specials(&mut app);
    match road {
        Road::Linked => {
            app.add_extension_module(module());
        }
        Road::Loaded => {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let file = ambition_platformer2d::extension::build_module_crate(&root, "ambition_demo_smash_modules")
                .expect("the Smash module crate builds for wasm32-unknown-unknown");
            let (backend, mut modules) =
                ambition_platformer2d::extension::WasmModules::load(&std::fs::read(file).unwrap())
                    .expect("the module file loads");
            let key = module().key;
            modules.retain(|loaded| loaded.key == key);
            app.add_loaded_extension_modules("ambition_demo_smash_modules.wasm", backend, modules, false);
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
