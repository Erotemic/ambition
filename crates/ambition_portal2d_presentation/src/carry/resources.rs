//! Capture storage and resolution admission. Geometry does not depend on texture quality.

use super::*;

/// Reduce the largest targets first. Zero means a clipped material proxy.
pub(super) fn capture_plan(requested: &[u32], bytes: u64) -> Vec<u32> {
    use std::{cmp::Reverse, collections::BinaryHeap};
    let pixels = bytes / 4;
    const MIN_CAPTURE: u32 = 16;
    let mut sides = requested.to_vec();
    for side in sides.iter_mut().skip((pixels / u64::from(MIN_CAPTURE).pow(2)).min(requested.len() as u64) as usize) { *side = 0; }
    let mut total: u64 = sides.iter().map(|side| u64::from(*side).pow(2)).sum();
    if total <= pixels { return sides; }
    let mut largest: BinaryHeap<_> = sides.iter().enumerate().filter(|(_,side)| **side > MIN_CAPTURE)
        .map(|(index,side)| (*side,Reverse(index))).collect();
    while total > pixels {
        let Some((side,Reverse(index))) = largest.pop() else { break; };
        let smaller = side / 2;
        total -= u64::from(side).pow(2) - u64::from(smaller).pow(2);
        sides[index] = smaller;
        if smaller > MIN_CAPTURE { largest.push((smaller,Reverse(index))); }
    }
    sides
}

pub(super) fn retire_target(slot: &mut CarrySlot, images: &mut Assets<Image>,
    materials: &mut Assets<PortalClipMaterial>, pool: &mut ClipQuadPool,
) {
    if let Some(image) = slot.image.take() {
        pool.forget_image(&image, materials);
        images.remove(image.id());
    }
    slot.side = 0;
}

pub(super) fn replace_target(slot: &mut CarrySlot, side: u32, images: &mut Assets<Image>,
    materials: &mut Assets<PortalClipMaterial>, pool: &mut ClipQuadPool,
) -> Handle<Image> {
    if slot.side == side {
        if let Some(image) = slot.image.as_ref().filter(|image| images.contains(image.id())) { return image.clone(); }
    }
    retire_target(slot, images, materials, pool);
    // No CPU pixels enter the upload queue. The camera supplies every texel.
    let mut image = Image::new_uninit(bevy::render::render_resource::Extent3d {
        width: side, height: side, depth_or_array_layers: 1,
    }, bevy::render::render_resource::TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::default());
    image.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(image);
    slot.side = side;
    slot.image = Some(image.clone());
    image
}

/// Canonical colour marks retain a visible direction through reflection.
pub(super) fn placeholder(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new(bevy::render::render_resource::Extent3d {
        width: 2, height: 2, depth_or_array_layers: 1,
    }, bevy::render::render_resource::TextureDimension::D2,
        vec![40,160,240,255, 240,160,40,255, 20,80,120,255, 120,80,20,255],
        TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::default()))
}
