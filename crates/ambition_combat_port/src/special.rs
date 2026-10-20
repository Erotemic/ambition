//! The ports of a fighter's special: what a special move does when its fighter
//! presses it, decided by a module in `technique_execution`
//! (`ambition_extension_sdk::phases::TECHNIQUE_EXECUTION`).
//!
//! A move authors an effect at a timeline event, a window's sustain slot or a
//! flow `Emit` node (`TechniqueDelivery::Action`). When the key of that effect
//! is bound by a module, the module decides what it does, from the fighter's
//! facts and the params the move authors. A game's special can then be a
//! module and not Rust in the demo crate.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a fighter's special.
///
/// Port card:
///
/// * **Operation** — a body pressed the special whose effect key is the
///   selector this tick. The selector is the key a move authors
///   (`Special("smash.riposte_strike")`).
/// * **Owner** — `ambition_combat::extension::special`, which reads
///   `ActorActionMessage::Special`.
/// * **Scope** — one invocation for each press: a body that presses one key
///   two times in one tick is invoked two times, in the order of the
///   presses. A body that pressed nothing is not invoked.
/// * **Time** — `technique_execution`. The body's kinematics are this tick's
///   settled values.
/// * **Read model** — world units, +Y down. `facing` is the sign of the
///   body's facing (`1.0` or `-1.0`). `params` are the move's authored params
///   for this effect, each leaf by the path an author reads (`reach`,
///   `half_extents[0]`, `hit_sfx`), a map's fields in the order of their
///   names (`ParamValue::leaves`). An absent optional states no leaf.
/// * **Replay** — the value is derived from this tick's messages; the port
///   keeps nothing between ticks.
pub struct FighterSpecialPort;

/// One authored value of a special's params.
#[derive(Clone, Debug, PartialEq)]
pub enum SpecialParam {
    Number(f32),
    Text(String),
    Flag(bool),
}

/// What a special module reads.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterSpecial {
    pub position: [f32; 2],
    pub facing: f32,
    pub params: Vec<(String, SpecialParam)>,
}

impl FighterSpecial {
    /// The number at `path`, or `None` when the move states none there.
    pub fn number(&self, path: &str) -> Option<f32> {
        number_at(&self.params, path)
    }

    /// The text at `path`, or `None` when the move states none there.
    pub fn text(&self, path: &str) -> Option<&str> {
        text_at(&self.params, path)
    }
}

/// The number at `path` of authored params, or `None` when they state none
/// there.
pub fn number_at(params: &[(String, SpecialParam)], path: &str) -> Option<f32> {
    params.iter().find_map(|(at, value)| match value {
        SpecialParam::Number(n) if at == path => Some(*n),
        _ => None,
    })
}

/// The text at `path` of authored params, or `None` when they state none
/// there.
pub fn text_at<'a>(params: &'a [(String, SpecialParam)], path: &str) -> Option<&'a str> {
    params.iter().find_map(|(at, value)| match value {
        SpecialParam::Text(text) if at == path => Some(text.as_str()),
        _ => None,
    })
}

/// Authored params on the wire: a count, then each path and its tagged value.
pub(crate) fn put_params(out: &mut Vec<u8>, params: &[(String, SpecialParam)]) {
    wire::put_u32(out, params.len() as u32);
    for (path, value) in params {
        wire::put_str(out, path);
        match value {
            SpecialParam::Number(n) => {
                wire::put_u8(out, 0);
                wire::put_f32(out, *n);
            }
            SpecialParam::Text(text) => {
                wire::put_u8(out, 1);
                wire::put_str(out, text);
            }
            SpecialParam::Flag(flag) => {
                wire::put_u8(out, 2);
                wire::put_bool(out, *flag);
            }
        }
    }
}

pub(crate) fn read_params(r: &mut WireReader<'_>) -> Result<Vec<(String, SpecialParam)>, WireError> {
    let count = r.read_len()?;
    let mut params = Vec::with_capacity(count);
    for _ in 0..count {
        let path = r.str()?.to_owned();
        let value = match r.u8()? {
            0 => SpecialParam::Number(r.f32()?),
            1 => SpecialParam::Text(r.str()?.to_owned()),
            2 => SpecialParam::Flag(r.bool()?),
            tag => return Err(WireError::BadTag(tag)),
        };
        params.push((path, value));
    }
    Ok(params)
}

impl Port for FighterSpecialPort {
    const KEY: PortKey = PortKey::new("ambition.fighter.special", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = FighterSpecial;

    fn encode(v: &FighterSpecial, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_f32(out, v.facing);
        put_params(out, &v.params);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<FighterSpecial, WireError> {
        let position = r.vec2()?;
        let facing = r.f32()?;
        let params = read_params(r)?;
        Ok(FighterSpecial { position, facing, params })
    }
}

/// The request port marker for a strike that follows its body.
///
/// Port card:
///
/// * **Operation** — put a damage box on the body the invocation ran for,
///   `forward` from its centre (`x` along its facing, `y` toward its feet),
///   which follows the body for `lifetime_s`. It is the body's own strike: the
///   body is not hit, and its side decides whom the strike hurts. `knockback`
///   is a share of the victim's feel-tuned launch. `hit_sound` is the cue a
///   hit plays (`"player.slash"`); without one, the victim's own hurt sound
///   plays.
/// * **Owner** — `ambition_combat::extension::special`, which spawns it as a
///   move's own volume (`strike::spawn_body_strike`).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `technique_execution`, this tick: the strike exists before
///   this tick's hit resolution.
/// * **Read model** — world units, in the body's own frame: a body in turned
///   gravity strikes in its turned frame.
pub struct BodyStrikePort;

#[derive(Clone, Debug, PartialEq)]
pub struct BodyStrike {
    pub forward: [f32; 2],
    pub half_extent: [f32; 2],
    pub damage: i32,
    pub knockback: f32,
    pub lifetime_s: f32,
    pub hit_sound: Option<String>,
}

impl Port for BodyStrikePort {
    const KEY: PortKey = PortKey::new("ambition.combat.body_strike", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BodyStrike;

    fn encode(v: &BodyStrike, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.forward);
        wire::put_vec2(out, v.half_extent);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
        wire::put_f32(out, v.lifetime_s);
        wire::put_opt(out, v.hit_sound.as_deref(), wire::put_str);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyStrike, WireError> {
        Ok(BodyStrike {
            forward: r.vec2()?,
            half_extent: r.vec2()?,
            damage: r.i32()?,
            knockback: r.f32()?,
            lifetime_s: r.f32()?,
            hit_sound: r.opt(|r| r.str().map(str::to_owned))?,
        })
    }
}

/// The request port marker for a sleep over an area around the body.
///
/// Port card:
///
/// * **Operation** — every other body of the invoking body's live room whose
///   box meets the area falls asleep for at least `duration_s`: it cannot
///   act until the time runs out or its own presses buy the time back (the
///   game's mash rule). A body that already sleeps longer keeps its time, so
///   two pulses do not add up to a sleep nobody wakes from. The area is
///   `forward` from the body's centre (`x` along its facing, `y` toward its
///   feet) with the half extents `half_extent`.
/// * **Owner** — `ambition_combat::extension::special`, which writes
///   `BodyCombat::sleep_timer`.
/// * **Scope** — the body the invocation ran for. It never falls asleep.
/// * **Time** — `technique_execution`, this tick: a body caught now cannot
///   act from this tick.
/// * **Read model** — world units, in the body's own frame: a body in turned
///   gravity sings into its turned frame.
pub struct SleepAreaPort;

#[derive(Clone, Debug, PartialEq)]
pub struct SleepArea {
    pub forward: [f32; 2],
    pub half_extent: [f32; 2],
    pub duration_s: f32,
}

impl Port for SleepAreaPort {
    const KEY: PortKey = PortKey::new("ambition.combat.sleep_area", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = SleepArea;

    fn encode(v: &SleepArea, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.forward);
        wire::put_vec2(out, v.half_extent);
        wire::put_f32(out, v.duration_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<SleepArea, WireError> {
        Ok(SleepArea { forward: r.vec2()?, half_extent: r.vec2()?, duration_s: r.f32()? })
    }
}

/// The request port marker for a slowed clock on the body.
///
/// Port card:
///
/// * **Operation** — the body the invocation ran for runs on a clock at
///   `scale` of the world's for `seconds` of world time, and then on its own
///   clock again. A newer dilation replaces the scale and the remainder, so
///   two slows do not multiply. A `scale` outside `0 ≤ scale < 1` or a time
///   that is not more than 0 is refused: a slow is the only direction, and a
///   slow with no end is never taken away.
/// * **Owner** — `ambition_combat::extension::special`, which writes the
///   body's `ProperTimeScale` and owns the countdown (`TimeDilated`).
/// * **Scope** — the body the invocation ran for. A counter's answer that
///   names the attacker runs for the attacker (`answers_the_attacker`).
/// * **Time** — `technique_execution`. The body's own step reads the scale
///   from the next tick, and the countdown spends world seconds from the next
///   tick, so `seconds` of N ticks slow N ticks.
/// * **Read model** — world seconds: a countdown on the slowed clock would
///   stretch itself.
pub struct DilateTimePort;

#[derive(Clone, Debug, PartialEq)]
pub struct DilateTime {
    pub scale: f32,
    pub seconds: f32,
}

impl DilateTime {
    /// Whether the owner applies it (see the port card).
    pub fn is_a_slow(&self) -> bool {
        (0.0..1.0).contains(&self.scale) && self.seconds.is_finite() && self.seconds > 0.0
    }
}

impl Port for DilateTimePort {
    const KEY: PortKey = PortKey::new("ambition.combat.dilate_time", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = DilateTime;

    fn encode(v: &DilateTime, out: &mut Vec<u8>) {
        wire::put_f32(out, v.scale);
        wire::put_f32(out, v.seconds);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<DilateTime, WireError> {
        Ok(DilateTime { scale: r.f32()?, seconds: r.f32()? })
    }
}

/// The request port marker for a refill of one of the body's resources.
///
/// Port card:
///
/// * **Operation** — add `amount` to the resource named `resource` (a
///   resource id, for example `smash.limit`) of the body the invocation ran
///   for, up to the resource's cap. A body that holds no resource of that
///   name gains nothing: the request names what it fills, not whatever pool
///   the body has. An amount that is not finite or is below 0 is refused: a
///   refill only adds.
/// * **Owner** — `ambition_combat::extension::special`, through the body's
///   `ActorResources`.
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `technique_execution`; the level is changed before the
///   phase's end, so a move that reads it next tick sees it.
pub struct RefillResourcePort;

#[derive(Clone, Debug, PartialEq)]
pub struct RefillResource {
    pub resource: String,
    pub amount: f32,
}

impl Port for RefillResourcePort {
    const KEY: PortKey = PortKey::new("ambition.resources.refill", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = RefillResource;

    fn encode(v: &RefillResource, out: &mut Vec<u8>) {
        wire::put_str(out, &v.resource);
        wire::put_f32(out, v.amount);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<RefillResource, WireError> {
        Ok(RefillResource { resource: r.str()?.to_owned(), amount: r.f32()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A refill decodes to what was encoded.
    #[test]
    fn a_refill_crosses_the_wire_unchanged() {
        let refill = RefillResource { resource: "smash.limit".into(), amount: 12.5 };
        let mut bytes = Vec::new();
        RefillResourcePort::encode(&refill, &mut bytes);
        assert_eq!(RefillResourcePort::decode(&mut WireReader::new(&bytes)).unwrap(), refill);
    }

    /// A dilation decodes to what was encoded.
    #[test]
    fn a_dilation_crosses_the_wire_unchanged() {
        let dilation = DilateTime { scale: 0.35, seconds: 0.45 };
        let mut bytes = Vec::new();
        DilateTimePort::encode(&dilation, &mut bytes);
        assert_eq!(DilateTimePort::decode(&mut WireReader::new(&bytes)).unwrap(), dilation);
    }

    /// A sleep area decodes to what was encoded.
    #[test]
    fn a_sleep_area_crosses_the_wire_unchanged() {
        let area = SleepArea { forward: [12.0, -3.0], half_extent: [70.0, 40.0], duration_s: 1.4 };
        let mut bytes = Vec::new();
        SleepAreaPort::encode(&area, &mut bytes);
        assert_eq!(SleepAreaPort::decode(&mut WireReader::new(&bytes)).unwrap(), area);
    }

    /// A special and a strike decode to what was encoded.
    #[test]
    fn a_special_and_a_strike_cross_the_wire_unchanged() {
        let special = FighterSpecial {
            position: [10.0, -4.0],
            facing: -1.0,
            params: vec![
                ("damage".to_owned(), SpecialParam::Number(9.0)),
                ("hit_sfx".to_owned(), SpecialParam::Text("player.slash".to_owned())),
                ("armed".to_owned(), SpecialParam::Flag(true)),
            ],
        };
        let mut bytes = Vec::new();
        FighterSpecialPort::encode(&special, &mut bytes);
        assert_eq!(FighterSpecialPort::decode(&mut WireReader::new(&bytes)).unwrap(), special);
        assert_eq!(special.number("damage"), Some(9.0));
        assert_eq!(special.text("hit_sfx"), Some("player.slash"));
        assert_eq!(special.number("hit_sfx"), None, "a text is not a number");

        for hit_sound in [Some("player.slash".to_owned()), None] {
            let strike = BodyStrike {
                forward: [30.0, 0.0],
                half_extent: [18.0, 12.0],
                damage: 9,
                knockback: 1.2,
                lifetime_s: 0.15,
                hit_sound,
            };
            let mut bytes = Vec::new();
            BodyStrikePort::encode(&strike, &mut bytes);
            assert_eq!(BodyStrikePort::decode(&mut WireReader::new(&bytes)).unwrap(), strike);
        }
    }
}
