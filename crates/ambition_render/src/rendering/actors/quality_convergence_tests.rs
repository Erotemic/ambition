//! A live quality change must reach the bodies already on screen.
//!
//! The engine's side of this — retiring a stale realization and re-materializing
//! it at the applied tier — is guarded in
//! `ambition_platformer2d_actor_monolith::character_runtime`. This file guards the
//! half that decides what a participant actually SEES: a body bound from the old
//! realization has to end up bound to the new one, on the same entity, without
//! losing its identity, and the old image has to die.
//!
//! a rebind that only works on the frame `GameAssets` changed is not convergence. The new
//! sheet's pages are `asset_server.load`ed, so they land some frames LATER — after the one-frame
//! `is_changed()` window has closed.

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
/// That is what a fresh `asset_server.load` looks like for the frames before the
/// decode finishes, and every binder here skips a sheet whose texture has not
/// landed — so a fixture that pre-populated the image would silently test only
/// the same-frame case.
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
        // The fixture's realization got exactly the tier it asked for: these
        // tests are about convergence, not about a fallback.
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
/// Medium (Half) → High (Full): the SAME entity, still naming the same feature
/// and the same actor identity, ends up drawn from the Full realization — and
/// the Half image is gone from `Assets<Image>`, not merely unreferenced by the
/// table.
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

    // THE FRAME THAT MATTERS. `GameAssets` last changed a frame ago; the
    // decode finishes now. A binder gated on `is_changed()` never looks again.
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
    // LOGICAL IDENTITY: the same entity, still the same feature. The realization
    // moved; nothing about who this body IS did.
    assert_eq!(
        app.world().get::<FeatureVisual>(body).map(|v| v.id.clone()),
        Some(ACTOR_ID.to_string()),
    );

    // NOTHING LIVE STILL REFERENCES THE OLD ONE. There is no evictor: the
    // table dropped its clones on republish and the body dropped its handle on
    // rebind, so the last strong handle is gone and Bevy reclaims the image.
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

/// A profile change that keeps the tier must not thrash the sprite.
///
/// `Low` and `Medium` realize the same `Half` pixels. A binder that keyed on the
/// active PROFILE — or on "`GameAssets` changed" — would rebuild the sprite and
/// reset the animation cursor for nothing, every time the participant nudged a
/// setting that does not touch sheets.
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
    // And touch the table, the way an unrelated asset reload does.
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

/// a spawn's ART IDENTITY names its art.
///
/// the sheet was the ONE thing bound off presentation: everything else about
/// an actor — barks, hurt feedback, sprite-derived collision, authored attack
/// volumes — resolves through `sprite_character_id`, while `upgrade_actor_sprites`
/// looked the sheet up by DISPLAY NAME. So `EnemySpawnSpec::character_id`, added
/// so a level's label and its art identity could differ, could not do the job it
/// exists for: any spawn whose id differed from its name drew the placeholder.
#[test]
fn an_actor_binds_the_sheet_of_its_character_id_not_its_display_name() {
    let mut app = asset_app();
    app.insert_resource(quality(VisualQualityProfile::Low));
    let mut assets = GameAssets::default();
    let art = a_pending_realization(&mut app, TextureResolutionScale::Full);
    the_image_lands(&mut app, &art);
    let art_image = art.texture.id();
    // Registered under the CATALOG ID only. Nothing answers to the label.
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
            // deliberately NOT a registered sheet: if the binder still
            // preferred the label this would find nothing and draw the
            // placeholder, which is the bug.
            name: "A Label Nobody Registered".to_string(),
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
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

/// An actor with NO `sprite_character_id` — every authored spawn in the game today — still
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

/// a prop's quality stamp is the tier of the ASSET IT WAS BUILT FROM,
/// never the tier that was requested.
///
/// an honest stamp is not a current prop. Props still carry no rematerialization recipe, so
/// the table keeps the old asset — this pins that the staleness stays VISIBLE, which is the
/// whole difference bought.
#[test]
fn a_prop_is_stamped_with_the_tier_it_was_actually_built_from() {
    use crate::rendering::primitives::PropVisual;
    use ambition_platformer2d_world::rooms::PropDraw;

    let mut app = asset_app();
    // The REQUEST is Full; the table holds only Half.
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

    // And the stamp settles: a second pass must not churn, or the honest stamp
    // becomes a per-frame rebuild.
    app.update();
    assert_eq!(
        app.world().get::<BoundSpriteQuality>(prop).map(|q| q.scale),
        Some(TextureResolutionScale::Half),
        "the comparison is self-limiting once stamped from the asset"
    );
}

/// THE DISCRIMINATOR: who owns the handle decides which question is asked.
///
/// `texture_is_ready` replaced `Assets<Image>::get(..).is_some()` at four
/// binders, and the whole point is that it stops conflating "the asset loaded"
/// with "a CPU copy is resident". Both branches are exercised here because each
/// one is the wrong answer for the other's case:
///
/// * a handle the ASSET SERVER owns is asked the semantic question, so it keeps
///   working if the main-world copy is ever evicted (Bevy's `RENDER_WORLD`-only
///   usage does exactly that after upload);
/// * a handle handed straight to the main world — `reserve_handle`, `add`, a
///   procedurally generated sprite — has no load to ask about, so its presence
///   IS its readiness. Asking the server about it would report "never loaded"
///   forever, and a game that builds its own sprite would never bind.
#[test]
fn texture_readiness_asks_the_owner_of_the_handle() {
    use super::texture_is_ready;

    // its OWN app, with the IO pool: `asset_server.load` spawns onto it and
    // panics without it, and the shared `asset_app()` fixture deliberately has no
    // pool because no other test here issues a real load.
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

    // Main-world-owned and present: readiness IS presence, because there is no
    // load to ask about.
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

    // Server-owned: the question goes to the server, and a load that has not
    // settled is not ready — regardless of what the main world holds.
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

/// A retired realization and one that never existed are the SAME
/// `CharacterSheetState::Declared`, and the placeholder warning used to assert
/// the second for both.
///
/// The retirement drops the token from `sheets` and deliberately leaves
/// `declared` standing — that declaration is the recipe for re-making it — so
/// nothing about the state distinguishes "was decoded, then dropped by a quality
/// transition" from "nothing has ever decoded this". `retired_tier` is the trace
/// that does. The warning it feeds fired 111 times on one Hall reveal saying
/// "nothing demanded it", which for every retired sheet among them was false
/// twice: it HAD been demanded, and it HAD been decoded.
#[test]
fn a_retired_realization_is_told_apart_from_one_that_never_existed() {
    use ambition_sprite_sheet::character::CharacterSheetState;

    let mut app = asset_app();
    let full = a_pending_realization(&mut app, TextureResolutionScale::Full);
    let mut assets = GameAssets::default();
    assets.characters.declare(ACTOR_ID, ACTOR_NAME);
    // A second declared character that is never published: the arm that keeps
    // this test from passing because EVERYTHING reports a retirement.
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

    // The quality transition: the active tier drops to Quarter, so a Full
    // realization is above the ceiling and goes.
    let retired = assets
        .characters
        .retire_realizations([ACTOR_ID.to_string()]);
    assert!(
        retired.contains(ACTOR_ID),
        "premise: the transition actually retired the fixture (retired {retired:?})"
    );

    // ── Both are now `Declared`, which is the whole problem ──────────────────
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

    // ── And the trace separates them ────────────────────────────────────────
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

/// A character that comes back must stop being described by the retirement it
/// recovered from.
///
/// Otherwise the trace is worse than nothing: it would accumulate, and a healthy
/// re-realized sheet would be reported as retired forever by anything that read
/// it without first checking residency.
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
    // ⭐ THE DISPLAY NAME TOO, not just the id. The table is double-keyed, the
    // retirement is recorded per TOKEN, and `publish` clears every token the
    // character was declared under — a clear that only covered the id would
    // leave the name reporting a retirement the character recovered from.
    assert_eq!(
        assets.characters.retired_tier(ACTOR_NAME),
        None,
        "the display name is a token too, and it was retired alongside the id"
    );
}

/// **AN ACTOR BIND IS ONE-SHOT, SO ITS GEOMETRY MUST BE COMPLETE BEFORE IT.**
///
/// `BoundFeatureKind` keys on kind + COLLISION size only (`feature_kind.rs`),
/// so a render size that is corrected AFTER the bind never reaches the sprite:
/// the collision footprint the key remembers has not changed, and nothing
/// invalidates it. That is deliberate, not an oversight — widening the key to
/// every presentation input would make a settling body rebuild its sprite
/// repeatedly, and the animator's basis is meant to be chosen once.
///
/// Locked here because it is the CONSTRAINT the construction seam has to
/// satisfy: an actor must be geometry-complete at its first observable
/// snapshot, because presentation will not take a second answer. The Mary-O
/// snake violated it — construction sized the body from the catalog join while
/// `sync_sprite_posed_bodies` resized it from the sheet a tick later, leaving
/// exactly one tick of final collision beside a 5x spawn render size — and the
/// fix was to give construction the sheet's body authority, not to widen this
/// key.
///
/// ⚠ The control is the same sequence with the collision size ALSO changing:
/// that one does rebind, which is what says this is about the key's contents
/// rather than about the binder never re-running.
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

        // The snake's own numbers: collision already final, render still the
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
        // ⛔ THE BASIS, NOT `Sprite.custom_size`: the constructor applies
        // frame-zero TRIM, so the sprite's quad is a per-frame derivation and
        // reading it measured the trim rather than the bind.
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
    // ANTI-VACUITY: the fixture must actually have bound something, and it
    // must have bound the STALE size, or the arm below proves nothing.
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

    // THE CONTROL: change the collision size too, and the same sequence
    // rebinds. Without this the arm above could be about binding never
    // updating at all.
    let control = control_after.expect("the control animator exists");
    assert!(
        (control.x - 23.3).abs() < 1.0,
        "the CONTROL failed: even with the collision size changing the quad did \
         not follow ({control:?}), so this fixture cannot tell a missing cache \
         key from a binder that never re-runs"
    );
}

/// ⛔ The refresh rebinds the character a sprite was bound FROM. A body no
/// binder stamped has none, and naming one for it (this defaulted to
/// `player_robot_v3`) gave an unworn shell a character's sheet.
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
    // The control: the same body, stamped, IS bound — so the refusal above is
    // the missing stamp speaking, not a fixture the pass cannot reach.
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
