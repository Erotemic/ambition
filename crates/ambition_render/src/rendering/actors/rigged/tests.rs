use std::sync::Arc;

use ambition_sprite_sheet::character::rigged::{RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterSpriteAsset,
};

use super::*;

const RENDER: Vec2 = Vec2::new(103.0, 114.0);

fn raider(rigged: bool) -> CharacterSpriteAsset {
    raider_with(rigged.then(|| RiggedSpriteAsset::baked("pirate_raider").expect("a published flipbook")))
}

fn raider_with(flipbook: Option<RiggedSpriteAsset>) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id("pirate_raider").expect("a baked pirate_raider sheet");
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

fn app_with(admit: bool, sheet: CharacterSpriteAsset) -> (App, Entity) {
    let mut app = App::new();
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
