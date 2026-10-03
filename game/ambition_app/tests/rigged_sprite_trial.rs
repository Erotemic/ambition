//! Rig packet 6: with the rigged sprites admitted, a pirate is drawn from its
//! transform flipbook; with the switch off, nothing changes. Since 2026-10-01
//! the shipped game admits them when it inserts nothing (Jon's go-ahead).
//!
//! ⛔ IN THE SHIPPED COMPOSITION, for the reason `admiral_gun_sword` gives: the
//! demo shell's catalog cannot seat `npc_pirate_admiral`.
//!
//! Not here: that a body stays baked while its part pages load. In this
//! composition the pages have loaded before a body binds, so a check on every
//! frame passed with the readiness guard removed (measured 2026-09-30). The
//! witnesses are in `ambition_render` (`rendering::actors::rigged::tests`),
//! which hold a page pending.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use ambition_platformer2d::render::rendering::actors::rigged::{
    RiggedPartSlot, RiggedPresentation, RiggedPresentations,
};
use ambition_platformer2d::sprite_sheet::character::rigged::RiggedSpriteAdmission;
use bevy::prelude::*;

/// Seat two admirals in the shipped game and let the match settle. `admit`
/// inserts that switch; `None` inserts nothing, as the shipped game does.
fn seated_admirals(admit: Option<bool>) -> App {
    use ambition_platformer2d::actor::MatchSeat;
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |app| {
            if let Some(admit) = admit {
                app.insert_resource(RiggedSpriteAdmission { admit });
            }
        },
    );
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().insert_resource(ambition_demo_smash::smash_roster([
        "npc_pirate_admiral",
        "npc_pirate_admiral",
    ]));
    app.world_mut().write_message(ShellCommand::GoTo(ShellRouteId::new(
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )));
    let mut settled = 0;
    for _ in 0..900 {
        app.update();
        let world = app.world_mut();
        let mut seats = world.query::<&MatchSeat>();
        if seats.iter(world).count() > 0 {
            settled += 1;
            if settled > 120 {
                break;
            }
        }
    }
    app
}

#[test]
fn an_admitted_admiral_is_drawn_from_its_parts() {
    let mut app = seated_admirals(Some(true));
    let presentations: Vec<Entity> = app
        .world()
        .resource::<RiggedPresentations>()
        .0
        .values()
        .copied()
        .collect();
    assert_eq!(presentations.len(), 2, "each seated admiral has one rigged presentation");
    for owner in presentations {
        let world = app.world();
        let presentation = world.get::<RiggedPresentation>(owner).unwrap();
        let root = presentation.root;
        let bound = world
            .get::<ambition_platformer2d::render::rendering::actors::BoundSpriteQuality>(root)
            .expect("a bound root");
        let pages = &presentation.pages;
        // The parts come from the tier the root is bound at.
        assert_eq!(
            pages.flipbook.texel_scale < 1.0,
            bound.scale != Default::default(), // Default is the full-resolution tier
            "a {:?} root drawing parts of texel scale {}",
            bound.scale,
            pages.flipbook.texel_scale
        );
        let visible = presentation
            .slots
            .iter()
            .filter(|slot| world.get::<Visibility>(**slot).is_some_and(|v| *v != Visibility::Hidden))
            .count();
        assert!(visible >= 8, "only {visible} parts drawn");
        // The pages loaded from where the tier published them: a wrong
        // directory draws every part as nothing.
        let server = world.resource::<AssetServer>();
        for page in &pages.pages {
            let state = server.load_state(page.id());
            assert!(
                state.is_loaded(),
                "part page {:?} is {state:?}",
                server.get_path(page.id())
            );
        }
        // The root draws its cell of the impostor atlas its parts are
        // composited into.
        let atlas = world
            .resource::<ambition_platformer2d::render::rendering::actors::rigged::RiggedImpostorAtlas>()
            .page(&presentation.impostor)
            .expect("the impostor atlas page");
        let sprite = world.get::<Sprite>(root).unwrap();
        assert!(
            sprite.image == atlas.image
                && sprite.texture_atlas.as_ref().map(|frame| frame.index) == Some(presentation.impostor.cell as usize),
            "the root does not draw its impostor cell"
        );
    }
    // Every slot belongs to a presentation: none leaked from a rebind.
    let mut owners = app.world_mut().query::<&RiggedPresentation>();
    let owned: usize = owners.iter(app.world()).map(|p| p.slots.len()).sum();
    let mut slots = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(slots.iter(app.world()).count(), owned);
}

#[test]
fn with_the_switch_off_no_part_is_drawn() {
    let mut app = seated_admirals(Some(false));
    assert!(app.world().resource::<RiggedPresentations>().0.is_empty());
    let mut slots = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(slots.iter(app.world()).count(), 0);
}

/// The shipped game, which inserts no switch, draws each admiral from its
/// parts (the go-ahead of 2026-10-01).
#[test]
fn the_shipped_game_draws_the_admirals_from_their_parts() {
    assert!(
        !std::env::var(ambition_platformer2d::sprite_sheet::character::rigged::RIGGED_SPRITE_ADMISSION_ENV)
            .is_ok_and(|value| !RiggedSpriteAdmission::from_setting(Some(&value)).admit),
        "precondition: the environment does not turn the rigged sprites off"
    );
    let mut app = seated_admirals(None);
    assert_eq!(
        app.world().resource::<RiggedPresentations>().0.len(),
        2,
        "each seated admiral has one rigged presentation"
    );
    let mut slots = app.world_mut().query::<&RiggedPartSlot>();
    assert!(slots.iter(app.world()).count() > 0, "the admirals' part slots exist");
}

/// Rig packet 7, with the impostor (decision D4): a second local view draws the
/// same body and makes no more parts.
///
/// The second pane is made as TwinTrack makes its laboratory pane: a
/// `LocalView` with its facts and a column placement, and a `MainCamera` on
/// layer 0 that presents it. Then:
///
/// * the presentations and the very same slot entities stay as they were with
///   one view: the parts belong to the body, not to a view;
/// * no main camera draws a part: the parts stand on the private impostor
///   layer, and every view draws the ROOT, which draws the impostor.
///
/// ⛔ Not each camera's `VisibleEntities`: without a window the host camera
/// lists no sprite at all, so those lists cannot tell a missing part from a
/// headless camera.
#[test]
fn a_second_view_draws_the_same_parts_and_makes_no_more() {
    use ambition_platformer2d::platformer::camera_layers::MainCamera;
    use ambition_platformer2d::sim_view::{local_view_facts, LocalView, LocalViewId, PresentsView, ViewPlacement};
    use bevy::camera::visibility::RenderLayers;

    let mut app = seated_admirals(Some(true));
    // The slot entities themselves, not a count: a count cannot see a slot
    // despawned and another spawned.
    let census = |app: &mut App| {
        let world = app.world_mut();
        let mut owners: Vec<Entity> = world.resource::<RiggedPresentations>().0.values().copied().collect();
        owners.sort();
        let mut slots = world.query_filtered::<Entity, With<RiggedPartSlot>>();
        let mut slots: Vec<Entity> = slots.iter(world).collect();
        slots.sort();
        (owners, slots)
    };
    let before = census(&mut app);
    assert_eq!(before.0.len(), 2, "two seated admirals, two presentations");

    let view = app
        .world_mut()
        .spawn((LocalView, LocalViewId(1), local_view_facts(), ViewPlacement::column(1, 2)))
        .id();
    app.world_mut().spawn((
        Camera2d,
        Camera { order: 1, ..default() },
        MainCamera,
        RenderLayers::layer(0),
        PresentsView(view),
    ));
    for _ in 0..60 {
        app.update();
    }

    // Not the total entity count: in 60 frames the host spawns about 40
    // entities of its own (two of them `PresentedForView`, for the new view),
    // and none of them is a part.
    assert_eq!(census(&mut app), before, "the owners or the part slots changed with a second view");

    let world = app.world_mut();
    let layers_of = |world: &World, entity: Entity| world.get::<RenderLayers>(entity).cloned().unwrap_or_default();
    let mut cameras = world.query_filtered::<Option<&RenderLayers>, (With<MainCamera>, With<Camera>)>();
    let cameras: Vec<RenderLayers> = cameras.iter(world).map(|layers| layers.cloned().unwrap_or_default()).collect();
    assert_eq!(cameras.len(), 2, "the host camera and the pane camera");
    let mut presentations = world.query::<&RiggedPresentation>();
    let mut checked = 0;
    for presentation in presentations.iter(world) {
        let root = layers_of(world, presentation.root);
        for camera in &cameras {
            assert!(camera.intersects(&root), "a view on {camera:?} does not draw the root on {root:?}");
        }
        for slot in &presentation.slots {
            if world.get::<Visibility>(*slot) == Some(&Visibility::Hidden) {
                continue;
            }
            let part = layers_of(world, *slot);
            for camera in &cameras {
                assert!(!camera.intersects(&part), "a view on {camera:?} draws a loose part on {part:?}");
            }
            checked += 1;
        }
    }
    assert!(checked >= 16, "only {checked} drawn parts checked");
}
