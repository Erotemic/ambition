//! The T-rex is drawn from his parts: the rig his hurtboxes are posed from.
//!
//! His boss sheet reuses the `trex_enemy` character's art, and that character
//! publishes a part flipbook for every row. A boss is bound by
//! `upgrade_boss_sprites` and drawn from the sim's cursor by `animate_bosses`;
//! the rigged driver follows a `CharacterAnimator`, which a boss did not have,
//! so he was drawn as one baked frame (Jon, 2026-10-06: "It is using part-based
//! rendering right? If not it should."). `BossSheetSpec::parts` names the
//! character whose parts draw a boss sheet, and the cell `animate_bosses`
//! draws poses that character's animator, row by name.
//!
//! ⛔ IN THE SHIPPED VISIBLE COMPOSITION: the rigged driver, the sheet demand
//! and the boss binder are all presentation, which the sim harness has none of.

use std::collections::BTreeSet;

use ambition_app::app::{build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode};
use ambition_platformer2d::render::rendering::actors::rigged::RiggedPresentations;
use ambition_platformer2d::sprite_sheet::boss::{BossAnimator, BossDrawnCell};
use ambition_platformer2d::sprite_sheet::character::CharacterAnimator;
use bevy::prelude::*;

/// Frames the arena gets to boot and the boss to bind his parts.
const BOOT_CAP: usize = 1500;

/// The part-drawn boss root, once the rigged driver owns it.
fn part_drawn_boss(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut bosses = world.query_filtered::<Entity, (With<BossAnimator>, With<CharacterAnimator>)>();
    let boss = bosses.iter(world).next()?;
    world.resource::<RiggedPresentations>().0.contains_key(&boss).then_some(boss)
}

#[test]
fn the_tyrant_is_drawn_from_his_parts_in_the_row_his_fight_draws() {
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(StartRoomOverride("trex_arena".to_string()));
        app.insert_resource(StartRoomMustResolve);
    });
    let mut boss = None;
    for _ in 0..BOOT_CAP {
        app.update();
        boss = part_drawn_boss(&mut app);
        if boss.is_some() {
            break;
        }
    }
    let boss = boss.unwrap_or_else(|| {
        let world = app.world_mut();
        let bound = world.query::<&BossAnimator>().iter(world).count();
        panic!(
            "no T-rex drawn from parts within {BOOT_CAP} frames ({bound} boss sheet(s) bound): \
             his sheet names `npc_trex_enemy`'s parts, which must be demanded, bound and driven"
        )
    });

    // Over a stretch of his fight, every frame: his parts show the row and
    // frame his fight draws, and his root does not draw the baked boss frame
    // under them.
    let mut rows = BTreeSet::new();
    for _ in 0..600 {
        app.update();
        let world = app.world();
        let sheet = world.get::<BossAnimator>(boss).expect("his boss sheet");
        let cell = *world.get::<BossDrawnCell>(boss).expect("the cell his fight draws");
        let animator = world.get::<CharacterAnimator>(boss).expect("his parts' animator");
        let want = sheet.record.rows[cell.row].animation.as_str();
        let shown = animator.drawn_row().and_then(|row| animator.spec.row_name(row));
        assert_eq!(shown, Some(want), "his parts must show the row his fight draws");
        assert_eq!(animator.frame, cell.frame, "his parts must show the frame of `{want}` his fight draws");
        let root = world.get::<Sprite>(boss).expect("his root sprite");
        assert!(
            !sheet.pages.iter().any(|page| page.texture == root.image),
            "his root draws the baked boss frame of `{want}` under his parts"
        );
        rows.insert(want.to_string());
    }
    assert!(rows.len() >= 2, "premise: he moved through more than one row, saw {rows:?}");
}
