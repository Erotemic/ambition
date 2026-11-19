//! The lights of a room drawn per pixel on the GPU: what the dark room's veil
//! (`darkness`) and the night's lamps (`daylight`) both read.
//!
//! Each light is a row of a uniform (where it is, how far it reaches, how
//! strong, its colour) and, when its light stops at the room's ground (Ultra),
//! a row of a float texture: how far it reaches along each of its directions
//! ([`super::occlusion::SolidGrid::reach`]). The shader (`lamps.wgsl`) sums
//! them at each pixel as [`super::darkness::lamp_texels`] does at each texel
//! of the CPU light map, so the light's falloff and its shadow's edge are as
//! sharp as the screen, not a 160x90 texture stretched over it.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, Origin3d, ShaderType, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDimension,
    TextureFormat,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;
use bevy::render::{MainWorld, Render, RenderApp, RenderSystems};

use super::darkness::Lamp;

/// The most lights drawn at once. Past it, the nearest the view's centre.
pub const MAX_LAMPS: usize = 64;
/// The most directions a light's reach is measured along
/// (`occlusion::RAYS`): the reach texture's width.
pub const REACH_RAYS: u32 = 256;

/// The lights in view, as the shader reads them.
#[derive(ShaderType, Clone, Debug, PartialEq)]
pub struct LampsUniform {
    /// Per light: its place (x, y, world units), its radius and its intensity.
    pub at: [Vec4; MAX_LAMPS],
    /// Per light: its colour, and how many directions its reach row holds
    /// (0: its light passes through the ground).
    pub color: [Vec4; MAX_LAMPS],
    pub count: u32,
}

impl Default for LampsUniform {
    fn default() -> Self {
        Self { at: [Vec4::ZERO; MAX_LAMPS], color: [Vec4::ZERO; MAX_LAMPS], count: 0 }
    }
}

/// The reach texture: a row of [`REACH_RAYS`] floats a light. Its rows are
/// written into the texture the GPU already has ([`ReachWrites`]), never by
/// changing the image: a changed image is a new texture, and a material that
/// binds it draws nothing until it has bound the new one.
pub fn reach_image() -> Image {
    let mut image = Image::new(
        Extent3d { width: REACH_RAYS, height: MAX_LAMPS as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0; REACH_RAYS as usize * MAX_LAMPS * 4],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

/// The reach rows to write into each reach texture ([`reach_image`]): the
/// latest for each, written once its texture is on the GPU.
#[derive(Resource, Default)]
pub struct ReachWrites(pub HashMap<AssetId<Image>, Vec<u8>>);

/// Writes [`ReachWrites`] into their textures, and loads the shader library
/// (`lamps.wgsl`) the lamps' materials import.
pub struct LampsPlugin;

impl Plugin for LampsPlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "lamps.wgsl");
        app.init_resource::<ReachWrites>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ReachWrites>()
                .add_systems(ExtractSchedule, extract_reach_writes)
                .add_systems(Render, write_reach.in_set(RenderSystems::PrepareResources));
        }
    }
}

fn extract_reach_writes(mut main: ResMut<MainWorld>, mut writes: ResMut<ReachWrites>) {
    if let Some(mut asked) = main.get_resource_mut::<ReachWrites>() {
        if !asked.0.is_empty() {
            writes.0.extend(asked.0.drain());
        }
    }
}

fn write_reach(mut writes: ResMut<ReachWrites>, images: Res<RenderAssets<GpuImage>>, queue: Res<RenderQueue>) {
    // A texture not yet on the GPU keeps its rows for a later frame.
    writes.0.retain(|id, rows| {
        let Some(gpu) = images.get(*id) else {
            return true;
        };
        queue.write_texture(
            TexelCopyTextureInfo { texture: &gpu.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            rows,
            TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(REACH_RAYS * 4), rows_per_image: None },
            Extent3d { width: REACH_RAYS, height: MAX_LAMPS as u32, depth_or_array_layers: 1 },
        );
        false
    });
}

/// The lights of `lamps` that reach into the rectangle about `center` of
/// `size`, at most [`MAX_LAMPS`] (the nearest its centre), as the uniform and,
/// with `reaches` (each lamp's, in its order), the reach texture's bytes.
pub fn pack(lamps: &[Lamp], reaches: Option<&[Vec<f32>]>, center: Vec2, size: Vec2) -> (LampsUniform, Option<Vec<u8>>) {
    let half = size / 2.0;
    let mut seen: Vec<usize> = (0..lamps.len())
        .filter(|&n| {
            let lamp = &lamps[n];
            let r = lamp.radius.max(1.0);
            lamp.intensity > 0.0
                && (lamp.at.x - center.x).abs() < half.x + r
                && (lamp.at.y - center.y).abs() < half.y + r
        })
        .collect();
    if seen.len() > MAX_LAMPS {
        seen.sort_by(|a, b| lamps[*a].at.distance_squared(center).total_cmp(&lamps[*b].at.distance_squared(center)));
        seen.truncate(MAX_LAMPS);
    }
    let mut uniform = LampsUniform { count: seen.len() as u32, ..LampsUniform::default() };
    let mut rows = reaches.map(|_| vec![0u8; REACH_RAYS as usize * MAX_LAMPS * 4]);
    for (slot, &n) in seen.iter().enumerate() {
        let lamp = &lamps[n];
        let reach = reaches.and_then(|reaches| reaches.get(n)).filter(|reach| !reach.is_empty());
        let rays = reach.map_or(0, |reach| reach.len().min(REACH_RAYS as usize));
        uniform.at[slot] = Vec4::new(lamp.at.x, lamp.at.y, lamp.radius.max(1.0), lamp.intensity.clamp(0.0, 1.0));
        uniform.color[slot] = Vec4::new(lamp.color[0], lamp.color[1], lamp.color[2], rays as f32);
        if let (Some(rows), Some(reach)) = (rows.as_mut(), reach) {
            let row = slot * REACH_RAYS as usize * 4;
            for (ray, distance) in reach.iter().take(rays).enumerate() {
                rows[row + ray * 4..row + ray * 4 + 4].copy_from_slice(&distance.to_le_bytes());
            }
        }
    }
    (uniform, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_lights_that_reach_the_view_are_drawn_and_their_reach_with_them() {
        let lamp = |x: f32| Lamp { at: Vec2::new(x, 0.0), radius: 100.0, intensity: 1.0, color: [1.0, 0.5, 0.25] };
        let lamps = [lamp(0.0), lamp(5_000.0), lamp(-450.0)];
        let reaches = vec![vec![10.0; 32], vec![20.0; 32], vec![30.0; 40]];
        let (uniform, rows) = pack(&lamps, Some(&reaches), Vec2::ZERO, Vec2::new(800.0, 600.0));
        assert_eq!(uniform.count, 2, "the far lamp is out of view");
        assert_eq!(uniform.at[1].x, -450.0, "a lamp just past the edge still lights into it");
        assert_eq!(uniform.color[1].w, 40.0);
        let rows = rows.expect("shadowed");
        let at = |slot: usize, ray: usize| {
            let i = (slot * REACH_RAYS as usize + ray) * 4;
            f32::from_le_bytes(rows[i..i + 4].try_into().unwrap())
        };
        assert_eq!((at(0, 31), at(1, 39), at(1, 40)), (10.0, 30.0, 0.0));
    }
}
