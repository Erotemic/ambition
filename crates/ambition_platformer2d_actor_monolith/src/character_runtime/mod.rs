//! Engine-owned character loading and materialization.
//!
//! Applications declare characters and submit `CharacterLoadDemand`; the engine
//! resolves those demands into `CharacterLoadStates`. Demand carries only the
//! character tokens that require art, so room staging, match rosters, startup,
//! and worn-identity changes share the same materialization path.

use ambition_characters::prepared::PreparedCharacterRegistry;
pub mod audit;
pub mod definition;
pub mod hurtbox;
pub mod live_match_clock;
pub mod physical_baseline;
pub mod prepared_match;
pub mod presentation;
pub mod seating;
pub mod staging;

#[cfg(test)]
// Test-only preparation seams supplied by `ambition_characters::test-support`.
#[cfg(test)]
pub(crate) use ambition_characters::prepared::{
    prepare_and_finalize_against_for_test, prepare_and_finalize_for_test,
};
pub use audit::{
    audit_character_capabilities, character_reveal_ready, unsettled_staged_characters,
    CharacterCapabilityGap,
};
// ⛔⛔ DO NOT REPUBLISH `ambition_characters::prepared`'s NAMES HERE. A
// re-export gives them an address that names the monolith as their owner, and
// every coupling census reads it that way — nine of them once accounted for ~250
// call sites, 57 outside this crate. Callers name the crate that owns the thing.
pub use definition::CharacterDefinitionAppExt;
pub use hurtbox::{
    resolve_hurtboxes, AuthoredHurtboxes, BodyPoseClock, HurtboxSelection, ResolvedHurtboxes,
    POSE_AIRBORNE, POSE_HITSTUN, POSE_IDLE,
};
pub use physical_baseline::{
    BaselineBoundary, BodyGeometry, DisplacedPhysicals, PhysicalBaseline, PhysicalRetraction,
};
pub use prepared_match::{
    activate_the_prepared_match, declare_the_match_cast_as_the_view, effective_abilities,
    prepare_match, prepare_the_match, release_the_opening_hold, seat_placement, ControlAuthority,
    MatchPreparationProblems, MatchRules, OpeningPhase, PreparedMatch, PreparedSeat, OPENING_BEATS,
};
pub use presentation::{
    authorize_staged_character_presentation_sources, grant_prepared_character_body,
    inherit_projectile_presentation_sources, project_prepared_character_definitions,
    provider_of_character, publish_body_presentation_sources, KitOwnership, ProjectedCharacterKit,
};
pub use seating::{match_participants, ActiveMatch, MatchInstance, MatchSeat};

#[cfg(test)]
use ambition_characters::actor::definition::CharacterDefinition;

/// Body-complete fixture cast for tests that need registered characters but do
/// not care which creatures they are.
#[cfg(test)]
pub(crate) fn fixture_cast(ids: &[&str]) -> PreparedCharacterRegistry {
    let mut registry = PreparedCharacterRegistry::default();
    for id in ids {
        let mut definition = CharacterDefinition::new(*id, *id, "test")
            .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                run_speed: 155.0,
                move_style: ambition_characters::brain::MoveStyleSpec::Walk,
                ..Default::default()
            })
            .with_contact_damage(ambition_characters::actor::ContactDamage {
                strength: 0.70,
                amount: 1,
            })
            .with_autonomous_profile(ambition_characters::brain::BrainProfile {
                patrol_effort: 0.6774,
                chase_effort: 1.0,
                aggro_radius: 460.0,
                attack_range: 150.0,
                ..Default::default()
            });
        definition.vitals.max_health = Some(4);
        let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
    }
    registry
}
pub use staging::{
    ControllerBinding, DirectStartupSpec, MatchItemSpawns, MatchParticipant,
    MatchParticipantRoster, NormalizedEffort, RoomStagingPlan, RosterProblem, RosterSeating,
    StagesCharacters,
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

    /// Take at most `limit` tokens, leaving the rest pending for later frames.
    ///
    /// ⛔⛔ WHY A LIMIT EXISTS AT ALL: a character is ~7 sheets at 4096x4096, about
    /// **470MB of decoded RGBA**, and `take()` drained the WHOLE demand set in one
    /// frame — so every fighter's sheets started loading together, finished
    /// together, and were extracted into the render world together. The first
    /// hardware profile (2026-08-29) measured the result:
    /// `extract_render_asset<GpuImage>` at **454.9ms against a 0.1ms mean**, inside
    /// a 516ms frame.
    ///
    /// ⭐ The hitch is not how long a decode takes — decode is already async on the
    /// IO pool. It is **how many finished decodes land on the same frame**. Starting
    /// them on different frames is what spreads the landing.
    ///
    /// ⚠ This DELAYS readiness, it does not drop it: whatever is not taken stays
    /// pending and is taken next frame, and the reveal barrier
    /// (`character_reveal_ready`) still waits for every demanded token to reach a
    /// terminal state. That is affordable precisely because demand is now raised at
    /// match PREPARATION rather than at the opening bell — there are frames to
    /// spread across.
    fn take_bounded(&mut self, limit: usize) -> BTreeSet<String> {
        if limit == 0 || self.pending.len() <= limit {
            return self.take();
        }
        // `BTreeSet`, so this split is by sorted token — deterministic, which a
        // rollback host needs and a `HashSet` could not promise.
        let mut taken = BTreeSet::new();
        for _ in 0..limit {
            let Some(token) = self.pending.iter().next().cloned() else {
                break;
            };
            self.pending.remove(&token);
            taken.insert(token);
        }
        taken
    }
}

/// How many characters may BEGIN materialising on one frame.
///
/// ⭐ One, because one character is ~7 sheets at 4096x4096 (~470MB of RGBA) and the
/// cost that lands on a frame is the render-world extract of everything that
/// finished decoding. Spreading the STARTS spreads the finishes.
/// ⚠ Not a memory budget — an arrival-rate limit. Eviction is a separate question.
///
/// ⭐ SWEPT, SO THE VALUE IS A CHOICE AND NOT A GUESS. One same-block run per arm
/// of `capture_scene hall_of_characters` (the gallery, worst case on purpose):
///
/// ```text
/// bound   worst simultaneous decodes   worst frame
///   0                          31         1049.0ms
///   1                          14          222.3ms
///   2                          14          393.1ms
/// ```
///
/// ⇒ bounding at all is what matters: 31 → 14 and ~1049ms → a few hundred.
/// ⚠ **1 AND 2 ARE NOT SEPARATED BY THIS DATA** — same simultaneous count, and one
/// run each cannot tell 222ms from 393ms under a software rasteriser. 1 is the
/// conservative end and nothing here argues for 2; raising it would want reps.
const MAX_CHARACTERS_MATERIALIZED_PER_FRAME: usize = 1;

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

/// Canonical character ids staged during this [`SessionScopeId`]. This is a cumulative
/// session-capability set, not a live roster: presentation-source authorization only adds sources
/// and clears them when a new session begins, so this set follows the same lifetime.
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

/// The canonical character id a demand token names.
///
/// Rooms, LDtk entities and roster entries all legitimately submit display names (`"Mary-O"`),
/// while every provider map — the prepared registry, the assembled catalog's owners — is keyed
/// by stable id (`"mary_o"`).
///
/// Deliberately NOT resolved through the sprite table, which is the other place a
/// token → id alias exists. The table answers about ART, and the cast is a roster
/// that has to be right for a character whose art never resolves at all — a
/// composition with no asset pipeline has an empty sheet table and a full cast.
/// (Its declarations do now outlive the decode, so it COULD answer; that makes it
/// a convenience, not an authority.)
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

/// Marker that the engine character-materialization service is installed.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CharacterMaterializationService;

/// Ensure a registered character is declared in the sprite table before a
/// synchronous materialization demand can classify its token as unknown.
///
/// Declaration is idempotent metadata registration, not sprite decoding.
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
        sprites.declare(prepared.id.as_str(), &prepared.display_name);
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
    // Sheets this app's PROVIDERS authored. A source of sheet
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
    // ⭐ ONE CHARACTER PER FRAME. See `take_bounded`: each is ~470MB of decoded
    // RGBA, and landing two in one frame is what produced a 516ms frame on
    // hardware. Anything not taken stays pending for the next frame.
    for token in demand.take_bounded(MAX_CHARACTERS_MATERIALIZED_PER_FRAME) {
        // Whose cues this character will emit under, resolved BEFORE any decode:
        // the cast is a roster, not a report on the art, and it must be right for a
        // character whose sheet never resolves.
        let character_id = canonical_character_id(registry, character_catalog, &token).to_string();
        declare_registered_character_into(sprites, registry, &token, &character_id);
        // They are different bugs.
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

/// Declare every registered character into the sprite read model.
///
/// The prepared registry is a source of declarations, and this is where it becomes one.
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

/// An ACTOR that resolved a character identity needs that art too.
///
/// That system watches `WornCharacter` — the identity a body PUTS ON. An `EnemySpawn` wears
/// nothing: it resolves its character through the display-name join (`ActorClusterSeed` →
/// `catalog.id_for_display_name`) and carries the answer on `ActorConfig::sprite_character_id`.
/// So a room full of authored enemies declared their characters, resolved them correctly, and
/// never asked for the art.
///
/// Four of them stand in `intro_escape_shaft`, in the sequence a stranger plays first .
///
/// `Added` rather than `Changed`. An actor's config is rebuilt every tick
/// as a read-model (`sync_actor_read_models` restores its reaction timers over a
/// fresh value), so `Changed` here would re-request the whole room's cast every
/// frame. The identity is decided at construction and does not drift, so asking
/// once when the component appears is both sufficient and the only affordable
/// option.
pub fn demand_actor_character_sheets(
    actors: Query<
        &ambition_combat::actor_tuning::ActorConfig,
        Added<ambition_combat::actor_tuning::ActorConfig>,
    >,
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
/// Ask for the whole cast the moment a ROSTER names it, not when its bodies spawn.
///
/// ⛔⛔ THIS IS THE UPSTREAM HALF, AND ITS ABSENCE IS A MEASURED HITCH.
/// `demand_actor_character_sheets` below keys on `Added<ActorConfig>` — the
/// instant a BODY exists — which is the opening bell. The first hardware profile
/// (2026-08-29) caught the consequence: **+307 megapixels of 4096x4096 sheets
/// decoded inside a 2.5s window whose worst frame was 516ms**, because one
/// character is ~7 sheets and ~470MB of RGBA and none of it was asked for until
/// the fighter stood on the stage.
///
/// ⭐ The ROSTER knows the cast strictly earlier: `MatchParticipant::character` is
/// a `CharacterId` and the roster is published at select/prepare time, before any
/// body is seated. Moving the consumer upstream widens what it sees — the select
/// screen knows who is playing, and a spawn only knows who just arrived.
///
/// ⚠ ADDITIVE, NOT A REPLACEMENT. `demand_actor_character_sheets` stays: a body
/// can appear that no roster named (a summon, a possession, a dev spawn), and
/// demand is a SET, so asking twice for the same character is free. This system
/// makes the roster case EARLY, it does not make the spawn case wrong.
pub fn demand_rostered_character_sheets(
    roster: Option<Res<staging::MatchParticipantRoster>>,
    demand: Option<ResMut<CharacterLoadDemand>>,
) {
    let (Some(roster), Some(mut demand)) = (roster, demand) else {
        return;
    };
    // Only when the roster itself moved: this walks every participant, and the
    // answer cannot change while the roster does not.
    if !roster.is_changed() {
        return;
    }
    for participant in &roster.participants {
        demand.request(participant.character.as_str());
    }
}

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

/// A quality Apply converges the art a session is already showing.
///
/// The transition is three moves and no new machinery:
///
/// 1. compare each resident realization's TIER against the active one;
/// 2. retire the stale ones back to `Declared` — dropping the
///    [`CharacterSpriteAsset`](ambition_sprite_sheet::character::CharacterSpriteAsset)
///    drops its strong `Handle<Image>`, and Bevy frees the image once the last
///    strong handle goes, so residency FALLS with no evictor anywhere;
/// 3. demand them again, which the materializer four systems later satisfies at
///    the new tier.
///
/// Logical identity never moves: the same `character_id`, the same demand token,
/// the same body entity, the same gameplay authority. Only the physical
/// realization is replaced.
///
/// `UserSettings`, the same source the materializer reads. Comparing
/// against one authority and stamping from another is how a transition becomes a
/// loop: every frame retires a realization that is immediately remade with the
/// tier it just failed.
pub fn converge_character_residency_to_active_quality(
    // NOT `Res`: this writes. But it is READ first (see below), because a
    // `ResMut` deref-mut marks `GameAssets` changed for every reader downstream,
    // every frame, forever.
    assets: Option<ResMut<ambition_sprite_sheet::game_assets::GameAssets>>,
    demand: Option<ResMut<CharacterLoadDemand>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
) {
    let (Some(mut assets), Some(mut demand)) = (assets, demand) else {
        return;
    };
    let budget = settings.map(|settings| settings.video.quality.resolved_budget());
    let active = crate::character_sprites::character_sprite_tier(budget.as_ref());
    // Read through the immutable deref: nothing is stale on almost every frame,
    // and taking the mutable borrow anyway would republish `GameAssets` at 60Hz.
    if !assets.characters.has_stale_realizations(active) {
        return;
    }
    let stale = assets.characters.demote_stale_realizations(active);
    bevy::log::info!(
        target: "ambition_platformer2d::character_sprites",
        "quality transition to {active:?}: retired {} character realization(s) and \
         re-demanded them",
        stale.len(),
    );
    demand.request_all(stale);
}

/// A new session gets a new cast.
///
/// `ActiveAudioSelection` resets on session select; the cast has to reset with it or the reset
/// means nothing.
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
/// Added unconditionally by the host simulation plugin so no application can compose the
/// engine without it.
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
            // THE MATCH CLOCK. Beside the plugin that owns `ActiveMatch`, so a
            // composition that can activate a match can time one — the readers
            // take it as a plain `Res` and would fail parameter validation
            // otherwise.
            .init_resource::<live_match_clock::LiveMatchTicks>()
            .add_systems(
                // The SIM schedule, not `Update`. (§4.11)
                //
                // These two read and write simulation state — a pose clock and the volumes
                // damage resolves against — so under rollback they must recompute on every
                // resimulated tick. In `Update` they ran once per FRAME while the sim re-ran
                // many times, which left `ResolvedHurtboxes` stale for every rewound tick even
                // though it is declared rollback-DERIVED on the promise that the sim rebuilds
                // it.
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
                    // Both edges are load-bearing. And every body's position is already
                    // post-movement here (`PlayerSimulation` and `WorldPrep` both precede
                    // `Combat`), so this is the one slot where clocks and positions are
                    // simultaneously current.
                    .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Combat)
                    .after(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback)
                    .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Resolve),
            )
            .add_systems(
                sim,
                // The move clock is the emitter this whole attribution exists for, so a
                // frame-scheduled publish would hand it stale (or missing) attribution on
                // exactly the ticks a rollback resimulates.
                (
                    presentation::publish_body_presentation_sources,
                    // G1/H1: the BACKSTOP for a projectile that reached the world
                    // without a source. The materializers stamp it themselves —
                    // this slot cannot cover a bolt that spawns and hits inside one
                    // tick, because immediate projectile requests materialize later
                    // in this same set and step immediately.
                    presentation::inherit_projectile_presentation_sources,
                )
                    .chain()
                    .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Combat)
                    .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback),
            )
            .add_systems(
                sim,
                // Before the character projection in the same phase, so a fighter seated this tick
                // wears its moveset and silhouette on the tick it appears rather than the one after
                // — the difference between a fighter that can be hit on frame one and one that is
                // briefly a bare rectangle. PREPARE, then ACTIVATE, chained on one tick. Two
                // systems rather than one because they answer different questions and only one of
                // them may fail: preparation resolves every permanent question against the
                // character authorities, and activation builds the cast from the answer without
                // consulting any authority at all. GGRS said so directly — *"sync-test checksum
                // mismatch at frames [10, 11, 12]"* — which is this repo's own "a derive's MEMO is
                // rollback state" trap: a value that GATES behaviour is not a cache.
                //
                // Activation stays here because building bodies is simulation,
                // and it replays correctly precisely because the plan it reads
                // was decided outside the window and cannot have changed.
                (
                    prepared_match::activate_the_prepared_match,
                    // The ceremony's other end: the tick the hold comes off.
                    // Chained after activation so a match whose ruleset declares
                    // NO countdown still releases on the tick it is built — the
                    // behaviour every match had before ceremonies existed.
                    prepared_match::release_the_opening_hold,
                )
                    .chain()
                    // Preparation needs an ASSEMBLED content composition, and a
                    // bare engine App legitimately has none. This is a run
                    // condition rather than an `Option<Res<..>>` parameter
                    // deliberately: `engine.character-authority-is-app-local`
                    // forbids making the character authority optional, and it is
                    // right to. "Not part of this composition" and "optional
                    // here" are different claims, and only the first is true.
                    //
                    // it gates on what preparation actually REQUIRES — the
                    // catalog its `Res<CharacterCatalog>` would panic without.
                    // Until AC6 it gated on the enemy archetype roster instead: a
                    // table about hostile bodies, standing in for "content was
                    // assembled" because it happened to be assembled at the same
                    // moment. The proxy went with the ontology; the requirement
                    // was always this one.
                    .run_if(resource_exists::<
                        ambition_characters::actor::character_catalog::CharacterCatalog,
                    >)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::CharacterProjection)
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
                // A phase name cannot make that mistake: the set says where it runs.
                presentation::project_prepared_character_definitions
                    .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::CharacterProjection),
            )
            .add_systems(
                sim,
                // HOW LONG THIS MATCH HAS BEEN FOUGHT, counted once for every
                // consumer. `WorldPrep` because both readers are downstream —
                // the timeout in `Combat` and the item cadence in the item
                // chain — so they see THIS tick's count rather than last
                // tick's, and neither has to state an ordering against the
                // other to agree about the number.
                //
                // ⛔ DELIBERATELY UNGATED on `gameplay_allowed`: the clock's own
                // answer to a stopped world is `sim_dt == 0`, which is the
                // condition itself rather than a schedule-level proxy for it,
                // and a gate here would be a second opinion that could disagree.
                live_match_clock::count_the_live_match_ticks
                    .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
            )
            .add_systems(
                Update,
                (
                    // The same requirement as the activation registration above,
                    // named the same way.
                    prepared_match::prepare_the_match.run_if(resource_exists::<
                        ambition_characters::actor::character_catalog::CharacterCatalog,
                    >),
                    // The camera's answer for a match with no local participant.
                    // In `Update` beside preparation because it is a projection
                    // FOR presentation, not simulation state.
                    prepared_match::declare_the_match_cast_as_the_view,
                ),
            )
            .add_systems(
                Update,
                (
                    // Declare before anything asks: a character registered only
                    // through `register_character` must not read as `Unknown`.
                    declare_registered_characters,
                    // The cast belongs to ONE session.
                    retire_previous_session_cast,
                    // EARLIEST FIRST: the roster names the cast before any body
                    // is seated, so this is the one that gets the decode started
                    // during preparation instead of at the opening bell.
                    demand_rostered_character_sheets,
                    demand_worn_character_sheets,
                    demand_actor_character_sheets,
                    // Before the drain: the audit reads OUTSTANDING demand, and the
                    // materializer empties it.
                    audit::report_character_capability_gaps,
                    // G4/H3: the declaration authorities, compared. Gated on any of
                    // the THREE changing rather than run every frame — it walks all
                    // of them, and the answer can only change when one does.
                    //
                    // The condition has to name every input the audit reads, or it is a claim
                    // about invalidation that the schedule does not make.
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
                    // AFTER the audit and immediately BEFORE the drain: the
                    // demand this re-raises is satisfied in the same frame, so
                    // §4.9's readiness barrier never sees a transient unsettled
                    // character that a quality change created.
                    converge_character_residency_to_active_quality,
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
