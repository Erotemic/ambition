//! Procedural lightning a module asks for: a bolt between two ends, and an
//! aura on its own body. Presentation only; no simulation state reads either.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// Where one end of a module's bolt is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoltEnd {
    /// A world point, +Y down.
    World([f32; 2]),
    /// An offset from the body the invocation ran for, where it is drawn: the
    /// end follows the body.
    Body([f32; 2]),
}

/// The request port marker for a bolt of lightning.
///
/// Port card:
///
/// * **Operation** — draw a bolt of the named `look` (`"strike"`, `"force"`,
///   `"spark_gap"`) from `from` to `to`, held `hold_s` seconds. LEASED by
///   `key`: the same key asked again renews the bolt and moves its ends, so
///   a channel asked every tick is one bolt. `glow` recolours the halo. An
///   unknown look is refused and logged.
/// * **Owner** — `ambition_boss_encounter::extension` in `boss_conduct`
///   (`VfxMessage::Lightning`), in the live room of the body; the key is
///   scoped to that body.
/// * **Time** — offered in `boss_conduct`.
/// * **Read model** — world units, +Y down.
pub struct LightningPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Lightning {
    pub key: String,
    pub from: BoltEnd,
    pub to: BoltEnd,
    pub look: String,
    pub glow: Option<[f32; 4]>,
    pub hold_s: f32,
}

fn put_end(out: &mut Vec<u8>, end: BoltEnd) {
    match end {
        BoltEnd::World(at) => {
            wire::put_u8(out, 0);
            wire::put_vec2(out, at);
        }
        BoltEnd::Body(offset) => {
            wire::put_u8(out, 1);
            wire::put_vec2(out, offset);
        }
    }
}

fn end(r: &mut WireReader<'_>) -> Result<BoltEnd, WireError> {
    match r.u8()? {
        0 => Ok(BoltEnd::World(r.vec2()?)),
        1 => Ok(BoltEnd::Body(r.vec2()?)),
        tag => Err(WireError::BadTag(tag)),
    }
}

fn put_rgba(out: &mut Vec<u8>, c: [f32; 4]) {
    for v in c {
        wire::put_f32(out, v);
    }
}

fn rgba(r: &mut WireReader<'_>) -> Result<[f32; 4], WireError> {
    Ok([r.f32()?, r.f32()?, r.f32()?, r.f32()?])
}

impl Port for LightningPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.lightning", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Lightning;

    fn encode(v: &Lightning, out: &mut Vec<u8>) {
        wire::put_str(out, &v.key);
        put_end(out, v.from);
        put_end(out, v.to);
        wire::put_str(out, &v.look);
        wire::put_opt(out, v.glow, put_rgba);
        wire::put_f32(out, v.hold_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Lightning, WireError> {
        Ok(Lightning {
            key: r.str()?.to_owned(),
            from: end(r)?,
            to: end(r)?,
            look: r.str()?.to_owned(),
            glow: r.opt(rgba)?,
            hold_s: r.f32()?,
        })
    }
}

/// The request port marker for an aura of lightning on the asking body.
///
/// Port card:
///
/// * **Operation** — draw arcs of the named `look` (`"aura"`) crawling over
///   an ellipse of `half` about `center` (offsets from where the body is
///   drawn), held `hold_s` seconds and LEASED by `key` as the bolt is. A
///   powered-up body asks for it every tick it is powered up. `glow`
///   recolours the halo. An unknown look is refused and logged.
/// * **Owner** — `ambition_boss_encounter::extension` in `boss_conduct`
///   (`VfxMessage::LightningAura`).
/// * **Time** — offered in `boss_conduct`.
/// * **Read model** — world units, +Y down.
pub struct LightningAuraPort;

#[derive(Clone, Debug, PartialEq)]
pub struct LightningAura {
    pub key: String,
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub look: String,
    pub glow: Option<[f32; 4]>,
    pub hold_s: f32,
}

impl Port for LightningAuraPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.lightning_aura", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LightningAura;

    fn encode(v: &LightningAura, out: &mut Vec<u8>) {
        wire::put_str(out, &v.key);
        wire::put_vec2(out, v.center);
        wire::put_vec2(out, v.half);
        wire::put_str(out, &v.look);
        wire::put_opt(out, v.glow, put_rgba);
        wire::put_f32(out, v.hold_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LightningAura, WireError> {
        Ok(LightningAura {
            key: r.str()?.to_owned(),
            center: r.vec2()?,
            half: r.vec2()?,
            look: r.str()?.to_owned(),
            glow: r.opt(rgba)?,
            hold_s: r.f32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bolt_and_an_aura_round_trip() {
        let bolt = Lightning {
            key: "lash".into(),
            from: BoltEnd::Body([4.0, -40.0]),
            to: BoltEnd::World([300.0, 120.0]),
            look: "force".into(),
            glow: Some([0.1, 0.2, 0.3, 0.4]),
            hold_s: 0.1,
        };
        let mut out = Vec::new();
        LightningPort::encode(&bolt, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(LightningPort::decode(&mut r).unwrap(), bolt);
        r.finish().unwrap();

        let aura = LightningAura {
            key: "unleashed".into(),
            center: [0.0, -4.0],
            half: [30.0, 52.0],
            look: "aura".into(),
            glow: None,
            hold_s: 0.1,
        };
        let mut out = Vec::new();
        LightningAuraPort::encode(&aura, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(LightningAuraPort::decode(&mut r).unwrap(), aura);
        r.finish().unwrap();
    }
}
