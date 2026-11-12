//! The physical scale and orientation of canonical body material.

use bevy_math::{DMat2, DVec2, Mat2};

use super::{NumericFault, Sim2};
use crate::Vec2;

/// Canonical material axes in the body's current room chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMaterial {
    scale: f64,
    orthogonal: DMat2,
}

/// A lossless material record. Integer words give saves exact equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct BodyMaterialRecord([u64; 5]);

impl Default for BodyMaterialRecord {
    fn default() -> Self { BodyMaterial::IDENTITY.record() }
}

impl BodyMaterialRecord {
    fn checked(self) -> Result<BodyMaterial, NumericFault> {
        let material = self.material();
        material.at(Vec2::ZERO)?;
        Ok(material)
    }

    /// Read the material that was validated when this record was made.
    pub fn material(self) -> BodyMaterial {
        let [scale, xx, xy, yx, yy] = self.0.map(f64::from_bits);
        BodyMaterial { scale, orthogonal: DMat2::from_cols(DVec2::new(xx, xy), DVec2::new(yx, yy)) }
    }
}

impl<'de> serde::Deserialize<'de> for BodyMaterialRecord {
    fn deserialize<D: serde::Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let record = Self(<[u64; 5] as serde::Deserialize>::deserialize(reader)?);
        record.checked().map_err(|fault| serde::de::Error::custom(format!("invalid body material: {fault:?}")))?;
        Ok(record)
    }
}

impl Default for BodyMaterial {
    fn default() -> Self { Self::IDENTITY }
}

impl BodyMaterial {
    pub const IDENTITY: Self = Self { scale: 1.0, orthogonal: DMat2::IDENTITY };

    pub fn new(scale: f32, orthogonal: Mat2) -> Result<Self, NumericFault> {
        let map = Sim2::new(scale, orthogonal, Vec2::ZERO)?;
        Ok(Self::from_linear(map))
    }

    pub fn scale(self) -> f32 { self.scale as f32 }
    pub fn orthogonal(self) -> Mat2 { Mat2::from_cols(self.orthogonal.x_axis.as_vec2(), self.orthogonal.y_axis.as_vec2()) }
    pub fn reflected(self) -> bool { self.orthogonal.determinant() < 0.0 }

    pub fn record(self) -> BodyMaterialRecord {
        BodyMaterialRecord([self.scale, self.orthogonal.x_axis.x, self.orthogonal.x_axis.y,
            self.orthogonal.y_axis.x, self.orthogonal.y_axis.y].map(f64::to_bits))
    }

    fn from_linear(map: Sim2) -> Self {
        Self { scale: map.scale_wide(), orthogonal: DMat2::from_cols(
            map.direction_wide(DVec2::X), map.direction_wide(DVec2::Y)) }
    }

    /// Map a facing sign between canonical x and a reference side.
    /// A perpendicular request cannot be expressed by a facing flip.
    pub fn project_facing(self, facing: f32, reference_side: Vec2) -> f32 {
        let alignment = self.orthogonal.x_axis.dot(reference_side.as_dvec2());
        if facing.abs() <= 0.001 || alignment.abs() <= 0.001 { return 0.0; }
        (f64::from(facing.signum()) * alignment).signum() as f32
    }

    /// Build the map from canonical coordinates into the current room chart.
    pub fn at(self, center: Vec2) -> Result<Sim2, NumericFault> {
        Sim2::from_wide_translation(self.scale, self.orthogonal, center.as_dvec2())
    }

    /// Carry physical scale, rotation and parity into another chart.
    pub fn carried(self, map: Sim2) -> Result<Self, NumericFault> {
        let linear = Sim2::from_wide_translation(map.scale_wide(),
            DMat2::from_cols(map.direction_wide(DVec2::X), map.direction_wide(DVec2::Y)), DVec2::ZERO)?;
        Ok(Self::from_linear(self.at(Vec2::ZERO)?.then(linear)?))
    }

    pub fn turned(self, angle: f64) -> Result<Self, NumericFault> {
        if !angle.is_finite() { return Err(NumericFault::NonFinite); }
        Ok(Self::from_linear(Sim2::from_wide_translation(self.scale,
            super::rotation_at(angle) * self.orthogonal, DVec2::ZERO)?))
    }

    /// Turn the physical down axis toward a target without changing parity.
    pub fn righting_turn(self, target_down: Vec2, max_step: f64) -> Result<f64, NumericFault> {
        if !target_down.is_finite() || !max_step.is_finite() { return Err(NumericFault::NonFinite); }
        if max_step < 0.0 { return Err(NumericFault::InvalidFrame); }
        let target = target_down.as_dvec2().normalize_or_zero();
        if target == DVec2::ZERO { return Ok(0.0); }
        let down = self.orthogonal.y_axis;
        Ok(down.perp_dot(target).atan2(down.dot(target)).clamp(-max_step, max_step))
    }

    /// Render-space rotation. Physical reflection is a separate projection.
    pub fn render_angle(self) -> f32 {
        self.orthogonal.y_axis.x.atan2(self.orthogonal.y_axis.y) as f32
    }

    /// The axis-aligned envelope is for broad phase queries.
    pub fn half_extent(self, canonical_half: Vec2) -> Vec2 {
        ((self.orthogonal.x_axis.abs() * f64::from(canonical_half.x)
            + self.orthogonal.y_axis.abs() * f64::from(canonical_half.y)) * self.scale).as_vec2()
    }

}

impl crate::snapshot::SnapshotState for BodyMaterial {
    fn encode(&self, out: &mut Vec<u8>) {
        for word in self.record().0 {
            crate::snapshot::put_u64(out, word);
        }
    }

    fn decode(reader: &mut crate::snapshot::Reader<'_>) -> Option<Self> {
        BodyMaterialRecord([reader.u64()?, reader.u64()?, reader.u64()?, reader.u64()?, reader.u64()?]).checked().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_reflections_restore_parity_and_keep_scale() {
        let map = Sim2::new(2.0, Mat2::from_cols(-Vec2::X, Vec2::Y), Vec2::ZERO).unwrap();
        let first = BodyMaterial::IDENTITY.carried(map).unwrap();
        assert!(first.reflected());
        let second = first.carried(map).unwrap();
        assert!(!second.reflected());
        assert_eq!(second.scale(), 4.0);
        assert_eq!(second.orthogonal(), Mat2::IDENTITY);
    }

    #[test]
    fn a_carried_material_keeps_the_composed_world_map() {
        let material = BodyMaterial::new(3.0, Mat2::from_angle(0.37)).unwrap();
        let map = Sim2::new(0.5, Mat2::from_cols(Vec2::Y, Vec2::X), Vec2::new(700.0, 25.0)).unwrap();
        let before = material.at(Vec2::new(20.0, 50.0)).unwrap();
        let after = material.carried(map).unwrap()
            .at(map.point(Vec2::new(20.0, 50.0)).unwrap()).unwrap();
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
    fn saved_material_keeps_every_bit_and_rejects_invalid_scale_or_axes() {
        for material in [BodyMaterial::IDENTITY,
            BodyMaterial::new(0.5, Mat2::from_angle(0.37) * Mat2::from_cols(-Vec2::X, Vec2::Y)).unwrap()] {
            let record = material.record();
            let saved = serde_json::to_string(&record).unwrap();
            let restored: BodyMaterialRecord = serde_json::from_str(&saved).unwrap();
            assert_eq!(restored, record);
            assert_eq!(encode_state(&restored.material()), encode_state(&material));
        }
        for (index, value) in [(0, 0.0_f64), (0, f64::NAN), (1, 2.0), (2, 1.0)] {
            let mut words = BodyMaterial::IDENTITY.record().0;
            words[index] = value.to_bits();
            let invalid = serde_json::to_string(&words).unwrap();
            assert!(serde_json::from_str::<BodyMaterialRecord>(&invalid).is_err());
        }
    }

    #[test]
    fn material_survives_a_body_snapshot_and_rejects_invalid_scale() {
        let body = crate::BodyKinematics {
            material: BodyMaterial::new(0.25, Mat2::from_cols(Vec2::Y, Vec2::X)).unwrap(),
            ..Default::default()
        };
        let bytes = encode_state(&body);
        assert_eq!(decode_state::<crate::BodyKinematics>(&bytes), Some(body));
        let mut invalid = bytes;
        invalid[28..36].copy_from_slice(&0.0_f64.to_le_bytes());
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
