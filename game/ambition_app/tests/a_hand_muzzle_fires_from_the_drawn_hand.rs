//! A weapon that fires from the hand fires from the hand the art draws.
//!
//! A hand muzzle (`Muzzle::Hand`: the gun-sword, the Officer's sidearm) fired
//! from a fixed offset from the body's centre for every body with no rig: a
//! fifth of its height ahead, at chest height. The pirates hold the gun-sword
//! at the hip. The shot now leaves the hand the body's art draws on the tick
//! it fires, from the landmark query (ruling Q41): the same place a rigged
//! body fires from.
//!
//! ⚠ The expected hand does NOT come from that query's table. It comes from
//! the draw table the renderer draws the body's row from, at the scale the
//! body states.

#![cfg(feature = "rl_sim")]

use crate::a_pet_hand_meets_the_contact_point::{art_point_in_world, unpublished};
use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::combat::components::{ActorRenderSize, FeatureId};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::projectiles::ProjectileSpawnRequest;
use ambition_platformer2d::sprite_sheet::character::rigged::RiggedSpriteAsset;
use bevy::math::Vec2;
use bevy::prelude::{Entity, Messages, With};

const RAIDER: &str = "npc_pirate_raider";
const SHEET: &str = "pirate_raider";
/// `gun_sword_discharge`: the muzzle is this far past the hand, along the shot.
const AHEAD: f32 = 18.0;

/// The row and frame a body's own clocks select on this tick, by the rule the
/// rig and the landmark query share, and the place of `track` in that frame
/// from the DRAW table, in the world.
pub(crate) fn drawn_hand_this_tick(
    world: &bevy::prelude::World,
    entity: Entity,
    body: &BodyKinematics,
    sheet: &str,
    world_per_pixel: f32,
    track: &str,
) -> (String, usize, Vec2) {
    use ambition_platformer2d::combat::body_rig::select_pose_frame;
    use ambition_platformer2d::combat::hurtbox_resolution::BodyPoseClock;
    use ambition_platformer2d::combat::moveset::MovePlayback;
    let flipbook = RiggedSpriteAsset::baked(sheet).unwrap_or_else(|| panic!("{}", unpublished(sheet)));
    let table = ambition_platformer2d::sprite_sheet::baked_landmarks::body_landmarks(sheet)
        .unwrap_or_else(|| panic!("{}", unpublished(sheet)));
    let playback = world.get::<MovePlayback>(entity);
    let clock = world.get::<BodyPoseClock>(entity);
    let (row, frame) = select_pose_frame(
        table.as_ref(),
        playback.map(|playback| (&playback.spec.clip, playback.phase())),
        clock.map(|clock| (clock.pose.as_str(), clock.elapsed_s)),
        clock.map(|clock| (clock.gait, clock.gait_elapsed_s)),
    )
    .expect("the art has a row");
    let pixels = flipbook
        .frame(row, frame)
        .expect("the selected frame")
        .iter()
        .find(|draw| draw.track.is_some_and(|index| flipbook.tracks[usize::from(index)] == track))
        .unwrap_or_else(|| panic!("`{sheet}` row `{row}` frame {frame} draws no `{track}`"))
        .at;
    (row.to_string(), frame, art_point_in_world(body, pixels, world_per_pixel))
}

/// The shot `owner` asks for on this tick: where it is born, its half extent
/// and its unit direction.
fn shot_of(sim: &ambition_app::Platformer2dSimHarness, owner: Entity) -> Option<(Vec2, Vec2, Vec2)> {
    sim.world()
        .resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages()
        .find(|request| request.owner == owner)
        .map(|request| {
            let kin = &request.projectile.body.kin;
            (kin.pos, kin.size * 0.5, kin.vel.normalize_or_zero())
        })
}

/// A hostile raider on foot fires its gun-sword at the player. The shot is
/// born past the hand its art draws on that tick.
#[test]
fn a_hostile_raiders_shot_is_born_at_the_hand_its_art_draws() {
    // A room with a flat floor: the two bodies stand level, and the shot has
    // a clear line. Not `combat_calibration_lab`: there a shot born at chest
    // height at this place is gone on its first tick (measured 2026-10-05,
    // cause not found), so that room cannot compare two heights.
    let mut sim = fixed_60hz_room_sim("mockingbird_arena");
    sim.step_n(base(), 60);
    let player = {
        let world = sim.world_mut();
        let mut query = world
            .query_filtered::<(Entity, &BodyKinematics), With<ambition_platformer2d::platformer::body::PrimaryBody>>();
        let (entity, kin) = query.single(world).expect("one primary body");
        (entity, kin.pos)
    };
    sim.spawn_enemy_character_at(
        "hand_muzzle_raider",
        "Pirate Raider",
        (player.1.x + 160.0, player.1.y),
        (22.0, 39.0),
        CharacterBrain::Custom("pirate_raider".to_string()),
        RAIDER,
    );
    let raider = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &FeatureId)>();
        query
            .iter(world)
            .find(|(_, id)| id.as_str() == "hand_muzzle_raider")
            .map(|(entity, _)| entity)
            .expect("the raider was built")
    };
    let frame_height = RiggedSpriteAsset::baked(SHEET)
        .unwrap_or_else(|| panic!("{}", unpublished(SHEET)))
        .frame_size
        .y as f32;
    let quad = sim
        .world()
        .get::<ActorRenderSize>(raider)
        .expect("a hostile body states the quad its art is drawn in")
        .0;
    let world_per_pixel = quad.y / frame_height;

    let mut fired = None;
    for _ in 0..900 {
        sim.step(base());
        if let Some(request) = shot_of(&sim, raider) {
            fired = Some(request);
            break;
        }
    }
    let (born, half, direction) = fired.expect("the raider fired no shot in fifteen seconds");
    let body = sim.world().get::<BodyKinematics>(raider).expect("a live body").clone();
    let target = sim.world().get::<BodyKinematics>(player.0).expect("a live body").clone();
    // The premise of the flight measurement: the two stand on one floor.
    assert!(
        ((body.pos.y + body.size.y * 0.5) - (target.pos.y + target.size.y * 0.5)).abs() < 1.0,
        "the raider at {:?} and the player at {:?} do not stand level",
        body.pos,
        target.pos,
    );
    let (row, frame, hand) = drawn_hand_this_tick(sim.world(), raider, &body, SHEET, world_per_pixel, "front_hand");
    let feet_y = body.pos.y + body.size.y * 0.5;
    let fixed_hand = ambition_platformer2d::mount::rider_hand_world_pos(body.pos, body.facing, body.size.y);
    // The gun-sword the presentation draws over the hand, on this tick.
    let props: Vec<Vec2> = sim
        .world()
        .get_resource::<ambition_platformer2d::sim_view::HostileWieldedItemsView>()
        .expect("the harness publishes the wielded-item view")
        .0
        .iter()
        .filter(|fact| fact.item_id == "gun_sword")
        .map(|fact| fact.hand_world)
        .collect();

    // A measurement, not an assertion: how the shot flies from where it is
    // born. See Q158.
    let health = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world()
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(player.0)
            .map(|health| health.current())
    };
    let health_before = health(&sim);
    let mut path = vec![born];
    for tick in 0..240 {
        sim.step(base());
        let world = sim.world_mut();

        let mut shots = world.query::<(&ambition_platformer2d::projectiles::ProjectileOwner, &BodyKinematics)>();
        match shots.iter(world).find(|(owner, _)| owner.0 == raider) {
            Some((_, kin)) => path.push(kin.pos),
            // The request is this tick's; the shot is built on the next.
            None if tick < 3 => {}
            None => break,
        }
    }
    let player_now = sim.world().get::<BodyKinematics>(player.0).map(|kin| kin.pos);
    // A hit is applied after the shot that made it is gone.
    sim.step_n(base(), 3);
    eprintln!(
        "raider shot: born {:?} from the body centre (body size {:?}, facing {}), {:.1} above the feet line, half \
         {half:?}, flying {direction:?}; row `{row}` frame {frame}; the drawn hand {:?} and the fixed hand {:?} \
         from the centre; alive {} ticks, travelled {:.0}, ended {:?} from the player; player health {:?} -> {:?}",
        born - body.pos,
        body.size,
        body.facing,
        feet_y - born.y,
        hand - body.pos,
        fixed_hand - body.pos,
        path.len(),
        (*path.last().expect("one place") - born).length(),
        player_now.map(|player| *path.last().expect("one place") - player),
        health_before,
        health(&sim),
    );

    // The muzzle is `AHEAD` past the hand along the shot. A hand nearer the
    // feet than the shot is tall lifts the shot clear of the feet line, never
    // down.
    let muzzle = hand + direction * AHEAD;
    let expected = Vec2::new(muzzle.x, muzzle.y.min(feet_y - half.y - 1.0));
    assert!(
        (born - expected).length() < 1.0,
        "the shot is born at {born:?}; {AHEAD} past the drawn hand ({hand:?}, row `{row}` frame {frame}) it is \
         born at {expected:?}",
    );
    // The prop is drawn at the hand the shot leaves: one answer, two readers.
    assert_eq!(props.len(), 1, "one raider wields one gun-sword: {props:?}");
    assert!(
        (props[0] - hand).length() < 1.0,
        "the gun-sword is drawn at {:?} and the shot leaves the hand at {hand:?}",
        props[0],
    );
    // Control: the fixed hand is far from the drawn hand for this body, so
    // this arm tells them apart.
    assert!(
        (fixed_hand - hand).length() > 8.0,
        "control: the fixed hand ({fixed_hand:?}) and the drawn hand ({hand:?}) are too near to witness which \
         one the shot used",
    );
}

/// The player holds the gun-sword and fires it. The shot is born past the
/// hand the robot's art draws on that tick, and not past the fixed hand.
#[test]
fn the_players_held_gun_sword_fires_from_the_hand_its_art_draws() {
    use ambition_platformer2d::sprite_sheet::character::SpritePosedBody;
    use bevy::ecs::system::RunSystemOnce;

    fn hold_the_gun_sword(
        mut commands: bevy::prelude::Commands,
        mut bodies: bevy::prelude::Query<
            (Entity, ambition_platformer2d::combat::hand::RepertoireQuery),
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
        >,
    ) {
        let (player, mut repertoire) = bodies.single_mut().expect("one primary body");
        let spec = ambition_platformer2d::held_items::held_spec_by_id("gun_sword")
            .expect("gun_sword is a registered held item");
        ambition_platformer2d::held_items::equip_held_spec(&mut commands, player, &mut repertoire, spec);
    }

    let mut sim = fixed_60hz_room_sim("mockingbird_arena");
    sim.step_n(base(), 120);
    sim.world_mut().run_system_once(hold_the_gun_sword).expect("the equip ran");
    sim.step_n(base(), 5);
    let player = {
        let world = sim.world_mut();
        let mut query =
            world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
        query.single(world).expect("one primary body")
    };
    let sheet = sim
        .world()
        .get::<SpritePosedBody>(player)
        .expect("the protagonist is a posed body, which states its own art scale")
        .clone();

    sim.step(ambition_app::AgentAction {
        projectile: true,
        projectile_held: true,
        ..base()
    });
    let mut fired = shot_of(&sim, player);
    sim.step(ambition_app::AgentAction {
        projectile_released: true,
        ..base()
    });
    for _ in 0..60 {
        if fired.is_some() {
            break;
        }
        fired = shot_of(&sim, player);
        if fired.is_none() {
            sim.step(base());
        }
    }
    let (born, half, direction) = fired.expect("a tap with the gun-sword in hand fired no shot");
    let body = sim.world().get::<BodyKinematics>(player).expect("a live body").clone();
    let (row, frame, hand) =
        drawn_hand_this_tick(sim.world(), player, &body, &sheet.target, sheet.world_per_pixel, "near_hand");
    let feet_y = body.pos.y + body.size.y * 0.5;
    let fixed_hand = ambition_platformer2d::mount::rider_hand_world_pos(body.pos, body.facing, body.size.y);
    eprintln!(
        "player gun-sword shot: born {:?} from the body centre (body size {:?}, facing {}), {:.1} above the feet \
         line, half {half:?}, flying {direction:?}; row `{row}` frame {frame}; the drawn hand {:?} and the fixed \
         hand {:?} from the centre",
        born - body.pos,
        body.size,
        body.facing,
        feet_y - born.y,
        hand - body.pos,
        fixed_hand - body.pos,
    );
    let muzzle = hand + direction * AHEAD;
    let expected = Vec2::new(muzzle.x, muzzle.y.min(feet_y - half.y - 1.0));
    assert!(
        (born - expected).length() < 1.0,
        "the shot is born at {born:?}; {AHEAD} past the drawn hand ({hand:?}, row `{row}` frame {frame}) it is \
         born at {expected:?}",
    );
    assert!(
        (fixed_hand - hand).length() > 8.0,
        "control: the fixed hand ({fixed_hand:?}) and the drawn hand ({hand:?}) are too near to witness which \
         one the shot used",
    );
}
