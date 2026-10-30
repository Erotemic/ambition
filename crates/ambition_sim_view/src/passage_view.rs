//! What the passages through doors and gates look like this tick
//! (`rooms::passage`): presentation reads these, never the passage.
//!
//! - A body in a passage draws the row its passage names, at the size, opacity
//!   and shade its passage gives ([`PassageLookView`] beside its
//!   `BodyPoseView`).
//! - A door opens and shuts with the passage through it, and a gate ripples
//!   ([`PassageViews`]).

use bevy::prelude::*;

use ambition_platformer2d_session::rooms::passage::{PassagePose, ZonePassage};
use ambition_sprite_sheet::character::CharacterAnim;

use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};

use crate::{ActorAnimIndex, BodyPoseView, ClipRequest};

/// How a body in a passage is drawn: about its feet, `scale` of its size,
/// `alpha` of it showing, `shade` of the way to black, `transit` of the way
/// into a gate's light (a cue the body overlay may draw).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PassageLookView {
    pub scale: f32,
    pub alpha: f32,
    pub shade: f32,
    pub transit: f32,
}

/// The doors and gates passages are going through this tick.
///
/// Each by its live room and zone id: a zone id is unique in its room only
/// (seven sandbox rooms author a `return_door`), and two rooms can be live.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PassageViews {
    /// A door's room and zone, and how open it is (0 shut, 1 open).
    pub doors: Vec<(Option<LiveRoomInstance>, String, f32)>,
    /// A gate's room and zone, the ripple row it plays (`swallow`, `emit`)
    /// and how far through it.
    pub gates: Vec<(Option<LiveRoomInstance>, String, &'static str, f32)>,
}

impl PassageViews {
    /// How open the door of `zone` in live room `room` is, when a passage
    /// goes through it.
    pub fn door_open(&self, room: Option<LiveRoomInstance>, zone: &str) -> Option<f32> {
        self.doors
            .iter()
            .filter(|(at, id, _)| *at == room && id == zone)
            .map(|(.., open)| *open)
            .reduce(f32::max)
    }

    /// The ripple the gate of `zone` in live room `room` plays, when a
    /// passage goes through it.
    pub fn gate_ripple(&self, room: Option<LiveRoomInstance>, zone: &str) -> Option<(&'static str, f32)> {
        self.gates
            .iter()
            .find(|(at, id, ..)| *at == room && id == zone)
            .map(|(.., row, k)| (*row, *k))
    }
}

fn row_of(pose: PassagePose) -> (&'static [&'static str], CharacterAnim) {
    match pose {
        PassagePose::TurnAway => (&["turn_away", "climb"], CharacterAnim::LadderClimb),
        PassagePose::WalkAway => (&["walk_away", "climb"], CharacterAnim::LadderClimb),
        PassagePose::BackTo => (&["climb"], CharacterAnim::LadderClimb),
        PassagePose::Walk => (&["walk"], CharacterAnim::Walk),
        PassagePose::Jump => (&["jump"], CharacterAnim::Jump),
        PassagePose::Fall => (&["fall"], CharacterAnim::Fall),
    }
}

/// Publish each passage's look, after the pose views are rebuilt.
///
/// A player body takes it beside its `BodyPoseView`. A body drawn on the
/// actor road (a possessed body, an NPC) takes it in its `ActorAnimIndex`
/// frame, which is rebuilt each tick, so it needs no removal.
pub fn rebuild_passage_views(
    mut commands: Commands,
    mut views: ResMut<PassageViews>,
    mut actors: Option<ResMut<ActorAnimIndex>>,
    mut bodies: Query<(
        Entity,
        &ZonePassage,
        Option<&mut BodyPoseView>,
        Option<&PassageLookView>,
        Option<&ambition_combat::components::FeatureId>,
    )>,
    stale: Query<Entity, (With<PassageLookView>, Without<ZonePassage>)>,
    rooms: LiveRooms,
) {
    let mut next = PassageViews::default();
    for (entity, passage, pose, shown, feature) in &mut bodies {
        // The door or gate is in the room the body is in: the room it leaves
        // while it goes in, the room it arrived in while it comes out.
        let room = rooms.of(entity);
        if let Some((zone, open)) = passage.door_open() {
            next.doors.push((room, zone.to_string(), open));
        }
        if let Some((zone, row, k)) = passage.gate_ripple() {
            next.gates.push((room, zone.to_string(), row, k));
        }
        match passage.look() {
            Some(look) => {
                if let Some(mut pose) = pose {
                    let (chain, anim) = row_of(look.pose);
                    pose.anim = anim;
                    pose.clip = ClipRequest::from_chain(chain);
                }
                let view = PassageLookView { scale: look.scale, alpha: look.alpha, shade: look.shade, transit: look.transit };
                let actor = feature.zip(actors.as_deref_mut()).and_then(|(id, actors)| actors.frame_mut(id.as_str()));
                if let Some(frame) = actor {
                    let (chain, anim) = row_of(look.pose);
                    frame.anim = anim;
                    frame.clip = ClipRequest::from_chain(chain);
                    frame.passage_look = Some(view);
                }
                if shown != Some(&view) {
                    commands.entity(entity).try_insert(view);
                }
            }
            None => {
                if shown.is_some() {
                    commands.entity(entity).try_remove::<PassageLookView>();
                }
            }
        }
    }
    for entity in &stale {
        commands.entity(entity).try_remove::<PassageLookView>();
    }
    if *views != next {
        *views = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_session::rooms::passage::{PassageKind, PassageStage};
    use ambition_platformer2d_core as ae;

    fn idle() -> crate::ActorAnimFrame {
        crate::ActorAnimFrame {
            anim: CharacterAnim::Idle,
            conversation_held: false,
            pos: ae::Vec2::ZERO,
            facing: 1.0,
            gravity_dir: ae::DEFAULT_GRAVITY_DIR,
            squash: ae::Vec2::ONE,
            air_stretch: 1.0,
            clip: None,
            move_charge: None,
            passage_look: None,
        }
    }

    /// Review finding #9 (2026-10-19). Two live rooms each have a door
    /// `return_door`, and a body goes through the one in the first room:
    /// that door opens and the other does not. Measured before: the door was
    /// named by its zone id alone, so both opened. Poison: drop the room
    /// from the key.
    #[test]
    fn a_passage_opens_the_door_of_its_own_room_only() {
        use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
        let mut app = App::new();
        app.init_resource::<PassageViews>();
        app.add_systems(Update, rebuild_passage_views);
        let (here, there) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        let passage = ZonePassage {
            kind: PassageKind::Door,
            stage: PassageStage::Entering,
            elapsed: 0.3,
            from: ae::Vec2::new(90.0, 177.0),
            zone: "return_door".into(),
            foot: ae::Vec2::new(100.0, 200.0),
            heart: ae::Vec2::new(100.0, 104.0),
            half_h: 23.0,
            facing: 1.0,
            crossing: None,
            recorded: false,
            recorded_at: 0.0,
            arrival: None,
            room: None,
        };
        app.world_mut().spawn((passage, InRoomInstance(here)));
        app.update();
        let views = app.world().resource::<PassageViews>();
        assert!(views.door_open(Some(here), "return_door").is_some_and(|open| open > 0.0), "premise: its door opens");
        assert_eq!(views.door_open(Some(there), "return_door"), None, "the door of the same name in the other room opened");
    }

    /// Review finding #3 (2026-10-19). A body drawn on the actor road (a
    /// possessed body, an NPC) that goes through a door draws its passage's
    /// row and look, as a player body does. Measured before: its actor frame
    /// kept the row the picker chose and carried no look. Control: a body
    /// that is not in a passage keeps its frame. Poison: do not write the
    /// actor frame.
    #[test]
    fn a_body_on_the_actor_road_draws_its_passage() {
        let mut app = App::new();
        app.init_resource::<PassageViews>();
        app.insert_resource(ActorAnimIndex::from_rows([
            ("npc".to_string(), idle()),
            ("bystander".to_string(), idle()),
        ]));
        app.add_systems(Update, rebuild_passage_views);
        let passage = ZonePassage {
            kind: PassageKind::Door,
            stage: PassageStage::Entering,
            elapsed: 0.01,
            from: ae::Vec2::new(90.0, 177.0),
            zone: "a door".into(),
            foot: ae::Vec2::new(100.0, 200.0),
            heart: ae::Vec2::new(100.0, 104.0),
            half_h: 23.0,
            facing: 1.0,
            crossing: None,
            recorded: false,
            recorded_at: 0.0,
            arrival: None,
            room: None,
        };
        let look = passage.look().expect("premise: the passage holds the body and has a look");
        app.world_mut().spawn((passage, ambition_combat::components::FeatureId::new("npc")));
        app.world_mut().spawn(ambition_combat::components::FeatureId::new("bystander"));
        app.update();

        let index = app.world().resource::<ActorAnimIndex>();
        let frame = index.get("npc").expect("the npc's frame");
        assert_eq!(
            (frame.anim, frame.clip.as_ref().map(|clip| clip.clip.as_str()), frame.passage_look),
            (
                CharacterAnim::LadderClimb,
                Some("turn_away"),
                Some(PassageLookView { scale: look.scale, alpha: look.alpha, shade: look.shade, transit: look.transit })
            ),
            "the actor frame draws the passage's row and look"
        );
        let bystander = index.get("bystander").expect("the bystander's frame");
        assert_eq!(
            (bystander.anim, bystander.clip.is_none(), bystander.passage_look),
            (CharacterAnim::Idle, true, None),
            "control: a body outside a passage keeps its frame"
        );
    }
}
