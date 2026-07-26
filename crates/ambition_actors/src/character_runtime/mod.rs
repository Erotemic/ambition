//! **The engine owns turning a declared character into loaded art.**
//!
//! ## Why this module exists
//!
//! It did not, and that was one bug reported three times. The step that decodes
//! a declared character's sheet lived in `ambition_app`'s room-transition asset
//! code — an APPLICATION crate. So:
//!
//! * `ambition_demo_mary_o_app` never ran it, and Mary-O rendered as a coloured
//!   rectangle in her own standalone game while rendering correctly in the
//!   multi-game host;
//! * `ambition_demo_sanic_app` hand-rolled a duplicate to work around it;
//! * only the PRIMARY PLAYER's worn sheet was covered, so any second worn body —
//!   exactly what a versus mode is made of — fell through.
//!
//! Nothing failed when two applications composed the engine differently, which is
//! what let the same defect ship three times. The fix is not "remember to add the
//! system": it is that no application can add it, because the engine always does.
//!
//! ## The shape
//!
//! Applications **declare** characters and **submit demand**. They never decode.
//!
//! ```text
//! room staging      ─┐
//! match roster      ─┼─→ CharacterLoadDemand ─→ [engine materializer] ─→ CharacterLoadStates
//! direct startup    ─┤                                                    Ready | Failed(named)
//! a worn identity   ─┘
//! ```
//!
//! Demand is a projection, not a rich object: several semantically different
//! sources (a room plan, a match roster, a startup spec, a body putting on a new
//! identity) share exactly one thing — a set of character tokens that now need
//! art. Transformations, summons, assists, alternate forms, and post-reveal
//! bosses all arrive through the same door.

pub mod audit;
pub mod definition;
pub mod hurtbox;
pub mod presentation;
pub mod staging;

pub use audit::{
    CharacterCapabilityGap, audit_character_capabilities, character_reveal_ready,
    unsettled_staged_characters,
};
pub use definition::{
    BodySource, CharacterBindings, CharacterDefinition, CharacterDefinitionAppExt,
    CharacterRegistrationError, Lineage, PreparedCharacter, PreparedCharacterDefinition,
    PreparedCharacterRegistry, Vitals, prepare_character,
};
pub use hurtbox::{
    AuthoredHurtboxes, BodyPoseClock, HurtboxSelection, POSE_AIRBORNE, POSE_HITSTUN, POSE_IDLE,
    ResolvedHurtboxes, resolve_hurtboxes,
};
pub use presentation::{
    authorize_staged_character_presentation_sources, provider_of_character,
};
pub use staging::{
    ControllerBinding, DirectStartupSpec, MatchParticipant, MatchParticipantRoster,
    NormalizedEffort, RoomStagingPlan, StagesCharacters,
};

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer_primitives::schedule::SimScheduleExt;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_sprite_sheet::character::{CharacterSheetState, CharacterSpriteAssets};

use crate::assets::sandbox_assets::SandboxAssetCatalog;

/// Character tokens a session has staged and therefore needs art for.
///
/// A token is a catalog id or an authored display name — whatever content wrote.
/// Requests accumulate until the materializer drains them, so a submitter never
/// has to know whether the decode already happened.
#[derive(Resource, Default, Debug, Clone)]
pub struct CharacterLoadDemand {
    pending: BTreeSet<String>,
}

impl CharacterLoadDemand {
    /// Ask for one character's art. Idempotent, and cheap enough to call every
    /// time a body's identity changes.
    pub fn request(&mut self, token: impl Into<String>) {
        let token = token.into();
        if !token.trim().is_empty() {
            self.pending.insert(token);
        }
    }

    /// Ask for many.
    pub fn request_all<I, S>(&mut self, tokens: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for token in tokens {
            self.request(token);
        }
    }

    pub fn pending(&self) -> impl Iterator<Item = &str> {
        self.pending.iter().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Take the outstanding demand. Deterministic order: `BTreeSet`, so two peers
    /// decode the same ids in the same sequence.
    fn take(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.pending)
    }
}

/// Why a demanded character has no art.
///
/// These are separate variants because they need different responses, and
/// collapsing them into one `None` is what made a typo look like a slow decode
/// for as long as this code has existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterLoadFailure {
    /// No loaded content declares this token under any key. A misspelling, or a
    /// character from a provider that is not part of this composition. Waiting
    /// will never fix it.
    UnknownCharacter,
    /// Content declares it and named a sheet, but the asset catalog gated the
    /// load or the decode produced nothing. The character draws the marked
    /// placeholder; this is legitimate for an art-free or reduced-asset build.
    NoSheetResolved,
    /// This composition has NO asset pipeline at all — a headless simulation, an
    /// RL rollout, an art-free shell. Legitimate, and a different fact from "the
    /// sheet did not resolve": nothing was ever going to decode.
    ///
    /// It is a TERMINAL state rather than leaving the demand pending, because
    /// §4.9's invariant forbids silence, and a headless run whose reveal barrier
    /// waits forever on art that cannot exist is exactly that silence with extra
    /// steps.
    NoAssetPipeline,
}

impl CharacterLoadFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownCharacter => "no loaded content declares this character",
            Self::NoSheetResolved => {
                "declared, but no sheet resolved under the active asset profile"
            }
            Self::NoAssetPipeline => "this composition has no asset pipeline (headless / art-free)",
        }
    }
}

/// Terminal state of one demanded character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterLoadOutcome {
    Ready,
    Failed(CharacterLoadFailure),
}

/// Every demanded character's outcome. §4.9's readiness invariant reads this:
/// every staged character must reach `Ready` or a NAMED terminal `Failed` before
/// the reveal barrier opens — never "still unknown".
#[derive(Resource, Default, Debug, Clone)]
pub struct CharacterLoadStates {
    by_token: BTreeMap<String, CharacterLoadOutcome>,
}

impl CharacterLoadStates {
    pub fn outcome(&self, token: &str) -> Option<CharacterLoadOutcome> {
        self.by_token.get(token).copied()
    }

    pub fn is_ready(&self, token: &str) -> bool {
        self.outcome(token) == Some(CharacterLoadOutcome::Ready)
    }

    /// Every token that reached a named failure, with its reason.
    pub fn failures(&self) -> impl Iterator<Item = (&str, CharacterLoadFailure)> {
        self.by_token
            .iter()
            .filter_map(|(token, outcome)| match outcome {
                CharacterLoadOutcome::Failed(failure) => Some((token.as_str(), *failure)),
                CharacterLoadOutcome::Ready => None,
            })
    }

    /// Every token this session staged, in deterministic order.
    ///
    /// The cast, as far as the load ledger is concerned — including the ones that
    /// reached a named failure, because a character whose sheet did not resolve is
    /// still IN the fight and still needs its cues authorized.
    pub fn staged_characters(&self) -> impl Iterator<Item = &str> {
        self.by_token.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_token.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_token.is_empty()
    }

    fn record(&mut self, token: String, outcome: CharacterLoadOutcome) {
        self.by_token.insert(token, outcome);
    }
}

/// Proof that the engine materialization service is installed.
///
/// §4.9's backstop: an unusual composition that somehow reaches staging without
/// this resource is one whose characters would silently draw placeholders
/// forever, so the audit in [`super::character_runtime::audit`] names it instead.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CharacterMaterializationService;

/// Decode every outstanding demand, recording a terminal outcome for each.
///
/// The engine's ONE decode path. Deliberately a free function as well as a
/// system, because a host that builds an asset manifest needs the decode to have
/// happened *synchronously* before it enumerates handles for the reveal barrier —
/// but the system below means an application that never calls it still gets its
/// characters materialized.
#[allow(clippy::too_many_arguments)]
pub fn materialize_character_demand(
    demand: &mut CharacterLoadDemand,
    states: &mut CharacterLoadStates,
    sprites: &mut CharacterSpriteAssets,
    character_catalog: &CharacterCatalog,
    // The registered definitions. A character may be declared HERE and nowhere
    // else — that is the point of the single seam — so this is a real source of
    // sheets, not decoration.
    registry: &PreparedCharacterRegistry,
    asset_catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
) {
    for token in demand.take() {
        // Ask BEFORE decoding: an unknown token must be reported as unknown, not
        // as a decode that produced nothing. They are different bugs.
        if matches!(sprites.sheet_state(&token), CharacterSheetState::Unknown) {
            states.record(
                token,
                CharacterLoadOutcome::Failed(CharacterLoadFailure::UnknownCharacter),
            );
            continue;
        }
        let ready = crate::character_sprites::materialize_declared_character_sprite(
            sprites,
            character_catalog,
            asset_catalog,
            asset_server,
            layouts,
            quality,
            registered_sheet_target(registry, sprites, &token),
            &token,
        );
        let outcome = if ready {
            CharacterLoadOutcome::Ready
        } else {
            CharacterLoadOutcome::Failed(CharacterLoadFailure::NoSheetResolved)
        };
        states.record(token, outcome);
    }
}

/// The sheet target the registered definition names for this token, if any.
///
/// The token may be a catalog id or a display name, so resolve it the same way
/// the sheet table does before asking the registry.
fn registered_sheet_target<'a>(
    registry: &'a PreparedCharacterRegistry,
    sprites: &CharacterSpriteAssets,
    token: &str,
) -> Option<&'a str> {
    let id = match sprites.sheet_state(token) {
        CharacterSheetState::Declared { character_id } => character_id,
        _ => token,
    };
    registry.get(id).and_then(|p| p.sheet.as_deref())
}

/// **Declare every registered character into the sprite read model.**
///
/// The sheet table used to be populated exclusively from `CharacterCatalog`, so a
/// character registered only through `register_character` was `Unknown` to the art
/// pipeline — the load state reported "no loaded content declares this character"
/// about a character a provider had just declared. The prepared registry is a
/// source of declarations, and this is where it becomes one.
///
/// Idempotent, and cheap: declaring does NOT decode. It only teaches the table
/// that the id exists and which display name aliases it, which is what turns
/// `Unknown` (a typo — waiting will not help) into `Declared` (a decode that has
/// not happened yet). Those two answers demand different responses, which is why
/// §7.1 separated them in the first place.
pub fn declare_registered_characters(
    registry: Option<Res<PreparedCharacterRegistry>>,
    // The sheet table lives INSIDE `GameAssets`, not as a standalone resource.
    assets: Option<ResMut<ambition_sprite_sheet::game_assets::GameAssets>>,
) {
    let (Some(registry), Some(mut assets)) = (registry, assets) else {
        // No registered characters, or no sprite table at all (an art-free
        // composition). The demand path reports `NoAssetPipeline` for the latter,
        // which is a named terminal state, not silence.
        return;
    };
    if !registry.is_changed() {
        return;
    }
    let sprites = &mut assets.characters;
    for id in registry.ids() {
        if matches!(sprites.sheet_state(id), CharacterSheetState::Unknown) {
            let display_name = registry
                .get(id)
                .map(|p| p.display_name.as_str())
                .unwrap_or(id);
            sprites.declare(id, display_name);
        }
    }
}

/// A body that put on an identity needs that identity's art.
///
/// Every worn body, not just the primary player. The host's version of this
/// watched `With<PrimaryPlayer>` only, which is correct for exactly one game mode
/// and wrong for the one this plan exists to reach: in a versus match every
/// fighter is a worn body, and only player one would have had a sheet.
///
/// `Changed` covers first appearance AND every later swap, so a runtime form
/// change into another declared sheet (Mary-O growing into `mary_o_tall`) demands
/// its art the tick the identity changes.
pub fn demand_worn_character_sheets(
    worn: Query<
        &ambition_characters::actor::WornCharacter,
        Changed<ambition_characters::actor::WornCharacter>,
    >,
    demand: Option<ResMut<CharacterLoadDemand>>,
) {
    let Some(mut demand) = demand else {
        return;
    };
    for identity in &worn {
        demand.request(identity.id());
    }
}

/// The standing materializer. No-ops without an asset pipeline (headless, art-free
/// shells) exactly like the room barrier does, so a sim-only build pays nothing.
#[allow(clippy::too_many_arguments)]
pub fn materialize_demanded_character_sheets(
    demand: Option<ResMut<CharacterLoadDemand>>,
    states: Option<ResMut<CharacterLoadStates>>,
    assets: Option<ResMut<ambition_sprite_sheet::game_assets::GameAssets>>,
    // NOT `Option<Res<..>>`. The character catalog is REQUIRED authority
    // (`engine.character-authority-is-app-local`): making it optional is how a
    // missing catalog silently becomes an empty one, and then every character
    // "has no sheet" for a reason nobody can see. The system is gated on the
    // resource existing instead, and a composition that reaches staging without
    // one is NAMED by the capability audit rather than quietly doing nothing.
    character_catalog: Res<CharacterCatalog>,
    // Registered definitions are a real source of sheets (see
    // `sheet_for_declared_character`). `Option` because a composition may have no
    // registered characters at all, which is not an error.
    registry: Option<Res<PreparedCharacterRegistry>>,
    asset_catalog: Option<Res<SandboxAssetCatalog>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
) {
    // The ledger and the demand come first and separately: the no-pipeline path
    // still has to SETTLE what was staged, so it cannot be inside a destructuring
    // that also consumes them.
    let (Some(mut demand), Some(mut states)) = (demand, states) else {
        return;
    };
    if demand.is_empty() {
        return;
    }

    let (Some(mut assets), Some(asset_catalog), Some(asset_server), Some(mut layouts)) =
        (assets, asset_catalog, asset_server, layouts)
    else {
        // No asset pipeline in this composition (headless sim, RL rollout, art-free
        // shell). SETTLE the demand with a NAMED terminal state rather than leaving
        // it pending: §4.9 forbids silence, and a reveal barrier waiting forever on
        // art that was never going to exist is that silence with extra steps.
        //
        // This is a legitimate outcome, not a defect — which is exactly why it needs
        // its own variant instead of borrowing `NoSheetResolved`. "Nothing was ever
        // going to decode here" and "the decode produced nothing" call for different
        // responses from whoever reads the ledger.
        for token in demand.take() {
            states.record(
                token,
                CharacterLoadOutcome::Failed(CharacterLoadFailure::NoAssetPipeline),
            );
        }
        return;
    };
    // Same source `ResolvedVisualQuality` mirrors, read directly so the engine
    // does not depend on the render crate to know its own texture budget.
    let budget = settings.map(|settings| settings.video.quality.resolved_budget());
    let fallback_registry = PreparedCharacterRegistry::default();
    materialize_character_demand(
        &mut demand,
        &mut states,
        &mut assets.characters,
        &character_catalog,
        registry.as_deref().unwrap_or(&fallback_registry),
        &asset_catalog,
        &asset_server,
        &mut layouts,
        budget.as_ref(),
    );
}

/// Installs the engine's character load pipeline.
///
/// Added unconditionally by the host simulation plugin so **no application can
/// compose the engine without it**. That is the whole point: this file exists
/// because "the app forgot the step" was a shippable state three times over.
pub struct CharacterRuntimePlugin;

impl Plugin for CharacterRuntimePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<CharacterLoadDemand>()
            .init_resource::<CharacterLoadStates>()
            .init_resource::<CharacterMaterializationService>()
            .add_systems(
                // **The SIM schedule, not `Update`.** (§4.11)
                //
                // These two read and write simulation state — a pose clock and the
                // volumes damage resolves against — so under rollback they must
                // recompute on every resimulated tick. In `Update` they ran once per
                // FRAME while the sim re-ran many times, which left
                // `ResolvedHurtboxes` stale for every rewound tick even though it is
                // declared rollback-DERIVED on the promise that the sim rebuilds it.
                // A frame-rate-dependent hurtbox is also just a bug: two peers at
                // different frame rates would disagree about what got hit.
                sim,
                (
                    // Gated, not `Option<Res<..>>`: a world with no clock has no
                    // pose elapsed to advance, and a system that quietly treats a
                    // missing clock as dt=0 would freeze every pose timeline
                    // without saying so.
                    hurtbox::advance_body_pose_clocks.run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<
                            ambition_time::WorldTime,
                        >,
                    ),
                    hurtbox::resolve_body_hurtboxes,
                )
                    .chain()
                    // Pinned to one exact window inside `Combat`: AFTER the move
                    // clock advances, BEFORE damage resolves.
                    //
                    // Both edges are load-bearing. A move override is selected by
                    // the move clock, so resolving before `advance_move_playback`
                    // would present the previous tick's silhouette on the first
                    // active frame — the frame that matters most. And every body's
                    // position is already post-movement here (`PlayerSimulation`
                    // and `WorldPrep` both precede `Combat`), so this is the one
                    // slot where clocks and positions are simultaneously current.
                    .in_set(crate::schedule::SandboxSet::Combat)
                    .after(crate::combat::moveset::advance_move_playback)
                    .before(crate::combat::hitbox::apply_hitbox_damage),
            )
            .add_systems(
                Update,
                (
                    // Declare before anything asks: a character registered only
                    // through `register_character` must not read as `Unknown`.
                    declare_registered_characters,
                    demand_worn_character_sheets,
                    // Before the drain: the audit reads OUTSTANDING demand, and the
                    // materializer empties it.
                    audit::report_character_capability_gaps,
                    materialize_demanded_character_sheets.run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<CharacterCatalog>,
                    ),
                    // AFTER the materializer has settled the demand: the staged
                    // cast is what authorizes presentation sources, and the
                    // ledger is where "staged" is written down.
                    presentation::authorize_staged_character_presentation_sources,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod definition_tests;
#[cfg(test)]
mod fight_tests;
#[cfg(test)]
mod tests;
