//! A block the level PAINTED must still be able to change its picture.
//!
//! one line in `level_1_2()` opted every block in the cavern out of art
//! updates, permanently. The level paints its stone —
//! `for block in &mut room.world.blocks { block.art_color = Some(UNDERGROUND_STONE) }`
//! — and `spawn_block` read that authored colour as *"content has said this
//! shape has no sprite yet"*, dropped the sprite key on the floor and therefore
//! attached no `BoundEntitySprite`. `apply_block_art`, the ONE system that
//! changes a block's picture mid-run, queries `&mut BoundEntitySprite`. No
//! binding, no match, no repaint — ever.
//!
//! `art_color` was saying two things at once: *"draw me as a flat coloured
//! quad"*, which is what the author meant, and *"never update my picture
//! again"*, which is an accident of how the first was implemented. These tests
//! pin the second meaning out of existence: a painted block keeps its flat quad
//! until a game NAMES art for it, and naming art is what takes the paint off.

#![cfg(feature = "visible")]

use bevy::prelude::*;

use ambition_demo_mary_o::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
use ambition_demo_mary_o::level_1_2::{level_1_2, LEVEL_1_2_ROOM_ID};
use ambition_demo_mary_o::powerups::SpentPowerBlocks;
use ambition_demo_mary_o_app::{build_windowed_demo_app_entering, RenderMode};
use ambition_platformer2d::view::EntitySprite;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::render::rendering::BlockVisual;
use ambition_platformer2d::view::GameAssets;

/// Copied from `two_rooms.rs`, whose header explains why both edges are load-bearing: a
/// `PreUpdate` write is silently overwritten under `--workspace` feature unification, and the
/// file that guessed otherwise was red for months.

/// The cavern, entered directly. `--room` exists for exactly this reason
/// : before it, reaching 1-2 meant playing 3328 px of 1-1.
fn cavern() -> App {
    let mut app = build_windowed_demo_app_entering(
        RenderMode::Headless,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        LEVEL_1_2_ROOM_ID,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn hold_jump() -> ControlFrame {
    ControlFrame {
        jump_pressed: true,
        jump_held: true,
        ..ControlFrame::default()
    }
}

fn authored(look: MaryOBlockLook) -> ae::world::Block {
    let room = level_1_2();
    let mut found: Vec<_> = room
        .world
        .blocks
        .iter()
        .filter(|block| block_look_of(&block.name) == Some(look))
        .cloned()
        .collect();
    assert_eq!(
        found.len(),
        1,
        "1-2 should author exactly one {look:?} block; found {} ({:?})",
        found.len(),
        found.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
    );
    found.remove(0)
}

/// The image the live render entity for this block is drawing, or `None` when
/// NOTHING is drawing it.
///
/// keyed by `geo_id`, which is what a bonk arrives with, not by the human
/// name — the two are kept side by side on `BlockVisual` for this reason.
///
/// `Option`, not a panic, because "there is no visual" is an ANSWER here.
/// helper that panics on the absent case reports it as a broken probe.
fn drawn_image_opt(app: &mut App, geo_id: &ae::GeoId) -> Option<Handle<Image>> {
    let mut query = app.world_mut().query::<(&BlockVisual, &Sprite)>();
    let world = app.world();
    query
        .iter(world)
        .find(|(visual, _)| &visual.geo_id == geo_id)
        .map(|(_, sprite)| sprite.image.clone())
}

fn drawn_image(app: &mut App, geo_id: &ae::GeoId) -> Handle<Image> {
    drawn_image_opt(app, geo_id).unwrap_or_else(|| panic!("no block visual is drawing {geo_id:?}"))
}

/// The catalog's handle for a named sprite — what "wearing that art" MEANS.
fn art(app: &App, key: EntitySprite) -> Handle<Image> {
    app.world()
        .get_resource::<GameAssets>()
        .expect("a drawn composition has GameAssets")
        .entities
        .get(key)
        .unwrap_or_else(|| panic!("the catalog holds no image for {key:?}"))
        .clone()
}

#[test]
fn a_question_block_in_the_painted_cavern_wears_its_own_art() {
    let mut app = cavern();
    let block = authored(MaryOBlockLook::Question);

    assert_eq!(
        drawn_image(&mut app, &block.id),
        art(&app, EntitySprite::BonusBlockTile),
        "the cavern's ?-block is not drawing the bonus plate — the level painted \
         it and the paint took its art away",
    );

    app.world_mut()
        .resource_mut::<SpentPowerBlocks>()
        .spend(block.id.clone());
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        drawn_image(&mut app, &block.id),
        art(&app, EntitySprite::SpentBlockTile),
        "a spent ?-block in 1-2 still does not look spent",
    );
}

/// the poison. A block nobody names art for keeps the flat quad the level
/// painted. The fix must not turn "no art yet" into "the kind's texture,
/// eventually" — that would repaint every honest placeholder in the engine.
#[test]
fn a_painted_block_nobody_dresses_keeps_its_flat_quad() {
    let mut app = cavern();
    let stone = level_1_2()
        .world
        .blocks
        .iter()
        .find(|block| block_look_of(&block.name).is_none())
        .expect("1-2 authors terrain")
        .clone();

    assert_eq!(
        drawn_image(&mut app, &stone.id),
        Handle::default(),
        "the cavern's own stone bound a texture — the authored colour is \
         supposed to be the whole picture for a block no game has dressed",
    );
}

/// A hidden bonk-only block must report underside contact and pay out when hit
/// from below. This test observes game state rather than rendering; discovery
/// rendering is covered separately.
#[test]
fn the_invisible_brick_triggers_from_below() {
    let mut app = cavern();
    let hidden = authored(MaryOBlockLook::Hidden);
    let underside = hidden.aabb.max.y;
    let strike = jump_into_from_below(&mut app, &hidden);

    // the SECOND term: her head actually got there. Asserted BEFORE the
    // payout so a probe that could not reach the block says so in its own name
    // instead of convicting the game.
    assert!(
        strike.apex_head <= underside + 2.0,
        "the probe never got her head into the brick, so it proves nothing \
         about the trigger. She left the floor at {:?}, her head peaked at \
         y={} and the brick's underside is at y={underside} (y grows downward)",
        strike.launched_from,
        strike.apex_head,
    );

    assert!(
        strike.spent,
        "she jumped into the invisible brick from below, her head reached \
         y={} against an underside at y={underside}, and it never paid \
         out. The trigger is dead, which is a DIFFERENT bug from not being able \
         to draw itself.",
        strike.apex_head,
    );

    // the EFFECT a player would see. A coin block credits the purse
    // directly rather than dropping a pickup, so the coin count IS the payout —
    // and `SpentPowerBlocks` alone would go green on a block that acknowledged
    // the strike and gave nothing.
    assert!(
        player_purse(&mut app) > strike.purse_before,
        "the brick was marked spent but paid nothing: purse {} → {}",
        strike.purse_before,
        player_purse(&mut app),
    );
}

/// the mechanism is a REPLACEMENT read as a DELETION.
/// `contribute_discovered_hidden_blocks_to_overlay` promotes the struck
/// `BonkOnly` to a `Solid` by pushing the block's own name into
/// `removed_block_names` AND the promoted block into `blocks` — the same name,
/// the same box, the same `GeoId`. `sync_removed_block_visuals` read only the
/// first half and despawned the sprite that the second half is asking for.
///
/// this is a SECOND test rather than more assertions on the trigger test,
/// because they fail for different reasons and a merged one cannot say which.
#[test]
fn a_discovered_hidden_block_reveals_itself() {
    let mut app = cavern();
    let hidden = authored(MaryOBlockLook::Hidden);

    // the pre-assertion: it IS drawn before it is struck — as a fully
    // transparent quad, which is the whole trick. A probe that started from a
    // block nothing was drawing could not tell "the reveal is broken" from
    // "this block was never in the render world".
    assert!(
        drawn_image_opt(&mut app, &hidden.id).is_some(),
        "nothing is drawing the hidden block BEFORE it is struck, so this probe \
         cannot say anything about what discovery does to its picture",
    );

    let strike = jump_into_from_below(&mut app, &hidden);
    assert!(
        strike.spent,
        "she never struck the hidden block (head peaked at y={}, underside \
         y={}), so the reveal was never asked for",
        strike.apex_head, hidden.aabb.max.y,
    );

    // Well past the promotion: the overlay is rebuilt every frame and the
    // renderer reconciles against it every frame, so a picture that survives 60
    // of them is not a one-frame accident.
    for _ in 0..60 {
        step(&mut app, ControlFrame::default());
    }

    // re-asserted AFTER the wait, not just before it. `SpentPowerBlocks`
    // is re-armed by `RoomLoaded` and by a replay; if one of those fired during
    // the wait the block is no longer discovered and the picture assertion below
    // would be asking the wrong question.
    assert!(
        app.world()
            .resource::<SpentPowerBlocks>()
            .is_spent(&hidden.id),
        "the block un-spent itself during the wait — something re-armed it, and \
         this probe is no longer looking at a discovered block",
    );

    assert_eq!(
        drawn_image_opt(&mut app, &hidden.id),
        Some(art(&app, EntitySprite::SpentBlockTile)),
        "a discovered hidden block is not wearing the spent plate. `None` means \
         its picture was DELETED: the promotion put its name in \
         `removed_block_names` and the render side read that as a deletion \
         rather than the replacement it is (queue D69)",
    );
}

/// What a strike from below observed, so the two probes above can each assert
/// the term they are about.
struct Strike {
    /// Where she was standing the frame she left the floor.
    launched_from: Vec2,
    /// The highest her head reached — y grows DOWNWARD, so the minimum.
    apex_head: f32,
    /// Her coin count the frame she left the floor.
    purse_before: i32,
    /// Whether the block ended up in `SpentPowerBlocks`.
    spent: bool,
}

/// Stand her on the nearest floor under `block` and jump into its underside.
///
/// Derive the setup from the room so authored block moves remain valid and the jump starts
/// from footing rather than mid-air.
///
/// her FOOTING is asserted HERE, inside the setup. A jump script that
/// never gets her off the ground reports "the block is broken" — a falsifier
/// watching the wrong thing, and this beat is precisely where that mistake is
/// cheap to make.
fn jump_into_from_below(app: &mut App, block: &ae::world::Block) -> Strike {
    let column = block.aabb.center().x;
    let underside = block.aabb.max.y;
    let floor_top = level_1_2()
        .world
        .blocks
        .iter()
        .filter(|candidate| !matches!(candidate.kind, ae::BlockKind::BonkOnly))
        .filter(|candidate| candidate.aabb.min.x <= column && column <= candidate.aabb.max.x)
        .filter(|candidate| candidate.aabb.min.y > underside)
        .map(|candidate| candidate.aabb.min.y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        floor_top.is_finite(),
        "nothing to stand on under the block at x={column}",
    );

    let half_height = player_box(app).y * 0.5;
    place_player(app, Vec2::new(column, floor_top - half_height - 1.0));

    let mut grounded = false;
    for _ in 0..90 {
        step(app, ControlFrame::default());
        if player_on_ground(app) {
            grounded = true;
            break;
        }
    }
    assert!(
        grounded,
        "she never found footing under the block — the probe cannot say \
         anything about it. floor top y={floor_top}, box {:?}, she ended at {:?}",
        player_box(app),
        player_pos(app),
    );

    let launched_from = player_pos(app);
    let purse_before = player_purse(app);
    let mut apex_head = f32::INFINITY;
    let mut spent = false;
    for _ in 0..120 {
        step(app, hold_jump());
        apex_head = apex_head.min(player_pos(app).y - half_height);
        if app
            .world()
            .resource::<SpentPowerBlocks>()
            .is_spent(&block.id)
        {
            spent = true;
            break;
        }
    }
    Strike {
        launched_from,
        apex_head,
        purse_before,
        spent,
    }
}

fn player_pos(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .pos
}

/// Her live collision box, asked of the sim rather than hardcoded — her forms
/// are different sizes and the sheets author them.
fn player_box(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .size
}

fn player_on_ground(app: &mut App) -> bool {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyGroundState, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .is_some_and(|ground| ground.on_ground)
}

/// The coin count. A coin BLOCK credits this directly — it spawns no pickup —
/// so this is the payout a player actually sees.
fn player_purse(app: &mut App) -> i32 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .map(|wallet| wallet.balance)
        .unwrap_or(0)
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ae::movement::TransitVelocity::Zero,
    );
}
