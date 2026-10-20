//! A blast in a tiny App: a native fixture module asks for one from a
//! special's params, through the real host.

use super::*;
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{Blast, FighterSpecialPort};
use ambition_extension_host::ExtensionHostPlugin;
use ambition_extension_sdk::{
    CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey,
    Port as _, TriggerBinding,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const BLAST: &str = "test.blast";
const DT: f32 = 1.0 / 60.0;

/// The special's blast; the credit is decimal text, since a param number is
/// an `f32`.
fn blast(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let number = |path: &str, or: f32| special.number(path).unwrap_or(or);
    let at = inv.trigger::<FighterSpecialPort>()?.position;
    inv.submit::<BlastPort>(Blast {
        center: [number("x", at[0]), at[1]],
        half_extent: [number("half", 40.0), number("half", 40.0)],
        damage: number("damage", 6.0) as i32,
        knockback: number("knockback", 1.2),
        lifetime_s: number("lifetime_s", 0.3),
        credit: special.text("credit").and_then(|text| text.parse().ok()).unwrap_or(0),
    })
}

/// A damage box as it was asked for.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    owner: Entity,
    center: ae::Vec2,
    faction: ambition_vfx::HitSide,
    half_extent: ae::Vec2,
    damage: i32,
    knockback: f32,
    lifetime_s: f32,
}

/// Every damage box asked for, in order.
#[derive(Resource, Default)]
struct Boxes(Vec<Seen>);

fn capture(mut reader: MessageReader<ambition_vfx::EffectRequest>, mut boxes: ResMut<Boxes>) {
    for request in reader.read() {
        if let ambition_vfx::Effect::DamageBox(b) = &request.effect {
            boxes.0.push(Seen {
                owner: request.owner,
                center: b.center,
                faction: b.faction,
                half_extent: b.half_extent,
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
            });
        }
    }
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, DT))
        .init_resource::<Boxes>()
        .add_message::<ActorActionMessage>();
    crate::extension::install_for_specials(&mut app);
    app.add_systems(Sim, capture.after(ambition_extension_host::ExtensionSet::Lower(TECHNIQUE_EXECUTION)));
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "blast"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: vec![],
        entries: vec![EntryDescriptor {
            key: "blast".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: BLAST.into() },
            reads: vec![],
            writes: vec![],
            requests: vec![BlastPort::KEY],
            after: vec![],
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(blast),
        }],
    });
    app.finish();
    app
}

fn body(app: &mut App, id: &str, at: ae::Vec2) -> Entity {
    let kin = ae::BodyKinematics { pos: at, ..Default::default() };
    app.world_mut().spawn((kin, SimId::placement(id))).id()
}

fn press(app: &mut App, who: Entity, params: &str) {
    app.world_mut().write_message(ActorActionMessage {
        actor: who,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(BLAST.into()),
            params: ambition_entity_catalog::ParamValue::parse(params).expect("fixture params parse"),
        },
        move_instance: None,
    });
}

fn step(app: &mut App) {
    app.world_mut().run_schedule(Sim);
}

fn boxes(app: &mut App) -> Vec<Seen> {
    std::mem::take(&mut app.world_mut().resource_mut::<Boxes>().0)
}

fn credit(app: &App, body: Entity) -> u64 {
    credit_of(app.world().get::<SimId>(body).unwrap())
}

/// The blast is where the scope body is, takes no side, is what the module
/// asked for, and is credited to the body the credit names.
#[test]
fn a_blast_is_where_its_body_is_and_credited_to_the_body_named() {
    let mut app = app();
    let marked = body(&mut app, "marked", ae::Vec2::new(120.0, -8.0));
    let striker = body(&mut app, "striker", ae::Vec2::ZERO);
    // A third body, so the credit is found by name, not by being the other.
    let _bystander = body(&mut app, "bystander", ae::Vec2::new(40.0, 0.0));
    let named = credit(&app, striker);
    press(&mut app, marked, &format!(r#"(credit: "{named}", half: 44.0, damage: 7.0, knockback: 1.5, lifetime_s: 0.08)"#));
    step(&mut app);
    let got = boxes(&mut app);
    assert_eq!(got.len(), 1, "one blast asked for, {} lowered", got.len());
    let b = &got[0];
    assert_eq!(b.owner, striker, "the blast is not credited to the body the credit names");
    assert_eq!(b.center, ae::Vec2::new(120.0, -8.0), "the blast is not where its body is");
    assert_eq!(b.faction, ambition_vfx::HitSide::Environment, "a blast took a side");
    assert_eq!((b.half_extent, b.damage, b.knockback, b.lifetime_s), (ae::Vec2::splat(44.0), 7, 1.5, 0.08));
}

/// A credit whose body has left goes to a stand-in that is not the body under
/// the blast, and the stand-in leaves once the blast can no longer land.
#[test]
fn a_blast_whose_credited_body_left_is_owned_by_a_stand_in_until_it_is_spent() {
    let mut app = app();
    let marked = body(&mut app, "marked", ae::Vec2::ZERO);
    let gone = body(&mut app, "gone", ae::Vec2::ZERO);
    let named = credit(&app, gone);
    app.world_mut().entity_mut(gone).despawn();
    press(&mut app, marked, &format!(r#"(credit: "{named}", lifetime_s: 0.3)"#));
    step(&mut app);
    let got = boxes(&mut app);
    let owner = got[0].owner;
    assert_ne!(owner, marked, "the credit fell to the body under the blast");
    assert!(app.world().get::<CreditStandIn>(owner).is_some(), "the owner is not a stand-in");
    let stays = ((0.3 + CREDIT_STAND_IN_MARGIN_S) / DT).ceil() as usize;
    for _ in 1..stays {
        step(&mut app);
    }
    assert!(app.world().get_entity(owner).is_ok(), "the stand-in left before its blast was spent");
    step(&mut app);
    assert!(app.world().get_entity(owner).is_err(), "the stand-in outlived its blast");
}

/// A blast that names nobody, has no end, or is out of its ranges is
/// refused. The control, the same request in range, is lowered.
#[test]
fn a_blast_out_of_range_or_naming_nobody_is_refused() {
    let mut app = app();
    let marked = body(&mut app, "marked", ae::Vec2::ZERO);
    let named = credit(&app, marked);
    for refused in [
        "()".to_string(),
        format!(r#"(credit: "{named}", lifetime_s: 0.0)"#),
        format!(r#"(credit: "{named}", lifetime_s: inf)"#),
        format!(r#"(credit: "{named}", half: -1.0)"#),
        format!(r#"(credit: "{named}", damage: -1.0)"#),
        format!(r#"(credit: "{named}", knockback: NaN)"#),
        format!(r#"(credit: "{named}", x: inf)"#),
    ] {
        press(&mut app, marked, &refused);
        step(&mut app);
        assert!(boxes(&mut app).is_empty(), "the blast {refused} was lowered");
    }
    press(&mut app, marked, &format!(r#"(credit: "{named}")"#));
    step(&mut app);
    assert_eq!(boxes(&mut app).len(), 1, "poison: this fixture lowers no blast at all");
}
