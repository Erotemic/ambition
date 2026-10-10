//! What the wielded abilities share: the descriptor of a stateless entry on
//! the `wielded_use` trigger, and the payment rule.

use ambition_combat_port::{BodySound, BodySoundPort, SpendMana, SpendManaPort, WieldedUsePort, Wielder};
use ambition_extension_sdk::{
    phases::WIELDED_USE, CodeIdentity, EntryCode, EntryDescriptor, EntryFn, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, PortKey, TriggerBinding, API_VERSION,
};

/// A module with one stateless entry, `use`, bound to the held item `item`.
pub fn module(key: &'static str, item: &'static str, requests: Vec<PortKey>, max_requests: u32, run: EntryFn) -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, key),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: "use".into(),
            phase: WIELDED_USE,
            trigger: TriggerBinding {
                port: WieldedUsePort::KEY,
                selector: item.into(),
            },
            reads: Vec::new(),
            writes: Vec::new(),
            requests,
            after: Vec::new(),
            limits: Limits { max_requests },
            // Not used this tick: nothing to do, and no state to reset.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(run),
        }],
    }
}

/// The ports every wielded ability asks for, plus its own.
pub fn requests(own: PortKey) -> Vec<PortKey> {
    vec![SpendManaPort::KEY, own, BodySoundPort::KEY]
}

/// Pay `cost` mana when the body can. `false`: the body cannot, and nothing
/// was asked.
pub fn pay(inv: &mut Invocation<'_>, wielder: &Wielder, cost: f32) -> Result<bool, Fault> {
    if !wielder.can_pay_mana(cost) {
        return Ok(false);
    }
    inv.submit::<SpendManaPort>(SpendMana { amount: cost })?;
    Ok(true)
}

/// The rock-hit cue every ranged wielded ability plays at the body.
pub fn rock_hit(inv: &mut Invocation<'_>, wielder: &Wielder) -> Result<(), Fault> {
    inv.submit::<BodySoundPort>(BodySound {
        cue: "world.rock.hit".into(),
        at: ambition_combat_port::Place::World(wielder.position),
    })
}
