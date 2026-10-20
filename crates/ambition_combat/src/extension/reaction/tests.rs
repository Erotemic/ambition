//! A reaction to a parry in a tiny App: a native fixture module answers each
//! parry caught in its stance with what the stance's params say, through the
//! real host.

use super::*;
use ambition_combat_port::{AnswerParry, HoldStatus, HoldStatusPort};
use ambition_platformer2d_core as ae;
use ambition_entity_catalog::smash_counter::{counter_move, CounterParams};
use ambition_entity_catalog::EffectRef;
use ambition_extension_host::ExtensionHostPlugin;
use ambition_extension_sdk::{
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, SchemaKey, TriggerBinding,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const STANCE: &str = "test.stance";
const EFFECT: &str = "test.effect";
const STRUCK: &str = "test.struck";

record! {
    /// What the last strike said to the struck body.
    struct Seen = SchemaKey::new("test", "reaction.seen", 1);
    1 striker: u64,
    2 tag: f32,
}

/// Keep the strike's striker and `tag` on the struck body, and hold a status
/// there.
fn struck(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let strike = inv.trigger::<StruckPort>()?.clone();
    Seen { striker: strike.striker, tag: strike.number("tag").unwrap_or(0.0) }.store(inv)?;
    inv.submit::<HoldStatusPort>(HoldStatus { kind: STRUCK.into(), held: true })
}

/// Answer as the stance says: `key`, on the attacker when `on_attacker`, with
/// the params at `at`, naming the parry `skip` places after its own.
fn answer(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let parried = inv.trigger::<ParriedPort>()?.clone();
    let skip = parried.number("skip").unwrap_or(0.0) as u32;
    inv.submit::<AnswerParryPort>(AnswerParry {
        parry: parried.parry + skip,
        key: parried.text("key").unwrap_or("test.answer").into(),
        params_at: parried.text("at").unwrap_or("").into(),
        on_attacker: parried.flag("on_attacker").unwrap_or(false),
    })
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0))
        .add_message::<ActorActionMessage>()
        .add_message::<ParriedBodyHit>();
    install_for_hit_reaction(&mut app);
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "reaction"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: vec![Seen::schema()],
        entries: vec![
            EntryDescriptor {
                key: "answer".into(),
                phase: HIT_REACTION,
                trigger: TriggerBinding { port: ParriedPort::KEY, selector: STANCE.into() },
                reads: vec![],
                writes: vec![],
                requests: vec![AnswerParryPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(answer),
            },
            EntryDescriptor {
                key: "struck".into(),
                phase: HIT_REACTION,
                trigger: TriggerBinding { port: StruckPort::KEY, selector: EFFECT.into() },
                reads: vec![],
                writes: vec![Seen::KEY],
                requests: vec![HoldStatusPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(struck),
            },
        ],
    });
    app.finish();
    app
}

/// A body standing in a stance whose live window sustains `key` with
/// `params`, the move's clock at `at` of the stance window (0 its start, 1
/// its end; outside that, the startup or the recovery).
fn stance_at(app: &mut App, key: &str, params: &str, at: f32) -> Entity {
    let mut spec = counter_move(
        "test_stance",
        "special",
        0.1,
        0.2,
        0.3,
        CounterParams {
            window_s: 0.05,
            response: "unused".into(),
            answers_the_attacker: false,
            response_params: ParamValue::default(),
            absorbs_projectiles: false,
        },
    );
    spec.windows[1].sustain_effect =
        Some(EffectRef { key: key.into(), params: ParamValue::parse(params).expect("fixture params parse") });
    let window = spec.windows[1].clone();
    let mut playback = MovePlayback::new(spec, 1.0);
    playback.t = window.start_s + (window.end_s - window.start_s) * at;
    app.world_mut().spawn(playback).id()
}

fn stance(app: &mut App, params: &str) -> Entity {
    stance_at(app, STANCE, params, 0.5)
}

fn parry(app: &mut App, defender: Entity, attacker: Entity) {
    app.world_mut().write_message(ParriedBodyHit {
        defender,
        attacker,
        hitbox: attacker,
        contact: ae::Vec2::ZERO,
    });
}

/// The presses this tick gave: on whom, which technique, with which params.
fn presses(app: &mut App) -> Vec<(Entity, String, ParamValue)> {
    app.world_mut().run_schedule(Sim);
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .drain()
        .filter_map(|m| match m.request {
            ActionRequest::Special { spec: SpecialActionSpec::Special(key), params } => Some((m.actor, key, params)),
            _ => None,
        })
        .collect()
}

/// The answer is pressed on the body it names, with the part of the stance's
/// params it names, whole: an optional inside stays an optional.
#[test]
fn an_answer_presses_its_technique_on_the_body_it_names_with_the_params_it_names() {
    let mut app = app();
    let attacker = app.world_mut().spawn_empty().id();
    let own = stance(&mut app, r#"(key: "test.own", at: "then", then: (reach: 2.0, extra: Some(3.0)))"#);
    parry(&mut app, own, attacker);
    assert_eq!(
        presses(&mut app),
        vec![(own, "test.own".to_string(), ParamValue::parse("(reach: 2.0, extra: Some(3.0))").unwrap())]
    );

    let aimed = stance(&mut app, r#"(key: "test.slow", on_attacker: true)"#);
    parry(&mut app, aimed, attacker);
    assert_eq!(
        presses(&mut app),
        vec![(attacker, "test.slow".to_string(), ParamValue::default())],
        "an answer on the attacker landed elsewhere, or an empty path handed on params"
    );
}

/// A parry outside the live window, or in a window whose effect no module
/// binds, answers with nothing. The control, in the same tick, is answered.
#[test]
fn only_a_parry_in_a_live_bound_window_is_answered() {
    let mut app = app();
    let attacker = app.world_mut().spawn_empty().id();
    let in_startup = stance_at(&mut app, STANCE, "()", -0.2);
    let in_recovery = stance_at(&mut app, STANCE, "()", 1.2);
    let unbound = stance_at(&mut app, "test.other", "()", 0.5);
    let live = stance(&mut app, "()");
    for defender in [in_startup, in_recovery, unbound, live] {
        parry(&mut app, defender, attacker);
    }
    let pressed: Vec<Entity> = presses(&mut app).into_iter().map(|(who, _, _)| who).collect();
    assert_eq!(pressed, vec![live], "startup, recovery and an unbound effect must not answer");

    // Two live windows, the first sustaining an effect no module binds: the
    // stance is the bound one, not the first.
    let layered = stance(&mut app, "()");
    let playback = app.world().get::<MovePlayback>(layered).unwrap();
    let (mut spec, t) = ((*playback.spec).clone(), playback.t);
    let mut other = spec.windows[1].clone();
    other.sustain_effect = Some(EffectRef::new("test.other"));
    spec.windows.insert(1, other);
    let mut playback = MovePlayback::new(spec, 1.0);
    playback.t = t;
    app.world_mut().entity_mut(layered).insert(playback);
    parry(&mut app, layered, attacker);
    let pressed: Vec<Entity> = presses(&mut app).into_iter().map(|(who, _, _)| who).collect();
    assert_eq!(pressed, vec![layered], "an unbound effect in an earlier live window hid the stance");
}

/// A module names only a parry its scope caught: one that names another
/// body's parry, or a parry this tick did not catch, is refused, and its
/// neighbour's own answer still lands.
#[test]
fn an_answer_to_a_parry_the_scope_did_not_catch_is_refused() {
    let mut app = app();
    let attacker = app.world_mut().spawn_empty().id();
    // Parry 0 is the poacher's and names parry 1, the honest body's.
    let poacher = stance(&mut app, "(skip: 1.0)");
    let honest = stance(&mut app, "()");
    parry(&mut app, poacher, attacker);
    parry(&mut app, honest, attacker);
    assert_eq!(
        presses(&mut app).into_iter().map(|(who, _, _)| who).collect::<Vec<_>>(),
        vec![honest],
        "the poacher's answer to the honest body's parry was pressed"
    );

    let past = stance(&mut app, "(skip: 1.0)");
    parry(&mut app, past, attacker);
    assert!(presses(&mut app).is_empty(), "an answer to a parry this tick did not catch was pressed");
}

/// An answer that names params the stance does not state is refused, not
/// pressed with none: a misspelt path would otherwise fire a default answer.
#[test]
fn an_answer_naming_params_the_stance_does_not_state_is_refused() {
    let mut app = app();
    let attacker = app.world_mut().spawn_empty().id();
    let typo = stance(&mut app, r#"(at: "thne", then: (reach: 2.0))"#);
    parry(&mut app, typo, attacker);
    assert!(presses(&mut app).is_empty(), "an answer with absent params was pressed");
    // Poison guard: the same stance with the path spelt right is answered.
    let right = stance(&mut app, r#"(at: "then", then: (reach: 2.0))"#);
    parry(&mut app, right, attacker);
    assert_eq!(presses(&mut app).len(), 1, "poison: this fixture answers nothing at all");
}

fn strike(app: &mut App, owner: Entity, victim: Entity, key: &str, tag: f32) {
    app.world_mut().write_message(OnHitEffectMessage {
        owner,
        victim,
        volume: ae::CombatVolume::Circle { center: ae::Vec2::ZERO, radius: 1.0 },
        contact: ae::Vec2::ZERO,
        effect: EffectRef { key: key.into(), params: ParamValue::parse(&format!("(tag: {tag})")).unwrap() },
    });
}

fn seen(app: &App, body: Entity) -> Option<Seen> {
    let records = app.world().get::<ambition_extension_host::BodyRecords>(body)?;
    Some(Seen::from_record(records.get(&Seen::KEY)?).unwrap())
}

fn struck_holds(app: &App, body: Entity) -> bool {
    app.world().get::<crate::extension::status::BodyStatuses>(body).is_some_and(|held| held.holds(STRUCK))
}

/// A strike is said to the effect's module on the body it struck, with the
/// effect's params and its striker's credit; the strikes of a tick in the
/// order they landed, so the last one is what the body keeps. A status held
/// in `hit_reaction` is held.
#[test]
fn a_strike_is_said_on_the_struck_body_with_its_strikers_credit_in_order() {
    let mut app = app();
    let first = app.world_mut().spawn((ae::BodyKinematics::default(), SimId::placement("first"))).id();
    let second = app.world_mut().spawn((ae::BodyKinematics::default(), SimId::placement("second"))).id();
    let victim = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    strike(&mut app, first, victim, EFFECT, 1.0);
    strike(&mut app, second, victim, EFFECT, 2.0);
    presses(&mut app);
    let credit = crate::extension::blast::credit_of(&SimId::placement("second"));
    assert_eq!(seen(&app, victim), Some(Seen { striker: credit, tag: 2.0 }), "the body kept another strike's word");
    assert!(seen(&app, second).is_none() && seen(&app, first).is_none(), "a strike was said on its striker");
    assert!(struck_holds(&app, victim), "a status held in hit_reaction was not held");
}

/// A striker with no simulation identity is credited `0`, and an effect no
/// module binds says nothing.
#[test]
fn a_strike_names_no_credit_for_a_striker_with_no_identity_and_none_for_an_unbound_effect() {
    let mut app = app();
    let anonymous = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    let victim = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    let other = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    strike(&mut app, anonymous, victim, EFFECT, 1.0);
    strike(&mut app, anonymous, other, "test.unbound", 1.0);
    presses(&mut app);
    assert_eq!(seen(&app, victim), Some(Seen { striker: 0, tag: 1.0 }));
    assert!(seen(&app, other).is_none(), "an unbound effect was said");
}
