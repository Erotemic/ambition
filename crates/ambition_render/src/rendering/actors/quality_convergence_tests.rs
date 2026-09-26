//! A live quality change must reach the bodies already on screen.
//!
//! `ambition_platformer2d_actor_monolith::character_runtime` guards the engine
//! side: retire a stale realization and re-materialize it at the applied tier.
//! This file guards what a participant sees: a body bound to the old
//! realization rebinds to the new one on the same entity, keeps its identity,
//! and the old image is freed.
//!
//! A rebind that works only on the frame `GameAssets` changed is not
//! convergence. The new pages load through `asset_server.load`, so they arrive
//! some frames later, after the one-frame `is_changed()` window.

use bevy::prelude::*;

use ambition_persistence::settings::{TextureResolutionScale, VisualQualityProfile};
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_sprite_sheet::character::{CharacterSpriteAsset, CharacterSpritePage};
use ambition_sprite_sheet::game_assets::GameAssets;

use super::{BoundSpriteQuality, PlayerSpriteCharacter};
use crate::quality::ResolvedVisualQuality;
use crate::rendering::primitives::{FeatureVisual, PlayerVisual};

const ACTOR_ID: &str = "probe_actor";
const ACTOR_NAME: &str = "Probe Actor";
const PLAYER_ID: &str = "player_robot_v3";

fn asset_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    app
}

fn quality(profile: VisualQualityProfile) -> ResolvedVisualQuality {
    ResolvedVisualQuality {
        profile,
        budget: ambition_persistence::settings::VisualQualityBudget::for_profile(profile),
    }
}

/// A realization at `tier` whose page-0 image is reserved but not present.
///
/// This is a fresh `asset_server.load` before decode finishes. The binders skip
/// a sheet whose texture is not present, so a fixture with the image already
/// present would test only the same-frame case.
fn a_pending_realization(app: &mut App, tier: TextureResolutionScale) -> CharacterSpriteAsset {
    use ambition_sprite_sheet::character::sheets::{try_load_spec_for_target, SheetTuning};

    let spec = try_load_spec_for_target("robot", &SheetTuning::new(1.0, 1))
        .expect("the baked `robot` sheet record is present");
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .reserve_handle();
    let layout = app
        .world_mut()
        .resource_mut::<Assets<bevy::image::TextureAtlasLayout>>()
        .add(bevy::image::TextureAtlasLayout::new_empty(
            bevy::math::UVec2::splat(128),
        ));
    CharacterSpriteAsset {
        texture: texture.clone(),
        layout: layout.clone(),
        spec,
        pages: vec![CharacterSpritePage { texture, layout }],
        // The fixture gets the tier it asks for: these tests are about
        // convergence, not fallback.
        requested_tier: tier,
        resolved_tier: tier,
    }
}

/// The decode finishes.
fn the_image_lands(app: &mut App, asset: &CharacterSpriteAsset) {
    let id = asset.texture.id();
    let _ = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(id, Image::default());
}

fn a_feature_view() -> ambition_sim_view::FeatureView {
    ambition_sim_view::FeatureView {
        pos: ambition_platformer2d_core::Vec2::new(64.0, 64.0),
        size: ambition_platformer2d_core::Vec2::new(96.0, 128.0),
        kind: FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach: None,
        line_anchor: None,
        limb_host: None,
        flash: false,
        breakable_state: None,
        chest_opened: false,
        fighting: true,
        switch_on: false,
        rotation_rad: 0.0,
        alive: true,
        hit_flash_secs: 0.0,
        parry_flash_secs: 0.0,
        hp_current: 40,
        hp_max: 40,
        training_dummy: false,
        hit_strength: 0.0,
        unhittable: false,
        defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
        sprite_offset: None,
    }
}

/// An actor body converges to the realization the table now holds.
///
/// Medium (Half) to High (Full): the same entity, with the same feature and
/// actor identity, is drawn from the Full realization. The Half image is
/// removed from `Assets<Image>`.
#[test]
fn an_actor_body_converges_to_the_new_tier_and_the_old_image_dies() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));

    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let half_image = half.texture.id();
    let mut assets = GameAssets::default();
    assets.characters.publish(ACTOR_NAME, half);
    app.insert_resource(assets);

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        ACTOR_ID.to_string(),
        a_feature_view(),
    )]));
    app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
        ACTOR_ID.to_string(),
        ambition_sim_view::ActorRenderView {
            sprite_character_id: None,
            name: ACTOR_NAME.to_string(),
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
            geometry: ambition_sim_view::PoseGeometry::Settled,
        },
    )]));
    app.insert_resource(ambition_sim_view::BossRenderIndex::default());
    app.add_systems(Update, super::upgrade_actor_sprites);

    let body = app
        .world_mut()
        .spawn(FeatureVisual {
            id: ACTOR_ID.to_string(),
        })
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(half_image),
        "the fixture must START bound to the half realization"
    );

    let full = a_pending_realization(&mut app, TextureResolutionScale::Full);
    let full_image = full.texture.id();
    app.insert_resource(quality(VisualQualityProfile::High));
    app.world_mut()
        .resource_mut::<GameAssets>()
        .characters
        .publish(ACTOR_NAME, full.clone());
    app.update();

    // `GameAssets` changed a frame ago; the decode finishes now. A binder
    // gated on `is_changed()` never looks again.
    the_image_lands(&mut app, &full);
    drop(full);
    app.update();

    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(full_image),
        "a body must converge to the applied tier even when its image lands after \
         the frame the table changed on"
    );
    assert_eq!(
        app.world().get::<BoundSpriteQuality>(body).map(|q| q.scale),
        Some(TextureResolutionScale::Full),
    );
    // Same entity, same feature. Only the realization moved.
    assert_eq!(
        app.world().get::<FeatureVisual>(body).map(|v| v.id.clone()),
        Some(ACTOR_ID.to_string()),
    );

    // Nothing live references the old image. There is no evictor: the table
    // and the body dropped their handles, so Bevy frees the image.
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Image>>()
            .get(half_image)
            .is_none(),
        "the half-tier image is still resident, so residency did not FALL — \
         something is holding a strong handle to the retired realization"
    );
}

/// The controlled body takes the same path, through its own binder.
#[test]
fn the_player_body_converges_to_the_new_tier_and_the_old_image_dies() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));

    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let half_image = half.texture.id();
    let mut assets = GameAssets::default();
    assets.characters.publish(PLAYER_ID, half);
    app.insert_resource(assets);
    app.add_systems(Update, super::refresh_player_sprites_for_resident_quality);

    let body = app
        .world_mut()
        .spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView::default(),
            PlayerSpriteCharacter {
                id: PLAYER_ID.to_string(),
            },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(half_image),
        "the fixture must START bound to the half realization"
    );

    let full = a_pending_realization(&mut app, TextureResolutionScale::Full);
    let full_image = full.texture.id();
    app.insert_resource(quality(VisualQualityProfile::High));
    app.world_mut()
        .resource_mut::<GameAssets>()
        .characters
        .publish(PLAYER_ID, full.clone());
    app.update();

    the_image_lands(&mut app, &full);
    drop(full);
    app.update();

    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(full_image),
        "the controlled body must converge too, on a later frame than the change"
    );
    assert_eq!(
        app.world()
            .get::<PlayerSpriteCharacter>(body)
            .map(|c| c.id.clone()),
        Some(PLAYER_ID.to_string()),
        "the body still wears the same character: only its realization moved"
    );

    app.update();
    assert!(
        app.world()
            .resource::<Assets<Image>>()
            .get(half_image)
            .is_none(),
        "the half-tier image is still resident after the player rebound"
    );
}

/// A profile change that keeps the tier must not rebind the sprite.
///
/// `Low` and `Medium` use the same `Half` pixels. A binder keyed on the
/// profile, or on "`GameAssets` changed", would rebuild the sprite and reset
/// the animation cursor for no reason.
#[test]
fn a_profile_change_that_keeps_the_tier_does_not_rebind() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Low));

    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let mut assets = GameAssets::default();
    assets.characters.publish(PLAYER_ID, half);
    app.insert_resource(assets);
    app.add_systems(Update, super::refresh_player_sprites_for_resident_quality);

    let body = app
        .world_mut()
        .spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView::default(),
            PlayerSpriteCharacter {
                id: PLAYER_ID.to_string(),
            },
        ))
        .id();
    app.update();
    app.world_mut()
        .get_mut::<ambition_sprite_sheet::character::CharacterAnimator>(body)
        .expect("bound")
        .frame = 7;

    app.insert_resource(quality(VisualQualityProfile::Medium));
    // Touch the table, like an unrelated asset reload.
    app.world_mut().resource_mut::<GameAssets>();
    app.update();
    app.update();

    assert_eq!(
        app.world()
            .get::<ambition_sprite_sheet::character::CharacterAnimator>(body)
            .expect("still bound")
            .frame,
        7,
        "the resident realization did not move, so neither should the presentation"
    );
}

/// A spawn's art identity names its art.
///
/// Barks, hurt feedback, sprite-derived collision, and authored attack volumes
/// all resolve through `sprite_character_id`. The sheet must too, not the
/// display name. Otherwise `EnemySpawnSpec::character_id` cannot separate a
/// label from its art, and the spawn draws the placeholder.
#[test]
fn an_actor_binds_the_sheet_of_its_character_id_not_its_display_name() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Low));
    let mut assets = GameAssets::default();
    let art = a_pending_realization(&mut app, TextureResolutionScale::Full);
    the_image_lands(&mut app, &art);
    let art_image = art.texture.id();
    // Registered under the catalog id only. Nothing answers to the label.
    assets.characters.publish("catalog_identity", art);
    app.insert_resource(assets);

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        ACTOR_ID.to_string(),
        a_feature_view(),
    )]));
    app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
        ACTOR_ID.to_string(),
        ambition_sim_view::ActorRenderView {
            sprite_character_id: Some("catalog_identity".to_string()),
            // Not a registered sheet: a binder that preferred the label would
            // draw the placeholder.
            name: "A Label Nobody Registered".to_string(),
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
            geometry: ambition_sim_view::PoseGeometry::Settled,
        },
    )]));
    app.insert_resource(ambition_sim_view::BossRenderIndex::default());
    app.add_systems(Update, super::upgrade_actor_sprites);

    let body = app
        .world_mut()
        .spawn(FeatureVisual {
            id: ACTOR_ID.to_string(),
        })
        .id();
    app.update();

    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(art_image),
        "the body must wear the art its character_id names"
    );
}

/// An actor with no `sprite_character_id` (every authored spawn today) still
/// resolves by its display name.
#[test]
fn an_actor_without_a_character_id_still_resolves_by_its_display_name() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Low));
    let mut assets = GameAssets::default();
    let art = a_pending_realization(&mut app, TextureResolutionScale::Full);
    the_image_lands(&mut app, &art);
    let art_image = art.texture.id();
    assets.characters.publish(ACTOR_NAME, art);
    app.insert_resource(assets);

    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        ACTOR_ID.to_string(),
        a_feature_view(),
    )]));
    app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
        ACTOR_ID.to_string(),
        ambition_sim_view::ActorRenderView {
            sprite_character_id: None,
            name: ACTOR_NAME.to_string(),
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
            geometry: ambition_sim_view::PoseGeometry::Settled,
        },
    )]));
    app.insert_resource(ambition_sim_view::BossRenderIndex::default());
    app.add_systems(Update, super::upgrade_actor_sprites);

    let body = app
        .world_mut()
        .spawn(FeatureVisual {
            id: ACTOR_ID.to_string(),
        })
        .id();
    app.update();

    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(art_image),
        "an actor with no character_id must still resolve by name"
    );
}

/// A prop's quality stamp is the tier of the asset it was built from, not the
/// requested tier.
///
/// Props have no rematerialization recipe, so the table keeps the old asset.
/// This test makes sure that staleness stays visible.
#[test]
fn a_prop_is_stamped_with_the_tier_it_was_actually_built_from() {
    use crate::rendering::primitives::PropVisual;
    use ambition_platformer2d_world::rooms::PropDraw;

    let mut app = asset_app();
    // The request is Full; the table holds only Half.
    app.insert_resource(quality(VisualQualityProfile::High));
    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let mut assets = GameAssets::default();
    assets
        .characters
        .props
        .insert("crate_kind".to_string(), half);
    app.insert_resource(assets);
    app.add_systems(Update, super::refresh_prop_sprites_on_game_assets_change);

    let prop = app
        .world_mut()
        .spawn(PropVisual {
            id: "prop_0".to_string(),
            kind: "crate_kind".to_string(),
            name: "A Crate".to_string(),
            size: Vec2::new(16.0, 16.0),
            draw: PropDraw::default(),
            flip_y: false,
        })
        .id();
    app.update();

    assert_eq!(
        app.world().get::<BoundSpriteQuality>(prop).map(|q| q.scale),
        Some(TextureResolutionScale::Half),
        "⛔ the prop was built from Half pixels; stamping the REQUESTED Full \
         marks it current forever and nothing ever rebuilds it"
    );

    // The stamp settles: a second pass must not churn into a per-frame
    // rebuild.
    app.update();
    assert_eq!(
        app.world().get::<BoundSpriteQuality>(prop).map(|q| q.scale),
        Some(TextureResolutionScale::Half),
        "the comparison is self-limiting once stamped from the asset"
    );
}

/// The owner of the handle decides which readiness question is asked.
///
/// `texture_is_ready` separates "the asset loaded" from "a CPU copy is
/// resident". Both branches are tested, because each is wrong for the other
/// case:
///
/// * For a handle the asset server owns, ask the load state. This keeps
///   working if the main-world copy is evicted (Bevy's `RENDER_WORLD`-only
///   usage does this after upload).
/// * For a handle given straight to the main world (`reserve_handle`, `add`,
///   a procedural sprite), presence is readiness. The server would report
///   "never loaded" forever, and the sprite would never bind.
#[test]
fn texture_readiness_asks_the_owner_of_the_handle() {
    use super::texture_is_ready;

    // A separate app with the IO pool: `asset_server.load` panics without it.
    // The shared `asset_app()` fixture has no pool.
    let mut app = App::new();
    app.add_plugins(bevy::app::TaskPoolPlugin::default());
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    let asset_server = app.world().resource::<AssetServer>().clone();

    // Main-world-owned, not yet present: reserved but nothing inserted.
    let reserved = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .reserve_handle();
    assert!(
        !texture_is_ready(
            &asset_server,
            app.world().resource::<Assets<Image>>(),
            &reserved
        ),
        "a reserved handle with no image is not ready — this is the frame a body \
         must keep its current pixels"
    );

    // Main-world-owned and present: presence is readiness.
    let present = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    assert!(
        texture_is_ready(
            &asset_server,
            app.world().resource::<Assets<Image>>(),
            &present
        ),
        "a directly-added image is usable now; asking the asset server about it \
         would report 'never loaded' forever and a procedurally generated sprite \
         would never bind"
    );

    // Server-owned: ask the server. An unsettled load is not ready, whatever
    // the main world holds.
    let requested: Handle<Image> = asset_server.load("no_such_sheet_for_this_test.png");
    assert!(
        !texture_is_ready(
            &asset_server,
            app.world().resource::<Assets<Image>>(),
            &requested
        ),
        "an outstanding server load is not ready"
    );
    assert!(
        asset_server.get_load_state(requested.id()).is_some(),
        "the fixture must actually produce a SERVER-OWNED handle, or the branch \
         above is the main-world one wearing a disguise"
    );
}

/// A retired realization and one that never existed are both
/// `CharacterSheetState::Declared`. `retired_tier` tells them apart.
///
/// Retirement drops the token from `sheets` and keeps `declared`, because the
/// declaration is the recipe to rebuild it. The placeholder warning uses
/// `retired_tier` so it does not say "nothing demanded it" for a sheet that
/// was demanded and decoded.
#[test]
fn a_retired_realization_is_told_apart_from_one_that_never_existed() {
    use ambition_sprite_sheet::character::CharacterSheetState;

    let mut app = asset_app();
    let full = a_pending_realization(&mut app, TextureResolutionScale::Full);
    let mut assets = GameAssets::default();
    assets.characters.declare(ACTOR_ID, ACTOR_NAME);
    // A second declared character that is never published, so the test cannot
    // pass if everything reports a retirement.
    assets.characters.declare(PLAYER_ID, "Never Realized");

    assets.characters.publish(ACTOR_ID, full);
    assert!(
        assets.characters.sheet_state(ACTOR_ID).is_ready(),
        "premise: the realization is resident before anything retires it"
    );
    assert_eq!(
        assets.characters.retired_tier(ACTOR_ID),
        None,
        "a RESIDENT sheet has no retirement to report"
    );

    // The quality transition: the active tier drops to Quarter, so the Full
    // realization is above the ceiling and is retired.
    let retired = assets
        .characters
        .retire_realizations([ACTOR_ID.to_string()]);
    assert!(
        retired.contains(ACTOR_ID),
        "premise: the transition actually retired the fixture (retired {retired:?})"
    );

    // Both are now `Declared`.
    assert!(
        matches!(
            assets.characters.sheet_state(ACTOR_ID),
            CharacterSheetState::Declared { .. }
        ),
        "a retired realization returns to Declared"
    );
    assert!(
        matches!(
            assets.characters.sheet_state(PLAYER_ID),
            CharacterSheetState::Declared { .. }
        ),
        "and so does one that was never realized — the states are identical"
    );

    // The trace separates them.
    assert_eq!(
        assets.characters.retired_tier(ACTOR_ID),
        Some(TextureResolutionScale::Full),
        "the retired token names the tier whose pixels it actually held"
    );
    assert_eq!(
        assets.characters.retired_tier(PLAYER_ID),
        None,
        "a token nothing ever realized reports no retirement, so the warning \
         still says 'never materialized' for the case that deserves it"
    );
}

/// A character that comes back must not still report its old retirement.
///
/// Otherwise the trace accumulates, and a healthy re-realized sheet reads as
/// retired to any reader that does not check residency first.
#[test]
fn a_re_realized_character_no_longer_reports_a_retirement() {
    let mut app = asset_app();
    let full = a_pending_realization(&mut app, TextureResolutionScale::Full);
    let quarter = a_pending_realization(&mut app, TextureResolutionScale::Quarter);
    let mut assets = GameAssets::default();
    assets.characters.declare(ACTOR_ID, ACTOR_NAME);

    assets.characters.publish(ACTOR_ID, full);
    assets
        .characters
        .retire_realizations([ACTOR_ID.to_string()]);
    assert_eq!(
        assets.characters.retired_tier(ACTOR_ID),
        Some(TextureResolutionScale::Full),
        "premise: it is retired before it is re-published"
    );

    assets.characters.publish(ACTOR_ID, quarter);
    assert!(assets.characters.sheet_state(ACTOR_ID).is_ready());
    assert_eq!(
        assets.characters.retired_tier(ACTOR_ID),
        None,
        "re-realizing clears the trace"
    );
    // The display name too. The table is double-keyed, retirement is recorded
    // per token, and `publish` clears every token the character is declared
    // under.
    assert_eq!(
        assets.characters.retired_tier(ACTOR_NAME),
        None,
        "the display name is a token too, and it was retired alongside the id"
    );
}

/// An actor bind is one-shot, so its geometry must be complete before it.
///
/// `BoundFeatureKind` keys on kind and collision size only (`feature_kind.rs`).
/// A render size corrected after the bind does not reach the sprite. This is
/// deliberate: a wider key would make a settling body rebuild its sprite, and
/// the animator basis is chosen once.
///
/// So an actor must be geometry-complete at its first observable snapshot.
/// The Mary-O snake broke this rule: construction sized the body from the
/// catalog and `sync_sprite_posed_bodies` resized it from the sheet a tick
/// later. The fix gave construction the sheet's body authority; the key stays
/// narrow.
///
/// Control: the same sequence with the collision size also changing does
/// rebind. So the test is about the key contents, not a binder that never
/// re-runs.
#[test]
fn an_actor_bind_is_one_shot_so_its_geometry_must_be_complete_before_it() {
    fn bind_then_correct(collision_changes: bool) -> (Option<Vec2>, Option<Vec2>) {
        let mut app = asset_app();
        app.insert_resource(quality(VisualQualityProfile::High));
        let art = a_pending_realization(&mut app, TextureResolutionScale::Full);
        the_image_lands(&mut app, &art);
        let mut assets = GameAssets::default();
        assets.characters.publish(ACTOR_NAME, art);
        app.insert_resource(assets);

        // The snake's numbers: collision already final, render still the
        // spawn/catalog size.
        let collision = ambition_platformer2d_core::Vec2::new(21.3, 9.5);
        let stale_render = ambition_platformer2d_core::Vec2::new(118.2, 118.2);
        let correct_render = ambition_platformer2d_core::Vec2::new(23.3, 23.3);

        let feature_view = |size| ambition_sim_view::FeatureView {
            size,
            ..a_feature_view()
        };
        let actor_view = |render_size| ambition_sim_view::ActorRenderView {
            sprite_character_id: None,
            name: ACTOR_NAME.to_string(),
            is_sandbag: false,
            render_size: Some(render_size),
            dream_seed: None,
            geometry: ambition_sim_view::PoseGeometry::Settled,
        };

        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            ACTOR_ID.to_string(),
            feature_view(collision),
        )]));
        app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
            ACTOR_ID.to_string(),
            actor_view(stale_render),
        )]));
        app.insert_resource(ambition_sim_view::BossRenderIndex::default());
        app.add_systems(Update, super::upgrade_actor_sprites);

        let body = app
            .world_mut()
            .spawn(FeatureVisual {
                id: ACTOR_ID.to_string(),
            })
            .id();
        app.update();
        // Read the basis, not `Sprite.custom_size`: the constructor applies
        // frame-zero trim, so the quad size measures trim, not the bind.
        let basis = |app: &App| {
            app.world()
                .get::<ambition_sprite_sheet::character::CharacterAnimator>(body)
                .and_then(|a| a.render_basis)
                .map(|b| b.render_size)
        };
        let bound_at = basis(&app);

        // The correction the posed-body pass makes on the next tick.
        let next_collision = if collision_changes {
            ambition_platformer2d_core::Vec2::new(12.0, 6.0)
        } else {
            collision
        };
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            ACTOR_ID.to_string(),
            feature_view(next_collision),
        )]));
        app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
            ACTOR_ID.to_string(),
            actor_view(correct_render),
        )]));
        app.update();
        let after = basis(&app);
        (bound_at, after)
    }

    let (bound_at, after) = bind_then_correct(false);
    let (control_bound, control_after) = bind_then_correct(true);
    println!(
        "[latch] collision UNCHANGED: bound={bound_at:?} after_correction={after:?}\n\
         [latch] collision CHANGED  : bound={control_bound:?} after_correction={control_after:?}"
    );
    // Non-vacuity: the fixture must bind the stale size, or the next arm
    // proves nothing.
    let bound_at = bound_at.expect("the fixture must bind an animator with a render basis");
    assert!(
        (bound_at.x - 118.2).abs() < 1.0,
        "the fixture did not bind the STALE render size ({bound_at:?}), so the \
         correction below has nothing to fail to reach"
    );

    let after = after.expect("the animator still exists after the correction");
    assert!(
        (after.x - 118.2).abs() < 1.0,
        "the corrected render size reached an already-bound sprite ({after:?}). \
         If the key was deliberately widened, this lock is what has to be \
         retired WITH that decision — the construction seam upstream is written \
         against a bind that takes one answer"
    );

    // Control: change the collision size too, and the same sequence rebinds.
    let control = control_after.expect("the control animator exists");
    assert!(
        (control.x - 23.3).abs() < 1.0,
        "the CONTROL failed: even with the collision size changing the quad did \
         not follow ({control:?}), so this fixture cannot tell a missing cache \
         key from a binder that never re-runs"
    );
}

/// The refresh rebinds the character that a sprite was bound from. A body
/// that no binder stamped has none, and must not get a default character's
/// sheet.
#[test]
fn the_refresh_binds_nothing_onto_a_body_no_binder_stamped() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));
    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let mut assets = GameAssets::default();
    assets.characters.publish(PLAYER_ID, half);
    app.insert_resource(assets);
    app.add_systems(Update, super::refresh_player_sprites_for_resident_quality);

    let unmarked = app
        .world_mut()
        .spawn((PlayerVisual, ambition_sim_view::BodyPoseView::default()))
        .id();
    // Control: the same body, stamped, is bound. So the refusal comes from the
    // missing stamp.
    let marked = app
        .world_mut()
        .spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView::default(),
            PlayerSpriteCharacter {
                id: PLAYER_ID.to_string(),
            },
        ))
        .id();
    app.update();
    assert!(
        app.world().get::<BoundSpriteQuality>(unmarked).is_none(),
        "an unstamped body was bound a sheet nobody chose for it"
    );
    assert!(
        app.world().get::<BoundSpriteQuality>(marked).is_some(),
        "the control: a stamped body is refreshed"
    );
}

/// A pose mid-swap carries the previous identity's geometry, so the refresh
/// must not finalize from it; it binds once the pose settles.
#[test]
fn the_refresh_waits_for_a_settled_pose() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));
    let half = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &half);
    let mut assets = GameAssets::default();
    assets.characters.publish(PLAYER_ID, half);
    app.insert_resource(assets);
    app.add_systems(Update, super::refresh_player_sprites_for_resident_quality);

    let body = app
        .world_mut()
        .spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView {
                geometry: ambition_sim_view::PoseGeometry::Pending,
                ..Default::default()
            },
            PlayerSpriteCharacter {
                id: PLAYER_ID.to_string(),
            },
        ))
        .id();
    app.update();
    assert!(
        app.world().get::<BoundSpriteQuality>(body).is_none(),
        "the refresh finalized a binding from a pending pose"
    );
    app.world_mut()
        .get_mut::<ambition_sim_view::BodyPoseView>(body)
        .unwrap()
        .geometry = ambition_sim_view::PoseGeometry::Settled;
    app.update();
    assert!(
        app.world().get::<BoundSpriteQuality>(body).is_some(),
        "the control: the same body binds once its pose settles"
    );
}

/// An actor whose own art is declared but not resident is not drawn with the
/// sheet its display name resolves. The actor binding keys on kind and
/// collision size, which the arriving art does not change, so a substitute
/// would stay. The actor keeps the placeholder, then binds its own art.
#[test]
fn an_actor_waits_for_its_declared_art_rather_than_binding_its_names() {
    const OWN_ART: &str = "probe_actor_art";
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));
    // The name resolves a resident sheet; the art identity is only declared.
    let by_name = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &by_name);
    let name_image = by_name.texture.id();
    let mut assets = GameAssets::default();
    assets.characters.publish(ACTOR_NAME, by_name);
    assets.characters.declare(OWN_ART, OWN_ART);
    app.insert_resource(assets);
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        ACTOR_ID.to_string(),
        a_feature_view(),
    )]));
    app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([(
        ACTOR_ID.to_string(),
        ambition_sim_view::ActorRenderView {
            sprite_character_id: Some(OWN_ART.to_string()),
            name: ACTOR_NAME.to_string(),
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
            geometry: ambition_sim_view::PoseGeometry::Settled,
        },
    )]));
    app.insert_resource(ambition_sim_view::BossRenderIndex::default());
    app.add_systems(Update, super::upgrade_actor_sprites);
    let body = app
        .world_mut()
        .spawn(FeatureVisual {
            id: ACTOR_ID.to_string(),
        })
        .id();
    app.update();
    assert_ne!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(name_image),
        "the actor was bound the sheet its NAME resolves while its own art was coming"
    );

    let own = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &own);
    let own_image = own.texture.id();
    app.world_mut()
        .resource_mut::<GameAssets>()
        .characters
        .publish(OWN_ART, own);
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(own_image),
        "the control: once its own art is resident the actor binds it"
    );
}

/// An actor is not bound its art while its body is incomplete. The Hall of
/// Characters builds `mary_o` at 32x48 on her first tick and 21.3x32 on the
/// second: the prepared body arrives a tick after the placement. A bind from
/// the first tick builds its render basis once from the seeded quad, so the
/// binder waits for `PoseGeometry::Settled`.
#[test]
fn an_actor_is_not_bound_its_art_from_a_body_whose_geometry_is_pending() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Medium));
    let own = a_pending_realization(&mut app, TextureResolutionScale::Half);
    the_image_lands(&mut app, &own);
    let own_image = own.texture.id();
    let mut assets = GameAssets::default();
    assets.characters.publish(ACTOR_NAME, own);
    app.insert_resource(assets);
    app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
        ACTOR_ID.to_string(),
        a_feature_view(),
    )]));
    let row = |geometry| {
        ambition_sim_view::ActorRenderIndex::from_rows([(
            ACTOR_ID.to_string(),
            ambition_sim_view::ActorRenderView {
                sprite_character_id: Some(ACTOR_NAME.to_string()),
                name: ACTOR_NAME.to_string(),
                is_sandbag: false,
                render_size: None,
                dream_seed: None,
                geometry,
            },
        )])
    };
    app.insert_resource(row(ambition_sim_view::PoseGeometry::Pending));
    app.insert_resource(ambition_sim_view::BossRenderIndex::default());
    app.add_systems(Update, super::upgrade_actor_sprites);
    let body = app
        .world_mut()
        .spawn(FeatureVisual {
            id: ACTOR_ID.to_string(),
        })
        .id();
    app.update();
    assert!(
        app.world().get::<BoundSpriteQuality>(body).is_none(),
        "the actor binder finalized a binding from a body whose geometry is pending"
    );

    app.insert_resource(row(ambition_sim_view::PoseGeometry::Settled));
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(body).map(|s| s.image.id()),
        Some(own_image),
        "the control: the same actor binds its art once its geometry settles"
    );
}
