//! App-local authored attack-volume resolution.
//!
//! Combat owns the query shape but not sprite metadata. The composition root
//! inserts an [`AuthoredAttackVolumeResolver`] resource whose function receives
//! the active App-local [`CharacterCatalog`]. This keeps the combat crate
//! content-free without using a process-global install seam: two Bevy `App`s in
//! one process may use different catalogs and resolvers safely.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_engine_core as ae;
use bevy::prelude::Resource;

/// Resolver signature: `(catalog, sprite_character_id, animation clip, body
/// position, collision size, facing, gravity direction) -> authored volume`.
/// `sprite_character_id = None` means the provider's default controllable-body
/// row (currently the `player` row for Ambition).
pub type AuthoredAttackVolumeFn = fn(
    &CharacterCatalog,
    Option<&str>,
    &str,
    ae::Vec2,
    ae::Vec2,
    f32,
    ae::Vec2,
) -> Option<ae::CombatVolume>;

/// App-local bridge from combat to the linked sprite-metadata implementation.
#[derive(Resource, Clone, Copy)]
pub struct AuthoredAttackVolumeResolver {
    resolve: AuthoredAttackVolumeFn,
}

impl AuthoredAttackVolumeResolver {
    pub const fn new(resolve: AuthoredAttackVolumeFn) -> Self {
        Self { resolve }
    }

    /// A content-free resolver for narrow combat fixtures. Production runtime
    /// composition replaces this with the actor sprite resolver.
    pub const fn disabled() -> Self {
        Self::new(no_authored_attack_volume)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        &self,
        catalog: &CharacterCatalog,
        sprite_character_id: Option<&str>,
        animation: &str,
        body_pos: ae::Vec2,
        collision: ae::Vec2,
        facing: f32,
        gravity_dir: ae::Vec2,
    ) -> Option<ae::CombatVolume> {
        (self.resolve)(
            catalog,
            sprite_character_id,
            animation,
            body_pos,
            collision,
            facing,
            gravity_dir,
        )
    }
}

impl Default for AuthoredAttackVolumeResolver {
    fn default() -> Self {
        Self::disabled()
    }
}

#[allow(clippy::too_many_arguments)]
fn no_authored_attack_volume(
    _catalog: &CharacterCatalog,
    _sprite_character_id: Option<&str>,
    _animation: &str,
    _body_pos: ae::Vec2,
    _collision: ae::Vec2,
    _facing: f32,
    _gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};
    use bevy::prelude::App;

    const ALPHA: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", spritesheet: "alpha.png", manifest: "alpha.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;
    const BETA: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "beta": (
                display_name: "Beta", spritesheet: "beta.png", manifest: "beta.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    #[allow(clippy::too_many_arguments)]
    fn catalog_sensitive_resolver(
        catalog: &CharacterCatalog,
        _cid: Option<&str>,
        _animation: &str,
        _body_pos: ae::Vec2,
        _collision: ae::Vec2,
        _facing: f32,
        _gravity_dir: ae::Vec2,
    ) -> Option<ae::CombatVolume> {
        let x = if catalog.get("alpha").is_some() {
            1.0
        } else if catalog.get("beta").is_some() {
            2.0
        } else {
            return None;
        };
        Some(ae::CombatVolume::aabb(ae::Aabb::new(
            ae::Vec2::new(x, 0.0),
            ae::Vec2::splat(0.5),
        )))
    }

    #[test]
    fn separate_apps_resolve_against_their_own_character_catalog() {
        let mut alpha = App::new();
        alpha.insert_resource(CharacterCatalog::from_data(parse_catalog(ALPHA)));
        alpha.insert_resource(AuthoredAttackVolumeResolver::new(catalog_sensitive_resolver));

        let mut beta = App::new();
        beta.insert_resource(CharacterCatalog::from_data(parse_catalog(BETA)));
        beta.insert_resource(AuthoredAttackVolumeResolver::new(catalog_sensitive_resolver));

        let resolve = |app: &App| {
            app.world()
                .resource::<AuthoredAttackVolumeResolver>()
                .resolve(
                    app.world().resource::<CharacterCatalog>(),
                    None,
                    "attack_side",
                    ae::Vec2::ZERO,
                    ae::Vec2::splat(1.0),
                    1.0,
                    ae::Vec2::new(0.0, 1.0),
                )
                .expect("fixture catalog should resolve")
                .bounds()
        };
        let center_x = |app: &App| {
            let bounds = resolve(app);
            (bounds.min.x + bounds.max.x) * 0.5
        };

        assert_eq!(center_x(&alpha), 1.0);
        assert_eq!(center_x(&beta), 2.0);
    }
}
