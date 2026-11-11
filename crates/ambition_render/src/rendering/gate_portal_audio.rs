//! Gate-portal sound: a wormhole is heard opening, humming while it is open,
//! and taking a body in and giving one out.
//!
//! Presentation, like `gate_portal_visuals`: it observes the sim's
//! [`GatePortalPhases`] and the passages' [`PassageViews`], for each gate whose
//! membrane is drawn in a live room, and emits on the edges it sees. A gate
//! is heard only where it is drawn, so a gate that is already open when its
//! room becomes live hums without the opening cue, and a gate whose room
//! unloads stops humming.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::primitives::PropVisual;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_world::rooms::{
    GatePortalPhase, GatePortalPhases, GatePortalRegistry, PORTAL_CLOSING_DURATION_SECS,
};
use ambition_sfx::{ids, SfxEmissionContext, SfxLoopKey, SfxLoopWriter, SfxMessage, SfxWriter};
use ambition_sim_view::PassageViews;

/// The hum swells in under the opening cue's bloom and tail.
pub const WORMHOLE_HUM_FADE_IN_SECS: f32 = 1.2;
/// The hum fades with the membrane's closing row.
pub const WORMHOLE_HUM_FADE_OUT_SECS: f32 = PORTAL_CLOSING_DURATION_SECS;

/// A drawn gate: the live room its membrane is in, and its zone.
type Gate = (Option<LiveRoomInstance>, String);

/// What the gates sounded like on the last run.
#[derive(Default)]
pub struct GateAudioMemory {
    /// The audio context the memory belongs to; a new one starts it afresh.
    owner: Option<ambition_sfx::AudioContextOwner>,
    /// Each drawn gate's phase as last heard.
    phases: HashMap<Gate, GatePortalPhase>,
    /// Gates whose hum is held.
    humming: HashSet<Gate>,
    /// How many bodies each gate was taking in (`swallow`) or giving out (`emit`).
    ripples: HashMap<(Gate, &'static str), usize>,
}

/// The loop key of a gate's hum: one hum per drawn gate.
pub fn wormhole_hum_key(gate: &Gate) -> SfxLoopKey {
    SfxLoopKey(ambition_sfx::fnv1a_64_str(&format!("world.wormhole.hum/{:?}/{}", gate.0, gate.1)))
}

fn hums(phase: GatePortalPhase) -> bool {
    matches!(phase, GatePortalPhase::Opening { .. } | GatePortalPhase::On)
}

/// Play the wormhole cues from the gates' phases and passages.
///
/// - `world.wormhole.open` when a drawn gate's phase enters `Opening`;
/// - `world.wormhole.hum` held while it is `Opening` or `On`, stopped with a
///   fade when it closes or its membrane is no longer drawn;
/// - `world.wormhole.enter` / `exit` when a body starts its `swallow` / `emit`
///   ripple through the membrane.
///
/// The source region uses simulation world coordinates.
#[allow(clippy::too_many_arguments)]
pub fn sync_gate_portal_audio(
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    passages: Option<Res<PassageViews>>,
    membranes: Query<(Entity, &PropVisual, &GlobalTransform)>,
    rooms: LiveRooms,
    geometry: Query<(Entity, &ambition_platformer2d_core::RoomGeometry)>,
    context: Option<Res<SfxEmissionContext>>,
    mut sfx: Option<SfxWriter>,
    mut loops: Option<SfxLoopWriter>,
    mut memory: Local<GateAudioMemory>,
) {
    let owner = context.as_deref().and_then(SfxEmissionContext::owner);
    if memory.owner != owner {
        *memory = GateAudioMemory { owner, ..default() };
    }

    let mut drawn: HashMap<Gate, (GatePortalPhase, ambition_sfx::SfxEmitter)> = HashMap::new();
    for (zone, config) in portals.iter() {
        for (entity, prop, transform) in &membranes {
            if prop.name == config.portal_sprite_name {
                let room = rooms.of(entity);
                let Some((_, geometry)) = geometry.iter().find(|(root, _)| rooms.of(*root) == room) else { continue };
                let at = ambition_platformer2d_core::config::bevy_size_to_world(geometry.0.size, transform.translation().truncate());
                let emitter = ambition_sfx::SfxEmitter::Spatial {
                    shape: ambition_sfx::SfxShape::Circle { center: at, radius: prop.size.min_element() * 0.5 },
                    falloff: ambition_sfx::SfxFalloff::new(0.0, 1200.0),
                    space: ambition_vfx::sfx_space(room),
                };
                drawn.insert((room, zone.clone()), (phases.phase(zone), emitter));
            }
        }
    }

    for (gate, (phase, emitter)) in &drawn {
        let opened = matches!(phase, GatePortalPhase::Opening { .. })
            && memory.phases.get(gate).is_some_and(|before| !matches!(before, GatePortalPhase::Opening { .. }));
        if opened {
            if let Some(sfx) = sfx.as_mut() {
                sfx.write(SfxMessage::Play { id: ids::WORLD_WORMHOLE_OPEN, emitter: *emitter });
            }
        }
        if hums(*phase) {
            if let Some(loops) = loops.as_mut() {
                loops.hold(wormhole_hum_key(gate), ids::WORLD_WORMHOLE_HUM, *emitter, WORMHOLE_HUM_FADE_IN_SECS);
            }
            memory.humming.insert(gate.clone());
        }
    }
    let quiet: Vec<Gate> = memory
        .humming
        .iter()
        .filter(|gate| !drawn.get(*gate).is_some_and(|(phase, _)| hums(*phase)))
        .cloned()
        .collect();
    for gate in quiet {
        if let Some(loops) = loops.as_mut() {
            loops.stop(wormhole_hum_key(&gate), WORMHOLE_HUM_FADE_OUT_SECS);
        }
        memory.humming.remove(&gate);
    }
    memory.phases = drawn.iter().map(|(gate, (phase, _))| (gate.clone(), *phase)).collect();

    let mut ripples: HashMap<(Gate, &'static str), usize> = HashMap::new();
    for (room, zone, row, _) in passages.as_deref().map(|views| views.gates.as_slice()).unwrap_or_default() {
        *ripples.entry(((*room, zone.clone()), *row)).or_default() += 1;
    }
    for ((gate, row), count) in &ripples {
        let before = memory.ripples.get(&(gate.clone(), *row)).copied().unwrap_or(0);
        let id = match *row {
            "swallow" => ids::WORLD_WORMHOLE_ENTER,
            "emit" => ids::WORLD_WORMHOLE_EXIT,
            _ => continue,
        };
        if *count > before {
            let Some((_, emitter)) = drawn.get(gate) else { continue };
            if let Some(sfx) = sfx.as_mut() {
                for _ in before..*count {
                    sfx.write(SfxMessage::Play { id, emitter: *emitter });
                }
            }
        }
    }
    memory.ripples = ripples;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sfx::{AudioContextOwner, OwnedSfxLoopMessage, OwnedSfxMessage, SfxId, SfxLoopRequest};
    use bevy::ecs::message::Messages;

    const ZONE: &str = "gate_zone";
    const MEMBRANE: &str = "gate membrane";

    fn app() -> App {
        let mut app = App::new();
        let mut portals = GatePortalRegistry::default();
        portals.try_register(ZONE, "gate_switch", MEMBRANE, "gate ring").expect("registers");
        app.insert_resource(portals)
            .init_resource::<GatePortalPhases>()
            .init_resource::<PassageViews>()
            .add_message::<OwnedSfxMessage>()
            .add_message::<OwnedSfxLoopMessage>();
        let mut context = SfxEmissionContext::default();
        context.set(AudioContextOwner::Direct, "test.provider");
        app.insert_resource(context);
        app.world_mut().spawn(ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "audio fixture", Vec2::new(1000.0, 800.0), Vec2::ZERO, Vec::new())));
        app.add_systems(Update, sync_gate_portal_audio);
        app
    }

    fn draw_membrane(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                PropVisual {
                    id: MEMBRANE.into(),
                    kind: "interdimensional_gate_portal".into(),
                    name: MEMBRANE.into(),
                    size: Vec2::splat(288.0),
                    draw: Default::default(),
                    flip_y: false,
                },
                GlobalTransform::from_xyz(40.0, 12.0, 0.0),
            ))
            .id()
    }

    fn set_phase(app: &mut App, phase: GatePortalPhase) {
        *app.world_mut().resource_mut::<GatePortalPhases>().phase_mut(ZONE) = phase;
    }

    /// The cues played on the last update.
    fn played(app: &mut App) -> Vec<SfxId> {
        let mut messages = app.world_mut().resource_mut::<Messages<OwnedSfxMessage>>();
        let played = messages.drain().filter_map(|owned| match owned.request {
            SfxMessage::Play { id, .. } => Some(id),
            _ => None,
        });
        played.collect()
    }

    /// The hum requests made on the last update.
    fn hum(app: &mut App) -> Vec<SfxLoopRequest> {
        app.world_mut().resource_mut::<Messages<OwnedSfxLoopMessage>>().drain().map(|owned| owned.request).collect()
    }

    fn holds(requests: &[SfxLoopRequest]) -> bool {
        requests.iter().any(|request| matches!(request, SfxLoopRequest::Hold { id, .. } if *id == ids::WORLD_WORMHOLE_HUM))
    }

    fn stops(requests: &[SfxLoopRequest]) -> usize {
        requests.iter().filter(|request| matches!(request, SfxLoopRequest::Stop { .. })).count()
    }

    /// A shut gate flips on: the opening cue plays once, on the tick its phase
    /// enters `Opening`, and not again while it opens or stands open.
    #[test]
    fn the_open_cue_plays_when_a_drawn_gate_starts_opening() {
        let mut app = app();
        draw_membrane(&mut app);
        app.update();
        assert!(played(&mut app).is_empty(), "a shut gate is silent");

        set_phase(&mut app, GatePortalPhase::Opening { elapsed: 0.0 });
        app.update();
        assert_eq!(played(&mut app), vec![ids::WORLD_WORMHOLE_OPEN]);

        set_phase(&mut app, GatePortalPhase::Opening { elapsed: 0.3 });
        app.update();
        set_phase(&mut app, GatePortalPhase::On);
        app.update();
        assert!(played(&mut app).is_empty(), "the opening cue plays once per opening");
    }

    /// The hum is held from the opening through the open gate, and stopped
    /// once, with the closing row's fade, when the gate closes.
    #[test]
    fn the_hum_holds_while_the_gate_is_open_and_stops_when_it_closes() {
        let mut app = app();
        draw_membrane(&mut app);
        app.update();
        assert!(!holds(&hum(&mut app)), "a shut gate does not hum");

        set_phase(&mut app, GatePortalPhase::Opening { elapsed: 0.0 });
        app.update();
        assert!(holds(&hum(&mut app)), "the hum swells in with the opening");
        set_phase(&mut app, GatePortalPhase::On);
        app.update();
        assert!(holds(&hum(&mut app)), "the open gate keeps humming");

        set_phase(&mut app, GatePortalPhase::Closing { elapsed: 0.0 });
        app.update();
        let closing = hum(&mut app);
        assert!(!holds(&closing));
        assert_eq!(closing.iter().find_map(|r| match r {
            SfxLoopRequest::Stop { fade_out_secs, .. } => Some(*fade_out_secs),
            _ => None,
        }), Some(WORMHOLE_HUM_FADE_OUT_SECS));
        app.update();
        assert_eq!(stops(&hum(&mut app)), 0, "the hum is stopped once");
    }

    /// A gate whose room unloads (its membrane is no longer drawn) stops
    /// humming; a gate already open when its room becomes live hums without
    /// the opening cue.
    #[test]
    fn a_gate_is_heard_only_while_its_membrane_is_drawn() {
        let mut app = app();
        set_phase(&mut app, GatePortalPhase::On);
        let membrane = draw_membrane(&mut app);
        app.update();
        assert!(played(&mut app).is_empty(), "a gate found open does not play its opening");
        assert!(holds(&hum(&mut app)), "a gate found open hums");

        app.world_mut().despawn(membrane);
        app.update();
        assert_eq!(stops(&hum(&mut app)), 1, "the hum of a gate whose room unloaded stops");
    }

    /// A body going in plays the enter cue as its `swallow` ripple starts, and
    /// a body coming out plays the exit cue as its `emit` ripple starts; each
    /// once, not every tick of the ripple.
    #[test]
    fn enter_plays_on_swallow_and_exit_on_emit() {
        let mut app = app();
        draw_membrane(&mut app);
        set_phase(&mut app, GatePortalPhase::On);
        app.update();
        played(&mut app);

        let ripple = |app: &mut App, row: Option<&'static str>, k: f32| {
            app.world_mut().resource_mut::<PassageViews>().gates =
                row.map(|row| vec![(None, ZONE.to_string(), row, k)]).unwrap_or_default();
        };
        ripple(&mut app, Some("swallow"), 0.0);
        app.update();
        assert_eq!(played(&mut app), vec![ids::WORLD_WORMHOLE_ENTER]);
        ripple(&mut app, Some("swallow"), 0.5);
        app.update();
        assert!(played(&mut app).is_empty(), "one body, one enter");

        ripple(&mut app, None, 0.0);
        app.update();
        ripple(&mut app, Some("emit"), 0.0);
        app.update();
        assert_eq!(played(&mut app), vec![ids::WORLD_WORMHOLE_EXIT]);
    }
    #[test]
    fn the_wormhole_hum_uses_its_world_region_and_fades_beyond_it() {
        let mut app = app();
        draw_membrane(&mut app);
        set_phase(&mut app, GatePortalPhase::On);
        app.update();
        let requests = app.world_mut().resource_mut::<Messages<OwnedSfxLoopMessage>>().drain().collect::<Vec<_>>();
        let emitter = requests.iter().find_map(|request| match request.request {
            SfxLoopRequest::Hold { emitter, .. } => Some(emitter), _ => None,
        }).expect("the open gate holds a hum");
        assert_eq!(emitter.position(), Some(Vec2::new(540.0, 388.0)));
        assert_eq!(emitter.gain_at(Vec2::new(540.0, 388.0)), 1.0);
        assert_eq!(emitter.gain_at(Vec2::new(600.0, 388.0)), 1.0);
        assert_eq!(emitter.gain_at(Vec2::new(2540.0, 388.0)), 0.0);
    }

}
