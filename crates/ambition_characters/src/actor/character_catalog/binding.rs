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
//!   preset plus the current [`BrainSelection`] (default / override / external).
//!   This is the authoritative snapshot state for "which autonomous brain is
//!   selected".
//! - [`AuthoredBrainContext`] — the per-spawn parameters a selected preset
//!   consumes (patrol lane anchor + radius), captured at spawn and retained so a
//!   runtime rebuild uses the actor's authored home, not wherever it wandered to.
//! - [`resolve_initial_brain`] — override → character default → clear error.
//!
//! Precedence and namespace resolution are deterministic (see
//! [`resolve_initial_brain`]). Once an actor carries a [`BrainBinding`], runtime
//! gameplay switches its brain through the authoritative command path
//! (`ambition_actors`'s `BrainCommand`), which rebuilds the [`Brain`] via
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

/// Which brain an actor is currently running, relative to its catalog default.
///
/// The mutable half of a [`BrainBinding`]. `External` means the live `Brain` was
/// installed by *another* authority (challenge/provocation, a mount transition,
/// player possession) and is NOT a catalog preset — snapshot reconciliation must
/// not try to rebuild it from `brain_presets` (that authority owns it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainSelection {
    /// Running the character's catalog default preset.
    Default,
    /// Running an explicit override preset (authored, or applied at runtime).
    Override(BrainPresetId),
    /// Running a brain installed by a non-catalog authority (provoke/mount/etc.).
    /// Not catalog-rebuildable; reconciliation leaves it to that authority.
    External,
}

/// Runtime record of an NPC's autonomous brain choice: its character-default
/// preset and which selection is currently live.
///
/// This is the authoritative simulation state for "which autonomous brain is
/// selected". The live [`Brain`] component is the instantiated, mutable state
/// machine; runtime switches (`BrainCommand`) mutate this binding + rebuild the
/// `Brain` together, and snapshot reconciliation uses this binding to keep the
/// two in agreement after a rewind that crossed a switch.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BrainBinding {
    /// The character's catalog `default_brain`, captured at spawn. Restoring the
    /// default rebuilds a fresh brain from THIS preset.
    pub default_preset: BrainPresetId,
    /// Which selection is live right now.
    pub selection: BrainSelection,
}

impl BrainBinding {
    pub fn new(default_preset: BrainPresetId, selection: BrainSelection) -> Self {
        Self {
            default_preset,
            selection,
        }
    }

    /// The catalog preset in effect right now: the override if one is selected,
    /// the character default when on the default, or `None` when the brain is
    /// externally owned (provoke/mount/possession — not a catalog preset).
    pub fn active_preset(&self) -> Option<&BrainPresetId> {
        match &self.selection {
            BrainSelection::Override(id) => Some(id),
            BrainSelection::Default => Some(&self.default_preset),
            BrainSelection::External => None,
        }
    }

    /// True iff an override preset is currently selected.
    pub fn is_override(&self) -> bool {
        matches!(self.selection, BrainSelection::Override(_))
    }

    /// True iff the live brain is owned by a non-catalog authority.
    pub fn is_external(&self) -> bool {
        matches!(self.selection, BrainSelection::External)
    }

    /// Switch to an override preset. (The caller rebuilds the live `Brain`.)
    pub fn use_preset(&mut self, preset: BrainPresetId) {
        self.selection = BrainSelection::Override(preset);
    }

    /// Return to the character default. (The caller rebuilds the live `Brain`.)
    pub fn restore_default(&mut self) {
        self.selection = BrainSelection::Default;
    }

    /// Record that another authority installed the live brain (provoke / mount /
    /// possession). Keeps the binding honest so reconciliation won't rebuild the
    /// catalog default over an externally-owned brain.
    pub fn mark_external(&mut self) {
        self.selection = BrainSelection::External;
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
    let default_preset = BrainPresetId::new(entry.default_brain.clone());

    // Interpret the authored field: empty/whitespace means "use the default".
    let override_name = authored_override.map(str::trim).filter(|s| !s.is_empty());

    let (selection, source, resolved_key) = match override_name {
        Some(name) => {
            // Authoring names a RAW local preset; qualify it into the character's
            // namespace so it matches the assembled catalog's `provider::name`
            // keys. The binding stores the QUALIFIED name so the runtime switch
            // path and snapshot reconcile resolve it identically.
            let key = qualify_preset_like(entry.default_brain.as_str(), name);
            (
                BrainSelection::Override(BrainPresetId::new(key.clone())),
                PresetSource::Override,
                key,
            )
        }
        None => (
            BrainSelection::Default,
            PresetSource::CharacterDefault,
            default_preset.0.clone(),
        ),
    };
    let binding = BrainBinding::new(default_preset, selection);

    let brain = catalog
        .build_brain_from_preset(&resolved_key, ctx)
        .ok_or_else(|| BrainBuildError::UnknownPreset {
            character_id: character_id.to_string(),
            preset: override_name.unwrap_or(&resolved_key).to_string(),
            resolved: resolved_key,
            source,
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
            BrainPresetId::new("wanderer_puppy_slug")
        );
        assert_eq!(binding.selection, BrainSelection::Default);
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
            assert_eq!(binding.selection, BrainSelection::Default, "{authored:?}");
        }
    }

    /// External selection has no active catalog preset — reconciliation reads
    /// this to leave an externally-owned (provoked/mounted) brain alone.
    #[test]
    fn external_selection_has_no_active_preset() {
        let mut binding = BrainBinding::new(
            BrainPresetId::new("wanderer_puppy_slug"),
            BrainSelection::Default,
        );
        binding.mark_external();
        assert!(binding.is_external());
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
}
