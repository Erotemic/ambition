//! The field of a spirit: near each spirit the picture ripples, as air over a
//! fire does, and takes the spirit's colour.
//!
//! A visual says that it is a spirit with a [`SpiritAura`]. Each frame the
//! auras a camera can see are packed for that camera, and a whole-screen pass
//! (`shaders/spirit_field.wgsl`) bends and tints the picture about each one.
//! It is a whole-screen pass because an overlay can not read the picture
//! under it.
//!
//! The pass is polish, and opt-in at two levels. A game adds
//! [`SpiritFieldPlugin`]; and a camera has the pass only while an aura is in
//! its view and the device has a budget for screen shaders, so a room with no
//! spirit pays nothing. A spirit is whole without the field: its glow and its
//! fire are drawn in its sheet.
//!
//! The camera, view and layer rules are the void field's
//! ([`crate::void_field`]), which this pass shares.

use bevy::{
    asset::embedded_asset,
    camera::visibility::RenderLayers,
    core_pipeline::{
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        schedule::Core2d,
        tonemapping::tonemapping,
        Core2dSystems,
    },
    ecs::{schedule::ScheduleConfigs, system::BoxedSystem},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;

use crate::void_field::{drawn_through, nearest, source_in_view, view_in_target};

/// The most auras one camera draws. The nearest to the middle of its view win.
pub const MAX_SPIRIT_SOURCES: usize = 4;

/// On a visual: the field of a spirit about it. `radius` is in world units,
/// from the middle of the visual to where the field ends; `strength` (0 to 1)
/// is how strong it is at its middle; `tint` is the colour it gives the
/// picture (linear 0 to 1).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SpiritAura {
    pub radius: f32,
    pub strength: f32,
    pub tint: Vec3,
}

/// Presentation plugin for the field of a spirit. Add it one time, in an app
/// that renders.
pub struct SpiritFieldPlugin;

impl Plugin for SpiritFieldPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/spirit_field.wgsl");
        app.add_plugins(FullscreenMaterialPlugin::<SpiritFieldSettings>::default())
            .add_systems(PostUpdate, sync_spirit_fields.after(bevy::transform::TransformSystems::Propagate));
    }
}

/// The GPU-facing component of a camera that draws the field. Its presence is
/// what makes Bevy run the pass for the camera.
#[derive(Component, Clone, Copy, Default, ExtractComponent, ShaderType, PartialEq, Debug)]
pub struct SpiritFieldSettings {
    /// x = the number of sources, y = view width over view height,
    /// z = seconds (the ripple moves with it), w = unused.
    pub control: Vec4,
    /// For each source: x, y = its middle in the view (0 to 1, y down);
    /// z = its radius, in view heights; w = its strength.
    pub sources: [Vec4; MAX_SPIRIT_SOURCES],
    /// For each source, in the order of `sources`: its tint (xyz).
    pub tints: [Vec4; MAX_SPIRIT_SOURCES],
    /// The view of the camera in its target, as the void field's.
    pub view: Vec4,
}

impl FullscreenMaterial for SpiritFieldSettings {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/shaders/spirit_field.wgsl".into()
    }

    /// Ambition is a 2D game; the default is `Core3d`.
    fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
        Core2d
    }

    /// After tonemapping, with the other screen filters.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.in_set(Core2dSystems::PostProcess).after(tonemapping)
    }
}

/// Pack the sources of one camera, each with its tint: the nearest to the
/// middle of the view first. `None` with no source.
fn pack(seen: Vec<(Vec4, Vec3)>, aspect: f32, seconds: f32, view: Vec4) -> Option<SpiritFieldSettings> {
    if seen.is_empty() {
        return None;
    }
    // The tint rides in the source's sort: pack the places, then find each
    // place's tint again (a place is a source's own).
    let (sources, count) = nearest::<MAX_SPIRIT_SOURCES>(seen.iter().map(|(source, _)| *source).collect());
    let mut tints = [Vec4::ZERO; MAX_SPIRIT_SOURCES];
    for (k, source) in sources.iter().take(count).enumerate() {
        if let Some((_, tint)) = seen.iter().find(|(s, _)| s == source) {
            tints[k] = tint.extend(1.0);
        }
    }
    Some(SpiritFieldSettings { control: Vec4::new(count as f32, aspect, seconds, 0.0), sources, tints, view })
}

/// Give each gameplay camera the field of the spirits in its view, and take
/// the pass away from a camera that sees none.
#[allow(clippy::type_complexity)]
fn sync_spirit_fields(
    mut commands: Commands,
    time: Res<Time>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    auras: Query<(&GlobalTransform, &SpiritAura, Option<&InheritedVisibility>, Option<&RenderLayers>)>,
    mut cameras: Query<
        (Entity, &Camera, &GlobalTransform, Option<&RenderLayers>, Option<&mut SpiritFieldSettings>),
        With<MainCamera>,
    >,
) {
    let in_budget = quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders());
    // The ripple's clock wraps, so a long session keeps its precision.
    let seconds = time.elapsed_secs_wrapped();
    for (entity, camera, camera_at, camera_layers, current) in &mut cameras {
        let next = camera.logical_viewport_rect().filter(|_| in_budget).and_then(|view_rect| {
            let view_px = view_rect.size();
            let seen = auras
                .iter()
                .filter(|(_, _, visible, layers)| visible.is_none_or(|visible| visible.get()) && drawn_through(camera_layers, *layers))
                .filter_map(|(at, aura, ..)| {
                    let middle = at.translation();
                    let middle_px = camera.world_to_viewport(camera_at, middle).ok()?;
                    let edge_px = camera.world_to_viewport(camera_at, middle + Vec3::X * aura.radius).ok()?;
                    source_in_view(view_px, middle_px - view_rect.min, middle_px.distance(edge_px), aura.strength).map(|source| (source, aura.tint))
                })
                .collect();
            pack(seen, view_px.x / view_px.y, seconds, view_in_target(camera))
        });
        match (next, current) {
            (Some(next), Some(mut current)) => {
                if *current != next {
                    *current = next;
                }
            }
            (Some(next), None) => {
                commands.entity(entity).try_insert(next);
            }
            (None, Some(_)) => {
                commands.entity(entity).try_remove::<SpiritFieldSettings>();
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each packed source keeps its own tint, whatever order the nearest-first
    /// sort puts them in.
    #[test]
    fn each_source_keeps_its_tint() {
        let far = (Vec4::new(0.9, 0.9, 0.2, 1.0), Vec3::new(0.0, 0.0, 1.0));
        let near = (Vec4::new(0.5, 0.5, 0.2, 1.0), Vec3::new(1.0, 0.0, 0.0));
        let packed = pack(vec![far, near], 16.0 / 9.0, 0.0, Vec4::new(0.0, 0.0, 1.0, 1.0)).expect("two sources");
        assert_eq!(packed.control.x, 2.0);
        assert_eq!((packed.sources[0], packed.tints[0].truncate()), (near.0, near.1));
        assert_eq!((packed.sources[1], packed.tints[1].truncate()), (far.0, far.1));
        assert!(pack(Vec::new(), 1.0, 0.0, Vec4::ONE).is_none(), "no spirit, no pass");
    }
}
