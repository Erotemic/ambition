//! Durable geometry identity — `GeoId`/`GeoFaceRef` (collision-and-ccd.md §3.6).
//!
//! What durably names a piece of ROOM geometry — for `WorldDelta` ops, the CC6
//! portal host ref, save overlays, and debug traces? `Block.name` is an informal
//! display string; this is the stable identity. Two-level: WHERE the geometry
//! came from + its deterministic ordinal within that source's emission.
//!
//! **This is the identity SUBSTRATE only.** The types exist so `Block` can carry
//! an id and the emission paths can assign real sources incrementally; no delta
//! or portal consumer is implemented here (that is CC6 / W-c). Fixture/test
//! geometry uses [`GeoId::anon`]; only the authored IR emission paths assign real
//! sources. Carve pieces / split blocks / per-frame composition products are
//! DERIVED state and are NEVER named by a persisted id (§3.6 rule 2).

/// Stable per-placement identity: an LDtk `iid` (or a baked/generated room's
/// synthesized `"{room}:{index}"`). The [W-d] record-layer id, lifted to
/// engine_core because `GeoSource` (geometry vocabulary) names it and Tier-0/2
/// both sit around this crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlacementId(pub String);

impl PlacementId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// WHERE a piece of geometry came from — the durable half of a [`GeoId`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GeoSource {
    /// Entity-authored geometry (a Solid/OneWay/SurfaceChain LDtk entity, or any
    /// backend's placement): the placement id IS the identity.
    Placement(PlacementId),
    /// Grid/tile-derived geometry (IntGrid merge → solid rects): keyed by layer
    /// name; the `GeoId.index` is the merge ordinal. The merger MUST iterate
    /// deterministically (row-major over the grid) so the same map always yields
    /// the same ids — that determinism is part of this contract.
    TileLayer { layer: String },
    /// Output of a parameterized generator marker (`SurfaceLoop`, `SurfaceRamp`):
    /// the MARKER's placement id + the emission ordinal (segment k of the arc).
    Generator(PlacementId),
    /// Geometry ADDED by a `WorldDelta` op (a dug tunnel's new wall): the op's
    /// sequence number in the room's delta list is durable because it is IN the
    /// save.
    Delta { op_index: u32 },
    /// Test/fixture geometry. The authoring pipeline NEVER emits this; the
    /// delta/save layer REJECTS ops naming it (validator lands with the first
    /// delta op).
    Anon,
}

/// Durable identity of one piece of ROOM geometry: WHERE it came from + its
/// deterministic ordinal within that source's emission.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GeoId {
    pub source: GeoSource,
    pub index: u16,
}

impl GeoId {
    /// Fixture/test identity: the authoring pipeline never emits `Anon`, so tests
    /// can construct geometry without threading a real source (§3.6 rule 1).
    pub fn anon() -> Self {
        Self {
            source: GeoSource::Anon,
            index: 0,
        }
    }

    /// A placement-sourced id (LDtk iid / bake-synth), ordinal within that
    /// placement's emission (0 for the common single-block placement).
    pub fn placement(id: PlacementId, index: u16) -> Self {
        Self {
            source: GeoSource::Placement(id),
            index,
        }
    }

    /// A tile-layer-sourced id: the layer name + the row-major merge ordinal.
    pub fn tile_layer(layer: impl Into<String>, index: u16) -> Self {
        Self {
            source: GeoSource::TileLayer {
                layer: layer.into(),
            },
            index,
        }
    }
}

impl Default for GeoId {
    fn default() -> Self {
        Self::anon()
    }
}

/// Which face of a piece of geometry a [`GeoFaceRef`] names.
///
/// AABB blocks use the four world-axis faces (`+y` grows DOWN, so `Top` is the
/// `min.y` face). Chains/polygons use `Segment(k)` — the polyline segment index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Segment(u16),
}

/// A face + position on identified geometry — the "host face" vocabulary moving
/// portals, deltas, and traces share (the CC6 `PortalHostRef`).
#[derive(Clone, Debug, PartialEq)]
pub struct GeoFaceRef {
    pub geo: GeoId,
    pub face: Face,
    /// px offset from the face's CENTER, tangent-signed. (px, not normalized —
    /// geometry doesn't resize; px is what placement math uses today.)
    pub along: f32,
}

impl GeoFaceRef {
    pub fn new(geo: GeoId, face: Face, along: f32) -> Self {
        Self { geo, face, along }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_is_anon() {
        assert_eq!(GeoId::default(), GeoId::anon());
        assert_eq!(GeoId::anon().source, GeoSource::Anon);
    }

    #[test]
    fn placement_and_tile_layer_ids_are_distinct_and_hashable() {
        // Two placement blocks with the same iid but different ordinals differ;
        // a tile-layer id keys by layer + merge ordinal. All usable as map keys.
        let a = GeoId::placement(PlacementId::new("iid-1"), 0);
        let b = GeoId::placement(PlacementId::new("iid-1"), 1);
        let t = GeoId::tile_layer("Collision", 7);
        assert_ne!(a, b);
        assert_ne!(a, t);

        let mut index: HashMap<GeoId, &str> = HashMap::new();
        index.insert(a.clone(), "a");
        index.insert(b.clone(), "b");
        index.insert(t.clone(), "t");
        assert_eq!(index.get(&a), Some(&"a"));
        assert_eq!(index.get(&b), Some(&"b"));
        assert_eq!(index.get(&t), Some(&"t"));
    }

    #[test]
    fn geo_face_ref_names_a_face_and_offset() {
        let f = GeoFaceRef::new(
            GeoId::placement(PlacementId::new("iid-9"), 0),
            Face::Top,
            12.5,
        );
        assert_eq!(f.face, Face::Top);
        assert_eq!(f.along, 12.5);
        assert_eq!(
            f.geo.source,
            GeoSource::Placement(PlacementId::new("iid-9"))
        );
    }
}
