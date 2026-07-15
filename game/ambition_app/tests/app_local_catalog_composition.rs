//! Real-provider proof for App-local authored catalogs.
//!
//! This deliberately stops short of activating all three games: the remaining
//! runtime lookup migration is a separate acceptance slice. It proves the hard
//! composition property underneath that work — all linked providers can publish
//! their immutable definitions into one App without plugin-order authority or
//! process-global cross-App contamination.

use ambition::actors::boss_encounter::{BossCatalog, BossCatalogRegistry};
use ambition::actors::features::CharacterRosterRegistry;
use ambition::audio::catalog::AudioCatalogRegistry;
use ambition::characters::actor::character_catalog::{
    CharacterCatalog, CharacterCatalogDefaults, CharacterCatalogOwners,
};
use bevy::prelude::*;

fn register_ambition(app: &mut App) {
    ambition_content::character_catalog::register(app);
    ambition_content::enemy_roster::register(app);
    ambition_content::bosses::register(app);
    ambition_content::audio_registries::register(app);
}

fn register_sanic(app: &mut App) {
    ambition_demo_sanic::install_sanic_content(app);
}

fn register_mary_o(app: &mut App) {
    ambition_demo_mary_o::install_mary_o_content(app);
}

fn character_ids(app: &App) -> Vec<String> {
    app.world()
        .resource::<CharacterCatalog>()
        .iter()
        .map(|(id, _)| id.clone())
        .collect()
}

fn audio_providers(app: &App) -> Vec<String> {
    app.world()
        .resource::<AudioCatalogRegistry>()
        .providers()
        .map(str::to_string)
        .collect()
}

fn hostile_providers(app: &App) -> Vec<String> {
    app.world()
        .resource::<CharacterRosterRegistry>()
        .providers()
        .map(str::to_string)
        .collect()
}

fn boss_providers(app: &App) -> Vec<String> {
    app.world()
        .resource::<BossCatalogRegistry>()
        .providers()
        .map(str::to_string)
        .collect()
}

#[test]
fn three_real_providers_compose_independent_of_registration_order() {
    let mut forward = App::new();
    register_ambition(&mut forward);
    register_sanic(&mut forward);
    register_mary_o(&mut forward);

    let mut reverse = App::new();
    register_mary_o(&mut reverse);
    register_sanic(&mut reverse);
    register_ambition(&mut reverse);

    assert_eq!(character_ids(&forward), character_ids(&reverse));
    assert_eq!(
        forward.world().resource::<CharacterCatalog>(),
        reverse.world().resource::<CharacterCatalog>()
    );
    assert_eq!(audio_providers(&forward), audio_providers(&reverse));
    assert_eq!(hostile_providers(&forward), hostile_providers(&reverse));
    assert_eq!(boss_providers(&forward), boss_providers(&reverse));

    let catalog = forward.world().resource::<CharacterCatalog>();
    for id in ["player", "sanic", "super_sanic", "mary_o"] {
        assert!(
            catalog.get(id).is_some(),
            "missing real provider character {id}"
        );
    }

    let defaults = forward.world().resource::<CharacterCatalogDefaults>();
    assert_eq!(defaults.for_provider("ambition"), Some("player"));
    assert_eq!(defaults.for_provider("sanic"), Some("sanic"));
    assert_eq!(defaults.for_provider("mary_o"), Some("mary_o"));

    let owners = forward.world().resource::<CharacterCatalogOwners>();
    assert_eq!(owners.provider_for("player"), Some("ambition"));
    assert_eq!(owners.provider_for("sanic"), Some("sanic"));
    assert_eq!(owners.provider_for("mary_o"), Some("mary_o"));

    let audio = forward.world().resource::<AudioCatalogRegistry>();
    assert!(audio.music_for("ambition").is_some());
    assert!(audio.sfx_for("ambition").is_some());
    assert!(audio.music_for("sanic").is_some());
    assert!(audio.sfx_for("sanic").is_some());
    assert!(audio.music_for("mary_o").is_none());
    audio
        .combined_music_registry("ambition")
        .expect("real provider music ids must compose without collision");

    // Ambition owns its enemy roster; Mary-O owns the crony's. Both are their own
    // App-local hostile-roster providers (BTreeMap-sorted), composed independent of
    // registration order. Sanic authors no hostile roster, so it adds none.
    assert_eq!(hostile_providers(&forward), vec!["ambition", "mary_o"]);
    assert_eq!(boss_providers(&forward), vec!["ambition"]);
    let bosses = forward.world().resource::<BossCatalog>();
    assert!(bosses.behavior("clockwork_warden").is_some());
    assert!(bosses.encounter("clockwork_warden").is_some());
}

#[test]
fn separate_apps_select_independent_provider_sets() {
    let mut sanic = App::new();
    register_sanic(&mut sanic);

    let mut mary_o = App::new();
    register_mary_o(&mut mary_o);

    let sanic_catalog = sanic.world().resource::<CharacterCatalog>();
    assert!(sanic_catalog.get("sanic").is_some());
    assert!(sanic_catalog.get("mary_o").is_none());

    let mary_o_catalog = mary_o.world().resource::<CharacterCatalog>();
    assert!(mary_o_catalog.get("mary_o").is_some());
    assert!(mary_o_catalog.get("sanic").is_none());

    assert!(sanic
        .world()
        .resource::<AudioCatalogRegistry>()
        .music_for("sanic")
        .is_some());
    assert!(mary_o
        .world()
        .get_resource::<AudioCatalogRegistry>()
        .is_none());

    // Sanic authors no hostile roster, so its App never inits that registry.
    assert!(sanic
        .world()
        .get_resource::<CharacterRosterRegistry>()
        .is_none());

    // Mary-O authors one (its crony), and it is App-local: the registry holds
    // only Mary-O's own provider, with zero contamination from Ambition's roster.
    let mary_o_roster = mary_o
        .world()
        .get_resource::<CharacterRosterRegistry>()
        .expect("Mary-O content publishes its own hostile roster (the crony)");
    assert_eq!(
        mary_o_roster.providers().collect::<Vec<_>>(),
        vec!["mary_o"]
    );

    for app in [&sanic, &mary_o] {
        assert!(app.world().get_resource::<BossCatalogRegistry>().is_none());
        assert!(app.world().get_resource::<BossCatalog>().is_none());
    }
}
