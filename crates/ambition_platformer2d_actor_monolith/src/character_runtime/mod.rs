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
pub mod physical_baseline;
pub mod presentation;
pub mod seating;
pub mod staging;

pub use audit::{
    audit_character_capabilities, character_reveal_ready, unsettled_staged_characters,
    CharacterCapabilityGap,
};
#[cfg(test)]
pub(crate) use definition::{prepare_and_finalize_against_for_test, prepare_and_finalize_for_test};
pub use definition::{
    BodySource, CharacterBindings, CharacterCatalogGeneration, CharacterDefinition,
    CharacterDefinitionAppExt, CharacterPreparationPlugin, CharacterRegistrationError, Lineage,
    PreparedCharacterDefinition, PreparedCharacterRegistry, PreparedKit, Vitals,
};
pub use hurtbox::{
    resolve_hurtboxes, AuthoredHurtboxes, BodyPoseClock, HurtboxSelection, ResolvedHurtboxes,
    POSE_AIRBORNE, POSE_HITSTUN, POSE_IDLE,
};
pub use physical_baseline::{
    BaselineBoundary, BodyGeometry, DisplacedPhysicals, PhysicalBaseline, PhysicalRetraction,
};
pub use presentation::{
    authorize_staged_character_presentation_sources, inherit_projectile_presentation_sources,
    project_prepared_character_definitions, provider_of_character,
    publish_body_presentation_sources, ProjectedCharacterKit,
};
pub use seating::{
    match_participants, seat_character, seat_match_participants, seat_placement, ActiveMatch,
    MatchSeat, MatchSeatingRefused,
};
pub use staging::{
    ControllerBinding, DirectStartupSpec, MatchParticipant, MatchParticipantRoster,
    NormalizedEffort, RoomStagingPlan, RosterProblem, RosterSeating, StagesCharacters,
};

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_sprite_sheet::character::{CharacterSheetState, CharacterSpriteAssets};

use crate::assets::platformer_assets::Platformer2dAssetCatalog;

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
    /// Content declares it, but no sheet DESCRIPTION resolved: neither a
    /// provider-authored sheet nor the engine's baked index knows the target its
    /// catalog row names. The character draws the marked placeholder.
    NoSheetResolved,
    /// The sheet resolved and the IMAGE did not — the asset catalog gated the
    /// load, or the path it produced reaches nothing.
    ///
    /// Split out from [`Self::NoSheetResolved`] on 2026-07-28 because they are
    /// different bugs with different fixes and one of them was wearing the
    /// other's name: the external fixture's character reported "no sheet
    /// resolved" while its sheet resolved perfectly and the desktop load gate
    /// was refusing a `game://` path it could not filesystem-check. Twenty
    /// minutes went into a metadata seam that was already correct.
    NoImageResolved,
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
                "declared, but no sheet DESCRIPTION resolved: neither an authored \
                 sheet nor the baked index knows the target its catalog row names"
            }
            Self::NoImageResolved => {
                "the sheet resolved and its IMAGE did not: the asset catalog \
                 gated the load, or its path reaches nothing"
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

/// **THIS session's cast, by canonical character id.**
///
/// Split out of the load ledger because the two answer different questions and
/// only one of them is a roster. [`CharacterLoadStates`] is append-only history
/// keyed by whatever TOKEN was demanded: it has to be, so a failure can name the
/// spelling that failed. Using it as the cast produced two bugs at once —
///
/// * after three rooms it holds every character the process ever loaded, so a
///   later session authorized the cues of characters who left the building; and
/// * a room that stages `"Mary-O"` (rooms legitimately author display names)
///   recorded `"Mary-O"`, which matches nothing in either provider map, so the
///   character loaded fine and her provider was never authorized.
///
/// This resource holds ids, resolved through the declaration authorities, and it
/// belongs to one [`SessionScopeId`].
///
/// ## What it is, exactly: a session CAPABILITY set, not a live roster
///
/// Within one session it only grows. Stage A and B, walk three rooms, stage C and
/// D, and all four are in it — it is not "the characters standing on the field right
/// now", and comments that read that way were imprecise (GPT 5.6, 2026-07-26).
///
/// That is the right shape for its one consumer, and deliberately so rather than by
/// omission. Its consumer is
/// [`authorize_staged_character_presentation_sources`](presentation::authorize_staged_character_presentation_sources),
/// and `ActiveAudioSelection` has no REVOKE: `authorize_sfx_source` adds a source
/// and only `select_gameplay` — a new session — clears the map. So a shrinking cast
/// could not un-authorize anybody, and modelling it as a live roster would produce a
/// resource whose contents implied a revocation the audio layer never performs. The
/// two facts are kept the same size on purpose.
///
/// The session boundary is where it resets, which is the boundary that matters:
/// authorizing a fifty-character roster after an evening of play is the bug this
/// closed, and a fifty-character roster is a fifty-character SESSION.
///
/// If a per-match live roster is ever wanted — a versus mode that reports who is on
/// stage, or an audio layer that gains revocation — it is a different resource with
/// a match generation, not a narrowing of this one. Adding a generation here without
/// giving `ActiveAudioSelection` a revoke would only make the authorization drift
/// out of sync with the thing that names it.
#[derive(Default, Debug, Clone)]
pub struct StagedCast {
    scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    ids: BTreeSet<String>,
}

impl StagedCast {
    /// Every character id THIS SESSION has staged, in deterministic order —
    /// including the ones whose art failed, because a fighter with no sheet is
    /// still in the fight and still needs its cues authorized.
    ///
    /// Accumulates for the session's lifetime; see the type docs for why that is the
    /// contract and not an oversight.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.ids.iter().map(String::as_str)
    }

    pub fn contains(&self, character_id: &str) -> bool {
        self.ids.contains(character_id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The session this cast belongs to, once one has claimed it.
    pub fn scope(&self) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
        self.scope
    }

    /// Bind the cast to the session that is now current, dropping the previous
    /// session's cast.
    ///
    /// Three cases, and the middle one is the load-bearing one:
    ///
    /// * `None` — no session owns anything yet, so there is nothing to reset
    ///   AGAINST. Leave the cast alone.
    /// * the cast has no scope — ADOPT this one and keep the ids. Startup stages
    ///   the player's characters before the first session scope is minted, and
    ///   treating that as a foreign cast would drop exactly the character the
    ///   player is about to control.
    /// * a different scope — this is a new session. The previous cast has left.
    pub fn enter_scope(
        &mut self,
        scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    ) {
        let Some(scope) = scope else {
            return;
        };
        match self.scope {
            None => self.scope = Some(scope),
            Some(current) if current == scope => {}
            Some(_) => {
                self.ids.clear();
                self.scope = Some(scope);
            }
        }
    }

    fn stage(&mut self, character_id: impl Into<String>) {
        self.ids.insert(character_id.into());
    }
}

/// Every demanded character's outcome. §4.9's readiness invariant reads this:
/// every staged character must reach `Ready` or a NAMED terminal `Failed` before
/// the reveal barrier opens — never "still unknown".
///
/// Append-only, and keyed by the demanded TOKEN rather than by character id, so a
/// misspelled or display-name demand is reported back in the spelling the caller
/// used. That makes it a diagnostic history and NOT a roster — see [`StagedCast`],
/// which it carries alongside.
#[derive(Resource, Default, Debug, Clone)]
pub struct CharacterLoadStates {
    by_token: BTreeMap<String, CharacterLoadOutcome>,
    cast: StagedCast,
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

    /// Every token ever demanded of this process, in deterministic order.
    ///
    /// Named for what it is. This is NOT the cast: it accumulates across rooms and
    /// across sessions and it is keyed by demand spelling. Ask [`Self::cast`] who
    /// is on stage.
    pub fn staged_tokens(&self) -> impl Iterator<Item = &str> {
        self.by_token.keys().map(String::as_str)
    }

    /// The current session's cast, by canonical character id.
    pub fn cast(&self) -> &StagedCast {
        &self.cast
    }

    pub fn cast_mut(&mut self) -> &mut StagedCast {
        &mut self.cast
    }

    pub fn len(&self) -> usize {
        self.by_token.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_token.is_empty()
    }

    /// Settle one demanded token: its outcome into the history, its canonical id
    /// into the cast.
    ///
    /// One method for both writes, because a token that reached a terminal state
    /// without joining the cast is a character who loaded and then made no sound.
    fn record(&mut self, token: String, character_id: &str, outcome: CharacterLoadOutcome) {
        self.cast.stage(character_id);
        self.by_token.insert(token, outcome);
    }
}

/// **The canonical character id a demand token names.**
///
/// Rooms, LDtk entities and roster entries all legitimately submit display names
/// (`"Mary-O"`), while every provider map — the prepared registry, the assembled
/// catalog's owners — is keyed by stable id (`"mary_o"`). Resolve through the two
/// declaration authorities, ids first, and hand back the token unchanged when
/// nothing claims it (an unknown token has no canonical form, and the load ledger
/// is where that gets reported).
///
/// Deliberately NOT resolved through the sprite table, which is the other place a
/// token → id alias exists: `CharacterSpriteAssets::publish` consumes its
/// declarations, so after the sheet decodes `sheet_state` answers `Ready` and the
/// id is no longer recoverable from it. An authority that stops answering once
/// the art arrives cannot be the one that names the cast.
pub fn canonical_character_id<'a>(
    registry: &'a PreparedCharacterRegistry,
    catalog: &'a CharacterCatalog,
    token: &'a str,
) -> &'a str {
    if registry.get(token).is_some() || catalog.get(token).is_some() {
        return token;
    }
    registry
        .id_for_display_name(token)
        .or_else(|| catalog.id_for_display_name(token))
        .unwrap_or(token)
}

/// Proof that the engine materialization service is installed.
///
/// §4.9's backstop: an unusual composition that somehow reaches staging without
/// this resource is one whose characters would silently draw placeholders
/// forever, so the audit in [`super::character_runtime::audit`] names it instead.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CharacterMaterializationService;

/// **A registered character declares itself, rather than hoping it was declared.**
///
/// [`declare_registered_characters`] teaches the sheet table about the registry once
/// per registry change — but it is an `Update` system, and the synchronous callers of
/// [`materialize_character_demand`] (direct startup, room transitions) can run in the
/// same frame BEFORE it. Nothing in the schedule expresses that order; they merely
/// touch the same resources, so Bevy serializes them in an unspecified sequence.
///
/// The consequence of losing that race is not a delay, it is a WRONG TERMINAL
/// VERDICT: `UnknownCharacter` means "no loaded content declares this, waiting will
/// never help", and it would have been reported about a character the caller had
/// just registered. The correctness of synchronous loading must not depend on which
/// system happened to run first, so the decode path establishes the declaration it
/// needs.
///
/// Declaring is not decoding — it teaches the table that the id exists and which
/// display name aliases it. Idempotent, and cheap.
/// `token` is the spelling that was demanded and `character_id` its canonical form
/// ([`canonical_character_id`]); the table is consulted for the TOKEN, because that
/// is the lookup whose `Unknown` answer would become the verdict.
pub fn declare_registered_character_into(
    sprites: &mut CharacterSpriteAssets,
    registry: &PreparedCharacterRegistry,
    token: &str,
    character_id: &str,
) {
    if !sprites.sheet_state(token).is_unknown() {
        return;
    }
    if let Some(prepared) = registry.get(character_id) {
        sprites.declare(&prepared.id, &prepared.display_name);
    }
}

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
    // Sheets this app's PROVIDERS authored (queue U1). A source of sheet
    // metadata that is not the engine's baked table, which is what lets a game
    // outside this workspace ship a character of its own.
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // The registered definitions. A character may be declared HERE and nowhere
    // else — that is the point of the single seam — so this is a real source of
    // sheets, not decoration.
    registry: &PreparedCharacterRegistry,
    asset_catalog: &Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
) {
    for token in demand.take() {
        // Whose cues this character will emit under, resolved BEFORE any decode:
        // the cast is a roster, not a report on the art, and it must be right for a
        // character whose sheet never resolves.
        let character_id = canonical_character_id(registry, character_catalog, &token).to_string();
        declare_registered_character_into(sprites, registry, &token, &character_id);
        // Ask BEFORE decoding: an unknown token must be reported as unknown, not
        // as a decode that produced nothing. They are different bugs.
        if matches!(sprites.sheet_state(&token), CharacterSheetState::Unknown) {
            states.record(
                token,
                &character_id,
                CharacterLoadOutcome::Failed(CharacterLoadFailure::UnknownCharacter),
            );
            continue;
        }
        let materialization = crate::character_sprites::materialize_declared_character_sprite(
            sprites,
            authored_sheets,
            character_catalog,
            asset_catalog,
            asset_server,
            layouts,
            quality,
            registered_sheet_target(registry, sprites, &token),
            &token,
        );
        let outcome = if materialization.is_ready() {
            CharacterLoadOutcome::Ready
        } else {
            CharacterLoadOutcome::Failed(match materialization {
                crate::character_sprites::SpriteMaterialization::NoSheet => {
                    CharacterLoadFailure::NoSheetResolved
                }
                crate::character_sprites::SpriteMaterialization::NoImage => {
                    CharacterLoadFailure::NoImageResolved
                }
                crate::character_sprites::SpriteMaterialization::Ready => {
                    unreachable!("the ready arm is handled above")
                }
            })
        };
        states.record(token, &character_id, outcome);
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

/// The portrait TARGET a registered definition named, if any.
///
/// The sibling of [`registered_sheet_target`], resolving the same token the same
/// way — a UI asking for a character's face and the materializer asking for its
/// body must agree about which character they are talking about.
pub fn registered_portrait_target<'a>(
    registry: &'a PreparedCharacterRegistry,
    sprites: &CharacterSpriteAssets,
    token: &str,
) -> Option<&'a str> {
    let id = match sprites.sheet_state(token) {
        CharacterSheetState::Declared { character_id } => character_id,
        _ => token,
    };
    registry.get(id).and_then(|p| p.portrait.as_deref())
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

/// **An ACTOR that resolved a character identity needs that art too.**
///
/// The exact sibling of [`demand_worn_character_sheets`], and it was missing.
/// That system watches `WornCharacter` — the identity a body PUTS ON. An
/// `EnemySpawn` wears nothing: it resolves its character through the display-name
/// join (`ActorClusterSeed` → `catalog.id_for_display_name`) and carries the
/// answer on `ActorConfig::sprite_character_id`. So a room full of authored
/// enemies declared their characters, resolved them correctly, and never asked
/// for the art.
///
/// The symptom is a marked placeholder rectangle, and the renderer says so
/// exactly: *"actor 'Puppy Slug' resolved no sprite and is drawing the
/// placeholder rectangle: declared as 'npc_puppy_slug' but not materialized —
/// nothing demanded it, so the engine never decoded its sheet"*. Four of them
/// stand in `intro_escape_shaft`, in the sequence a stranger plays first
/// (2026-07-29, found by photographing the room).
///
/// ⚠ **`Added` rather than `Changed`.** An actor's config is rebuilt every tick
/// as a read-model (`sync_actor_read_models` restores its reaction timers over a
/// fresh value), so `Changed` here would re-request the whole room's cast every
/// frame. The identity is decided at construction and does not drift, so asking
/// once when the component appears is both sufficient and the only affordable
/// option.
pub fn demand_actor_character_sheets(
    actors: Query<&crate::features::ActorConfig, Added<crate::features::ActorConfig>>,
    demand: Option<ResMut<CharacterLoadDemand>>,
) {
    let Some(mut demand) = demand else {
        return;
    };
    for config in &actors {
        if let Some(character_id) = config.sprite_character_id.as_deref() {
            demand.request(character_id);
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

/// **A new session gets a new cast.**
///
/// The load ledger accumulates forever by design, so without this the cast a
/// session authorizes cues for is every character the PROCESS ever loaded: quit to
/// the menu, start a different fight, and the previous fight's providers are
/// authorized alongside the current one. `ActiveAudioSelection` resets on session
/// select; the cast has to reset with it or the reset means nothing.
///
/// Runs before the materializer so a session that begins and stages in the same
/// frame keeps what it staged.
pub fn retire_previous_session_cast(
    scope: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    states: Option<ResMut<CharacterLoadStates>>,
) {
    let (Some(scope), Some(mut states)) = (scope, states) else {
        return;
    };
    let current = scope.current();
    if states.cast().scope() != current {
        states.cast_mut().enter_scope(current);
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
    // Sheets this app's providers authored. REQUIRED like the catalog and for
    // the same reason: it is authority, and `Option<Res<..>>` on authority is
    // how a missing registration turns into a silent placeholder instead of a
    // loud one. `SheetRegistryPlugin` initialises it, and the engine's own
    // characters resolve identically when it is empty.
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // Registered definitions are a real source of sheets (see
    // `sheet_for_declared_character`). `Option` because a composition may have no
    // registered characters at all, which is not an error.
    registry: Option<Res<PreparedCharacterRegistry>>,
    asset_catalog: Option<Res<Platformer2dAssetCatalog>>,
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
        let fallback_registry = PreparedCharacterRegistry::default();
        let registry = registry.as_deref().unwrap_or(&fallback_registry);
        for token in demand.take() {
            // Canonicalized here too. An art-free composition still emits cues, and
            // authorization is deliberately not gated on the asset pipeline — so a
            // headless session that staged `"Mary-O"` must still authorize
            // `mary_o_demo`, or the one build where nothing is visible is also the
            // one where nothing is audible.
            let character_id =
                canonical_character_id(registry, &character_catalog, &token).to_string();
            states.record(
                token,
                &character_id,
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
        &authored_sheets,
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
            // The provider-authored sheet registry (U1). Initialised HERE, by
            // the plugin that owns the decode, for the same reason this plugin
            // is added unconditionally: a required resource that only some
            // composition installs is a system that silently stops running, and
            // `materialize_demanded_character_sheets` failing its parameter
            // validation is exactly that failure wearing a panic. Empty is the
            // correct state for an app whose providers author no sheets.
            .init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>()
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
                    .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::Combat)
                    .after(crate::schedule::CombatSet::Playback)
                    .before(crate::schedule::CombatSet::Resolve),
            )
            .add_systems(
                sim,
                // A13: who a body sounds like, published BEFORE the move timeline
                // reads it. The move clock is the emitter this whole attribution
                // exists for, so a frame-scheduled publish would hand it stale (or
                // missing) attribution on exactly the ticks a rollback resimulates.
                (
                    presentation::publish_body_presentation_sources,
                    // G1/H1: the BACKSTOP for a projectile that reached the world
                    // without a source. The materializers stamp it themselves —
                    // this slot cannot cover a bolt that spawns and hits inside one
                    // tick, because the enemy pool spawns later in this same set
                    // and steps immediately.
                    presentation::inherit_projectile_presentation_sources,
                )
                    .chain()
                    .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::Combat)
                    .before(crate::schedule::CombatSet::Playback),
            )
            .add_systems(
                sim,
                // C4 slice 1: a match roster becomes bodies. Before the character
                // projection in the same phase, so a fighter seated this tick wears
                // its moveset and silhouette on the tick it appears rather than the
                // one after — the difference between a fighter that can be hit on
                // frame one and one that is briefly a bare rectangle.
                seating::seat_match_participants
                    // Seating needs an ASSEMBLED content composition. The archetype
                    // roster is built from registered fragments, so a bare engine
                    // App legitimately has none — and this is a run condition
                    // rather than an `Option<Res<..>>` parameter deliberately:
                    // `engine.character-authority-is-app-local` forbids making the
                    // character authority optional, and it is right to. "Not part
                    // of this composition" and "optional here" are different
                    // claims, and only the first one is true.
                    .run_if(resource_exists::<crate::features::CharacterRoster>)
                    .in_set(crate::schedule::PlayerInputSet::CharacterProjection)
                    .before(presentation::project_prepared_character_definitions),
            )
            .add_systems(
                sim,
                // C3/I1: a registered character's authored moveset and silhouette
                // land on the body BEFORE the PERSONA construction, which builds a
                // worn body's action set, moveset and identity baseline together
                // and then has equipment overlaid onto it. Landing after that pair
                // erased equipment-granted moves; the two agree now because
                // `apply_worn_character_kit` consults the same registry.
                //
                // Named as a PHASE, which is the whole point of D7. This used to
                // read `.in_set(Combat).before(apply_worn_character_gameplay)` —
                // and that leaf lives in `PlayerInput`, which PRECEDES `Combat`,
                // so the edge closed a cycle through the set ordering and nothing
                // at the call site could have revealed it. A phase name cannot
                // make that mistake: the set says where it runs.
                presentation::project_prepared_character_definitions
                    .in_set(crate::schedule::PlayerInputSet::CharacterProjection),
            )
            .add_systems(
                Update,
                (
                    // Declare before anything asks: a character registered only
                    // through `register_character` must not read as `Unknown`.
                    declare_registered_characters,
                    // The cast belongs to ONE session. Before anything stages into
                    // it, drop the previous session's.
                    retire_previous_session_cast,
                    demand_worn_character_sheets,
                    demand_actor_character_sheets,
                    // Before the drain: the audit reads OUTSTANDING demand, and the
                    // materializer empties it.
                    audit::report_character_capability_gaps,
                    // G4/H3: the declaration authorities, compared. Gated on any of
                    // the THREE changing rather than run every frame — it walks all
                    // of them, and the answer can only change when one does.
                    //
                    // Gating on the catalog alone was wrong even though startup
                    // masked it (both resources are new on the first frame): a
                    // character registered later, into an unchanged catalog, would
                    // never be compared against it. The condition has to name every
                    // input the audit reads, or it is a claim about invalidation
                    // that the schedule does not make (GPT 5.6, 2026-07-26).
                    audit::report_character_authority_conflicts.run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists_and_changed::<
                            CharacterCatalog,
                        >
                        .or(
                            bevy::ecs::schedule::common_conditions::resource_exists_and_changed::<
                                PreparedCharacterRegistry,
                            >,
                        )
                        .or(
                            bevy::ecs::schedule::common_conditions::resource_exists_and_changed::<
                                ambition_characters::actor::character_catalog::CharacterCatalogOwners,
                            >,
                        ),
                    ),
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
mod fight_tests;
#[cfg(test)]
mod tests;
