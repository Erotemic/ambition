//! The catalog row → sheet join: given a character catalog and the sheets a
//! provider authored, which manifest does an id render from, and how big is the
//! body inside it.
//!
//!  this is the one place in the crate that names `ambition_characters`, and
//! the whole `ambition_characters` edge in `Cargo.toml` exists for it. The join
//! is still content-free — it reads a `CharacterCatalogData` it is handed and
//! owns no catalog of its own — but it does know that a *character* is the thing
//! a sheet belongs to, which the rest of this crate deliberately does not. Keeping
//! it in a module named after the join means the coupling is one file, and a
//! reader asking "why does a sheet crate know about characters" finds the answer
//! here instead of grepping.
//!
//! The monolith keeps the `*_in` wrappers that adapt the Bevy `CharacterCatalog` resource to
//! the plain data these take.
//!
//! The suffix is the promise that the catalog arrives as an argument.

use bevy::math::Vec2;

use ambition_characters::actor::character_catalog::CharacterCatalogData;

use super::sheets;
use super::{CharacterAnim, CharacterSheetSpec};
use crate::BodyMetrics;

/// Look up the [`CharacterSheetSpec`] for a catalog `character_id` —
/// fully DATA-driven:
///
/// 1. The catalog row names the sheet-manifest record (its own
///    `manifest` filename root, or an explicit `sprite_target` when a
///    character renders with another character's sheet) and carries
///    the gameplay tuning (`sprite_tuning`: collision_scale /
///    frame_sample_inset / feet-anchor override).
/// 2. Ids without a catalog row fall back to the manifest-by-id load
///    with default tuning ([`sheets::try_load_spec_for_character_id`]).
///
/// There is no hardcoded `*_SHEET` table behind this — adding a character's
/// bespoke tuning is a `character_catalog.ron` edit.
///
/// Returns `None` only when no manifest exists for the id — usually
/// because the renderer hasn't been run for that target; the actor
/// then renders the colored-rectangle placeholder.
pub fn sheet_for_character_id_from_data(
    // Sheets a PROVIDER authored, consulted before the engine's baked cache.
    // Threaded rather than reached for globally: two Apps in one process must
    // not share one game's art declarations.
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

/// Collision footprint derived from a character's *published sprite body
/// metrics*, plus the render-quad size that keeps the on-screen sprite
/// identical to the legacy `collision_scale` render.
///
/// `render_size` is exactly what [`sheets::sprite_render_size`] produces today —
/// the caller stores it so the renderer draws the sprite at its current size even
/// though the collision box shrank to the body. (The renderer's `collision_scale`
/// path assumes `collision == visible body`; once the collision IS the body, the
/// render must come from the stored size rather than re-deriving
/// `body * collision_scale`, which double-scales.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteBodyCollision {
    pub collision: Vec2,
    pub render_size: Vec2,
}

/// Pixel-space extent of the visible body in the sheet's standing frame.
///
/// It read the static `body_pixel_bbox` where the sheet-authored actor route
/// (`posed_body_geometry`) reads `pose_body_bbox`, which prefers the per-animation `idle` hurtbox.
/// One reader now, in the crate that owns the metadata.
fn body_pixel_extent(metrics: &BodyMetrics) -> Option<(f32, f32)> {
    metrics.body_pixel_extent(CharacterAnim::Idle)
}

/// Derive a character's collision box from its published sprite body metrics,
/// given the authored LDtk collision (used only to anchor the render scale).
///
/// Returns `None` when the character has no catalog row, no loadable spec, or
/// no published `body_metrics` — the caller then keeps the LDtk bounds. This
/// is the "sprite metadata supersedes the spawn box when present, else fall
/// back to LDtk" rule (matching the boss `body_metrics` pipeline, generalized
/// to ordinary catalog characters).
pub fn sprite_body_collision_for_character_id_from_data(
    // A body's collision box is DERIVED from its sheet, so a consumer-authored
    // sheet has to reach this or a third party's character renders from its own
    // art and collides with the engine's default box.
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
    //  an authored STANDING HEIGHT overrides the room's spawn box. Without
    // one, size is `LDtk box x collision_scale x (body / frame)` — two
    // per-character guesses and a rectangle drawn in a level editor, none of
    // which is a claim about how tall anybody is. With one, the height IS the
    // input and everything else follows the sheet: scale the frame so the
    // visible body measures `height`, and keep the frame's aspect so the art is
    // never stretched.
    //
    //  the LDtk box still decides where a character STANDS and how much room a
    // level reserved for it; it stops deciding how big the character is.
    let standing_height = entry
        .standing_height
        .or_else(|| entry.body_kind.default_standing_height())
        .filter(|height| *height > 0.0);
    // Both branches produce `frame x scale`; the renderer must not apply
    // `collision_scale` again to the resulting collision box.
    let scale = match standing_height {
        Some(height) if body_h > 0.0 => height / body_h,
        // `CharacterBodyKind::default_standing_height` deliberately answers for
        // `Standard` only — *"a crawler, a floating drone and a wide body have no
        // shared height to be consistent about"* — so this population keeps the
        // derivation it was left with, and `collision_scale` keeps doing exactly
        // this one job until somebody authors a height for them.
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
