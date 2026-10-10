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

/// `v` with length one, or zero when it has no direction. The same
/// arithmetic as the engine's `normalize_or_zero`: a multiply by the
/// reciprocal of the length.
pub fn unit_or_zero(v: [f32; 2]) -> [f32; 2] {
    let recip = 1.0 / (v[0] * v[0] + v[1] * v[1]).sqrt();
    if recip.is_finite() && recip > 0.0 {
        [v[0] * recip, v[1] * recip]
    } else {
        [0.0, 0.0]
    }
}
