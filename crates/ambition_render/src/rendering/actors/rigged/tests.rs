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
    assets.characters.declare("raider", "Raider");
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

fn atlas(app: &App) -> &ImpostorAtlas {
    app.world().resource::<RiggedImpostorAtlas>().0.as_ref().expect("the impostor atlas was built")
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
        .as_ref()
        .is_some_and(|atlas| sprite.image == atlas.image);
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
    let owner = owner(&app, root);
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
