//! The strike rules that several boss techniques share.
//!
//! A STRIKE is a run of ticks on which the boss presses the technique's key.
//!
//! [`once`]: fire on the first tick of a strike the boss is alive; a tick
//! with no press ends the strike.
//!
//! [`once_numbered`]: [`once`], and each strike that fires gets the next
//! number, from 0. The number continues across strikes: it names what the
//! strike makes (a summon's id).
//!
//! [`locked`]: the same, and the target is locked during the telegraph. Each
//! tick, for one boss:
//!
//! * dead: forget the lock and the strike;
//! * telegraphing this technique: lock the tracked target once, if there is
//!   one, and arm a new strike;
//! * not pressing: forget the lock and the strike;
//! * pressing, and the strike has not fired: FIRE at the lock, else at the
//!   tracked target, else at the boss itself; then forget the lock.
//!
//! The record lives under each technique's OWN schema key, so two techniques
//! never share a lock.

use ambition_boss_special_port::BossCaster;
use ambition_extension_sdk::{
    Attachment, Fault, FieldDecl, FieldKind, FieldRef, Invocation, SaveEligibility, SchemaKey,
    StateSchema, Value,
};

const LOCKED_TARGET: FieldRef = FieldRef(0);
const FIRED_THIS_STRIKE: FieldRef = FieldRef(1);
const ONCE_FIRED: FieldRef = FieldRef(0);

/// The record schema for [`once`].
pub fn once_schema(key: SchemaKey) -> StateSchema {
    StateSchema {
        key,
        attachment: Attachment::Body,
        save: SaveEligibility::Transient,
        fields: vec![FieldDecl::new(1, "fired_this_strike", FieldKind::Bool)],
    }
}

/// Advance the once-per-strike rule. `true` means: fire now.
pub fn once(inv: &mut Invocation<'_>, key: &SchemaKey, caster: &BossCaster) -> Result<bool, Fault> {
    let record = inv.state(key)?;
    let schema_fault = |error| Fault::Schema { schema: key.clone(), error };
    if !caster.pressed {
        record.set(ONCE_FIRED, Value::Bool(false)).map_err(schema_fault)?;
        return Ok(false);
    }
    let fired = record.get(ONCE_FIRED).map_err(schema_fault)?.as_bool().unwrap_or(false);
    if !caster.alive || fired {
        return Ok(false);
    }
    record.set(ONCE_FIRED, Value::Bool(true)).map_err(schema_fault)?;
    Ok(true)
}

const NUMBERED_FIRED: FieldRef = FieldRef(0);
const NUMBERED_NEXT: FieldRef = FieldRef(1);

/// The record schema for [`once_numbered`].
pub fn once_numbered_schema(key: SchemaKey) -> StateSchema {
    StateSchema {
        key,
        attachment: Attachment::Body,
        save: SaveEligibility::Transient,
        fields: vec![
            FieldDecl::new(1, "fired_this_strike", FieldKind::Bool),
            FieldDecl::new(2, "next_number", FieldKind::U32),
        ],
    }
}

/// Advance the numbered once-per-strike rule. `Some(n)` means: fire now; this
/// is strike `n`. An entry that uses it declares `IdlePolicy::Invoke`: an
/// idle reset would forget the number.
pub fn once_numbered(
    inv: &mut Invocation<'_>,
    key: &SchemaKey,
    caster: &BossCaster,
) -> Result<Option<u32>, Fault> {
    let record = inv.state(key)?;
    let schema_fault = |error| Fault::Schema { schema: key.clone(), error };
    if !caster.pressed {
        record.set(NUMBERED_FIRED, Value::Bool(false)).map_err(schema_fault)?;
        return Ok(None);
    }
    let fired = record.get(NUMBERED_FIRED).map_err(schema_fault)?.as_bool().unwrap_or(false);
    if !caster.alive || fired {
        return Ok(None);
    }
    let number = record.get(NUMBERED_NEXT).map_err(schema_fault)?.as_u32().unwrap_or(0);
    record.set(NUMBERED_FIRED, Value::Bool(true)).map_err(schema_fault)?;
    record.set(NUMBERED_NEXT, Value::U32(number.wrapping_add(1))).map_err(schema_fault)?;
    Ok(Some(number))
}

/// The record schema for [`locked`].
pub fn locked_schema(key: SchemaKey) -> StateSchema {
    StateSchema {
        key,
        attachment: Attachment::Body,
        save: SaveEligibility::Transient,
        fields: vec![
            FieldDecl::new(1, "locked_target", FieldKind::opt(FieldKind::Vec2)),
            FieldDecl::new(2, "fired_this_strike", FieldKind::Bool),
        ],
    }
}

/// Advance the locked-strike rule. `Some(point)` means: fire now, at `point`.
pub fn locked(
    inv: &mut Invocation<'_>,
    key: &SchemaKey,
    caster: &BossCaster,
) -> Result<Option<[f32; 2]>, Fault> {
    locked_when(inv, key, caster, true)
}

/// [`locked`], for a technique that cannot always fire. When the strike
/// would fire and `can_fire` is false, nothing changes: the strike stays
/// armed and fires on a later tick of it.
pub fn locked_when(
    inv: &mut Invocation<'_>,
    key: &SchemaKey,
    caster: &BossCaster,
    can_fire: bool,
) -> Result<Option<[f32; 2]>, Fault> {
    let record = inv.state(key)?;
    let field = |f| record.get(f).map_err(|error| Fault::Schema { schema: key.clone(), error });
    let locked = field(LOCKED_TARGET)?
        .as_opt()
        .flatten()
        .and_then(Value::as_vec2);
    let fired = field(FIRED_THIS_STRIKE)?.as_bool().unwrap_or(false);

    let (locked, fired, fire_at) = if !caster.alive {
        (None, false, None)
    } else if caster.telegraphing {
        (locked.or(caster.target), false, None)
    } else if !caster.pressed {
        (None, false, None)
    } else if fired {
        (locked, true, None)
    } else if !can_fire {
        (locked, false, None)
    } else {
        let at = locked.or(caster.target).unwrap_or(caster.position);
        (None, true, Some(at))
    };

    let record = inv.state(key)?;
    let set = |record: &mut ambition_extension_sdk::Record, f, v| {
        record.set(f, v).map_err(|error| Fault::Schema { schema: key.clone(), error })
    };
    set(record, LOCKED_TARGET, Value::Opt(locked.map(|v| Box::new(Value::Vec2(v)))))?;
    set(record, FIRED_THIS_STRIKE, Value::Bool(fired))?;
    Ok(fire_at)
}
