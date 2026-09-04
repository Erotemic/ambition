//! S2: the reusable selected-character presentation binder.
//!
//! Proves the binder derives presentation from the canonical `WornCharacter`
//! identity — generically (two character profiles, no per-character branch),
//! binding on first appearance, rebinding on identity change, and leaving no
//! stale/duplicate sheet components — using deterministic sheet fixtures and
//! nothing from `ambition_app`.
use super::bind_worn_character_presentation;
use super::{PlayerSpriteCharacter, PlayerVisual};
use ambition_characters::actor::WornCharacter;
use ambition_sprite_sheet::character::{
    try_load_spec_for_character_id, CharacterAnimator, CharacterSpriteAsset,
};
use ambition_sprite_sheet::game_assets::GameAssets;
use bevy::prelude::*;

/// A deterministic sheet fixture: a real baked spec for `sheet_root`, with
/// placeholder texture/atlas handles (the binder only clones handles).
fn fixture(sheet_root: &str) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(sheet_root)
        .unwrap_or_else(|| panic!("baked sheet spec exists for '{sheet_root}'"));
    CharacterSpriteAsset {
        texture: Handle::default(),
        layout: Handle::default(),
        spec,
        pages: Vec::new(),
        requested_tier: ambition_persistence::settings::TextureResolutionScale::Full,
        resolved_tier: ambition_persistence::settings::TextureResolutionScale::Full,
    }
}

/// Two distinct character profiles resolve through the SAME binder with no
/// per-character code: "robot" and "goblin" each bind their own sheet and are
/// marked with their own id.
fn two_character_assets() -> GameAssets {
    let mut assets = GameAssets::default();
    assets.characters.publish("robot", fixture("robot"));
    assets.characters.publish("goblin", fixture("goblin"));
    assets
}

fn spawn_worn(app: &mut App, id: &str) -> Entity {
    app.world_mut()
        .spawn((PlayerVisual, WornCharacter::new(id)))
        .id()
}

#[test]
fn binds_on_first_appearance_for_two_profiles() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(two_character_assets());
    app.add_systems(Update, bind_worn_character_presentation);

    let robot = spawn_worn(&mut app, "robot");
    let goblin = spawn_worn(&mut app, "goblin");
    app.update();

    // Each body is bound to ITS OWN identity through the one generic path.
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(robot).unwrap().id,
        "robot"
    );
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(goblin).unwrap().id,
        "goblin"
    );
    // A real sheet resolved → an animator + textured sprite were installed.
    assert!(app.world().get::<CharacterAnimator>(robot).is_some());
    assert!(app.world().get::<Sprite>(goblin).is_some());
    // The two bodies bound DIFFERENT identities through one generic path —
    // the genericity claim (no per-character branch).
    assert_ne!(
        app.world().get::<PlayerSpriteCharacter>(robot).unwrap().id,
        app.world().get::<PlayerSpriteCharacter>(goblin).unwrap().id
    );
    assert!(
        app.world().get::<CharacterAnimator>(goblin).is_some(),
        "the second profile also bound a real sheet animator"
    );
}

#[test]
fn rebinds_and_leaves_no_stale_sheet_components_on_identity_change() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Only "robot" has a sheet; the second identity has NONE, exercising the
    // sheet → fallback rebind (the stale-component path).
    let mut assets = GameAssets::default();
    assets.characters.publish("robot", fixture("robot"));
    app.insert_resource(assets);
    app.add_systems(Update, bind_worn_character_presentation);

    let e = spawn_worn(&mut app, "robot");
    app.update();
    assert!(
        app.world().get::<CharacterAnimator>(e).is_some(),
        "robot binds a real sheet (animator present)"
    );
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(e).unwrap().id,
        "robot"
    );

    // Re-wear to an identity with no sheet: the binder must REPLACE the stale
    // animator/anchor/baseline with the colored-rectangle fallback, not layer
    // a duplicate.
    *app.world_mut().get_mut::<WornCharacter>(e).unwrap() = WornCharacter::new("no_such_sheet");
    app.update();
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(e).unwrap().id,
        "no_such_sheet",
        "the marker follows the new identity"
    );
    assert!(
        app.world().get::<CharacterAnimator>(e).is_none(),
        "the stale sheet animator was removed on rebind (no duplicate/stale state)"
    );
    assert!(
        app.world().get::<super::PlayerSpriteBaseline>(e).is_none(),
        "the stale crouch-squash baseline was removed on rebind"
    );
    assert!(
        app.world().get::<Sprite>(e).is_some(),
        "a fallback sprite is present"
    );
}

#[test]
fn no_game_assets_still_draws_a_marked_fallback() {
    // An art-free demo shell (no GameAssets) must still draw the worn player:
    // the binder installs the colored-rectangle fallback AND marks the identity,
    // so a demo without a sheet never renders an invisible player.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, bind_worn_character_presentation);
    let e = spawn_worn(&mut app, "sanic");
    app.update();
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(e).unwrap().id,
        "sanic",
        "the identity is marked even with no art"
    );
    assert!(
        app.world().get::<Sprite>(e).is_some(),
        "a fallback sprite is drawn with no GameAssets"
    );
    assert!(
        app.world().get::<CharacterAnimator>(e).is_none(),
        "no sheet → no animator, just the rectangle"
    );
}

#[test]
fn already_bound_identity_is_not_rebound() {
    // Non-vacuity: a body correctly bound to its identity (same id AND a real
    // sheet installed) is SKIPPED — the binder does not thrash the sprite every
    // frame. Prove it by advancing the animator's frame cursor and confirming a
    // no-change update preserves it (a rebind would install a fresh frame-0
    // animator).
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(two_character_assets());
    app.add_systems(Update, bind_worn_character_presentation);
    let e = spawn_worn(&mut app, "robot");
    app.update();
    app.world_mut()
        .get_mut::<CharacterAnimator>(e)
        .unwrap()
        .frame = 7;
    app.update();
    assert_eq!(
        app.world().get::<CharacterAnimator>(e).unwrap().frame,
        7,
        "a correctly-bound identity is not rebound, so animator state is preserved"
    );
}

#[test]
fn a_fallback_upgrades_when_its_sheet_appears_later() {
    // The reusable binder must not permanently stick on a fallback: if GameAssets
    // (or the id's sheet) arrives AFTER the first bind, the next run upgrades the
    // marked fallback to the real sheet.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, bind_worn_character_presentation);
    let e = spawn_worn(&mut app, "robot");
    app.update();
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(e).unwrap().id,
        "robot"
    );
    assert!(
        app.world().get::<CharacterAnimator>(e).is_none(),
        "no assets yet → marked fallback, no animator"
    );

    // The sheet loads now.
    app.insert_resource(two_character_assets());
    app.update();
    assert!(
        app.world().get::<CharacterAnimator>(e).is_some(),
        "the fallback upgraded to the real sheet once its asset appeared"
    );
}

/// A body's sprite baseline is its OWN standing size, not a constant.
///
/// `standing_collision` is the reference `sync_visuals` scales the art against
/// (`base_size / standing_collision`), and that ratio exists only for the dev
/// menu's live body-profile experiment. Seeding it from the default player size
/// made the ratio non-1 for any body that simply is not that size, so Mary-O
/// growing to her tall collider stretched the tall sheet's art by 1.5 instead of
/// drawing the tall art at the tall size — her forms have their own SHEETS, and
/// growing must never scale art.
#[test]
fn the_sprite_baseline_records_the_bodys_own_standing_size() {
    let mut app = App::new();
    app.insert_resource(two_character_assets());
    app.add_systems(Update, super::bind_worn_character_presentation);

    let tall = ambition_platformer2d_core::Vec2::new(30.0, 72.0);
    let grown = app
        .world_mut()
        .spawn((
            PlayerVisual,
            WornCharacter::new("robot"),
            // the READ-MODEL, not the sim's `BodyBaseSize`. Presentation reads
            // `ambition_sim_view` (E4), and building the fixture from the live
            // cluster made this test disagree with the system it exercises the
            // moment that rule was enforced.
            ambition_sim_view::BodyPoseView {
                base_size: tall,
                ..Default::default()
            },
        ))
        .id();
    // A body that never states one still binds, on the engine default.
    let plain = spawn_worn(&mut app, "goblin");

    app.update();

    let baseline = app
        .world()
        .get::<super::PlayerSpriteBaseline>(grown)
        .expect("a bound body records a sprite baseline");
    assert_eq!(
        baseline.standing_collision, tall,
        "the baseline is the body's own size, so the render scale is 1 and the \
         tall sheet draws at tall size instead of being stretched"
    );

    let fallback = app
        .world()
        .get::<super::PlayerSpriteBaseline>(plain)
        .expect("a body with no authored baseline still binds");
    assert_eq!(
        fallback.standing_collision,
        ambition_platformer2d_core::Vec2::new(
            ambition_platformer2d_core::DEFAULT_PLAYER_BODY_WIDTH,
            ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT,
        ),
        "and one that states no size falls back to the engine default"
    );
}

/// A trimmed character must never be drawable at its full logical-frame size.
///
/// Regression for the title-shell launch pop: the binder used to insert frame
/// zero's packed atlas rect with the FULL logical `custom_size`, and only the
/// first animation pass applied its trim. That exposed one giant robot frame
/// before `BodyPoseView` existed. Construction now composes the sprite and
/// animator geometry before either becomes drawable.
#[test]
fn a_trimmed_character_is_geometry_complete_on_its_first_drawable_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let asset = fixture("player_robot_v3");
    assert!(
        asset.spec.is_trimmed(),
        "the regression fixture must exercise a packed/trimmed sheet"
    );
    let mut assets = GameAssets::default();
    assets.characters.publish("player_robot_v3", asset);
    app.insert_resource(assets);
    app.add_systems(Update, bind_worn_character_presentation);

    // Deliberately NO BodyPoseView: this is the exact zero-sim-tick state the
    // title shell exposed in the failing launch frame.
    let player = spawn_worn(&mut app, "player_robot_v3");
    app.update();

    let animator = app
        .world()
        .get::<CharacterAnimator>(player)
        .expect("the real sheet installs an animator on the first bind");
    let sprite = app
        .world()
        .get::<Sprite>(player)
        .expect("the first bound frame is drawable");
    let anchor = app
        .world()
        .get::<bevy::sprite::Anchor>(player)
        .expect("the trimmed frame carries its adjusted anchor");
    let baseline = app
        .world()
        .get::<super::PlayerSpriteBaseline>(player)
        .expect("the logical render basis is retained separately");

    let (expected_size, expected_anchor) = animator
        .current_render()
        .expect("a trimmed animator with a seeded basis has current geometry");
    let actual_size = sprite.custom_size.expect("character sprites have an explicit size");

    assert!(
        actual_size.distance(expected_size) < 1.0e-4,
        "the first drawable sprite must already use frame-zero trim: actual={actual_size:?} expected={expected_size:?}"
    );
    assert!(
        anchor.0.distance(expected_anchor) < 1.0e-4,
        "the first drawable sprite must already use frame-zero's trim-adjusted anchor"
    );
    assert!(
        actual_size.distance(baseline.standing_render) > 1.0,
        "this fixture must prove the packed frame is not drawn at the full logical render size"
    );
}
