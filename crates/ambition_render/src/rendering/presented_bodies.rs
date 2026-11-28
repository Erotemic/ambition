//! Borrowed body facts for effects that apply to any controller kind.

use ambition_platformer2d_core::{self as ae, RoomGeometry};
use ambition_platformer2d_shared_tangle::{
    feature_kind::FeatureVisualKind,
    lifecycle::{LiveRoomInstance, LiveRoomOf, PlayerVisual, SessionScopeId, SessionScopedEntity},
    sim_id::SimId,
};
use ambition_sim_view::{BodyPoseView, FeatureViewIndex, PresentedFeaturePoses, PresentedPose};
use bevy::{ecs::system::SystemParam, prelude::*};

/// Stable identity of a body representation in one live room.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresentedBodyKey {
    pub session: Option<SessionScopeId>,
    pub room: Option<LiveRoomInstance>,
    pub subject: SimId,
}

enum Subject<'a> {
    Body(&'a SimId),
    Feature(&'a str),
}

/// One body representation. Entity handles name this frame's visual only.
pub struct PresentedBody<'a> {
    subject: Subject<'a>,
    session: Option<SessionScopeId>,
    pub room: Option<LiveRoomInstance>,
    pub visual: Entity,
    pub geometry: Option<&'a ae::World>,
    pub pos: Vec2,
    pub size: Vec2,
    pub wire_anchor: Option<Vec2>,
    pub grab_reach: Option<Vec2>,
    pub line_anchor: Option<Vec2>,
    pub submerged: bool,
    pub visible: bool,
    pub depth_plane: Option<ae::DepthPlane>,
}

impl PresentedBody<'_> {
    pub fn key(&self) -> PresentedBodyKey {
        PresentedBodyKey {
            session: self.session,
            room: self.room,
            subject: match self.subject {
                Subject::Body(id) => id.clone(),
                Subject::Feature(id) => SimId::placement(id),
            },
        }
    }
}

/// Project existing read models without a second state cache.
///
/// This adapter lives beside `FeatureVisual`, which belongs to presentation.
/// Effects do not need to select the source of a body's facts.
#[derive(SystemParam)]
pub struct PresentedBodies<'w, 's> {
    poses: Query<
        'w,
        's,
        (
            Entity,
            &'static BodyPoseView,
            Option<&'static PresentedPose>,
            &'static SimId,
            Option<&'static SessionScopedEntity>,
        ),
        With<PlayerVisual>,
    >,
    features: Query<
        'w,
        's,
        (
            Entity,
            &'static super::FeatureVisual,
            Option<&'static SessionScopedEntity>,
        ),
        Without<PlayerVisual>,
    >,
    views: Option<Res<'w, FeatureViewIndex>>,
    presented_features: Option<Res<'w, PresentedFeaturePoses>>,
    rooms: LiveRoomOf<'w, 's, RoomGeometry>,
}

impl PresentedBodies<'_, '_> {
    pub fn iter(&self) -> impl Iterator<Item = PresentedBody<'_>> {
        let poses = self
            .poses
            .iter()
            .map(|(visual, pose, presented, id, session)| PresentedBody {
                subject: Subject::Body(id),
                session: session.map(|scope| scope.0),
                room: self.rooms.room_of(visual),
                visual,
                geometry: self.rooms.of(visual).map(|room| &room.0),
                pos: ambition_sim_view::presented_pose::draw_pos(pose, presented),
                size: Vec2::new(pose.size.x, pose.size.y),
                wire_anchor: pose.wire_anchor.map(|p| Vec2::new(p.x, p.y)),
                grab_reach: pose.grab_reach.map(|p| Vec2::new(p.x, p.y)),
                line_anchor: pose.line_anchor.map(|p| Vec2::new(p.x, p.y)),
                submerged: pose.submerged,
                visible: !super::body_visibility::body_presentation_hidden(pose),
                depth_plane: None,
            });
        let features = self
            .features
            .iter()
            .filter_map(|(visual, feature, session)| {
                let views = self.views.as_ref()?;
                let room = self.rooms.room_of(visual);
                let view = views.get_in(&feature.id, room)?;
                if view.kind != FeatureVisualKind::Actor {
                    return None;
                }
                let pos = self.presented_features.as_deref()
                    .map_or(view.pos, |poses| poses.presented(&feature.id, room, view.pos));
                Some(PresentedBody {
                    subject: Subject::Feature(&feature.id),
                    session: session.map(|scope| scope.0),
                    room,
                    visual,
                    geometry: self.rooms.of(visual).map(|room| &room.0),
                    pos,
                    size: Vec2::new(view.size.x, view.size.y),
                    wire_anchor: view.wire_anchor.map(|p| Vec2::new(p.x, p.y)),
                    grab_reach: view.grab_reach.map(|p| Vec2::new(p.x, p.y)),
                    line_anchor: view.line_anchor.map(|p| Vec2::new(p.x, p.y)),
                    submerged: view.submerged,
                    visible: view.visible,
                    depth_plane: Some(view.depth_plane),
                })
            });
        poses.chain(features)
    }
}
