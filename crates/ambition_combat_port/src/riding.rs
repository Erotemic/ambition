//! A volume that rides its owner, and a burst of particles: what a conducted
//! boss swings and shows.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a hitbox that rides its owner.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — put a hitbox at `offset` from the owner's position,
///   following the owner, for `lifetime_s`. It is a box of `half_extent`, or
///   a circle of `circle_radius`. Its knockback is a feel-scaled launch or a
///   fixed launch speed, along `launch_dir` when given. The owner's own art
///   shows it (`DepictedByOwner`); `name` names it for inspectors.
/// * **Owner** — `ambition_combat::extension`.
/// * **Scope and grant** — the owner is the body the invocation ran for, and
///   the hitbox is on that body's EFFECTIVE side; a body with no faction has
///   its hitboxes refused.
/// * **Time** — offered in `boss_conduct`; the hitbox exists for this tick's
///   combat.
/// * **Read model** — world units, +Y down; a launch's `x` is mirrored away
///   from its source.
pub struct RidingHitboxPort;

/// How a riding hitbox launches what it hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RidingKnockback {
    /// A multiplier over the victim's standard feel-tuned launch.
    FeelScale(f32),
    /// A launch at `base` speed, growing by `growth` with damage taken.
    LaunchSpeed { base: f32, growth: Option<f32> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RidingHitbox {
    pub offset: [f32; 2],
    pub half_extent: [f32; 2],
    pub circle_radius: Option<f32>,
    pub damage: i32,
    pub knockback: RidingKnockback,
    pub launch_dir: Option<[f32; 2]>,
    pub lifetime_s: f32,
    pub name: String,
}

impl Port for RidingHitboxPort {
    const KEY: PortKey = PortKey::new("ambition.combat.riding_hitbox", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = RidingHitbox;

    fn encode(v: &RidingHitbox, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.offset);
        wire::put_vec2(out, v.half_extent);
        wire::put_opt(out, v.circle_radius, wire::put_f32);
        wire::put_i32(out, v.damage);
        match v.knockback {
            RidingKnockback::FeelScale(f) => {
                wire::put_u8(out, 0);
                wire::put_f32(out, f);
            }
            RidingKnockback::LaunchSpeed { base, growth } => {
                wire::put_u8(out, 1);
                wire::put_f32(out, base);
                wire::put_opt(out, growth, wire::put_f32);
            }
        }
        wire::put_opt(out, v.launch_dir, wire::put_vec2);
        wire::put_f32(out, v.lifetime_s);
        wire::put_str(out, &v.name);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<RidingHitbox, WireError> {
        let offset = r.vec2()?;
        let half_extent = r.vec2()?;
        let circle_radius = r.opt(WireReader::f32)?;
        let damage = r.i32()?;
        let knockback = match r.u8()? {
            0 => RidingKnockback::FeelScale(r.f32()?),
            1 => RidingKnockback::LaunchSpeed {
                base: r.f32()?,
                growth: r.opt(WireReader::f32)?,
            },
            tag => return Err(WireError::BadTag(tag)),
        };
        Ok(RidingHitbox {
            offset,
            half_extent,
            circle_radius,
            damage,
            knockback,
            launch_dir: r.opt(WireReader::vec2)?,
            lifetime_s: r.f32()?,
            name: r.str()?.to_owned(),
        })
    }
}

/// The request port marker for a burst of particles.
///
/// Port card:
///
/// * **Operation** — show `count` particles of `kind` (`"dust"`, `"spark"`)
///   bursting from `at` at `speed`, in `color` (RGBA, 0..1). Presentation: no
///   simulation state reads it. An unknown kind is refused and logged.
/// * **Owner** — `ambition_boss_encounter::extension` (`VfxMessage::Burst`).
/// * **Time** — offered in `boss_conduct`.
pub struct BurstPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Burst {
    pub at: [f32; 2],
    pub count: u32,
    pub speed: f32,
    pub color: [f32; 4],
    pub kind: String,
}

impl Port for BurstPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.burst", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Burst;

    fn encode(v: &Burst, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.at);
        wire::put_u32(out, v.count);
        wire::put_f32(out, v.speed);
        for c in v.color {
            wire::put_f32(out, c);
        }
        wire::put_str(out, &v.kind);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Burst, WireError> {
        Ok(Burst {
            at: r.vec2()?,
            count: r.u32()?,
            speed: r.f32()?,
            color: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
            kind: r.str()?.to_owned(),
        })
    }
}

/// The request port marker for a shake of the camera.
///
/// Port card:
///
/// * **Operation** — shake the camera by `amplitude_px` (world pixels, before
///   the player's shake tuning caps it). Several in one frame settle on the
///   strongest. Presentation: no simulation state reads it.
/// * **Owner** — `ambition_boss_encounter::extension`, as a
///   `CameraShakeRequest`: an intent released on the confirmed frame, so a
///   predicted frame a rollback erases shakes nothing.
/// * **Time** — offered in `boss_conduct`.
pub struct CameraShakePort;

#[derive(Clone, Debug, PartialEq)]
pub struct CameraShake {
    pub amplitude_px: f32,
}

impl Port for CameraShakePort {
    const KEY: PortKey = PortKey::new("ambition.feedback.camera_shake", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = CameraShake;

    fn encode(v: &CameraShake, out: &mut Vec<u8>) {
        wire::put_f32(out, v.amplitude_px);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<CameraShake, WireError> {
        Ok(CameraShake { amplitude_px: r.f32()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_shake_survives_the_wire() {
        let v = CameraShake { amplitude_px: 9.5 };
        let mut out = Vec::new();
        CameraShakePort::encode(&v, &mut out);
        assert_eq!(CameraShakePort::decode(&mut WireReader::new(&out)).unwrap(), v);
    }

    #[test]
    fn riding_values_survive_the_wire() {
        for knockback in [
            RidingKnockback::FeelScale(1.3),
            RidingKnockback::LaunchSpeed { base: 560.0, growth: Some(0.0) },
            RidingKnockback::LaunchSpeed { base: 1.0, growth: None },
        ] {
            let h = RidingHitbox {
                offset: [1.0, -2.0],
                half_extent: [3.0, 4.0],
                circle_radius: Some(34.0),
                damage: 2,
                knockback,
                launch_dir: Some([-0.5, -0.87]),
                lifetime_s: 0.3,
                name: "fsm_grasp".into(),
            };
            let mut out = Vec::new();
            RidingHitboxPort::encode(&h, &mut out);
            let mut r = WireReader::new(&out);
            assert_eq!(RidingHitboxPort::decode(&mut r).unwrap(), h);
            r.finish().unwrap();
        }
        let b = Burst { at: [1.0, 2.0], count: 24, speed: 380.0, color: [0.9, 0.8, 0.7, 1.0], kind: "spark".into() };
        let mut out = Vec::new();
        BurstPort::encode(&b, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(BurstPort::decode(&mut r).unwrap(), b);
        r.finish().unwrap();
    }
}
