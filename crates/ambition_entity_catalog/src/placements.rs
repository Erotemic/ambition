//! Pure authored placement schema lowered into runtime behavior by higher layers.

use crate::PickupKind;
use serde::{Deserialize, Serialize};

/// Damage/team relationship used by hitboxes and hurtboxes — the `can_damage`
/// matrix that decides whether one side's hit may affect another.
///
/// Deliberately distinct from `ActorFaction` (`ambition_characters`), which
/// is a `#[derive(Component)]` actor-side tag (`is_player_side`/`is_hostile_side`,
/// with `Npc`/`Boss` variants). This one is the *damage* relationship; that one
/// is the *ECS actor* tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageTeam {
    Player,
    Enemy,
    Neutral,
    Environment,
}

impl DamageTeam {
    /// True when damage from `self` is allowed to affect `target` by default.
    pub fn can_damage(self, target: Self) -> bool {
        match (self, target) {
            (Self::Player, Self::Enemy) => true,
            (Self::Enemy, Self::Player) => true,
            (Self::Environment, Self::Player | Self::Enemy | Self::Neutral) => true,
            (Self::Neutral, _) => false,
            _ => false,
        }
    }
}

/// The broad gameplay category of damage. This is intentionally separate from
/// presentation so hazards, attacks, and projectiles can share damage handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    Slash,
    Pogo,
    Contact,
    Hazard,
    Projectile,
    Environmental,
    Custom,
}

/// How temporary/destructible hazards/props return after being consumed or
/// killed. (The ADR-0022 *actor* `RespawnPolicy` is a distinct enum — this one
/// covers the authored hazard/prop lifecycle.)
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum HazardRespawn {
    /// Never respawn inside the current run/session.
    #[default]
    Never,
    /// Respawn after a timer in simulation seconds.
    AfterSeconds(f32),
    /// Respawn when the room is re-entered.
    OnRoomReload,
    /// The object is persistent and controlled by story/save state.
    Persistent,
}

/// Authored enemy behavior tag. The sandbox maps `Custom(name)` to its
/// own `ArchetypeSpec` via `spec_for_brain`; the engine
/// only carries this enum as a typed payload between LDtk authoring
/// and sandbox dispatch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CharacterBrain {
    Passive,
    Patrol { path_id: Option<String> },
    Guard { leash_radius: f32 },
    Custom(String),
}

/// Authored boss behavior tag. Same shape and contract as
/// `CharacterBrain`: the engine doesn't simulate against the variants;
/// the sandbox decides per-boss behavior from the payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BossBrain {
    Dormant,
    PhaseScript { script_id: String },
    Custom(String),
}

/// Authored hazard data, kept independent of runtime component types.
///
/// Moving hazards reference room-level `KinematicPath` entries by `path_id`;
/// inline runtime motion paths are not part of the authored schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HazardSpec {
    pub damage: i32,
    pub knockback: [f32; 2],
    pub kind: DamageKind,
    pub team: DamageTeam,
    pub hitstop_seconds: f32,
    pub respawn: HazardRespawn,
    /// Reference to a room-level `KinematicPath` (moving hazards).
    pub path_id: Option<String>,
}

/// Authored interaction data lowered to runtime components when the room loads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InteractableSpec {
    pub prompt: String,
    pub kind: InteractionKindSpec,
    pub requires_facing: bool,
    pub enabled: bool,
}

impl InteractableSpec {
    pub fn new(prompt: impl Into<String>, kind: InteractionKindSpec) -> Self {
        Self {
            prompt: prompt.into(),
            kind,
            requires_facing: false,
            enabled: true,
        }
    }
}

/// The authored interaction category carried by [`InteractableSpec`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InteractionKindSpec {
    Door {
        target: Option<String>,
    },
    Npc {
        character_id: Option<String>,
        dialogue_id: Option<String>,
        patrol_radius: f32,
        patrol_path_id: Option<String>,
        /// Explicit initial brain preset override (a `brain_presets` key). `None`
        /// / empty means use the character's catalog `default_brain`. A non-empty
        /// value names the preset this placement's brain is instantiated from,
        /// regardless of the character's default. The brain is NEVER selected by
        /// inspecting radius/path/hostility; this string is the authored choice,
        /// resolved by `ambition_characters`'s `resolve_initial_brain`.
        #[serde(default)]
        brain_override: Option<String>,
    },
    Chest,
    Pickup,
    Breakable,
    Custom(String),
}

/// Authored pickup reward, respawn policy, collection state, and optional presentation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PickupSpec {
    pub kind: PickupKind,
    pub respawn: HazardRespawn,
    pub collected: bool,
    /// Optional animated sprite sheet (a prop-kind key registered in
    /// `GameAssets.characters.props`). When set and resolvable, the render binds
    /// the pickup's visual as an idle-looping character sheet (a spinning ring, a
    /// pulsing gem) instead of the static per-kind entity sprite; unresolved or
    /// `None` falls back to the static coin/heart/ability art. Reward semantics
    /// stay on `kind` — this is presentation only.
    #[serde(default)]
    pub sprite: Option<String>,
}

impl PickupSpec {
    pub fn new(kind: PickupKind) -> Self {
        Self {
            kind,
            respawn: HazardRespawn::Never,
            collected: false,
            sprite: None,
        }
    }

    /// Author an animated sprite sheet (a `GameAssets` prop kind) for this pickup.
    pub fn with_sprite(mut self, sprite: impl Into<String>) -> Self {
        self.sprite = Some(sprite.into());
        self
    }
}


/// Authored chest state, optional reward, and persistence policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChestSpec {
    pub state: ChestStateSpec,
    pub reward: Option<PickupKind>,
    pub persistent: bool,
}

impl ChestSpec {
    pub fn new(reward: Option<PickupKind>) -> Self {
        Self {
            state: ChestStateSpec::Closed,
            reward,
            persistent: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChestStateSpec {
    Closed,
    Opening,
    Opened,
}

/// When a breakable's break is triggered (on hit, on being stood on, or either).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BreakableTriggerSpec {
    #[default]
    OnHit,
    OnStand,
    Either,
}

impl BreakableTriggerSpec {
    pub fn allows_hit(self) -> bool {
        matches!(
            self,
            BreakableTriggerSpec::OnHit | BreakableTriggerSpec::Either
        )
    }

    pub fn allows_stand(self) -> bool {
        matches!(
            self,
            BreakableTriggerSpec::OnStand | BreakableTriggerSpec::Either
        )
    }
}

/// How a breakable collides while intact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BreakableCollisionSpec {
    #[default]
    None,
    Solid,
    OneWayUp,
}

impl BreakableCollisionSpec {
    pub fn blocks_movement(self) -> bool {
        !matches!(self, BreakableCollisionSpec::None)
    }

    pub fn is_solid(self) -> bool {
        matches!(self, BreakableCollisionSpec::Solid)
    }
}

/// Authored breakable health, collision, trigger, respawn, and debris behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BreakableSpec {
    pub state: BreakableStateSpec,
    pub health_current: i32,
    pub health_max: i32,
    pub respawn: HazardRespawn,
    pub collision: BreakableCollisionSpec,
    pub trigger: BreakableTriggerSpec,
    pub debris_cue: Option<String>,
    pub pogo_refresh: bool,
}

impl BreakableSpec {
    pub fn new(max_hp: i32) -> Self {
        let max_hp = max_hp.max(1);
        Self {
            state: BreakableStateSpec::Intact,
            health_current: max_hp,
            health_max: max_hp,
            respawn: HazardRespawn::Never,
            collision: BreakableCollisionSpec::None,
            trigger: BreakableTriggerSpec::OnHit,
            debris_cue: None,
            pogo_refresh: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakableStateSpec {
    Intact,
    Cracking,
    Broken,
    Respawning,
}

/// Pure authored portal-channel color lowered to runtime portal vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortalChannelColorSpec {
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
    Indexed(u8),
}

impl PortalChannelColorSpec {
    pub fn partner(self) -> Self {
        use PortalChannelColorSpec::*;
        match self {
            Purple => Yellow,
            Yellow => Purple,
            Teal => Red,
            Red => Teal,
            Green => Magenta,
            Magenta => Green,
            Cyan => Rose,
            Rose => Cyan,
            Indexed(n) => Indexed(n ^ 1),
        }
    }

    pub fn name(self) -> String {
        use PortalChannelColorSpec::*;
        match self {
            Purple => "purple".into(),
            Yellow => "yellow".into(),
            Teal => "teal".into(),
            Red => "red".into(),
            Green => "green".into(),
            Magenta => "magenta".into(),
            Cyan => "cyan".into(),
            Rose => "rose".into(),
            Indexed(n) => format!("c{n}"),
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        use PortalChannelColorSpec::*;
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "purple" => Purple,
            "yellow" => Yellow,
            "teal" => Teal,
            "red" => Red,
            "green" => Green,
            "magenta" => Magenta,
            "cyan" => Cyan,
            "rose" => Rose,
            other => Indexed(other.strip_prefix('c')?.parse::<u8>().ok()?),
        })
    }
}

/// Authored static portal data without runtime vector types.
/// Position is derived from the placement AABB center during lowering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalSchema {
    pub color: PortalChannelColorSpec,
    /// Outward axis-aligned surface normal, pointing into the room.
    pub normal: [f32; 2],
    /// Explicit link id (`None`  legacy color pairing).
    pub link: Option<String>,
    /// Authored along-surface half-length (opening size); `None`  default.
    pub half_length: Option<f32>,
}

/// Closed authored placement payload keyed by [`PlacementKind`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlacementSchema {
    Hazard(HazardSpec),
    Interactable(InteractableSpec),
    Pickup(PickupSpec),
    Chest(ChestSpec),
    Breakable(BreakableSpec),
    Portal(PortalSchema),
}

/// Fieldless key for [`PlacementSchema`], used by the room-load lowering
/// registry. This stays beside the schema so a new authored placement variant
/// cannot forget to expose its registry key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlacementKind {
    Hazard,
    Interactable,
    Pickup,
    Chest,
    Breakable,
    Portal,
}

impl PlacementKind {
    /// Stable construction-schema identity; unlike `Debug`, this is an explicit
    /// compatibility contract and may only change with a fingerprint-schema bump.
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Hazard => "hazard",
            Self::Interactable => "interactable",
            Self::Pickup => "pickup",
            Self::Chest => "chest",
            Self::Breakable => "breakable",
            Self::Portal => "portal",
        }
    }
}

impl PlacementSchema {
    pub const fn kind(&self) -> PlacementKind {
        match self {
            Self::Hazard(_) => PlacementKind::Hazard,
            Self::Interactable(_) => PlacementKind::Interactable,
            Self::Pickup(_) => PlacementKind::Pickup,
            Self::Chest(_) => PlacementKind::Chest,
            Self::Breakable(_) => PlacementKind::Breakable,
            Self::Portal(_) => PlacementKind::Portal,
        }
    }
}

/// Spawn-context disposition for this placement.
///
/// Disposition belongs to the placement rather than the character template: the
/// same character may be hostile in one context and peaceful in another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnDisposition {
    /// Seeks a target and publishes contact damage. The ordinary enemy.
    #[default]
    Hostile,
    /// Dormant until something provokes it: ambient wildlife, a peaceful crew.
    Peaceful,
}

impl SpawnDisposition {
    /// Whether a body with this disposition attacks on sight.
    ///
    /// Target selection remains the responsibility of faction/team targeting rules.
    pub fn is_hostile(self) -> bool {
        matches!(self, Self::Hostile)
    }
}

/// Authored rule for when a defeated actor reappears.
///
/// `DeadStaysDead` is the default; respawning must be selected explicitly per archetype/placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum RespawnPolicy {
    /// Dead stays dead — forever (an explicit save reset is the only
    /// return). THE DEFAULT: named/unique actors take it implicitly.
    #[default]
    DeadStaysDead,
    /// Stays dead until the player rests at a save point
    /// (mini-boss-tier presences: brutes, colossi, pirate heavies).
    OnRest,
    /// Fresh every time the player enters the room — the "Mob" choice
    /// (trash grunts: skitters, lurkers, raiders, goblins).
    OnRoomReenter,
    /// Revives in place this many seconds after death, where it stood
    /// (training sandbags). No death drops, no flag.
    InPlace(f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_damage_affects_player_and_enemy() {
        assert!(DamageTeam::Environment.can_damage(DamageTeam::Player));
        assert!(DamageTeam::Environment.can_damage(DamageTeam::Enemy));
        assert!(!DamageTeam::Player.can_damage(DamageTeam::Player));
    }

    #[test]
    fn can_damage_matrix_encodes_the_friendly_fire_rules() {
        use DamageTeam::{Enemy, Environment, Neutral, Player};
        // The two combat loops cross faction lines.
        assert!(Player.can_damage(Enemy), "player hits enemies");
        assert!(Enemy.can_damage(Player), "enemies hit the player");
        // No same-faction friendly fire.
        assert!(!Player.can_damage(Player));
        assert!(!Enemy.can_damage(Enemy));
        // Environment (hazards) hits everything except itself.
        assert!(Environment.can_damage(Neutral));
        // Neutral never deals damage; nothing targets it offensively except
        // the environment.
        assert!(!Neutral.can_damage(Player));
        assert!(!Neutral.can_damage(Enemy));
        assert!(!Player.can_damage(Neutral));
        assert!(!Enemy.can_damage(Neutral));
        assert!(!Player.can_damage(Environment));
    }
}
