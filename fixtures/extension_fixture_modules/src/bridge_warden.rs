//! The bridge warden's methods as a portable module: the same composition
//! as the Rhai warden, so a test can bind either. It counts each defeat it
//! hears in the world and, once the world's own count says two, lowers the
//! bridge with an expected revision, hearing every outcome.

use ambition_extension_sdk::{action::{ActionResultPort, ReplyTo}, phases::ACTOR_THINK, record,
    script::ScriptValue, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION};
use ambition_world_port::behavior::{AddWorldCount, AddWorldCountPort, ObjectDefeatedPort, SetWorldValue,
    SetWorldValuePort, WorldValuesPort};

record! {
    pub struct Ward = SchemaKey::new("fixture", "bridge_warden.ward", 1);
    1 downs: u32,
    2 asked: bool,
    3 log: String [max 256],
}

const COUNT: &str = "bridge.posts_down";

pub fn module() -> ModuleDescriptor {
    let entry = |key: &'static str, trigger, run, reads: Vec<_>, requests: Vec<_>, replies: Vec<&'static str>| EntryDescriptor {
        key: key.into(), phase: ACTOR_THINK, trigger: TriggerBinding {port: trigger, selector: key.into()},
        reads, writes: vec![Ward::KEY], requests, after: Vec::new(),
        replies: replies.into_iter().map(Into::into).collect(),
        limits: Limits {max_requests: 1}, on_idle: IdlePolicy::Invoke, run: EntryCode::Native(run),
    };
    ModuleDescriptor {key: ModuleKey::new("fixture", "bridge_warden"), api: API_VERSION,
        code: CodeIdentity::StaticNative {build: None, crate_name: env!("CARGO_PKG_NAME").into(), version: env!("CARGO_PKG_VERSION").into()},
        schemas: vec![Ward::schema()], entries: vec![
            // A subscribed method's selector names the native fact it hears.
            EntryDescriptor {trigger: TriggerBinding {port: ObjectDefeatedPort::KEY, selector: "defeated".into()},
                ..entry("down", ObjectDefeatedPort::KEY, down, Vec::new(), vec![AddWorldCountPort::KEY],
                    vec!["fixture::bridge_warden/counted"])},
            entry("counted", ActionResultPort::KEY, counted, vec![WorldValuesPort::KEY], vec![SetWorldValuePort::KEY],
                vec!["fixture::bridge_warden/lowered"]),
            entry("lowered", ActionResultPort::KEY, lowered, Vec::new(), Vec::new(), Vec::new()),
        ]}
}

fn down(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    inv.trigger::<ObjectDefeatedPort>()?;
    let mut ward = Ward::load(inv)?;
    ward.downs += 1;
    ward.log.push_str("down;");
    inv.submit::<AddWorldCountPort>(AddWorldCount {key: COUNT.into(), delta: 1,
        reply: ReplyTo {request: ward.downs, selector: "fixture::bridge_warden/counted".into()}})?;
    ward.store(inv)
}

fn counted(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let result = inv.trigger::<ActionResultPort>()?.clone();
    if !result.terminal { return Ok(()); }
    let mut ward = Ward::load(inv)?;
    ward.log.push_str(&result.code);
    ward.log.push(';');
    // Lower on the count the world holds, not this module's own tally.
    let count = inv.observe::<WorldValuesPort>()?.values.iter().find(|row| row.key == COUNT)
        .and_then(|row| match row.value { ScriptValue::Int(count) => Some(count), _ => None }).unwrap_or(0);
    if !ward.asked && count >= 2 {
        ward.asked = true;
        inv.submit::<SetWorldValuePort>(SetWorldValue {key: "bridge.state".into(),
            value: ScriptValue::Text("lowered".into()), expect: Some(0),
            reply: ReplyTo {request: 1, selector: "fixture::bridge_warden/lowered".into()}})?;
    }
    ward.store(inv)
}

fn lowered(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let result = inv.trigger::<ActionResultPort>()?.clone();
    if !result.terminal { return Ok(()); }
    let mut ward = Ward::load(inv)?;
    ward.log.push_str(&result.code);
    ward.log.push(';');
    ward.store(inv)
}
