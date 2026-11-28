//! The field of the void: the picture loses its colour, and a part of its
//! light, near each void thing.
//!
//! A visual says that it is a void thing with a [`VoidAura`]. Each frame the
//! auras a camera can see are packed for that camera, and a whole-screen pass
//! (`shaders/void_field.wgsl`) takes the colour out of the picture about
//! each one. It is a whole-screen pass because an overlay can not read the
//! picture under it.
//!
//! A visual with a [`VoidWard`] is the opposite: about it the void takes
//! nothing, and the picture has the colour it had.
//!
//! The pass is opt-in at two levels. A game adds [`VoidFieldPlugin`]; and a
//! camera has the pass only while an aura is in its view and the device has a
//! budget for shaders, so a room with no void thing pays nothing. A void
//! thing is whole without the field: it is a sprite, and the field is light
//! taken from what is near it.

use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};

use crate::screen_pass::{view_in_target, ScreenPass, ScreenPassPlugin, ScreenPassStage};
use crate::screen_pass::fields::{FieldSite, FieldViews, nearest};

/// The most auras one camera draws. The nearest to the middle of its view win.
pub const MAX_VOID_SOURCES: usize = 8;

/// On a visual: the void about it. `radius` is in world units, from the
/// middle of the visual to where the field ends. `strength` is how much of
/// the colour it takes at its middle, from 0 to 1.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VoidAura {
    pub radius: f32,
    pub strength: f32,
}

/// The most wards one camera draws.
pub const MAX_VOID_WARDS: usize = 4;

/// On a visual: a light that the void can not take. Within `radius` world
/// units of the middle of the visual the field of each [`VoidAura`] is less
/// by `strength` (0 to 1): the picture keeps its colour there.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VoidWard {
    pub radius: f32,
    pub strength: f32,
}

/// How much of the light a field takes at its middle. The colour goes first;
/// the dark is a small part of the look.
const VOID_DARKEN: f32 = 0.28;

/// Run condition of [`sync_void_fields`]: an aura or a ward stands, or a camera
/// still carries a field to take away.
fn a_void_or_its_field_stands(
    auras: Query<(), With<VoidAura>>,
    wards: Query<(), With<VoidWard>>,
    fields: Query<(), With<VoidFieldSettings>>,
) -> bool {
    !auras.is_empty() || !wards.is_empty() || !fields.is_empty()
}

/// Presentation plugin for the field of the void. Add it one time, in an app
/// that renders.
pub struct VoidFieldPlugin;

impl Plugin for VoidFieldPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/void_field.wgsl");
        app.add_plugins(ScreenPassPlugin::<VoidFieldSettings>::default())
            .add_systems(
                PostUpdate,
                sync_void_fields
                    .run_if(a_void_or_its_field_stands)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(bevy::camera::CameraUpdateSystems)
                    .after(crate::rendering::view_isolation::isolate_live_rooms),
            );
    }
}

/// The GPU-facing component of a camera that draws the field. Its presence is
/// what makes Bevy run the pass for the camera.
#[derive(Component, Clone, Copy, Default, ExtractComponent, ShaderType, PartialEq, Debug)]
pub struct VoidFieldSettings {
    /// x = the number of sources, y = view width over view height,
    /// z = how much light the void takes at its middle, w = the number of
    /// wards.
    pub control: Vec4,
    /// For each source: x, y = its middle in the view (0 to 1, y down);
    /// z = its radius, in view heights; w = its strength.
    pub sources: [Vec4; MAX_VOID_SOURCES],
    /// For each ward, as for a source.
    pub wards: [Vec4; MAX_VOID_WARDS],
    /// The view of the camera in its target: x, y = its top left, z, w = its
    /// size, each as a part of the target (0 to 1).
    ///
    /// ⛔ The pass draws on the whole target, and a camera can draw to a part
    /// of it (a window with bars at its sides: `apply_gameplay_camera_viewport`).
    /// The sources are in the view, so the shader must know where the view
    /// is. With no such field, the field of a void thing was at the place the
    /// thing has in a view as large as the window, and went across the screen
    /// at another speed than the thing (Jon, 2026-10-11: "the void star void
    /// field isn't actually on the void star, it lags behind it"). A capture
    /// has no bars and did not show it.
    pub view: Vec4,
}

impl ScreenPass for VoidFieldSettings {
    const SHADER: &'static str = "embedded://ambition_render/shaders/void_field.wgsl";
    const STAGE: ScreenPassStage = ScreenPassStage::Void;
}

/// Pack the sources and the wards of one camera: the nearest to the middle
/// of the view first, and no more than the shader has places for. `None`
/// with no source: a ward has nothing to stop.
fn pack_sources(seen: Vec<Vec4>, wards: Vec<Vec4>, aspect: f32, view: Vec4) -> Option<VoidFieldSettings> {
    if seen.is_empty() {
        return None;
    }
    let (sources, count) = nearest::<MAX_VOID_SOURCES>(seen);
    let (wards, ward_count) = nearest::<MAX_VOID_WARDS>(wards);
    Some(VoidFieldSettings {
        control: Vec4::new(count as f32, aspect, VOID_DARKEN, ward_count as f32),
        sources,
        wards,
        view,
    })
}

/// Give each gameplay camera the field of the auras in its view, and take the
/// pass away from a camera that sees none.
fn sync_void_fields(
    mut commands: Commands,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    auras: Query<(&VoidAura, FieldSite)>,
    wards: Query<(&VoidWard, FieldSite)>,
    views: FieldViews,
    mut settings: Query<&mut VoidFieldSettings>,
) {
    // A visual that is hidden has no field. A visual that is only out of the
    // view keeps its field: the field is wider than the visual.
    let in_budget = quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders());
    for (entity, view) in views.iter() {
        let next = view.filter(|_| in_budget).and_then(|view| {
            let seen = auras.iter()
                .filter_map(|(aura, site)| views.project(&view, site, aura.radius, aura.strength))
                .collect();
            let wards = wards.iter()
                .filter_map(|(ward, site)| views.project(&view, site, ward.radius, ward.strength))
                .collect();
            pack_sources(seen, wards, view.aspect(), view_in_target(view.camera))
        });
        match (next, settings.get_mut(entity).ok()) {
            (Some(next), Some(mut current)) => {
                if *current != next {
                    *current = next;
                }
            }
            (Some(next), None) => {
                commands.entity(entity).try_insert(next);
            }
            (None, Some(_)) => {
                commands.entity(entity).try_remove::<VoidFieldSettings>();
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen_pass::fields::source_in_view;
    use crate::screen_pass::WHOLE_TARGET;
    use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
    use bevy::camera::visibility::RenderLayers;
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
    use bevy::ecs::system::RunSystemOnce;

    /// A main camera at the world origin, 640 by 360, on `layers`, with no
    /// window or render app. `ComputedCameraValues` is public for this.
    fn camera(world: &mut World, layers: Option<RenderLayers>) -> Entity {
        let mut camera = Camera::default();
        camera.computed = ComputedCameraValues {
            clip_from_view: Mat4::orthographic_rh(-320.0, 320.0, -180.0, 180.0, -1000.0, 1000.0),
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(640, 360),
                scale_factor: 1.0,
            }),
            ..default()
        };
        let mut entity = world.spawn((camera, GlobalTransform::IDENTITY, MainCamera));
        if let Some(layers) = layers {
            entity.insert(layers);
        }
        entity.id()
    }

    /// ⭐ A camera that draws to a part of its target (a window with bars at
    /// its sides) has the field of a thing at the middle of its view at the
    /// middle of the view, not one bar to the side.
    #[test]
    fn a_thing_at_the_middle_of_a_view_with_bars_is_a_source_at_the_middle() {
        let mut world = World::new();
        let entity = camera(&mut world, None);
        let mut camera = world.get_mut::<Camera>(entity).unwrap();
        camera.computed.clip_from_view = Mat4::orthographic_rh(-220.0, 220.0, -180.0, 180.0, -1000.0, 1000.0);
        camera.viewport = Some(bevy::camera::Viewport {
            physical_position: UVec2::new(100, 0),
            physical_size: UVec2::new(440, 360),
            ..default()
        });
        at_the_middle(&mut world, AURA, None);
        world.run_system_once(sync_void_fields).unwrap();
        let field = world.get::<VoidFieldSettings>(entity).expect("the aura is in the view");
        assert_eq!(field.sources[0].truncate().truncate(), Vec2::splat(0.5), "{field:?}");
        assert_eq!(field.view, Vec4::new(100.0 / 640.0, 0.0, 440.0 / 640.0, 1.0));
    }

    fn at_the_middle(world: &mut World, thing: impl Bundle, layers: Option<RenderLayers>) {
        let mut entity = world.spawn((GlobalTransform::IDENTITY, thing));
        if let Some(layers) = layers {
            entity.insert(layers);
        }
    }

    const AURA: VoidAura = VoidAura { radius: 60.0, strength: 1.0 };
    const WARD: VoidWard = VoidWard { radius: 60.0, strength: 1.0 };

    /// (auras, wards) a camera's pass has, or `None` with no pass.
    fn field(world: &World, camera: Entity) -> Option<(usize, usize)> {
        world
            .get::<VoidFieldSettings>(camera)
            .map(|field| (field.control.x as usize, field.control.w as usize))
    }

    /// ⭐ R7: two cameras on two layers, and an aura and a ward of each room
    /// at the same pixel. Each camera has the field of its own room only.
    #[test]
    fn a_camera_has_only_the_field_of_the_things_it_draws() {
        let mut world = World::new();
        let (a, b) = (RenderLayers::layer(5), RenderLayers::layer(6));
        let camera_a = camera(&mut world, Some(a.clone()));
        let camera_b = camera(&mut world, Some(b.clone()));
        // The control: a camera and a thing with no layers, both on layer 0.
        let camera_world = camera(&mut world, None);
        at_the_middle(&mut world, AURA, Some(a.clone()));
        at_the_middle(&mut world, AURA, Some(a));
        at_the_middle(&mut world, AURA, Some(b.clone()));
        at_the_middle(&mut world, WARD, Some(b));
        at_the_middle(&mut world, AURA, None);
        world.run_system_once(sync_void_fields).unwrap();
        assert_eq!(field(&world, camera_a), Some((2, 0)), "room A: its two auras, no ward");
        assert_eq!(field(&world, camera_b), Some((1, 1)), "room B: its aura and its ward");
        assert_eq!(field(&world, camera_world), Some((1, 0)), "layer 0: the aura with no layers");
    }

    /// A camera that draws no void thing has no pass, though another room's
    /// void is at the same place.
    #[test]
    fn a_camera_that_draws_no_void_thing_has_no_pass() {
        let mut world = World::new();
        let camera_a = camera(&mut world, Some(RenderLayers::layer(5)));
        at_the_middle(&mut world, AURA, Some(RenderLayers::layer(6)));
        world.run_system_once(sync_void_fields).unwrap();
        assert_eq!(field(&world, camera_a), None);
    }

    #[cfg(feature = "card_view")]
    #[test]
    fn a_card_output_projects_the_field_in_its_view_and_keeps_the_room_mask() {
        use crate::rendering::card_view::{CardViewCamera, PlaneCameraForCard};
        let mut world = World::new();
        let room_layer = RenderLayers::layer(5);
        let plane = camera(&mut world, Some(room_layer.clone()));
        let card = camera(&mut world, Some(RenderLayers::layer(7)));
        world.get_mut::<Camera>(plane).unwrap().is_active = false;
        let at = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(0.4)));
        world.entity_mut(card).remove::<MainCamera>().insert((CardViewCamera, PlaneCameraForCard(plane), at));
        world.get_mut::<Camera>(card).unwrap().viewport = Some(bevy::camera::Viewport {
            physical_position: UVec2::new(80, 20),
            physical_size: UVec2::new(480, 320),
            ..default()
        });
        at_the_middle(&mut world, AURA, Some(RenderLayers::layer(6)));
        world.run_system_once(sync_void_fields).unwrap();
        assert_eq!(field(&world, card), None);

        let site = Vec3::new(80.0, 20.0, 10.0);
        let aura = VoidAura { radius: 8000.0, ..AURA };
        world.spawn((aura, GlobalTransform::from_translation(site), room_layer));
        world.run_system_once(sync_void_fields).unwrap();
        let camera = world.get::<Camera>(card).unwrap();
        let settings = world.get::<VoidFieldSettings>(card).unwrap();
        let rect = camera.logical_viewport_rect().unwrap();
        let expected = (camera.world_to_viewport(&at, site).unwrap() - rect.min) / rect.size();
        assert!(camera.world_to_viewport(&at, site + Vec3::X * aura.radius).is_err());
        assert!(settings.sources[0].truncate().truncate().abs_diff_eq(expected, 1e-6));
        assert!(settings.sources[0].z > 1.0);
        assert_eq!(field(&world, card), Some((1, 0)));
        assert_eq!(field(&world, plane), None);

        world.get_mut::<Camera>(card).unwrap().is_active = false;
        world.get_mut::<Camera>(plane).unwrap().is_active = true;
        world.run_system_once(sync_void_fields).unwrap();
        assert_eq!(field(&world, card), None);
        assert_eq!(field(&world, plane), Some((1, 0)));
    }

    /// An aura with no part in the view is not a source: a camera that sees
    /// none has no pass.
    #[test]
    fn an_aura_out_of_the_view_is_not_a_source() {
        let view = Vec2::new(640.0, 360.0);
        assert!(source_in_view(view, Vec2::new(-200.0, 180.0), 100.0, 1.0).is_none());
        assert!(source_in_view(view, Vec2::new(-50.0, 180.0), 100.0, 1.0).is_some());
        let seen = source_in_view(view, Vec2::new(320.0, 90.0), 90.0, 0.8).unwrap();
        assert_eq!(seen, Vec4::new(0.5, 0.25, 0.25, 0.8));
    }

    /// More auras than the shader has places for: the nearest to the middle
    /// of the view are kept.
    #[test]
    fn the_sources_nearest_to_the_middle_of_the_view_are_kept() {
        let seen: Vec<Vec4> = (0..12).map(|index| Vec4::new(0.5 + index as f32 * 0.04, 0.5, 0.1, 1.0)).collect();
        let packed = pack_sources(seen, vec![Vec4::new(0.5, 0.5, 0.2, 1.0)], 16.0 / 9.0, WHOLE_TARGET).unwrap();
        assert_eq!(packed.view, WHOLE_TARGET);
        assert_eq!(packed.control.w, 1.0);
        assert_eq!(packed.control.x as usize, MAX_VOID_SOURCES);
        assert!(packed.sources.iter().all(|source| source.x < 0.5 + 8.0 * 0.04));
        // A ward with no void in the view is no pass.
        assert!(pack_sources(Vec::new(), vec![Vec4::ONE], 1.0, WHOLE_TARGET).is_none());
    }
}
