//! Explicit NPC brain authority.
//!
//! An NPC's brain is determined ONLY by its explicit authored override or the
//! character catalog default — never by inspecting the resulting [`Brain`].
//! There is no "basic brain" classification, no `is_hostile` gate, no
//! `patrol_radius == 0` sentinel, and no peaceful-specialized bypass.
//!
//! The pieces:
//! - [`BrainPresetId`] — a key into the catalog `brain_presets` map.
//! - [`BrainBinding`] — the runtime component recording the character's default
//!   preset plus its current [`AutonomousSource`] (catalog default / catalog
//!   preset override / provoked hostile archetype). This is the authoritative
//!   snapshot state for "which autonomous actor mode should exist when no
//!   temporary controller masks it".
//! - [`AuthoredBrainContext`] — the per-spawn parameters a selected preset
//!   consumes (patrol lane anchor + radius), captured at spawn and retained so a
//!   runtime rebuild uses the actor's authored home, not wherever it wandered to.
//! - [`resolve_initial_brain`] — override → character default → clear error.
//!
//! Precedence and namespace resolution are deterministic (see
//! [`resolve_initial_brain`]). Once an actor carries a [`BrainBinding`], runtime
//! gameplay switches its brain through the authoritative command path
//! (`ambition_platformer2d_actor_monolith`'s `BrainCommand`), which rebuilds the [`Brain`] via
//! [`CharacterCatalog::build_brain_from_preset`] — the same seam this module uses
//! at spawn, so a preset resolves identically at spawn and at a later switch.

use super::CharacterCatalog;
use crate::brain::Brain;
use bevy::prelude::Component;

/// Stable id of a brain preset — a key into the catalog `brain_presets` map.
///
/// A newtype over `String` so a preset id can't be silently confused with a
/// character id or a bare string in a signature.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BrainPresetId(pub String);

impl BrainPresetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrainPresetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrainPresetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BrainPresetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Authored, provider-relative reference to a brain profile.
///
/// A reference such as `"combatant"` is qualified against the character's
/// namespace before becoming the canonical [`BrainPresetId`] such as
/// `"hall::combatant"`. Resolution currently occurs at spawn.
/// TODO(compat-remove): resolve brain references during preparation and carry
/// `BrainPresetId` in prepared definitions.
///
/// Distinct from `content_schema::BrainPresetRef`, the zero-sized validator tag.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BrainPresetRef(pub String);

impl BrainPresetRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    /// The authored text, exactly as content wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrainPresetRef {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrainPresetRef {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BrainPresetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A stable id for a boss's authored autonomous mode — the boss's catalog id
/// (`BossConfig.id`), the key `ambition_platformer2d_actor_monolith`' `BossCatalog` resolves a
/// `BossBehaviorProfile` from. A newtype over `String` so it can't be confused
/// with a character id, a catalog preset id, or a hostile archetype id.
///
/// It is *just* a carrier: `ambition_characters` never interprets it (the boss catalog and the `id → BossPatternCfg` projection
/// live in `ambition_platformer2d_actor_monolith`). It is what an [`AutonomousSource::Boss`] retains so
/// a possessed boss has a reconstructible autonomous mode independent of the live
/// (masked) `Brain`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BossAutonomyId(pub String);

impl BossAutonomyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BossAutonomyId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BossAutonomyId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BossAutonomyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which *autonomous* actor mode a catalog-backed actor should be in when no
/// temporary controller (player possession / mount) is masking it.
///
/// The mutable half of a [`BrainBinding`]. Every variant is a STABLE,
/// reconstructible source — a snapshot restores this and the whole autonomous
/// configuration (live `Brain`, action set, disposition, archetype tuning) is
/// rebuilt from it deterministically. There is no lossy "some other authority
/// owns the live brain" escape hatch: a provoked actor names its hostile
/// archetype, so a rewind can rerun the roster construction that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutonomousSource {
    /// The character's catalog default preset.
    CatalogDefault,
    /// An explicit catalog preset override (authored `brain_override`, or a
    /// runtime `UsePreset` switch).
    CatalogPreset(BrainPresetId),
    /// PROVOKED, WITH NOTHING TO LOOK UP — the body is driven by the
    /// engine's default provoked policy.
    ///
    ///  payloadless for the same reason [`Self::CharacterProfile`] is: the
    /// answer is not somewhere to look up, it is a thing the engine states.
    /// `brain_builders::default_provoked_policy()`, whose home is the session
    /// ruleset when a second experience wants a different one.
    ///
    ///  a creature that states its OWN provoked policy records
    /// [`Self::ProvokedProfile`] instead and never reaches this — which is the
    /// distinction the two variants exist to keep, and the reason this one can
    /// be payloadless without losing anything.
    ProvokedDefault,
    /// A provoked character's autonomous policy, named by stable profile id.
    /// The body remains unchanged; rollback stores the resolvable id rather than
    /// embedding the profile's tuning values.
    ProvokedProfile {
        profile: ambition_entity_catalog::BrainProfileId,
    },
    /// Use the character's own authored autonomous profile as the default.
    /// Carries no payload: body identity identifies the character, and lowering
    /// the profile requires that body's capabilities and movement parameters.
    CharacterProfile,
    /// A boss's authored autonomous mode. The live brain is a `BossPattern`
    /// rebuilt from the boss catalog by this id (never a catalog preset). A boss
    /// carries this so it has a reconstructible autonomous source when a player
    /// possession masks its live brain.
    Boss { archetype: BossAutonomyId },
}

/// Runtime source to restore when temporary control ends.
///
/// Preset-backed, character-profile, and absent defaults are distinct so restore
/// logic never encodes one source as an empty id or guesses which authority to use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutonomousDefault {
    /// Nothing to restore. A boss rebuilds its autonomous mode from its own
    /// authority; there is no default to go back to here.
    None,
    /// The character's catalog `default_brain`, captured at spawn. Restoring
    /// rebuilds a fresh brain from THIS preset.
    Preset(BrainPresetId),
    /// The character's own authored [`BrainProfile`] is the default. Nothing
    /// to look up — the body carries the policy its character stated, and
    /// lowering it needs the body anyway.
    ///
    /// [`BrainProfile`]: crate::brain::BrainProfile
    CharacterProfile,
}

impl AutonomousDefault {
    /// The preset to rebuild from, when the default IS a preset.
    pub fn preset(&self) -> Option<&BrainPresetId> {
        match self {
            Self::Preset(id) => Some(id),
            Self::None | Self::CharacterProfile => None,
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BrainBinding {
    /// What restoring this body's autonomous default means — see
    /// [`AutonomousDefault`].
    pub default_preset: AutonomousDefault,
    /// Which autonomous source is live right now.
    pub source: AutonomousSource,
}

impl BrainBinding {
    /// A catalog-backed binding: `default_preset` is the character's catalog
    /// default, always present.
    pub fn new(default_preset: BrainPresetId, source: AutonomousSource) -> Self {
        Self {
            default_preset: AutonomousDefault::Preset(default_preset),
            source,
        }
    }

    /// A binding whose default is the character's own authored policy.
    pub fn from_character_profile() -> Self {
        Self {
            default_preset: AutonomousDefault::CharacterProfile,
            source: AutonomousSource::CharacterProfile,
        }
    }

    /// A boss binding: no catalog default preset; the autonomous source is the
    /// boss's authored [`AutonomousSource::Boss`] mode.
    pub fn boss(archetype: BossAutonomyId) -> Self {
        Self {
            default_preset: AutonomousDefault::None,
            source: AutonomousSource::Boss { archetype },
        }
    }

    /// The catalog preset in effect right now: the override if one is selected,
    /// the character default when on the default, or `None` when the actor's live
    /// brain is not a catalog preset (provoked archetype / boss / no default).
    pub fn active_preset(&self) -> Option<&BrainPresetId> {
        match &self.source {
            AutonomousSource::CatalogPreset(id) => Some(id),
            AutonomousSource::CatalogDefault => self.default_preset.preset(),
            AutonomousSource::ProvokedDefault
            | AutonomousSource::ProvokedProfile { .. }
            | AutonomousSource::CharacterProfile
            | AutonomousSource::Boss { .. } => None,
        }
    }

    /// True iff an override preset is currently selected.
    pub fn is_override(&self) -> bool {
        matches!(self.source, AutonomousSource::CatalogPreset(_))
    }

    /// True iff the actor is currently provoked into the engine's default
    /// policy. (A creature provoked into its OWN authored policy records
    /// [`AutonomousSource::ProvokedProfile`]; ask that separately.)
    pub fn is_provoked(&self) -> bool {
        matches!(self.source, AutonomousSource::ProvokedDefault)
    }

    /// True iff the actor's autonomous source is a boss mode.
    pub fn is_boss(&self) -> bool {
        matches!(self.source, AutonomousSource::Boss { .. })
    }

    /// The boss archetype id, if this is a boss binding.
    pub fn boss_archetype(&self) -> Option<&BossAutonomyId> {
        match &self.source {
            AutonomousSource::Boss { archetype } => Some(archetype),
            _ => None,
        }
    }

    /// Switch to an override preset. (The caller rebuilds the live `Brain`.)
    pub fn use_preset(&mut self, preset: BrainPresetId) {
        self.source = AutonomousSource::CatalogPreset(preset);
    }

    /// Return to the character default. (The caller rebuilds the live `Brain`.)
    pub fn restore_default(&mut self) {
        //  THE SOURCE HAS TO MATCH THE DEFAULT. This wrote `CatalogDefault`
        // unconditionally, which was harmless only while every default WAS a catalog preset.
        self.source = match self.default_preset {
            AutonomousDefault::CharacterProfile => AutonomousSource::CharacterProfile,
            AutonomousDefault::Preset(_) | AutonomousDefault::None => {
                AutonomousSource::CatalogDefault
            }
        };
    }

    /// Record that the actor was provoked into the engine's default policy. The
    /// caller rebuilds the coupled autonomous state (brain / action set) from
    /// that policy; a snapshot reconstructs it the same way, so the provoked
    /// mode survives a rewind in both directions.
    pub fn provoke(&mut self) {
        self.source = AutonomousSource::ProvokedDefault;
    }
}

/// Per-spawn parameters a *selected* brain preset consumes when instantiated.
///
/// These parameterize a preset that was already chosen; they never SELECT it. A
/// [`Patrol`](crate::brain::state_machine::StateMachineCfg::Patrol) preset
/// consumes `spawn_world_x` for its lane center and, when authored, `patrol_radius`
/// as a lane-radius override. Every non-patrol preset ignores the patrol field.
///
/// This is the transient build INPUT. The persistent, snapshot-safe form an actor
/// carries is [`AuthoredBrainContext`], which produces one of these.
#[derive(Clone, Debug, PartialEq)]
pub struct BrainBuildContext {
    /// The patrol lane center anchor (world X).
    pub spawn_world_x: f32,
    /// Placement lane-radius override for a selected patrol preset. `None` keeps
    /// the preset's authored radius.
    pub patrol_radius: Option<f32>,
}

impl BrainBuildContext {
    /// A context that only anchors the patrol lane center (no radius override).
    pub fn at(spawn_world_x: f32) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: None,
        }
    }

    /// A context carrying a placement's authored patrol radius. A non-positive
    /// `patrol_radius` is treated as "unset" (keep the preset's radius).
    pub fn from_placement(spawn_world_x: f32, patrol_radius: f32) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: (patrol_radius > 0.0).then_some(patrol_radius),
        }
    }
}

/// The authored brain-build context an actor carries for the life of the entity.
///
/// Captured at spawn from the placement (its world anchor + authored patrol
/// radius) and kept through runtime brain switches, so `RestoreDefault` / a
/// snapshot reconcile rebuild a patrol brain around its authored HOME, not
/// wherever the actor happened to wander. A separate component (rather than a
/// field on [`BrainBinding`]) so the binding stays compact and equality-friendly.
///
/// Note on patrol PATHS: a `path_id` is a separate movement attachment
/// (`ActorMotionPath`), not a brain-build parameter — the lane-based patrol
/// preset does not consume one. So it is deliberately absent here.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AuthoredBrainContext {
    /// The actor's authored world-space spawn X — the patrol lane center anchor.
    pub spawn_anchor_x: f32,
    /// The placement's authored patrol-radius override, if any.
    pub patrol_radius: Option<f32>,
}

impl AuthoredBrainContext {
    /// Capture a placement's authored patrol tuning. A non-positive radius is
    /// treated as "unset".
    pub fn from_placement(spawn_anchor_x: f32, patrol_radius: f32) -> Self {
        Self {
            spawn_anchor_x,
            patrol_radius: (patrol_radius > 0.0).then_some(patrol_radius),
        }
    }

    /// The transient build input a preset resolver consumes.
    pub fn build_context(&self) -> BrainBuildContext {
        BrainBuildContext {
            spawn_world_x: self.spawn_anchor_x,
            patrol_radius: self.patrol_radius,
        }
    }
}

/// Which side of the precedence chain a failing preset came from — for a clear
/// spawn/validation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetSource {
    /// The placement's explicit `brain_override`.
    Override,
    /// The character catalog `default_brain`.
    CharacterDefault,
}

impl std::fmt::Display for PresetSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Override => f.write_str("brain_override"),
            Self::CharacterDefault => f.write_str("catalog default_brain"),
        }
    }
}

/// Why an initial-brain resolution failed. Both variants are content errors: an
/// unknown character id, or a named preset that isn't in the catalog. Neither
/// falls back silently — the spawn/validation site fails loud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainBuildError {
    /// `character_id` is not in the catalog.
    UnknownCharacter(String),
    /// The selected preset name is not in `brain_presets`. `resolved` is the key
    /// actually looked up after namespace qualification.
    UnknownPreset {
        character_id: String,
        preset: String,
        resolved: String,
        source: PresetSource,
    },
    /// NOBODY NAMES A PRESET, and no placement override was given.
    ///
    ///  it is NOT necessarily an error. A character that states a
    /// [`BrainProfile`] instead is fully authored — this is the signal to ask
    /// THAT authority, which only a caller holding the body can do.
    ///
    /// [`BrainProfile`]: crate::brain::BrainProfile
    NoAutonomousDefault { character_id: String },
}

impl std::fmt::Display for BrainBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCharacter(id) => {
                write!(
                    f,
                    "unknown character_id `{id}` (not in the character catalog)"
                )
            }
            Self::UnknownPreset {
                character_id,
                preset,
                resolved,
                source,
            } => write!(
                f,
                "character `{character_id}`: {source} names unknown brain preset `{preset}` \
                 (resolved to `{resolved}`, not in brain_presets)"
            ),
            Self::NoAutonomousDefault { character_id } => write!(
                f,
                "character `{character_id}`: nothing names an autonomous default — its \
                 definition states no brain profile and its catalog row's `default_brain` \
                 is empty. A caller holding the body must ask the character's own \
                 `BrainProfile`; there is no preset to build."
            ),
        }
    }
}

impl std::error::Error for BrainBuildError {}

/// Qualify a (possibly raw) local brain-preset name into the namespace of
/// `reference` — a fully-qualified preset name like `"provider::name"`.
///
/// The assembled [`CharacterCatalog`] namespaces every provider's brain presets
/// by owner (`"default::stand_still"`), and rewrites each character's
/// `default_brain` to match. Authoring surfaces (an LDtk `brain_override`, a
/// `<<use_brain>>` command) name a RAW local preset, so we qualify it into the
/// SAME namespace the character's `default_brain` lives in — passing `reference =
/// entry.default_brain` (or an actor's `BrainBinding::default_preset`). If `local`
/// is already qualified, or `reference` is un-namespaced (a raw / single-fragment
/// catalog, e.g. in tests), `local` is returned unchanged.
///
/// This is the whole namespace rule: a fully-qualified override is used exactly;
/// a raw override resolves ONLY within the character's own provider namespace.
/// There is no silent cross-provider fallback — a raw override the character's
/// provider does not own is a content error, not a lookup into some other
/// provider's presets.
pub fn qualify_preset_like(reference: &str, local: &str) -> String {
    if local.contains("::") {
        return local.to_string();
    }
    match reference.rsplit_once("::") {
        Some((provider, _)) => format!("{provider}::{local}"),
        None => local.to_string(),
    }
}

/// Qualify a local brain-profile reference into a PROVIDER's namespace.
///
///  the same rule as [`qualify_preset_like`], stated against the authority
/// that actually owns the namespace. That function infers the namespace from
/// a neighbouring key (`entry.default_brain`), which works only where a catalog
/// row is in hand — so a character's own prepared default needed a parallel
/// catalog row merely to learn its own provider. A `CharacterDefinition` states
/// its provider directly, and the assembled catalog namespaces every preset as
/// `provider::name` with the SAME provider id a definition carries. That
/// equality is guarded, not assumed: see
/// `character_definitions_and_catalog_fragments_share_one_provider_namespace`.
///
/// An already-qualified reference is used exactly, so a character may point at
/// another provider's profile deliberately; a raw one never silently falls
/// through to some other provider's presets.
pub fn qualify_in_provider(provider: &str, local: &str) -> String {
    if local.contains("::") || provider.is_empty() {
        return local.to_string();
    }
    format!("{provider}::{local}")
}

/// Resolve the initial brain for a placed NPC.
///
/// `authored_override` is the raw `brain_override` field: `None`/empty/whitespace
/// means the character default; a non-empty value is an explicit preset override.
/// Precedence: explicit override → character catalog default → clear error.
///
/// Namespace resolution is deterministic ([`qualify_preset_like`]): a raw override
/// resolves within the character's own provider namespace; a fully-qualified
/// override is used exactly. An override that does not resolve is a loud
/// [`BrainBuildError::UnknownPreset`] — never a silent fall back to the default.
///
/// Returns both the runtime [`BrainBinding`] (for snapshot + later runtime
/// switching) and a freshly instantiated [`Brain`]. This function NEVER inspects
/// the resulting brain to decide anything.
pub fn resolve_initial_brain(
    catalog: &CharacterCatalog,
    character_id: &str,
    authored_override: Option<&str>,
    ctx: &BrainBuildContext,
) -> Result<(BrainBinding, Brain), BrainBuildError> {
    let entry = catalog
        .get(character_id)
        .ok_or_else(|| BrainBuildError::UnknownCharacter(character_id.to_string()))?;
    //  AN ABSENT DEFAULT IS `None`, NEVER `BrainPresetId::new("")`.
    //
    // The field is already `Option`, and its own doc already said what `None` means; the
    // constructor was the one place that could not say it.
    let default_preset = Some(BrainPresetId::new(entry.default_brain.clone()))
        .filter(|id| !id.as_str().trim().is_empty());

    // Interpret the authored field: empty/whitespace means "use the default".
    let override_name = authored_override.map(str::trim).filter(|s| !s.is_empty());

    let (source, preset_source, resolved_key) = match override_name {
        Some(name) => {
            // Authoring names a RAW local preset; qualify it into the character's
            // namespace so it matches the assembled catalog's `provider::name`
            // keys. The binding stores the QUALIFIED name so the runtime switch
            // path and snapshot reconcile resolve it identically.
            //
            //  THE ENTRY'S OWN PROVIDER FIRST — and this site is why the fourth attempt to
            // empty a `default_brain` still failed. `CharacterCatalog::validate_brain_override`
            // had been taught to ask the provider; THIS one had not, and it is the one the NPC
            // road reaches.
            //
            //  the fallbacks are ORDERED so the field that may be absent is
            // last: `default_action_set` is still required of every row,
            // `default_brain` is not.
            let namespace_carrier = if !entry.provider.is_empty() {
                format!("{}::_", entry.provider)
            } else if !entry.default_action_set.is_empty() {
                entry.default_action_set.clone()
            } else {
                entry.default_brain.clone()
            };
            let key = qualify_preset_like(&namespace_carrier, name);
            (
                AutonomousSource::CatalogPreset(BrainPresetId::new(key.clone())),
                PresetSource::Override,
                key,
            )
        }
        //  NO OVERRIDE AND NO PRESET IS NOT AN ERROR HERE — it is a
        // REDIRECTION. The character may state its policy as a `BrainProfile`,
        // which this function cannot lower (the lowering needs the body). Say so
        // precisely and let the caller, which has one, ask that authority.
        None => {
            let Some(default) = default_preset.clone() else {
                return Err(BrainBuildError::NoAutonomousDefault {
                    character_id: character_id.to_string(),
                });
            };
            (
                AutonomousSource::CatalogDefault,
                PresetSource::CharacterDefault,
                default.0.clone(),
            )
        }
    };
    let binding = BrainBinding {
        default_preset: match default_preset {
            Some(id) => AutonomousDefault::Preset(id),
            None => AutonomousDefault::None,
        },
        source,
    };

    let brain = catalog
        .build_brain_from_preset(&resolved_key, ctx)
        .ok_or_else(|| BrainBuildError::UnknownPreset {
            character_id: character_id.to_string(),
            preset: override_name.unwrap_or(&resolved_key).to_string(),
            resolved: resolved_key,
            source: preset_source,
        })?;
    Ok((binding, brain))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::character_catalog::{parse_catalog, CharacterCatalog};
    use crate::brain::{Brain, StateMachineCfg};

    // A fixture roster: a peaceful WANDERER default (puppy slug), a HOSTILE
    // MeleeBrute default (a fighter placed as a talkable NPC), and a PATROL
    // default whose lane radius a placement can override.
    const CATALOG: &str = r#"(
        brain_presets: {
            "stand_still": StandStill,
            "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
            "melee_brute_striker": MeleeBrute(
                aggressiveness: 1.0, aggro_radius: 220.0, attack_range: 36.0, chase_speed: 110.0,
            ),
            "patrol_peaceful": Patrol(
                spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
                aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
            ),
        },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_puppy_slug": (
                display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Crawler, composition: None,
                default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
            ),
            "npc_brute": (
                display_name: "Brute", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "melee_brute_striker", default_action_set: "peaceful", tags: [],
            ),
            "npc_patroller": (
                display_name: "Patroller", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "patrol_peaceful", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(parse_catalog(CATALOG))
    }

    fn resolve(
        cid: &str,
        authored: Option<&str>,
        ctx: BrainBuildContext,
    ) -> Result<(BrainBinding, Brain), BrainBuildError> {
        resolve_initial_brain(&catalog(), cid, authored, &ctx)
    }

    /// #1 — a puppy slug with no override receives its `wanderer_puppy_slug`
    /// default, and the binding records that default on the Default selection.
    #[test]
    fn character_default_resolves_the_catalog_default_brain() {
        let (binding, brain) = resolve("npc_puppy_slug", None, BrainBuildContext::at(0.0)).unwrap();
        assert_eq!(brain.label(), "wanderer");
        assert_eq!(
            binding.default_preset,
            AutonomousDefault::Preset(BrainPresetId::new("wanderer_puppy_slug"))
        );
        assert_eq!(binding.source, AutonomousSource::CatalogDefault);
        assert_eq!(
            binding.active_preset(),
            Some(&BrainPresetId::new("wanderer_puppy_slug"))
        );
    }

    /// #2 — a puppy slug with a `stand_still` override receives a StandStill
    /// brain, and the binding records the override (default preset preserved).
    #[test]
    fn stand_still_override_wins_over_the_wander_default() {
        let (binding, brain) = resolve(
            "npc_puppy_slug",
            Some("stand_still"),
            BrainBuildContext::at(0.0),
        )
        .unwrap();
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert_eq!(
            binding.default_preset,
            AutonomousDefault::Preset(BrainPresetId::new("wanderer_puppy_slug"))
        );
        assert_eq!(
            binding.source,
            AutonomousSource::CatalogPreset(BrainPresetId::new("stand_still"))
        );
    }

    /// #6 — a hostile catalog default is USED when no override is authored (the
    /// removed is_hostile gate no longer peaceful-izes a placed hostile default).
    #[test]
    fn hostile_default_is_used_without_an_override() {
        let (_, brain) = resolve("npc_brute", None, BrainBuildContext::at(0.0)).unwrap();
        assert_eq!(brain.label(), "melee_brute");
        assert!(
            brain.is_hostile(),
            "the hostile default drives a hostile brain"
        );
    }

    /// #7 — a hostile character with a StandStill override stays stationary.
    #[test]
    fn hostile_character_with_stand_still_override_is_stationary() {
        let (_, brain) =
            resolve("npc_brute", Some("stand_still"), BrainBuildContext::at(0.0)).unwrap();
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert!(!brain.is_hostile());
    }

    /// #8 — `patrol_radius > 0` does NOT independently select a patrol brain: a
    /// wanderer default with a placement patrol_radius stays a wanderer.
    #[test]
    fn patrol_radius_does_not_select_a_patrol_brain() {
        let (_, brain) = resolve(
            "npc_puppy_slug",
            None,
            BrainBuildContext::from_placement(0.0, 96.0),
        )
        .unwrap();
        assert_eq!(
            brain.label(),
            "wanderer",
            "a non-patrol default ignores placement patrol_radius; it must not become Patrol"
        );
    }

    /// #9 — a SELECTED patrol preset consumes the placement's patrol radius (and
    /// centers its lane on the spawn anchor); with no placement override it keeps
    /// the preset's authored radius.
    #[test]
    fn selected_patrol_preset_consumes_placement_radius() {
        // Placement radius override wins.
        let (_, brain) = resolve(
            "npc_patroller",
            None,
            BrainBuildContext::from_placement(100.0, 200.0),
        )
        .unwrap();
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.lane.radius_px, 200.0, "placement radius feeds the lane");
                assert_eq!(cfg.lane.center_x, 100.0, "lane centers on spawn_world_x");
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
        // No placement override -> the preset's authored radius (64).
        let (_, brain) = resolve("npc_patroller", None, BrainBuildContext::at(100.0)).unwrap();
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(
                    cfg.lane.radius_px, 64.0,
                    "keeps the preset radius with no override"
                );
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    /// #10 — an unknown non-empty preset id fails resolution (no silent fallback),
    /// and an unknown character id fails too.
    #[test]
    fn unknown_preset_and_character_fail_resolution() {
        let err = resolve(
            "npc_puppy_slug",
            Some("no_such_preset"),
            BrainBuildContext::at(0.0),
        )
        .unwrap_err();
        assert!(
            matches!(err, BrainBuildError::UnknownPreset { source: PresetSource::Override, .. }),
            "an unknown override preset is an Override-sourced error, not a silent fallback: {err:?}"
        );

        let err = resolve("no_such_character", None, BrainBuildContext::at(0.0)).unwrap_err();
        assert!(matches!(err, BrainBuildError::UnknownCharacter(_)));
    }

    /// Empty / whitespace `brain_override` means the character default.
    #[test]
    fn empty_override_means_character_default() {
        for authored in [Some(""), Some("   "), None] {
            let (binding, _) =
                resolve("npc_puppy_slug", authored, BrainBuildContext::at(0.0)).unwrap();
            assert_eq!(
                binding.source,
                AutonomousSource::CatalogDefault,
                "{authored:?}"
            );
        }
    }

    /// A provoked source has no active catalog preset — reconciliation reads
    /// this to rebuild the provoked mode rather than the catalog default.
    #[test]
    fn provoked_source_has_no_active_preset() {
        let mut binding = BrainBinding::new(
            BrainPresetId::new("wanderer_puppy_slug"),
            AutonomousSource::CatalogDefault,
        );
        binding.provoke();
        assert!(binding.is_provoked());
        assert_eq!(binding.active_preset(), None);
        //  the poison: a body provoked into its OWN authored policy is a
        // different source, and `is_provoked` must not answer for it — the two
        // rebuild differently, which is the whole reason there are two.
        binding.source = AutonomousSource::ProvokedProfile {
            profile: ambition_entity_catalog::BrainProfileId::new("cellular_duelist"),
        };
        assert!(!binding.is_provoked());
        assert_eq!(binding.active_preset(), None);
    }

    /// AuthoredBrainContext captures the placement home and reproduces a build
    /// context — the same lane the spawn used, so a later rebuild recenters on the
    /// authored anchor, not the actor's current pose.
    #[test]
    fn authored_context_reproduces_the_spawn_lane() {
        let authored = AuthoredBrainContext::from_placement(100.0, 200.0);
        let ctx = authored.build_context();
        assert_eq!(ctx.spawn_world_x, 100.0);
        assert_eq!(ctx.patrol_radius, Some(200.0));
        // A non-positive radius is "unset".
        assert_eq!(
            AuthoredBrainContext::from_placement(100.0, 0.0).patrol_radius,
            None
        );
    }
    // They pinned a real ruling — *"a character definition may state its normal
    // autonomous behaviour, and it outranks the catalog row"* — through the
    // `definition_default` parameter, a `BrainPresetRef` a definition could
    // author. That parameter is deleted: no character in the repo ever
    // authored one, and its absence is what made the resolver manufacture an
    // empty preset id and crash two shipped rooms.
    //
    //  the RULING survives; only its vocabulary changed. A character states its
    // policy as a `BrainProfile` now, and the same three claims — the character
    // outranks the row, an authored placement override outranks both, and a
    // silent character leaves the row standing — are asserted in
    // `features::npcs`, which is where the profile can be lowered against a body.
    // Re-asserting them here would need a body this crate does not have.
}
