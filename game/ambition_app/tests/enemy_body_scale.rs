//! The picture is the body: every character's drawn quad is the size of the
//! box it collides with, and the two render-size publishers agree.
//!
//! The print-only instruments that measured this before it was asserted
//! were retired on 2026-11-21 (git history has them). One printed body
//! boxes in sheet pixels and could not answer the size question. One printed
//! the two publishers, which the test below now asserts over every
//! character. One printed the heights legacy-road rows would author, and
//! all nineteen of those rows author `standing_height` now.

// ─────────────────────────────────────────────────────────────────────────────
// What the RENDERER draws, against the box the body actually collides with.
// ─────────────────────────────────────────────────────────────────────────────

use ambition_app::app::{build_visible_app_with, VisibleRenderMode};
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;
use ambition_platformer2d::assets::character_sprites::{
    sheet_for_character_id_in, sprite_body_collision_for_character_id_in,
};
use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::sprite_sheet::character::sprite_render_size;

/// A spawn rectangle a level might draw around a character — a placement, not a
/// claim about anybody's size.
const LDTK_PLACEMENT: Vec2 = Vec2::new(28.0, 44.0);

/// The TWO independent render-size publishers, for one character.
///
/// So this asks the OTHER publisher the same question about the SAME box:
/// [`sprite_render_size`], which is what `bind_worn_character_presentation` uses
/// for the player and what `upgrade_actor_sprites` uses for every actor whose
/// box was set by content rather than derived here (the AI Slop, a mounted
/// rider, anything a demo resizes). Those two are genuinely different code, and
/// where they disagree the SAME body is drawn at two different sizes depending
/// on which spawn path reached it.
struct Publishers {
    /// The collision box — from the catalog→sheet pipeline.
    body: Vec2,
    /// Quad A: `SpriteBodyCollision::render_size` (the standing-height route).
    catalog_quad: Vec2,
    /// Quad B: `sprite_render_size(spec, body)` — the renderer, given that box.
    render_quad: Vec2,
    /// What the renderer's quad actually DRAWS: the sheet's body rectangle
    /// scaled by the same quad/frame ratio the GPU applies to every pixel.
    render_ink: Vec2,
    /// `render_quad.x/frame_w` over `render_quad.y/frame_h`. Not 1.0  the art
    /// is scaled by different amounts per axis — the stretch this repo has
    /// already paid for once.
    stretch: f32,
}

fn publishers(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent, catalog: &CharacterCatalog, id: &str) -> Option<Publishers> {
    let derived = sprite_body_collision_for_character_id_in(&sprite_content,
        &Default::default(),
        catalog,
        id,
        LDTK_PLACEMENT,
    )?;
    let spec = sheet_for_character_id_in(&sprite_content, &Default::default(), catalog, id)?;
    let frame = spec.frame_pixels();
    let body_px = spec.body_pixel_extent(CharacterAnim::Idle)?;
    let body = Vec2::new(derived.collision.x, derived.collision.y);
    let render_quad = sprite_render_size(&spec, bevy::math::Vec2::new(body.x, body.y));
    let render_quad = Vec2::new(render_quad.x, render_quad.y);
    Some(Publishers {
        body,
        catalog_quad: Vec2::new(derived.render_size.x, derived.render_size.y),
        render_quad,
        render_ink: Vec2::new(
            body_px.x / frame.x * render_quad.x,
            body_px.y / frame.y * render_quad.y,
        ),
        stretch: (render_quad.x / frame.x) / (render_quad.y / frame.y).max(f32::EPSILON),
    })
}

/// The picture is the body, and the two publishers say the same thing.
///
/// this asserts the CORRESPONDENCE, not a size. How tall anybody should be is
/// character's drawing and its hurtbox describe the same creature is not.
///
/// asserted over the whole population rather than a sample, because the
/// defect it pins was never one character: `collision_scale` was a per-sheet
/// fudge, so a spot check on the one that happened to be tuned right reports the
/// success condition.
#[test]
fn every_characters_drawing_is_the_size_of_the_body_it_collides_with() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let app = build_visible_app_with(VisibleRenderMode::NoWindow, true, crate::common::in_the_sandbox_hub);
    let catalog = app.world().resource::<CharacterCatalog>();
    let mut measured = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    let mut ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    ids.sort();
    for id in ids {
        let Some(m) = publishers(&sprite_content, catalog, id) else {
            continue;
        };
        measured += 1;
        // The fit touches on the binding axis and may leave slack on the other,
        // so both are checked for OVERSHOOT and the binding one for reaching 1.
        let (rx, ry) = (m.render_ink.x / m.body.x, m.render_ink.y / m.body.y);
        let publishers_agree =
            (m.render_quad.y / m.catalog_quad.y.max(f32::EPSILON) - 1.0).abs() < 0.01;
        if !(0.99..=1.01).contains(&rx.max(ry))
            || rx > 1.01
            || ry > 1.01
            || (m.stretch - 1.0).abs() > 1e-3
            || !publishers_agree
        {
            wrong.push(format!(
                "  {id}: box {:.1}x{:.1}, drawn {:.1}x{:.1} (x {rx:.2}, y {ry:.2}), \
                 stretch {:.3}, catalog quad {:.1}x{:.1} vs render quad {:.1}x{:.1}",
                m.body.x,
                m.body.y,
                m.render_ink.x,
                m.render_ink.y,
                m.stretch,
                m.catalog_quad.x,
                m.catalog_quad.y,
                m.render_quad.x,
                m.render_quad.y,
            ));
        }
    }
    // SKIP, not fail, with no baked art: sheets are generated and gitignored,
    // so a clean checkout has none.
    if measured == 0 {
        eprintln!("[skip] no baked character sheets — run ./scripts/regen/sprites.sh");
        return;
    }
    assert!(
        wrong.is_empty(),
        "{} of {measured} characters are drawn at a different size from the body \
         they collide with, or the two render-size publishers disagree:\n{}",
        wrong.len(),
        wrong.join("\n"),
    );
}
