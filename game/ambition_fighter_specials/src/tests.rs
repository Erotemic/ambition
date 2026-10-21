use super::*;
use ambition_platformer2d::extension::{ExtensionGeneration, ExtensionHostPlugin};

fn app() -> bevy::prelude::App {
    let mut app = bevy::prelude::App::new();
    app.add_plugins(ExtensionHostPlugin::new(bevy::prelude::Update));
    app
}

/// Every special is declared for its key, owned by its module; a second
/// install, as a second game makes, declares no module again.
#[test]
fn installing_twice_installs_once() {
    let mut app = app();
    install(&mut app);
    let generation = app.world().resource::<ExtensionGeneration>().clone();
    install(&mut app);
    assert_eq!(
        *app.world().resource::<ExtensionGeneration>(),
        generation,
        "the second install declared modules again"
    );
    let support = &app.world().resource::<InstalledTechniques>().0;
    let specials = specials();
    assert_eq!(specials.len(), 14);
    for special in &specials {
        let held = support.offer(special.key).unwrap_or_else(|| panic!("`{}` is not declared", special.key));
        assert_eq!(held.owner, special.offer.owner, "`{}`", special.key);
    }
}

/// A key another owner already declared is still a conflict.
#[test]
#[should_panic(expected = "smash.counter")]
fn a_key_another_owner_declared_is_a_conflict() {
    let mut app = app();
    let mut support = InstalledTechniques::default();
    support
        .0
        .declare(
            catalog::smash_counter::COUNTER,
            action("someone_else", TechniqueParams::None, NestedReferences::None),
        )
        .unwrap();
    app.insert_resource(support);
    install(&mut app);
}
