//! Mark / Recall: while the body holds the mark/recall item, Attack puts its
//! mark where it stands, and Blink takes it back to the mark at once and
//! strikes there. A press of both is a mark. Driven bodies only. Migrated
//! from the native system (fast-iteration I4, body motion).
//!
//! The mark is the world's fact (`PlayerMark`): the beacon visual, the
//! session reset and the simulation view read it. The module asks to set it
//! (`ambition.items.set_mark`) and reads it (`ambition.items.mark`, which
//! gives no mark of another live room).

use ambition_combat_port::{
    BodySound, BodySoundPort, Destination, Effect, EffectPort, MarkPort, Place, SetMark, SetMarkPort, Strike,
    StrikePort, StrikeVolume, Transit, TransitPort, WieldedAlternatePort, WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{
    phases::WIELDED_USE, CodeIdentity, EntryCode, EntryDescriptor, EntryFn, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, PortKey, TriggerBinding, API_VERSION,
};

/// The held item's id.
pub const ITEM: &str = "mark_recall";

/// The recall strike: lure enemies onto the mark, and recall in to hit them.
const STRIKE_RADIUS: f32 = 36.0;
const STRIKE_DAMAGE: i32 = 2;
const EFFECT: &str = "classic_burst";

fn entry(key: &'static str, trigger: PortKey, reads: Vec<PortKey>, requests: Vec<PortKey>, run: EntryFn) -> EntryDescriptor {
    EntryDescriptor {
        key: key.into(),
        phase: WIELDED_USE,
        trigger: TriggerBinding {
            port: trigger,
            selector: ITEM.into(),
        },
        reads,
        writes: Vec::new(),
        after: Vec::new(),
        limits: Limits { max_requests: requests.len() as u32 },
        requests,
        // Not pressed this tick: nothing to do, and no state to reset.
        on_idle: IdlePolicy::ResetState,
        run: EntryCode::Native(run),
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "mark_recall"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![
            entry(
                "mark",
                WieldedUsePort::KEY,
                Vec::new(),
                vec![SetMarkPort::KEY, BodySoundPort::KEY, EffectPort::KEY],
                mark,
            ),
            entry(
                "recall",
                WieldedAlternatePort::KEY,
                vec![MarkPort::KEY],
                vec![TransitPort::KEY, StrikePort::KEY, BodySoundPort::KEY, EffectPort::KEY],
                recall,
            ),
        ],
    }
}

fn mark(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven {
        return Ok(());
    }
    let here = Place::World(w.position);
    inv.submit::<SetMarkPort>(SetMark { at: here })?;
    inv.submit::<BodySoundPort>(BodySound {
        cue: "player.dash".into(),
        at: here,
    })?;
    inv.submit::<EffectPort>(Effect {
        at: here,
        fx: EFFECT.into(),
        scale: 0.4,
    })
}

fn recall(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedAlternatePort>()?.clone();
    // A press of both is a mark, not a recall.
    if !w.driven || w.pressed {
        return Ok(());
    }
    let Some(at) = inv.observe::<MarkPort>()?.at else {
        return Ok(());
    };
    let there = Place::World(at);
    inv.submit::<TransitPort>(Transit {
        to: Destination::To(at),
        facing: None,
    })?;
    inv.submit::<StrikePort>(Strike {
        volume: StrikeVolume::Circle { at: there, radius: STRIKE_RADIUS },
        damage: STRIKE_DAMAGE,
        knockback: None,
    })?;
    inv.submit::<BodySoundPort>(BodySound {
        cue: "player.blink".into(),
        at: there,
    })?;
    inv.submit::<EffectPort>(Effect {
        at: there,
        fx: EFFECT.into(),
        scale: 0.6,
    })
}
