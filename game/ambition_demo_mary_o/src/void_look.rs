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

use std::collections::HashSet;

use bevy::prelude::*;

use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::render::rendering::projectile_visuals::{ProjectileVisual, VisualProjectile};
use ambition_platformer2d::render::rendering::FeatureVisual;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, WornCharacter};
use ambition_platformer2d::render::rendering::PlayerVisual;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
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
        .add_systems(
            Update,
            (reconcile_void_auras, ward_her_with_the_quasar).in_set(crate::MaryOPresentationSet),
        );
}

fn aura_of(sprite: &Sprite) -> VoidAura {
    let size = sprite.custom_size.map_or(DEFAULT_VOID_SIZE, |size| size.max_element());
    VoidAura {
        radius: size * AURA_SIZES,
        strength: AURA_STRENGTH,
    }
}

/// Make the aura of each visual what its thing is now: the visual of a void
/// body, and a shot whose look is the void orb, have one the size of their
/// sprite; every other visual has none.
///
/// ⛔ EACH FRAME, AND BOTH WAYS. It gave an aura once and never took it
/// away, so a visual kept its field after its body went, and a
/// sprite that changed size kept its old radius. It also found the body of
/// each visual with a search of all bodies by name: one pass over the
/// bodies now, keyed by live room and id, because two live rooms of one
/// definition have the same ids.
#[allow(clippy::type_complexity)]
pub fn reconcile_void_auras(
    mut commands: Commands,
    bodies: Query<(&FeatureId, &ActorConfig, Option<&InRoomInstance>)>,
    visuals: Query<(Entity, &FeatureVisual, &Sprite, Option<&InRoomInstance>, Option<&VoidAura>)>,
    shots: Query<(Entity, &VisualProjectile, &Sprite, Option<&VoidAura>), With<ProjectileVisual>>,
    views: Query<&ProjectileView>,
) {
    let void: HashSet<(Option<LiveRoomInstance>, &str)> = bodies
        .iter()
        .filter(|(_, config, _)| is_void_brain(&config.brain))
        .map(|(id, _, room)| (room.map(|room| room.0), id.as_str()))
        .collect();
    for (entity, visual, sprite, room, current) in &visuals {
        let wanted = void
            .contains(&(room.map(|room| room.0), visual.id.as_str()))
            .then(|| aura_of(sprite));
        set_aura(&mut commands, entity, current, wanted);
    }
    for (entity, shot, sprite, current) in &shots {
        let wanted = views
            .get(shot.0)
            .is_ok_and(|view| view.visual_id == crate::void_lizard::VOID_ORB_VISUAL)
            .then(|| aura_of(sprite));
        set_aura(&mut commands, entity, current, wanted);
    }
}

/// Write the aura only when it changes.
fn set_aura(commands: &mut Commands, visual: Entity, current: Option<&VoidAura>, wanted: Option<VoidAura>) {
    match (current, wanted) {
        (Some(current), Some(wanted)) if *current == wanted => {}
        (_, Some(wanted)) => {
            commands.entity(visual).try_insert(wanted);
        }
        (Some(_), None) => {
            commands.entity(visual).try_remove::<VoidAura>();
        }
        (None, None) => {}
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

    fn body(world: &mut World, id: &str, brain: &str, room: LiveRoomInstance) -> Entity {
        let config = ActorConfig {
            tuning: Default::default(),
            brain: CharacterBrain::Custom(brain.into()),
            preserves_mirror_symmetry: false,
        };
        world.spawn((FeatureId::new(id), config, InRoomInstance(room))).id()
    }

    fn visual(world: &mut World, id: &str, size: f32, room: LiveRoomInstance) -> Entity {
        let sprite = Sprite {
            custom_size: Some(Vec2::splat(size)),
            ..Default::default()
        };
        world
            .spawn((FeatureVisual { id: id.into() }, sprite, InRoomInstance(room)))
            .id()
    }

    fn reconcile(world: &mut World) {
        use bevy::ecs::system::RunSystemOnce;
        world.run_system_once(reconcile_void_auras).unwrap();
    }

    /// ⭐ R6: the aura of a visual follows its body, of its own live room.
    #[test]
    fn a_visual_has_the_aura_of_its_own_body_now() {
        let mut world = World::new();
        let (a, b) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        let lizard = body(&mut world, "lizard", crate::void_lizard::VOID_LIZARD_BRAIN_KEY, a);
        // Room B is the same definition: the same id, and not a void thing.
        body(&mut world, "lizard", "goomba", b);
        let in_a = visual(&mut world, "lizard", 40.0, a);
        let in_b = visual(&mut world, "lizard", 40.0, b);
        reconcile(&mut world);
        assert_eq!(world.get::<VoidAura>(in_a).map(|aura| aura.radius), Some(40.0 * AURA_SIZES));
        assert_eq!(world.get::<VoidAura>(in_b), None, "the body of the other live room is not void");

        // A bigger sprite: a wider field.
        world.get_mut::<Sprite>(in_a).unwrap().custom_size = Some(Vec2::splat(60.0));
        reconcile(&mut world);
        assert_eq!(world.get::<VoidAura>(in_a).map(|aura| aura.radius), Some(60.0 * AURA_SIZES));

        // Its body goes and the visual stays: its field goes.
        world.despawn(lizard);
        reconcile(&mut world);
        assert_eq!(world.get::<VoidAura>(in_a), None, "the old aura goes with its body");
    }
}
