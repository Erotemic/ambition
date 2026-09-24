//! Tests for the reusable selected-character presentation binder.
//!
//! The binder derives presentation from the canonical `WornCharacter`
//! identity, generically (two character profiles, no per-character branch):
//! it binds on first appearance, rebinds on identity change, and leaves no
//! stale or duplicate sheet components. Uses deterministic sheet fixtures and
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

/// Two character profiles through the same binder with no per-character code:
/// "robot" and "goblin" each bind their own sheet and are marked with their
/// own id.
fn two_character_assets() -> GameAssets {
    let mut assets = GameAssets::default();
    assets.characters.publish("robot", fixture("robot"));
    assets.characters.publish("goblin", fixture("goblin"));
    assets
}

/// A worn player with a published pose: the ordinary shape.
///
/// A sheet-backed presentation is final only once `BodyPoseView` exists: the
/// authored quad and offset come from it, and
/// `CharacterAnimator::render_basis` is initialized once. A fixture with no
/// pose tests the provisional state; see [`spawn_worn_before_its_pose`].
fn spawn_worn(app: &mut App, id: &str) -> Entity {
    app.world_mut()
        .spawn((
            PlayerVisual,
            WornCharacter::new(id),
            ambition_sim_view::BodyPoseView {
                base_size: ambition_platformer2d_core::Vec2::new(
                    ambition_platformer2d_core::DEFAULT_PLAYER_BODY_WIDTH,
                    ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT,
                ),
                ..Default::default()
            },
        ))
        .id()
}

/// A worn player in the window before the sim has published its pose.
fn spawn_worn_before_its_pose(app: &mut App, id: &str) -> Entity {
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

    // Each body is bound to its own identity through the one generic path.
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(robot).unwrap().id,
        "robot"
    );
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(goblin).unwrap().id,
        "goblin"
    );
    // A real sheet resolved, so an animator and textured sprite exist.
    assert!(app.world().get::<CharacterAnimator>(robot).is_some());
    assert!(app.world().get::<Sprite>(goblin).is_some());
    // The two bodies bound different identities through one generic path
    // (no per-character branch).
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
    // Only "robot" has a sheet; the second identity has none, which tests the
    // sheet-to-fallback rebind (the stale-component path).
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

    // Re-wear to an identity with no sheet: the binder must replace the
    // stale animator, anchor, and baseline with the coloured-rectangle
    // fallback, not add a duplicate.
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
    // An art-free demo shell (no `GameAssets`) must still draw the worn
    // player: the binder installs the coloured-rectangle fallback and marks
    // the identity.
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
    // Non-vacuity: a body already bound to its identity (same id and a real
    // sheet) is skipped, so the binder does not rebuild the sprite every
    // frame. Advance the animator's frame cursor and check that an unchanged
    // update keeps it (a rebind would install a new frame-0 animator).
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
    // The binder must not stay on a fallback: if `GameAssets` (or the id's
    // sheet) arrives after the first bind, the next run upgrades the marked
    // fallback to the real sheet.
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

/// A body's sprite baseline is its own standing size, not a constant.
///
/// `standing_collision` is the reference `sync_visuals` scales the art by
/// (`base_size / standing_collision`), a ratio that exists only for the dev
/// menu's live body-profile experiment. Seeded from the default player size,
/// the ratio is not 1 for a body of another size: Mary-O's tall collider
/// would stretch the tall sheet by 1.5. Her forms have their own sheets, so
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
            // The read model, not the sim's `BodyBaseSize`: presentation reads
            // `ambition_sim_view`.
            ambition_sim_view::BodyPoseView {
                base_size: tall,
                ..Default::default()
            },
        ))
        .id();
    // A body whose pose publishes the engine default binds on that.
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
        .expect("a body at the default standing size still binds");
    assert_eq!(
        fallback.standing_collision,
        ambition_platformer2d_core::Vec2::new(
            ambition_platformer2d_core::DEFAULT_PLAYER_BODY_WIDTH,
            ambition_platformer2d_core::DEFAULT_PLAYER_BODY_HEIGHT,
        ),
        "and one that states no size falls back to the engine default"
    );
}

/// A form change and its resize land on different frames, and the binder
/// finalizes on the second.
///
/// Mary-O's `sync_grown_form` swaps the identity in `FeatureInteraction`; the
/// prepared body for the new form arrives at the head of the next tick. The
/// pose between reads `mary_o_tall` with the small form's base (21x32, offset
/// -19.8), then settles a tick later (21x64, offset -4.6). Binding on the
/// swap frame would spend the once-only `CharacterAnimator::render_basis` on
/// the old form.
///
/// The pose says whether its geometry belongs to the worn identity
/// (`BodyPoseView::geometry`). On the swap frame the body keeps its current
/// binding, which still matches its body; the final bind waits for the
/// settled pose.
#[test]
fn a_form_change_binds_the_new_form_only_once_its_geometry_is_settled() {
    let mut app = App::new();
    app.insert_resource(two_character_assets());
    app.add_systems(Update, super::bind_worn_character_presentation);

    // Her real numbers, from `powerups::form_body_size`.
    let small = ambition_platformer2d_core::Vec2::new(21.33, 32.0);
    let tall = ambition_platformer2d_core::Vec2::new(21.33, 64.0);

    let body = app
        .world_mut()
        .spawn((
            PlayerVisual,
            WornCharacter::new("robot"),
            ambition_sim_view::BodyPoseView {
                base_size: small,
                ..Default::default()
            },
        ))
        .id();
    app.update();

    // The frame she grows: the identity swaps, and the pose still carries
    // the old form and says so.
    *app.world_mut().get_mut::<WornCharacter>(body).unwrap() = WornCharacter::new("goblin");
    app.world_mut()
        .get_mut::<ambition_sim_view::BodyPoseView>(body)
        .unwrap()
        .geometry = ambition_sim_view::PoseGeometry::Pending;
    app.update();
    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(body).map(|c| c.id.as_str()),
        Some("robot"),
        "the binder finalized the new identity from the geometry of the old one"
    );
    assert!(
        app.world().get::<CharacterAnimator>(body).is_some(),
        "the body stopped being drawn while its new geometry was pending"
    );

    // The next frame: her new body arrives and the pose settles.
    {
        let mut pose = app
            .world_mut()
            .get_mut::<ambition_sim_view::BodyPoseView>(body)
            .unwrap();
        pose.base_size = tall;
        pose.geometry = ambition_sim_view::PoseGeometry::Settled;
    }
    app.update();

    assert_eq!(
        app.world().get::<PlayerSpriteCharacter>(body).map(|c| c.id.as_str()),
        Some("goblin"),
    );
    let baseline = app
        .world()
        .get::<super::PlayerSpriteBaseline>(body)
        .expect("a bound body records a sprite baseline");
    assert_eq!(
        baseline.standing_collision, tall,
        "the new form was bound against the box it LEFT"
    );
}

/// A trimmed character must never be drawable at its full logical-frame size.
///
/// Construction composes the sprite and animator geometry before either is
/// drawable. Otherwise frame zero's packed atlas rect would show at the full
/// logical `custom_size` before the first animation pass applies the trim
/// (a giant frame at launch, before `BodyPoseView` exists).
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

    // No `BodyPoseView`: the zero-sim-tick state of the first launch frame.
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

/// A worn player is not finally bound until its pose exists.
///
/// `CharacterAnimator::render_basis` is initialized once, and the authored
/// quad and offset for it are on `BodyPoseView`. So a sheet-backed
/// presentation must not be installed before the pose, or the one
/// initialization is spent on a collision-derived guess that the binder's
/// only key (the worn identity) cannot invalidate.
///
/// Before the pose the player is still drawn (the fallback rectangle, marked
/// with the identity) and still eligible, because it has no animator. When
/// the pose lands, the first final basis is the authored one.
#[test]
fn a_worn_player_is_not_finally_bound_until_its_pose_exists() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut assets = GameAssets::default();
    assets
        .characters
        .publish("player_robot_v3", fixture("player_robot_v3"));
    app.insert_resource(assets);
    app.add_systems(Update, bind_worn_character_presentation);

    let player = spawn_worn_before_its_pose(&mut app, "player_robot_v3");
    app.update();

    let basis = |app: &App| {
        app.world()
            .get::<CharacterAnimator>(player)
            .and_then(|a| a.render_basis)
            .map(|b| b.render_size)
    };

    // Not finally bound: no animator has claimed the basis.
    assert!(
        basis(&app).is_none(),
        "a worn player with no pose took the one render basis on a guess"
    );
    // Still drawn and marked: a pre-pose player is never invisible.
    assert!(
        app.world().get::<Sprite>(player).is_some(),
        "the pre-pose player must still draw something"
    );
    assert_eq!(
        app.world()
            .get::<PlayerSpriteCharacter>(player)
            .expect("the pre-pose bind marks the identity")
            .id,
        "player_robot_v3"
    );

    // The pose lands with the sheet's authored quad and offset (what
    // `sync_sprite_posed_bodies` publishes). The worn identity does not
    // change.
    let authored = ambition_platformer2d_core::Vec2::new(70.0, 84.0);
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_sim_view::BodyPoseView {
            size: authored,
            base_size: authored,
            authored_render: Some(authored),
            authored_offset: Some(ambition_platformer2d_core::Vec2::new(0.0, -3.0)),
            ..Default::default()
        });
    app.update();

    let authored_quad = Vec2::new(authored.x, authored.y);
    // Non-vacuity: a collision-derived bind of this body would not give the
    // authored quad, so the two answers differ.
    let (collision_derived, _) = super::character_render_basis(
        &fixture("player_robot_v3").spec,
        authored,
        None,
        None,
    );
    assert!(
        collision_derived.distance(authored_quad) > 1.0,
        "the collision-derived quad ({collision_derived:?}) already equals the \
         authored one, so this fixture cannot witness which one was bound"
    );

    let first_final = basis(&app).expect("the pose makes the sheet bind final");
    assert!(
        first_final.distance(authored_quad) < 1.0,
        "the FIRST final basis is {first_final:?}, not the authored {authored_quad:?} \
         the pose published — the binder spent its one initialization before the \
         authored answer existed"
    );

    // Initialized once: another pass over an unchanged pose keeps the basis.
    app.update();
    assert_eq!(
        basis(&app),
        Some(first_final),
        "the basis must be initialized once; a later pass re-derived it"
    );
}
