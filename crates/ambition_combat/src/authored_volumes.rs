//! App-local authored attack-volume resolution.
//!
//! Combat owns the query shape but not sprite metadata. The composition root
//! inserts an [`AuthoredAttackVolumeResolver`] resource whose function receives
//! the active App-local [`CharacterCatalog`]. This keeps the combat crate
//! content-free without using a process-global install seam: two Bevy `App`s in
//! one process may use different catalogs and resolvers safely.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_core as ae;
use bevy::prelude::Resource;

/// Resolver signature: `(catalog, sprite_character_id, animation clip,
/// collision size, seconds into the clip) -> BODY-LOCAL authored volume`.
/// `sprite_character_id = None` means the provider's default controllable-body
/// row (currently the `player` row for Ambition).
///
/// ## Why BODY-LOCAL, and why no `facing`
///
/// The volume comes back in the frame every authored `HitVolume` is already
/// written in — `+x` toward the body's committed facing, `+y` toward its feet,
/// origin at the body centre — and the caller places it, exactly as it places
/// a synthetic one.
///
/// It used to take a position, a facing and a gravity direction, and the strike
/// path passed `(ZERO, 1.0, down)` to mean *"give it to me unplaced"*. That
/// convention is what let a real mirror go missing: the sheet's own drawn
/// facing was never applied, so a left-drawn character's blade resolved behind
/// her, and no argument at this seam could have said so. The placement terms
/// are gone rather than documented — an argument that must be a specific
/// constant is not an argument.
pub type AuthoredAttackVolumeFn =
    fn(&CharacterCatalog, Option<&str>, &str, ae::Vec2, Option<f32>) -> Option<ae::CombatVolume>;

/// App-local bridge from combat to the linked sprite-metadata implementation.
///
/// ## Why a CLOSURE and not a function pointer
///
/// It was a bare `fn`, which is exactly as much as combat needs to know — and
/// exactly one thing too few. Provider-authored sheets (`AuthoredSheets`) are a
/// second source of the metadata this resolver reads, and a function pointer
/// can carry no state, so the only way to reach them was to widen the signature
/// with a type from `ambition_sprite_sheet`. That would make the combat crate
/// name sprite metadata, which the module docs above say it must not, and they
/// are right: the query shape is combat's, the metadata is not.
///
/// A boxed closure moves the problem to where it belongs. The composition root
/// — which links both crates and therefore may name both — builds a resolver
/// that CAPTURES the authored sheets, and combat calls something it cannot see
/// inside. `Arc` because the resource is cloned into fixtures and must stay
/// `Send + Sync`; rebuilt on change, so a provider registering a sheet after
/// composition is not invisible.
#[derive(Resource, Clone)]
pub struct AuthoredAttackVolumeResolver {
    resolve: std::sync::Arc<
        dyn Fn(
                &CharacterCatalog,
                Option<&str>,
                &str,
                ae::Vec2,
                Option<f32>,
            ) -> Option<ae::CombatVolume>
            + Send
            + Sync,
    >,
}

impl AuthoredAttackVolumeResolver {
    /// A resolver that is a plain function — the shape every fixture uses, and
    /// the one that needs no captured content.
    pub fn new(resolve: AuthoredAttackVolumeFn) -> Self {
        Self {
            resolve: std::sync::Arc::new(resolve),
        }
    }

    /// A resolver that CARRIES content: the composition root captures whatever
    /// registries the implementation needs, and combat stays unable to name them.
    pub fn from_closure(
        resolve: impl Fn(
                &CharacterCatalog,
                Option<&str>,
                &str,
                ae::Vec2,
                Option<f32>,
            ) -> Option<ae::CombatVolume>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            resolve: std::sync::Arc::new(resolve),
        }
    }

    /// A content-free resolver for narrow combat fixtures. Production runtime
    /// composition replaces this with the actor sprite resolver.
    pub fn disabled() -> Self {
        Self::new(no_authored_attack_volume)
    }

    /// The authored volume for this body's current clip, BODY-LOCAL. Place it
    /// with [`ae::CombatVolume::place_body_local`] (or let a spawned hitbox
    /// place it per query, which is what the strike path does).
    pub fn resolve(
        &self,
        catalog: &CharacterCatalog,
        sprite_character_id: Option<&str>,
        animation: &str,
        collision: ae::Vec2,
        clip_elapsed: Option<f32>,
    ) -> Option<ae::CombatVolume> {
        (self.resolve)(catalog, sprite_character_id, animation, collision, clip_elapsed)
    }
}

impl Default for AuthoredAttackVolumeResolver {
    fn default() -> Self {
        Self::disabled()
    }
}

fn no_authored_attack_volume(
    _catalog: &CharacterCatalog,
    _sprite_character_id: Option<&str>,
    _animation: &str,
    _collision: ae::Vec2,
    _clip_elapsed: Option<f32>,
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

    fn catalog_sensitive_resolver(
        catalog: &CharacterCatalog,
        _cid: Option<&str>,
        _animation: &str,
        _collision: ae::Vec2,
        _clip_elapsed: Option<f32>,
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
        alpha.insert_resource(AuthoredAttackVolumeResolver::new(
            catalog_sensitive_resolver,
        ));

        let mut beta = App::new();
        beta.insert_resource(CharacterCatalog::from_data(parse_catalog(BETA)));
        beta.insert_resource(AuthoredAttackVolumeResolver::new(
            catalog_sensitive_resolver,
        ));

        let resolve = |app: &App| {
            app.world()
                .resource::<AuthoredAttackVolumeResolver>()
                .resolve(
                    app.world().resource::<CharacterCatalog>(),
                    None,
                    "attack_side",
                    ae::Vec2::splat(1.0),
                    None,
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
