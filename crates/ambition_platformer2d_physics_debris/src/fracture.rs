use ambition_platformer2d_core::AabbExt;
use ambition_sprite_sheet::character::CharacterSpriteAsset;
use ambition_sprite_sheet::prop_sheets::FractureLook;
use ambition_vfx::vfx::{BreakMotion, Fracture};
use bevy::prelude::*;

use super::{BVec2, DebrisPiece};

pub(super) fn prepare(
    asset: &CharacterSpriteAsset,
    material: FractureLook,
    event: &Fracture,
    count: usize,
) -> Option<Vec<DebrisPiece>> {
    let row = asset.spec.clip_slot(["fracture"])?;
    let count = count.min(asset.spec.frame_count_at(row));
    (0..count)
        .map(|index| {
            let trim = asset.spec.frame_trim_at(row, index);
            let logical = trim.logical.as_vec2();
            let size = trim.trimmed.as_vec2();
            let pivot = (trim.offset.as_vec2() + size * 0.5) / logical - BVec2::splat(0.5);
            let world_size = event.bounds.half_size() * 2.0;
            let position = event.bounds.center() + pivot * world_size;
            let size = size / logical * world_size;
            if !size.is_finite() || size.min_element() <= 0.0 || !position.is_finite() {
                return None;
            }
            let page = asset
                .pages
                .get(asset.spec.page_of_at(row, index) as usize)?;
            let atlas = TextureAtlas {
                layout: page.layout.clone(),
                index: asset.spec.flat_index_at(row, index),
            };
            let sprite = Sprite {
                image: page.texture.clone(),
                texture_atlas: Some(atlas),
                custom_size: Some(size),
                ..Default::default()
            };
            let (velocity, spin) = launch(event, index, pivot, material.spin);
            Some(DebrisPiece {
                position,
                size,
                sprite,
                velocity,
                spin,
                bounce: material.bounce,
                lifetime: 1.25 + noise(event.seed, index, 2) * 0.35,
            })
        })
        .collect()
}

fn launch(event: &Fracture, index: usize, pivot: BVec2, spin: f32) -> (BVec2, f32) {
    let jitter = noise(event.seed, index, 0) - 0.5;
    let direction = event.direction();
    let scatter = pivot.normalize_or_zero() * (75.0 + 48.0 * jitter);
    let directed = direction * (130.0 + 100.0 * jitter);
    let velocity = match event.motion {
        BreakMotion::Collapse => directed * 0.65 + BVec2::new(scatter.x * 1.45, scatter.y * 0.25),
        _ => directed + scatter - BVec2::Y * 108.0,
    };
    let omega = (0.48 + noise(event.seed, index, 1) * 0.8) * spin;
    let spin = if (index % 2 == 0) ^ (direction.x < 0.0) {
        omega
    } else {
        -omega
    };
    (BVec2::new(velocity.x, -velocity.y), spin)
}

fn noise(seed: u64, index: usize, channel: u64) -> f32 {
    let mut x = seed ^ ((index as u64 * 3 + channel + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((x ^ (x >> 31)) >> 48) as f32 / 65535.0
}
