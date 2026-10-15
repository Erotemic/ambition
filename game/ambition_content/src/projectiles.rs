//! Ambition's projectile looks: the pack's `data/projectile_visuals.ron`.
//!
//! Every named look Ambition ships (the player fireball and Hadouken kit, the
//! GNU-ton apple rain, the pirate gun-sword discharge, the Perfect Cell-ular
//! Automaton's glider) is a row of that file (schema `projectile_visuals`,
//! owned by the projectile capability). This registers the rows into the
//! App's [`ProjectileVisualCatalog`](ambition_projectiles::ProjectileVisualCatalog),
//! which other providers fill too; a content reload restates Ambition's rows
//! (`reload::publish_projectile_looks`).

use ambition_projectiles::ProjectileVisualAppExt;
use bevy::prelude::{App, Resource};

/// The visual ids Ambition's pack registered: the rows a reload restates, and
/// no other provider's.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionProjectileLooks(pub(crate) Vec<String>);

/// Register every look the selected pack states into the App-local catalog.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let looks = ambition_projectiles::content_schema::lowered_projectile_looks(&pack).cloned().unwrap_or_default();
    for (id, art) in &looks {
        app.register_projectile_visual(id.clone(), art.clone());
    }
    app.insert_resource(AmbitionProjectileLooks(looks.into_keys().collect()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_projectiles::{ProjectileRotation, ProjectileVisualCatalog};

    #[test]
    fn ambition_registers_every_named_projectile_look() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<ProjectileVisualCatalog>();
        for id in [
            "fireball",
            "hadouken",
            "hadouken_super",
            "apple",
            "meatball",
            "trex_rock",
            "honey_glob",
            "red_balloon",
            "thrown_rock",
            "arrow",
            "spit",
            // the look of a shot that names none
            "",
            "eye_beam",
            "mockingbird_missile",
            "mockingbird_fireball",
            "mockingbird_coldfire",
            "mockingbird_lightsaber",
            "mockingbird_lightsaber_cold",
            "lasersword",
            "glider",
        ] {
            assert!(catalog.get(id).is_some(), "{id} must be registered");
        }
        // An unregistered id resolves to the engine's generic hostile shot.
        assert!(catalog.get("unregistered").is_none());
    }

    #[test]
    fn lasersword_owns_its_detonation_and_the_glider_animates_upright() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<ProjectileVisualCatalog>();
        assert!(
            catalog.get("lasersword").unwrap().expiry_vfx.is_some(),
            "the pirate lasersword detonates on expiry"
        );
        let glider = catalog.get("glider").unwrap();
        assert_eq!(glider.rotation, ProjectileRotation::GravityUpright);
    }
}
