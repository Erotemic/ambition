//! The "lock the target during the telegraph, fire once per strike" rule that
//! several boss techniques share.
//!
//! Each tick, for one boss:
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

/// The record schema for one technique's locked strike.
pub fn schema(key: SchemaKey) -> StateSchema {
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

/// Advance the rule for this tick. `Some(point)` means: fire now, at `point`.
pub fn advance(
    inv: &mut Invocation<'_>,
    key: &SchemaKey,
    caster: &BossCaster,
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
