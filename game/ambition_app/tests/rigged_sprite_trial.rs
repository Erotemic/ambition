//! Rig packet 6: with the rigged-sprite trial admitted, a pirate is drawn from
//! its transform flipbook; without it, nothing changes.
//!
//! ⛔ IN THE SHIPPED COMPOSITION, for the reason `admiral_gun_sword` gives: the
//! demo shell's catalog cannot seat `npc_pirate_admiral`.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use ambition_platformer2d::render::rendering::actors::rigged::{
    RiggedPartSlot, RiggedPresentation, RiggedPresentations,
};
use ambition_platformer2d::sprite_sheet::character::rigged::RiggedSpriteAdmission;
use bevy::prelude::*;

/// Seat two admirals in the shipped game and let the match settle.
fn seated_admirals(admit: bool) -> App {
    use ambition_platformer2d::actor::MatchSeat;
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |app| {
            app.insert_resource(RiggedSpriteAdmission { admit });
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
    let mut app = seated_admirals(true);
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
        assert_eq!(world.get::<Sprite>(root).unwrap().color.alpha(), 0.0);
    }
    // Every slot belongs to a presentation: none leaked from a rebind.
    let mut owners = app.world_mut().query::<&RiggedPresentation>();
    let owned: usize = owners.iter(app.world()).map(|p| p.slots.len()).sum();
    let mut slots = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(slots.iter(app.world()).count(), owned);
}

#[test]
fn the_shipped_game_draws_no_parts() {
    let mut app = seated_admirals(false);
    assert!(app.world().resource::<RiggedPresentations>().0.is_empty());
    let mut slots = app.world_mut().query::<&RiggedPartSlot>();
    assert_eq!(slots.iter(app.world()).count(), 0);
}
