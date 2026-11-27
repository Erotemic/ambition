use std::sync::Arc;

use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;
use ambition_sprite_sheet::character::rigged::{ClipTween, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterSpriteAsset,
};

use super::*;
use crate::rendering::actors::draw_held_frame;

const RENDER: Vec2 = Vec2::new(103.0, 114.0);

#[test]
fn a_library_wearer_asks_for_the_track_of_its_authored_fit() {
    use ambition_platformer2d_shared_tangle::worn_gear::{GearFit, WornGearEntry, WornGearManifest};
    use ambition_sim_view::{WornGearFact, WornGearView};
    use crate::rendering::worn_gear_visuals::sync_worn_gear_visuals;

    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    let target = "player_robot_v3";
    let sheet = sheet_with(&sprites, target, None);
    assert_ne!(sheet.spec.base_sheet_key(), target);
    let mut app = App::new();
    let body = app.world_mut().spawn(CharacterAnimator::new(&sheet)).id();
    app.insert_resource(WornGearView(vec![WornGearFact {
        body, feature: None, gear: "helmet".into(), room: None,
    }]));
    app.insert_resource(WornGearManifest(vec![WornGearEntry {
        id: "helmet".into(),
        anchor: Default::default(),
        slot: "head".into(),
        back: None,
        front: Some("sprites/space_helmet_front.png".into()),
        size: Vec2::splat(64.0),
        socket: Vec2::new(0.5, 0.58),
        fits: std::collections::BTreeMap::from([(target.to_owned(), GearFit {
            track: "head".into(), scale: 1.0, offset: Vec2::ZERO, turn_degrees: 0.0,
        })]),
    }]));
    app.add_systems(Update, sync_worn_gear_visuals);

    app.update();
    let request = app.world().get::<PartSetRequest>(body).expect("the helmet asks for its anchor");
    assert_eq!(request.tracks_of("worn_gear:head"), Some(&["head".to_owned()][..]));
}

fn raider(sprite_content: &ambition_sprite_sheet::SpriteContent, rigged: bool) -> CharacterSpriteAsset {
    raider_with(&sprite_content, rigged.then(|| RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").expect("a published flipbook")))
}

fn raider_with(sprite_content: &ambition_sprite_sheet::SpriteContent, flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    sheet_with(&sprite_content, "pirate_raider", flipbook)
}

fn sheet_with(sprite_content: &ambition_sprite_sheet::SpriteContent, character: &str, flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(&sprite_content, character).expect("a baked sheet");
    let page = ambition_sprite_sheet::character::CharacterSpritePage {
        texture: Handle::default(),
        layout: Handle::default(),
    };
    CharacterSpriteAsset {
        texture: Handle::default(),
        layout: Handle::default(),
        spec,
        pages: vec![page],
        requested_tier: TextureResolutionScale::Full,
        resolved_tier: TextureResolutionScale::Full,
        rigged: flipbook.map(|flipbook| RiggedSpritePages {
            flipbook: Arc::new(flipbook),
            pages: vec![Handle::default()],
        }),
    }
}

/// An app with the two systems, the raider's sheet in the table, and one root
/// drawn from it with its anchor on the sheet's feet.
fn app(sprite_content: &ambition_sprite_sheet::SpriteContent, admit: bool) -> (App, Entity) {
    app_with(&sprite_content, admit, raider(&sprite_content, true))
}

/// A page whose image is present: ready to draw.
fn ready_page(app: &mut App) -> Handle<Image> {
    let page = pending_page(app);
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(page.id(), Image::default())
        .unwrap();
    page
}

/// A page whose image is not there yet, as while it loads.
fn pending_page(app: &mut App) -> Handle<Image> {
    app.world_mut().resource_mut::<Assets<Image>>().reserve_handle()
}

/// `sheet` with its part pages replaced by pages from `page`.
fn with_pages(mut sheet: CharacterSpriteAsset, mut page: impl FnMut() -> Handle<Image>) -> CharacterSpriteAsset {
    if let Some(rigged) = sheet.rigged.as_mut() {
        rigged.pages = rigged.pages.iter().map(|_| page()).collect();
    }
    sheet
}

/// A crouch squash the stand-in animator applies, as `StanceSquash` does for a
/// sheet with no compact row: `(ratio, about_quad_foot)`.
#[derive(Resource, Default)]
struct Squash(Option<(f32, bool)>);

/// The stand-in for the game's animator system: every frame it draws the
/// root's current frame the game's way (page, atlas index, trimmed size and
/// anchor), which the rigged driver then replaces with the impostor.
/// A limb's twin has no animator of its own: it follows its root
/// ([`follow_limb_twins`]).
#[allow(clippy::type_complexity)]
fn animate(
    squash: Res<Squash>,
    mut roots: Query<(&mut Sprite, &mut CharacterAnimator, &mut Anchor), (Without<RiggedPartSlot>, Without<LimbTwin>)>,
) {
    for (mut sprite, mut animator, mut anchor) in &mut roots {
        let flip = sprite.flip_x;
        draw_held_frame(&mut sprite, &mut animator, &mut anchor, flip);
        if let (Some((ratio, about_foot)), Some(size)) = (squash.0, sprite.custom_size.as_mut()) {
            let (h0, a0) = (size.y, anchor.0.y);
            size.y = h0 * ratio;
            if about_foot {
                let foot = -(a0 + 0.5) * h0;
                let top = foot + ((0.5 - a0) * h0 - foot) * ratio;
                anchor.0.y = 0.5 - top / size.y;
            }
        }
    }
}

/// Whether the test app reads every body as one image
/// ([`ComposedBodyDemand`]), as a hit flash or a portal does: on for the tests
/// of the compositor, off for the tests of direct drawing.
#[derive(Resource)]
struct Composite(bool);

fn declare(composite: Res<Composite>, roots: Query<Entity, With<CharacterAnimator>>, mut demand: ResMut<ComposedBodyDemand>) {
    if composite.0 {
        for root in &roots {
            demand.declare(root);
        }
    }
}

fn app_with(sprite_content: &ambition_sprite_sheet::SpriteContent, admit: bool, sheet: CharacterSpriteAsset) -> (App, Entity) {
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    app_anchored(&sprite_content, admit, sheet, Anchor(feet))
}

/// [`app_with`], with the root built at `anchor`: the feet for an NPC,
/// `Anchor::CENTER` for a player with a sheet-authored quad
/// (`character_render_basis`).
fn app_anchored(sprite_content: &ambition_sprite_sheet::SpriteContent, admit: bool, sheet: CharacterSpriteAsset, anchor: Anchor) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(sprite_content.clone());
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<TextureAtlasLayout>>();
    let sheet = with_pages(sheet, || ready_page(&mut app));
    app.add_observer(give_back_the_cell_of_a_removed_presentation);
    app.init_resource::<RiggedImpostorAtlas>()
        .init_resource::<Squash>()
        .init_resource::<ComposedBodyDemand>()
        .insert_resource(Composite(true))
        .insert_resource(RiggedSpriteAdmission { admit })
        .add_systems(Update, (animate, declare, bind_rigged_presentations, rigged_presentation_drive()).chain());
    let mut assets = GameAssets::default();
    assets.characters.declare("raider");
    assets.characters.publish(&sprite_content, "raider", sheet.clone());
    app.insert_resource(assets);
    let asset = sheet;
    let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, anchor);
    let root = app
        .world_mut()
        .spawn((
            sprite,
            anchor,
            animator,
            Transform::default(),
            Visibility::Inherited,
            BoundSpriteQuality::of_tier(TextureResolutionScale::Full),
        ))
        .id();
    (app, root)
}

/// Run the app until a body declared as read as one image draws its cell: its
/// first cell is on a page built that frame, whose cameras arrive with the
/// frame's commands, so it draws its parts directly once more.
fn settle(app: &mut App) {
    app.update();
    app.update();
}

/// No root has a presentation owner.
fn no_presentations(app: &mut App) -> bool {
    let mut owners = app.world_mut().query::<&RiggedPresentation>();
    owners.iter(app.world()).count() == 0
}

fn owner(app: &App, root: Entity) -> Entity {
    app.world()
        .get::<RiggedPresentedBy>(root)
        .expect("the root has a rigged presentation")
        .owner()
}

#[test]
fn a_prop_sheet_binds_its_parts_without_a_character_sheet() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    let sheet = app.world().resource::<GameAssets>().characters.sheet("raider").unwrap().clone();
    let mut assets = GameAssets::default();
    assets.characters.props.insert("platform".into(), sheet);
    app.insert_resource(assets);

    settle(&mut app);

    let owner = owner(&app, root);
    assert!(app.world().get::<RiggedLocalSample>(owner).unwrap().parts().is_some());
}

/// `(translation, visible)` of every slot, in draw order: sheet pixels from
/// the feet, +y up, in the body's private cell.
fn slots(app: &App, owner: Entity) -> Vec<(Vec2, bool)> {
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    presentation
        .slots
        .iter()
        .map(|slot| {
            let world = app.world();
            (
                world.get::<Transform>(*slot).unwrap().translation.truncate(),
                *world.get::<Visibility>(*slot).unwrap() != Visibility::Hidden,
            )
        })
        .collect()
}

fn frame_draws(app: &App, root: Entity, target: &str) -> Vec<Vec2> {
    let sprite_content = app.world().resource::<ambition_sprite_sheet::SpriteContent>().clone();
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    let row = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, target).unwrap();
    flipbook
        .frame(&row, animator.frame)
        .unwrap()
        .iter()
        .map(|draw| Vec2::new(draw.at.x, -draw.at.y))
        .collect()
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 1.0e-3
}

/// The first page of the first class's atlas: the one every body of these
/// tests takes.
fn atlas(app: &App) -> &ImpostorAtlas {
    app.world().resource::<RiggedImpostorAtlas>().0[0].first().expect("the impostor atlas was built")
}

/// Whether the root draws its impostor: its cell of the shared atlas.
fn draws_impostor(app: &App, root: Entity, owner: Entity) -> bool {
    let sprite = app.world().get::<Sprite>(root).unwrap();
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    sprite.image == atlas(app).image
        && sprite
            .texture_atlas
            .as_ref()
            .is_some_and(|frame| frame.index == presentation.impostor.expect("an impostor cell").cell as usize && Some(&frame.layout) == atlas(app).layout.as_ref())
}

/// The root draws its baked frame: an atlas frame on its own page.
fn draws_baked(app: &App, root: Entity) -> bool {
    let sprite = app.world().get::<Sprite>(root).unwrap();
    let on_atlas = app
        .world()
        .resource::<RiggedImpostorAtlas>()
        .0
        .iter()
        .flatten()
        .any(|atlas| sprite.image == atlas.image);
    sprite.texture_atlas.is_some() && !on_atlas
}

#[test]
fn a_rigged_root_draws_its_impostor_from_parts_in_reusable_slots() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    settle(&mut app);
    let owner = owner(&app, root);
    let max_draws = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").unwrap().max_draws();
    let drawn = slots(&app, owner);
    assert_eq!(drawn.len(), max_draws);
    let expected = frame_draws(&app, root, "pirate_raider");
    assert_eq!(drawn.iter().filter(|(_, visible)| *visible).count(), expected.len());
    for ((at, _), want) in drawn.iter().zip(&expected) {
        assert!(close(*at, *want), "a part at {at:?}, its draw at {want:?}");
    }
    assert!(draws_impostor(&app, root, owner), "the root does not draw its impostor");
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 1.0, "the root is drawn, not hidden");

    // The root's quad is its cell, with the feet on the root's feet.
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").unwrap();
    let per_pixel = RENDER / flipbook.frame_size.as_vec2();
    let feet = presentation.impostor.expect("an impostor cell").feet;
    assert_eq!(feet, flipbook.feet_pixel + Vec2::splat(impostor_margin(&flipbook)));
    let sprite = app.world().get::<Sprite>(root).unwrap();
    assert!(close(sprite.custom_size.unwrap(), Vec2::splat(IMPOSTOR_CELL) * per_pixel));
    let anchor = app.world().get::<Anchor>(root).unwrap().0;
    assert!(
        close(anchor, Vec2::new(feet.x / IMPOSTOR_CELL - 0.5, 0.5 - feet.y / IMPOSTOR_CELL)),
        "{anchor:?}"
    );

    // Another frame reuses the slots: nothing is spawned.
    let entities = app.world().entities().count_spawned();
    let slot_ids = app.world().get::<RiggedPresentation>(owner).unwrap().slots.clone();
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().frame = 3;
    app.update();
    assert_eq!(app.world().entities().count_spawned(), entities);
    assert_eq!(self::owner(&app, root), owner, "the frame change rebound the root");
    assert_eq!(app.world().get::<RiggedPresentation>(owner).unwrap().slots, slot_ids);
    let expected = frame_draws(&app, root, "pirate_raider");
    for ((at, _), want) in slots(&app, owner).iter().zip(&expected) {
        assert!(close(*at, *want), "frame 3: a part at {at:?}, its draw at {want:?}");
    }

    // A mirrored root mirrors its quad about its feet; the parts in the
    // impostor do not move.
    app.world_mut().get_mut::<Sprite>(root).unwrap().flip_x = true;
    app.update();
    for ((at, _), want) in slots(&app, owner).iter().zip(&expected) {
        assert!(close(*at, *want), "mirrored: a part moved to {at:?} from {want:?}");
    }
    assert!(close(app.world().get::<Anchor>(root).unwrap().0, Vec2::new(-anchor.x, anchor.y)));
}

#[test]
fn rigged_slots_follow_view_and_image_demand() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.world_mut().resource_mut::<Composite>().0 = false;
    let mut camera = Camera::default();
    camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
        physical_size: UVec2::splat(400),
        ..default()
    });
    let projection = OrthographicProjection {
        area: Rect::from_center_size(Vec2::ZERO, Vec2::splat(400.0)),
        ..OrthographicProjection::default_2d()
    };
    app.world_mut().spawn((Camera2d, camera, Projection::Orthographic(projection), GlobalTransform::IDENTITY));
    let home = *app.world().get::<Transform>(root).unwrap();
    let away = Transform::from_xyz(8000.0, 0.0, 0.0);
    *app.world_mut().get_mut::<Transform>(root).unwrap() = away;

    settle(&mut app);
    let owner = owner(&app, root);
    assert!(slots(&app, owner).is_empty());
    assert_eq!(app.world().get::<Visibility>(owner), Some(&Visibility::Hidden));

    *app.world_mut().get_mut::<Transform>(root).unwrap() = home;
    app.update();
    assert!(slots(&app, owner).iter().any(|&(_, shown)| shown));
    assert_eq!(app.world().get::<Visibility>(owner), Some(&Visibility::Inherited));

    *app.world_mut().get_mut::<Transform>(root).unwrap() = away;
    for _ in 0..SLOT_HOLD_FRAMES {
        app.update();
    }
    assert!(slots(&app, owner).is_empty());
    assert_eq!(app.world().get::<Visibility>(owner), Some(&Visibility::Hidden));

    app.world_mut().resource_mut::<Composite>().0 = true;
    settle(&mut app);
    assert!(draws_impostor(&app, root, owner));
    assert!(slots(&app, owner).iter().any(|&(_, shown)| shown));
}

#[cfg(feature = "card_view")]
#[test]
fn a_tilted_output_samples_edge_bodies_and_allocates_sprite_slots_for_readers() {
    use crate::rendering::camera::{
        coverage::ReplacedPlaneCamera, PresentedViewFrame, PresentedViewFrames,
    };
    use crate::rendering::card_view::{
        realize_card_bodies, CardBody, CardMeshes, CardRootDepths, CardViewMode, CardViewSettings,
    };
    use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use ambition_sim_view::{LocalView, LocalViewId, PresentsView};

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<CardMeshes>()
        .init_resource::<CardRootDepths>()
        .insert_resource(CardViewSettings {
            yaw_radians: 45.0_f32.to_radians(),
            pitch_radians: 0.0,
            ..CardViewSettings::for_game(CardViewMode::SideOn)
        })
        .add_systems(Update, realize_card_bodies.after(RiggedDriveSet));
    app.world_mut().resource_mut::<Composite>().0 = false;

    let world = app.world_mut();
    let observer = world.spawn((LocalView, LocalViewId::FIRST)).id();
    world.insert_resource(PresentedViewFrames::of_views(vec![(observer, PresentedViewFrame {
        room: LiveRoomInstance::ACTIVATION,
        plane_center: Vec2::ZERO,
        roll_radians: 0.0,
        orthographic_scale: 2.5,
    })]));
    let mut camera = Camera { is_active: false, ..default() };
    camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
        physical_size: UVec2::new(1280, 720),
        ..default()
    });
    world.spawn((Camera2d, MainCamera, camera, ReplacedPlaneCamera, PresentsView(observer),
        Projection::Orthographic(OrthographicProjection { scale: 2.5, ..OrthographicProjection::default_2d() })));
    world.get_mut::<Transform>(root).unwrap().translation.x = 2100.0;

    settle(&mut app);
    let owner = owner(&app, root);
    assert!(app.world().get::<RiggedLocalSample>(owner).unwrap().parts().is_none());

    app.world_mut().resource_mut::<CardViewSettings>().mode = CardViewMode::Gameplay;
    settle(&mut app);
    assert!(app.world().get::<RiggedLocalSample>(owner).unwrap().parts().is_some());
    assert!(slots(&app, owner).is_empty());
    let world = app.world_mut();
    assert!(world.query::<(&CardBody, &Visibility)>().iter(world)
        .any(|(card, visibility)| card.owner == owner && *visibility != Visibility::Hidden
            && !card.parts.is_empty()));

    app.world_mut().resource_mut::<Composite>().0 = true;
    settle(&mut app);
    assert!(draws_impostor(&app, root, owner));
    assert!(slots(&app, owner).iter().any(|&(_, shown)| shown));
}

/// Posed parts are made from the flipbook's draw program and the admitted body
/// rig. A sheet realized again with the same program keeps its binding; a
/// flipbook whose program differs (here its track names, which the named
/// bindings read) binds its own, not the ones the old flipbook bound.
#[test]
fn a_new_flipbook_binds_its_own_posed_parts() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    settle(&mut app);
    let posed = |app: &App| app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().posed.clone();
    let first = posed(&app).expect("the premise: the raider publishes a body rig, so its parts are posed");

    let sheet = with_pages(raider(&sprite_content, true), || ready_page(&mut app));
    app.world_mut().resource_mut::<GameAssets>().characters.publish(&sprite_content, "raider", sheet);
    settle(&mut app);
    let same = posed(&app).expect("the realized-again sheet's parts are posed");
    assert!(Arc::ptr_eq(&first, &same), "an unchanged rig and program were bound again");

    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").expect("a published flipbook");
    let renamed = flipbook.tracks().iter().map(|name| format!("{name}_renamed")).collect();
    let sheet = with_pages(raider_with(&sprite_content, Some(flipbook.with_tracks(renamed))), || ready_page(&mut app));
    app.world_mut().resource_mut::<GameAssets>().characters.publish(&sprite_content, "raider", sheet);
    settle(&mut app);
    assert!(
        posed(&app).is_none_or(|second| !Arc::ptr_eq(&first, &second)),
        "the new program was bound with the posed parts of the old one"
    );
}

#[test]
fn with_the_trial_off_no_root_gets_parts() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, false);
    app.update();
    assert!(no_presentations(&mut app));
    let mut parts = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(parts.iter(app.world()).count(), 0);
    assert!(draws_baked(&app, root));
}

#[test]
fn a_root_whose_sheet_loses_its_flipbook_draws_itself_again() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.update();
    let owner = owner(&app, root);
    app.world_mut()
        .resource_mut::<GameAssets>()
        .characters
        .publish(&sprite_content, "raider", raider(&sprite_content, false));
    app.update();
    assert!(app.world().get_entity(owner).is_err(), "the owner outlived the flipbook");
    let mut parts = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(parts.iter(app.world()).count(), 0, "a slot outlived its owner");
    // Its page held only this body, so the page is retired: no camera runs,
    // or stays, with no body to draw.
    assert!(app.world().resource::<RiggedImpostorAtlas>().0.iter().all(Vec::is_empty), "an empty page outlived its last body");
    let mut cameras = app.world_mut().query::<&RiggedImpostorCamera>();
    app.update();
    assert_eq!(cameras.iter(app.world()).count(), 0, "a retired page's cameras stayed");
    assert!(draws_baked(&app, root), "the root did not take its baked frame back");
}

/// The parts draw only into the impostor: on the private layer, in the body's
/// own cell. Every view draws the ROOT (on whatever layers it has), so a
/// second pane adds no presentation entity.
#[test]
fn the_parts_draw_only_into_their_impostor() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    settle(&mut app);
    let owner = owner(&app, root);
    let private = RenderLayers::layer(RIGGED_IMPOSTOR_LAYER);
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    for slot in &presentation.slots {
        assert_eq!(app.world().get::<RenderLayers>(*slot), Some(&private));
    }
    for camera in &atlas(&app).cameras {
        assert_eq!(app.world().get::<RenderLayers>(*camera), Some(&private));
        assert!(app.world().get::<Camera>(*camera).unwrap().is_active, "a body draws but the atlas rests");
    }
    let origin = app.world().get::<Transform>(owner).unwrap().translation.truncate();
    assert_eq!(origin, atlas(&app).cell_feet(presentation.impostor.expect("an impostor cell").cell, presentation.impostor.expect("an impostor cell").feet));

    let pane = RenderLayers::layer(0).with(5);
    app.world_mut().entity_mut(root).insert(pane.clone());
    let entities = app.world().entities().count_spawned();
    app.update();
    assert_eq!(app.world().entities().count_spawned(), entities);
    assert_eq!(app.world().get::<RenderLayers>(root), Some(&pane), "the root's pane was rewritten");
    assert!(draws_impostor(&app, root, owner));
}

/// Two bodies stand in two cells (a second page, one growth step larger, makes
/// room), and a freed cell is reused.
#[test]
fn each_body_has_its_own_cell() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, first) = app(&sprite_content, true);
    let second = {
        let asset = raider(&sprite_content, true);
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    settle(&mut app);
    let cell = |app: &App, root| {
        let impostor = app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().impostor.expect("an impostor cell");
        (impostor.page, impostor.cell)
    };
    let (a, b) = (cell(&app, first), cell(&app, second));
    assert_ne!(a, b);
    let pages = &app.world().resource::<RiggedImpostorAtlas>().0[0];
    assert_eq!(pages.iter().map(|page| page.side).collect::<Vec<_>>(), [1, 2], "the second body's page");
    // Each body's parts stand in its own cell.
    for root in [first, second] {
        let owner = owner(&app, root);
        let impostor = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell");
        let page = &app.world().resource::<RiggedImpostorAtlas>().0[0][impostor.page];
        let at = app.world().get::<Transform>(owner).unwrap().translation.truncate();
        assert_eq!(at, page.cell_feet(impostor.cell, impostor.feet));
    }
    app.world_mut().entity_mut(first).despawn();
    app.update();
    let third = {
        let asset = raider(&sprite_content, true);
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    settle(&mut app);
    assert_eq!(cell(&app, third), a, "the freed cell was not reused");
}

/// More bodies drawn from parts alone than a page of their class holds: each
/// draws its own cell, the class opens pages one growth step larger each time
/// (1, then 2, then 4 cells a side), and no page is rebuilt for a later body —
/// a body already drawn keeps its target — and no body draws
/// [`NO_BAKED_IMAGE`], which is what a body with no cell falls back to.
#[test]
fn a_class_with_every_cell_taken_opens_a_page() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    let mut sheet = raider(&sprite_content, true);
    sheet.texture = NO_BAKED_IMAGE;
    for page in &mut sheet.pages {
        page.texture = NO_BAKED_IMAGE;
    }
    assert!(sheet.parts_only(), "the sheet is not drawn from parts alone, so this tests nothing");
    let (mut app, first) = app_with(&sprite_content, true, sheet.clone());
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let spawn = |app: &mut App| {
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&sheet, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    let mut roots = vec![first];
    for _ in 0..4 {
        roots.push(spawn(&mut app));
    }
    settle(&mut app);
    let targets = |app: &App| app.world().resource::<RiggedImpostorAtlas>().0[0].iter().map(|page| page.image.clone()).collect::<Vec<_>>();
    let before = targets(&app);
    assert_eq!(before.len(), 2, "five bodies fill a page of one cell and a page of four");
    // A sixth opens a third page; the first two are untouched.
    roots.push(spawn(&mut app));
    settle(&mut app);
    let pages = &app.world().resource::<RiggedImpostorAtlas>().0[0];
    assert_eq!(pages.iter().map(|page| page.side).collect::<Vec<_>>(), [1, 2, 4]);
    assert_eq!(targets(&app)[..2], before[..], "a page was rebuilt for a later body");
    let mut per_page = [0; 3];
    for root in &roots {
        let sprite = app.world().get::<Sprite>(*root).unwrap();
        assert!(sprite.image != NO_BAKED_IMAGE, "a body drawn from parts alone draws no image");
        let owner = owner(&app, *root);
        let impostor = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell");
        let page = &app.world().resource::<RiggedImpostorAtlas>().0[0][impostor.page];
        assert!(
            sprite.image == page.image
                && sprite.texture_atlas.as_ref().map(|frame| frame.index) == Some(impostor.cell as usize),
            "a body does not draw its cell of page {}",
            impostor.page
        );
        let place = app.world().get::<Transform>(owner).unwrap().translation.truncate();
        assert_eq!(place, impostor_cell_feet(0, impostor.page, page.side, impostor.cell, impostor.feet));
        per_page[impostor.page] += 1;
    }
    assert_eq!(per_page, [1, 4, 1]);
    // The third page stands to the right of the second.
    let pages = &app.world().resource::<RiggedImpostorAtlas>().0[0];
    assert!(pages[2].cell_feet(0, Vec2::ZERO).x >= 2.0 * IMPOSTOR_PAGE_STEP);

    // Each page renders for its own cells only: a frame with no change rests
    // every page, and a new frame of the body on the third page runs that
    // page's cameras and no other's.
    let active = |app: &App, page: usize| {
        let cameras = &app.world().resource::<RiggedImpostorAtlas>().0[0][page].cameras;
        cameras.iter().map(|camera| app.world().get::<Camera>(*camera).unwrap().is_active).collect::<Vec<_>>()
    };
    app.update();
    assert!((0..3).all(|page| !active(&app, page).contains(&true)), "a page renders with no change");
    let on_third = *roots.last().unwrap();
    app.world_mut().get_mut::<CharacterAnimator>(on_third).unwrap().frame = 3;
    app.update();
    assert!(!active(&app, 2).contains(&false), "the third page rests while its body changes");
    assert!(!active(&app, 0).contains(&true) && !active(&app, 1).contains(&true), "another page renders for a body of the third");
}

/// The raider's published flipbook with `row` left to the baked sheet: its
/// clip is taken out of `clips` and named in `baked_clips`, as a hybrid
/// publish states it.
fn hybrid_raider(sprite_content: &ambition_sprite_sheet::SpriteContent, row: &str) -> RiggedSpriteAsset {
    let text = ambition_sprite_sheet::baked_part_flipbooks::published_ron_on_build_host(&sprite_content, "pirate_raider").unwrap();
    let key = format!("\"{row}\": (");
    let start = text.find(&key).expect("the raider has the clip");
    // The clip ends at the parenthesis that closes the one after its key.
    let open = start + key.len() - 1;
    let mut depth = 0;
    let mut end = open;
    for (offset, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open + offset + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let after = text[end..].strip_prefix(',').unwrap_or(&text[end..]);
    let text = format!("{}{after}", &text[..start]);
    let close = text.rfind(')').unwrap();
    let text = format!("{}    baked_clips: [\"{row}\"],\n{}", &text[..close], &text[close..]);
    let hybrid = RiggedSpriteAsset::from_published_ron(&text).expect("the hybrid parses");
    let spec = try_load_spec_for_character_id(&sprite_content, "pirate_raider").unwrap();
    hybrid.check_rows(spec.row_names()).expect("the hybrid states every row");
    hybrid
}

/// Rig packet 9: one body moves between a part clip and a baked clip. The
/// same root, animator and slots draw both, so nothing is spawned and the
/// root does not move.
#[test]
fn a_hybrid_body_crosses_between_part_and_baked_clips_in_place() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::rigged::ClipRealization;
    use ambition_sprite_sheet::character::CharacterAnim;

    let hybrid = hybrid_raider(&sprite_content, "slash");
    assert_eq!(hybrid.realization("slash"), Some(ClipRealization::Baked));
    assert_eq!(hybrid.realization("idle"), Some(ClipRealization::Parts));
    let (mut app, root) = app_with(&sprite_content, true, raider_with(&sprite_content, Some(hybrid)));
    settle(&mut app);
    let owner = owner(&app, root);
    let slot_ids = app.world().get::<RiggedPresentation>(owner).unwrap().slots.clone();
    let placed = *app.world().get::<Transform>(root).unwrap();
    let parts = |app: &App| slots(app, owner).iter().filter(|(_, visible)| *visible).count();
    assert!(draws_impostor(&app, root, owner), "idle is a part clip: the root draws its impostor");
    assert!(parts(&app) > 0);

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Slash);
    app.update();
    assert!(draws_baked(&app, root), "slash is baked: the root draws its frame");
    let index = app.world().get::<CharacterAnimator>(root).unwrap().atlas_index();
    assert_eq!(app.world().get::<Sprite>(root).unwrap().texture_atlas.as_ref().unwrap().index, index);
    assert_eq!(
        *app.world().get::<Visibility>(owner).unwrap(),
        Visibility::Hidden,
        "the parts draw under a baked clip"
    );

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Idle);
    app.update();
    assert!(draws_impostor(&app, root, owner), "back to parts");
    assert!(parts(&app) > 0);
    assert_eq!(self::owner(&app, root), owner);
    assert_eq!(app.world().get::<RiggedPresentation>(owner).unwrap().slots, slot_ids);
    assert_eq!(*app.world().get::<Transform>(root).unwrap(), placed);
}

/// ⛔ A page render clears the WHOLE page. A body drawn baked meanwhile (a
/// hybrid on its baked clip: its parts hidden) loses its cell's pixels when
/// another body's change renders the page, so on its return to the very same
/// part frame its cell is stale and must be redrawn — not judged unchanged by
/// its own last draws (GPT review, 2026-10-03: the body vanished until
/// something else dirtied the page).
#[test]
fn a_cell_cleared_while_its_body_drew_baked_is_redrawn_when_it_returns() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::CharacterAnim;

    // The app's own root fills the first page (one cell), so A and B share
    // the second.
    let (mut app, _filler) = app_with(&sprite_content, true, raider_with(&sprite_content, Some(hybrid_raider(&sprite_content, "slash"))));
    settle(&mut app);
    let a = {
        let asset = raider_with(&sprite_content, Some(hybrid_raider(&sprite_content, "slash")));
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    let b = {
        let asset = raider_with(&sprite_content, Some(hybrid_raider(&sprite_content, "slash")));
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    pin_clip(&mut app, a, "idle", 0);
    pin_clip(&mut app, b, "idle", 0);
    app.update();
    app.update();
    let presentation = |app: &App, root| app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().impostor.expect("an impostor cell");
    let (pa, pb) = (presentation(&app, a), presentation(&app, b));
    assert_eq!((pa.class, pa.page), (pb.class, pb.page), "the premise: both bodies share a page");
    let generation = |app: &App| app.world().resource::<RiggedImpostorAtlas>().0[pa.class][pa.page].generation;
    let settled = generation(&app);
    app.update();
    assert_eq!(generation(&app), settled, "nothing changed: the page does not render");

    // A goes baked; B changes, so the page renders (and clears) without A.
    app.world_mut().get_mut::<CharacterAnimator>(a).unwrap().request(CharacterAnim::Slash);
    pin_clip(&mut app, b, "idle", 1);
    app.update();
    assert!(draws_baked(&app, a), "the premise: A draws its baked frame");
    assert_eq!(generation(&app), settled + 1, "B's change renders the page");

    // A returns to exactly the part frame its cell held before the clear.
    pin_clip(&mut app, a, "idle", 0);
    app.update();
    assert_eq!(generation(&app), settled + 2, "A's cleared cell is redrawn on its return");
    app.update();
    assert_eq!(generation(&app), settled + 2, "and then rests");
}

/// The root draws its baked frame until every part page is ready, and the
/// parts take over in one frame when they are (drawn directly that frame, its
/// cell's new page rendering from the next).
#[test]
fn a_body_stays_baked_until_every_part_page_is_ready() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app_with(&sprite_content, true, raider(&sprite_content, false));
    let pending: Vec<Handle<Image>> = (0..RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").unwrap().pages.len())
        .map(|_| pending_page(&mut app))
        .collect();
    let mut pages = pending.clone().into_iter();
    let sheet = with_pages(raider(&sprite_content, true), || pages.next().unwrap());
    app.world_mut().resource_mut::<GameAssets>().characters.publish(&sprite_content, "raider", sheet);
    for _ in 0..5 {
        app.update();
        assert!(no_presentations(&mut app), "bound to pages still loading");
        assert!(draws_baked(&app, root), "the baked root stopped drawing");
    }

    for page in &pending {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(page.id(), Image::default())
            .unwrap();
    }
    app.update();
    let owner = owner(&app, root);
    assert!(slots(&app, owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
    app.update();
    assert!(draws_impostor(&app, root, owner));
}

/// A quality-tier change keeps drawing the parts of the old tier until every
/// page of the new tier is ready, and then changes in one frame.
#[test]
fn a_tier_change_keeps_the_old_parts_until_the_new_pages_are_ready() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.update();
    let full_owner = owner(&app, root);

    let quarter = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider")
        .unwrap()
        .for_tier(&sprite_content, TextureResolutionScale::Quarter)
        .expect("the raider publishes a quarter tier")
        .expect("the quarter tier is the raider's");
    let mut sheet = raider_with(&sprite_content, Some(quarter));
    sheet.requested_tier = TextureResolutionScale::Quarter;
    sheet.resolved_tier = TextureResolutionScale::Quarter;
    let pending: Vec<Handle<Image>> = sheet.rigged.as_ref().unwrap().pages.iter().map(|_| pending_page(&mut app)).collect();
    let mut pages = pending.clone().into_iter();
    let sheet = with_pages(sheet, || pages.next().unwrap());
    app.world_mut().resource_mut::<GameAssets>().characters.publish(&sprite_content, "raider", sheet);
    app.world_mut().get_mut::<BoundSpriteQuality>(root).unwrap().scale = TextureResolutionScale::Quarter;
    for _ in 0..5 {
        app.update();
        assert_eq!(owner(&app, root), full_owner, "left the full tier's parts for pages still loading");
        assert!(draws_impostor(&app, root, full_owner));
        assert!(slots(&app, full_owner).iter().any(|(_, visible)| *visible));
    }

    for page in &pending {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(page.id(), Image::default())
            .unwrap();
    }
    app.update();
    let quarter_owner = owner(&app, root);
    assert_ne!(quarter_owner, full_owner);
    assert!(app.world().get_entity(full_owner).is_err(), "the full tier's parts outlived the change");
    let presentation = app.world().get::<RiggedPresentation>(quarter_owner).unwrap();
    assert!(presentation.pages.flipbook.texel_scale < 1.0);
    assert!(draws_impostor(&app, root, quarter_owner));
    assert!(slots(&app, quarter_owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
}

/// Review of the readiness guard (2026-09-30): a re-wear to another rigged
/// character whose pages still load drops the old character's parts in the
/// same frame. The root draws the new character's baked sheet until its
/// pages are ready, and then its parts. Only a tier change of one character
/// keeps the old parts meanwhile.
#[test]
fn a_rewear_drops_the_old_characters_parts_while_the_new_pages_load() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.update();
    let raider_owner = owner(&app, root);

    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "pirate_lookout").expect("the lookout publishes a flipbook");
    let mut pending = Vec::new();
    let lookout = with_pages(sheet_with(&sprite_content, "pirate_lookout", Some(flipbook)), || {
        let page = pending_page(&mut app);
        pending.push(page.clone());
        page
    });
    app.world_mut().resource_mut::<GameAssets>().characters.publish(&sprite_content, "lookout", lookout.clone());
    let feet = Vec2::new(lookout.spec.feet_anchor_x, lookout.spec.feet_anchor_y);
    let (sprite, anchor, animator) = build_character_presentation_with_render_size(&lookout, RENDER, Anchor(feet));
    app.world_mut().entity_mut(root).insert((sprite, anchor, animator));
    for _ in 0..3 {
        app.update();
        assert!(
            app.world().get_entity(raider_owner).is_err(),
            "the raider's parts stayed on the lookout's root while its pages loaded"
        );
        assert!(no_presentations(&mut app), "bound to pages still loading");
        assert!(draws_baked(&app, root), "the baked root is not drawn");
    }

    for page in &pending {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(page.id(), Image::default())
            .unwrap();
    }
    app.update();
    let lookout_owner = owner(&app, root);
    assert_eq!(app.world().get::<RiggedPresentation>(lookout_owner).unwrap().target, "content://ambition_shared/sprites/pirate_lookout");
    assert!(slots(&app, lookout_owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
    // The raider's page was retired with its last body; the lookout's is new,
    // so it is composited from the next frame.
    app.update();
    assert!(draws_impostor(&app, root, lookout_owner));
}

/// A sheet with no row for the compact pose is squashed: the animator draws
/// the root's quad shorter about a line that holds still. The impostor quad
/// takes the same squash about the same line, for both pivots `StanceSquash`
/// uses: the anchor (a feet-anchored quad) and the quad's own foot edge (an
/// authored-offset quad).
#[test]
fn a_squashed_root_squashes_its_impostor_about_the_same_line() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    // A rigged character still published with a packed, trimmed sheet: the
    // game squashes only a trimmed frame (`draw_animator_frame`), and the
    // part-first characters (the raider among them) are drawn untrimmed.
    const TRIMMED: &str = "mary_o_v2_cosmic";
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, TRIMMED).expect("a published flipbook");
    let sheet = sheet_with(&sprite_content, TRIMMED, Some(flipbook));
    assert!(sheet.spec.is_trimmed(), "the fixture must be a trimmed sheet");
    let (mut app, root) = app_with(&sprite_content, true, sheet);
    settle(&mut app);
    let (h0, a0) = {
        let animator = app.world().get::<CharacterAnimator>(root).unwrap();
        let (size, anchor) = animator.current_render().unwrap();
        (size.y, anchor.y)
    };
    let quad = |app: &App| {
        (
            app.world().get::<Sprite>(root).unwrap().custom_size.unwrap().y,
            app.world().get::<Anchor>(root).unwrap().0.y,
        )
    };
    let (full_h, full_a) = quad(&app);
    // The local y of a quad's point at normalized height `t` (0 bottom).
    let at = |h: f32, a: f32, t: f32| (t - (a + 0.5)) * h;

    // About the anchor: the height halves, the anchor (the feet) holds.
    app.world_mut().resource_mut::<Squash>().0 = Some((0.5, false));
    app.update();
    let (h, a) = quad(&app);
    assert!((h - full_h * 0.5).abs() < 1.0e-3, "{h} for {full_h}");
    assert!((at(h, a, a + 0.5) - at(full_h, full_a, full_a + 0.5)).abs() < 1.0e-3);

    // About the baked quad's foot edge: that line holds.
    let ratio = 0.6;
    app.world_mut().resource_mut::<Squash>().0 = Some((ratio, true));
    app.update();
    let (h, a) = quad(&app);
    assert!((h - full_h * ratio).abs() < 1.0e-3);
    let foot = -(a0 + 0.5) * h0;
    // The point of the impostor quad that sat on the baked foot edge is still
    // there: in the unsquashed quad it was at `foot`; after, it must map to
    // `foot` too.
    let t_full = foot / full_h + (full_a + 0.5);
    assert!((at(h, a, t_full) - foot).abs() < 1.0e-3, "the foot edge moved: {} vs {foot}", at(h, a, t_full));
}

/// A tweened clip (Mary-O's walk) draws `frame_phase` of the way to the next
/// frame: half-way through a frame, each slot sits half-way between its place
/// in this frame and in the next (`RiggedSpriteAsset::tween_into`).
#[test]
fn a_tweened_clip_draws_between_its_frames() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2_tall").expect("a published flipbook");
    assert_eq!(flipbook.clip("walk").unwrap().tween, ClipTween::Linear);
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "mary_o_v2_tall", Some(flipbook.clone())));
    app.update();
    let owner = owner(&app, root);
    let duration = {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(ambition_sprite_sheet::character::CharacterAnim::Walk);
        animator.frame = 0;
        animator.elapsed = 0.0;
        let row = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
        assert_eq!(row, "walk");
        ambition_sprite_sheet::character::sheets::record_for_sheet_key(&sprite_content, "mary_o_v2_tall")
            .unwrap()
            .rows
            .iter()
            .find(|sheet_row| sheet_row.animation == row)
            .unwrap()
            .duration_secs
    };
    app.update();
    // The change from idle to walk starts a clip blend (POSE-CONTINUITY), and
    // this clock does not run it down: settle it, so the frames measured are
    // the walk's own.
    app.world_mut().get_mut::<RiggedPresentation>(owner).unwrap().blend_left = 0.0;
    app.update();
    let at_frame = slots(&app, owner);
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().elapsed = duration * 0.5;
    app.update();
    let halfway = slots(&app, owner);
    let (this, next) = (flipbook.frame("walk", 0).unwrap(), flipbook.frame("walk", 1).unwrap());
    let mut moved = 0;
    for (index, draw) in this.iter().enumerate() {
        let Some(target) = next.iter().find(|next| next.track == draw.track && next.part == draw.part) else {
            continue;
        };
        let expected = at_frame[index].0 + Vec2::new(target.at.x - draw.at.x, -(target.at.y - draw.at.y)) * 0.5;
        assert!(close(halfway[index].0, expected), "slot {index}: {:?}, expected {expected:?}", halfway[index].0);
        moved += usize::from(target.at != draw.at);
    }
    assert!(moved > 0, "the walk's first two frames move no part, so this tests nothing");
}

/// POSE-CONTINUITY: a change between two carriage poses is drawn as a blend
/// from the pose before it, and a strike is not. From idle to walk, the first
/// frame after the change is the idle pose; once the blend has run its time,
/// the walk's own. The same body changing into a slash draws the slash at once.
#[test]
fn a_change_between_carriage_poses_blends_and_a_strike_does_not() {
    use ambition_sprite_sheet::character::CharacterAnim;
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2_tall").expect("a published flipbook");
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "mary_o_v2_tall", Some(flipbook)));
    app.update();
    let owner = owner(&app, root);
    let set = |app: &mut App, anim: CharacterAnim| {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(anim);
        animator.frame = 0;
        animator.elapsed = 0.0;
    };
    set(&mut app, CharacterAnim::Idle);
    app.update();
    app.update();
    let idle = slots(&app, owner);

    set(&mut app, CharacterAnim::Walk);
    app.update();
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    assert!(presentation.blend_left > 0.0, "idle to walk starts a blend");
    let first = slots(&app, owner);
    app.world_mut().get_mut::<RiggedPresentation>(owner).unwrap().blend_left = 0.0;
    app.update();
    let walk = slots(&app, owner);
    assert_ne!(idle, walk, "premise: the walk's first frame is not the idle pose");
    let from_idle = first.iter().zip(&idle).filter(|(a, b)| close(a.0, b.0)).count();
    let from_walk = first.iter().zip(&walk).filter(|(a, b)| close(a.0, b.0)).count();
    assert!(from_idle > from_walk, "the first frame of the change is drawn from the idle pose ({from_idle} parts) more than the walk ({from_walk})");

    set(&mut app, CharacterAnim::Slash);
    app.update();
    assert_eq!(
        app.world().get::<RiggedPresentation>(owner).unwrap().blend_left,
        0.0,
        "a strike is drawn sharp, with no blend"
    );
}

/// The impostor lands where the baked frame would, whatever the root's anchor:
/// a frame pixel is drawn at the same root-local point from the cell quad as
/// from the baked FULL frame (`render_size` at the basis anchor), for a
/// feet-anchored NPC and for a centre-anchored player (`character_render_basis`
/// gives a body with a sheet-authored quad `Anchor::CENTER`), facing both ways.
///
/// ⛔ The first impostor assumed the feet and drew every centre-anchored body —
/// Mary-O — half a body too high; every other test here built feet-anchored
/// roots, so none could see it.
#[test]
fn the_impostor_lands_where_the_baked_frame_would_for_either_anchor() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2_tall").expect("a published flipbook");
    let frame = flipbook.frame_size.as_vec2();
    let spec = try_load_spec_for_character_id(&sprite_content, "mary_o_v2_tall").unwrap();
    // Map a point in the source frame to the quad with the specified axis signs.
    let local = |size: Vec2, anchor: Vec2, uv: Vec2, signs: Vec2| {
        (Vec2::new(uv.x - 0.5, 0.5 - uv.y) * signs - anchor) * size
    };
    // The anchors the game builds bodies with, from the builder the game uses
    // (`character_render_basis`): an NPC's (no authored quad) and a player's
    // with a sheet-authored quad. Asked of the builder, not written here, so a
    // third convention would be held too.
    let collision = Vec2::new(21.0, 32.0);
    let npc = crate::rendering::actors::character_render_basis(&spec, collision, None, None).1;
    let player =
        crate::rendering::actors::character_render_basis(&spec, collision, Some(Vec2::new(61.0, 73.0)), Some(Vec2::ZERO)).1;
    assert_ne!(npc.0, player.0, "premise: the two conventions differ");
    for built_at in [npc, player] {
        for signs in [Vec2::ONE, Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0), -Vec2::ONE] {
            let (mut app, root) = app_anchored(&sprite_content, true, sheet_with(&sprite_content, "mary_o_v2_tall", Some(flipbook.clone())), built_at);
            let mut sprite = app.world_mut().get_mut::<Sprite>(root).unwrap();
            sprite.flip_x = signs.x < 0.0;
            sprite.flip_y = signs.y < 0.0;
            settle(&mut app);
            let owner = owner(&app, root);
            assert!(draws_impostor(&app, root, owner));
            let size = app.world().get::<Sprite>(root).unwrap().custom_size.unwrap();
            let anchor = app.world().get::<Anchor>(root).unwrap().0;
            let basis = app.world().get::<CharacterAnimator>(root).unwrap().render_basis.unwrap();
            let base_anchor = basis.feet_anchor * signs;
            for pixel in [flipbook.feet_pixel, Vec2::ZERO, frame, Vec2::new(frame.x, 0.0)] {
                let baked = local(basis.render_size, base_anchor, pixel / frame, signs);
                let cell = local(size, anchor, (pixel + Vec2::splat(impostor_margin(&flipbook))) / IMPOSTOR_CELL, signs);
                assert!(
                    close(cell, baked),
                    "{built_at:?} signs {signs}: frame pixel {pixel} draws at {cell} from the impostor, {baked} baked"
                );
            }
        }
    }
}

/// A body facing left on a sheet drawn from both sides (`SheetRow::mirror_of`)
/// draws its MIRROR row, unflipped, with its feet at the mirrored anchor. The
/// impostor puts that frame where the baked road does.
///
/// ⛔ The quad mirrored the feet anchor only for `flip_x`. The player robot's
/// mirror rows drew 3 to 5 px off its baked frame facing left, in all 944 of
/// its left-facing frames (in-engine parity, 2026-10-04); facing right, and
/// pinned to a mirror row, it matched.
#[test]
fn a_mirror_row_drawn_facing_left_lands_where_the_baked_frame_would() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "player_robot_v3").expect("the robot publishes a flipbook");
    let frame = flipbook.frame_size.as_vec2();
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "player_robot_v3", Some(flipbook.clone())));
    app.update();
    {
        // As the game faces a body (`apply_character_frame`).
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(ambition_sprite_sheet::character::CharacterAnim::Idle);
        assert!(!animator.face(true), "premise: the robot answers a left facing with its mirror row");
    }
    app.update();
    let owner = owner(&app, root);
    assert!(draws_impostor(&app, root, owner));
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    assert!(animator.draws_mirror_row(), "premise: the frame drawn is the mirror row");
    let sprite = app.world().get::<Sprite>(root).unwrap();
    assert!(!sprite.flip_x, "premise: a mirror row is drawn unflipped");
    let size = sprite.custom_size.unwrap();
    let anchor = app.world().get::<Anchor>(root).unwrap().0;
    // The baked road's full frame for a mirror row: the render size, at the
    // feet anchor mirrored (`CharacterAnimator::current_render`).
    let basis = animator.render_basis.unwrap();
    let baked_anchor = Vec2::new(-basis.feet_anchor.x, basis.feet_anchor.y);
    let local = |size: Vec2, anchor: Vec2, uv: Vec2| (Vec2::new(uv.x - 0.5, 0.5 - uv.y) - anchor) * size;
    for pixel in [flipbook.feet_pixel, Vec2::ZERO, frame, Vec2::new(frame.x, 0.0)] {
        let baked = local(basis.render_size, baked_anchor, pixel / frame);
        let cell = local(size, anchor, (pixel + Vec2::splat(impostor_margin(&flipbook))) / IMPOSTOR_CELL);
        assert!(
            close(cell, baked),
            "facing left: frame pixel {pixel} draws at {cell} from the impostor, {baked} baked"
        );
    }
}

/// The player robot's flipbook, published (`player_robot_v3.py`): every row
/// from parts, placed between pixels.
fn robot(sprite_content: &ambition_sprite_sheet::SpriteContent) -> (RiggedSpriteAsset, App, Entity) {
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "player_robot_v3").expect("the robot publishes a flipbook");
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "player_robot_v3", Some(flipbook.clone())));
    app.update();
    (flipbook, app, root)
}

/// Pin `root` to frame `frame` of the clip row `row`.
fn pin_clip(app: &mut App, root: Entity, row: &str, frame: usize) {
    let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
    animator.request_clip([row], ambition_sprite_sheet::character::CharacterAnim::Idle);
    animator.frame = frame;
    animator.elapsed = 0.0;
    let drawn = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
    assert_eq!(drawn, row, "the pin did not reach the row");
}

/// A frame that fades AS ONE PICTURE (the robot's death) fades its body's
/// cell, after the parts are composited (`ImpostorCellOpacity`): never its
/// parts one by one, which would show them through each other. Every other
/// cell stays opaque, and the cell is opaque again once the clip ends.
#[test]
fn a_frame_that_fades_as_one_picture_fades_its_cell() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (flipbook, mut app, root) = robot(&sprite_content);
    let owner = owner(&app, root);
    let last = flipbook.clip("death").expect("a death clip").frame_count() - 1;
    let fade = flipbook.frame_opacity("death", last);
    assert!(fade < 0.6, "the death clip does not fade ({fade}), so this tests nothing");
    pin_clip(&mut app, root, "death", last);
    app.update();
    let cell = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell").cell as usize;
    let cells = atlas(&app).cells.clone();
    assert_eq!(cells.opacity[cell / 4][cell % 4], fade);
    let others = (0..IMPOSTOR_MAX_CELLS).filter(|other| *other != cell);
    assert!(others.into_iter().all(|other| cells.opacity[other / 4][other % 4] == 1.0));
    for slot in &app.world().get::<RiggedPresentation>(owner).unwrap().slots {
        let alpha = app.world().get::<Sprite>(*slot).unwrap().color.alpha();
        assert_eq!(alpha, 1.0, "a death frame's parts are drawn opaque; the cell fades");
    }
    pin_clip(&mut app, root, "idle", 0);
    app.update();
    assert_eq!(atlas(&app).cells.opacity[cell / 4][cell % 4], 1.0);
}

/// The robot's published flipbook with one part of its body (track 0) half
/// clear in each frame of `row`: a sheet with a fade of its own there.
fn robot_that_fades_its_body_in(sprite_content: &ambition_sprite_sheet::SpriteContent, row: &str) -> RiggedSpriteAsset {
    let text = ambition_sprite_sheet::baked_part_flipbooks::published_ron_on_build_host(&sprite_content, "player_robot_v3").unwrap();
    let key = format!("\"{row}\": (");
    let start = text.find(&key).expect("the robot has the clip");
    // The clip ends at the parenthesis that closes the one after its key.
    let open = start + key.len() - 1;
    let (mut depth, mut end) = (0, open);
    for (offset, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open + offset + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let clip = text[open..end].replace("track: 0),", "track: 0, opacity: 0.5),");
    assert_ne!(clip, text[open..end], "the row has no draw of track 0 to fade");
    let text = format!("{}{clip}{}", &text[..open], &text[end..]);
    RiggedSpriteAsset::from_published_ron(&text).expect("the variant parses")
}

/// A body on a blink row is taken apart in its cell by the engine's warp, by
/// the ROW it draws (no preview): the cell's warp says a departure and how far
/// through the row the frame is, and it is gone when the row ends. The row's
/// own frame is drawn whole, for the warp to cut.
///
/// The control is a sheet that fades its own body in that row: its cell has
/// no warp, and it keeps its own blink. Each published sheet draws a plain
/// blink row since 2026-10-09, so the control is the robot's own table with
/// one body part of that row made half clear.
#[test]
fn a_blink_row_warps_its_bodys_cell_and_a_sheet_with_its_own_blink_keeps_it() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (flipbook, mut app, root) = robot(&sprite_content);
    assert!(
        flipbook.row_draws_the_body_whole("blink_out"),
        "premise: the robot's blink row is a plain pose. The sheets are untracked: if this machine's is from \
         before 2026-10-09, publish it again: scripts/regen/sprites.sh --target player_robot_v3"
    );
    let owner_of = owner(&app, root);
    let last = flipbook.clip("blink_out").expect("a blink clip").frame_count() - 1;
    assert!(last >= 2, "the blink clip has too few frames to be part way through");
    pin_clip(&mut app, root, "blink_out", last / 2);
    app.update();
    let cell = app.world().get::<RiggedPresentation>(owner_of).unwrap().impostor.expect("an impostor cell").cell as usize;
    let warp = atlas(&app).cells.warp[cell];
    assert_eq!(warp.x, 1.0, "the cell has no departure warp: {warp:?}");
    assert!(warp.y > 0.2 && warp.y < 0.8, "the warp is not part way through the row: {warp:?}");
    assert!(warp.w > warp.z, "the warp has no span to cut: {warp:?}");
    let others = (0..IMPOSTOR_MAX_CELLS).filter(|other| *other != cell);
    assert!(others.into_iter().all(|other| atlas(&app).cells.warp[other] == Vec4::ZERO));
    pin_clip(&mut app, root, "blink_in", 0);
    app.update();
    assert_eq!(atlas(&app).cells.warp[cell].x, 2.0, "the arrival row has the arrival warp");
    pin_clip(&mut app, root, "idle", 0);
    app.update();
    assert_eq!(atlas(&app).cells.warp[cell], Vec4::ZERO, "the warp stays after the row ends");

    // The control.
    let own = robot_that_fades_its_body_in(&sprite_content, "blink_out");
    assert!(!own.row_draws_the_body_whole("blink_out"), "control: the variant draws its body whole");
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "player_robot_v3", Some(own)));
    app.update();
    pin_clip(&mut app, root, "blink_out", 2);
    app.update();
    // No cell of an atlas has a warp (its body is composited to fade).
    let warped = app
        .world()
        .resource::<RiggedImpostorAtlas>()
        .0
        .iter()
        .flatten()
        .any(|atlas| atlas.cells.warp.iter().any(|warp| *warp != Vec4::ZERO));
    assert!(!warped, "a sheet with its own blink got the engine's on top of it");
}

/// The drawn row of `root`, by name.
fn drawn_row(app: &App, root: Entity) -> String {
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned()
}

/// The warp of `root`'s cell.
fn warp_of(app: &App, root: Entity) -> Vec4 {
    let owner = owner(app, root);
    let cell = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell").cell as usize;
    atlas(app).cells.warp[cell]
}

/// A body that performs a blink needs no blink row. A sheet with none draws
/// its idle for the request, and the engine's warp is on the body over that
/// row, by the time since the body asked (Jon, 2026-10-09: "just using the
/// engine effect on a idle pose might be fine").
///
/// The control is the same body when it asks for its idle: the same row, and
/// no warp.
#[test]
fn a_body_with_no_blink_row_that_asks_for_a_blink_is_warped_on_the_row_it_draws() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::rigged::BLINK_WARP_S;
    use ambition_sprite_sheet::character::CharacterAnim;
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").expect("the raider publishes a flipbook");
    assert!(
        flipbook.clip("blink_out").is_none() && flipbook.clip("blink_in").is_none(),
        "premise: the raider has no blink row; choose another sheet"
    );
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "pirate_raider", Some(flipbook)));
    settle(&mut app);
    let idle = drawn_row(&app, root);
    assert_eq!(warp_of(&app, root), Vec4::ZERO, "control: a body that asks for its idle has a warp");

    let ask = |app: &mut App, anim: CharacterAnim, for_s: f32| {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(anim);
        animator.tick(for_s);
        app.update();
    };
    ask(&mut app, CharacterAnim::BlinkOut, BLINK_WARP_S * 0.5);
    assert_eq!(drawn_row(&app, root), idle, "premise: the sheet draws its idle for a blink it has no row for");
    let warp = warp_of(&app, root);
    assert_eq!(warp.x, 1.0, "no departure warp on a body that asks for a blink: {warp:?}");
    assert!((warp.y - 0.5).abs() < 0.05, "the warp is not half way after half its time: {warp:?}");
    // It stays apart while the body asks, as a blink row holds its last frame.
    ask(&mut app, CharacterAnim::BlinkOut, BLINK_WARP_S * 4.0);
    assert_eq!(warp_of(&app, root).y, 1.0);
    ask(&mut app, CharacterAnim::BlinkIn, 0.0);
    let warp = warp_of(&app, root);
    assert_eq!((warp.x, warp.y), (2.0, 0.0), "the arrival starts from its start: {warp:?}");
    ask(&mut app, CharacterAnim::Idle, 0.0);
    assert_eq!(warp_of(&app, root), Vec4::ZERO, "the warp stays after the body asks for its idle");
}

/// A blink that something performs ON a body is on whatever row the body
/// draws, with the animation it has (`PerformedBodyWarp`): the body asks for
/// nothing. The renderer runs its clock and takes it off at its end.
#[test]
fn a_blink_performed_on_a_body_warps_it_on_the_row_it_draws_and_ends() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::rigged::PerformedBodyWarp;
    let (_flipbook, mut app, root) = robot(&sprite_content);
    app.init_resource::<Time>();
    app.add_systems(Update, advance_performed_body_warps.before(RiggedDriveSet));
    pin_clip(&mut app, root, "walk", 1);
    settle(&mut app);
    assert_eq!(warp_of(&app, root), Vec4::ZERO, "control: a walking body has a warp");

    app.world_mut().entity_mut(root).insert(PerformedBodyWarp::new(BodyWarp::TeleportIn, 0.4));
    let step = |app: &mut App, seconds: f32| {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    };
    step(&mut app, 0.1);
    assert_eq!(drawn_row(&app, root), "walk", "the body keeps the animation it has");
    let warp = warp_of(&app, root);
    assert_eq!(warp.x, 2.0, "no arrival warp: {warp:?}");
    assert!((warp.y - 0.25).abs() < 0.01, "the warp is not a quarter through: {warp:?}");
    step(&mut app, 0.4);
    step(&mut app, 0.0);
    assert!(app.world().get::<PerformedBodyWarp>(root).is_none(), "the warp is on the body after its end");
    assert_eq!(warp_of(&app, root), Vec4::ZERO);
}

/// A body's colour shift (`CharacterColorShift`: an enemy variant, a buff)
/// reaches its cell of the page's material, and only its cell: one sheet
/// draws every coloured variant. A body without one is drawn as painted.
#[test]
fn a_color_shift_reaches_its_bodys_cell() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::CharacterColorShift;
    let (_flipbook, mut app, root) = robot(&sprite_content);
    let owner = owner(&app, root);
    let cell = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell").cell as usize;
    assert_eq!(atlas(&app).cells.shift[cell], CharacterColorShift::NONE.as_uniform(), "premise: unshifted");
    let shift = CharacterColorShift {
        hue_degrees: 120.0,
        saturation: 0.8,
        value: 1.1,
    };
    app.world_mut().entity_mut(root).insert(shift);
    app.update();
    let cells = &atlas(&app).cells;
    assert_eq!(cells.shift[cell], shift.as_uniform());
    for (other, uniform) in cells.shift.iter().enumerate() {
        if other != cell {
            assert_eq!(*uniform, CharacterColorShift::NONE.as_uniform(), "cell {other} took another body's shift");
        }
    }
    app.world_mut().entity_mut(root).remove::<CharacterColorShift>();
    app.update();
    assert_eq!(atlas(&app).cells.shift[cell], CharacterColorShift::NONE.as_uniform(), "removing the shift restores the colours");
}

/// A body drawn from its other side draws its mirror row (`~mirrored`), whose
/// draws are the same parts mirrored about their pivots (`scale.x == -1`):
/// each slot carries the draw's scale in its transform.
#[test]
fn a_mirrored_draw_mirrors_its_slot() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (flipbook, mut app, root) = robot(&sprite_content);
    let owner = owner(&app, root);
    {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(ambition_sprite_sheet::character::CharacterAnim::Idle);
        assert!(!animator.face(true), "the robot answers a flip with its mirror rows");
    }
    app.update();
    let draws = flipbook.frame("idle~mirrored", 0).expect("a mirror row");
    assert!(draws.iter().any(|draw| draw.scale.x < 0.0), "the mirror row has no mirrored draw");
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    for (draw, slot) in draws.iter().zip(&presentation.slots) {
        let scale = app.world().get::<Transform>(*slot).unwrap().scale;
        assert_eq!(scale.truncate(), draw.scale);
    }
}

/// A part that fades on its own (the robot's blade after a smash) draws its
/// slot at the draw's opacity.
#[test]
fn a_faded_draw_tints_its_slot() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (flipbook, mut app, root) = robot(&sprite_content);
    let owner = owner(&app, root);
    let frames = flipbook.clip("smash_forward").expect("a smash clip").frame_count();
    let (frame, index, opacity) = (0..frames)
        .flat_map(|frame| {
            let draws = flipbook.frame("smash_forward", frame).unwrap();
            draws.iter().enumerate().map(move |(index, draw)| (frame, index, draw.opacity()))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .unwrap();
    assert!(opacity < 0.5, "no draw of the smash fades ({opacity}), so this tests nothing");
    pin_clip(&mut app, root, "smash_forward", frame);
    app.update();
    let slot = app.world().get::<RiggedPresentation>(owner).unwrap().slots[index];
    let alpha = app.world().get::<Sprite>(slot).unwrap().color.alpha();
    assert!((alpha - opacity).abs() < 1.0e-6, "{alpha} for {opacity}");
    let rgb = app.world().get::<Sprite>(slot).unwrap().color.to_linear();
    assert_eq!((rgb.red, rgb.green, rgb.blue), (1.0, 1.0, 1.0), "an untinted draw multiplies by white");
}

/// Every published flipbook's frame fits an impostor cell class with its
/// margins. A body whose frame fits none keeps its baked sheet in game, with
/// one warning in a log nobody reads — the robot's 256 px frame did not fit the
/// old single 256 px cell.
#[test]
fn every_published_flipbook_fits_an_impostor_cell() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let mut checked = Vec::new();
    // Full-resolution tables only: a tier's table has the same frame.
    for key in ambition_sprite_sheet::baked_part_flipbooks::baked_part_flipbook_targets(&sprite_content) {
        let flipbook = RiggedSpriteAsset::baked(&sprite_content, key).expect("a published flipbook parses");
        assert!(
            impostor_cell_class(flipbook.frame_size.as_vec2(), impostor_margin(&flipbook)).is_some(),
            "`{key}`'s {} px frame, with its art's {} px margin, fits no impostor cell ({IMPOSTOR_CELL_CLASSES:?})",
            flipbook.frame_size,
            impostor_margin(&flipbook)
        );
        checked.push(key);
    }
    // Mary-O's three forms, the five pirates and the robot, at least.
    assert!(checked.contains(&"content://ambition_shared/sprites/player_robot_v3") && checked.len() >= 9, "{checked:?}");
}

/// A body takes the smallest cell its frame fits: Noether's 496 x 528 frame a
/// 576 px cell of the second atlas, which the root then draws; the first
/// atlas is never built for her.
#[test]
fn a_large_frame_takes_a_cell_of_its_size() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "noether").expect("noether publishes a flipbook");
    assert_eq!(impostor_cell_class(flipbook.frame_size.as_vec2(), impostor_margin(&flipbook)), Some(1));
    assert_eq!(impostor_cell_class(Vec2::new(256.0, 256.0), IMPOSTOR_MARGIN), Some(0));
    assert_eq!(impostor_cell_class(Vec2::new(2000.0, 10.0), IMPOSTOR_MARGIN), None);
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "noether", Some(flipbook.clone())));
    app.update();
    app.update();
    let owner = owner(&app, root);
    let impostor = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.expect("an impostor cell");
    assert_eq!(impostor.class, 1);
    let atlases = &app.world().resource::<RiggedImpostorAtlas>().0;
    assert!(atlases[0].is_empty(), "the first atlas was built for a body that does not fit it");
    let atlas = atlases[1].first().expect("the second atlas");
    assert_eq!(atlas.cell_size(), 576.0);
    let sprite = app.world().get::<Sprite>(root).unwrap();
    assert!(sprite.image == atlas.image, "the root does not draw the second atlas");
    // Its cell's place is the second grid's: no camera of the first sees it.
    let place = app.world().get::<Transform>(owner).unwrap().translation.truncate();
    assert_eq!(place, impostor_cell_feet(1, 0, atlas.side, impostor.cell, impostor.feet));
    assert!(place.y > IMPOSTOR_ORIGIN.y, "{place}");
}

/// The test app with nothing reading its body as one image: the parts draw
/// directly.
fn app_direct(sprite_content: &ambition_sprite_sheet::SpriteContent, sheet: CharacterSpriteAsset, anchor: Anchor) -> (App, Entity) {
    let (mut app, root) = app_anchored(&sprite_content, true, sheet, anchor);
    app.insert_resource(Composite(false));
    (app, root)
}

/// Review finding #7 (2026-10-19). A part-drawn body that its passage
/// fades is composited: it fades as one picture. Control: a passage look at
/// full opacity leaves it drawn directly. Measured before: nothing declared
/// it, so it was drawn directly and each part took the fade, showing the
/// parts under it. Poison: declare nothing for a passage.
#[test]
fn a_body_its_passage_fades_is_composited() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    for (alpha, composited) in [(0.5, true), (1.0, false)] {
        let sheet = raider(&sprite_content, true);
        let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
        let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
        app.add_systems(
            Update,
            crate::rendering::passage_visuals::declare_passage_fade_demand.before(bind_rigged_presentations),
        );
        app.world_mut().entity_mut(root).insert((
            crate::rendering::primitives::PlayerVisual,
            ambition_sim_view::PassageLookView { scale: 0.9, alpha, shade: 0.2, transit: 0.0 },
        ));
        settle(&mut app);
        let owner = owner(&app, root);
        assert_eq!(
            app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_some(),
            composited,
            "a passage look at opacity {alpha}"
        );
    }
}

/// A body nothing reads as one image draws its parts in the world: no cell,
/// no atlas page, no camera; its root draws no image; its slots draw in the
/// root's own layers, under an owner that follows the root's visibility.
#[test]
fn a_body_nothing_reads_draws_its_parts_in_the_world() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    let sheet = raider(&sprite_content, true);
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
    app.update();
    let owner = owner(&app, root);
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    assert!(presentation.impostor.is_none(), "a body nothing reads took a cell");
    assert!(app.world().resource::<RiggedImpostorAtlas>().0.iter().all(Vec::is_empty), "an atlas page was built");
    let mut cameras = app.world_mut().query::<&RiggedImpostorCamera>();
    assert_eq!(cameras.iter(app.world()).count(), 0);
    assert_eq!(app.world().get::<Sprite>(root).unwrap().image, NO_BAKED_IMAGE, "the root draws an image over its parts");
    let drawn = slots(&app, owner);
    let expected = frame_draws(&app, root, "pirate_raider");
    assert_eq!(drawn.iter().filter(|(_, visible)| *visible).count(), expected.len());
    for ((at, _), want) in drawn.iter().zip(&expected) {
        assert!(close(*at, *want), "a part at {at:?}, its draw at {want:?}");
    }
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    for slot in &presentation.slots {
        assert_eq!(app.world().get::<RenderLayers>(*slot), Some(&RenderLayers::default()));
    }
    assert_eq!(*app.world().get::<Visibility>(owner).unwrap(), Visibility::Inherited);

    // A root in another layer (a room that is not live, a pane) takes its
    // parts with it; a hidden root hides them, on the frame it is hidden.
    let pane = RenderLayers::layer(5);
    app.world_mut().entity_mut(root).insert((pane.clone(), Visibility::Hidden));
    app.update();
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    for slot in &presentation.slots {
        assert_eq!(app.world().get::<RenderLayers>(*slot), Some(&pane), "a part left its root's layer");
    }
    assert_eq!(*app.world().get::<Visibility>(owner).unwrap(), Visibility::Hidden, "a hidden root's parts draw");
}

/// A part drawn directly lands where its pixel of the baked frame would, for
/// both anchors the game builds bodies with, both facings, and a root that is
/// turned: the owner stands at the feet, scaled from sheet pixels to world
/// units, and a slot stands at its draw's place from the feet.
///
/// The turn is the root's own (`sync_visuals` writes the body's roll: the
/// gravity of its room, a somersault). The baked quad is drawn through the
/// root's transform, so it turns about the root. A direct body that did not
/// take the turn was drawn level in a room with sideways gravity (2026-10-05).
#[test]
fn a_direct_part_lands_where_the_baked_frame_would_for_either_anchor() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2_tall").expect("a published flipbook");
    let frame = flipbook.frame_size.as_vec2();
    let spec = try_load_spec_for_character_id(&sprite_content, "mary_o_v2_tall").unwrap();
    let collision = Vec2::new(21.0, 32.0);
    let npc = crate::rendering::actors::character_render_basis(&spec, collision, None, None).1;
    let player =
        crate::rendering::actors::character_render_basis(&spec, collision, Some(Vec2::new(61.0, 73.0)), Some(Vec2::ZERO)).1;
    assert_ne!(npc.0, player.0, "premise: the two conventions differ");
    for built_at in [npc, player] {
        for physical_scale in [Vec3::new(0.5, 0.5, 1.0), Vec3::new(-0.5, 0.5, 1.0), Vec3::new(2.0, 2.0, 1.0)] {
        for signs in [Vec2::ONE, Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0), -Vec2::ONE] {
            for turn in [0.0, std::f32::consts::FRAC_PI_2, 0.6] {
                let (mut app, root) = app_direct(&sprite_content, sheet_with(&sprite_content, "mary_o_v2_tall", Some(flipbook.clone())), built_at);
                let mut sprite = app.world_mut().get_mut::<Sprite>(root).unwrap();
                sprite.flip_x = signs.x < 0.0;
                sprite.flip_y = signs.y < 0.0;
                let root_place =
                    Transform::from_translation(Vec3::new(40.0, -12.0, 3.0)).with_rotation(Quat::from_rotation_z(turn)).with_scale(physical_scale);
                *app.world_mut().get_mut::<Transform>(root).unwrap() = root_place;
                app.update();
                let owner = owner(&app, root);
                let place = *app.world().get::<Transform>(owner).unwrap();
                let basis = app.world().get::<CharacterAnimator>(root).unwrap().render_basis.unwrap();
                for pixel in [flipbook.feet_pixel, Vec2::ZERO, frame, Vec2::new(frame.x, 0.0)] {
                    // The baked frame: a quad of the basis size at its anchor,
                    // mirrored about the root when flipped, then drawn through
                    // the root's transform.
                    let baked = (Vec2::new(pixel.x / frame.x - 0.5, 0.5 - pixel.y / frame.y) - basis.feet_anchor) * basis.render_size * signs;
                    let baked = root_place.transform_point(baked.extend(0.0)).truncate();
                    // The same pixel from the parts: sheet pixels from the feet,
                    // +y up, through the owner.
                    let from_feet = Vec2::new(pixel.x - flipbook.feet_pixel.x, flipbook.feet_pixel.y - pixel.y);
                    let direct = place.transform_point(from_feet.extend(0.0)).truncate();
                    assert!(
                        close(direct, baked),
                        "{built_at:?} signs {signs} turn {turn}: frame pixel {pixel} draws at {direct} directly, {baked} baked"
                    );
                }
            }
        }
    }
    }
}

/// A body is composited while something reads it as one image, stays so for
/// [`COMPOSED_HOLD_FRAMES`] after the last read, then draws directly again
/// and gives its cell back.
#[test]
fn a_body_read_as_one_image_is_composited_while_it_is_read() {

    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let sheet = raider(&sprite_content, true);
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
    app.update();
    let owner = owner(&app, root);
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_none());

    let direct_scale = app.world().get::<Transform>(owner).unwrap().scale;
    assert_ne!(direct_scale, Vec3::ONE, "premise: a direct owner is scaled to world units");
    app.insert_resource(Composite(true));
    settle(&mut app);
    assert!(draws_impostor(&app, root, owner), "a body read as one image is not composited");
    assert_eq!(
        app.world().get::<Transform>(owner).unwrap().scale,
        Vec3::ONE,
        "a composited body's parts keep the world scale it was drawn at directly"
    );
    let private = RenderLayers::layer(RIGGED_IMPOSTOR_LAYER);
    for slot in &app.world().get::<RiggedPresentation>(owner).unwrap().slots {
        assert_eq!(app.world().get::<RenderLayers>(*slot), Some(&private), "a composited part draws in the world");
    }
    // A reader of the root's image finds the body's frame where `FrameInSprite`
    // says: carried through the root's quad, that box is the baked frame's
    // (the deep dream laid its pattern over the whole square cell, and a wide
    // slug, a thin band of it, wore it magnified into stripes).
    let frame = *app.world().get::<FrameInSprite>(root).expect("a composited root states its frame");
    let (quad, anchor) = (
        app.world().get::<Sprite>(root).unwrap().custom_size.unwrap(),
        app.world().get::<Anchor>(root).unwrap().0,
    );
    let at = |image: Vec2| Vec2::new((image.x - 0.5 - anchor.x) * quad.x, (0.5 - anchor.y - image.y) * quad.y);
    let baked = |corner: Vec2| (corner - Vec2::splat(0.5) - feet) * RENDER;
    assert!(close(at(frame.min), baked(Vec2::new(0.0, 1.0))), "{:?} vs {:?}", at(frame.min), baked(Vec2::new(0.0, 1.0)));
    assert!(close(at(frame.max), baked(Vec2::new(1.0, 0.0))), "{:?} vs {:?}", at(frame.max), baked(Vec2::new(1.0, 0.0)));

    app.insert_resource(Composite(false));
    for frame in 1..COMPOSED_HOLD_FRAMES {
        app.update();
        assert!(draws_impostor(&app, root, owner), "let go {frame} frames after its last read");
    }
    app.update();
    let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
    assert!(presentation.impostor.is_none(), "still composited long after its last read");
    assert_eq!(app.world().get::<Sprite>(root).unwrap().image, ambition_sprite_sheet::character::NO_BAKED_IMAGE);
    assert!(app.world().get::<FrameInSprite>(root).is_none(), "a root with no image states a frame in it");
    for slot in &presentation.slots {
        assert_eq!(app.world().get::<RenderLayers>(*slot), Some(&RenderLayers::default()));
    }
    assert!(
        app.world().resource::<RiggedImpostorAtlas>().0.iter().all(Vec::is_empty),
        "its cell was not given back, or its empty page kept"
    );
}

/// A frame that fades as one picture is composited with nothing reading the
/// body (the fade needs the composite), and the body draws directly again
/// once its frames stop fading.
#[test]
fn a_fading_frame_is_composited_with_nothing_reading_it() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "player_robot_v3").expect("the robot publishes a flipbook");
    let sheet = sheet_with(&sprite_content, "player_robot_v3", Some(flipbook.clone()));
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
    app.update();
    let owner = owner(&app, root);
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_none(), "premise: idle is drawn directly");
    let last = flipbook.clip("death").expect("a death clip").frame_count() - 1;
    assert!(flipbook.frame_opacity("death", last) < 1.0, "the death clip does not fade, so this tests nothing");
    pin_clip(&mut app, root, "death", last);
    settle(&mut app);
    assert!(draws_impostor(&app, root, owner), "a fading frame is not composited");
    pin_clip(&mut app, root, "idle", 0);
    for _ in 0..=COMPOSED_HOLD_FRAMES {
        app.update();
    }
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_none());
}

/// The root's colour (a tint, a flash written on the body) multiplies every
/// part drawn directly, as it multiplied the composited body.
#[test]
fn a_direct_body_tints_its_parts_with_its_root_colour() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let sheet = raider(&sprite_content, true);
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
    #[cfg(feature = "card_view")]
    {
        use crate::rendering::card_view::{realize_card_bodies, CardMeshes};
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardMeshes>()
            .add_systems(Update, realize_card_bodies.after(RiggedDriveSet));
    }
    settle(&mut app);
    let owner = owner(&app, root);
    let slot = app.world().get::<RiggedPresentation>(owner).unwrap().slots[0];
    let plain = app.world().get::<Sprite>(slot).unwrap().color.to_linear();
    app.world_mut().get_mut::<Sprite>(root).unwrap().color = Color::linear_rgba(2.0, 0.5, 0.25, 0.5);
    app.update();
    let tinted = app.world().get::<Sprite>(slot).unwrap().color.to_linear();
    assert!((tinted.red - plain.red * 2.0).abs() < 1.0e-6);
    assert!((tinted.green - plain.green * 0.5).abs() < 1.0e-6);
    assert!((tinted.blue - plain.blue * 0.25).abs() < 1.0e-6);
    assert!((tinted.alpha - plain.alpha * 0.5).abs() < 1.0e-6);

    #[cfg(feature = "card_view")]
    let material_before = assert_card_color(&mut app, owner, tinted);

    app.world_mut().get_mut::<Sprite>(root).unwrap().color = Color::linear_rgba(2.0, 0.5, 0.25, 0.25);
    app.update();
    let faded = app.world().get::<Sprite>(slot).unwrap().color.to_linear();
    assert!((faded.alpha - plain.alpha * 0.25).abs() < 1.0e-6);
    assert_eq!(faded.to_vec3(), tinted.to_vec3());

    #[cfg(feature = "card_view")]
    {
        assert_eq!(assert_card_color(&mut app, owner, faded), material_before);
    }

    #[cfg(feature = "card_view")]
    fn assert_card_color(app: &mut App, owner: Entity, expected: LinearRgba) -> AssetId<StandardMaterial> {
        use crate::rendering::card_view::CardBody;
        let world = app.world_mut();
        let mut bodies = world.query::<&CardBody>();
        let body = bodies.iter(world).find(|body| body.owner == owner).unwrap();
        let material = &world.get::<MeshMaterial3d<StandardMaterial>>(body.parts[0]).unwrap().0;
        let tint = world.resource::<Assets<StandardMaterial>>().get(material).unwrap().base_color;
        assert_eq!(tint.to_linear(), expected);
        material.id()
    }
}

/// ⭐ THE RENDERER TAKES ITS POSE FROM WHOEVER GIVES ONE (the ragdoll seam). A
/// root carrying a `PartPose` — Mary-O's idle with her near arm swung, a pose
/// no clip authored — has its parts drawn where that pose puts them, by the
/// same slots on the same road; without one, where the flipbook does.
#[test]
fn a_part_pose_on_the_root_places_its_parts() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::rigged::{PartPose, PosedParts};
    use bevy::math::Affine2;

    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2").expect("a published flipbook");
    let sheet = sheet_with(&sprite_content, "mary_o_v2", Some(flipbook.clone()));
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
    pin_clip(&mut app, root, "idle", 0);
    app.update();
    let owner = owner(&app, root);
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().posed.is_some(), "Mary-O's tracks were not bound to her rig");
    let as_drawn = slots(&app, owner);

    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(
        ambition_sprite_sheet::baked_body_rigs::baked_body_rig(&sprite_content, "mary_o_v2").unwrap(),
    )
    .unwrap()
    .prepare()
    .unwrap();
    let mut joints = Vec::new();
    assert!(rig.solve("idle", 0, &mut joints));
    let arm = rig.joint_names().iter().position(|name| name == "near_arm").unwrap();
    let pivot = joints[arm].translation;
    joints[arm] = Affine2::from_translation(pivot) * Affine2::from_angle(1.0) * Affine2::from_translation(-pivot) * joints[arm];
    app.world_mut().entity_mut(root).insert(PartPose { joints: joints.clone() });
    app.update();

    let posed = PosedParts::bind(&flipbook, &rig, 1.0, 1.0_f32.to_radians());
    let mut expected = Vec::new();
    posed.place(flipbook.frame("idle", 0).unwrap(), &joints, &mut expected);
    let now = slots(&app, owner);
    let mut moved = 0;
    for (index, draw) in expected.iter().enumerate() {
        let want = Vec2::new(draw.at.x, -draw.at.y);
        assert!(close(now[index].0, want), "slot {index} at {:?}, the pose puts it at {want:?}", now[index].0);
        if !close(now[index].0, as_drawn[index].0) {
            moved += 1;
        }
    }
    assert!(moved > 0, "no part moved: the pose did not reach the slots");

    // Without the pose, the flipbook draws the body again.
    app.world_mut().entity_mut(root).remove::<PartPose>();
    app.update();
    assert_eq!(slots(&app, owner), as_drawn, "the flipbook's frame did not come back");
}

/// ⛔ A composited cell holds all the art its body draws. The oni leader's
/// banner and ears reach past its frame by more than the old fixed 16 px in its
/// jumps: composited (every hit flash) they were cut off, a blob of 522 px
/// against the same frame drawn directly. Every published flipbook's cell now
/// covers the farthest any of its draws reaches, with the fringe to spare.
#[test]
fn a_cell_holds_every_draw_of_its_body() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let mut widest = ("", 0.0_f32);
    for key in ambition_sprite_sheet::baked_part_flipbooks::baked_part_flipbook_targets(&sprite_content) {
        let flipbook = RiggedSpriteAsset::baked(&sprite_content, key).unwrap();
        assert!(impostor_margin(&flipbook) >= flipbook.art_overhang + 2.0, "`{key}`'s cell clips its art");
        if flipbook.art_overhang > widest.1 {
            widest = (key, flipbook.art_overhang);
        }
    }
    // ⛔ Premise: some published body reaches past the old fixed margin, so the
    // loop above judged the case this test is for. The oni leader was that
    // body until its redraw (2026-10-13) kept its art within 7 px.
    assert!(
        widest.1 > IMPOSTOR_MARGIN,
        "no published flipbook reaches past {IMPOSTOR_MARGIN} px (the widest is `{}` at {}), so the loop tested no art that the old margin cut",
        widest.0,
        widest.1
    );
}

/// ⛔ A POSE THAT REACHES PAST THE CELL IS NOT COMPOSITED. A cell holds the
/// frame and the room its flipbook's own draws need (`impostor_margin`). A
/// `PartPose` puts a part where no file says (a ragdoll, a reach): Mary-O's
/// near arm carried 400 px from her body, outside every published frame. One
/// camera draws a whole page with no scissor, so that arm, composited, was
/// drawn into another body's cell and cut out of her own.
///
/// While her posed parts reach past her cell she is drawn directly, as a body
/// whose frame fits no cell is: whole, and her readers see no image. The
/// control is the same body with the pose her clip already has: composited.
#[test]
fn a_part_pose_that_reaches_past_its_cell_is_drawn_directly() {


    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use ambition_sprite_sheet::character::rigged::PartPose;
    use ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    use bevy::math::Affine2;

    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "mary_o_v2").expect("a published flipbook");
    let (mut app, root) = app_with(&sprite_content, true, sheet_with(&sprite_content, "mary_o_v2", Some(flipbook.clone())));
    pin_clip(&mut app, root, "idle", 0);
    settle(&mut app);
    let owner = owner(&app, root);
    assert!(draws_impostor(&app, root, owner), "premise: read as one image, she is composited");

    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(
        ambition_sprite_sheet::baked_body_rigs::baked_body_rig(&sprite_content, "mary_o_v2").unwrap(),
    )
    .unwrap()
    .prepare()
    .unwrap();
    let mut joints = Vec::new();
    assert!(rig.solve("idle", 0, &mut joints));

    // Control: the pose of her own clip, given as a `PartPose`.
    app.world_mut().entity_mut(root).insert(PartPose { joints: joints.clone() });
    settle(&mut app);
    assert!(draws_impostor(&app, root, owner), "control: a pose inside her cell took her off the composited road");

    // Her near arm, carried far outside every published frame.
    let arm = rig.joint_names().iter().position(|name| name == "near_arm").unwrap();
    joints[arm] = Affine2::from_translation(Vec2::new(400.0, 0.0)) * joints[arm];
    let reach = {
        let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
        let mut placed = Vec::new();
        presentation.posed.as_ref().expect("her tracks ride her rig").place(flipbook.frame("idle", 0).unwrap(), &joints, &mut placed);
        flipbook.reach_past_frame(&placed)
    };
    assert!(
        reach > impostor_margin(&flipbook),
        "premise: the carried arm reaches {reach} px past her frame, inside the {} px her cell gives",
        impostor_margin(&flipbook)
    );
    app.world_mut().entity_mut(root).insert(PartPose { joints });
    app.update();
    {
        let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
        assert!(presentation.impostor.is_none(), "a pose that reaches {reach} px past her frame is still composited in a cell");
        assert_eq!(app.world().get::<Sprite>(root).unwrap().image, NO_BAKED_IMAGE, "her root still draws a cell");
        for slot in &presentation.slots {
            assert_eq!(
                app.world().get::<RenderLayers>(*slot),
                Some(&RenderLayers::default()),
                "a part of a body drawn directly draws on the impostor layer, where no view sees it"
            );
        }
    }

    // The pose ends: she is composited again, after the hold.
    app.world_mut().entity_mut(root).remove::<PartPose>();
    for _ in 0..=COMPOSED_HOLD_FRAMES + 2 {
        app.update();
    }
    assert!(draws_impostor(&app, root, owner), "she did not go back to her cell after the pose ended");
}

/// ⭐ Review A6: the driver answers each reader of a body's parts with the
/// draws of its own tracks, from one pass. Two readers on different tracks
/// each get theirs; a third that asks for both gets both; when one forgets,
/// the others keep their answers.
#[test]
fn the_driver_answers_each_reader_with_its_own_tracks() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, root) = app(&sprite_content, true);
    settle(&mut app);
    let tracks: Vec<String> = {
        let owner = owner(&app, root);
        app.world().get::<RiggedPresentation>(owner).unwrap().pages.flipbook.tracks().to_vec()
    };
    assert!(tracks.len() >= 2, "the premise: the raider has two tracks");
    let (a, b) = (tracks[0].clone(), tracks[1].clone());
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            ask_for_parts(&mut commands, root, "a", &[a.as_str()]);
            ask_for_parts(&mut commands, root, "b", &[b.as_str()]);
            ask_for_parts(&mut commands, root, "both", &[a.as_str(), b.as_str()]);
        })
        .unwrap();
    settle(&mut app);
    let counts = |app: &App| {
        let draws = app.world().get::<PartSetDraws>(root).expect("the answers");
        (draws.parts("a").len(), draws.parts("b").len(), draws.parts("both").len())
    };
    let (in_a, in_b, in_both) = counts(&app);
    assert!(in_a > 0 && in_b > 0, "each reader has the draws of its track: {:?}", counts(&app));
    assert_eq!(in_both, in_a + in_b, "a reader of both tracks has the draws of both");
    {
        let draws = app.world().get::<PartSetDraws>(root).unwrap();
        assert_ne!(draws.parts("a"), draws.parts("b"), "two tracks are two sets of draws");
    }

    app.world_mut().run_system_once(move |mut commands: Commands| forget_parts(&mut commands, root, "a")).unwrap();
    settle(&mut app);
    assert_eq!(counts(&app), (0, in_b, in_both), "the others keep their answers when one forgets");
}

/// A body drawn directly that no view shows is hidden and not posed, and its
/// root draws no baked frame; the frame a view shows it, it is posed and
/// shown before it is drawn. A view on other layers does not show it, a view
/// whose target is not measured yet shows everything, and with no view at
/// all (a host with no camera) every body is posed.
#[test]
fn a_body_no_view_shows_is_not_posed_until_a_view_shows_it() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (flipbook, mut app, root) = robot(&sprite_content);
    app.insert_resource(Composite(false));
    let owner = owner(&app, root);
    let clip = flipbook.clip("run").expect("a run clip");
    assert!(clip.frame_count() > 1, "the premise: the run clip has two frames");
    let at = |frame: usize| -> Vec<Vec2> {
        flipbook.frame("run", frame).unwrap().iter().map(|draw| Vec2::new(draw.at.x, -draw.at.y)).collect()
    };
    assert_ne!(at(0), at(1), "the premise: two frames of the run differ");
    let drawn = |app: &App| -> Vec<Vec2> { slots(app, owner).iter().filter(|(_, shown)| *shown).map(|(at, _)| *at).collect() };
    let shown = |app: &App| *app.world().get::<Visibility>(owner).unwrap() != Visibility::Hidden;

    // Out of its composited hold, onto the direct road.
    for _ in 0..=COMPOSED_HOLD_FRAMES {
        app.update();
    }
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_none(), "the premise: drawn directly");

    // No view: posed.
    pin_clip(&mut app, root, "run", 0);
    app.update();
    assert!(shown(&app), "with no view the body is shown");
    let first = drawn(&app);

    // A view far away, on the body's layer: hidden, and not posed.
    let measured = || {
        let mut camera = Camera::default();
        camera.computed.target_info =
            Some(bevy::camera::RenderTargetInfo { physical_size: UVec2::new(640, 360), scale_factor: 1.0 });
        camera
    };
    let view = |x: f32| {
        (
            Camera2d,
            measured(),
            Projection::Orthographic(OrthographicProjection {
                area: Rect::new(-320.0, -180.0, 320.0, 180.0),
                ..OrthographicProjection::default_2d()
            }),
            GlobalTransform::from_translation(Vec3::new(x, 0.0, 0.0)),
        )
    };
    let camera = app.world_mut().spawn(view(10_000.0)).id();
    pin_clip(&mut app, root, "run", 1);
    // A writer gives the root a baked frame, as the boss animator does.
    let baked = ready_page(&mut app);
    app.world_mut().get_mut::<Sprite>(root).unwrap().image = baked;
    app.update();
    assert!(!shown(&app), "a body no view shows is hidden");
    assert_eq!(drawn(&app), first, "a body no view shows is not posed");
    assert_eq!(
        app.world().get::<Sprite>(root).unwrap().image,
        ambition_sprite_sheet::character::NO_BAKED_IMAGE,
        "a hidden body's root draws a baked frame"
    );

    // A view whose target is not measured yet shows everything.
    let unmeasured = app.world_mut().spawn(view(10_000.0)).id();
    app.world_mut().get_mut::<Camera>(unmeasured).unwrap().computed.target_info = None;
    app.update();
    assert!(shown(&app), "a view with no measured target hid the body");
    let second = drawn(&app);
    assert_ne!(second, first, "the premise: the unmeasured view posed the second frame");
    app.world_mut().despawn(unmeasured);
    app.update();
    assert!(!shown(&app), "the premise: hidden again with only the far view");

    // A view on another layer over the body does not show it.
    app.world_mut().spawn(view(0.0)).insert(RenderLayers::layer(7));
    app.update();
    assert!(!shown(&app), "a view on other layers does not show the body");

    // The view comes over the body: posed (to the first frame again) and
    // shown in that frame.
    pin_clip(&mut app, root, "run", 0);
    *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() = GlobalTransform::IDENTITY;
    app.update();
    assert!(shown(&app), "the frame a view shows the body, it is shown");
    assert_eq!(drawn(&app), first, "the frame a view shows the body, it is posed");
}

/// A frame draw writes only what differs. Drawn again with nothing advanced,
/// it reports no write, so an animator system that bypasses change detection
/// leaves the sprite and its anchor unmarked; a new facing is a sprite write.
#[test]
fn a_frame_drawn_again_unchanged_writes_nothing() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    use crate::rendering::actors::animation::{draw_animator_frame, FrameWrites, StanceSquash};
    let sheet = raider(&sprite_content, false);
    let mut animator = CharacterAnimator::new(&sheet);
    let mut sprite = Sprite::from_atlas_image(Handle::default(), TextureAtlas { layout: Handle::default(), index: 0 });
    sprite.custom_size = Some(RENDER);
    let mut anchor = Anchor::BOTTOM_CENTER;
    let _ = draw_animator_frame(&mut sprite, &mut animator, Some(&mut anchor), 0.0, false, StanceSquash::NONE);
    let again = draw_animator_frame(&mut sprite, &mut animator, Some(&mut anchor), 0.0, false, StanceSquash::NONE);
    assert_eq!(again, FrameWrites::default(), "an unchanged frame reported a write");
    let flipped = draw_animator_frame(&mut sprite, &mut animator, Some(&mut anchor), 0.0, true, StanceSquash::NONE);
    assert!(flipped.sprite, "a new facing did not report a sprite write");
}

/// A limb sent through a portal pair is drawn by three pictures: the body
/// under the limb, the limb's twin, and the body over the limb. Together they
/// draw each part of the frame once, and in the frame's order: the arm is
/// not lost behind the giant, and an epaulette on its shoulder stays over it.
#[test]
fn a_limb_drawn_apart_and_the_body_under_and_over_it_draw_each_part_once_in_order() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&sprite_content, "pirate_raider").expect("a published flipbook").with_tracks(
        ["body", "limb:hand_left:upper", "limb:hand_left:fist", "epaulette", "head"].map(str::to_owned).to_vec(),
    );
    let draw = |track: Option<u16>| PartDraw { at: Vec2::ZERO, scale: Vec2::ONE, rotation: 0.0, part: 0, track, color: [255; 4] };
    let draws: Vec<PartDraw> = [Some(0), Some(1), Some(2), Some(3), Some(4), None].map(draw).to_vec();
    let tracks = |mask: RiggedTrackMask| mask.keep(&flipbook, &draws).iter().map(|d| d.track).collect::<Vec<_>>();
    let limbs = vec!["hand_left".to_owned()];
    let under = tracks(RiggedTrackMask::under(limbs.clone()));
    let limb = tracks(RiggedTrackMask::only("hand_left"));
    let over = tracks(RiggedTrackMask::over(limbs.clone()));
    assert_eq!(under, vec![Some(0)]);
    assert_eq!(limb, vec![Some(1), Some(2)]);
    assert_eq!(over, vec![Some(3), Some(4), None]);
    let all: Vec<_> = under.into_iter().chain(limb).chain(over).collect();
    assert_eq!(all, draws.iter().map(|d| d.track).collect::<Vec<_>>(), "each part once, in order");
    // A frame with no part of the limb: the body draws all of it.
    let no_limb = &draws[..1];
    assert_eq!(RiggedTrackMask::under(limbs.clone()).keep(&flipbook, no_limb).len(), 1);
    assert!(RiggedTrackMask::over(limbs).keep(&flipbook, no_limb).is_empty());
}

/// A limb's twin is composited at its root's size every frame. No animator
/// writes a twin's size, so a twin that kept its own read its last cell as a
/// squash and grew each frame (GNU-ton's arm, 2026-10-25: millions of pixels
/// tall after a few frames, so the arm was drawn nowhere).
#[test]
fn a_limb_twin_keeps_its_roots_size_frame_after_frame() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprite_content, true);
    app.add_systems(Update, follow_limb_twins.after(animate).before(bind_rigged_presentations));
    let twin = LimbTwin::spawn(&mut app.world_mut().commands(), root, "hand_left");
    app.world_mut().flush();
    app.world_mut().entity_mut(twin).insert(Visibility::Inherited);
    for _ in 0..12 {
        app.update();
    }
    let size = |entity: Entity| app.world().get::<Sprite>(entity).unwrap().custom_size;
    assert!(size(root).is_some(), "the root draws its cell");
    assert_eq!(size(twin), size(root), "the twin is drawn at its root's size");
}

#[test]
fn a_held_blink_dissolves_once_and_stays_dissolved_until_arrival() {
    use ambition_sprite_sheet::character::CharacterAnim;
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (_, mut app, root) = robot(&sprite_content);
    {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(CharacterAnim::BlinkOut);
        animator.tick(2.0);
    }
    app.update();
    let dissolved = warp_of(&app, root);
    assert_eq!(dissolved.x, 1.0);
    assert!(dissolved.y >= 1.0, "the body did not finish dissolving: {dissolved:?}");
    for _ in 0..80 {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(CharacterAnim::BlinkOut);
        animator.tick(0.017);
        drop(animator);
        app.update();
        assert!(warp_of(&app, root).y >= 1.0, "the body dissolved again while held");
    }
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::BlinkIn);
    app.update();
    assert_eq!(warp_of(&app, root).x, 2.0);
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Idle);
    app.update();
    assert_eq!(warp_of(&app, root), Vec4::ZERO);
}

#[test]
fn a_tracking_glow_keeps_its_geometry_when_the_body_is_composited() {
    use crate::rendering::actor_glow::{ActorGlowVisual, build_actor_glow_picture, sync_actor_glows};
    use ambition_platformer2d_shared_tangle::gameplay_presentation::ActorGlow;
    let content = ambition_sprite_sheet::test_support::shared_sprites();
    for anchor in [Anchor::CENTER, Anchor(Vec2::new(0.0, -0.5))] {
        let (mut app, root) = app_anchored(&content, true, raider(&content, true), anchor);
        app.world_mut().resource_mut::<Composite>().0 = false;
        app.world_mut().entity_mut(root).insert(ActorGlow::player_highlight());
        app.add_systems(Startup, build_actor_glow_picture);
        app.add_systems(Update, sync_actor_glows.after(RiggedDriveSet));
        settle(&mut app);
        let read = |app: &mut App| {
            let (place, sprite) = app.world_mut().query_filtered::<(&Transform, &Sprite), With<ActorGlowVisual>>()
                .single(app.world()).unwrap();
            (place.translation, sprite.custom_size, sprite.color)
        };
        let direct = read(&mut app);
        let direct_quad = app.world().get::<Sprite>(root).unwrap().custom_size;
        assert!(app.world().get::<FrameInSprite>(root).is_none());
        app.world_mut().resource_mut::<Composite>().0 = true;
        settle(&mut app);
        assert!(app.world().get::<FrameInSprite>(root).is_some(), "the body switched to its shader-readable image");
        assert_ne!(app.world().get::<Sprite>(root).unwrap().custom_size, direct_quad,
            "the shader image has a different canvas");
        assert_eq!(read(&mut app), direct, "the canvas cannot resize or move the tracking glow");
        app.world_mut().get_mut::<Sprite>(root).unwrap().color = Color::linear_rgba(4.0, 2.0, 1.0, 1.0);
        app.update();
        assert_eq!(read(&mut app), direct, "an actor tint cannot brighten the tracking glow");
    }
}

#[test]
fn a_held_blink_refreshes_its_finish_without_redrawing_its_parts() {
    use ambition_sprite_sheet::character::CharacterAnim;
    let content = ambition_sprite_sheet::test_support::shared_sprites();
    let flipbook = RiggedSpriteAsset::baked(&content, "player_robot_v3").unwrap();
    let (mut app, root) = app_with(&content, true, sheet_with(&content, "player_robot_v3", Some(flipbook)));
    app.init_resource::<Time>();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<ImpostorUnpremultiply>>();
    {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(CharacterAnim::BlinkOut);
        animator.tick(10.0);
    }
    settle(&mut app);
    app.update();
    let generation = atlas(&app).generation;
    assert_eq!(warp_of(&app, root).y, 1.0);
    let cameras = atlas(&app).cameras.clone();
    assert_eq!(cameras.len(), 2);
    assert!(cameras.iter().all(|camera| !app.world().get::<Camera>(*camera).unwrap().is_active));

    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
    app.update();
    assert!(!app.world().get::<Camera>(cameras[0]).unwrap().is_active, "the source parts stay cached");
    assert!(app.world().get::<Camera>(cameras[1]).unwrap().is_active, "the held strands refresh");
    assert_eq!(atlas(&app).generation, generation);
    assert_eq!(warp_of(&app, root).y, 1.0, "the dissolve does not restart");
    app.update();
    assert!(!app.world().get::<Camera>(cameras[1]).unwrap().is_active, "no refresh without a new tick");

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Idle);
    app.update();
    app.update();
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
    app.update();
    assert_eq!(warp_of(&app, root), Vec4::ZERO);
    assert!(cameras.iter().all(|camera| !app.world().get::<Camera>(*camera).unwrap().is_active));
}

#[cfg(feature = "portal_render")]
#[test]
fn a_paired_opening_composites_only_the_pictures_it_can_carry() {
    use ambition_portal2d_presentation::{PlacedPortal, PortalChannel, PortalChannelColor, PortalTransitView};
    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprites, true);
    app.world_mut().resource_mut::<Composite>().0 = false;
    settle(&mut app);
    assert!(app.world().get::<RiggedPresentation>(owner(&app,root)).unwrap().impostor.is_none());
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "portal picture", Vec2::splat(1000.0), Vec2::ZERO, Vec::new())));
    let channel = PortalChannel::Authored(PortalChannelColor::Purple);
    app.world_mut().spawn(PlacedPortal::fixed(channel, Vec2::new(550.0, 500.0), -Vec2::X, 40.0).unwrap());
    app.add_systems(Update, crate::rendering::portal_body_demand::declare_portal_body_demand.before(RiggedDriveSet));
    let composited = |app: &App| app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().impostor.is_some();
    settle(&mut app);
    assert!(!composited(&app), "an unpaired opening cannot carry the picture");

    app.world_mut().get_mut::<Transform>(root).unwrap().translation.x = -350.0;
    app.world_mut().spawn(PlacedPortal::fixed(channel.partner(), Vec2::new(900.0, 500.0), Vec2::X, 40.0).unwrap());
    settle(&mut app);
    assert!(!composited(&app), "a distant actor draws its parts directly");
    assert!(app.world().resource::<RiggedImpostorAtlas>().0.iter().all(Vec::is_empty));

    app.world_mut().get_mut::<Transform>(root).unwrap().translation.x = 0.0;
    for _ in 0..4 { app.update(); }
    assert!(composited(&app), "an approaching picture has its image ready for the carry");
    assert_ne!(app.world().get::<Sprite>(root).unwrap().image,
        ambition_sprite_sheet::character::NO_BAKED_IMAGE);

    app.world_mut().get_mut::<Transform>(root).unwrap().translation.x = -350.0;
    app.world_mut().entity_mut(root).insert(PortalTransitView { straddling: channel });
    for _ in 0..=COMPOSED_HOLD_FRAMES { app.update(); }
    assert!(composited(&app), "a transit can use a picture far from the opening");

    app.world_mut().entity_mut(root).remove::<PortalTransitView>();
    for _ in 0..=COMPOSED_HOLD_FRAMES { app.update(); }
    assert!(!composited(&app), "a distant actor returns its cell after transit");
}

#[test]
fn an_unchanged_impostor_waits_for_gpu_readiness_before_it_rests() {
    use ambition_asset_manager::image_stages::AppGpuPreparedImages;
    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app(&sprites, true);
    let images_ready = AppGpuPreparedImages::default();
    app.insert_resource(images_ready.clone()).init_resource::<readiness::GpuImpostors>()
        .init_resource::<Assets<Mesh>>().init_resource::<Assets<ImpostorUnpremultiply>>();
    let ids: Vec<_> = app.world().resource::<Assets<Image>>().ids().collect();
    for id in ids { images_ready.mark_prepared(id.untyped()); }
    for _ in 0..4 { app.update(); }
    let impostor = app.world().get::<RiggedPresentation>(owner(&app,root)).unwrap().impostor.unwrap();
    let atlas = app.world().resource::<RiggedImpostorAtlas>().page(&impostor).unwrap();
    let cameras = atlas.cameras.clone();
    let pass_ready = atlas.pass_ready.clone();
    let active = |app: &App| cameras.iter().all(|camera|app.world().get::<Camera>(*camera).unwrap().is_active);
    assert!(active(&app), "an unchanged pose cannot certify a finished image");
    app.update();
    assert!(active(&app));
    pass_ready.acknowledge();
    app.update();
    assert!(!active(&app), "the finished image can stay cached");
    let source = app.world().get::<RiggedPresentation>(owner(&app,root)).unwrap().pages.pages[0].id().untyped();
    images_ready.mark_awaiting(source);
    app.update();
    assert!(active(&app), "new source pixels need another render");
    images_ready.mark_prepared(source);
    app.update();
    assert!(!active(&app));
}

/// RV1: the 2D driver draws the body's local sample and interprets nothing
/// itself. Every sampled draw shows in a slot, in draw order. A system between
/// the sampler and the driver that takes the last draw out of the sample takes
/// it out of the picture: the driver had no other source to draw it from.
#[test]
fn the_slots_draw_the_local_sample_and_nothing_else() {
    fn drop_the_last_draw(mut samples: Query<&mut RiggedLocalSample>) {
        for mut sample in &mut samples {
            sample.draws.pop();
        }
    }
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let shown = |trimmed: bool| {
        let sheet = raider(&sprite_content, true);
        let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
        let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
        if trimmed {
            app.add_systems(Update, drop_the_last_draw.after(sample_rigged_presentations).before(drive_rigged_presentations));
        }
        settle(&mut app);
        let owner = owner(&app, root);
        let sample = app.world().get::<RiggedLocalSample>(owner).unwrap();
        assert_eq!(sample.state, LocalSampleState::Parts);
        let sampled = sample.draws.len();
        let drawn = slots(&app, owner).iter().filter(|(_, visible)| *visible).count();
        (sampled, drawn)
    };
    let (sampled, drawn) = shown(false);
    assert!(sampled > 1, "the raider samples its parts");
    assert_eq!(drawn, sampled, "each sampled draw shows in a slot");
    let (trimmed, drawn) = shown(true);
    assert_eq!(trimmed, sampled - 1);
    assert_eq!(drawn, trimmed, "a draw taken out of the sample is not drawn");
}

/// RV2: the card view and the 2D driver draw one sample. Each sampled draw is
/// one visible card, at its slot's place in the body and in paint order; the
/// card body stands where the 2D owner stands. A draw taken out of the sample
/// before both renderers is gone from both, so neither reads the other.
#[cfg(feature = "card_view")]
#[test]
fn the_cards_and_the_slots_draw_one_sample() {
    use crate::rendering::card_view::{card_part_local, realize_card_bodies, CardBody, CardMeshes};
    fn drop_the_last_draw(mut samples: Query<&mut RiggedLocalSample>) {
        for mut sample in &mut samples {
            sample.draws.pop();
        }
    }
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let shown = |trimmed: bool| {
        let sheet = raider(&sprite_content, true);
        let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
        let (mut app, root) = app_direct(&sprite_content, sheet, Anchor(feet));
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CardMeshes>()
            .add_systems(Update, realize_card_bodies.after(RiggedDriveSet));
        if trimmed {
            app.add_systems(Update, drop_the_last_draw.after(sample_rigged_presentations).before(drive_rigged_presentations));
        }
        settle(&mut app);
        // The card body is spawned on the first frame and filled from the next.
        app.update();
        let owner = owner(&app, root);
        let world = app.world_mut();
        let sample = world.get::<RiggedLocalSample>(owner).unwrap();
        assert_eq!(sample.state, LocalSampleState::Parts);
        let draws = sample.draws.clone();
        assert!(sample.feet.is_some(), "a sample with parts has its feet");
        let slots_shown = slots(&app, owner).iter().filter(|(_, visible)| *visible).count();
        let world = app.world_mut();
        let mut bodies = world.query::<(&CardBody, &Transform)>();
        let (body, body_place) = bodies
            .iter(world)
            .find(|(body, _)| body.owner == owner)
            .expect("the sampled body has cards");
        let cards: Vec<(Transform, bool)> = body
            .parts
            .iter()
            .map(|card| {
                (
                    *world.get::<Transform>(*card).unwrap(),
                    *world.get::<Visibility>(*card).unwrap() != Visibility::Hidden,
                )
            })
            .collect();
        let owner_place = *world.get::<Transform>(owner).unwrap();
        let body_place = *body_place;
        let shown_cards: Vec<Transform> = cards.iter().filter(|(_, visible)| *visible).map(|(t, _)| *t).collect();
        assert_eq!(shown_cards.len(), draws.len(), "one visible card per sampled draw");
        for (index, (card, draw)) in shown_cards.iter().zip(&draws).enumerate() {
            assert_eq!(*card, card_part_local(draw, index), "card {index} is its draw's place in the body");
        }
        assert_eq!(body_place.translation.truncate(), owner_place.translation.truncate(), "the cards stand where the 2D owner stands");
        assert_eq!(body_place.rotation, owner_place.rotation);
        assert_eq!(body_place.scale, owner_place.scale);
        (draws.len(), slots_shown, shown_cards.len())
    };
    let (sampled, slots_shown, cards_shown) = shown(false);
    assert!(sampled > 1, "the raider samples its parts");
    assert_eq!((slots_shown, cards_shown), (sampled, sampled));
    let (trimmed, slots_shown, cards_shown) = shown(true);
    assert_eq!(trimmed, sampled - 1);
    assert_eq!((slots_shown, cards_shown), (trimmed, trimmed), "a draw taken out of the sample is drawn by neither renderer");
}

/// RV3: a hybrid body on its baked clip is one card in the card view, at the
/// root, with the frame the animator chose (its page region and quad); back
/// on a part clip it is one card per sampled draw again. Before, a baked
/// sample drew nothing in the card view.
#[cfg(feature = "card_view")]
#[test]
fn a_baked_frame_is_one_card_at_the_root() {
    use crate::rendering::card_view::{realize_card_bodies, CardBody, CardMeshes};
    use ambition_sprite_sheet::character::CharacterAnim;
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let hybrid = hybrid_raider(&sprite_content, "slash");
    let (mut app, root) = app_with(&sprite_content, true, raider_with(&sprite_content, Some(hybrid)));
    // Drawn part by part: a composited body is one card of its cell
    // (`a_rigged_body_flashes_its_parts_only_without_a_whole_frame`).
    app.insert_resource(Composite(false));
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<CardMeshes>()
        .add_systems(Update, realize_card_bodies.after(RiggedDriveSet));
    settle(&mut app);
    app.update();
    let owner = owner(&app, root);
    let shown = |app: &mut App| {
        let world = app.world_mut();
        let mut bodies = world.query::<(&CardBody, &Transform, &Visibility)>();
        let (body, place, visibility) = bodies.iter(world).find(|(body, ..)| body.owner == owner).expect("the body has cards");
        assert_ne!(*visibility, Visibility::Hidden);
        let place = *place;
        let visible = body.parts.iter().filter(|card| *world.get::<Visibility>(**card).unwrap() != Visibility::Hidden).count();
        (visible, place)
    };
    let sampled = app.world().get::<RiggedLocalSample>(owner).unwrap().draws.len();
    assert!(sampled > 1);
    assert_eq!(shown(&mut app).0, sampled, "a part clip: one card per draw");

    // The fixture's sheet pages are neither laid out nor loaded; give them
    // a layout and a picture, as a loaded sheet has.
    let pages = app.world().get::<CharacterAnimator>(root).unwrap().pages.clone();
    let grid = TextureAtlasLayout::from_grid(UVec2::splat(32), 16, 16, None, None);
    for page in &pages {
        let _ = app.world_mut().resource_mut::<Assets<TextureAtlasLayout>>().insert(page.layout.id(), grid.clone());
        if !app.world().resource::<Assets<Image>>().contains(&page.texture) {
            let _ = app.world_mut().resource_mut::<Assets<Image>>().insert(page.texture.id(), Image::default());
        }
    }
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Slash);
    app.update();
    let sample = app.world().get::<RiggedLocalSample>(owner).unwrap();
    assert_eq!(sample.state, LocalSampleState::Baked);
    let baked = app.world().get::<RiggedLocalSample>(owner).unwrap().baked.clone().expect("the baked sample carries its frame");
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    let sprite = app.world().get::<Sprite>(root).unwrap();
    assert_eq!(Some(baked.size), sprite.custom_size, "the quad the animator gave the root");
    assert_eq!(baked.flip_x, sprite.flip_x);
    assert_eq!(baked.image, animator.pages[animator.current_page() as usize].texture);
    assert_eq!(baked.rect, grid.textures[animator.atlas_index()].as_rect(), "the region of the frame the animator chose");
    let (visible, place) = shown(&mut app);
    assert_eq!(visible, 1, "a baked frame is one card");
    let root_place = *app.world().get::<Transform>(root).unwrap();
    assert_eq!(place.translation.truncate(), root_place.translation.truncate(), "at the root");

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Idle);
    app.update();
    let sampled = app.world().get::<RiggedLocalSample>(owner).unwrap().draws.len();
    assert_eq!(shown(&mut app).0, sampled, "back to parts");
}

/// RV3: a body the 2D view composites (here every body is read as one image,
/// as a flash or a fade asks) is one card of its cell in the card view,
/// drawn whole: the cell carries the frame's fade, so the parts do not fade
/// one by one. The shared flash overlay draws on the whole frame.
/// A rig drawn directly has a silhouette over each part during a flash.
#[cfg(feature = "card_view")]
#[test]
fn a_rigged_body_flashes_its_parts_only_without_a_whole_frame() {
    use crate::rendering::card_view::{realize_card_bodies, CardBody, CardMeshes};
    use crate::rendering::hit_flash::PresentedFlash;
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app_with(&sprite_content, true, raider(&sprite_content, true));
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<CardMeshes>()
        .add_systems(Update, realize_card_bodies.after(RiggedDriveSet));
    settle(&mut app);
    app.update();
    let owner = owner(&app, root);
    assert!(draws_impostor(&app, root, owner), "the 2D root draws its cell");
    let cards = |app: &mut App| -> Vec<StandardMaterial> {
        let world = app.world_mut();
        let mut bodies = world.query::<&CardBody>();
        let body = bodies.iter(world).find(|body| body.owner == owner).expect("the body has cards");
        let materials = world.resource::<Assets<StandardMaterial>>();
        body.parts
            .iter()
            .filter(|card| *world.get::<Visibility>(**card).unwrap() != Visibility::Hidden)
            .map(|card| materials.get(&world.get::<MeshMaterial3d<StandardMaterial>>(*card).unwrap().0).unwrap().clone())
            .collect()
    };
    let shown = cards(&mut app);
    assert_eq!(shown.len(), 1, "a composited body is one card");
    let page = atlas(&app).image.clone();
    assert_eq!(shown[0].base_color_texture.as_ref(), Some(&page), "the card draws the cell's page");
    assert_eq!(shown[0].base_color.alpha(), 1.0, "the cell carries the fade");

    let cue = PresentedFlash { intensity: 0.75, tint: Vec3::new(1.0, 0.5, 0.0) };
    let color = Color::srgba_u8(120, 190, 240, 128);
    app.world_mut().entity_mut(root).insert(cue).get_mut::<Sprite>().unwrap().color = color;
    app.update();
    let shown = cards(&mut app);
    assert_eq!(shown.len(), 1, "the shared overlay draws the flash");
    assert_eq!(shown[0].base_color_texture.as_ref(), Some(&page));
    assert_eq!(shown[0].base_color.to_srgba().to_u8_array(), color.to_srgba().to_u8_array());

    app.insert_resource(Composite(false));
    for _ in 0..=COMPOSED_HOLD_FRAMES {
        app.update();
    }
    assert!(app.world().get::<RiggedPresentation>(owner).unwrap().impostor.is_none());
    let sampled = app.world().get::<RiggedLocalSample>(owner).unwrap().draws.len();
    let shown = cards(&mut app);
    assert_eq!(shown.len(), 2 * sampled, "part by part, a silhouette over each card");
    assert_eq!(shown[1].base_color.alpha(), 0.75, "as strong as the cue");
    assert_eq!(shown[1].emissive, Color::srgb(1.0, 0.5, 0.0).to_linear(), "in the cue's colour");
    app.world_mut().entity_mut(root).insert(PresentedFlash::default());
    app.update();
    assert_eq!(cards(&mut app).len(), sampled, "no cue, no silhouette");
}

#[cfg(feature = "card_view")]
#[test]
fn a_morphed_body_hides_its_rigged_cards_and_can_show_them_again() {
    use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
    use ambition_sim_view::BodyPoseView;
    use crate::rendering::body_visibility::sync_body_visibility;
    use crate::rendering::card_view::{realize_card_bodies, CardBody, CardMeshes};

    let sprites = ambition_sprite_sheet::test_support::shared_sprites();
    let (mut app, root) = app_direct(&sprites, raider(&sprites, true), Anchor::CENTER);
    app.world_mut().entity_mut(root).insert((PlayerVisual, BodyPoseView::default()));
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<CardMeshes>()
        .add_systems(Update, (
            sync_body_visibility.before(RiggedDriveSet),
            realize_card_bodies.after(RiggedDriveSet),
        ));
    settle(&mut app);
    let card = app.world_mut().query_filtered::<Entity, With<CardBody>>()
        .single(app.world()).expect("the visible body has a card root");
    assert_eq!(app.world().get::<Visibility>(card), Some(&Visibility::Inherited));

    app.world_mut().get_mut::<BodyPoseView>(root).unwrap().morph_ball = true;
    app.update();
    let sample = app.world().get::<RiggedLocalSample>(owner(&app, root)).unwrap();
    assert!(sample.parts().is_some());
    assert_eq!(app.world().get::<Visibility>(card), Some(&Visibility::Hidden));

    app.world_mut().get_mut::<BodyPoseView>(root).unwrap().morph_ball = false;
    app.update();
    assert_eq!(app.world().get::<Visibility>(card), Some(&Visibility::Inherited));
}

#[cfg(feature = "card_view")]
#[test]
fn a_cart_drawn_from_parts_keeps_its_rider_between_its_panels() {
    use ambition_platformer2d_core::Vec2 as WorldPoint;
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, PlayerVisual, RoomInstanceRoot};
    use ambition_platformer2d_world::{collision::MovingPlatformSet, platforms::MovingPlatformState};
    use ambition_sim_view::{BodyPoseView, FeatureViewIndex};
    use crate::rendering::card_view::{realize_card_bodies, realize_card_sprite_bodies, CardBody, CardMeshes, CardRootDepths, CardSpriteBody};
    use crate::rendering::moving_platforms::{seat_bodies_in_vessels, MovingPlatformVisual, PlatformLook, PlatformLooks, SeatedInVessel};

    let content = ambition_sprite_sheet::test_support::shared_sprites();
    let sheet = raider(&content, true);
    let (mut app, rider) = app_direct(&content, sheet, Anchor::CENTER);
    let sheet = app.world().resource::<GameAssets>().characters.sheet("raider").unwrap().clone();
    let room = LiveRoomInstance::ACTIVATION;
    let cart = MovingPlatformState::from_authored(
        WorldPoint::new(100.0, 108.0), WorldPoint::new(96.0, 16.0), 0.0, 0.0,
    ).with_visual("cart");
    app.world_mut().spawn((RoomInstanceRoot, room, MovingPlatformSet(vec![cart])));
    app.world_mut().entity_mut(rider).insert((
        PlayerVisual,
        BodyPoseView { pos: WorldPoint::new(100.0, 84.0), size: WorldPoint::new(16.0, 32.0), ..default() },
    ));
    app.insert_resource(PlatformLooks(HashMap::from([("cart".into(), PlatformLook {
        sheet: None,
        row: "idle".into(),
        frame_px: [100.0, 60.0],
        span_px: [2.0, 98.0],
        top_px: 40.0,
        faces_right: true,
        front_row: Some("rail".into()),
        rim_px: Some(12.0),
    })])))
    .init_resource::<FeatureViewIndex>()
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .init_resource::<CardMeshes>()
    .init_resource::<CardRootDepths>()
    .add_systems(Update, seat_bodies_in_vessels.before(RiggedDriveSet))
    .add_systems(Update, (realize_card_bodies, realize_card_sprite_bodies).chain().after(RiggedDriveSet));
    let mut layer = |index, kind: Option<&str>, front, z| {
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&sheet, RENDER, Anchor::CENTER);
        app.world_mut().spawn((
            sprite, anchor, animator, BoundSpriteQuality::of(&sheet),
            Transform::from_xyz(0.0, 0.0, z),
            MovingPlatformVisual { room, index, sheet: kind.map(str::to_owned), front },
        )).id()
    };
    let far = layer(0, Some("cart"), false, 11.0);
    let near = layer(0, Some("cart"), true, 14.5);
    let deck = layer(1, Some("deck"), false, 11.0);
    app.world_mut().spawn((
        PlayerVisual, Sprite::from_color(Color::WHITE, Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, 20.0),
    ));
    settle(&mut app);
    assert!(app.world().get::<SeatedInVessel>(rider).is_some(), "the rider is inside the cart");

    let placement = |app: &mut App, source| {
        let owner = owner(app, source);
        let world = app.world_mut();
        let mut cards = world.query::<(Entity, &CardBody, &Transform)>();
        let (entity, body, place) = cards.iter(world).find(|(_, body, _)| body.owner == owner).unwrap();
        assert!(body.parts.len() > 1);
        let last_depth = body.parts.iter()
            .map(|part| world.get::<Transform>(*part).unwrap().translation.z + place.translation.z)
            .max_by(f32::total_cmp).unwrap();
        (entity, place.translation, last_depth)
    };
    let far = placement(&mut app, far);
    let rider = placement(&mut app, rider);
    let before = placement(&mut app, near);
    let deck = placement(&mut app, deck);
    let free_depth = app.world_mut().query_filtered::<&Transform, With<CardSpriteBody>>()
        .single(app.world()).unwrap().translation.z;
    assert!(far.2 < rider.1.z && rider.2 < before.1.z && before.2 < free_depth);
    assert!(deck.2 < free_depth, "all parts of the deck are behind the free actor");

    app.world_mut().get_mut::<Transform>(near).unwrap().translation.x += 30.0;
    app.update();
    let moved = placement(&mut app, near);
    assert_eq!(moved, (before.0, before.1 + Vec3::X * 30.0, before.2));
}

#[cfg(feature = "card_view")]
#[test]
fn parts_of_a_room_prop_stay_in_the_props_depth() {
    use ambition_platformer2d_world::rooms::PropDraw;
    use crate::rendering::card_view::{realize_card_bodies, realize_card_scenery, realize_card_sprite_bodies, CardBody, CardMeshes, CardRootDepths, CardScenery, CardSpriteBody};
    use crate::rendering::primitives::{PlayerVisual, PropVisual, WorldSprite};
    use crate::rendering::world::SCENERY_Z_STEP;
    use ambition_platformer2d_core::config::{WORLD_Z_DUMMY, WORLD_Z_PLAYER};

    let content = ambition_sprite_sheet::test_support::shared_sprites();
    let sheet = raider(&content, true);
    let (mut app, prop) = app_direct(&content, sheet, Anchor::CENTER);
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<CardMeshes>()
        .init_resource::<CardRootDepths>()
        .add_systems(Update, (realize_card_bodies, realize_card_sprite_bodies, realize_card_scenery).chain().after(RiggedDriveSet));
    app.world_mut().spawn((
        PlayerVisual, Sprite::from_color(Color::WHITE, Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, WORLD_Z_PLAYER),
    ));
    app.world_mut().entity_mut(prop).insert(PropVisual {
        id: "prop".into(), kind: "prop".into(), name: "Prop".into(), size: RENDER, draw: PropDraw::Decoration, flip_y: false,
    });

    for (z, above_player) in [(WORLD_Z_DUMMY + 0.55, false), (WORLD_Z_PLAYER + 1.0, true)] {
        app.world_mut().get_mut::<Transform>(prop).unwrap().translation.z = z;
        let next = spawn_scenery(&mut app, z + SCENERY_Z_STEP);
        settle(&mut app);
        let owner = owner(&app, prop);
        let world = app.world_mut();
        let free_depth = world.query_filtered::<&Transform, With<CardSpriteBody>>().single(world).unwrap().translation.z;
        let next_depth = world.query_filtered::<&Transform, With<CardScenery>>().single(world).unwrap().translation.z;
        let mut cards = world.query::<(&CardBody, &Transform)>();
        let (body, place) = cards.iter(world).find(|(body, _)| body.owner == owner).unwrap();
        assert!(body.parts.len() > 1);
        let root_depth = place.translation.z;
        let parts: Vec<_> = body.parts.iter().map(|part| world.get::<Transform>(*part).unwrap().translation.z).collect();
        assert!(parts.windows(2).all(|pair| pair[0] < pair[1]), "the parts keep their paint order");
        assert!(root_depth + parts.last().unwrap() < next_depth, "one prop stays behind the next prop");
        assert_eq!(root_depth > free_depth, above_player);
        world.entity_mut(next).despawn();
    }

    fn spawn_scenery(app: &mut App, depth: f32) -> Entity {
        let place = Transform::from_xyz(0.0, 0.0, depth);
        app.world_mut().spawn((
            WorldSprite, Sprite::from_color(Color::WHITE, Vec2::ONE), place,
            GlobalTransform::from(place), InheritedVisibility::VISIBLE,
        )).id()
    }
}
