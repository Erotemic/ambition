//! The authored-placement schema vocabulary — architecture.md §4b.
//!
//! These are the closed, serde-able authoring enums that authored maps
//! declare over content: brains (who a spawned actor becomes), the
//! damage relationship/category, and the hazard/prop respawn policy.
//! They are pure data (no `Vec2`, no runtime state, no Bevy) so they live
//! in the Tier-0 catalog — below every crate that interprets them. The
//! sim/content LOWERS these records into behavior at room-load; the arrow
//! is always sim/content → catalog, never the reverse.

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
/// own `CharacterArchetypeSpec` via `spec_for_brain`; the engine
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

/// The authored hazard schema — what a `DamageVolume`-style placement SAYS,
/// in plain pairs (the `HitVolume` idiom: `[f32; 2]`, never kernel types).
/// The lowering interpreter (W-queue step 3) converts to `Vec2`/components
/// once at room load; the legacy `DamageVolume` runtime type dissolves when
/// that interpreter lands ([W-a] verdict 3).
///
/// NOTE for step 3: legacy hazards may still carry an INLINE motion path
/// (`DamageVolume.motion`). The schema deliberately has `path_id` only —
/// dissolution lifts inline paths into room-level `KinematicPath` entries.
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

/// The authored interaction schema — what an interactable placement SAYS
/// (an NPC to talk to, a door, a chest/pickup/breakable prompt, or a
/// game-specific `Custom` payload). Fully plain data (no `Vec2`, no runtime
/// components) so it lives in the Tier-0 catalog; the interaction runtime
/// lowers it into live components at room load.
///
/// Moved down from `ambition_world::rooms` (fable audit F9.2 IR consolidation):
/// interactables now flow through the single `PlacementRecord` channel, so the
/// schema payload and the world IR share ONE pure type instead of a mirror.
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

/// The authored pickup schema — a collectible's reward category, respawn
/// policy, and collected flag. Fully plain data; the interaction runtime lowers
/// it to a live pickup component at room load. Moved down from
/// `ambition_world::rooms` (fable audit F9.2) so the schema and world IR share
/// ONE pure type carried on the single `PlacementRecord` channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PickupSpec {
    pub kind: PickupKindSpec,
    pub respawn: HazardRespawn,
    pub collected: bool,
}

impl PickupSpec {
    pub fn new(kind: PickupKindSpec) -> Self {
        Self {
            kind,
            respawn: HazardRespawn::Never,
            collected: false,
        }
    }
}

/// The authored reward category carried by [`PickupSpec`] (and a [`ChestSpec`]
/// reward).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PickupKindSpec {
    Health { amount: i32 },
    Currency { amount: i32 },
    Ability { ability_id: String },
    StoryFlag { flag: String },
    Custom(String),
}

/// The authored chest schema — open/closed state, an optional reward, and a
/// persistence flag. Fully plain data; the interaction runtime lowers it to a
/// live chest at room load. Moved down from `ambition_world::rooms` (fable audit
/// F9.2) onto the single `PlacementRecord` channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChestSpec {
    pub state: ChestStateSpec,
    pub reward: Option<PickupKindSpec>,
    pub persistent: bool,
}

impl ChestSpec {
    pub fn new(reward: Option<PickupKindSpec>) -> Self {
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

/// The authored breakable schema — destructible platform/orb with health,
/// collision, trigger, and debris cue. Fully plain data; lowered to a live
/// breakable at room load. Moved down from `ambition_world::rooms` (fable audit
/// F9.2) onto the single `PlacementRecord` channel.
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

/// Authored/runtime portal channel color. A pure enum (mirrors the Ambition
/// portal crate's color vocabulary) so it lives in the Tier-0 catalog; portal
/// lowerings map it to their runtime channel at the sim edge. Moved down from
/// `ambition_world::rooms` (fable audit F9.2).
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

/// The authored static-portal schema (the Tier-0 MIRROR of the runtime-facing
/// `ambition_world::rooms::PortalSpec`). Unlike the other placement families,
/// the runtime spec carries `Vec2` (`pos`/`normal`) and cannot itself live in
/// Tier-0, so this plain mirror stores `normal` as a pair and DERIVES the face
/// center from the placement record's `aabb.center()` at lowering time (the
/// converter sets `pos = box center = aabb.center()`). `half_length` is the
/// authored along-surface half-extent (also derivable from the aabb + normal,
/// stored explicitly to preserve the authored value exactly).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalSchema {
    pub color: PortalChannelColorSpec,
    /// Outward axis-aligned surface normal, pointing into the room.
    pub normal: [f32; 2],
    /// Explicit link id (`None` ⇒ legacy color pairing).
    pub link: Option<String>,
    /// Authored along-surface half-length (opening size); `None` ⇒ default.
    pub half_length: Option<f32>,
}

/// The CLOSED authored-placement schema (architecture.md §4b.3): everything an
/// authored map may declare beyond geometry, as editor-visible plain data.
/// Variants grow as W-queue step 3 converts hardcoded spawn branches into
/// registered lowering interpreters; `PlacementKind` (the fieldless mirror
/// keying the lowering registry) lands with the registry in that step.
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

/// Authored rule for when a defeated actor reappears (ADR 0022) — ONE
/// enum for every reappearance mechanic, authored per archetype row
/// (`respawn:` in `character_archetypes.ron`); a future EnemySpawn LDtk
/// field can override a single placement.
///
/// **The default is `DeadStaysDead`** — the intuitively-correct rule for
/// a unique actor in a persistent world ("Morrowind rules"). Respawning
/// is an AUTHOR'S choice: trash mobs opt into `OnRoomReenter`,
/// mini-boss-tier presences into `OnRest`, training dummies into
/// `InPlace(secs)`.
///
/// Mechanics: the kill hook in `damage/actor_hit.rs` matches this policy
/// — `InPlace` arms the in-place revive timer (no flag, no drops);
/// `DeadStaysDead` / `OnRest` write the persistent death flag their
/// respawn horizon implies; `OnRoomReenter` writes nothing. The
/// room-load `save_sync` reads the flags back into `alive = false`. A
/// "rest" event clears just the `_dead_until_rest` flags.
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
