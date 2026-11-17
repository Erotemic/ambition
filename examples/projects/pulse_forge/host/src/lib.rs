//! The project composition used by the deterministic promotion fixture.

use ambition_app::app::project::Project;
use ambition_app::{Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::extension::ExtensionAppExt;

pub fn sim(project: &Project, options: Platformer2dSimHarnessOptions) -> Result<Platformer2dSimHarness, String> {
    Platformer2dSimHarness::build(options, |app, options| {
        project.compose(app)?;
        app.add_extension_module(pulse_forge_modules::module());
        ambition_app::rl_sim::ambition_sim_composition(app, options)
    })
}
