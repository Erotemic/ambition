//! Encode hit properties for rollback.

use ambition_snapshot::{snapshot::SnapshotState, snapshot_unit_enum};

snapshot_unit_enum!(crate::ClashPolicy { Normal = 0, Transcendent = 1 });
snapshot_unit_enum!(crate::TechPolicy { Normal = 0, Deny = 1 });
snapshot_unit_enum!(crate::ArmorInteraction { Respect = 0, Bypass = 1 });

impl SnapshotState for crate::HitProperties {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_snapshot::snapshot::{put_bool, put_f32};
        put_bool(out, self.targets.grounded);
        put_bool(out, self.targets.airborne);
        put_bool(out, self.targets.prone);
        self.clash.encode(out);
        put_f32(out, self.hitlag_scale);
        put_f32(out, self.sdi_scale);
        self.tech.encode(out);
        match self.weight {
            crate::LaunchWeight::Victim => put_bool(out, false),
            crate::LaunchWeight::Fixed(w) => {
                put_bool(out, true);
                put_f32(out, w);
            }
        }
        self.armor.encode(out);
    }
    fn decode(r: &mut ambition_snapshot::snapshot::Reader<'_>) -> Option<Self> {
        let properties = Self {
            targets: crate::HitTargets {
                grounded: r.bool()?,
                airborne: r.bool()?,
                prone: r.bool()?,
            },
            clash: crate::ClashPolicy::decode(r)?,
            hitlag_scale: r.f32()?,
            sdi_scale: r.f32()?,
            tech: crate::TechPolicy::decode(r)?,
            weight: if r.bool()? {
                crate::LaunchWeight::Fixed(r.f32()?)
            } else {
                crate::LaunchWeight::Victim
            },
            armor: crate::ArmorInteraction::decode(r)?,
        };
        properties.is_valid().then_some(properties)
    }
}

impl SnapshotState for crate::HitGuard {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_snapshot::snapshot::{put_bool, put_u32};
        match self {
            Self::Unblockable => put_bool(out, false),
            Self::Blockable {
                extra_shield_damage,
            } => {
                put_bool(out, true);
                put_u32(out, u32::from(*extra_shield_damage));
            }
        }
    }
    fn decode(r: &mut ambition_snapshot::snapshot::Reader<'_>) -> Option<Self> {
        Some(if r.bool()? {
            Self::Blockable {
                extra_shield_damage: r.u32()?.try_into().ok()?,
            }
        } else {
            Self::Unblockable
        })
    }
}

impl ambition_snapshot::snapshot::PasteEncode for crate::TechPolicy {
    fn put(self, out: &mut Vec<u8>) {
        ambition_snapshot::snapshot::SnapshotState::encode(&self, out);
    }
}

impl ambition_snapshot::snapshot::ExactState for crate::TechPolicy {
    fn same_bits(&self, other: &Self) -> bool {
        self == other
    }
}
