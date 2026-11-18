//! Lightning a module shows, not one it strikes with: a bolt between two
//! ends, and an aura on its own body. Presentation only; no simulation state
//! reads either. A bolt that hurts is a ray ([`crate::ray::HeldRayPort`]),
//! drawn from its own geometry.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

use crate::ray::Spot;

/// The request port marker for a bolt of lightning.
///
/// Port card:
///
/// * **Operation** — draw a bolt of the named `look` (`"strike"`, `"force"`,
///   `"spark_gap"`) from `from` to `to`, held `hold_s` seconds. LEASED by
///   `key`: the same key asked again renews the bolt and moves its ends, so
///   a channel asked every tick is one bolt. `glow` recolours the halo. An
///   unknown look is refused and logged. A [`crate::ray::Spot::Part`] end
///   is refused: a feedback bolt needs a world or body endpoint.
/// * **Owner** — `ambition_boss_encounter::extension` in `boss_conduct`
///   (`VfxMessage::Lightning`), in the live room of the body; the key is
///   scoped to that body.
/// * **Time** — offered in `boss_conduct`.
/// * **Read model** — world units, +Y down.
pub struct LightningPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Lightning {
    pub key: String,
    pub from: Spot,
    pub to: Spot,
    pub look: String,
    pub glow: Option<[f32; 4]>,
    pub hold_s: f32,
}

impl Port for LightningPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.lightning", 3);
    const ROLE: PortRole = PortRole::Request;
    type Value = Lightning;

    fn encode(v: &Lightning, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Lightning, WireError> {
        ambition_extension_sdk::structure::read(r)
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
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LightningAura, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bolt_and_an_aura_round_trip() {
        let bolt = Lightning {
            key: "lash".into(),
            from: Spot::Body([4.0, -40.0]),
            to: Spot::World([300.0, 120.0]),
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

ambition_extension_sdk::port_record! {
    Lightning {
        key: String,
        from: Spot,
        to: Spot,
        look: String,
        glow: Option<[f32; 4]> = None,
        hold_s: f32,
    }
}
ambition_extension_sdk::port_record! {
    LightningAura {
        key: String,
        center: [f32; 2],
        half: [f32; 2],
        look: String,
        glow: Option<[f32; 4]> = None,
        hold_s: f32,
    }
}
ambition_extension_sdk::script_port!(LightningPort);
ambition_extension_sdk::script_port!(LightningAuraPort);
