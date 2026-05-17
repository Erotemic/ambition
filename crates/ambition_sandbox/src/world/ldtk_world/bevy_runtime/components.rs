use bevy::prelude::{Bundle, Component};
use bevy_ecs_ldtk::prelude::{EntityInstance as PluginEntityInstance, LdtkEntity};

/// Lightweight bundle registered for every Ambition-authored LDtk entity.
///
/// This makes `bevy_ecs_ldtk` the owner of LDtk entity lifecycle/identity
/// without letting the plugin render its default unregistered-entity
/// placeholders. Ambition systems then consume the spawned `EntityInstance`
/// component and attach gameplay semantics deliberately.
#[derive(Bundle, LdtkEntity, Default)]
pub struct AmbitionLdtkMarkerBundle {
    #[from_entity_instance]
    pub entity_instance: PluginEntityInstance,
    pub marker: AmbitionLdtkMarker,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct AmbitionLdtkMarker;

#[derive(Component, Clone, Debug)]
pub struct AmbitionLdtkEntity {
    pub iid: String,
    pub identifier: String,
    pub px: [i32; 2],
    pub size: [i32; 2],
    pub world: Option<[i32; 2]>,
}

impl AmbitionLdtkEntity {
    pub fn summary(&self) -> String {
        let world = self
            .world
            .map(|world| format!(" world=({}, {})", world[0], world[1]))
            .unwrap_or_default();
        format!(
            "{} {} px=({}, {}) size={}x{}{}",
            self.identifier, self.iid, self.px[0], self.px[1], self.size[0], self.size[1], world
        )
    }
}

/// Ambition-facing role for a plugin-spawned LDtk entity.
///
/// These are deliberately narrower than the full LDtk identifier set. The
/// first promoted runtime-spine categories are the low-risk entities that
/// should be observable directly from `bevy_ecs_ldtk` before we migrate
/// collision and gameplay-heavy objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LdtkRuntimeRole {
    PlayerStart,
    LoadingZone,
    DebugLabel,
    CameraZone,
    Solid,
    OneWayPlatform,
    DamageVolume,
    Other,
}

impl LdtkRuntimeRole {
    pub fn from_identifier(identifier: &str) -> Self {
        match identifier {
            "PlayerStart" => Self::PlayerStart,
            "LoadingZone" => Self::LoadingZone,
            "DebugLabel" => Self::DebugLabel,
            "CameraZone" => Self::CameraZone,
            "Solid" => Self::Solid,
            "OneWayPlatform" => Self::OneWayPlatform,
            "DamageVolume" | "HazardBlock" => Self::DamageVolume,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PlayerStart => "player starts",
            Self::LoadingZone => "loading zones",
            Self::DebugLabel => "debug labels",
            Self::CameraZone => "camera zones",
            Self::Solid => "solids",
            Self::OneWayPlatform => "one-way platforms",
            Self::DamageVolume => "damage volumes",
            Self::Other => "other",
        }
    }

    pub fn promoted(self) -> bool {
        !matches!(self, Self::Other)
    }
}

/// Typed Ambition collision component attached to plugin-spawned `Solid`
/// entities.
///
/// The first collision-heavy LDtk category to leave the JSON-only adapter path:
/// while `compose_runtime_area` still produces `ae::Block::solid()` entries for
/// the runtime collision world, every spawned `Solid` LDtk entity now also
/// carries this typed component so future systems can query ECS-side without
/// reparsing the LDtk file. Once the raw-LDtk-vs-runtime overlay (Step 2 of the
/// LDtk roadmap) verifies parity, the JSON path can be retired and these
/// components become collision authority.
#[derive(Component, Clone, Debug, Default)]
pub struct LdtkSolid {
    /// Top-left corner in LDtk-level-local pixel coordinates.
    pub level_px: [i32; 2],
    /// Width and height in pixels.
    pub size: [i32; 2],
}

/// Typed Ambition component attached to plugin-spawned `OneWayPlatform` entities.
///
/// Same shape as `LdtkSolid` — the JSON adapter still produces the
/// matching `ae::Block::one_way_up()` for the runtime collision world,
/// but the typed component lets gameplay/debug systems query ECS-side
/// instead of reparsing identifiers. Step in the LDtk runtime-spine
/// roadmap that mirrors `LdtkSolid`.
#[derive(Component, Clone, Debug, Default)]
pub struct LdtkOneWayPlatform {
    pub level_px: [i32; 2],
    pub size: [i32; 2],
}

/// Typed Ambition component attached to plugin-spawned `DamageVolume`
/// (and the legacy `HazardBlock`) entities.
///
/// The JSON adapter still produces the matching `ae::Block::hazard(...)`
/// for the runtime collision world; this component is the typed
/// sibling for ECS-side query and the parity overlay.
#[derive(Component, Clone, Debug, Default)]
pub struct LdtkDamageVolume {
    pub level_px: [i32; 2],
    pub size: [i32; 2],
    /// Damage amount (1 by default) — sandbox doesn't yet expose
    /// per-volume damage in the LDtk schema, so this defaults to 1
    /// and future LDtk field reads can populate it.
    pub damage: i32,
}
