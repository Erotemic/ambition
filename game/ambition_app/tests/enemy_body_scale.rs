//! so this asks the SHEET, at `world_per_pixel = 1.0`, which answers in
//! sheet pixels — the unit the generator works in, and the same choice
//! `hall_scale_spread` makes for the same reason.
//!
//! Run the report:
//!
//! ```text
//! cargo test -p ambition_app --test app_it enemy_body_scale -- --ignored --nocapture
//! ```
//!
//! AND THE REPORT BELOW CANNOT ANSWER THE QUESTION IT WAS POINTED AT
//! . `print_enemy_bodies_against_the_player` asks
//! [`posed_body_geometry`] at `world_per_pixel = 1.0`, so its `collision` column
//! is the sheet's body bbox in sheet pixels and its `render` column is the
//! sheet's FRAME in sheet pixels. Both are facts about a generated `.ron` file.
//! Its `x_vs_p` / `y_vs_p` columns divide the COLLISION column by the player's,
//! across sheets with completely different pixel densities. No change to any
//! sizing code can move a single number in it — only regenerating art can.
//! It was cited three times as the falsifier for the bbox-quad route; it is not
//! one.

use ambition_platformer2d::character_sprites::posed_body_geometry;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

/// The player is the ruler: "too big" is a statement about a RATIO, and this is
/// its denominator.
const REFERENCE: &str = "player_robot_v3";

const SUBJECTS: &[&str] = &["player_robot_v3", "solid_snake", "ai_slop", "mary_o_v2"];

fn measured(target: &str) -> Option<(f32, f32, f32, f32)> {
    let g = posed_body_geometry(target, CharacterAnim::Idle, 1.0)?;
    Some((g.collision.x, g.collision.y, g.render.x, g.render.y))
}

/// The instrument. Prints each subject's body box and drawn quad in sheet
/// pixels, and both as a multiple of the player's.
#[test]
#[ignore]
fn print_enemy_bodies_against_the_player() {
    let Some((ref_w, ref_h, _, _)) = measured(REFERENCE) else {
        println!("[skip] no baked sheet for {REFERENCE} — run ./regen_sprites.sh");
        return;
    };
    println!(
        "{:>18} {:>13} {:>13} {:>8} {:>8}",
        "target", "collision", "render", "x_vs_p", "y_vs_p"
    );
    for target in SUBJECTS {
        match measured(target) {
            Some((cw, ch, rw, rh)) => println!(
                "{target:>18} {:>6.0}x{:<6.0} {:>6.0}x{:<6.0} {:>7.2}x {:>7.2}x",
                cw,
                ch,
                rw,
                rh,
                cw / ref_w,
                ch / ref_h
            ),
            None => println!("{target:>18} (no baked sheet)"),
        }
    }
}

/// The instrument cannot rot silently.
#[test]
fn the_enemy_body_report_is_actually_measuring_something() {
    // SKIP, not fail, when the baked art is absent: sheets are generated and
    // gitignored, so a clean checkout has none, and a source test that goes red
    // because a working tree was not rendered teaches people to ignore it.
    let Some((ref_w, ref_h, _, _)) = measured(REFERENCE) else {
        eprintln!("[skip] no baked character sheets — run ./regen_sprites.sh");
        return;
    };
    assert!(
        ref_w > 0.0 && ref_h > 0.0,
        "the reference body measured {ref_w}x{ref_h}, which is not a measurement"
    );
    for target in SUBJECTS {
        let Some((cw, ch, rw, rh)) = measured(target) else {
            panic!(
                "`{target}` has no baked sheet while `{REFERENCE}` does, so the \
                 report above would print one row and look complete"
            );
        };
        assert!(
            cw > 0.0 && ch > 0.0 && rw > 0.0 && rh > 0.0,
            "`{target}` measured a zero-sized body or quad"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// What the RENDERER draws, against the box the body actually collides with.
// ─────────────────────────────────────────────────────────────────────────────

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::actors::character_sprites::{
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

fn publishers(catalog: &CharacterCatalog, id: &str) -> Option<Publishers> {
    let derived = sprite_body_collision_for_character_id_in(
        &Default::default(),
        catalog,
        id,
        LDTK_PLACEMENT,
    )?;
    let spec = sheet_for_character_id_in(&Default::default(), catalog, id)?;
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

/// The instrument for the bbox-quad route. For every character the shipped
/// host registers: the box it collides with, what each of the two publishers
/// sizes its quad to, and what the renderer's quad actually draws.
///
/// ```text
/// cargo test -p ambition_app --test app_it -- \
///     enemy_body_scale::print_the_two_render_size_publishers --ignored --nocapture
/// ```
#[test]
#[ignore]
fn print_the_two_render_size_publishers() {
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app.world().resource::<CharacterCatalog>();
    let mut ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    ids.sort();

    let mut rows: Vec<(String, Publishers)> = Vec::new();
    for id in ids {
        if let Some(measured) = publishers(catalog, id) {
            rows.push((id.clone(), measured));
        }
    }
    if rows.is_empty() {
        println!("[skip] no baked character sheets — run ./regen_sprites.sh");
        return;
    }
    rows.sort_by(|a, b| {
        (a.1.render_ink.y / a.1.body.y)
            .partial_cmp(&(b.1.render_ink.y / b.1.body.y))
            .unwrap()
    });

    println!(
        "{:>34} {:>13} {:>13} {:>13} {:>13} {:>8} {:>8} {:>8}",
        "character",
        "box",
        "quad(catalog)",
        "quad(render)",
        "drawn(render)",
        "drawn/box",
        "y",
        "stretch"
    );
    for (id, m) in &rows {
        println!(
            "{id:>34} {:>5.1}x{:<7.1} {:>5.1}x{:<7.1} {:>5.1}x{:<7.1} {:>5.1}x{:<7.1} \
             {:>8.2} {:>8.2} {:>8.3}",
            m.body.x,
            m.body.y,
            m.catalog_quad.x,
            m.catalog_quad.y,
            m.render_quad.x,
            m.render_quad.y,
            m.render_ink.x,
            m.render_ink.y,
            m.render_ink.x / m.body.x,
            m.render_ink.y / m.body.y,
            m.stretch,
        );
    }
    let ratios: Vec<f32> = rows
        .iter()
        .map(|(_, m)| m.render_ink.y / m.body.y)
        .collect();
    let lo = ratios.iter().cloned().fold(f32::MAX, f32::min);
    let hi = ratios.iter().cloned().fold(f32::MIN, f32::max);
    let disagree = rows
        .iter()
        .map(|(_, m)| (m.render_quad.y / m.catalog_quad.y.max(f32::EPSILON) - 1.0).abs())
        .fold(0.0f32, f32::max);
    println!(
        "\n{} characters measured.\n  drawn/collided height: {lo:.2} .. {hi:.2} \
         (spread {:.2}x) — 1.00 is 'the picture is the body'.\n  \
         worst disagreement between the two publishers: {:.1}%",
        rows.len(),
        hi / lo.max(f32::EPSILON),
        disagree * 100.0,
    );
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
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app.world().resource::<CharacterCatalog>();
    let mut measured = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    let mut ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    ids.sort();
    for id in ids {
        let Some(m) = publishers(catalog, id) else {
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
        eprintln!("[skip] no baked character sheets — run ./regen_sprites.sh");
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

/// WHAT EACH LEGACY-ROAD CHARACTER MEASURES UNDER ITS OWN SPAWN BOX.
///
/// ⛔⛔ `print_the_two_render_size_publishers` asks every character the same
/// question with `LDTK_PLACEMENT` — a generic humanoid rectangle no boss room
/// uses — and on the legacy road (`body_kind` with no default height) the answer
/// is `body_px × ldtk_max × collision_scale / FRAME_H`, so it scales with the
/// box. Quoting that table as "today's size" for a boss would resize it.
///
/// ⭐ THIS ASKS THE PRODUCTION FUNCTION WITH THE REAL BOX, so the number a row
/// should author is read rather than derived. The boxes are the ones every
/// `.ldtk` under `game/ambition_map_assets` places these characters with,
/// measured 2026-08-28; each of these nineteen has exactly ONE, which is what
/// makes authoring its height a road-swap and not a decision.
#[test]
#[ignore]
fn print_the_height_each_legacy_row_already_has() {
    const BOXES: &[(&str, f32, f32)] = &[
        ("npc_bear_mauler", 48.0, 80.0),
        ("npc_boss", 48.0, 80.0),
        ("npc_dark_lord", 48.0, 80.0),
        ("npc_flying_spaghetti_monster_boss", 48.0, 80.0),
        ("npc_hunny_horror_boss", 48.0, 80.0),
        ("npc_mantis_lancer", 48.0, 80.0),
        ("npc_mockingbird_boss", 48.0, 80.0),
        ("npc_raptor_stalker", 48.0, 80.0),
        ("npc_smart_house", 48.0, 80.0),
        ("npc_smirking_behemoth_boss", 48.0, 80.0),
        ("npc_trex_enemy", 48.0, 80.0),
        ("imperfect_cellular_automaton", 32.0, 48.0),
        ("npc_hand_saint", 32.0, 48.0),
        ("npc_le_beast", 32.0, 48.0),
        ("npc_ninja_heavy", 32.0, 48.0),
        ("npc_puppy_slug_variant2", 32.0, 48.0),
        ("npc_puppy_slug_velvet", 32.0, 48.0),
        ("npc_viking_heavy_shieldmaiden", 32.0, 48.0),
        ("npc_viking_heavy_warrior", 32.0, 48.0),
    ];
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app.world().resource::<CharacterCatalog>();
    println!(
        "{:>36} {:>10} {:>18}",
        "character", "spawn box", "derived w x h"
    );
    for (id, w, h) in BOXES {
        match sprite_body_collision_for_character_id_in(
            &Default::default(),
            catalog,
            id,
            Vec2::new(*w, *h),
        ) {
            Some(derived) => println!(
                "{id:>36} {:>4.0}x{:<5.0} {:>8.2} x {:<8.2}",
                w, h, derived.collision.x, derived.collision.y
            ),
            None => println!("{id:>36} {:>4.0}x{:<5.0}   (no sheet)", w, h),
        }
    }
}
