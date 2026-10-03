use std::sync::Arc;

use ambition_sprite_sheet::character::rigged::{ClipTween, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterSpriteAsset,
};

use super::*;

const RENDER: Vec2 = Vec2::new(103.0, 114.0);

fn raider(rigged: bool) -> CharacterSpriteAsset {
    raider_with(rigged.then(|| RiggedSpriteAsset::baked("pirate_raider").expect("a published flipbook")))
}

fn raider_with(flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    sheet_with("pirate_raider", flipbook)
}

fn sheet_with(character: &str, flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(character).expect("a baked sheet");
    CharacterSpriteAsset {
        texture: Handle::default(),
        layout: Handle::default(),
        spec,
        pages: Vec::new(),
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

fn app_with(admit: bool, sheet: CharacterSpriteAsset) -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Assets<Image>>();
    let sheet = with_pages(sheet, || ready_page(&mut app));
    app.init_resource::<RiggedPresentations>()
        .insert_resource(RiggedSpriteAdmission { admit })
        .add_systems(Update, (bind_rigged_presentations, drive_rigged_presentations).chain());
    let mut assets = GameAssets::default();
    assets.characters.declare("raider", "Raider");
    assets.characters.publish("raider", sheet.clone());
    app.insert_resource(assets);
    let asset = sheet;
    let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
    let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
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

/// `(translation, visible)` of every slot, in draw order.
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

fn frame_draws(app: &App, root: Entity) -> Vec<Vec2> {
    let animator = app.world().get::<CharacterAnimator>(root).unwrap();
    let row = animator.spec.row_name(animator.drawn_row().unwrap()).unwrap().to_owned();
    let flipbook = RiggedSpriteAsset::baked("pirate_raider").unwrap();
    let scale = RENDER / flipbook.frame_size.as_vec2();
    flipbook
        .frame(&row, animator.frame)
        .unwrap()
        .iter()
        .map(|draw| Vec2::new(draw.at.x, -draw.at.y) * scale)
        .collect()
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 1.0e-3
}

#[test]
fn a_rigged_root_draws_its_frame_from_parts_in_reusable_slots() {
    let (mut app, root) = app(true);
    app.update();
    let owner = owner(&app, root);
    let max_draws = RiggedSpriteAsset::baked("pirate_raider").unwrap().max_draws();
    let drawn = slots(&app, owner);
    assert_eq!(drawn.len(), max_draws);
    let expected = frame_draws(&app, root);
    assert_eq!(drawn.iter().filter(|(_, visible)| *visible).count(), expected.len());
    for ((at, _), want) in drawn.iter().zip(&expected) {
        assert!(close(*at, *want), "a part at {at:?}, its draw at {want:?}");
    }
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 0.0, "the root still draws itself");

    // Another frame reuses the slots: nothing is spawned.
    let entities = app.world().entities().count_spawned();
    let slot_ids = app.world().get::<RiggedPresentation>(owner).unwrap().slots.clone();
    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().frame = 3;
    app.update();
    assert_eq!(app.world().entities().count_spawned(), entities);
    assert_eq!(self::owner(&app, root), owner, "the frame change rebound the root");
    assert_eq!(app.world().get::<RiggedPresentation>(owner).unwrap().slots, slot_ids);
    let expected = frame_draws(&app, root);
    for ((at, _), want) in slots(&app, owner).iter().zip(&expected) {
        assert!(close(*at, *want), "frame 3: a part at {at:?}, its draw at {want:?}");
    }

    // A mirrored root mirrors every part about its feet.
    app.world_mut().get_mut::<Sprite>(root).unwrap().flip_x = true;
    app.update();
    for ((at, _), want) in slots(&app, owner).iter().zip(&expected) {
        assert!(close(*at, Vec2::new(-want.x, want.y)), "mirrored: {at:?} for {want:?}");
    }
}

#[test]
fn with_the_trial_off_no_root_gets_parts() {
    let (mut app, root) = app(false);
    app.update();
    assert!(app.world().resource::<RiggedPresentations>().0.is_empty());
    let mut parts = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(parts.iter(app.world()).count(), 0);
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 1.0);
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
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 1.0);
}

#[test]
fn the_parts_are_drawn_by_each_camera_that_draws_their_root() {
    let (mut app, root) = app(true);
    app.update();
    let owner = owner(&app, root);
    let layers = |app: &App| -> Vec<Option<RenderLayers>> {
        let presentation = app.world().get::<RiggedPresentation>(owner).unwrap();
        presentation
            .slots
            .iter()
            .filter(|slot| *app.world().get::<Visibility>(**slot).unwrap() != Visibility::Hidden)
            .map(|slot| app.world().get::<RenderLayers>(*slot).cloned())
            .collect()
    };
    assert!(layers(&app).iter().all(Option::is_none));

    // A root on a pane's own layer: its parts go to that pane too.
    let pane = RenderLayers::layer(0).with(5);
    app.world_mut().entity_mut(root).insert(pane.clone());
    app.update();
    let drawn = layers(&app);
    assert!(!drawn.is_empty());
    assert!(drawn.iter().all(|layers| layers.as_ref() == Some(&pane)), "{drawn:?}");

    app.world_mut().entity_mut(root).remove::<RenderLayers>();
    app.update();
    assert!(layers(&app).iter().all(Option::is_none));
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
    let state = |app: &App| {
        let parts = slots(app, owner).iter().filter(|(_, visible)| *visible).count();
        (app.world().get::<Sprite>(root).unwrap().color.alpha(), parts)
    };
    let (alpha, parts) = state(&app);
    assert_eq!(alpha, 0.0, "idle is a part clip: the root draws nothing");
    assert!(parts > 0);

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Slash);
    app.update();
    assert_eq!(state(&app), (1.0, 0), "slash is baked: the root draws, with no part");

    app.world_mut().get_mut::<CharacterAnimator>(root).unwrap().request(CharacterAnim::Idle);
    app.update();
    let (alpha, parts) = state(&app);
    assert_eq!((alpha, parts > 0), (0.0, true), "back to parts");
    assert_eq!(self::owner(&app, root), owner);
    assert_eq!(app.world().get::<RiggedPresentation>(owner).unwrap().slots, slot_ids);
    assert_eq!(*app.world().get::<Transform>(root).unwrap(), placed);
}

/// The root draws its baked frame until every part page is ready, and the
/// parts take over in one frame when they are.
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
        assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 1.0, "the baked root went transparent");
    }

    for page in &pending {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(page.id(), Image::default())
            .unwrap();
    }
    app.update();
    let owner = owner(&app, root);
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 0.0);
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
        assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 0.0);
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
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 0.0);
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
        assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 1.0, "the baked root is not drawn");
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
    assert_eq!(app.world().get::<Sprite>(root).unwrap().color.alpha(), 0.0);
    assert!(slots(&app, lookout_owner).iter().any(|(_, visible)| *visible), "no part drawn in the frame of the change");
}

/// A sheet with no row for the compact pose is squashed: the root's quad is
/// drawn shorter about a line that holds still. The parts take the same
/// squash through their owner, about the same line, for both pivots
/// `StanceSquash` uses: the anchor (a feet-anchored quad) and the quad's own
/// foot edge (an authored-offset quad).
#[test]
fn a_squashed_root_squashes_its_parts_about_the_same_line() {
    let (mut app, root) = app(true);
    app.update();
    let owner = owner(&app, root);
    let at = app.world().get::<Transform>(root).unwrap().translation;
    let (h0, a0) = {
        let animator = app.world().get::<CharacterAnimator>(root).unwrap();
        let (size, anchor) = animator.current_render().unwrap();
        (size.y, anchor.y)
    };

    // About the anchor: the height halves, the anchor stays.
    app.world_mut().get_mut::<Sprite>(root).unwrap().custom_size.as_mut().unwrap().y = h0 * 0.5;
    app.update();
    let squashed = *app.world().get::<Transform>(owner).unwrap();
    assert!((squashed.scale.y - 0.5).abs() < 1.0e-5, "{squashed:?}");
    assert!((squashed.translation - at).length() < 1.0e-3, "the anchor moved: {squashed:?}");

    // About the quad's foot edge: the anchor moves so the foot edge holds.
    let ratio = 0.6;
    let foot = -(a0 + 0.5) * h0;
    let top = foot + ((0.5 - a0) * h0 - foot) * ratio;
    let h1 = h0 * ratio;
    app.world_mut().get_mut::<Sprite>(root).unwrap().custom_size.as_mut().unwrap().y = h1;
    app.world_mut().get_mut::<Anchor>(root).unwrap().0.y = 0.5 - top / h1;
    app.update();
    let squashed = *app.world().get::<Transform>(owner).unwrap();
    assert!((squashed.scale.y - ratio).abs() < 1.0e-5, "{squashed:?}");
    // A part point on the foot edge (local y = `foot`) stays where it was.
    let held = squashed.translation.y + squashed.scale.y * foot;
    assert!((held - (at.y + foot)).abs() < 1.0e-3, "the foot edge moved from {} to {held}", at.y + foot);
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
    let scale = RENDER / flipbook.frame_size.as_vec2();
    let (this, next) = (flipbook.frame("walk", 0).unwrap(), flipbook.frame("walk", 1).unwrap());
    let mut moved = 0;
    for (index, draw) in this.iter().enumerate() {
        let Some(target) = next.iter().find(|next| next.track == draw.track && next.part == draw.part) else {
            continue;
        };
        let expected = at_frame[index].0 + Vec2::new(target.at.x - draw.at.x, -(target.at.y - draw.at.y)) * scale * 0.5;
        assert!(close(halfway[index].0, expected), "slot {index}: {:?}, expected {expected:?}", halfway[index].0);
        moved += usize::from(target.at != draw.at);
    }
    assert!(moved > 0, "the walk's first two frames move no part, so this tests nothing");
}
