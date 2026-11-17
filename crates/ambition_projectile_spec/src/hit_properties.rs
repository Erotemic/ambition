//! Portable codecs for the shared hit values. The values do not depend on the SDK.

use ambition_extension_sdk::{
    script::ScriptValue,
    structure::{Field, Shape},
    wire::{WireError, WireReader},
};
use ambition_hit_spec::*;

macro_rules! text_choice {
    ($ty:ty as $codec:ident, {$($case:ident => $text:literal),+ $(,)?}) => {
        pub struct $codec;
        impl Field<$ty> for $codec {
            fn shape() -> Shape {
                Shape::Choice(vec![$(ambition_extension_sdk::structure::CaseShape {
                    name: $text, payload: None, script: concat!("\"", $text, "\""),
                }),+])
            }
            fn put(v: &$ty, out: &mut Vec<u8>) {
                String::put(&match v { $(<$ty>::$case => $text),+ }.to_owned(), out)
            }
            fn read(r: &mut WireReader<'_>) -> Result<$ty, WireError> {
                match String::read(r)?.as_str() {
                    $($text => Ok(<$ty>::$case),)+
                    _ => Err(WireError::BadTag(255)),
                }
            }
            fn to_script(v: &$ty) -> ScriptValue {
                String::to_script(&match v { $(<$ty>::$case => $text),+ }.to_owned())
            }
            fn from_script(v: &ScriptValue) -> Result<$ty, String> {
                match String::from_script(v)?.as_str() {
                    $($text => Ok(<$ty>::$case),)+
                    other => Err(format!("unknown {}: {other}", stringify!($ty))),
                }
            }
        }
    };
}
text_choice!(ClashPolicy as ClashPolicyCodec, {Normal => "normal", Transcendent => "transcendent"});
text_choice!(TechPolicy as TechPolicyCodec, {Normal => "normal", Deny => "deny"});
text_choice!(ArmorInteraction as ArmorInteractionCodec, {Respect => "respect", Bypass => "bypass"});

pub struct LaunchWeightCodec;
impl Field<LaunchWeight> for LaunchWeightCodec {
    fn shape() -> Shape {
        <Option<f32>>::shape()
    }
    fn put(v: &LaunchWeight, out: &mut Vec<u8>) {
        Option::<f32>::put(
            &match v {
                LaunchWeight::Victim => None,
                LaunchWeight::Fixed(w) => Some(*w),
            },
            out,
        )
    }
    fn read(r: &mut WireReader<'_>) -> Result<LaunchWeight, WireError> {
        match Option::<f32>::read(r)? {
            None => Ok(LaunchWeight::Victim),
            Some(w) if w.is_finite() && w > 0.0 => Ok(LaunchWeight::Fixed(w)),
            _ => Err(WireError::BadTag(255)),
        }
    }
    fn to_script(v: &LaunchWeight) -> ScriptValue {
        Option::<f32>::to_script(&match v {
            LaunchWeight::Victim => None,
            LaunchWeight::Fixed(w) => Some(*w),
        })
    }
    fn from_script(v: &ScriptValue) -> Result<LaunchWeight, String> {
        match Option::<f32>::from_script(v)? {
            None => Ok(LaunchWeight::Victim),
            Some(w) if w > 0.0 => Ok(LaunchWeight::Fixed(w)),
            _ => Err("fixed launch weight must be positive".into()),
        }
    }
}

pub struct HitGuardCodec;
impl Field<HitGuard> for HitGuardCodec {
    fn shape() -> Shape {
        <Option<u32>>::shape()
    }
    fn put(v: &HitGuard, out: &mut Vec<u8>) {
        Option::<u32>::put(
            &match v {
                HitGuard::Unblockable => None,
                HitGuard::Blockable {
                    extra_shield_damage,
                } => Some(u32::from(*extra_shield_damage)),
            },
            out,
        )
    }
    fn read(r: &mut WireReader<'_>) -> Result<HitGuard, WireError> {
        match Option::<u32>::read(r)? {
            None => Ok(HitGuard::Unblockable),
            Some(n) => Ok(HitGuard::Blockable {
                extra_shield_damage: n.try_into().map_err(|_| WireError::BadTag(255))?,
            }),
        }
    }
    fn to_script(v: &HitGuard) -> ScriptValue {
        Option::<u32>::to_script(&match v {
            HitGuard::Unblockable => None,
            HitGuard::Blockable {
                extra_shield_damage,
            } => Some(u32::from(*extra_shield_damage)),
        })
    }
    fn from_script(v: &ScriptValue) -> Result<HitGuard, String> {
        match Option::<u32>::from_script(v)? {
            None => Ok(HitGuard::Unblockable),
            Some(n) => Ok(HitGuard::Blockable {
                extra_shield_damage: n.try_into().map_err(|_| "shield damage is out of range")?,
            }),
        }
    }
}

pub struct HitTargetsCodec;
ambition_extension_sdk::port_record! {
    HitTargets as HitTargetsCodec {
        grounded: bool = true,
        airborne: bool = true,
        prone: bool = true,
    }
}
pub struct HitPropertiesCodec;
ambition_extension_sdk::port_record! {
    HitProperties as HitPropertiesCodec {
        targets: HitTargets as HitTargetsCodec = HitTargets::default(),
        clash: ClashPolicy as ClashPolicyCodec = ClashPolicy::Normal,
        hitlag_scale: f32 = 1.0,
        sdi_scale: f32 = 1.0,
        tech: TechPolicy as TechPolicyCodec = TechPolicy::Normal,
        weight: LaunchWeight as LaunchWeightCodec = LaunchWeight::Victim,
        armor: ArmorInteraction as ArmorInteractionCodec = ArmorInteraction::Respect,
    }
}
