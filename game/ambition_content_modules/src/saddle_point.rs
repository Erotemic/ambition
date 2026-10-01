//! Saddle point: on the first strike tick a damage arm appears across the
//! boss, horizontal; every period the arm turns (vertical, then horizontal
//! again), each new arm where the boss is at that moment. The arm goes when
//! the strike ends. Migrated from the native system (fast-iteration I4/I7):
//! the arm is a HELD box, so the module names a slot and a generation and the
//! combat domain owns the entity.

use ambition_boss_special_port::{BossCaster, BossSpecialCast};
use ambition_combat_port::{HeldDamageBox, HeldDamageBoxPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, Attachment, CodeIdentity, EntryCode, EntryDescriptor, Fault, FieldDecl,
    FieldKind, FieldRef, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey, Port,
    SaveEligibility, SchemaKey, StateSchema, TriggerBinding, Value, API_VERSION,
};

/// The special-action key in `boss_profiles.ron`.
pub const KEY: &str = "saddle_point";

const ARM_LENGTH: f32 = 220.0;
const ARM_THICKNESS: f32 = 36.0;
const AXIS_PERIOD_S: f32 = 1.2;
const DAMAGE: i32 = 2;
const KNOCKBACK: f32 = 1.6;
/// The module's one held box.
const ARM: u32 = 0;

pub const CROSS: SchemaKey = SchemaKey::new(crate::PROVIDER, "saddle_point.cross", 1);
const STRIKE_ACTIVE: FieldRef = FieldRef(0);
const AXIS_HORIZONTAL: FieldRef = FieldRef(1);
const AXIS_REMAINING_S: FieldRef = FieldRef(2);
const GENERATION: FieldRef = FieldRef(3);
const ARM_CENTER: FieldRef = FieldRef(4);
const HOLDING: FieldRef = FieldRef(5);

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "saddle_point"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![StateSchema {
            key: CROSS,
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![
                FieldDecl::new(1, "strike_active", FieldKind::Bool),
                FieldDecl::new(2, "axis_horizontal", FieldKind::Bool),
                // Gameplay seconds until the arm turns.
                FieldDecl::new(3, "axis_remaining_s", FieldKind::F32),
                // The arm's generation: each new arm is the next one.
                FieldDecl::new(4, "generation", FieldKind::U32),
                // Where the current arm was put. It does not follow the boss.
                FieldDecl::new(5, "arm_center", FieldKind::Vec2),
                // False after the boss died: no arm until the next turn.
                FieldDecl::new(6, "holding", FieldKind::Bool),
            ],
        }],
        entries: vec![EntryDescriptor {
            key: "cross".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding {
                port: BossSpecialCast::KEY,
                selector: KEY.into(),
            },
            reads: Vec::new(),
            writes: vec![CROSS],
            requests: vec![HeldDamageBoxPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            // An idle tick ends the strike: the records go back to initial and,
            // with nothing submitted, the combat domain takes the arm away.
            on_idle: IdlePolicy::ResetState,
            run: EntryCode::Native(cross),
        }],
    }
}

struct Cross {
    strike_active: bool,
    axis_horizontal: bool,
    axis_remaining_s: f32,
    generation: u32,
    arm_center: [f32; 2],
    holding: bool,
}

fn cross(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let caster: BossCaster = inv.trigger::<BossSpecialCast>()?.clone();
    let dt = inv.dt();
    let schema_fault = |error| Fault::Schema { schema: CROSS, error };
    let record = inv.state(&CROSS)?;
    let field = |f| record.get(f).map_err(schema_fault);
    let mut c = Cross {
        strike_active: field(STRIKE_ACTIVE)?.as_bool().unwrap_or(false),
        axis_horizontal: field(AXIS_HORIZONTAL)?.as_bool().unwrap_or(false),
        axis_remaining_s: field(AXIS_REMAINING_S)?.as_f32().unwrap_or(0.0),
        generation: field(GENERATION)?.as_u32().unwrap_or(0),
        arm_center: field(ARM_CENTER)?.as_vec2().unwrap_or([0.0, 0.0]),
        holding: field(HOLDING)?.as_bool().unwrap_or(false),
    };
    let period = AXIS_PERIOD_S.max(0.05);
    if !caster.pressed {
        // Not pressing (a telegraph): no strike, no arm.
        c.strike_active = false;
        c.axis_remaining_s = 0.0;
        c.holding = false;
    } else if !caster.alive {
        // A dead boss holds no arm; the strike is not reset.
        c.holding = false;
    } else if !c.strike_active {
        c.axis_horizontal = true;
        c.axis_remaining_s = period;
        c.strike_active = true;
        c.generation = c.generation.wrapping_add(1);
        c.arm_center = caster.position;
        c.holding = true;
    } else {
        c.axis_remaining_s = (c.axis_remaining_s - dt).max(0.0);
        if c.axis_remaining_s <= 0.0 {
            c.axis_horizontal = !c.axis_horizontal;
            c.axis_remaining_s = period;
            c.generation = c.generation.wrapping_add(1);
            c.arm_center = caster.position;
            c.holding = true;
        }
    }

    let record = inv.state(&CROSS)?;
    record.set(STRIKE_ACTIVE, Value::Bool(c.strike_active)).map_err(schema_fault)?;
    record.set(AXIS_HORIZONTAL, Value::Bool(c.axis_horizontal)).map_err(schema_fault)?;
    record.set(AXIS_REMAINING_S, Value::F32(c.axis_remaining_s)).map_err(schema_fault)?;
    record.set(GENERATION, Value::U32(c.generation)).map_err(schema_fault)?;
    record.set(ARM_CENTER, Value::Vec2(c.arm_center)).map_err(schema_fault)?;
    record.set(HOLDING, Value::Bool(c.holding)).map_err(schema_fault)?;

    if c.holding {
        let half_extent = if c.axis_horizontal {
            [ARM_LENGTH, ARM_THICKNESS]
        } else {
            [ARM_THICKNESS, ARM_LENGTH]
        };
        inv.submit::<HeldDamageBoxPort>(HeldDamageBox {
            slot: ARM,
            generation: c.generation,
            center: c.arm_center,
            half_extent,
            damage: DAMAGE,
            knockback: KNOCKBACK,
            lifetime_s: period * 2.0,
        })?;
    }
    Ok(())
}
