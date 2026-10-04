use std::sync::Arc;

use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;
use ambition_sprite_sheet::character::rigged::{ClipTween, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterSpriteAsset,
};

use super::*;
use crate::rendering::actors::draw_held_frame;

const RENDER: Vec2 = Vec2::new(103.0, 114.0);

fn raider(rigged: bool) -> CharacterSpriteAsset {
    raider_with(rigged.then(|| RiggedSpriteAsset::baked("pirate_raider").expect("a published flipbook")))
}

fn raider_with(flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    sheet_with("pirate_raider", flipbook)
}

fn sheet_with(character: &str, flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(character).expect("a baked sheet");
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
fn app(admit: bool) -> (App, Entity) {
    app_with(admit, raider(true))
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
fn animate(squash: Res<Squash>, mut roots: Query<(&mut Sprite, &mut CharacterAnimator, &mut Anchor), Without<RiggedPartSlot>>) {
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

fn app_with(admit: bool, sheet: CharacterSpriteAsset) -> (App, Entity) {
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    app_anchored(admit, sheet, Anchor(feet))
}

/// [`app_with`], with the root built at `anchor`: the feet for an NPC,
/// `Anchor::CENTER` for a player with a sheet-authored quad
/// (`character_render_basis`).
fn app_anchored(admit: bool, sheet: CharacterSpriteAsset, anchor: Anchor) -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<TextureAtlasLayout>>();
    let sheet = with_pages(sheet, || ready_page(&mut app));
    app.init_resource::<RiggedPresentations>()
        .init_resource::<RiggedImpostorAtlas>()
        .init_resource::<Squash>()
        .insert_resource(RiggedSpriteAdmission { admit })
        .add_systems(Update, (animate, bind_rigged_presentations, drive_rigged_presentations).chain());
    let mut assets = GameAssets::default();
    assets.characters.declare("raider");
    assets.characters.publish("raider", sheet.clone());
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
            BoundSpriteQuality {
                scale: TextureResolutionScale::Full,
            },
        ))
        .id();
    (app, root)
}

fn owner(app: &App, root: Entity) -> Entity {
    *app.world()
        .resource::<RiggedPresentations>()
        .0
        .get(&root)
        .expect("the root has a rigged presentation")
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
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    let row = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
    let flipbook = RiggedSpriteAsset::baked(target).unwrap();
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
            .is_some_and(|frame| frame.index == presentation.impostor.cell as usize && Some(&frame.layout) == atlas(app).layout.as_ref())
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
    let (mut app, root) = app(true);
    app.update();
    let owner = owner(&app, root);
    let max_draws = RiggedSpriteAsset::baked("pirate_raider").unwrap().max_draws();
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
    let flipbook = RiggedSpriteAsset::baked("pirate_raider").unwrap();
    let per_pixel = RENDER / flipbook.frame_size.as_vec2();
    let feet = presentation.impostor.feet;
    assert_eq!(feet, flipbook.feet_pixel + Vec2::splat(IMPOSTOR_MARGIN));
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
fn with_the_trial_off_no_root_gets_parts() {
    let (mut app, root) = app(false);
    app.update();
    assert!(app.world().resource::<RiggedPresentations>().0.is_empty());
    let mut parts = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(parts.iter(app.world()).count(), 0);
    assert!(draws_baked(&app, root));
}

#[test]
fn a_root_whose_sheet_loses_its_flipbook_draws_itself_again() {
    let (mut app, root) = app(true);
    app.update();
    let owner = owner(&app, root);
    app.world_mut()
        .resource_mut::<GameAssets>()
        .characters
        .publish("raider", raider(false));
    app.update();
    assert!(app.world().get_entity(owner).is_err(), "the owner outlived the flipbook");
    let mut parts = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(parts.iter(app.world()).count(), 0, "a slot outlived its owner");
    let cameras = atlas(&app).cameras.clone();
    assert!(
        cameras.iter().all(|camera| !app.world().get::<Camera>(*camera).unwrap().is_active),
        "the impostor cameras run with no body to draw"
    );
    assert!(draws_baked(&app, root), "the root did not take its baked frame back");
}

/// The parts draw only into the impostor: on the private layer, in the body's
/// own cell. Every view draws the ROOT (on whatever layers it has), so a
/// second pane adds no presentation entity.
#[test]
fn the_parts_draw_only_into_their_impostor() {
    let (mut app, root) = app(true);
    app.update();
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
    assert_eq!(origin, atlas(&app).cell_feet(presentation.impostor.cell, presentation.impostor.feet));

    let pane = RenderLayers::layer(0).with(5);
    app.world_mut().entity_mut(root).insert(pane.clone());
    let entities = app.world().entities().count_spawned();
    app.update();
    assert_eq!(app.world().entities().count_spawned(), entities);
    assert_eq!(app.world().get::<RenderLayers>(root), Some(&pane), "the root's pane was rewritten");
    assert!(draws_impostor(&app, root, owner));
}

/// Two bodies stand in two cells (the atlas grows from one cell to make room),
/// and a freed cell is reused.
#[test]
fn each_body_has_its_own_cell() {
    let (mut app, first) = app(true);
    let second = {
        let asset = raider(true);
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    app.update();
    let cell = |app: &App, root| app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().impostor.cell;
    let (a, b) = (cell(&app, first), cell(&app, second));
    assert_ne!(a, b);
    assert_eq!(atlas(&app).side, 2, "two bodies in a one-cell atlas");
    // Each body's parts stand in its own cell of the grown atlas.
    for root in [first, second] {
        let owner = owner(&app, root);
        let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
        let at = app.world().get::<Transform>(owner).unwrap().translation.truncate();
        assert_eq!(at, atlas(&app).cell_feet(presentation.impostor.cell, presentation.impostor.feet));
    }
    app.world_mut().entity_mut(first).despawn();
    app.update();
    let third = {
        let asset = raider(true);
        let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        app.world_mut()
            .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
            .id()
    };
    app.update();
    assert_eq!(cell(&app, third), a, "the freed cell was not reused");
}

/// More bodies drawn from parts alone than one page of their class holds:
/// each draws its own cell, the class opens a second page for the 37th, and no
/// body draws [`NO_BAKED_IMAGE`], which is what a body with no cell falls back
/// to.
#[test]
fn a_class_with_every_cell_taken_opens_a_page() {
    use ambition_sprite_sheet::character::NO_BAKED_IMAGE;
    let mut sheet = raider(true);
    sheet.texture = NO_BAKED_IMAGE;
    for page in &mut sheet.pages {
        page.texture = NO_BAKED_IMAGE;
    }
    assert!(sheet.parts_only(), "the sheet is not drawn from parts alone, so this tests nothing");
    let (mut app, first) = app_with(true, sheet.clone());
    let most = (IMPOSTOR_CELL_CLASSES[0].1 * IMPOSTOR_CELL_CLASSES[0].1) as usize;
    let mut roots = vec![first];
    let feet = Vec2::new(sheet.spec.feet_anchor_x, sheet.spec.feet_anchor_y);
    for _ in 0..most {
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&sheet, RENDER, Anchor(feet));
        roots.push(
            app.world_mut()
                .spawn((sprite, anchor, animator, Transform::default(), Visibility::Inherited))
                .id(),
        );
    }
    app.update();
    let pages = &app.world().resource::<RiggedImpostorAtlas>().0[0];
    assert_eq!(pages.len(), 2, "{} bodies in one page of {most} cells", roots.len());
    assert_ne!(pages[0].image, pages[1].image, "two pages share one target");
    let mut per_page = [0; 2];
    for root in &roots {
        let sprite = app.world().get::<Sprite>(*root).unwrap();
        assert!(sprite.image != NO_BAKED_IMAGE, "a body drawn from parts alone draws no image");
        let owner = owner(&app, *root);
        let impostor = app.world().get::<RiggedPresentation>(owner).unwrap().impostor;
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
    assert_eq!(per_page, [most, 1]);
    // The second page stands to the right of the first and its quad, and its
    // cameras run for the body it holds.
    let pages = &app.world().resource::<RiggedImpostorAtlas>().0[0];
    assert!(pages[1].cell_feet(0, Vec2::ZERO).x >= IMPOSTOR_PAGE_STEP);
    for camera in &pages[1].cameras {
        assert!(app.world().get::<Camera>(*camera).unwrap().is_active, "the second page rests");
    }

    // Each page renders for its own cells only: a frame with no change rests
    // both, and a new frame of the body on the second page runs the second
    // page's cameras and not the first's.
    let active = |app: &App, page: usize| {
        let cameras = &app.world().resource::<RiggedImpostorAtlas>().0[0][page].cameras;
        cameras.iter().map(|camera| app.world().get::<Camera>(*camera).unwrap().is_active).collect::<Vec<_>>()
    };
    app.update();
    assert!(!active(&app, 0).contains(&true) && !active(&app, 1).contains(&true), "a page renders with no change");
    let on_second = *roots
        .iter()
        .find(|root| app.world().get::<RiggedPresentation>(owner(&app, **root)).unwrap().impostor.page == 1)
        .unwrap();
    app.world_mut().get_mut::<CharacterAnimator>(on_second).unwrap().frame = 3;
    app.update();
    assert!(!active(&app, 1).contains(&false), "the second page rests while its body changes");
    assert!(!active(&app, 0).contains(&true), "the first page renders for a body of the second");
}

/// The raider's published flipbook with `row` left to the baked sheet: its
/// clip is taken out of `clips` and named in `baked_clips`, as a hybrid
/// publish states it.
fn hybrid_raider(row: &str) -> RiggedSpriteAsset {
    let text = ambition_sprite_sheet::baked_part_flipbooks::baked_part_flipbook("pirate_raider").unwrap();
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
    let spec = try_load_spec_for_character_id("pirate_raider").unwrap();
    hybrid.check_rows(spec.row_names()).expect("the hybrid states every row");
    hybrid
}

/// Rig packet 9: one body moves between a part clip and a baked clip. The
/// same root, animator and slots draw both, so nothing is spawned and the
/// root does not move.
#[test]
fn a_hybrid_body_crosses_between_part_and_baked_clips_in_place() {
    use ambition_sprite_sheet::character::rigged::ClipRealization;
    use ambition_sprite_sheet::character::CharacterAnim;

    let hybrid = hybrid_raider("slash");
    assert_eq!(hybrid.realization("slash"), Some(ClipRealization::Baked));
    assert_eq!(hybrid.realization("idle"), Some(ClipRealization::Parts));
    let (mut app, root) = app_with(true, raider_with(Some(hybrid)));
    app.update();
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
    use ambition_sprite_sheet::character::CharacterAnim;

    let (mut app, a) = app_with(true, raider_with(Some(hybrid_raider("slash"))));
    let b = {
        let asset = raider_with(Some(hybrid_raider("slash")));
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
    let presentation = |app: &App, root| app.world().get::<RiggedPresentation>(owner(app, root)).unwrap().impostor;
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
/// impostor takes over in one frame when they are.
#[test]
fn a_body_stays_baked_until_every_part_page_is_ready() {
    let (mut app, root) = app_with(true, raider(false));
    let pending: Vec<Handle<Image>> = (0..RiggedSpriteAsset::baked("pirate_raider").unwrap().pages.len())
        .map(|_| pending_page(&mut app))
        .collect();
    let mut pages = pending.clone().into_iter();
    let sheet = with_pages(raider(true), || pages.next().unwrap());
    app.world_mut().resource_mut::<GameAssets>().characters.publish("raider", sheet);
    for _ in 0..5 {
        app.update();
        assert!(app.world().resource::<RiggedPresentations>().0.is_empty(), "bound to pages still loading");
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
    assert!(draws_impostor(&app, root, owner));
    assert!(slots(&app, owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
}

/// A quality-tier change keeps drawing the parts of the old tier until every
/// page of the new tier is ready, and then changes in one frame.
#[test]
fn a_tier_change_keeps_the_old_parts_until_the_new_pages_are_ready() {
    let (mut app, root) = app(true);
    app.update();
    let full_owner = owner(&app, root);

    let quarter = RiggedSpriteAsset::baked("pirate_raider")
        .unwrap()
        .for_tier(TextureResolutionScale::Quarter)
        .expect("the raider publishes a quarter tier")
        .expect("the quarter tier is the raider's");
    let mut sheet = raider_with(Some(quarter));
    sheet.requested_tier = TextureResolutionScale::Quarter;
    sheet.resolved_tier = TextureResolutionScale::Quarter;
    let pending: Vec<Handle<Image>> = sheet.rigged.as_ref().unwrap().pages.iter().map(|_| pending_page(&mut app)).collect();
    let mut pages = pending.clone().into_iter();
    let sheet = with_pages(sheet, || pages.next().unwrap());
    app.world_mut().resource_mut::<GameAssets>().characters.publish("raider", sheet);
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
    let (mut app, root) = app(true);
    app.update();
    let raider_owner = owner(&app, root);

    let flipbook = RiggedSpriteAsset::baked("pirate_lookout").expect("the lookout publishes a flipbook");
    let mut pending = Vec::new();
    let lookout = with_pages(sheet_with("pirate_lookout", Some(flipbook)), || {
        let page = pending_page(&mut app);
        pending.push(page.clone());
        page
    });
    app.world_mut().resource_mut::<GameAssets>().characters.publish("lookout", lookout.clone());
    let feet = Vec2::new(lookout.spec.feet_anchor_x, lookout.spec.feet_anchor_y);
    let (sprite, anchor, animator) = build_character_presentation_with_render_size(&lookout, RENDER, Anchor(feet));
    app.world_mut().entity_mut(root).insert((sprite, anchor, animator));
    for _ in 0..3 {
        app.update();
        assert!(
            app.world().get_entity(raider_owner).is_err(),
            "the raider's parts stayed on the lookout's root while its pages loaded"
        );
        assert!(app.world().resource::<RiggedPresentations>().0.is_empty(), "bound to pages still loading");
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
    assert_eq!(app.world().get::<RiggedPresentation>(lookout_owner).unwrap().target, "pirate_lookout");
    assert!(draws_impostor(&app, root, lookout_owner));
    assert!(slots(&app, lookout_owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
}

/// A sheet with no row for the compact pose is squashed: the animator draws
/// the root's quad shorter about a line that holds still. The impostor quad
/// takes the same squash about the same line, for both pivots `StanceSquash`
/// uses: the anchor (a feet-anchored quad) and the quad's own foot edge (an
/// authored-offset quad).
#[test]
fn a_squashed_root_squashes_its_impostor_about_the_same_line() {
    let (mut app, root) = app(true);
    app.update();
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
    let flipbook = RiggedSpriteAsset::baked("mary_o_v2_tall").expect("a published flipbook");
    assert_eq!(flipbook.clip("walk").unwrap().tween, ClipTween::Linear);
    let (mut app, root) = app_with(true, sheet_with("mary_o_v2_tall", Some(flipbook.clone())));
    app.update();
    let owner = owner(&app, root);
    let duration = {
        let mut animator = app.world_mut().get_mut::<CharacterAnimator>(root).unwrap();
        animator.request(ambition_sprite_sheet::character::CharacterAnim::Walk);
        animator.frame = 0;
        animator.elapsed = 0.0;
        let row = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
        assert_eq!(row, "walk");
        ambition_sprite_sheet::character::sheets::record_for_sheet_key("mary_o_v2_tall")
            .unwrap()
            .rows
            .iter()
            .find(|sheet_row| sheet_row.animation == row)
            .unwrap()
            .duration_secs
    };
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
    let flipbook = RiggedSpriteAsset::baked("mary_o_v2_tall").expect("a published flipbook");
    let frame = flipbook.frame_size.as_vec2();
    let spec = try_load_spec_for_character_id("mary_o_v2_tall").unwrap();
    // Where a quad of `size` at `anchor` draws the point at `uv` of itself
    // (0..1, +y down), mirrored about the origin when `flip`.
    let local = |size: Vec2, anchor: Vec2, uv: Vec2, flip: bool| {
        let mut at = (Vec2::new(uv.x - 0.5, 0.5 - uv.y) - anchor) * size;
        if flip {
            at.x = -at.x;
        }
        at
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
        for flip in [false, true] {
            let (mut app, root) = app_anchored(true, sheet_with("mary_o_v2_tall", Some(flipbook.clone())), built_at);
            app.world_mut().get_mut::<Sprite>(root).unwrap().flip_x = flip;
            app.update();
            let owner = owner(&app, root);
            assert!(draws_impostor(&app, root, owner));
            let size = app.world().get::<Sprite>(root).unwrap().custom_size.unwrap();
            let mut anchor = app.world().get::<Anchor>(root).unwrap().0;
            if flip {
                // The drawn anchor is mirrored with the quad; unmirror it to
                // read the quad in its own frame, as `local` mirrors after.
                anchor.x = -anchor.x;
            }
            let basis = app.world().get::<CharacterAnimator>(root).unwrap().render_basis.unwrap();
            let base_anchor = basis.feet_anchor;
            for pixel in [flipbook.feet_pixel, Vec2::ZERO, frame, Vec2::new(frame.x, 0.0)] {
                let baked = local(basis.render_size, base_anchor, pixel / frame, flip);
                let cell = local(size, anchor, (pixel + Vec2::splat(IMPOSTOR_MARGIN)) / IMPOSTOR_CELL, flip);
                assert!(
                    close(cell, baked),
                    "{built_at:?} flip {flip}: frame pixel {pixel} draws at {cell} from the impostor, {baked} baked"
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
    let flipbook = RiggedSpriteAsset::baked("player_robot_v3").expect("the robot publishes a flipbook");
    let frame = flipbook.frame_size.as_vec2();
    let (mut app, root) = app_with(true, sheet_with("player_robot_v3", Some(flipbook.clone())));
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
        let cell = local(size, anchor, (pixel + Vec2::splat(IMPOSTOR_MARGIN)) / IMPOSTOR_CELL);
        assert!(
            close(cell, baked),
            "facing left: frame pixel {pixel} draws at {cell} from the impostor, {baked} baked"
        );
    }
}

/// The player robot's flipbook, published (`player_robot_v3.py`): every row
/// from parts, placed between pixels.
fn robot() -> (RiggedSpriteAsset, App, Entity) {
    let flipbook = RiggedSpriteAsset::baked("player_robot_v3").expect("the robot publishes a flipbook");
    let (mut app, root) = app_with(true, sheet_with("player_robot_v3", Some(flipbook.clone())));
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
    let (flipbook, mut app, root) = robot();
    let owner = owner(&app, root);
    let last = flipbook.clip("death").expect("a death clip").frame_count() - 1;
    let fade = flipbook.frame_opacity("death", last);
    assert!(fade < 0.6, "the death clip does not fade ({fade}), so this tests nothing");
    pin_clip(&mut app, root, "death", last);
    app.update();
    let cell = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.cell as usize;
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

/// A body's colour shift (`CharacterColorShift`: an enemy variant, a buff)
/// reaches its cell of the page's material, and only its cell: one sheet
/// draws every coloured variant. A body without one is drawn as painted.
#[test]
fn a_color_shift_reaches_its_bodys_cell() {
    use ambition_sprite_sheet::character::CharacterColorShift;
    let (_flipbook, mut app, root) = robot();
    let owner = owner(&app, root);
    let cell = app.world().get::<RiggedPresentation>(owner).unwrap().impostor.cell as usize;
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
    let (flipbook, mut app, root) = robot();
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
    let (flipbook, mut app, root) = robot();
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
    let mut checked = Vec::new();
    for (key, _text) in ambition_sprite_sheet::baked_part_flipbooks::BAKED_PART_FLIPBOOKS {
        // `<target>.<tier>` is a tier's table of the same frame.
        if key.contains('.') {
            continue;
        }
        let flipbook = RiggedSpriteAsset::baked(key).expect("a published flipbook parses");
        assert!(
            impostor_cell_class(flipbook.frame_size.as_vec2()).is_some(),
            "`{key}`'s {} px frame fits no impostor cell ({IMPOSTOR_CELL_CLASSES:?})",
            flipbook.frame_size
        );
        checked.push(*key);
    }
    // Mary-O's three forms, the five pirates and the robot, at least.
    assert!(checked.contains(&"player_robot_v3") && checked.len() >= 9, "{checked:?}");
}

/// A body takes the smallest cell its frame fits: Noether's 496 x 528 frame a
/// 576 px cell of the second atlas, which the root then draws; the first
/// atlas is never built for her.
#[test]
fn a_large_frame_takes_a_cell_of_its_size() {
    let flipbook = RiggedSpriteAsset::baked("noether").expect("noether publishes a flipbook");
    assert_eq!(impostor_cell_class(flipbook.frame_size.as_vec2()), Some(1));
    assert_eq!(impostor_cell_class(Vec2::new(256.0, 256.0)), Some(0));
    assert_eq!(impostor_cell_class(Vec2::new(2000.0, 10.0)), None);
    let (mut app, root) = app_with(true, sheet_with("noether", Some(flipbook.clone())));
    app.update();
    app.update();
    let owner = owner(&app, root);
    let impostor = app.world().get::<RiggedPresentation>(owner).unwrap().impostor;
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
