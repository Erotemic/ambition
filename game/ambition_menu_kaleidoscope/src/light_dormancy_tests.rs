use super::sleep_light_visibility_without_lights;
use bevy::light::{DirectionalLight, PointLight, SimulationLightSystems};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Runs(u32);

/// A member of the light visibility set: it runs when the set runs.
fn count_run(mut runs: ResMut<Runs>) {
    runs.0 += 1;
}

/// The light checks sleep in a world with no light, and wake for a light.
#[test]
fn the_light_checks_run_only_while_a_light_exists() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Runs>();
    app.add_systems(PostUpdate, count_run.in_set(SimulationLightSystems::CheckLightVisibility));
    sleep_light_visibility_without_lights(&mut app);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 0, "the light checks ran with no light");

    let light = app.world_mut().spawn(DirectionalLight::default()).id();
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 1, "a directional light did not wake the light checks");

    app.world_mut().despawn(light);
    app.world_mut().spawn(PointLight::default());
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 2, "a point light did not wake the light checks");
}
