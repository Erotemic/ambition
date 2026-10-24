//! The hand of the pose a body draws now, for an item that is drawn in it.
//!
//! An item in a hand was drawn at a fixed place on the body's box and turned
//! there by a formula. The body's art did not know: an arm went one way and
//! the item another. Here the art leads. A body drawn from parts has a hand
//! part, and each frame of each row puts that part somewhere and turns it. An
//! item is held rigidly in that hand: it is where the hand part is, and it
//! turns as the hand part turns. A swing is then a row of the character's
//! art, with the item in its hand through the whole of it.
//!
//! The reader asks a holder's rigged root for its hand tracks
//! ([`ask_holders_for_their_hands`]) and reads the answer of the frame that
//! is drawn ([`PosedHands::of`]). A body that is not drawn from parts, or
//! whose art names no hand, has no answer, and the item is drawn as before.
//!
//! THE CONTRACT WITH THE ART: a hand part that is not turned holds an item
//! that points the way the body faces and [`ITEM_REST_ANGLE`] degrees up.
//! That is true of each hand part, the far one also: a rig whose far hand is
//! drawn the other way round must turn it in its item rows.

use std::collections::BTreeSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_characters::actor::GestureHand;
use ambition_sim_view::{HeldItemView, HostileWieldedItemsView, ItemHolder};

use super::actors::rigged::{ask_for_parts, forget_parts, PartSetDraws, PartSetRequest};
use super::primitives::FeatureVisual;

/// How far up an item points in a hand that is not turned, in degrees from
/// level.
pub const ITEM_REST_ANGLE: f32 = 24.0;

/// The names of the two asks. One ask for each hand, because an answer does
/// not say which track a part is of.
const NEAR_READER: &str = "held_item:near_hand";
const FAR_READER: &str = "held_item:far_hand";

/// The tracks that are a hand, as the landmark tables name them
/// (`ambition_sprite_sheet::character::landmarks_published::LANDMARK_TRACKS`).
const NEAR_TRACKS: [&str; 3] = ["near_hand", "front_hand", "right_hand"];
const FAR_TRACKS: [&str; 3] = ["far_hand", "back_hand", "left_hand"];

/// The hand of the frame a body draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PosedHand {
    /// Where the hand is, in the space sprites are drawn in.
    pub at: Vec2,
    /// The depth the hand part is drawn at, in that space. An item drawn
    /// just under it is in front of the parts drawn before the hand (the
    /// body, for the near hand) and behind the fist's fingers.
    pub z: f32,
    /// The way an item in the hand points, in that space (+y up). Unit.
    pub along: Vec2,
    /// The body is drawn as its mirror image.
    pub mirrored: bool,
}

impl PosedHand {
    /// The turn of a sprite whose long side goes along the item.
    pub fn angle(&self) -> f32 {
        self.along.y.atan2(self.along.x)
    }
}

/// The hand `hand` of the frame `draws` is of, else the other hand. `None`
/// when the body is not drawn from parts this frame, or draws no hand in it.
pub fn posed_hand(draws: &PartSetDraws, hand: GestureHand) -> Option<PosedHand> {
    let readers = match hand {
        GestureHand::Near => [NEAR_READER, FAR_READER],
        GestureHand::Far => [FAR_READER, NEAR_READER],
    };
    // The last draw of a track is the one on top.
    let part = readers.into_iter().find_map(|reader| draws.parts(reader).last())?;
    let world = draws.place.to_matrix() * part.local.to_matrix();
    let rest = ITEM_REST_ANGLE.to_radians();
    let along = world.transform_vector3(Vec3::new(rest.cos(), rest.sin(), 0.0)).truncate().try_normalize()?;
    let x = world.transform_vector3(Vec3::X).truncate();
    let y = world.transform_vector3(Vec3::Y).truncate();
    let pivot = world.transform_point3(Vec3::ZERO);
    Some(PosedHand {
        at: pivot.truncate(),
        z: pivot.z,
        along,
        mirrored: x.perp_dot(y) < 0.0,
    })
}

/// The hands of the bodies that are drawn. See the module docs.
#[derive(SystemParam)]
pub struct PosedHands<'w, 's> {
    draws: Query<'w, 's, (&'static PartSetDraws, Option<&'static ambition_sprite_sheet::character::CharacterAnimator>)>,
    // A feature's stand-in (`UnclaimedBodyPlaceholder`) can carry the feature's
    // id for a frame of a transition, and it draws no parts.
    visuals: Query<'w, 's, (Entity, &'static FeatureVisual), Without<super::features::UnclaimedBodyPlaceholder>>,
}

impl PosedHands<'_, '_> {
    /// The rigged root that draws `holder`: the visual of its feature when it
    /// is an actor (a separate entity that carries the feature's id), else
    /// the holder itself (a player's root is its simulation body). Never the
    /// feature's stand-in, which carries the same id in a transition.
    pub fn root(&self, holder: &ItemHolder) -> Entity {
        holder
            .feature
            .as_deref()
            .and_then(|feature| self.visuals.iter().find(|(_, visual)| visual.id == feature))
            .map_or(holder.body, |(root, _)| root)
    }

    /// The hand `holder` holds an item in, in the frame it draws now.
    ///
    /// A sheet that is drawn from both sides answers a turn with its mirror
    /// row: the body seen from its other side. The item stays in the hand
    /// that holds it, and from the other side that hand is the other one
    /// (near for far), as the blade of the player robot is.
    pub fn of(&self, holder: &ItemHolder) -> Option<PosedHand> {
        let (draws, animator) = self.draws.get(self.root(holder)).ok()?;
        let hand = match (animator.is_some_and(|animator| animator.draws_mirror_row()), holder.hand) {
            (false, hand) => hand,
            (true, GestureHand::Near) => GestureHand::Far,
            (true, GestureHand::Far) => GestureHand::Near,
        };
        posed_hand(draws, hand)
    }
}

/// The presentation of held and wielded items: the in-hand and over-hand
/// sprites and the hand requests they read. It runs while a body holds or
/// wields an item, and on the frame after the last one lets go, so the pass
/// with no holders takes the sprites down and forgets the requests.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HeldItemPresentationSet;

/// [`HeldItemPresentationSet`]'s condition. Both views are rebuilt each frame,
/// so their change ticks cannot tell an empty frame from a full one.
pub fn items_are_held_or_were(
    held: Option<Res<HeldItemView>>,
    wielded: Option<Res<HostileWieldedItemsView>>,
    mut were: Local<bool>,
) -> bool {
    let held = held.is_some_and(|view| !view.0.is_empty()) || wielded.is_some_and(|view| !view.0.is_empty());
    let run = held || *were;
    *were = held;
    run
}

/// Ask the root of each body that holds an item for its hand tracks, and
/// forget the ask of a root whose body holds none.
///
/// An ask is answered from the next frame that is drawn; the item is drawn
/// the old way for the one frame between.
pub fn ask_holders_for_their_hands(
    mut commands: Commands,
    held: Res<HeldItemView>,
    wielded: Res<HostileWieldedItemsView>,
    hands: PosedHands,
    asked: Query<(Entity, &PartSetRequest)>,
) {
    let holders = held.0.iter().map(|fact| &fact.holder).chain(wielded.0.iter().map(|fact| &fact.holder));
    let wanted: BTreeSet<Entity> = holders.map(|holder| hands.root(holder)).collect();
    for &root in &wanted {
        if asked.get(root).is_ok_and(|(_, request)| request.tracks_of(NEAR_READER).is_some()) {
            continue;
        }
        ask_for_parts(&mut commands, root, NEAR_READER, &NEAR_TRACKS);
        ask_for_parts(&mut commands, root, FAR_READER, &FAR_TRACKS);
    }
    for (root, request) in &asked {
        if request.tracks_of(NEAR_READER).is_some() && !wanted.contains(&root) {
            forget_parts(&mut commands, root, NEAR_READER);
            forget_parts(&mut commands, root, FAR_READER);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::actors::rigged::PlacedPart;

    /// The held-item pass runs while an item is wielded and on the one frame
    /// after the last is let go (that pass takes its sprite down), then
    /// sleeps; a frame with no view at all also sleeps.
    #[test]
    fn the_held_item_pass_runs_while_an_item_is_held_and_one_frame_after() {
        use ambition_platformer2d_core as ae;
        let mut world = World::new();
        let condition = world.register_system(items_are_held_or_were);
        let run = |world: &mut World| world.run_system(condition).expect("the condition runs");
        assert!(!run(&mut world), "no views: asleep");
        world.insert_resource(HeldItemView::default());
        world.insert_resource(HostileWieldedItemsView(vec![ambition_sim_view::HostileWieldedItemFact {
            item_id: "gun_sword".into(),
            hand_world: ae::Vec2::ZERO,
            aim_world: ae::Vec2::X,
            wielder_height: 48.0,
            room: None,
            ranged: true,
            holder: ItemHolder {
                body: Entity::PLACEHOLDER,
                feature: None,
                hand: GestureHand::default(),
            },
        }]));
        assert!(run(&mut world), "an item is wielded");
        world.resource_mut::<HostileWieldedItemsView>().0.clear();
        assert!(run(&mut world), "the frame after it is let go");
        assert!(!run(&mut world), "then asleep");
    }

    fn part(at: Vec2, turn: f32) -> PlacedPart {
        PlacedPart {
            image: Handle::default(),
            rect: Rect::new(0.0, 0.0, 4.0, 4.0),
            size: Vec2::splat(4.0),
            anchor: Vec2::ZERO,
            local: Transform::from_translation(at.extend(0.0)).with_rotation(Quat::from_rotation_z(turn)),
        }
    }

    /// A hand that is not turned holds an item that points forward and
    /// `ITEM_REST_ANGLE` up; the hand is where the part is, under the place
    /// of the body.
    #[test]
    fn a_hand_at_rest_holds_the_item_up_and_forward() {
        let place = Transform::from_translation(Vec3::new(100.0, 50.0, 0.0)).with_scale(Vec3::new(2.0, 2.0, 1.0));
        let draws = PartSetDraws::from_parts(place, NEAR_READER, vec![part(Vec2::new(10.0, 20.0), 0.0)]);
        let hand = posed_hand(&draws, GestureHand::Near).expect("the near hand is drawn");
        assert_eq!(hand.at, Vec2::new(120.0, 90.0));
        assert!((hand.angle().to_degrees() - ITEM_REST_ANGLE).abs() < 1e-3);
        assert!(!hand.mirrored);
    }

    /// The item turns as the hand part turns.
    #[test]
    fn the_item_turns_with_the_hand() {
        let draws = PartSetDraws::from_parts(
            Transform::IDENTITY,
            NEAR_READER,
            vec![part(Vec2::ZERO, 90.0_f32.to_radians())],
        );
        let hand = posed_hand(&draws, GestureHand::Near).expect("the near hand is drawn");
        assert!((hand.angle().to_degrees() - (ITEM_REST_ANGLE + 90.0)).abs() < 1e-3);
    }

    /// A body that faces the other way is drawn as its mirror image: the
    /// hand is on the other side of it, and the item points the other way,
    /// still up.
    #[test]
    fn a_mirrored_body_holds_the_item_the_other_way() {
        let place = Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let draws = PartSetDraws::from_parts(place, NEAR_READER, vec![part(Vec2::new(10.0, 0.0), 0.0)]);
        let hand = posed_hand(&draws, GestureHand::Near).expect("the near hand is drawn");
        assert_eq!(hand.at, Vec2::new(-10.0, 0.0));
        assert!(hand.mirrored);
        assert!(hand.along.x < 0.0 && hand.along.y > 0.0);
    }

    /// A character that holds with its far hand is answered with that hand,
    /// and with the other when the frame does not draw it. A body that is
    /// not drawn from parts has no answer.
    #[test]
    fn the_stated_hand_is_first_and_the_other_is_the_fallback() {
        let near_only = PartSetDraws::from_parts(Transform::IDENTITY, NEAR_READER, vec![part(Vec2::new(1.0, 0.0), 0.0)]);
        assert_eq!(posed_hand(&near_only, GestureHand::Far).map(|hand| hand.at), Some(Vec2::new(1.0, 0.0)));
        assert!(posed_hand(&PartSetDraws::default(), GestureHand::Near).is_none());
    }

    /// ⭐ THE ROOT OF A FEATURE IS ITS RIGGED VISUAL, NOT ITS STAND-IN. In a
    /// transition the feature's placeholder (`UnclaimedBodyPlaceholder`) can
    /// carry the same id as its rigged visual, and the first visual with the
    /// id was taken. The placeholder draws no parts, so the hand was `None` and
    /// the item went back to the old placement for that frame (review of
    /// 80116a77b, finding 2). The stand-in is spawned first here, so a scan
    /// meets it first.
    #[test]
    fn the_root_of_a_feature_is_its_rigged_visual_and_not_its_stand_in() {
        use bevy::ecs::system::SystemState;
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let stand_in = world
            .spawn((FeatureVisual { id: "npc".into() }, crate::rendering::features::UnclaimedBodyPlaceholder))
            .id();
        let draws = PartSetDraws::from_parts(Transform::IDENTITY, NEAR_READER, vec![part(Vec2::new(1.0, 0.0), 0.0)]);
        let rigged = world.spawn((FeatureVisual { id: "npc".into() }, draws)).id();
        let holder = ItemHolder { body, feature: Some("npc".into()), hand: GestureHand::Near };
        let mut state = SystemState::<PosedHands>::new(&mut world);
        let hands = state.get(&world).expect("the hands of this world");
        assert_eq!(hands.root(&holder), rigged, "the stand-in {stand_in:?} was taken for the root");
        assert_eq!(hands.of(&holder).map(|hand| hand.at), Some(Vec2::new(1.0, 0.0)), "the hand of the rigged visual");
    }
}
