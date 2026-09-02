#![allow(dead_code)]

//! Shared fixtures for `ambition_app` integration tests.
//!
//! Keep this intentionally small: integration tests should still read like
//! end-to-end scripts, but the neutral `AgentAction` and fixed-60Hz sim setup are
//! common enough that copying them into every test obscures the scenario logic.

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

/// A fully-neutral action; build scenario inputs with struct update:
/// `AgentAction { move_x: 1.0, ..base() }`.
pub fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        left_pressed: false,
        right_pressed: false,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        attack_held: false,
        attack_released: false,
        attack_strength: Default::default(),
        // A scripted action steers its attack with the movement axis; only a
        // C-stick replay says otherwise.
        attack_from_aim_stick: false,
        attack_aim: (0.0, 0.0),
        special: false,
        special_held: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        interact_held: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        modifier: false,
        modifier_held: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

/// Hold full right for tests that only need a simple locomotion input.
pub fn hold_right() -> AgentAction {
    AgentAction {
        move_x: 1.0,
        ..base()
    }
}

pub fn fixed_60hz_options() -> Platformer2dSimHarnessOptions {
    Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz())
}

pub fn fixed_60hz_room_options(room: &str) -> Platformer2dSimHarnessOptions {
    fixed_60hz_options().with_required_start_room(room)
}

pub fn fixed_60hz_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(fixed_60hz_options())
        .expect("Platformer2dSimHarness::new")
}

pub fn fixed_60hz_room_sim(room: &str) -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room))
        .expect("Platformer2dSimHarness::new")
}

#[cfg(feature = "portal")]
use ambition_platformer2d::portal::PlacedPortal;

/// Return all currently-live authored portal pairs, after any link resolution.
///
/// `portal_lab` now authors explicit `link` ids. After the app steps once,
/// linked portals are assigned generated `Indexed` channels, so tests should not
/// assume the old Purple/Yellow channels remain on the live `PlacedPortal`s.
#[cfg(feature = "portal")]
pub fn authored_portal_pairs(
    sim: &mut Platformer2dSimHarness,
) -> Vec<(PlacedPortal, PlacedPortal)> {
    let mut q = sim.world_mut().query::<&PlacedPortal>();
    let world = sim.world();
    let mut portals: Vec<PlacedPortal> = q
        .iter(world)
        .filter(|p| !p.channel.is_gun_pair())
        .cloned()
        .collect();
    portals.sort_by(|a, b| {
        a.pos
            .x
            .total_cmp(&b.pos.x)
            .then(a.pos.y.total_cmp(&b.pos.y))
            .then(a.channel.name().cmp(&b.channel.name()))
    });

    let mut pairs = Vec::new();
    for entry in &portals {
        if let Some(exit) = portals
            .iter()
            .find(|candidate| candidate.channel == entry.channel.partner())
        {
            pairs.push((entry.clone(), exit.clone()));
        }
    }
    pairs
}

/// First live authored pair in deterministic left-to-right/top-to-bottom order.
#[cfg(feature = "portal")]
pub fn first_authored_portal_pair(
    sim: &mut Platformer2dSimHarness,
) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .next()
        .expect("room has a linked authored portal pair")
}

/// First floor-to-floor authored pair, used by tests that must exercise a floor
/// carve instead of a wall/ceiling portal.
#[cfg(feature = "portal")]
pub fn first_floor_authored_portal_pair(
    sim: &mut Platformer2dSimHarness,
) -> (PlacedPortal, PlacedPortal) {
    authored_portal_pairs(sim)
        .into_iter()
        .find(|(entry, exit)| entry.normal.y < -0.5 && exit.normal.y < -0.5)
        .expect("room has a linked floor-to-floor authored portal pair")
}

/// Drive `vertical_shaft`'s authored enemy, and return the body and the
/// identity its room minted it under.
///
/// It lives here rather than in one of them because `carried_item_crosses_rooms` and
/// `a_save_remembers_where_you_left_things` are siblings, not a hierarchy.
///
/// the possession is asserted here, once. A test whose setup silently
/// failed to possess anything measures a body nobody is driving, and every
/// assertion about custody below would pass for the wrong reason.
pub fn possess_the_authored_enemy(sim: &mut Platformer2dSimHarness) -> (Entity, SimId) {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::engine_core::BodyKinematics;

    for _ in 0..30 {
        sim.step(base());
    }
    let (actor, id) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &Brain, &BodyKinematics)>();
        q.iter(world)
            .find(|(_, id, _, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, _, _)| (e, id.clone()))
            .expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(actor)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            break;
        }
    }
    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(actor),
        "setup: nothing below is about a driven body unless one is being driven"
    );
    (actor, id)
}

/// THIS App's resident character pages, judged per App.
///
/// The image-stage ledger is a process-global static and `app_it` runs its
/// tests as threads of one process, so reading the ledger's resident rows
/// directly counts every sibling test's pages too — a "leak" that tracks
/// whoever else happens to be running (measured 2026-09-02: 15→14 on one
/// schedule, 20→19 on another, always a page some other App owned). Asset ids
/// are per-arena indices and collide across Apps by construction, so the id
/// alone cannot attribute a row either.
///
/// ⭐ What IS per App: `Assets<Image>` and the asset server's path for each
/// id. So residency is read from this App's own assets, the ledger is consulted
/// only as a CLASSIFIER — "was this PATH demanded on the `character-sheet`
/// road" — and a row counts only when this App's asset at that id has the
/// same path the row names. A sibling's row can share an index; it cannot make
/// a page resident here that is not.
///
/// Returns `(path, megapixels)` for every such page.
pub fn resident_character_pages(app: &bevy::prelude::App) -> Vec<(String, f64)> {
    use bevy::prelude::{Assets, Image};
    let world = app.world();
    let images = world.resource::<Assets<Image>>();
    let server = world.resource::<bevy::asset::AssetServer>();
    let ledger = ambition_platformer2d::sprite_sheet::game_assets::image_stages::ledger();
    let mut out = Vec::new();
    for (id, image) in images.iter() {
        let Some(path) = server.get_path(id).map(|path| path.to_string()) else {
            continue;
        };
        let Some(row) = ledger.get(id.untyped()) else {
            continue;
        };
        if row.source != Some("character-sheet") || row.path.as_deref() != Some(path.as_str()) {
            continue;
        }
        let megapixels = f64::from(image.width()) * f64::from(image.height()) / 1.0e6;
        out.push((path, megapixels));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The resident character pages of THIS App that no realization in its
/// tables (character sheets or props) owns. A retired realization drops its
/// pages' last handles, so a page still resident with no owner is held by
/// something else — the leak the residency rule exists to catch.
pub fn orphan_character_pages(app: &bevy::prelude::App) -> Vec<String> {
    let assets = app
        .world()
        .resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>();
    let owned: std::collections::BTreeSet<String> = assets
        .characters
        .resident_sheets()
        .map(|(_, sheet)| sheet)
        .chain(assets.characters.props.values())
        .flat_map(|sheet| sheet.pages.iter())
        .filter_map(|page| page.texture.path().map(|path| path.to_string()))
        .collect();
    resident_character_pages(app)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| !owned.contains(path))
        .collect()
}
