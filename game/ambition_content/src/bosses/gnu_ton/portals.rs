//! The look lines of GNU-ton's fight, and the trail of each fist.
//!
//! The pair of the punch (phase 1) is a REAL pair of portals, and it is not
//! here: his conductor (the module
//! `ambition_content_modules::gnu_ton::conductor`) asks the portal domain for
//! it each tick it is open (`ambition.portal.held_pair`), and
//! `ambition_portal2d::extension` keeps it.
//!
//! The line at each shoulder and behind each wrist (after phase 1) is a LOOK:
//! a `PortalLightLine`, which the simulation does not know. The arm is in it.
//! It is not a portal with no partner: a free portal that moves pushes each
//! body on its plane to one side, and these lines move with the fists and are
//! on the giant. The conductor says in its record where each one is; this
//! draws it. The record holds the clock, so this adds no rollback state.
//!
//! The engine draws each line (`ambition_portal2d_presentation`). No sprite
//! has a portal in it.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_content_modules::gnu_ton::conductor::Conductor;
use ambition_platformer2d::platformer::lifecycle::{LiveRoomInstance, LiveRooms};
#[cfg(feature = "portal_render")]
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, SessionCommands, SpawnSessionScopedExt};
#[cfg(feature = "portal_render")]
use ambition_platformer2d_core as ae;
#[cfg(feature = "portal_render")]
use ambition_portal2d::{PortalChannel, PortalChannelColor};

/// Which look line of the fight a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GnuTonLine {
    /// The line an arm is in, on the side of the left fist (0) or the right (1).
    Shoulder(u8),
    /// The line behind the wrist of the left fist (0) or the right (1).
    Wrist(u8),
}

#[cfg(feature = "portal_render")]
impl GnuTonLine {
    /// The look lines, in the order of the record's `lines`.
    const LOOKS: [GnuTonLine; 4] =
        [GnuTonLine::Shoulder(0), GnuTonLine::Shoulder(1), GnuTonLine::Wrist(0), GnuTonLine::Wrist(1)];

    /// The channel of its colour. The ends of the punch are blue and orange
    /// (24 and 25, the conductor's `hold_punch_pair`). A look line has the
    /// colour of the end it looks like: a shoulder's is blue, a wrist's is
    /// orange.
    fn channel(self) -> PortalChannel {
        let index = match self {
            GnuTonLine::Shoulder(_) => 24,
            GnuTonLine::Wrist(_) => 25,
        };
        PortalChannel::Authored(PortalChannelColor::Indexed(index))
    }

    /// The side of the line that the room is on.
    fn normal(self) -> ae::Vec2 {
        match self {
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
            GnuTonLine::Shoulder(0) => "shoulder_left",
            GnuTonLine::Shoulder(_) => "shoulder_right",
            GnuTonLine::Wrist(0) => "wrist_left",
            GnuTonLine::Wrist(_) => "wrist_right",
        }
    }
}

/// A look line of GNU-ton's fight: this system draws it and takes it away.
/// `owner` is the placement id of the scholar whose conductor places it: two
/// GNU-tons in one room each have their own lines.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct GnuTonLookLine {
    pub line: GnuTonLine,
    pub owner: String,
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
#[cfg(feature = "portal_render")]
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

/// Keep a line of light at each shoulder and behind each wrist while a living
/// GNU-ton's conductor says the giant's arms are in their lines.
#[cfg(feature = "portal_render")]
pub fn light_gnu_ton_lines(
    mut commands: SessionCommands,
    live: LiveRooms,
    scholars: Scholars,
    mut lines: Query<(Entity, &GnuTonLookLine, &mut ambition_portal2d_presentation::PortalLightLine)>,
) {
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    // Each line: its room, which it is, its middle, its length. The length
    // changes: the lines open wide and close on the arms (the clamp).
    let mut wanted: Vec<(LiveRoomInstance, GnuTonLookLine, ae::Vec2, f32)> = Vec::new();
    for (owner, room, conductor) in conductors(&scholars, &live) {
        for (line, placed) in GnuTonLine::LOOKS.into_iter().zip(conductor.look_lines()) {
            if let Some((at, half)) = placed {
                wanted.push((room, GnuTonLookLine { line, owner: owner.clone() }, ae::Vec2::new(at.x, at.y), half * 2.0));
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
/// its arm has its trail from the giant, where its body is: no start of its
/// own. The start is on the fist (`LimbBondStart`), so each fist has its
/// own, and a fist of a scholar who is dead has none.
pub fn join_gnu_ton_lines(
    mut commands: Commands,
    live: LiveRooms,
    scholars: Scholars,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    starts: Query<&ambition_characters::actor::LimbBondStart>,
) {
    use ambition_characters::actor::{LimbBondStart, LimbSlot};
    let living = conductors_of(&scholars, &live);
    for (scholar, boss, ..) in &scholars {
        if !super::boss_is_gnu_ton(&boss.as_boss_ref()) {
            continue;
        }
        let Some(rig) = riders.get(scholar).ok().and_then(|riding| rigs.get(riding.mount).ok()) else {
            continue;
        };
        let conductor = living.iter().find(|(s, ..)| *s == scholar).map(|(.., conductor)| conductor);
        for (index, slot) in [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].into_iter().enumerate() {
            let Some(fist) = rig.get(slot) else {
                continue;
            };
            let want = conductor.and_then(|conductor| {
                // The right fist is the one that goes through the pair of the punch.
                let through = conductor.punch_pair().filter(|_| index == 1).map(|[near, _]| near);
                conductor.look_lines()[index].map(|(at, _)| at).or(through)
            });
            let now = starts.get(fist).ok().map(|start| start.0);
            match want.map(|at| Vec2::new(at.x, at.y)) {
                Some(at) if now != Some(at) => {
                    commands.entity(fist).try_insert(LimbBondStart(at));
                }
                None if now.is_some() => {
                    commands.entity(fist).try_remove::<LimbBondStart>();
                }
                _ => {}
            }
        }
    }
}
