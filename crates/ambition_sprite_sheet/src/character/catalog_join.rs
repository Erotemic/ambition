//! The catalog row to sheet join: for a character id, which manifest it
//! renders from, and how big the body is inside it.
//!
//! This is the one module in the crate that uses `ambition_characters`, and it
//! is why `Cargo.toml` has that dependency. The join has no content of its own:
//! it reads a `CharacterCatalogData` it is given. But it knows that a sheet
//! belongs to a character, which the rest of this crate does not. So the
//! coupling stays in this one file.
//!
//! The monolith keeps the `*_in` wrappers that adapt the Bevy
//! `CharacterCatalog` resource to the plain data these functions take. The
//! suffix means the catalog is passed as an argument.

use bevy::math::Vec2;

use ambition_characters::actor::character_catalog::CharacterCatalogData;

use super::sheets;
use super::{CharacterAnim, CharacterSheetSpec};
use crate::BodyMetrics;

/// Look up the [`CharacterSheetSpec`] for a catalog `character_id`, from data
/// only:
///
/// 1. The catalog row names the sheet-manifest record (its own `manifest`
///    root, or an explicit `sprite_target` when a character uses another
///    character's sheet) and carries the tuning (`sprite_tuning`:
///    collision_scale / frame_sample_inset / feet-anchor override).
/// 2. Ids without a catalog row load the manifest by id with default tuning
///    ([`sheets::try_load_spec_for_character_id`]).
///
/// Per-character tuning is a `character_catalog.ron` edit.
///
/// Returns `None` only when no manifest exists for the id (usually the
/// renderer has not run for that target). The actor then renders the
/// colored-rectangle placeholder.
pub fn sheet_for_character_id_from_data(
    // Check provider-authored sheets before the baked cache. They are passed
    // in, not global, so two Apps in one process do not share art.
    authored: &sheets::AuthoredSheets,
    catalog: &CharacterCatalogData,
    character_id: &str,
) -> Option<CharacterSheetSpec> {
    if let Some(entry) = catalog.characters.get(character_id) {
        if let Some(target) = entry.manifest_target() {
            let tuning = entry
                .sprite_tuning
                .map(|spec| {
                    sheets::SheetTuning::from_parts(
                        spec.collision_scale,
                        spec.frame_sample_inset,
                        spec.feet_anchor_y,
                    )
                })
                .unwrap_or_default();
            if let Some(spec) = sheets::try_load_spec_for_target_authored(authored, target, &tuning)
            {
                return Some(spec);
            }
        }
    }
    let spec = sheets::try_load_spec_for_character_id(character_id);
    if spec.is_none() {
        bevy::log::debug!(
            target: "ambition_platformer2d::character_sprites",
            "character_sprites: no sheet manifest for catalog id '{character_id}' — \
             actor will render the colored-rectangle placeholder",
        );
    }
    spec
}

/// Collision footprint from a character's published sprite body metrics, and
/// the render-quad size that keeps the sprite the same size as the legacy
/// `collision_scale` render.
///
/// `render_size` is what [`sheets::sprite_render_size`] produces. The caller
/// stores it so the renderer draws the sprite at that size while the
/// collision box is the body. Do not derive `body * collision_scale` again;
/// that scales twice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteBodyCollision {
    pub collision: Vec2,
    pub render_size: Vec2,
}

/// Pixel-space extent of the visible body in the sheet's standing frame.
///
/// Uses `pose_body_bbox`, which prefers the per-animation `idle` hurtbox, as
/// the sheet-authored actor route (`posed_body_geometry`) does.
fn body_pixel_extent(metrics: &BodyMetrics) -> Option<(f32, f32)> {
    metrics.body_pixel_extent(CharacterAnim::Idle)
}

/// Derive a character's collision box from its published sprite body metrics,
/// given the authored LDtk collision (used only to anchor the render scale).
///
/// Returns `None` when the character has no catalog row, no loadable spec, or
/// no published `body_metrics`; the caller then keeps the LDtk bounds. Sprite
/// metadata replaces the spawn box when present (as in the boss
/// `body_metrics` pipeline).
pub fn sprite_body_collision_for_character_id_from_data(
    // The collision box comes from the sheet, so a consumer-authored sheet
    // must reach this. Otherwise a third-party character collides with the
    // engine's default box.
    authored: &sheets::AuthoredSheets,
    catalog: &CharacterCatalogData,
    character_id: &str,
    ldtk_collision: Vec2,
) -> Option<SpriteBodyCollision> {
    let entry = catalog.characters.get(character_id)?;
    let target = entry.manifest_target()?;
    let spec = sheet_for_character_id_from_data(authored, catalog, character_id)?;
    let record = sheets::record_for_sheet_key(target)?;
    let metrics = record.body_metrics.as_ref()?;
    let (body_w, body_h) = body_pixel_extent(metrics)?;
    let frame_w = record.frame_width.max(1) as f32;
    let frame_h = record.frame_height.max(1) as f32;
    // An authored standing height overrides the room's spawn box. Without
    // one, size is `LDtk box x collision_scale x (body / frame)`. With one,
    // scale the frame so the visible body measures `height`, and keep the
    // frame's aspect so the art does not stretch.
    //
    // The LDtk box still decides where a character stands and how much room
    // the level reserves; it does not decide the character's size.
    let standing_height = entry
        .standing_height
        .or_else(|| entry.body_kind.default_standing_height())
        .filter(|height| *height > 0.0);
    // Both branches produce `frame x scale`; the renderer must not apply
    // `collision_scale` again to the resulting collision box.
    let scale = match standing_height {
        Some(height) if body_h > 0.0 => height / body_h,
        // `CharacterBodyKind::default_standing_height` answers only for
        // `Standard` (other body kinds share no height), so these bodies keep
        // the `collision_scale` derivation until a height is authored.
        _ => ldtk_collision.x.max(ldtk_collision.y).max(8.0) * spec.collision_scale / frame_h,
    };
    let render = Vec2::new(frame_w * scale, frame_h * scale);
    // The visible body occupies (body / frame) of that render quad.
    let collision = Vec2::new(body_w / frame_w * render.x, body_h / frame_h * render.y);
    Some(SpriteBodyCollision {
        collision,
        render_size: render,
    })
}
