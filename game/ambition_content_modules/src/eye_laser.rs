//! Eye Laser: while a body holds the slot its worn eye laser takes, a wide
//! straight laser leaves its whole visor along its facing, as far as the first
//! wall. It costs
//! mana for each tick it burns, and it hits a body it touches again on a fixed
//! beat. While it burns, the body is drawn in its `eye_laser` row (its `shoot`
//! row on a sheet without one).
//!
//! The technique is the Projectile slot of a body that wears the `eye_laser`
//! row: the inventory's Eye Laser ability (`items.ron`).

use ambition_boss_special_port::{DrawnRow, DrawnRowPort};
use ambition_combat_port::{
    BodySound, BodySoundPort, HeldRay, HeldRayPort, Place, SpendMana, SpendManaPort, Spot, TechniqueUse,
    TechniqueUsePort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The technique id the worn row puts on the Projectile slot.
pub const TECHNIQUE: &str = "eye_laser";

/// Mana per second of burn (of 100): a full bar burns for a little over three
/// seconds.
pub const MANA_PER_S: f32 = 30.0;
/// How far the laser reaches when no wall stops it.
pub const REACH: f32 = 520.0;
/// The ray is as thick as the visor's red fill, as a part of the body's
/// height.
pub const VISOR_HEIGHT: f32 = 0.29;
const DAMAGE: i32 = 1;
/// A light push, a multiple of the victim's feel-tuned launch.
const KNOCKBACK: f32 = 0.3;
/// A body the laser touches is hit again after this long.
pub const REHIT_S: f32 = 0.2;
/// The ray is asked for again each tick it burns; this only outlives one tick.
const RAY_LIFETIME_S: f32 = 0.1;
/// The front of the visor at its middle, body-local, as a part of the body's
/// size from its middle: ahead (along its facing) and up. Measured from the
/// robot's `eye_laser` row (sprite renderer `eefd4b0`).
pub const VISOR: [f32; 2] = [0.52, -0.159];
/// The presentation's ray look.
const LOOK: &str = "laser";
/// The rows the body is drawn with while it burns, in order of preference.
const ROWS: [&str; 2] = ["eye_laser", "shoot"];
const SFX_IGNITE: &str = "boss.wolfenstein.zap";
/// The ray's slot: one laser per body.
const RAY_SLOT: u32 = 0;

record! {
    /// What the laser keeps between ticks.
    pub struct Burn = SchemaKey::new(crate::PROVIDER, "eye_laser.burn", 1);
    /// The laser burns.
    1 burning: bool,
    /// A new generation is a new ray, and a new ray hits a body again.
    2 generation: u32,
    /// Seconds since the generation changed.
    3 since_hit: f32,
    /// Seconds since the laser began to burn: the drawn row's clock.
    4 burnt: f32,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "eye_laser"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Burn::schema()],
        entries: vec![EntryDescriptor {
            key: "burn".into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: TechniqueUsePort::KEY, selector: TECHNIQUE.into() },
            reads: Vec::new(),
            writes: vec![Burn::KEY],
            requests: vec![SpendManaPort::KEY, HeldRayPort::KEY, DrawnRowPort::KEY, BodySoundPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 4 },
            // An idle tick must run: the tick after the laser stops is the one
            // that gives the body its own row back.
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(burn),
        }],
    }
}

/// What one tick of the laser asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BurnTick {
    /// The mana this tick costs, when it burns.
    pub cost: Option<f32>,
    /// The ray this tick, when it burns.
    pub ray: Option<HeldRay>,
    /// The rows to draw the body with: `Some(empty)` gives it its own row back.
    pub row: Option<DrawnRow>,
    /// The laser began to burn this tick.
    pub ignited: bool,
}

/// One tick of the laser for a body that uses it as `used`, `dt` seconds
/// after the last. `s` is the burn so far; it is advanced.
pub fn burn_tick_of(s: &mut Burn, used: &TechniqueUse, dt: f32) -> BurnTick {
    let user = &used.user;
    let cost = MANA_PER_S * dt;
    let wants = user.pressed || used.held;
    let mut out = BurnTick::default();
    if !wants || !user.can_pay_mana(cost) {
        if s.burning {
            *s = Burn { generation: s.generation, ..Burn::default() };
            out.row = Some(DrawnRow::body_as_engine());
        }
        return out;
    }
    if s.burning {
        s.since_hit += dt;
        s.burnt += dt;
        if s.since_hit >= REHIT_S {
            s.since_hit -= REHIT_S;
            s.generation = s.generation.wrapping_add(1);
        }
    } else {
        s.burning = true;
        s.since_hit = 0.0;
        s.burnt = 0.0;
        s.generation = s.generation.wrapping_add(1);
        out.ignited = true;
    }
    let side = if user.facing < 0.0 { -1.0 } else { 1.0 };
    let visor = user.to_world([side * VISOR[0] * user.size[0], VISOR[1] * user.size[1]]);
    out.cost = Some(cost);
    out.ray = Some(HeldRay {
        slot: RAY_SLOT,
        generation: s.generation,
        from: Spot::Body(visor),
        toward: user.to_world([side, 0.0]),
        reach: REACH,
        width: VISOR_HEIGHT * user.size[1],
        damage: DAMAGE,
        knockback: KNOCKBACK,
        launch_dir: None,
        lifetime_s: RAY_LIFETIME_S,
        look: LOOK.into(),
    });
    out.row = Some(DrawnRow {
        on: ambition_boss_special_port::DrawnPart::Body,
        names: ROWS.iter().map(|name| (*name).to_owned()).collect(),
        elapsed: s.burnt,
        looping: true,
    });
    out
}

fn burn(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let used: TechniqueUse = inv.trigger::<TechniqueUsePort>()?.clone();
    let dt = inv.dt();
    let mut s = Burn::load(inv)?;
    let tick = burn_tick_of(&mut s, &used, dt);
    s.store(inv)?;
    if let Some(amount) = tick.cost {
        inv.submit::<SpendManaPort>(SpendMana { amount })?;
    }
    if let Some(ray) = tick.ray {
        inv.submit::<HeldRayPort>(ray)?;
    }
    if let Some(row) = tick.row {
        inv.submit::<DrawnRowPort>(row)?;
    }
    if tick.ignited {
        inv.submit::<BodySoundPort>(BodySound { cue: SFX_IGNITE.into(), at: Place::Body })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::Wielder;

    fn used(pressed: bool, held: bool, facing: f32, mana: f32) -> TechniqueUse {
        TechniqueUse {
            held,
            released: false,
            user: Wielder {
                pressed,
                driven: true,
                position: [100.0, 200.0],
                size: [28.0, 32.0],
                facing,
                frame_side: [1.0, 0.0],
                frame_down: [0.0, 1.0],
                aim_local: [0.0, 0.0],
                mana: Some(mana),
                names_spawns: true,
                cooldown_ready: true,
                swept: true,
            },
        }
    }

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn the_laser_burns_straight_ahead_from_the_visor_while_held() {
        let mut s = Burn::default();
        let first = burn_tick_of(&mut s, &used(true, true, -1.0, 100.0), DT);
        assert!(first.ignited);
        let ray = first.ray.expect("a press burns");
        assert_eq!(ray.toward, [-1.0, 0.0], "straight along the facing, not the aim");
        let Spot::Body([x, y]) = ray.from else { panic!("the ray rides the body") };
        assert!(x < 0.0 && y < 0.0, "the visor is ahead and up: {x} {y}");
        assert!(ray.width > 8.0, "the ray is as wide as the visor: {}", ray.width);
        assert_eq!(first.cost, Some(MANA_PER_S * DT));
        assert_eq!(first.row.expect("a row").names[0], "eye_laser");

        let held = burn_tick_of(&mut s, &used(false, true, -1.0, 100.0), DT);
        assert!(!held.ignited && held.ray.is_some(), "held, it keeps burning");
    }

    #[test]
    fn a_body_it_touches_is_hit_again_on_a_beat() {
        let mut s = Burn::default();
        let first = burn_tick_of(&mut s, &used(true, true, 1.0, 100.0), DT).ray.unwrap().generation;
        let mut generations = vec![first];
        for _ in 0..30 {
            generations.push(burn_tick_of(&mut s, &used(false, true, 1.0, 100.0), DT).ray.unwrap().generation);
        }
        generations.dedup();
        // Half a second of burn at a 0.2 s beat: the first ray and two more.
        assert_eq!(generations.len(), 3, "{generations:?}");
    }

    #[test]
    fn letting_go_or_running_dry_ends_it_and_gives_the_row_back() {
        let mut s = Burn::default();
        burn_tick_of(&mut s, &used(true, true, 1.0, 100.0), DT);
        let off = burn_tick_of(&mut s, &used(false, false, 1.0, 100.0), DT);
        assert!(off.ray.is_none() && off.cost.is_none());
        assert_eq!(off.row, Some(DrawnRow::body_as_engine()));
        assert_eq!(burn_tick_of(&mut s, &used(false, false, 1.0, 100.0), DT), BurnTick::default(), "an idle tick asks nothing");

        burn_tick_of(&mut s, &used(true, true, 1.0, 100.0), DT);
        let dry = burn_tick_of(&mut s, &used(false, true, 1.0, 0.0), DT);
        assert!(dry.ray.is_none(), "no mana, no laser");
        assert_eq!(dry.row, Some(DrawnRow::body_as_engine()));
    }
}
