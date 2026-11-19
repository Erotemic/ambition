//! Portable reusable behavior with native actions and explicit call completion.

use ambition_extension_sdk::{action::{ActionResultPort, ActionStage, ReplyTo}, phases::ACTOR_THINK,
    record, script::ScriptValue, structure::Field, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy,
    Invocation, Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION};
use ambition_world_port::{behavior::{BehaviorContext, BehaviorContextPort, CompleteObjectCall,
    CompleteObjectCallPort, ObjectCallPort}, inventory::{InventoryExchange, InventoryExchangePort, InventoryTerms}};

record! {
    pub struct Memory = SchemaKey::new("fixture", "object_worker.memory", 1);
    1 requests: u32,
    2 pending: u64,
    3 applied: u32,
    4 refused: u32,
    5 busy: bool,
    6 completions: u32,
}

pub fn module() -> ModuleDescriptor {
    let entry = |key: &'static str, trigger, run, requests: Vec<_>, replies: Vec<_>| EntryDescriptor {
        replies, key: key.into(), phase: ACTOR_THINK,
        trigger: TriggerBinding {port: trigger, selector: key.into()},
        reads: vec![BehaviorContextPort::KEY], writes: vec![Memory::KEY], requests,
        after: Vec::new(), limits: Limits {max_requests: 1}, on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    ModuleDescriptor {key: ModuleKey::new("fixture", "object_worker"), api: API_VERSION,
        code: CodeIdentity::StaticNative {build: None, crate_name: env!("CARGO_PKG_NAME").into(), version: env!("CARGO_PKG_VERSION").into()},
        schemas: vec![Memory::schema()], entries: vec![
            entry("work", ObjectCallPort::KEY, work, vec![InventoryExchangePort::KEY, CompleteObjectCallPort::KEY],
                vec!["fixture::object_worker/trade".into(), "fixture::object_worker/ack".into()]),
            entry("trade", ActionResultPort::KEY, trade, vec![CompleteObjectCallPort::KEY], vec!["fixture::object_worker/ack".into()]),
            entry("ack", ActionResultPort::KEY, ack, Vec::new(), Vec::new()),
        ]}
}
fn context(inv: &Invocation<'_>) -> Result<BehaviorContext, Fault> {
    inv.observe::<BehaviorContextPort>()?.clone().ok_or_else(|| Fault::Module("missing behavior context".into()))
}
fn param<T: Field<T>>(context: &BehaviorContext, key: &str) -> Result<T, Fault> {
    context.params.field(key).and_then(T::from_script).map_err(|error| Fault::Module(error.into()))
}
fn finish(inv: &mut Invocation<'_>, state: &mut Memory, sequence: u64, accepted: bool, code: String) -> Result<(), Fault> {
    state.completions += 1;
    let context = context(inv)?;
    let data = ScriptValue::map([("binding", ScriptValue::Text(context.binding)),
        ("object", <ambition_extension_sdk::ObjectRef as Field<_>>::to_script(&context.object))]);
    inv.submit::<CompleteObjectCallPort>(CompleteObjectCall {sequence, accepted, code, data,
        reply: ReplyTo {request: state.completions, selector: "fixture::object_worker/ack".into()}})
}
fn work(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let call = inv.trigger::<ObjectCallPort>()?.clone();
    let mut state = Memory::load(inv)?;
    if state.busy {
        finish(inv, &mut state, call.sequence, false, "busy".into())?;
        return state.store(inv);
    }
    let context = context(inv)?;
    for (key, reference) in &context.references {
        let expected = match key.as_str() { "owner" => context.object, "peer" => call.caller, _ => continue };
        if *reference != expected { return Err(Fault::Module("a bound reference names another occurrence".into())); }
    }
    state.busy = true;
    state.requests += 1;
    state.pending = call.sequence;
    inv.submit::<InventoryExchangePort>(InventoryExchange {
        reply: ReplyTo {request: state.requests, selector: "fixture::object_worker/trade".into()},
        exchange: InventoryTerms {pay_coins: param(&context, "pay")?, receive_coins: param(&context, "receive")?, ..Default::default()},
    })?;
    state.store(inv)?;
    if param::<bool>(&context, "fail")? { return Err(Fault::Module("deliberate staged action fault".into())); }
    Ok(())
}
fn trade(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let result = inv.trigger::<ActionResultPort>()?.clone();
    if !result.terminal { return Ok(()); }
    if context(inv)?.binding == "credit" && result.stage != ActionStage::Applied {
        return Err(Fault::Module(result.code.into()));
    }
    let mut state = Memory::load(inv)?;
    if result.stage == ActionStage::Applied { state.applied += 1; } else { state.refused += 1; }
    state.busy = false;
    let sequence = state.pending;
    finish(inv, &mut state, sequence, result.stage == ActionStage::Applied, result.code)?;
    state.store(inv)
}
fn ack(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let result = inv.trigger::<ActionResultPort>()?;
    if result.stage == ActionStage::Rejected { return Err(Fault::Module(result.code.clone().into())); }
    Ok(())
}
