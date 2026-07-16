//! Explicit NPC brain authority.
//!
//! Replaces the old implicit heuristic (`npc_brain_from_catalog`): an NPC's
//! brain is determined ONLY by its explicit initial [`InitialBrainSelection`]
//! or the character catalog default — never by inspecting the resulting
//! [`Brain`]. There is no "basic brain" classification, no `is_hostile` gate,
//! no `patrol_radius == 0` sentinel, and no peaceful-specialized bypass.
//!
//! The pieces:
//! - [`BrainPresetId`] — a key into the catalog `brain_presets` map.
//! - [`InitialBrainSelection`] — what a *placement* authors: the character
//!   default, or an explicit preset override.
//! - [`BrainBinding`] — the runtime component that records the character's
//!   default preset plus whether the actor is currently on that default or an
//!   override. This is the authoritative snapshot state for "which brain is
//!   selected"; the live [`Brain`] is rebuilt from it deterministically.
//! - [`BrainBuildContext`] — per-spawn parameters a *selected* preset consumes
//!   (patrol lane center/radius/path). These parameterize a chosen preset; they
//!   never choose it.
//! - [`resolve_initial_brain`] — override → character default → clear error.
//!
//! Once an actor carries a [`BrainBinding`], runtime gameplay switches its brain
//! through the authoritative command path (`ambition_actors`'s `BrainCommand`),
//! which rebuilds the [`Brain`] from a preset via
//! [`CharacterCatalog::build_brain_from_preset`] — the same seam this module
//! uses at spawn, so a preset resolves identically at spawn and at a later
//! runtime switch.

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

/// What brain a placed NPC receives at spawn.
///
/// The serialized authoring form is an optional `brain_override` string field on
/// the LDtk `NpcSpawn` entity: absent or empty means [`CharacterDefault`], a
/// non-empty preset name means [`Preset`]. A runtime `Brain` is never serialized;
/// only the stable preset id is authored, resolved when the actor is built.
///
/// [`CharacterDefault`]: InitialBrainSelection::CharacterDefault
/// [`Preset`]: InitialBrainSelection::Preset
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InitialBrainSelection {
    /// Use the character's catalog `default_brain`.
    #[default]
    CharacterDefault,
    /// Use an explicit preset instead of the catalog default.
    Preset(BrainPresetId),
}

impl InitialBrainSelection {
    /// Interpret an authored `brain_override` field. Absent, empty, or
    /// whitespace-only means [`CharacterDefault`](Self::CharacterDefault).
    pub fn from_authored(field: Option<&str>) -> Self {
        match field.map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => Self::Preset(BrainPresetId::new(name)),
            None => Self::CharacterDefault,
        }
    }

    /// The override preset id, if this selection is an explicit override.
    pub fn preset_id(&self) -> Option<&BrainPresetId> {
        match self {
            Self::Preset(id) => Some(id),
            Self::CharacterDefault => None,
        }
    }
}

/// Whether an actor is currently using its character-default brain or an
/// explicit override preset. The mutable half of a [`BrainBinding`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainSelection {
    /// Using the character's catalog default preset.
    Default,
    /// Using an explicit override preset (authored, or applied at runtime).
    Override(BrainPresetId),
}

/// Runtime record of an NPC's brain choice: its character-default preset and
/// whether it is currently on that default or an override.
///
/// This is the authoritative simulation state for "which brain is selected". The
/// live [`Brain`] component is the instantiated, mutable state machine; on
/// snapshot restore the `Brain` is rebuilt from this binding so the two always
/// agree, and runtime switches (`BrainCommand`) mutate this binding + rebuild the
/// `Brain` together.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BrainBinding {
    /// The character's catalog `default_brain`, captured at spawn. Restoring the
    /// default rebuilds a fresh brain from THIS preset.
    pub default_preset: BrainPresetId,
    /// Whether the actor is on its default or an override right now.
    pub selection: BrainSelection,
}

impl BrainBinding {
    pub fn new(default_preset: BrainPresetId, selection: BrainSelection) -> Self {
        Self {
            default_preset,
            selection,
        }
    }

    /// The preset id in effect right now: the override if one is selected, else
    /// the character default.
    pub fn active_preset(&self) -> &BrainPresetId {
        match &self.selection {
            BrainSelection::Override(id) => id,
            BrainSelection::Default => &self.default_preset,
        }
    }

    /// True iff an override preset is currently selected.
    pub fn is_override(&self) -> bool {
        matches!(self.selection, BrainSelection::Override(_))
    }

    /// Switch to an override preset. (The caller rebuilds the live `Brain`.)
    pub fn use_preset(&mut self, preset: BrainPresetId) {
        self.selection = BrainSelection::Override(preset);
    }

    /// Return to the character default. (The caller rebuilds the live `Brain`.)
    pub fn restore_default(&mut self) {
        self.selection = BrainSelection::Default;
    }
}

/// Per-spawn parameters a *selected* brain preset consumes when it is
/// instantiated.
///
/// These parameterize a preset that was already chosen; they never SELECT the
/// preset. A [`Patrol`](crate::brain::state_machine::StateMachineCfg::Patrol)
/// preset consumes `spawn_world_x` for its lane center and, when the placement
/// authored one, `patrol_radius` as a lane-radius override. Every non-patrol
/// preset ignores the patrol fields.
///
/// [`Patrol`]: crate::brain::state_machine::StateMachineCfg::Patrol
#[derive(Clone, Debug, PartialEq)]
pub struct BrainBuildContext {
    /// The actor's world-space spawn X — the patrol lane center anchor.
    pub spawn_world_x: f32,
    /// Placement lane-radius override for a selected patrol preset. `None` (or a
    /// non-positive value) keeps the preset's authored radius.
    pub patrol_radius: Option<f32>,
    /// Placement patrol path id, threaded to a selected patrol preset that
    /// supports one. (The current lane-based patrol preset ignores it.)
    pub patrol_path_id: Option<String>,
}

impl BrainBuildContext {
    /// A context that only anchors the patrol lane center (no placement patrol
    /// override) — the shape a runtime rebuild uses.
    pub fn at(spawn_world_x: f32) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: None,
            patrol_path_id: None,
        }
    }

    /// A context carrying a placement's authored patrol tuning. A non-positive
    /// `patrol_radius` is treated as "unset" (keep the preset's radius).
    pub fn from_placement(
        spawn_world_x: f32,
        patrol_radius: f32,
        patrol_path_id: Option<String>,
    ) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: (patrol_radius > 0.0).then_some(patrol_radius),
            patrol_path_id,
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
/// falls back silently — the spawn site fails loud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainBuildError {
    /// `character_id` is not in the catalog.
    UnknownCharacter(String),
    /// The selected preset name is not in `brain_presets`.
    UnknownPreset {
        character_id: String,
        preset: String,
        source: PresetSource,
    },
}

impl std::fmt::Display for BrainBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCharacter(id) => {
                write!(f, "unknown character_id `{id}` (not in the character catalog)")
            }
            Self::UnknownPreset {
                character_id,
                preset,
                source,
            } => write!(
                f,
                "character `{character_id}`: {source} names unknown brain preset `{preset}` (not in brain_presets)"
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
pub fn qualify_preset_like(reference: &str, local: &str) -> String {
    if local.contains("::") {
        return local.to_string();
    }
    match reference.rsplit_once("::") {
        Some((provider, _)) => format!("{provider}::{local}"),
        None => local.to_string(),
    }
}

/// Resolve the initial brain for a placed NPC.
///
/// Precedence: explicit override preset → character catalog default → clear
/// error. Returns both the runtime [`BrainBinding`] (for snapshot + later
/// runtime switching) and a freshly instantiated [`Brain`]. This function NEVER
/// inspects the resulting brain to decide anything — the selection is authored,
/// the default is catalog data.
pub fn resolve_initial_brain(
    catalog: &CharacterCatalog,
    character_id: &str,
    selection: &InitialBrainSelection,
    ctx: &BrainBuildContext,
) -> Result<(BrainBinding, Brain), BrainBuildError> {
    let entry = catalog
        .get(character_id)
        .ok_or_else(|| BrainBuildError::UnknownCharacter(character_id.to_string()))?;
    let default_preset = BrainPresetId::new(entry.default_brain.clone());

    let binding_selection = match selection {
        InitialBrainSelection::CharacterDefault => BrainSelection::Default,
        // Authoring names a RAW local preset; qualify it into the character's
        // namespace so it matches the assembled catalog's `provider::name` keys.
        // The binding stores the QUALIFIED name so the runtime switch path and
        // snapshot reconcile resolve it identically without re-qualifying.
        InitialBrainSelection::Preset(id) => BrainSelection::Override(BrainPresetId::new(
            qualify_preset_like(entry.default_brain.as_str(), id.as_str()),
        )),
    };
    let binding = BrainBinding::new(default_preset, binding_selection);

    let active = binding.active_preset();
    let brain = catalog
        .build_brain_from_preset(active.as_str(), ctx)
        .ok_or_else(|| BrainBuildError::UnknownPreset {
            character_id: character_id.to_string(),
            preset: active.0.clone(),
            source: if binding.is_override() {
                PresetSource::Override
            } else {
                PresetSource::CharacterDefault
            },
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
        selection: InitialBrainSelection,
        ctx: BrainBuildContext,
    ) -> Result<(BrainBinding, Brain), BrainBuildError> {
        resolve_initial_brain(&catalog(), cid, &selection, &ctx)
    }

    /// #1 — a puppy slug with no override receives its `wanderer_puppy_slug`
    /// default, and the binding records that default on the Default selection.
    #[test]
    fn character_default_resolves_the_catalog_default_brain() {
        let (binding, brain) = resolve(
            "npc_puppy_slug",
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::at(0.0),
        )
        .unwrap();
        assert_eq!(brain.label(), "wanderer");
        assert_eq!(
            binding.default_preset,
            BrainPresetId::new("wanderer_puppy_slug")
        );
        assert_eq!(binding.selection, BrainSelection::Default);
    }

    /// #2 — a puppy slug with a `stand_still` override receives a StandStill
    /// brain, and the binding records the override (default preset preserved).
    #[test]
    fn stand_still_override_wins_over_the_wander_default() {
        let (binding, brain) = resolve(
            "npc_puppy_slug",
            InitialBrainSelection::Preset(BrainPresetId::new("stand_still")),
            BrainBuildContext::at(0.0),
        )
        .unwrap();
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert_eq!(
            binding.default_preset,
            BrainPresetId::new("wanderer_puppy_slug")
        );
        assert_eq!(
            binding.selection,
            BrainSelection::Override(BrainPresetId::new("stand_still"))
        );
    }

    /// #6 — a hostile catalog default is USED when no override is authored (the
    /// removed is_hostile gate no longer peaceful-izes a placed hostile default).
    #[test]
    fn hostile_default_is_used_without_an_override() {
        let (_, brain) = resolve(
            "npc_brute",
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::at(0.0),
        )
        .unwrap();
        assert_eq!(brain.label(), "melee_brute");
        assert!(
            brain.is_hostile(),
            "the hostile default drives a hostile brain"
        );
    }

    /// #7 — a hostile character with a StandStill override stays stationary.
    #[test]
    fn hostile_character_with_stand_still_override_is_stationary() {
        let (_, brain) = resolve(
            "npc_brute",
            InitialBrainSelection::Preset(BrainPresetId::new("stand_still")),
            BrainBuildContext::at(0.0),
        )
        .unwrap();
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
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::from_placement(0.0, 96.0, None),
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
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::from_placement(100.0, 200.0, None),
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
        let (_, brain) = resolve(
            "npc_patroller",
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::at(100.0),
        )
        .unwrap();
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
            InitialBrainSelection::Preset(BrainPresetId::new("no_such_preset")),
            BrainBuildContext::at(0.0),
        )
        .unwrap_err();
        assert!(
            matches!(err, BrainBuildError::UnknownPreset { source: PresetSource::Override, .. }),
            "an unknown override preset is an Override-sourced error, not a silent fallback: {err:?}"
        );

        let err = resolve(
            "no_such_character",
            InitialBrainSelection::CharacterDefault,
            BrainBuildContext::at(0.0),
        )
        .unwrap_err();
        assert!(matches!(err, BrainBuildError::UnknownCharacter(_)));
    }

    /// The authored-field interpretation: absent / empty / whitespace means
    /// CharacterDefault; a non-empty value is a preset override.
    #[test]
    fn from_authored_maps_empty_to_default() {
        assert_eq!(
            InitialBrainSelection::from_authored(None),
            InitialBrainSelection::CharacterDefault
        );
        assert_eq!(
            InitialBrainSelection::from_authored(Some("")),
            InitialBrainSelection::CharacterDefault
        );
        assert_eq!(
            InitialBrainSelection::from_authored(Some("   ")),
            InitialBrainSelection::CharacterDefault
        );
        assert_eq!(
            InitialBrainSelection::from_authored(Some("stand_still")),
            InitialBrainSelection::Preset(BrainPresetId::new("stand_still"))
        );
    }
}
