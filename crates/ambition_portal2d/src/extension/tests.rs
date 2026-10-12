//! The held pair adapter in a tiny App: a native entry asks for one pair
//! for each body it is triggered for, through the real host, and
//! `lower_held_portal_pairs` keeps the portals.

use super::*;
use ambition_extension_host::{ExtensionHostPlugin, ExtensionInvocations, ExtensionSet};
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

/// The pair of the punch, as GNU-ton's conductor asks for it.
fn ask(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let x = *inv.trigger::<Poke>()?;
    if x == 0 {
        return Ok(());
    }
    let end = |at: ae::Vec2, normal: [f32; 2]| PortalEnd { at: [at.x, at.y], normal, half_length: 65.0 };
    inv.submit::<HeldPortalPairPort>(HeldPortalPair {
        slot: 0,
        ends: [end(near_of(x as f32), [0.0, -1.0]), end(far_of(x as f32), [0.0, 1.0])],
        looks: [24, 25],
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
        .install_extension_request::<HeldPortalPairPort, _>(BOSS_CONDUCT, "ambition_portal2d", lower_held_portal_pairs)
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
    let mut app = app();
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
        let mut app = app();
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
