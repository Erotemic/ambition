//! Module items in a tiny App: a native fixture module spawns items, reads
//! what it owns and ends one, an item entity reads what it is, and a flown
//! entity is moved, through the real host.

use super::*;
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{
    EndOwnedEntity, FighterSpecialPort, ItemStatePort, ModuleItemSpawn, MoveModuleEntity, OwnedEntitiesPort,
    SpawnModuleItemPort,
};
use ambition_extension_host::{ExtensionAppExt as _, ExtensionHostPlugin, ExtensionSet};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::{
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, SchemaKey, TriggerBinding,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const PLACE: &str = "test.place";
const OWN: &str = "test.own";
const END: &str = "test.end";
const ITEM: &str = "test.item";
const FLOWN: &str = "test.flown";
const DRAGGED: &str = "test.dragged";
const DT: f32 = 1.0 / 60.0;

record! {
    /// What the fixture saw of what its body owns.
    struct Owned = SchemaKey::new("test", "items.owned", 1);
    1 count: u32,
    2 first_age_s: f32,
    3 first_position: [f32; 2],
    4 first_handle: u64,
}

record! {
    /// What an item entity saw of the item it is.
    struct Seen = SchemaKey::new("test", "items.seen", 1);
    1 ticks: u32,
    2 in_world: bool,
    3 impact_speed: f32,
}

/// Spawn the special's `item` (the bomb by default) of `kind` (`ITEM` by
/// default) where the body is.
fn place(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    inv.submit::<SpawnModuleItemPort>(ModuleItemSpawn {
        kind: special.text("kind").unwrap_or(ITEM).into(),
        item: special.text("item").unwrap_or("polygon_bomb").into(),
        at: special.position,
        lifetime_s: 3600.0,
        half_extents: [8.0, 8.0],
        params: Vec::new(),
    })
}

fn own(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let owned = inv.observe::<OwnedEntitiesPort>()?.entities.clone();
    let first = owned.first();
    Owned {
        count: owned.len() as u32,
        first_age_s: first.map_or(0.0, |o| o.age_s),
        first_position: first.map_or([0.0; 2], |o| o.position),
        first_handle: first.map_or(0, |o| o.handle),
    }
    .store(inv)
}

/// End the entity the special's decimal `handle` names.
fn end(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let handle = special.text("handle").and_then(|text| text.parse().ok()).unwrap_or(0);
    inv.submit::<EndOwnedEntityPort>(EndOwnedEntity { handle })
}

fn seen(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let state = *inv.observe::<ItemStatePort>()?;
    let ticks = Seen::load(inv)?.ticks + 1;
    Seen { ticks, in_world: state.in_world, impact_speed: state.impact_speed }.store(inv)
}

/// Move the entity to its params' `to`, or far away.
fn fly(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let to = [tick.number("to[0]").unwrap_or(500.0), tick.number("to[1]").unwrap_or(500.0)];
    inv.submit::<MoveModuleEntityPort>(MoveModuleEntity { to })
}

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::SpriteContent::default());
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, DT))
        .add_message::<ActorActionMessage>()
        .add_message::<ambition_vfx::VfxInRoom>()
        .add_message::<ambition_sfx::OwnedSfxMessage>();
    ambition_combat::extension::install_for_specials(&mut app);
    crate::extension::install_for_technique_execution(&mut app);
    crate::extension::install_module_entities(&mut app);
    app.configure_sets(
        Sim,
        ExtensionSet::Collect(TECHNIQUE_EXECUTION).after(ExtensionSet::Lower(MODULE_ENTITY_TICK)),
    );
    let entry = |key: &'static str, phase, port, selector: &'static str, reads, writes, requests, run| EntryDescriptor {
        key: key.into(),
        phase,
        trigger: TriggerBinding { port, selector: selector.into() },
        reads,
        writes,
        requests,
        after: vec![],
        limits: Limits { max_requests: 1 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "items"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: vec![Owned::schema(), Seen::schema()],
        entries: vec![
            entry("place", TECHNIQUE_EXECUTION, FighterSpecialPort::KEY, PLACE, vec![], vec![], vec![SpawnModuleItemPort::KEY], place),
            entry("own", TECHNIQUE_EXECUTION, FighterSpecialPort::KEY, OWN, vec![OwnedEntitiesPort::KEY], vec![Owned::KEY], vec![], own),
            entry("end", TECHNIQUE_EXECUTION, FighterSpecialPort::KEY, END, vec![], vec![], vec![EndOwnedEntityPort::KEY], end),
            entry("seen", MODULE_ENTITY_TICK, ModuleEntityTickPort::KEY, ITEM, vec![ItemStatePort::KEY], vec![Seen::KEY], vec![], seen),
            entry("fly", MODULE_ENTITY_TICK, ModuleEntityTickPort::KEY, FLOWN, vec![], vec![], vec![MoveModuleEntityPort::KEY], fly),
            entry("drag", MODULE_ENTITY_TICK, ModuleEntityTickPort::KEY, DRAGGED, vec![], vec![], vec![MoveModuleEntityPort::KEY], fly),
        ],
    });
    app.finish();
    app
}

fn body(app: &mut App, slot: u8, at: ae::Vec2) -> Entity {
    let kin = ae::BodyKinematics { pos: at, facing: 1.0, size: ae::Vec2::new(28.0, 64.0), ..Default::default() };
    app.world_mut().spawn((kin, SimId::player_slot(slot))).id()
}

fn press(app: &mut App, who: Entity, key: &str, params: &str) {
    app.world_mut().write_message(ActorActionMessage {
        actor: who,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(key.into()),
            params: ambition_entity_catalog::ParamValue::parse(params).expect("fixture params parse"),
        },
        move_instance: None,
    });
}

fn step(app: &mut App) {
    app.world_mut().run_schedule(Sim);
}

fn items(app: &mut App) -> Vec<(Entity, ModuleEntity)> {
    let mut q = app.world_mut().query::<(Entity, &ModuleEntity)>();
    q.iter(app.world()).map(|(e, m)| (e, m.clone())).collect()
}

fn owned(app: &App, who: Entity) -> Owned {
    let records = app.world().get::<ambition_extension_host::BodyRecords>(who).unwrap();
    Owned::from_record(records.get(&Owned::KEY).unwrap()).unwrap()
}

/// A placed item is a module entity of the spawner's, at rest on the stage
/// as the item it names; an item the game does not register is refused.
#[test]
fn a_placed_item_is_its_spawners_entity_and_a_ground_item() {
    let mut app = app();
    let placer = body(&mut app, 0, ae::Vec2::new(40.0, 10.0));
    press(&mut app, placer, PLACE, r#"(item: "no_such_item")"#);
    step(&mut app);
    assert!(items(&mut app).is_empty(), "an unregistered item was placed");
    press(&mut app, placer, PLACE, "()");
    step(&mut app);
    let placed = items(&mut app);
    assert_eq!(placed.len(), 1);
    let (entity, module_entity) = &placed[0];
    assert_eq!(module_entity.spawner, Some(SimId::player_slot(0)));
    assert_eq!(module_entity.half_extents, ae::Vec2::ZERO, "an item entity was made touchable as a module entity");
    let item = app.world().get::<ambition_held_items::GroundItem>(*entity).expect("the entity is an item");
    assert_eq!((item.spec.id.as_str(), item.pos), ("polygon_bomb", ae::Vec2::new(40.0, 10.0)));
}

/// What a body owns is its spawns only, with their age, where they are (an
/// item in a hand: the hand) and a handle that names them.
#[test]
fn a_body_owns_its_own_spawns_wherever_they_are() {
    let mut app = app();
    let placer = body(&mut app, 0, ae::Vec2::new(40.0, 10.0));
    let rival = body(&mut app, 1, ae::Vec2::new(-300.0, 10.0));
    press(&mut app, placer, PLACE, "()");
    press(&mut app, rival, PLACE, "()");
    step(&mut app);
    for _ in 0..3 {
        step(&mut app);
    }
    let (mine, _) = items(&mut app)
        .into_iter()
        .find(|(_, m)| m.spawner == Some(SimId::player_slot(0)))
        .expect("the placer's item");
    // The rival carries it off: it stays the placer's, and is where the rival
    // holds it.
    app.world_mut().entity_mut(mine).insert(ambition_held_items::ItemCustody::Held { holder: rival });
    press(&mut app, placer, OWN, "()");
    step(&mut app);
    let seen = owned(&app, placer);
    assert_eq!(seen.count, 1, "the placer owns {} entities, not its one", seen.count);
    assert!((seen.first_age_s - 4.0 * DT).abs() < 1e-5, "an entity four steps old is {} s old", seen.first_age_s);
    assert!(seen.first_position[0] < -250.0, "a carried item is not where its carrier is: {:?}", seen.first_position);
    let handle = ambition_combat::extension::blast::credit_of(app.world().get::<SimId>(mine).unwrap());
    assert_eq!(seen.first_handle, handle, "the handle does not name the entity");
}

/// A body ends only an entity it spawned.
#[test]
fn a_body_ends_only_its_own_entity() {
    let mut app = app();
    let placer = body(&mut app, 0, ae::Vec2::ZERO);
    let rival = body(&mut app, 1, ae::Vec2::ZERO);
    press(&mut app, placer, PLACE, "()");
    step(&mut app);
    let (mine, _) = items(&mut app)[0].clone();
    let handle = ambition_combat::extension::blast::credit_of(app.world().get::<SimId>(mine).unwrap());
    press(&mut app, rival, END, &format!(r#"(handle: "{handle}")"#));
    step(&mut app);
    assert!(app.world().get_entity(mine).is_ok(), "a rival ended an entity it did not spawn");
    press(&mut app, placer, END, &format!(r#"(handle: "{handle}")"#));
    step(&mut app);
    assert!(app.world().get_entity(mine).is_err(), "the spawner could not end its own entity");
}

/// An item entity reads whether it is loose and the hardest it struck: the
/// settle's speed or a body's, whichever is greater.
#[test]
fn an_item_entity_reads_whether_it_is_loose_and_how_hard_it_struck() {
    let mut app = app();
    let placer = body(&mut app, 0, ae::Vec2::ZERO);
    press(&mut app, placer, PLACE, "()");
    step(&mut app);
    let (item, _) = items(&mut app)[0].clone();
    app.world_mut().entity_mut(item).insert((
        ambition_held_items::SettledItem { impact_speed: 120.0 },
        ambition_held_items::ItemStruckBody { impact_speed: 340.0, bodies: vec![Entity::PLACEHOLDER] },
    ));
    step(&mut app);
    let read = |app: &App| {
        let records = app.world().get::<ambition_extension_host::BodyRecords>(item).unwrap();
        Seen::from_record(records.get(&Seen::KEY).unwrap()).unwrap()
    };
    assert_eq!(read(&app), Seen { ticks: 1, in_world: true, impact_speed: 340.0 });
    app.world_mut().entity_mut(item).insert(ambition_held_items::ItemCustody::Held { holder: placer });
    app.world_mut().entity_mut(item).remove::<ambition_held_items::ItemStruckBody>();
    step(&mut app);
    assert_eq!(read(&app), Seen { ticks: 2, in_world: false, impact_speed: 120.0 });
}

/// A module moves the entity it flies; a point off the world and an entity
/// that is an item (it is where its item is) are refused.
#[test]
fn a_module_moves_its_entity_but_not_an_item_or_off_the_world() {
    let mut app = app();
    let flown = |to: [f32; 2]| ModuleEntity {
        params: vec![
            ("to[0]".to_owned(), SpecialParam::Number(to[0])),
            ("to[1]".to_owned(), SpecialParam::Number(to[1])),
        ],
        ..ModuleEntity::point(FLOWN, ae::Vec2::new(1.0, 2.0), 5.0)
    };
    let moved = app.world_mut().spawn(flown([40.0, -5.0])).id();
    let lost = app.world_mut().spawn(flown([f32::INFINITY, 0.0])).id();
    let placer = body(&mut app, 0, ae::Vec2::ZERO);
    press(&mut app, placer, PLACE, &format!("(kind: \"{DRAGGED}\")"));
    step(&mut app);
    step(&mut app);
    let at = |app: &App, entity: Entity| app.world().get::<ModuleEntity>(entity).unwrap().pos;
    assert_eq!(at(&app, moved), ae::Vec2::new(40.0, -5.0), "the flown entity was not moved");
    assert_eq!(at(&app, lost), ae::Vec2::new(1.0, 2.0), "an entity was moved off the world");
    let dragged: Vec<_> = items(&mut app).into_iter().filter(|(_, entity)| entity.kind == DRAGGED).collect();
    assert_eq!(dragged.len(), 1, "the dragged item was not placed");
    assert_eq!(dragged[0].1.pos, ae::Vec2::ZERO, "an item entity was moved away from its item");
}

/// A module entity's spawner is its credit and its player's stick while it
/// is in the world, and nobody once it has left.
#[test]
fn an_entity_reads_its_spawners_credit_and_stick() {
    let mut world = World::new();
    let id = SimId::player_slot(2);
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.undamped_locomotion = Some(ae::LocalAxes::new(0.5, -1.0));
    let spawner = world.spawn((id.clone(), ambition_characters::control::ActorControl(frame))).id();
    let entity = world
        .spawn(ModuleEntity { spawner: Some(id.clone()), ..ModuleEntity::point("test", ae::Vec2::ZERO, 1.0) })
        .id();
    assert_eq!(
        spawner_body_of(&world, entity),
        Some(SpawnerBody { credit: ambition_combat::extension::blast::credit_of(&id), stick: [0.5, -1.0] })
    );
    assert_eq!(spawner_body_of(&world, spawner), None, "a body that is not a module entity has a spawner");
    world.despawn(spawner);
    assert_eq!(spawner_body_of(&world, entity), Some(SpawnerBody::default()), "a spawner that left is somebody");
}

/// Each toucher of a tick is named by its credit, in the tick's order; one
/// with no identity is `0`.
#[test]
fn a_tick_names_its_touchers_in_order() {
    let mut world = World::new();
    let (first, second) = (SimId::player_slot(1), SimId::player_slot(0));
    let a = world.spawn(first.clone()).id();
    let nameless = world.spawn_empty().id();
    let b = world.spawn(second.clone()).id();
    let entity = world.spawn_empty().id();
    world.insert_resource(ModuleEntityTouchers(vec![(entity, vec![a, nameless, b])]));
    let credit = ambition_combat::extension::blast::credit_of;
    assert_eq!(toucher_credits_of(&world, entity), Some(vec![credit(&first), 0, credit(&second)]));
    assert_eq!(toucher_credits_of(&world, a), None, "a scope that was not ticked has touchers");
}
