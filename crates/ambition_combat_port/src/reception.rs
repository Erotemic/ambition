//! Bounded batches of decisions from native hit reception.

use ambition_extension_sdk::{
    wire::{WireError, WireReader},
    Port, PortKey, PortRole,
};

ambition_extension_sdk::port_choices! {
    pub enum ReceptionDecision {
        Ignored = 0 => "ignored",
        Protected = 1 => "protected",
        Blocked = 2 => "blocked",
        Armored = 3 => "armored",
        WalletShielded = 4 => "wallet_shielded",
        Damaged = 5 => "damaged",
    }
}
ambition_extension_sdk::port_choices! {
    pub enum ReceptionSource {
        Melee = 0 => "melee",
        Projectile = 1 => "projectile",
        Contact = 2 => "contact",
        Hazard = 3 => "hazard",
        LeftTheWorld = 4 => "left_the_world",
        Pogo = 5 => "pogo",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReceptionBody {
    pub id: Option<String>,
    pub session: Option<u64>,
    pub room: Option<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HitReception {
    pub observed_at: u64,
    pub victim: ReceptionBody,
    pub attacker: Option<ReceptionBody>,
    pub source: ReceptionSource,
    pub decision: ReceptionDecision,
    pub raw_damage: i32,
    pub damage: i32,
    pub wallet_spent: i32,
    pub health_depleted: bool,
    pub move_instance: Option<u32>,
    pub incoming: bool,
    pub outgoing: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HitReceptions {
    pub events: Vec<HitReception>,
    pub truncated: bool,
}
ambition_extension_sdk::port_record! {
    ReceptionBody {
        /// Absent when the native source has no semantic identity.
        id: Option<String>,
        session: Option<u64>,
        room: Option<u32>,
    }
}
ambition_extension_sdk::port_record! {
    HitReception {
        observed_at: u64,
        victim: ReceptionBody,
        attacker: Option<ReceptionBody>,
        source: ReceptionSource,
        decision: ReceptionDecision,
        raw_damage: i32,
        /// The resolver's damage amount, after defenses and modifiers.
        damage: i32,
        wallet_spent: i32,
        /// Health reached its limit. The lifecycle owner decides defeat.
        health_depleted: bool,
        move_instance: Option<u32>,
        /// The invocation body received this hit.
        incoming: bool,
        /// The invocation body owns this hit.
        outgoing: bool,
    }
}
ambition_extension_sdk::port_record! {
    HitReceptions {
        events: Vec<HitReception>,
        /// More decisions matched the scope than its game-authored budget.
        truncated: bool,
    }
}

/// Native reception decisions in `hit_reaction`, after damage resolution.
/// A held status binds a selector on any controller. Authored combat rules
/// bind room observers. Bodies
/// receive their incoming and outgoing decisions. Room roots receive decisions
/// for their session and live room. Each nonempty batch is delivered once at
/// the next phase read cut, including during hitstop. No event implies that a
/// projectile was reflected, absorbed, or retired. Contact and interception
/// have their own owners.
pub struct HitReceptionsPort;
impl Port for HitReceptionsPort {
    const KEY: PortKey = PortKey::new("ambition.combat.hit_receptions", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = HitReceptions;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(HitReceptionsPort);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::script::ScriptPort;
    #[test]
    fn reception_facts_keep_exact_scopes_and_zero_damage_across_both_codecs() {
        let value = HitReceptions {
            truncated: true,
            events: vec![HitReception {
                observed_at: u64::MAX - 1,
                victim: ReceptionBody {
                    id: Some("receiver".into()),
                    session: Some(u64::MAX),
                    room: Some(2),
                },
                attacker: Some(ReceptionBody {
                    id: None,
                    session: None,
                    room: None,
                }),
                source: ReceptionSource::Projectile,
                decision: ReceptionDecision::WalletShielded,
                raw_damage: 10,
                damage: 0,
                wallet_spent: 7,
                health_depleted: false,
                move_instance: Some(u32::MAX),
                incoming: true,
                outgoing: false,
            }],
        };
        let mut bytes = Vec::new();
        HitReceptionsPort::encode(&value, &mut bytes);
        assert_eq!(
            HitReceptionsPort::decode(&mut WireReader::new(&bytes)).unwrap(),
            value
        );
        assert_eq!(
            HitReceptionsPort::from_script(&HitReceptionsPort::to_script(&value)).unwrap(),
            value
        );
    }
}
