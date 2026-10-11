//! The field of the void about each void thing of Mary-O: the Void Star, the
//! Void Lizard, and the orb the Void Lizard throws.
//!
//! Presentation only. The field is the engine's
//! (`ambition_platformer2d::render::void_field`): this module says which
//! visuals of Mary-O are void things, and how far the field of each goes. A
//! void thing is whole with no field: it is a sprite.
//!
//! Mary-O with the quasar is the light the void can not take: she has a ward
//! about her (`VoidWard`), and the picture near her keeps its colour. She has
//! it in her cosmic form at all times.

use bevy::prelude::*;

use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::render::rendering::projectile_visuals::{ProjectileVisual, VisualProjectile};
use ambition_platformer2d::render::rendering::FeatureVisual;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, WornCharacter};
use ambition_platformer2d::render::rendering::PlayerVisual;
use ambition_platformer2d::render::void_field::{VoidAura, VoidFieldPlugin, VoidWard};
use ambition_platformer2d::sim_view::ProjectileView;

/// The field of a void thing ends this many of its sizes from its middle. It
/// is wide: a field the size of the thing is not seen in play (Jon, 2026-10-11).
const AURA_SIZES: f32 = 4.4;
/// The size of a void thing whose sprite states no size, in world px.
const DEFAULT_VOID_SIZE: f32 = 40.0;
/// How much of the colour the field takes at its middle.
const AURA_STRENGTH: f32 = 1.0;
/// How far the ward of the quasar goes from her middle, in world px.
const QUASAR_WARD_RADIUS: f32 = 150.0;

/// Whether a body with this brain is a void thing.
fn is_void_brain(brain: &CharacterBrain) -> bool {
    matches!(
        brain,
        CharacterBrain::Custom(key)
            if key == crate::void_star::VOID_STAR_BRAIN_KEY || key == crate::void_lizard::VOID_LIZARD_BRAIN_KEY
    )
}

/// Install the field and the system that gives each void thing its aura.
/// Idempotent. An app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.is_plugin_added::<VoidFieldPlugin>() || app.get_sub_app(bevy::render::RenderApp).is_none() {
        return;
    }
    app.add_plugins(VoidFieldPlugin)
        .add_systems(Update, (give_each_void_thing_its_aura, ward_her_with_the_quasar));
}

fn aura_of(sprite: &Sprite) -> VoidAura {
    let size = sprite.custom_size.map_or(DEFAULT_VOID_SIZE, |size| size.max_element());
    VoidAura {
        radius: size * AURA_SIZES,
        strength: AURA_STRENGTH,
    }
}

/// Give its aura to each void visual that has none: the visual of a void
/// body, and a shot whose look is the void orb.
#[allow(clippy::type_complexity)]
fn give_each_void_thing_its_aura(
    mut commands: Commands,
    bodies: Query<(&FeatureId, &ActorConfig)>,
    visuals: Query<(Entity, &FeatureVisual, &Sprite), Without<VoidAura>>,
    shots: Query<(Entity, &VisualProjectile, &Sprite), (With<ProjectileVisual>, Without<VoidAura>)>,
    views: Query<&ProjectileView>,
) {
    for (entity, visual, sprite) in &visuals {
        let is_void = bodies
            .iter()
            .any(|(id, config)| id.as_str() == visual.id && is_void_brain(&config.brain));
        if is_void {
            commands.entity(entity).try_insert(aura_of(sprite));
        }
    }
    for (entity, shot, sprite) in &shots {
        if views
            .get(shot.0)
            .is_ok_and(|view| view.visual_id == crate::void_lizard::VOID_ORB_VISUAL)
        {
            commands.entity(entity).try_insert(aura_of(sprite));
        }
    }
}

/// Give her the ward while she has the quasar, and take it away after.
fn ward_her_with_the_quasar(
    mut commands: Commands,
    bodies: Query<(Entity, &WornCharacter, &BodyHealth, Has<VoidWard>), With<PlayerVisual>>,
) {
    for (body, worn, health, warded) in &bodies {
        // The quasar, or the form that is the quasar worn for good.
        let has_quasar = crate::cosmic::is_cosmic(worn.id())
            || (crate::powerups::is_her_form(worn.id()) && health.health.invulnerable.holds(Invulnerability::EMPOWERED));
        if has_quasar && !warded {
            commands.entity(body).try_insert(VoidWard {
                radius: QUASAR_WARD_RADIUS,
                strength: 1.0,
            });
        } else if !has_quasar && warded {
            commands.entity(body).try_remove::<VoidWard>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_headless_app_installs_no_field() {
        let mut app = App::new();
        install(&mut app);
        assert!(!app.is_plugin_added::<VoidFieldPlugin>());
    }
}
