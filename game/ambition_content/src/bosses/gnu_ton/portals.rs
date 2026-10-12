//! The portals of GNU-ton's fight.
//!
//! His conductor (the module `ambition_content_modules::gnu_ton::conductor`)
//! says in its record where each line is. This opens each one, moves it,
//! and closes it. A module has no port that opens a portal, and the record
//! holds the clock, so this adds no rollback state.
//!
//! * The pair of the punch (phase 1) is a REAL pair: two [`PlacedPortal`]s
//!   of one [`PortalLink`] of their scholar's own. The portal core gives the
//!   link a channel pair of the room, so two GNU-tons in one room have two
//!   pairs. The blue and orange of the fight are a [`PortalLook`]. The giant's fist goes down into the near line; the
//!   drawing of the fist comes down out of the far line. A body that goes up
//!   into the far line comes up out of the near line, by the giant's
//!   shoulder.
//! * The line at each shoulder and behind each wrist (after phase 1) is a
//!   LOOK: a `PortalLightLine`, which the simulation does not know. The arm
//!   is in it. It is not a portal with no partner: a free portal that moves
//!   pushes each body on its plane to one side, and these lines move with
//!   the fists and are on the giant.
//!
//! The engine draws each line (`ambition_portal2d_presentation`). No sprite
//! has a portal in it.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_content_modules::gnu_ton::choreography::arms;
use ambition_content_modules::gnu_ton::conductor::Conductor;
use ambition_platformer2d::platformer::lifecycle::{
    InRoomInstance, LiveRoomInstance, LiveRooms, SessionCommands, SpawnSessionScopedExt,
};
use ambition_portal2d::{
    link_hash, portal_half_extent_with_length, PlacedPortal, PortalChannel, PortalChannelColor, PortalLink, PortalLook,
    UNRESOLVED_LINK_CHANNEL,
};

/// Which line of the fight a portal is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GnuTonLine {
    /// The punch's line by the giant's shoulder: the fist goes down into it.
    PunchNear,
    /// The punch's line over the player: the fist comes down out of it.
    PunchFar,
    /// The line an arm is in, on the side of the left fist (0) or the right (1).
    Shoulder(u8),
    /// The line behind the wrist of the left fist (0) or the right (1).
    Wrist(u8),
}

impl GnuTonLine {
    /// The look lines, in the order of the record's `lines`.
    #[cfg(feature = "portal_render")]
    const LOOKS: [GnuTonLine; 4] =
        [GnuTonLine::Shoulder(0), GnuTonLine::Shoulder(1), GnuTonLine::Wrist(0), GnuTonLine::Wrist(1)];

    /// The channel of its colour. The ends of the punch are blue and orange
    /// (24 and 25). A look line has the colour of the end it looks like: a
    /// shoulder's is blue, a wrist's is orange. It is a look: the pair is
    /// paired by its link.
    fn channel(self) -> PortalChannel {
        let index = match self {
            GnuTonLine::PunchNear | GnuTonLine::Shoulder(_) => 24,
            GnuTonLine::PunchFar | GnuTonLine::Wrist(_) => 25,
        };
        PortalChannel::Authored(PortalChannelColor::Indexed(index))
    }

    /// The side of the line that the room is on.
    fn normal(self) -> ae::Vec2 {
        match self {
            // The fist goes down into it; a body comes up out of it.
            GnuTonLine::PunchNear => ae::Vec2::new(0.0, -1.0),
            // The fist comes down out of it; a body goes up into it.
            GnuTonLine::PunchFar => ae::Vec2::new(0.0, 1.0),
            // The giant is between its two shoulders' lines.
            GnuTonLine::Shoulder(0) => ae::Vec2::new(1.0, 0.0),
            GnuTonLine::Shoulder(_) => ae::Vec2::new(-1.0, 0.0),
            // A fist is out from its wrist's line, away from the giant.
            GnuTonLine::Wrist(0) => ae::Vec2::new(-1.0, 0.0),
            GnuTonLine::Wrist(_) => ae::Vec2::new(1.0, 0.0),
        }
    }

    fn name(self) -> &'static str {
        match self {
            GnuTonLine::PunchNear => "punch_near",
            GnuTonLine::PunchFar => "punch_far",
            GnuTonLine::Shoulder(0) => "shoulder_left",
            GnuTonLine::Shoulder(_) => "shoulder_right",
            GnuTonLine::Wrist(0) => "wrist_left",
            GnuTonLine::Wrist(_) => "wrist_right",
        }
    }
}

/// A portal (or a look line) of GNU-ton's fight: this system opened it and
/// closes it. `owner` is the placement id of the scholar whose conductor
/// asks for it: two GNU-tons in one room each have their own lines.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct GnuTonPortal {
    pub line: GnuTonLine,
    pub owner: String,
}

impl GnuTonPortal {
    /// Its simulation identity: its owner's line. Two live instances of one
    /// room give one placement's line one id, as they give its body one.
    fn sim_id(&self) -> ambition_platformer2d::platformer::sim_id::SimId {
        ambition_platformer2d::platformer::sim_id::SimId::singleton(
            "gnu_ton_portal",
            &format!("{}/{}", self.owner, self.line.name()),
        )
    }

    /// The link of its owner's pair of the punch.
    fn punch_link(owner: &str) -> PortalLink {
        PortalLink(link_hash(&format!("gnu_ton/{owner}/punch")))
    }
}

/// A living GNU-ton's scholar: his body, his encounter, his conductor's
/// record.
type Scholars<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        BossClusterRef,
        &'static ambition_extension_host::BodyRecords,
        &'static ambition_characters::actor::BodyHealth,
    ),
>;

/// The conductor's record of each living GNU-ton, with his placement id and
/// his live room.
fn conductors(scholars: &Scholars, live: &LiveRooms) -> Vec<(String, LiveRoomInstance, Conductor)> {
    conductors_of(scholars, live).into_iter().map(|(_, owner, room, conductor)| (owner, room, conductor)).collect()
}

/// The same, with the scholar.
fn conductors_of(scholars: &Scholars, live: &LiveRooms) -> Vec<(Entity, String, LiveRoomInstance, Conductor)> {
    scholars
        .iter()
        .filter(|(_, boss, _, health)| super::boss_is_gnu_ton(&boss.as_boss_ref()) && health.alive())
        .filter_map(|(scholar, boss, records, _)| {
            let conductor = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
            Some((scholar, boss.as_boss_ref().config.id.clone(), live.of(scholar)?, conductor))
        })
        .collect()
}

/// Keep the pair of the punch open while a living GNU-ton's conductor says
/// it is, in his live room.
pub fn open_gnu_ton_portals(
    mut commands: SessionCommands,
    live: LiveRooms,
    scholars: Scholars,
    mut portals: Query<(Entity, &GnuTonPortal, &mut PlacedPortal)>,
) {
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    let mut wanted: Vec<(LiveRoomInstance, GnuTonPortal, ae::Vec2)> = Vec::new();
    for (owner, room, conductor) in conductors(&scholars, &live) {
        if let Some([near, far]) = conductor.punch_pair() {
            for (line, at) in [(GnuTonLine::PunchNear, near), (GnuTonLine::PunchFar, far)] {
                wanted.push((room, GnuTonPortal { line, owner: owner.clone() }, ae::Vec2::new(at.x, at.y)));
            }
        }
    }
    for (entity, tag, mut portal) in &mut portals {
        let room = live.of(entity);
        match wanted.iter().position(|(r, want, _)| Some(*r) == room && want == tag) {
            Some(index) => {
                let (_, _, at) = wanted.swap_remove(index);
                if portal.pos != at {
                    portal.pos = at;
                    portal.prev_pos = at;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (room, tag, at) in wanted {
        let line = tag.line;
        let normal = line.normal();
        commands
            .spawn_room_in_session(
                scope,
                (
                    PlacedPortal::fixed(
                        UNRESOLVED_LINK_CHANNEL,
                        at,
                        normal,
                        portal_half_extent_with_length(normal, arms::PUNCH_HALF),
                    ),
                    GnuTonPortal::punch_link(&tag.owner),
                    PortalLook(line.channel()),
                    Name::new(format!("GNU-ton portal: {} of {}", line.name(), tag.owner)),
                    tag.sim_id(),
                    tag,
                ),
            )
            .insert(InRoomInstance(room));
    }
}

/// Keep a line of light at each shoulder and behind each wrist while a living
/// GNU-ton's conductor says the giant's arms are in their lines.
#[cfg(feature = "portal_render")]
pub fn light_gnu_ton_lines(
    mut commands: SessionCommands,
    live: LiveRooms,
    scholars: Scholars,
    mut lines: Query<(Entity, &GnuTonPortal, &mut ambition_portal2d_presentation::PortalLightLine)>,
) {
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    // Each line: its room, which it is, its middle, its length. The length
    // changes: the lines open wide and close on the arms (the clamp).
    let mut wanted: Vec<(LiveRoomInstance, GnuTonPortal, ae::Vec2, f32)> = Vec::new();
    for (owner, room, conductor) in conductors(&scholars, &live) {
        for (line, placed) in GnuTonLine::LOOKS.into_iter().zip(conductor.look_lines()) {
            if let Some((at, half)) = placed {
                wanted.push((room, GnuTonPortal { line, owner: owner.clone() }, ae::Vec2::new(at.x, at.y), half * 2.0));
            }
        }
    }
    for (entity, tag, mut light) in &mut lines {
        let room = live.of(entity);
        match wanted.iter().position(|(r, want, ..)| Some(*r) == room && want == tag) {
            Some(index) => {
                let (_, _, at, length) = wanted.swap_remove(index);
                if light.pos != at || light.length != length {
                    light.pos = at;
                    light.length = length;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (room, tag, at, length) in wanted {
        let line = tag.line;
        commands
            .spawn_room_in_session(
                scope,
                (
                    ambition_portal2d_presentation::PortalLightLine {
                        channel: line.channel(),
                        pos: at,
                        normal: line.normal(),
                        length,
                    },
                    Name::new(format!("GNU-ton light line: {} of {}", line.name(), tag.owner)),
                    tag,
                ),
            )
            .insert(InRoomInstance(room));
    }
}

/// The trail of each fist is part of his spell: it joins the two lines the
/// arm is in. It starts at the shoulder's line (after phase 1), or at the
/// near line of the punch for the fist that is through the pair. A fist on
/// its arm has its trail from the giant, where its body is: no length.
pub fn join_gnu_ton_lines(
    live: LiveRooms,
    scholars: Scholars,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    ids: Query<&ambition_combat::components::FeatureId>,
    starts: Option<ResMut<ambition_render::rendering::limb_trail::LimbTrailStarts>>,
) {
    use ambition_characters::actor::LimbSlot;
    let Some(mut starts) = starts else {
        return;
    };
    let mut wanted = bevy::platform::collections::HashMap::new();
    for (scholar, _, _, conductor) in conductors_of(&scholars, &live) {
        let Some(rig) = riders.get(scholar).ok().and_then(|riding| rigs.get(riding.mount).ok()) else {
            continue;
        };
        let lines = conductor.look_lines();
        for (index, slot) in [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].into_iter().enumerate() {
            let Some(id) = rig.get(slot).and_then(|fist| ids.get(fist).ok()) else {
                continue;
            };
            // The right fist is the one that goes through the pair of the punch.
            let through = conductor.punch_pair().filter(|_| index == 1).map(|[near, _]| near);
            if let Some(from) = lines[index].map(|(at, _)| at).or(through) {
                wanted.insert(id.as_str().to_string(), Vec2::new(from.x, from.y));
            }
        }
    }
    if starts.0 != wanted {
        starts.0 = wanted;
    }
}
