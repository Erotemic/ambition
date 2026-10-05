//! A pet puts the petter's hand on the place the petted body is petted.
//!
//! The pet's mark was the petted body's front plus a gap, from the two
//! collision boxes. The robot's hand reaches 4 world units in front of its
//! feet, so it stroked the air 15 units in front of the dog's nose. The mark
//! now comes from the landmark query
//! (`ambition_platformer2d::combat::body_landmarks`, ruling Q41): the hand in
//! the `pet` row, and the head in the `petted` row plus the contact offset
//! the dog's catalog row authors (its nose).
//!
//! ⚠ The expected places here do NOT come from that query. They come from the
//! draw table the renderer draws the two rows from
//! (`RiggedSpriteAsset::frame`), at the scale each body states. So the arm
//! also witnesses the build's projection of that table, the sheet a body's
//! character resolves to, and the scale.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::actors::features::ecs::{PetBeat, PetStage};
use ambition_platformer2d::characters::actor::body::{PETTED_CLIPS, PETTING_CLIPS};
use ambition_platformer2d::characters::actor::{BodyAnimFacts, Landmark, WornCharacter};
use ambition_platformer2d::combat::body_landmarks::{BodyLandmarks, LandmarkPose};
use ambition_platformer2d::combat::components::ActorRenderSize;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::sprite_sheet::character::rigged::RiggedSpriteAsset;
use ambition_platformer2d::sprite_sheet::character::SpritePosedBody;
use ambition_platformer2d::vfx::vfx::{VfxInRoom, VfxMessage};
use bevy::math::Vec2;
use bevy::prelude::{Entity, In, Messages, Query};

const DOG: &str = "npc_companion_dog";

/// The bodies of a pet, and the hearts the gesture's first tick wrote.
struct Pet {
    player: Entity,
    dog: Entity,
    hearts: Vec<Vec2>,
}

fn kin(sim: &Platformer2dSimHarness, body: Entity) -> BodyKinematics {
    sim.world().get::<BodyKinematics>(body).expect("a live body").clone()
}

fn petting(sim: &Platformer2dSimHarness, body: Entity) -> bool {
    sim.world().get::<BodyAnimFacts>(body).is_some_and(|anim| anim.petting)
}

/// Talk to the basement dog, choose the pet, and step to the first tick of
/// the gesture.
fn pet_the_dog(sim: &mut Platformer2dSimHarness) -> Pet {
    sim.step_n(base(), 10);
    let dog = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &WornCharacter)>();
        query
            .iter(world)
            .find(|(_, worn)| worn.id() == DOG)
            .map(|(entity, _)| entity)
            .expect("the basement stages the authored dog")
    };
    let player = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<
            Entity,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        query.single(world).expect("one primary player")
    };
    let here = kin(sim, dog).pos;
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    let talking = |sim: &Platformer2dSimHarness| {
        sim.world()
            .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
            .is_some_and(|conversation| conversation.talker() == Some(dog))
    };
    for _ in 0..10 {
        if talking(sim) {
            break;
        }
        sim.step(base());
    }
    assert!(talking(sim), "Interact beside the dog talks to it");
    sim.world_mut()
        .run_system_cached(ambition_content::yarn_vocabulary::cmd_pet)
        .expect("the `<<pet>>` command runs");
    let mut hearts = Vec::new();
    for _ in 0..240 {
        if petting(sim, player) {
            break;
        }
        sim.step(base());
        hearts.extend(
            sim.world()
                .resource::<Messages<VfxInRoom>>()
                .iter_current_update_messages()
                .filter_map(|message| match message.vfx {
                    VfxMessage::Hearts { pos, .. } => Some(pos),
                    _ => None,
                }),
        );
    }
    assert!(
        petting(sim, player),
        "the choice pets the dog: beat {:?}",
        sim.world().get::<PetBeat>(player),
    );
    Pet { player, dog, hearts }
}

/// The failure of a checkout that did not publish `target`'s part flipbook.
/// Published sprites are gitignored, and without the flipbook the build embeds
/// no landmark table, so the pet falls back to the box mark.
fn unpublished(target: &str) -> String {
    format!(
        "`{target}` publishes no part flipbook on this checkout, so this arm cannot run. \
         Publish it: `scripts/regen/sprites.sh --target {target}`, then build again."
    )
}

/// Where the renderer draws `track` in the middle frame of the first row of
/// `rows` that `target` has: sheet pixels from the feet, +x the way the art
/// faces. The middle frame is the one the pet's mark is planned for.
fn drawn_at_the_middle(target: &str, rows: &[&str], track: &str) -> Vec2 {
    let flipbook = RiggedSpriteAsset::baked(target)
        .unwrap_or_else(|| panic!("{}", unpublished(target)));
    let (row, clip) = rows
        .iter()
        .find_map(|row| flipbook.clip(row).map(|clip| (*row, clip)))
        .unwrap_or_else(|| panic!("`{target}` draws none of {rows:?}"));
    let middle = clip.frame_count() / 2;
    flipbook
        .frame(row, middle)
        .expect("the middle frame")
        .iter()
        .find(|draw| draw.track.is_some_and(|index| flipbook.tracks[usize::from(index)] == track))
        .unwrap_or_else(|| panic!("`{target}` row `{row}` frame {middle} draws no `{track}`"))
        .at
}

/// A feet-relative art point of a body, in the world: the art is mirrored
/// about the feet for a body that faces left.
fn art_point_in_world(body: &BodyKinematics, pixels: Vec2, world_per_pixel: f32) -> Vec2 {
    let feet = body.pos + Vec2::new(0.0, body.size.y * 0.5);
    feet + Vec2::new(body.facing.signum() * pixels.x, pixels.y) * world_per_pixel
}

/// The landmark query, asked from outside the schedule.
fn ask(
    In((body, landmark, chain)): In<(Entity, Landmark, &'static [&'static str])>,
    landmarks: BodyLandmarks,
    bodies: Query<&BodyKinematics>,
) -> Option<Vec2> {
    let pose = LandmarkPose::Clip { chain, phase: 0.5 };
    landmarks.in_world(body, landmark, pose, bodies.get(body).ok()?, Vec2::new(0.0, 1.0))
}

/// The hand and the head of a pet in progress, each two ways: from the
/// renderer's draw table (`drawn`), and from the landmark query (`asked`).
/// `point` is where the dog is petted: its drawn head, plus the offset its
/// catalog row authors.
struct Contact {
    drawn_hand: Vec2,
    drawn_head: Vec2,
    point: Vec2,
    asked_hand: Option<Vec2>,
    asked_head: Option<Vec2>,
}

fn contact(sim: &mut Platformer2dSimHarness, pet: &Pet) -> Contact {
    let (player, dog) = (kin(sim, pet.player), kin(sim, pet.dog));
    let robot_sheet = sim
        .world()
        .get::<SpritePosedBody>(pet.player)
        .expect("P1: the player robot is a posed body, which states its own art scale")
        .clone();
    let dog_quad = sim
        .world()
        .get::<ActorRenderSize>(pet.dog)
        .expect("P1: the dog states the quad its frame is drawn in")
        .0;
    let dog_frame = RiggedSpriteAsset::baked("companion_dog")
        .unwrap_or_else(|| panic!("{}", unpublished("companion_dog")))
        .frame_size;
    let drawn_hand = art_point_in_world(
        &player,
        drawn_at_the_middle(&robot_sheet.target, PETTING_CLIPS, "near_hand"),
        robot_sheet.world_per_pixel,
    );
    let drawn_head = art_point_in_world(
        &dog,
        drawn_at_the_middle("companion_dog", PETTED_CLIPS, "head"),
        dog_quad.y / dog_frame.y as f32,
    );
    let offset = sim
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .get(DOG)
        .and_then(|row| row.petting.as_ref())
        .expect("the dog's row authors `petting`")
        .contact_offset;
    let mut asked = |body, landmark, chain| {
        sim.world_mut()
            .run_system_cached_with(ask, (body, landmark, chain))
            .expect("the query runs")
    };
    Contact {
        drawn_hand,
        drawn_head,
        point: drawn_head + Vec2::new(dog.facing.signum() * offset.0, offset.1),
        asked_hand: asked(pet.player, Landmark::HandNear, PETTING_CLIPS),
        asked_head: asked(pet.dog, Landmark::Head, PETTED_CLIPS),
    }
}

/// How far the hand may be from the contact point, in world units: the walk's
/// arrive tolerance (2.0), the shift of a posed body's pose rectangle from its
/// feet pixel (about 1.5 for the robot's `pet` row), and the difference in
/// height between the robot's hand and the dog's nose (under 1.0).
const HAND_ON_CONTACT: f32 = 4.0;

#[test]
fn the_petting_hand_is_on_the_place_the_dog_is_petted() {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    let pet = pet_the_dog(&mut sim);
    let contact = contact(&mut sim, &pet);
    let (player, dog) = (kin(&sim, pet.player), kin(&sim, pet.dog));
    let miss = (contact.drawn_hand - contact.point).length();
    // The mark the two boxes gave: the dog's front, plus the petter's half
    // width, plus the gap of 6.
    let box_mark = dog.size.x * 0.5 + player.size.x * 0.5 + 6.0;
    let stands = (player.pos.x - dog.pos.x).abs();
    eprintln!(
        "pet: the petter stands {stands:.1} from the dog (the box mark was {box_mark:.1}); \
         drawn hand {:?}, contact point {:?}, miss {miss:.2}; hearts {:?}",
        contact.drawn_hand, contact.point, pet.hearts,
    );
    assert!(
        miss <= HAND_ON_CONTACT,
        "the petting hand is {miss:.1} world units from the place the dog is petted (hand {:?}, \
         contact point {:?}): the petter stands {stands:.1} from the dog",
        contact.drawn_hand,
        contact.point,
    );
    // Control: the two marks are far apart for this pair, so this arm tells
    // them apart. On the box mark the same hand is this far from the head.
    assert!(
        box_mark - stands > 3.0 * HAND_ON_CONTACT,
        "control: the box mark ({box_mark:.1}) and the landmark mark ({stands:.1}) are too near \
         for this pair to witness which one the pet used",
    );

    // The query gives the places the renderer draws.
    let near = |asked: Option<Vec2>, drawn: Vec2, what: &str| {
        let asked = asked.unwrap_or_else(|| panic!("the landmark query has no answer for {what}"));
        assert!(
            (asked - drawn).length() < 0.01,
            "the query puts {what} at {asked:?}; the renderer draws it at {drawn:?}",
        );
    };
    near(contact.asked_hand, contact.drawn_hand, "the robot's near hand in its pet row");
    near(contact.asked_head, contact.drawn_head, "the dog's head in its petted row");

    // The hearts rise from the head, on the tick the gesture starts.
    assert_eq!(pet.hearts.len(), 1, "one burst of hearts: {:?}", pet.hearts);
    assert!(
        (pet.hearts[0] - contact.drawn_head).length() < 0.01,
        "the hearts rise from {:?}; the head is at {:?}",
        pet.hearts[0],
        contact.drawn_head,
    );

    // The petter now stands over the dog's front. Nothing may end the gesture
    // for that: it lasts its length, and neither body is pushed.
    let before = (player.pos, dog.pos);
    sim.step_n(base(), 60);
    assert!(
        matches!(
            sim.world().get::<PetBeat>(pet.player).map(|beat| beat.stage),
            Some(PetStage::Gesture { .. })
        ),
        "the gesture ended early with the petter over the dog: {:?}",
        sim.world().get::<PetBeat>(pet.player),
    );
    let after = (kin(&sim, pet.player).pos, kin(&sim, pet.dog).pos);
    assert!(
        (after.0 - before.0).length() < 0.5 && (after.1 - before.1).length() < 0.5,
        "a body moved during the gesture: {before:?} -> {after:?}",
    );
}

/// The landmark tables are compiled in, and a landmark is a simulation input.
/// The session's prepared content carries their digest in its own section, so
/// two builds with different tables are two content identities.
#[test]
fn the_session_content_identity_covers_the_landmark_tables() {
    use ambition_platformer2d::provider::lifecycle::BAKED_LANDMARKS_SECTION;
    use ambition_platformer2d::runtime::PreparedContent;
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 2);
    let world = sim.world_mut();
    let mut query = world.query::<&PreparedContent>();
    let content = query.single(world).expect("one prepared session");
    let section = content
        .sections()
        .iter()
        .find(|section| section.name == BAKED_LANDMARKS_SECTION)
        .expect("the prepared session content has the landmark section");
    assert_eq!(
        section.canonical_bytes(),
        ambition_platformer2d::sprite_sheet::baked_landmarks::BAKED_LANDMARKS_DIGEST.as_bytes(),
    );
}
