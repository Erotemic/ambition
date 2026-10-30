//! The held pair adapter in a tiny App: a native entry asks for one pair
//! for each body it is triggered for, through the real host, and
//! `lower_held_portal_pairs` keeps the portals.

use super::*;
use ambition_extension_host::{AdmittedExtensions, ExtensionHostPlugin, ExtensionInvocations, ExtensionSet};
use ambition_extension_sdk::{
    wire, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey,
    Port, PortKey, PortRole, TriggerBinding,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
use ambition_portal_port::{HeldPortalPair, PortalEnd};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

/// On a body: the x of the pair its entry asks for, or 0 for no pair.
#[derive(Component)]
struct Asks(u32);

struct Poke;
impl Port for Poke {
    const KEY: PortKey = PortKey::new("test.held_pair_poke", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = u32;
    fn encode(v: &u32, out: &mut Vec<u8>) {
        wire::put_u32(out, *v);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<u32, wire::WireError> {
        r.u32()
    }
}

fn near_of(x: f32) -> ae::Vec2 {
    ae::Vec2::new(x, 500.0)
}

fn far_of(x: f32) -> ae::Vec2 {
    ae::Vec2::new(x + 50.0, 200.0)
}

/// A poke at or past this asks for the pair TURNED: its normals along the
/// side axis, a narrower opening and other looks, at `x - TURNED`.
const TURNED: u32 = 10_000;

/// The pair of the punch, as GNU-ton's conductor asks for it.
fn ask(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let x = *inv.trigger::<Poke>()?;
    if x == 0 {
        return Ok(());
    }
    let (x, normals, half_length, looks) = if x >= TURNED {
        (x - TURNED, [[1.0, 0.0], [-1.0, 0.0]], 40.0, [26, 27])
    } else {
        (x, [[0.0, -1.0], [0.0, 1.0]], 65.0, [24, 25])
    };
    let end = |at: ae::Vec2, normal: [f32; 2]| PortalEnd { at: [at.x, at.y], normal, half_length, half_thickness: None };
    inv.submit::<HeldPortalPairPort>(HeldPortalPair {
        slot: 0,
        ends: [end(near_of(x as f32), normals[0]), end(far_of(x as f32), normals[1])],
        looks,
        closes_on_use: false,
    })
}

fn collect_pokes(mut invocations: ResMut<ExtensionInvocations>, bodies: Query<(Entity, &Asks)>) {
    for (entity, asks) in &bodies {
        invocations.trigger::<Poke>(&BOSS_CONDUCT, "go", entity, None, false, asks.0);
    }
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(BOSS_CONDUCT, "test", collect_pokes)
        .install_extension_request::<HeldPortalPairPort, _>(
            BOSS_CONDUCT,
            "ambition_portal2d",
            lower_held_portal_pairs::<ambition_extension_host::InBossConduct>,
        )
        .add_message::<crate::PortalBodyTransited>()
        .add_systems(
            Sim,
            (crate::resolve_portal_links, crate::equalize_pair_apertures)
                .chain()
                .after(ExtensionSet::Lower(BOSS_CONDUCT)),
        )
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "punch"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "conduct".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding { port: Poke::KEY, selector: "go".into() },
                reads: vec![],
                writes: vec![],
                requests: vec![HeldPortalPairPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(ask),
            }],
        });
    app
}

fn finished_app() -> App {
    let mut app = app();
    app.finish();
    app
}

/// Each open portal: its holder's placement id, its room, its place, its
/// channel, and whether it is the near end (normal up the screen).
fn portals(app: &mut App) -> Vec<(String, LiveRoomInstance, ae::Vec2, crate::PortalChannel, bool)> {
    let world = app.world_mut();
    let mut q = world.query::<(&HeldPortalOf, &PlacedPortal, &InRoomInstance)>();
    let rows: Vec<_> = q.iter(world).map(|(of, portal, room)| (of.0, room.0, portal.pos, portal.channel, portal.normal.y < 0.0)).collect();
    rows.into_iter()
        .map(|(holder, room, pos, channel, near)| {
            let id = world.get::<SimId>(holder).map(|id| id.to_string()).unwrap_or_default();
            (id, room, pos, channel, near)
        })
        .collect()
}

/// ⭐ DEBT 12: EACH HOLDER HAS HIS OWN PAIR. GNU-ton's pair was joined by
/// (room, line) on the fixed channels 24/25 with a singleton id, so two
/// GNU-tons in one room had one pair. Here two holders share room #0, and a
/// second live instance (#1) holds a body with the first one's placement id.
/// A pair that is no longer asked for closes, and a pair whose holder is
/// gone closes; the others stay where they were asked to be.
#[test]
fn each_holder_has_his_own_pair_while_he_asks_for_it() {
    let mut app = finished_app();
    let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in rooms {
        app.world_mut().spawn((RoomInstanceRoot, room));
    }
    let holders = [("a", rooms[0], 100u32), ("b", rooms[0], 900), ("a", rooms[1], 300)];
    let bodies: Vec<Entity> = holders
        .iter()
        .map(|(id, room, x)| app.world_mut().spawn((Asks(*x), SimId::placement(id), InRoomInstance(*room))).id())
        .collect();
    for _ in 0..3 {
        app.world_mut().run_schedule(Sim);
    }
    let open = portals(&mut app);
    assert_eq!(open.len(), 6, "three holders, three pairs, and no pair spawned twice: {open:?}");
    for (id, room, x) in holders {
        let end = |near: bool| {
            open.iter()
                .find(|(owner, r, .., n)| owner.ends_with(id) && *r == room && *n == near)
                .unwrap_or_else(|| panic!("{id} in {room:?} has his end (near {near}): {open:?}"))
        };
        let (near, far) = (end(true), end(false));
        assert_eq!((near.2, far.2), (near_of(x as f32), far_of(x as f32)), "{id} in {room:?}: his ends are where he asked");
        assert_eq!(near.3.partner(), far.3, "{id} in {room:?}: his two ends are a pair");
        let others: Vec<_> = open.iter().filter(|(owner, r, ..)| *r == room && !owner.ends_with(id)).collect();
        assert!(
            others.iter().all(|other| other.3 != near.3.partner() && other.3 != far.3.partner()),
            "{id} in {room:?}: no end of another holder is the partner of his: {others:?}"
        );
    }

    // A stops asking in room #0; B moves his pair.
    app.world_mut().get_mut::<Asks>(bodies[0]).unwrap().0 = 0;
    app.world_mut().get_mut::<Asks>(bodies[1]).unwrap().0 = 800;
    app.world_mut().run_schedule(Sim);
    app.world_mut().run_schedule(Sim);
    let first: Vec<_> = portals(&mut app).into_iter().filter(|(_, room, ..)| *room == rooms[0]).collect();
    assert_eq!(first.len(), 2, "only B's pair is open in room #0: {first:?}");
    for (owner, _, pos, _, near) in &first {
        assert!(owner.ends_with('b'), "only B's pair is open in room #0: {first:?}");
        assert_eq!(*pos, if *near { near_of(800.0) } else { far_of(800.0) }, "B's ends follow where he asks");
    }

    // B is gone: his pair closes. A in room #1 keeps his.
    app.world_mut().despawn(bodies[1]);
    app.world_mut().run_schedule(Sim);
    app.world_mut().run_schedule(Sim);
    let left = portals(&mut app);
    assert_eq!(left.len(), 2, "a holder that is gone takes his pair with him: {left:?}");
    assert!(left.iter().all(|(owner, room, ..)| owner.ends_with('a') && *room == rooms[1]), "{left:?}");
}

/// ⛔ SUBMITTED IS NOT APPLIED: a body with no simulation identity has no
/// stable link, so the adapter refuses its pair. The control is the same
/// body with an identity.
#[test]
fn a_body_with_no_identity_gets_no_pair() {
    for with_id in [true, false] {
        let mut app = finished_app();
        app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
        let body = app.world_mut().spawn((Asks(100), InRoomInstance(LiveRoomInstance::ACTIVATION))).id();
        if with_id {
            app.world_mut().entity_mut(body).insert(SimId::placement("a"));
        }
        app.world_mut().run_schedule(Sim);
        let count = app.world_mut().query::<&HeldPortalOf>().iter(app.world()).count();
        assert_eq!(count, if with_id { 2 } else { 0 }, "with an identity {with_id}");
    }
}

/// On a body: how many lines of light its entry asks for, at which x.
#[derive(Component)]
struct AsksLines(u32, f32);

struct LinePoke;
impl Port for LinePoke {
    const KEY: PortKey = PortKey::new("test.light_line_poke", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = (u32, f32);
    fn encode(v: &(u32, f32), out: &mut Vec<u8>) {
        wire::put_u32(out, v.0);
        wire::put_f32(out, v.1);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<(u32, f32), wire::WireError> {
        Ok((r.u32()?, r.f32()?))
    }
}

/// The lines of GNU-ton's shoulders, as his conductor asks for them: slot
/// `i` at `x + 10 i`. Slot 0 is asked for twice, and the second one counts.
fn ask_lines(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let (count, x) = *inv.trigger::<LinePoke>()?;
    let line = |slot: u32, x: f32| ambition_portal_port::LightLine {
        slot,
        at: [x, 400.0],
        normal: [1.0, 0.0],
        length: 120.0,
        look: 24,
        on_limb: None,
    };
    if count > 0 {
        inv.submit::<LightLinePort>(line(0, -1.0))?;
    }
    for slot in 0..count {
        inv.submit::<LightLinePort>(line(slot, x + 10.0 * slot as f32))?;
    }
    Ok(())
}

fn collect_line_pokes(mut invocations: ResMut<ExtensionInvocations>, bodies: Query<(Entity, &AsksLines)>) {
    for (entity, asks) in &bodies {
        invocations.trigger::<LinePoke>(&BOSS_CONDUCT, "go", entity, None, false, (asks.0, asks.1));
    }
}

fn line_app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<LinePoke, _>(BOSS_CONDUCT, "test", collect_line_pokes)
        .install_extension_request::<LightLinePort, _>(BOSS_CONDUCT, "ambition_portal2d", lower_light_lines)
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "lines"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "conduct".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding { port: LinePoke::KEY, selector: "go".into() },
                reads: vec![],
                writes: vec![],
                requests: vec![LightLinePort::KEY],
                after: vec![],
                limits: Limits { max_requests: 8 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(ask_lines),
            }],
        });
    app.finish();
    app
}

/// Each line of a body's view: its slot and its x.
fn lines_of(app: &App, body: Entity) -> Option<Vec<(u32, f32)>> {
    app.world().get::<HeldLightLines>(body).map(|held| held.0.iter().map(|line| (line.slot, line.at.x)).collect())
}

/// ⭐ A BODY'S LINES OF LIGHT ARE THE LINES ITS ENTRY ASKED FOR THIS TICK. Two
/// bodies each have their own; a line that is not asked for goes, and a body
/// that asks for none has no view. The simulation spawns nothing for a line.
#[test]
fn each_body_has_the_lines_its_entry_asks_for_this_tick() {
    let mut app = line_app();
    let a = app.world_mut().spawn(AsksLines(2, 100.0)).id();
    let b = app.world_mut().spawn(AsksLines(1, 900.0)).id();
    app.world_mut().run_schedule(Sim);
    assert_eq!(lines_of(&app, a), Some(vec![(0, 100.0), (1, 110.0)]), "A's two lines; the second word for slot 0 counts");
    assert_eq!(lines_of(&app, b), Some(vec![(0, 900.0)]), "B's one line");
    let entities = app.world().entities().len();

    app.world_mut().get_mut::<AsksLines>(a).unwrap().0 = 1;
    app.world_mut().get_mut::<AsksLines>(b).unwrap().1 = 800.0;
    app.world_mut().run_schedule(Sim);
    assert_eq!(lines_of(&app, a), Some(vec![(0, 100.0)]), "a line that A no longer asks for goes");
    assert_eq!(lines_of(&app, b), Some(vec![(0, 800.0)]), "B's line follows where he asks");

    app.world_mut().get_mut::<AsksLines>(a).unwrap().0 = 0;
    app.world_mut().run_schedule(Sim);
    app.world_mut().run_schedule(Sim);
    assert_eq!(lines_of(&app, a), None, "a body that asks for no line has no view");
    assert_eq!(lines_of(&app, b), Some(vec![(0, 800.0)]));
    assert_eq!(app.world().entities().len(), entities, "the simulation spawns nothing for a line");
}

/// The first end of `holder`'s pair: its normal, its opening as placed, and
/// its look.
fn first_end(app: &mut App, holder: Entity) -> (ae::Vec2, f32, crate::PortalLook) {
    let world = app.world_mut();
    let record = world.get::<HeldPortalPairs>(holder).expect("a held pair").0[0].clone();
    let portal = world.get::<PlacedPortal>(record.ends[0]).expect("its first end");
    let look = *world.get::<crate::PortalLook>(record.ends[0]).expect("its look");
    (portal.normal, portal.opening_half_length(), look)
}

/// ⛔ EACH REQUEST IS THE WHOLE OF ITS PAIR. A module that turns its held
/// pair and narrows it gets a turned, narrower pair, with the looks it asks
/// for, and the same two portals (review 2026-10-12: only the places
/// followed). The control is the first request, which the pair was made
/// from.
#[test]
fn a_held_pair_takes_the_normal_the_opening_and_the_look_it_is_asked_for() {
    let mut app = finished_app();
    let room = LiveRoomInstance::ACTIVATION;
    app.world_mut().spawn((RoomInstanceRoot, room));
    let holder = app.world_mut().spawn((Asks(100), SimId::placement("a"), InRoomInstance(room))).id();
    app.world_mut().run_schedule(Sim);
    let ends = app.world().get::<HeldPortalPairs>(holder).expect("a held pair").0[0].ends;
    let indexed = |look: u8| crate::PortalLook(PortalChannel::Authored(PortalChannelColor::Indexed(look)));
    assert_eq!(
        first_end(&mut app, holder),
        (ae::Vec2::new(0.0, -1.0), 65.0, indexed(24)),
        "control: the pair is made as first asked"
    );

    app.world_mut().get_mut::<Asks>(holder).unwrap().0 = TURNED + 100;
    app.world_mut().run_schedule(Sim);
    assert_eq!(
        first_end(&mut app, holder),
        (ae::Vec2::new(1.0, 0.0), 40.0, indexed(26)),
        "a turned, narrower request turns and narrows the held pair"
    );
    assert_eq!(app.world().get::<HeldPortalPairs>(holder).unwrap().0[0].ends, ends, "the same two portals");
}

/// ⛔ A PAIR IS HELD ONLY WHILE ITS ENTRY CAN ASK FOR IT. When the entry is
/// no longer admitted (a reload that removed it), its pair is released and
/// its portals close (review 2026-10-12: a pair of an unadmitted entry was
/// kept as asked). The control is the tick before, with the entry admitted.
#[test]
fn the_pair_of_an_entry_that_is_no_longer_admitted_is_released() {
    let mut app = finished_app();
    let room = LiveRoomInstance::ACTIVATION;
    app.world_mut().spawn((RoomInstanceRoot, room));
    let holder = app.world_mut().spawn((Asks(100), SimId::placement("a"), InRoomInstance(room))).id();
    app.world_mut().run_schedule(Sim);
    assert_eq!(portals(&mut app).len(), 2, "control: the admitted entry holds its pair");

    let mut admitted = (*app.world().resource::<AdmittedExtensions>().0).clone();
    admitted.entries.clear();
    app.world_mut().resource_mut::<AdmittedExtensions>().0 = std::sync::Arc::new(admitted);
    app.world_mut().run_schedule(Sim);
    app.world_mut().run_schedule(Sim);
    assert_eq!(portals(&mut app), vec![], "the pair of an entry that is gone is closed");
    assert!(
        app.world().get::<HeldPortalPairs>(holder).is_none_or(|held| held.0.is_empty()),
        "and its record released"
    );
}

/// A move's poke, in `technique_execution`: the slot of the pair its entry
/// asks for, or 0 for none. A slot at or past `CLOSING` closes on use; a
/// poke at or past `DEEPER` asks for the slot `poke - DEEPER`, deeper.
#[derive(Component)]
struct MoveAsks(u32);

struct MovePoke;
impl Port for MovePoke {
    const KEY: PortKey = PortKey::new("test.held_pair_move_poke", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = u32;
    fn encode(v: &u32, out: &mut Vec<u8>) {
        wire::put_u32(out, *v);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<u32, wire::WireError> {
        r.u32()
    }
}

const CLOSING: u32 = 100;
const DEEPER: u32 = 1000;

/// A recovery's pair: thin, at x 100, in the asked slot.
fn ask_move_pair(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let poke = *inv.trigger::<MovePoke>()?;
    if poke == 0 {
        return Ok(());
    }
    let (slot, depth) = if poke >= DEEPER { (poke - DEEPER, 11.0) } else { (poke, 6.0) };
    let end = |at: ae::Vec2, normal: [f32; 2]| PortalEnd {
        at: [at.x, at.y],
        normal,
        half_length: 30.0,
        half_thickness: Some(depth),
    };
    inv.submit::<HeldPortalPairPort>(HeldPortalPair {
        slot,
        ends: [end(near_of(100.0), [0.0, -1.0]), end(far_of(100.0), [0.0, 1.0])],
        looks: [8, 9],
        closes_on_use: slot >= CLOSING,
    })
}

/// On a body: the entry that asks for its pair is `keep`, not `go`.
#[derive(Component)]
struct KeptBy;

fn collect_move_pokes(
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<(Entity, &MoveAsks, Has<KeptBy>)>,
) {
    for (entity, asks, kept) in &bodies {
        let selector = if kept { "keep" } else { "go" };
        invocations.trigger::<MovePoke>(&TECHNIQUE_EXECUTION, selector, entity, None, false, asks.0);
    }
}

/// The held pair in both phases: GNU-ton's boss entry and a move's entry.
fn two_phase_app() -> App {
    let mut app = app();
    install_for_technique_execution(&mut app);
    app.install_extension_trigger::<MovePoke, _>(TECHNIQUE_EXECUTION, "test", collect_move_pokes)
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "recovery"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "open".into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: MovePoke::KEY, selector: "go".into() },
                reads: vec![],
                writes: vec![],
                requests: vec![HeldPortalPairPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(ask_move_pair),
            }, EntryDescriptor {
                key: "keep".into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: MovePoke::KEY, selector: "keep".into() },
                reads: vec![],
                writes: vec![],
                requests: vec![HeldPortalPairPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(ask_move_pair),
            }],
        });
    app.finish();
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
    app
}

fn mover(app: &mut App, slot: u32) -> Entity {
    app.world_mut()
        .spawn((MoveAsks(slot), Asks(0), SimId::placement("mover"), InRoomInstance(LiveRoomInstance::ACTIVATION)))
        .id()
}

fn open_ends(app: &mut App) -> Vec<(ae::Vec2, f32)> {
    let world = app.world_mut();
    let mut q = world.query::<(&HeldPortalOf, &PlacedPortal)>();
    q.iter(world).map(|(_, portal)| (portal.pos, portal.half_thickness)).collect()
}

fn came_out_at(app: &mut App, body: Entity, at: ae::Vec2) {
    app.world_mut().write_message(crate::PortalBodyTransited {
        body,
        enter_normal: ae::Vec2::new(0.0, -1.0),
        exit_normal: ae::Vec2::new(0.0, 1.0),
        facing_flip: false,
        input_warp: false,
        exit_pos: at,
    });
}

/// A pair a move holds in `technique_execution` stays open while the boss
/// phase's adapter runs every tick and is asked for nothing.
#[test]
fn a_pair_held_in_one_phase_is_not_released_by_the_other() {
    let mut app = two_phase_app();
    mover(&mut app, 1);
    for _ in 0..3 {
        app.world_mut().run_schedule(Sim);
    }
    let ends = open_ends(&mut app);
    assert_eq!(ends.len(), 2, "the boss phase released the move's pair: {ends:?}");
}

/// A pair opens as deep as it was asked on the tick it opens, and an asked
/// change of depth for its slot is made.
#[test]
fn a_pair_is_as_deep_as_it_is_asked() {
    let mut app = two_phase_app();
    let body = mover(&mut app, 1);
    app.world_mut().run_schedule(Sim);
    let ends = open_ends(&mut app);
    assert!(ends.len() == 2 && ends.iter().all(|(_, depth)| *depth == 6.0), "it opened at another depth: {ends:?}");
    app.world_mut().get_mut::<MoveAsks>(body).unwrap().0 = DEEPER + 1;
    app.world_mut().run_schedule(Sim);
    let ends = open_ends(&mut app);
    assert!(ends.len() == 2 && ends.iter().all(|(_, depth)| *depth == 11.0), "the asked depth was not made: {ends:?}");
}

/// A pair that closes on use closes when a body comes out of it, and stays
/// closed while its slot is asked for; a new slot opens a new pair. An
/// ordinary pair survives the same use, and a use elsewhere closes nothing.
#[test]
fn a_pair_that_closes_on_use_closes_once_a_body_comes_out() {
    let mut app = two_phase_app();
    let body = mover(&mut app, CLOSING);
    app.world_mut().run_schedule(Sim);
    came_out_at(&mut app, body, far_of(100.0) + ae::Vec2::new(0.0, 200.0));
    app.world_mut().run_schedule(Sim);
    assert_eq!(open_ends(&mut app).len(), 2, "a use elsewhere closed the pair");
    came_out_at(&mut app, body, far_of(100.0));
    app.world_mut().run_schedule(Sim);
    assert_eq!(open_ends(&mut app).len(), 0, "the used pair stayed open");
    app.world_mut().run_schedule(Sim);
    assert_eq!(open_ends(&mut app).len(), 0, "asking for a used slot opened it again");
    app.world_mut().get_mut::<MoveAsks>(body).unwrap().0 = CLOSING + 1;
    app.world_mut().run_schedule(Sim);
    assert_eq!(open_ends(&mut app).len(), 2, "a new slot did not open a pair");

    let mut app = two_phase_app();
    let body = mover(&mut app, 1);
    app.world_mut().run_schedule(Sim);
    came_out_at(&mut app, body, far_of(100.0));
    app.world_mut().run_schedule(Sim);
    assert_eq!(open_ends(&mut app).len(), 2, "an ordinary pair closed on its use");
}

/// One module's entries name one pair by its slot: a pair one entry opened
/// and another keeps is the same two portals, not a new pair each time the
/// asking entry changes.
#[test]
fn a_pair_one_entry_opens_and_another_keeps_is_one_pair() {
    let mut app = two_phase_app();
    let body = mover(&mut app, 1);
    app.world_mut().run_schedule(Sim);
    let opened: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<HeldPortalOf>>();
        q.iter(world).collect()
    };
    app.world_mut().entity_mut(body).insert(KeptBy);
    app.world_mut().run_schedule(Sim);
    let kept: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<HeldPortalOf>>();
        q.iter(world).collect()
    };
    assert_eq!(opened.len(), 2);
    assert_eq!(kept, opened, "the keeping entry closed the pair and opened another");
}

/// ⛔ Two portals never share a place (Jon, 2026-10-30: "if a portal detects
/// an overlap they just fizzle"). A pair asked where an end would overlap a
/// portal already open fizzles, and stays closed while it is asked for; a
/// held pair asked onto another portal closes. The portal that was there
/// first stays. The controls are the same pairs apart.
#[test]
fn a_pair_that_would_overlap_a_portal_fizzles() {
    for (second_x, overlaps) in [(400u32, false), (150, true)] {
        let mut app = finished_app();
        app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
        let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
        let first = app.world_mut().spawn((Asks(100), SimId::placement("a"), room)).id();
        // Asked in the same tick: the first in the outbox opens.
        let second = app.world_mut().spawn((Asks(second_x), SimId::placement("b"), room)).id();
        for _ in 0..3 {
            app.world_mut().run_schedule(Sim);
        }
        let open = portals(&mut app);
        let of = |id: char| open.iter().filter(|(owner, ..)| owner.ends_with(id)).count();
        assert_eq!(of('a'), 2, "the first pair is open: {open:?}");
        assert_eq!(of('b'), if overlaps { 0 } else { 2 }, "the second pair fizzles only on an overlap: {open:?}");

        // A fizzled pair stays closed while it is asked, even when its place
        // comes clear; asked anew in a fresh slot (a new module ask after a
        // stop), it opens.
        if overlaps {
            app.world_mut().get_mut::<Asks>(first).unwrap().0 = 0;
            app.world_mut().run_schedule(Sim);
            app.world_mut().run_schedule(Sim);
            assert_eq!(portals(&mut app).len(), 0, "a fizzled pair stays closed while it is asked");
            app.world_mut().get_mut::<Asks>(second).unwrap().0 = 0;
            app.world_mut().run_schedule(Sim);
            app.world_mut().get_mut::<Asks>(second).unwrap().0 = second_x;
            app.world_mut().run_schedule(Sim);
            app.world_mut().run_schedule(Sim);
            assert_eq!(portals(&mut app).len(), 2, "asked again once the place is clear, it opens");
        }
    }
}

#[test]
fn a_held_pair_asked_onto_another_portal_closes() {
    let mut app = finished_app();
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
    let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
    app.world_mut().spawn((Asks(100), SimId::placement("a"), room));
    let mover = app.world_mut().spawn((Asks(600), SimId::placement("b"), room)).id();
    for _ in 0..2 {
        app.world_mut().run_schedule(Sim);
    }
    assert_eq!(portals(&mut app).len(), 4, "two pairs apart");
    app.world_mut().get_mut::<Asks>(mover).unwrap().0 = 120;
    app.world_mut().run_schedule(Sim);
    app.world_mut().run_schedule(Sim);
    let open = portals(&mut app);
    assert!(open.iter().all(|(owner, ..)| owner.ends_with('a')) && open.len() == 2, "B's pair closed, A's stayed: {open:?}");
    app.world_mut().get_mut::<Asks>(mover).unwrap().0 = 600;
    app.world_mut().run_schedule(Sim);
    assert_eq!(portals(&mut app).len(), 2, "and it stays closed while it is asked");
}

#[test]
fn a_pair_asked_across_an_open_portal_fizzles() {
    let mut app = finished_app();
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION));
    let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
    // Its near line is at (x, 500), its far line at (x + 50, 200): a portal
    // already open across the far line's place is overlapped by it.
    app.world_mut().spawn((
        PlacedPortal::fixed(crate::UNRESOLVED_LINK_CHANNEL, ae::Vec2::new(150.0, 200.0), ae::Vec2::new(1.0, 0.0), 40.0),
        room,
    ));
    app.world_mut().spawn((Asks(100), SimId::placement("a"), room));
    for _ in 0..2 {
        app.world_mut().run_schedule(Sim);
    }
    assert_eq!(portals(&mut app).len(), 0, "a pair one of whose ends crosses an open portal fizzles");
}
