//! The physical scale and orientation of canonical body material.

use bevy_math::Mat2;

use super::{NumericFault, Sim2};
use crate::Vec2;

/// Material axes relative to the body's acceleration frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMaterial {
    scale: f32,
    orthogonal: Mat2,
}

impl Default for BodyMaterial {
    fn default() -> Self { Self::IDENTITY }
}

impl BodyMaterial {
    pub const IDENTITY: Self = Self { scale: 1.0, orthogonal: Mat2::IDENTITY };

    pub fn new(scale: f32, orthogonal: Mat2) -> Result<Self, NumericFault> {
        let map = Sim2::new(scale, orthogonal, Vec2::ZERO)?;
        Ok(Self { scale, orthogonal: map.orthogonal() })
    }

    pub fn scale(self) -> f32 { self.scale }
    pub fn orthogonal(self) -> Mat2 { self.orthogonal }
    pub fn reflected(self) -> bool { self.orthogonal.determinant() < 0.0 }

    /// Build the map from canonical coordinates to the body's world chart.
    pub fn at(self, center: Vec2, down: Vec2) -> Result<Sim2, NumericFault> {
        let frame = crate::AccelerationFrame::new(down);
        Sim2::new(self.scale, Mat2::from_cols(frame.side, frame.down) * self.orthogonal, center)
    }

    /// The new base frame can differ from the frame at departure.
    pub fn carried(self, map: Sim2, from_down: Vec2, to_down: Vec2) -> Result<Self, NumericFault> {
        let from = crate::AccelerationFrame::new(from_down);
        let to = crate::AccelerationFrame::new(to_down);
        let from = Mat2::from_cols(from.side, from.down);
        let to = Mat2::from_cols(to.side, to.down);
        let composed = Sim2::new(self.scale, from * self.orthogonal, Vec2::ZERO)?
            .then(Sim2::new(map.scale(), map.orthogonal(), Vec2::ZERO)?)?;
        Self::new(composed.scale(), to.transpose() * composed.orthogonal())
    }

    /// The axis-aligned envelope is for broad phase queries.
    pub fn half_extent(self, canonical_half: Vec2, down: Vec2) -> Vec2 {
        let frame = crate::AccelerationFrame::new(down);
        let basis = Mat2::from_cols(frame.side, frame.down) * self.orthogonal;
        (basis.x_axis.abs() * canonical_half.x + basis.y_axis.abs() * canonical_half.y) * self.scale
    }
}

impl crate::snapshot::SnapshotState for BodyMaterial {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::snapshot::{put_f32, put_vec2};
        put_f32(out, self.scale);
        put_vec2(out, self.orthogonal.x_axis);
        put_vec2(out, self.orthogonal.y_axis);
    }

    fn decode(reader: &mut crate::snapshot::Reader<'_>) -> Option<Self> {
        let scale = reader.f32()?;
        let orthogonal = Mat2::from_cols(reader.vec2()?, reader.vec2()?);
        Sim2::new(scale, orthogonal, Vec2::ZERO).ok()?;
        // Restore the admitted bits. Normalization can change signed zero
        // or round an axis again and change the peer checksum.
        Some(Self { scale, orthogonal })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_reflections_restore_parity_and_keep_scale() {
        let map = Sim2::new(2.0, Mat2::from_cols(-Vec2::X, Vec2::Y), Vec2::ZERO).unwrap();
        let first = BodyMaterial::IDENTITY.carried(map, Vec2::Y, Vec2::X).unwrap();
        assert!(first.reflected());
        let second = first.carried(map, Vec2::X, Vec2::Y).unwrap();
        assert!(!second.reflected());
        assert_eq!(second.scale(), 4.0);
        assert_eq!(second.orthogonal(), Mat2::IDENTITY);
    }

    #[test]
    fn changed_base_frame_keeps_the_world_map() {
        let material = BodyMaterial::new(3.0, Mat2::from_angle(0.37)).unwrap();
        let map = Sim2::new(0.5, Mat2::from_cols(Vec2::Y, Vec2::X), Vec2::new(700.0, 25.0)).unwrap();
        let before = material.at(Vec2::new(20.0, 50.0), -Vec2::X).unwrap();
        let after = material.carried(map, -Vec2::X, -Vec2::Y).unwrap()
            .at(map.point(Vec2::new(20.0, 50.0)).unwrap(), -Vec2::Y).unwrap();
        for point in [Vec2::ZERO, Vec2::X, Vec2::new(-10.0, 20.0)] {
            assert!((map.point(before.point(point).unwrap()).unwrap() - after.point(point).unwrap()).length() < 1.0e-4);
        }
    }
}


#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::snapshot::{decode_state, encode_state};

    #[test]
    fn material_survives_a_body_snapshot_and_rejects_invalid_scale() {
        let body = crate::BodyKinematics {
            material: BodyMaterial::new(0.25, Mat2::from_cols(Vec2::Y, Vec2::X)).unwrap(),
            ..Default::default()
        };
        let bytes = encode_state(&body);
        assert_eq!(decode_state::<crate::BodyKinematics>(&bytes), Some(body));
        let mut invalid = bytes;
        invalid[28..32].copy_from_slice(&0.0_f32.to_le_bytes());
        assert!(decode_state::<crate::BodyKinematics>(&invalid).is_none());
    }

    #[test]
    fn restoring_material_keeps_every_snapshot_bit() {
        for material in [
            BodyMaterial::IDENTITY,
            BodyMaterial::new(3.0, Mat2::from_angle(0.37)).unwrap(),
            BodyMaterial::new(0.25, Mat2::from_cols(Vec2::Y, Vec2::X)).unwrap(),
        ] {
            let body = crate::BodyKinematics { material, ..Default::default() };
            let saved = encode_state(&body);
            let restored = decode_state::<crate::BodyKinematics>(&saved).unwrap();
            assert_eq!(encode_state(&restored), saved);
        }
    }
}
