//! The homing dash: the fighter is carried at whoever they were pointing at.
//!
//! When a move authors `smash.homing_dash`, its fighter is carried for the
//! move's time at the move's speed. The cone opens the way the fighter faces
//! when the move fires (as for every body-local `offset`), and is remembered:
//! turning mid-dash does not sweep it. Each tick the heading bends toward the
//! nearest foe inside the cone and the move's range, asked again each tick,
//! so a foe who leaves the cone stops attracting it and the move can be
//! dodged. Who is a foe is the engine's answer (`ambition.combat.foes`); the
//! cone, the range and the nearest are this module's.
//!
//! The dash is a status on the fighter ([`HOMING`]) with its clock and its
//! committed direction in a record ([`Dash`]). The tick that starts the dash
//! carries it and spends a tick of it. The velocity is set, not added
//! (`ambition.motion.velocity`): a running start does not make it faster. A
//! newer dash replaces the one the fighter has.
//!
//! Each number is the move's (`params` of its effect), and admission checked
//! that they hydrate, so a missing one is a fault and not a dash of nothing.

use ambition_combat_port::{
    FighterSpecial, FighterSpecialPort, Foe, FoesPort, HoldStatus, HoldStatusPort, SetVelocity, SetVelocityPort,
    StatusTickPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for the dash. The Smash crate installs this
/// module for its catalog's key (`install_module_technique`).
pub const HOMING_DASH: &str = "smash.homing_dash";

/// The status a fighter holds while a dash carries it.
pub const HOMING: &str = "smash.homing";

/// The entry that carries a dash each tick.
const TICK: &str = "smash.homing_tick";

record! {
    /// A dash in progress: its clock, its speed, its cone and the direction
    /// it committed to.
    pub struct Dash = SchemaKey::new(crate::PROVIDER, "homing.dash", 1);
    /// World seconds of homing left.
    1 remaining_s: f32,
    2 speed: f32,
    /// The cone, in radians either side of the committed direction.
    3 cone_rad: f32,
    /// How far a foe may be and still attract it.
    4 max_range: f32,
    /// The committed direction, remembered from the press.
    5 commanded: [f32; 2],
}

pub fn module() -> ModuleDescriptor {
    let key = ModuleKey::new(crate::PROVIDER, "homing");
    let tick_path = format!("{key}/{TICK}");
    let entry = |name: &'static str, port, selector: &'static str, after: Vec<String>, run| EntryDescriptor {
        key: name.into(),
        phase: TECHNIQUE_EXECUTION,
        trigger: TriggerBinding { port, selector: selector.into() },
        reads: vec![FoesPort::KEY],
        writes: vec![Dash::KEY],
        requests: vec![SetVelocityPort::KEY, HoldStatusPort::KEY],
        after: after.into_iter().map(Into::into).collect(),
        limits: Limits { max_requests: 2 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    ModuleDescriptor {
        key,
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Dash::schema()],
        entries: vec![
            entry(TICK, StatusTickPort::KEY, HOMING, Vec::new(), carry),
            // After the carry: a dash pressed while one carries replaces it
            // and is not spent twice on that tick.
            entry(HOMING_DASH, FighterSpecialPort::KEY, HOMING_DASH, vec![tick_path], dash),
        ],
    }
}

/// The dash a move's params ask for, committed to the fighter's facing.
pub fn dash_of(special: &FighterSpecial) -> Result<Dash, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    Ok(Dash {
        remaining_s: number("duration_s")?,
        speed: number("speed")?,
        cone_rad: number("cone_degrees")?.to_radians(),
        max_range: number("max_range")?,
        commanded: [special.facing, 0.0],
    })
}

/// The heading from `from`: the committed direction, bent toward the nearest
/// foe inside the cone and the range. Of foes at one distance, the first
/// (the lowest identity) wins. With none, the committed direction.
pub fn heading(from: [f32; 2], dash: &Dash, foes: &[Foe]) -> [f32; 2] {
    let length = (dash.commanded[0] * dash.commanded[0] + dash.commanded[1] * dash.commanded[1]).sqrt();
    if length <= 0.0 {
        return [0.0, 0.0];
    }
    let commanded = [dash.commanded[0] / length, dash.commanded[1] / length];
    let cos_limit = dash.cone_rad.cos();
    let mut best: Option<(f32, [f32; 2])> = None;
    for foe in foes {
        let offset = [foe.position[0] - from[0], foe.position[1] - from[1]];
        let distance = (offset[0] * offset[0] + offset[1] * offset[1]).sqrt();
        if distance <= f32::EPSILON || distance > dash.max_range {
            continue;
        }
        let toward = [offset[0] / distance, offset[1] / distance];
        if toward[0] * commanded[0] + toward[1] * commanded[1] < cos_limit {
            continue;
        }
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, toward));
        }
    }
    best.map_or(commanded, |(_, toward)| toward)
}

fn status(held: bool) -> HoldStatus {
    HoldStatus { kind: HOMING.into(), held }
}

/// Spend a tick of the dash; carry the fighter while it lasts, and end it
/// when it runs out.
fn advance(inv: &mut Invocation<'_>, from: [f32; 2], mut dash: Dash) -> Result<(), Fault> {
    dash.remaining_s -= inv.dt();
    if dash.remaining_s <= 0.0 {
        Dash::default().store(inv)?;
        return inv.submit::<HoldStatusPort>(status(false));
    }
    let toward = heading(from, &dash, &inv.observe::<FoesPort>()?.foes);
    dash.store(inv)?;
    inv.submit::<SetVelocityPort>(SetVelocity { velocity: [toward[0] * dash.speed, toward[1] * dash.speed] })?;
    inv.submit::<HoldStatusPort>(status(true))
}

/// Start a dash, or replace the one the fighter has. It carries on this tick.
fn dash(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let dash = dash_of(&special)?;
    advance(inv, special.position, dash)
}

/// One tick of a dash in progress.
fn carry(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let from = inv.trigger::<StatusTickPort>()?.position;
    let dash = Dash::load(inv)?;
    advance(inv, from, dash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn dash() -> Dash {
        Dash { remaining_s: 0.28, speed: 900.0, cone_rad: 60f32.to_radians(), max_range: 320.0, commanded: [1.0, 0.0] }
    }

    /// The dash is the move's, opening the way the fighter faces; one with no
    /// speed is a fault.
    #[test]
    fn the_dash_is_what_the_move_authors() {
        let special = |params: &[(&str, f32)]| FighterSpecial {
            position: [0.0, 0.0],
            facing: -1.0,
            params: params.iter().map(|(path, n)| (path.to_string(), SpecialParam::Number(*n))).collect(),
        };
        let mut params = vec![("duration_s", 0.28), ("speed", 900.0), ("cone_degrees", 60.0), ("max_range", 320.0)];
        assert_eq!(dash_of(&special(&params)).unwrap(), Dash { commanded: [-1.0, 0.0], ..dash() });
        params.remove(1);
        assert!(dash_of(&special(&params)).is_err(), "a dash with no speed is a fault");
    }

    /// The nearest foe in the cone wins; one behind, one too far and none
    /// leave the committed direction; of two at one distance the first wins.
    #[test]
    fn the_heading_bends_to_the_nearest_foe_in_the_cone() {
        let foe = |x: f32, y: f32| Foe { position: [x, y] };
        let up = heading([0.0, 0.0], &dash(), &[foe(300.0, 0.0), foe(100.0, -100.0)]);
        assert!(up[1] < -0.5, "the nearer foe in the cone did not win: {up:?}");
        assert_eq!(heading([0.0, 0.0], &dash(), &[foe(-100.0, 0.0)]), [1.0, 0.0], "a foe behind bent it");
        assert_eq!(heading([0.0, 0.0], &dash(), &[foe(900.0, 0.0)]), [1.0, 0.0], "a foe too far bent it");
        assert_eq!(heading([0.0, 0.0], &dash(), &[]), [1.0, 0.0]);
        let tie = heading([0.0, 0.0], &dash(), &[foe(100.0, -100.0), foe(100.0, 100.0)]);
        assert!(tie[1] < 0.0, "of two foes at one distance, the second won: {tie:?}");
    }
}
