//! Session scope — activation-owned entity lifetime.
//!
//! A *session* is one activated gameplay experience: a launched Sanic run, a
//! launched Mary-O run, or the main game entered from a launcher. Every entity
//! created on behalf of that activation belongs to its [`SessionScopeId`] and is
//! retired with it.
//!
//! Session lifetime is distinct from [`super::ModeScopedEntity`]. Consecutive
//! runs may share a mode while requiring completely fresh runtime ownership.
//! Session identity is therefore minted once per activation and propagated
//! explicitly through [`SessionSpawnScope`] at the moment spawn work is
//! requested. A later route change cannot reassign a deferred spawn to another
//! activation.
//!
//! This abstraction sits below the game shell. Route providers map shell
//! activations to session scopes, while simulation and world-construction code
//! use the scope without importing shell vocabulary.

use std::ops::{Deref, DerefMut};

use bevy::ecs::change_detection::Ref;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::{Single, SystemParam};
use bevy::prelude::*;

use super::markers::RoomScopedEntity;

/// Stable identity of one activated gameplay session.
///
/// Minted from a deterministic monotonic counter so the same activation order
/// produces the same identities in replay and tests.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionScopeId(pub u64);

/// The currently active gameplay-session scope and its deterministic allocator.
#[derive(Resource, Default, Debug)]
pub struct ActiveSessionScope {
    current: Option<SessionScopeId>,
    next_raw: u64,
}

impl ActiveSessionScope {
    /// Mint a fresh scope identity WITHOUT making it current.
    ///
    /// ⛔⛤ **ALLOCATING A SESSION IDENTITY AND SELECTING THE LIVE SESSION ARE
    /// DIFFERENT OPERATIONS — SEPARATED 2026-09-14 (A10.5).** [`Self::begin`]
    /// did both in one statement, which is right for a session that is live the
    /// moment it exists and wrong for a CANDIDATE: a candidate needs an identity
    /// to own its entities and its content binding long before anything has
    /// decided it may be played. Calling `begin` for a candidate would make it
    /// current before its verdict, which is the whole premise inverted.
    ///
    /// ⚠ **THIS IS NOT A SECOND CURRENT-SCOPE AUTHORITY.** There is still exactly
    /// one `current`, written by [`Self::publish`] and [`Self::begin`] and
    /// cleared by the two clears. A reserved scope that is never published is
    /// simply an id nobody used, and gaps are legal.
    pub fn reserve(&mut self) -> SessionScopeId {
        let id = SessionScopeId(self.next_raw);
        self.next_raw += 1;
        id
    }

    /// Make a reserved scope the live one.
    ///
    /// ⛔ The publication half of the split above. A candidate session calls this
    /// when — and only when — it has been admitted.
    pub fn publish(&mut self, id: SessionScopeId) {
        self.current = Some(id);
    }

    /// Mint a fresh scope, make it current, and return it.
    ///
    /// The ordinary road, for a session that is live as soon as it exists.
    pub fn begin(&mut self) -> SessionScopeId {
        let id = self.reserve();
        self.publish(id);
        id
    }

    /// The active scope, when gameplay currently owns a session.
    pub fn current(&self) -> Option<SessionScopeId> {
        self.current
    }

    /// Capture the current scope for spawn work requested now.
    pub fn spawn_scope(&self) -> SessionSpawnScope {
        SessionSpawnScope::new(self.current)
    }

    /// Clear the active scope unconditionally.
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Clear only when `id` is still current. Retiring A after B activated must
    /// not clear B's spawn context.
    pub fn clear_if_current(&mut self, id: SessionScopeId) {
        if self.current == Some(id) {
            self.current = None;
        }
    }
}

/// Marker resource: this App's gameplay simulation belongs to shell-routed
/// gameplay sessions. Inserted by the session bridge (the host composition that
/// routes gameplay through a launcher); never inserted by direct-entry apps or
/// headless harnesses, whose synchronously published root is sufficient authority.
///
/// [`simulation_authorized`] reads it: with the marker present, the gameplay
/// simulation root set runs only while a session scope is live, so launcher /
/// title / loading frames run zero simulation against zero session entities.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct SessionGatedSimulation;

/// Optional process-start gate for visible direct-entry hosts.
///
/// The canonical session world may be constructed synchronously before its
/// presentation assets have settled. A visible host can insert this resource
/// in the closed state, present an opaque loading surface while it gathers
/// real readiness evidence, then open it once the first coherent gameplay
/// frame is safe to reveal. Apps that do not insert the resource retain the
/// existing behavior.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialGameplayReadiness {
    ready: bool,
}

impl InitialGameplayReadiness {
    /// Construct a gate that keeps gameplay simulation dormant.
    pub const fn closed() -> Self {
        Self { ready: false }
    }

    /// Construct an already-open gate.
    pub const fn open() -> Self {
        Self { ready: true }
    }

    /// Allow gameplay simulation to begin.
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    /// Whether gameplay may run.
    pub const fn is_ready(self) -> bool {
        self.ready
    }
}

/// Run condition for the gameplay-simulation root set.
///
/// Every app requires exactly one [`SessionRoot`] before gameplay systems may
/// run. Direct-entry and headless apps do not require shell scope identity, but
/// they still publish the same canonical root synchronously. A visible direct
/// host may also install [`InitialGameplayReadiness`] while its first coherent
/// presentation frame is being prepared. Shell-routed hosts additionally
/// require [`ActiveSessionScope`] to name that exact root. This keeps
/// empty/minimal apps, frontend routes, provider preparation, startup reveal,
/// and stale delayed roots structurally dormant instead of letting required
/// world parameters fail validation.
pub fn simulation_authorized(
    gate: Option<Res<SessionGatedSimulation>>,
    initial_readiness: Option<Res<InitialGameplayReadiness>>,
    scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
) -> bool {
    if initial_readiness
        .as_deref()
        .is_some_and(|readiness| !readiness.is_ready())
    {
        return false;
    }
    live_scope_of(gate.as_deref(), scope.as_deref(), &roots).is_some()
}

/// A captured entity-ownership context.
///
/// The value is copied into spawn commands when work is requested. It never
/// consults [`ActiveSessionScope`] during command application, so deferred work
/// remains attached to the activation that authored it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionSpawnScope {
    id: Option<SessionScopeId>,
    visibility: SessionSpawnVisibility,
}

/// ⛔⛤ **THE VISIBILITY IS A SPAWN POLICY, NOT PART OF THE OWNERSHIP IDENTITY.**
/// Two scopes naming the same session ARE the same session, whether one of them
/// is currently spawning hidden or not — and `SessionSpawnScope` is compared for
/// exactly that question all over the tree (a plan's scope against a
/// transaction's, a root's owner against the active one). Deriving `PartialEq`
/// with the new field would have made `candidate(7) != scoped(7)`, which is a
/// different claim entirely.
///
/// ⚠ There is no `Hash` here to keep in step; if one is ever added it must hash
/// `id` alone, for the same reason. `TransactionId` already reads `session.id()`
/// and nothing else, so no construction identity changes.
impl PartialEq for SessionSpawnScope {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SessionSpawnScope {}

/// Whether work captured under a session scope is spawned PUBLISHED or as part
/// of a hidden candidate session.
///
/// ⛔⛤ **A10.4: HIDING THE ROOT IS NOT HIDING THE SESSION.** Bevy's disabling
/// components do not inherit through ownership, so a candidate session whose
/// ROOT carries `InactiveCandidate` still spawned a fully visible initial player
/// — and the gameplay queries that find a body find it by its own markers, not
/// through the root. A candidate session prepared beside a live one would have
/// produced a published player A and a visible player B at once.
///
/// ⇒ The policy rides on the ownership context every session-owned spawn already
/// captures, so EVERY `spawn_*_scoped` / `insert_*_scoped` site is covered by one
/// statement rather than by teaching each site to ask which session is active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionSpawnVisibility {
    /// Ordinary session-owned work: visible the moment it is spawned.
    #[default]
    Published,
    /// Part of a candidate session that has not been admitted. Hidden until
    /// [`crate::construction::publish_candidate_session`] promotes the whole
    /// population.
    HiddenCandidate,
}

impl SessionSpawnScope {
    /// Process-/frontend-resident work with no gameplay-session owner.
    pub const UNSCOPED: Self = Self {
        id: None,
        visibility: SessionSpawnVisibility::Published,
    };

    /// Capture an explicit gameplay-session owner.
    pub const fn scoped(id: SessionScopeId) -> Self {
        Self {
            id: Some(id),
            visibility: SessionSpawnVisibility::Published,
        }
    }

    /// Capture an explicit gameplay-session owner whose work is being prepared
    /// OFF TO THE SIDE: every entity spawned under it is hidden until the
    /// candidate session is admitted.
    pub const fn candidate(id: SessionScopeId) -> Self {
        Self {
            id: Some(id),
            visibility: SessionSpawnVisibility::HiddenCandidate,
        }
    }

    /// Construct from an optional scope.
    pub const fn new(id: Option<SessionScopeId>) -> Self {
        Self {
            id,
            visibility: SessionSpawnVisibility::Published,
        }
    }

    /// Whether work captured under this scope spawns hidden.
    pub const fn visibility(self) -> SessionSpawnVisibility {
        self.visibility
    }

    /// The captured owner.
    pub const fn id(self) -> Option<SessionScopeId> {
        self.id
    }

    /// Resolve the spawn policy for a system that supports both legacy apps and
    /// session-aware shell hosts.
    ///
    /// An absent [`ActiveSessionScope`] resource means the app has not installed
    /// session lifecycle and therefore uses process-resident legacy spawning.
    /// A present resource with no current scope means the shell is at a
    /// non-gameplay experience, so gameplay-owned spawning sleeps.
    pub fn for_optional_active_session(active: Option<&ActiveSessionScope>) -> Option<Self> {
        match active {
            None => Some(Self::UNSCOPED),
            Some(active) => active.current().map(Self::scoped),
        }
    }

    /// Attach this ownership context to an already-created entity command.
    ///
    /// ⛔ **AND ITS VISIBILITY POLICY WITH IT.** This is the one place every
    /// session-owned spawn passes through, which is why the candidate policy
    /// lives on the scope rather than at each spawn site — see
    /// [`SessionSpawnVisibility`].
    pub fn apply_to(self, entity: &mut EntityCommands<'_>) {
        if let Some(id) = self.id {
            entity.insert(SessionScopedEntity(id));
        }
        if self.visibility == SessionSpawnVisibility::HiddenCandidate {
            crate::construction::hide_candidate_session_entity(entity);
        }
    }
}

impl From<SessionScopeId> for SessionSpawnScope {
    fn from(id: SessionScopeId) -> Self {
        Self::scoped(id)
    }
}

/// A single Bevy system parameter carrying entity commands and the session
/// ownership captured for work requested by that system invocation.
///
/// Besides making the intended spawn context explicit, this keeps large
/// gameplay systems within Bevy's supported system-parameter arity: replacing
/// separate `Commands` and `Option<Res<ActiveSessionScope>>` parameters with
/// `SessionCommands` consumes one parameter slot.
#[derive(SystemParam)]
pub struct SessionCommands<'w, 's> {
    commands: Commands<'w, 's>,
    active: Option<Res<'w, ActiveSessionScope>>,
}

impl SessionCommands<'_, '_> {
    /// Resolve the captured spawn policy for this system invocation.
    ///
    /// Legacy apps without [`SessionScopePlugin`] receive an unscoped command
    /// context. Shell hosts at a non-gameplay route receive `None`, allowing
    /// gameplay-owned systems to sleep rather than author frontend entities.
    pub fn spawn_scope(&self) -> Option<SessionSpawnScope> {
        SessionSpawnScope::for_optional_active_session(self.active.as_deref())
    }
}

impl<'w, 's> Deref for SessionCommands<'w, 's> {
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl<'w, 's> DerefMut for SessionCommands<'w, 's> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

/// Tag carried by every entity owned by a gameplay-session activation.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopedEntity(pub SessionScopeId);

/// Marker on the canonical root entity for a gameplay session.
///
/// ⛔ THERE IS EXACTLY ONE, ALIVE OR HIDDEN. A prepared candidate session is
/// not one — it carries [`CandidateSessionRoot`] until it is adopted, and the
/// swap happens at exactly one place, `publish_candidate_session`. See that
/// type for why invisibility is not the invariant.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionRoot(pub SessionScopeId);

/// Marker on the root entity of a PREPARED CANDIDATE session — a world built
/// under a scope that is not yet authoritative.
///
/// ⛔⛤ **A CANDIDATE IS NOT A SESSION ROOT, AND HIDING ONE IS NOT THE SAME
/// CLAIM — RULED 2026-09-19 (`Q132`).** The candidate used to carry
/// `SessionRoot` itself and rely on `InactiveCandidate` to keep ordinary
/// queries from seeing it. That made invisibility the invariant, which is a
/// different and weaker statement: every construction query that legitimately
/// says `Allow<InactiveCandidate>` — and several must, because publication
/// happens while the candidate is still hidden — saw TWO canonical roots for
/// one live world. The distinct marker makes the claim structural: counting
/// `SessionRoot` INCLUDING hidden entities is at most one, at every frame of a
/// handoff.
///
/// ⚠ THE `SimId` IS DELIBERATELY SHARED AND THAT IS A DIFFERENT QUESTION. A
/// hidden candidate carries the live root's `session:root` identity so two
/// hosts checksum a session identically — pinned by
/// `a_hidden_candidate_may_share_the_live_worlds_identity_and_a_published_one_may_not`.
/// What must not be shared is the CLAIM TO BE THE LIVE ROOT.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSessionRoot(pub SessionScopeId);

/// Read one component from the exact canonical live session-world root.
///
/// The root entity is the authority: at a frontend route no such entity exists,
/// while a gameplay activation owns exactly one. Systems using this parameter
/// therefore cannot accidentally fall back to process-resident world state.
pub type SessionWorldRef<'w, 's, T> = Single<'w, 's, Ref<'static, T>, With<SessionRoot>>;

/// Mutate one component on the exact canonical live session-world root.
pub type SessionWorldMut<'w, 's, T> = Single<'w, 's, &'static mut T, With<SessionRoot>>;

/// True only while the exact canonical live session-world root exists.
///
/// Direct-entry apps have no [`SessionGatedSimulation`] marker and therefore
/// require only one root. Shell-routed hosts additionally require that root's
/// scope to equal the active activation scope. A delayed root from A can never
/// wake gameplay or presentation while B is current or still preparing.
pub fn session_world_exists(
    gate: Option<Res<SessionGatedSimulation>>,
    active: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
) -> bool {
    live_scope_of(gate.as_deref(), active.as_deref(), &roots).is_some()
}

/// The query-side [`live_session_world_root`], shared by every SCOPE-AWARE
/// system-parameter form so the World-level and query-level answers cannot drift
/// apart: [`session_world_exists`], [`LiveSessionScope`], and the readiness
/// condition above.
///
/// ⛔⛤ **IT IS NOT SHARED BY [`SessionWorldRef`] / [`SessionWorldMut`], WHICH
/// THIS SENTENCE CLAIMED UNTIL 2026-09-18 AND WHICH ARE THE OVERWHELMING
/// MAJORITY OF THE PRODUCTION USES.** Those are `pub type` aliases for
/// `Single<.., With<SessionRoot>>` and ask no question about scope at all, so a
/// sentence saying "every form" is what stops the next reader looking.
///
/// ⚠ **THE SIZE IS DELIBERATELY NOT A NUMBER HERE.** `BEVY-SESSION-ROOT` in
/// `docs/planning/consolidation/architecture-census.md` owns it and prints the
/// method beside it. This comment said `193` until 2026-09-19 while that row
/// said `183`; both were right and neither carried its method — `183` counts the
/// parameter form `SessionWorldRef<`, `193` counts the bare name and so includes
/// the `use` imports. Two correct numbers that look like a contradiction is what
/// a number copied across a document boundary without its method produces.
///
/// ⇒ **AND THE SPLIT IS NOT MERELY PRUDENT, AS OF `Q132` (DECIDED 2026-09-19).**
/// There is exactly one canonical live `SessionRoot`, so a two-root frame is
/// INVALID rather than ambiguous. `Single` yielding `None` on such a frame is a
/// defence against a state that must not occur, not a designed response to one
/// that may; the scope-aware family below is for the lifecycle code that
/// legitimately sees both sides of a handoff.
///
/// ⚠ **AND THE REASON THE SIMPLE FORM IS ENOUGH LIVES IN ANOTHER FILE.** Two
/// roots do not coexist across a system boundary because `SessionScopeSet`
/// chains `RetireAuthority -> Cleanup -> Activate`: the retired scope's sweep
/// runs before the incoming activation installs its root. Reorder that and these
/// aliases start returning `None` in ordinary play, which is a silent skip
/// rather than an error — see `ORDER-SESSION-LIFECYCLE` in
/// `docs/planning/consolidation/architecture-census.md`.
fn live_scope_of(
    gate: Option<&SessionGatedSimulation>,
    active: Option<&ActiveSessionScope>,
    roots: &Query<&SessionRoot>,
) -> Option<SessionScopeId> {
    match gate {
        // Shell-routed: the activation names its root. Selecting by scope means
        // a lingering retired root is not a candidate rather than an ambiguity.
        Some(_) => {
            let active = active.and_then(ActiveSessionScope::current)?;
            roots.iter().map(|root| root.0).find(|owner| *owner == active)
        }
        // Direct-entry: exactly one root IS the authority.
        None => roots.single().ok().map(|root| root.0),
    }
}

fn unique_session_world_root(world: &World) -> Option<(Entity, SessionScopeId)> {
    // `try_query` builds read-only query state from `&World` and yields `None`
    // when `SessionRoot` was never registered — the correct "no root" answer.
    let mut query = world.try_query::<(Entity, &SessionRoot)>()?;
    let mut roots = query.iter(world).map(|(entity, root)| (entity, root.0));
    let root = roots.next()?;
    assert!(
        roots.next().is_none(),
        "more than one canonical SessionRoot exists"
    );
    Some(root)
}

/// The canonical root of the LIVE session, and the scope that owns it.
///
/// ⭐⭐ A SHELL-ROUTED HOST SELECTS BY SCOPE; IT DOES NOT ASSERT UNIQUENESS.
/// The activation names its root, so a root from a retired activation that has
/// not been despawned yet is simply NOT A CANDIDATE — it can neither be chosen
/// nor make the choice ambiguous.
///
/// ⛔⛔ THIS ASSERTED INSTEAD, AND THE ASSERT WAS THE HAZARD. A composition that
/// briefly held two roots PANICKED rather than resolving the live one, which is
/// the opposite of what ownership is for: a stale entity must be harmless, and a
/// process abort is not harmless. The same panic is already recorded once in
/// `ambition_app::app::resources` against a build-time root coexisting with an
/// activation's.
///
/// A direct-entry host has no activation to name a root, so uniqueness IS the
/// authority there and the assert stays.
fn live_session_world_root(world: &World) -> Option<(Entity, SessionScopeId)> {
    let mut query = world.try_query::<(Entity, &SessionRoot)>()?;
    if world.contains_resource::<SessionGatedSimulation>() {
        let active = world
            .get_resource::<ActiveSessionScope>()
            .and_then(ActiveSessionScope::current)?;
        return query
            .iter(world)
            .find(|(_, root)| root.0 == active)
            .map(|(entity, root)| (entity, root.0));
    }
    unique_session_world_root(world)
}

/// Locate the one exact live session-world root without constructing a
/// persistent query state. Useful at imperative App/World boundaries such as
/// snapshot codecs, CLI inspection, and focused tests.
///
/// Shell-routed worlds additionally require the root owner to equal the active
/// session scope. A delayed root from a retired activation therefore remains
/// structurally unreadable even at imperative boundaries.
pub fn session_world_entity(world: &World) -> Option<Entity> {
    live_session_world_root(world).map(|(entity, _)| entity)
}

/// The session root a ROOM TRANSACTION would publish into: the root owned by the
/// scope the transaction was prepared under, **including a hidden candidate
/// session's**.
///
/// ⛔⛤ **THIS IS NOT [`session_world_entity`], AND THE DIFFERENCE IS A10.4.**
/// That function answers *"which root is LIVE right now"* — correct for a system
/// reading the running world, and wrong for a publication, which is a question
/// about the session the transaction BELONGS to. A candidate session's first room
/// is prepared under the candidate's scope while the previous session's root is
/// still the live one, so `session_world_entity` would hand a room publication
/// the root of a session it has nothing to do with, and a candidate session's own
/// root is hidden from it entirely.
///
/// ⚠ `Allow<InactiveCandidate>` means *"entities WITH and WITHOUT the marker"*,
/// not "only hidden ones": an ordinary live session's root is found here exactly
/// as it is anywhere else.
///
/// ⛔⛤ **AND IT ACCEPTS EITHER MARKER, WHICH IS THE WHOLE REASON `Q132`'S
/// REPRESENTATION COULD LAND.** A candidate's root carries
/// [`CandidateSessionRoot`] rather than [`SessionRoot`] until it is adopted, so
/// a publication sink looked up by `SessionRoot` alone would not find the
/// session it belongs to, the first room's publication would refuse, and the
/// candidate would die before adoption — taking `publish_candidate_session`
/// with it. This is a question about the session a transaction BELONGS to, and
/// a candidate belongs to its own scope as surely as a live session does.
///
/// ⚠ ONE ROOT PER SCOPE STILL, ACROSS BOTH MARKERS. A scope holding a
/// `SessionRoot` and a `CandidateSessionRoot` at once is the duplicate-authority
/// condition under a new spelling, so the two-root refusal below counts the
/// union rather than each marker separately.
pub fn session_root_for_scope(world: &mut World, scope: SessionScopeId) -> Option<Entity> {
    // ⛔⛤ **TWO LOOKUPS RATHER THAN ONE `Or`, AND THE REASON IS `try_query`'S
    // CONTRACT.** `try_query_filtered` answers `None` when ANY component it
    // names is unregistered in this world, which is the honest "no root" answer
    // for a single marker and a silent, total blackout for a union: a fixture
    // that has never built a candidate has never registered
    // `CandidateSessionRoot`, so one `Or` query would refuse every publication
    // in every direct-entry composition in the project. MEASURED that way —
    // `a_room_publishes_into_a_session_root_that_is_still_a_hidden_candidate`
    // went red with `NoSessionRootToPublishInto` against a fixture whose root
    // carries a perfectly ordinary `SessionRoot`.
    let mut roots: Vec<Entity> = Vec::new();
    if let Some(mut query) = world.try_query_filtered::<(Entity, &SessionRoot), bevy::ecs::query::Allow<
        crate::construction::InactiveCandidate,
    >>() {
        roots.extend(
            query
                .iter(world)
                .filter(|(_, root)| root.0 == scope)
                .map(|(entity, _)| entity),
        );
    }
    if let Some(mut query) =
        world.try_query_filtered::<(Entity, &CandidateSessionRoot), bevy::ecs::query::Allow<
            crate::construction::InactiveCandidate,
        >>()
    {
        roots.extend(
            query
                .iter(world)
                .filter(|(_, root)| root.0 == scope)
                .map(|(entity, _)| entity),
        );
    }
    let mut found = roots.into_iter();
    let root = found.next()?;
    // ⛔⛤ **TWO ROOTS ON ONE SCOPE IS THE DUPLICATE-AUTHORITY CONDITION A10
    // FORBIDS, AND A `debug_assert` ALONE LETS THE SHIPPED GAME PICK ONE IN
    // SILENCE.** Every session-owned authority is read through this lookup, so
    // whichever root it happened to return first would then answer for the
    // session — an arbitrary choice, made per call, with nothing said. The assert
    // stays (a test build should stop dead), and the shipped build now SAYS so
    // rather than choosing quietly.
    if found.next().is_some() {
        tracing::error!(
            target: "ambition_platformer2d::lifecycle",
            "session scope {scope:?} owns more than one SessionRoot; every \
             session-owned authority read through this lookup is now answering \
             from an arbitrary one of them"
        );
        debug_assert!(
            false,
            "session scope {scope:?} owns more than one SessionRoot"
        );
    }
    Some(root)
}

/// Which gameplay session owns the one exact live session world, if any.
///
/// ⭐⭐ THE SCOPE ANY SESSION-OWNED AUTHORITY MUST NAME TO BE READ. At a
/// frontend route it is `None`; during gameplay it is the activation's; in a
/// direct-entry app or headless harness it is the root's own, which is what
/// lets a harness and the shell host share ONE ownership rule instead of two.
///
/// ⛔ NOT [`ActiveSessionScope::current`], which is `None` in every app that
/// never installed shell routing — the root is the authority, and this is the
/// same question [`session_world_entity`] answers, returning who instead of
/// which entity.
pub fn live_session_scope(world: &World) -> Option<SessionScopeId> {
    live_session_world_root(world).map(|(_, owner)| owner)
}

/// [`live_session_scope`] as a system parameter.
#[derive(SystemParam)]
pub struct LiveSessionScope<'w, 's> {
    gate: Option<Res<'w, SessionGatedSimulation>>,
    active: Option<Res<'w, ActiveSessionScope>>,
    roots: Query<'w, 's, &'static SessionRoot>,
}

impl LiveSessionScope<'_, '_> {
    /// The owning scope, or `None` when no session world is live.
    pub fn get(&self) -> Option<SessionScopeId> {
        live_scope_of(self.gate.as_deref(), self.active.as_deref(), &self.roots)
    }
}

/// Advance until [`session_world_entity`] resolves, returning the frame count.
///
/// Shell-routed hosts may need several updates before the prepared session root
/// exists. Returns `Err(max_frames)` instead of panicking when the budget expires.
pub fn settle_until_session_world(app: &mut App, max_frames: u32) -> Result<u32, u32> {
    for frame in 0..=max_frames {
        if session_world_entity(app.world()).is_some() {
            return Ok(frame);
        }
        app.update();
    }
    Err(max_frames)
}

/// Advance until the session has both a world and a controlled subject.
///
/// Use [`settle_until_session_world`] when the caller only needs room state;
/// controlled-body materialization may complete on a later frame.
pub fn settle_until_controlled_subject(app: &mut App, max_frames: u32) -> Result<u32, u32> {
    for frame in 0..=max_frames {
        let seated = session_world_entity(app.world()).is_some()
            && app
                .world()
                .get_resource::<crate::markers::ControlledSubject>()
                .is_some_and(|subject| subject.0.is_some());
        if seated {
            return Ok(frame);
        }
        app.update();
    }
    Err(max_frames)
}

/// Frames a shell activation is given to produce its world before a caller
/// gives up.
///
/// Preparation is eight work items behind a load barrier; a handful of frames
/// covers it with room to spare, and a bound that is far too generous would
/// turn a genuine hang into a slow test rather than a failure.
pub const SESSION_SETTLE_FRAMES: u32 = 240;

/// Read one canonical session-world component at an imperative World boundary.
pub fn session_world_component<T: Component>(world: &World) -> Option<&T> {
    world.get::<T>(session_world_entity(world)?)
}

/// Mutate one canonical session-world component at an imperative World boundary.
pub fn session_world_component_mut<T: Component<Mutability = Mutable>>(
    world: &mut World,
) -> Option<Mut<'_, T>> {
    let entity = session_world_entity(world)?;
    world.get_mut::<T>(entity)
}

/// Mutate one session-world component on an **explicitly resolved** root.
///
/// ⭐⭐ **THE ROOT IS HANDED IN, AND THAT IS THE WHOLE DIFFERENCE FROM
/// [`session_world_component_mut`]**, which asks *"which root is LIVE right
/// now"*. The moment a hidden candidate session exists those are two different
/// questions: a publication that verified against candidate B and then re-asked
/// could be applied to live A. So the caller that resolved the target keeps it
/// and passes it here — 2026-09-15 review, finding 5.
///
/// ⛔⛤ **IT ALSO EXISTS SO THE WRITE IS VISIBLE TO THE CENSUS, AND THAT IS NOT
/// COSMETIC.** `world.get_mut::<T>(root)` is the same three tokens as any other
/// component write, so `scripts/multi_writer_resource_census.py` could not see
/// the room publication at all — and the cost was paid in the wrong place:
/// `handle_ldtk_hot_reload` kept `SessionWorldMut<RoomSet>` on a parameter it
/// only READS, with a comment saying so, because demoting it would have taken
/// `RoomSet` to zero mutable-reach sites while the publication still replaced
/// it. An instrument that cannot see the real writer makes a reader pretend to
/// be one. ⇒ Naming the road is what let that pretence be removed.
///
/// ⛔⛤ **THE ASSERTION IS THE INVARIANT, NOT DECORATION — AND IT WAS A
/// `debug_assert!` UNTIL 2026-09-18, WHICH MADE THOSE TWO SENTENCES
/// CONTRADICT EACH OTHER.** Review finding: an invariant a release build does
/// not check is not an invariant, and this one guards a PUBLICATION BOUNDARY. A
/// target that is not a [`SessionRoot`] means the resolution upstream went
/// wrong, and in release the write would land on whatever entity was named —
/// publishing world-defining state into nowhere while reporting success.
///
/// ⚠ There is no performance argument for the debug-only form here: this runs
/// once per published component per publication, not per body per frame, and
/// the check is one `get`. ⇒ It is a plain `assert!`, and reaching it is a
/// crash rather than a silent wrong-entity write.
pub fn session_world_component_mut_at<T: Component<Mutability = Mutable>>(
    world: &mut World,
    root: Entity,
) -> Option<Mut<'_, T>> {
    assert!(
        world.get::<SessionRoot>(root).is_some(),
        "session-world state was written onto {root:?}, which carries no `SessionRoot`. \
         The publication resolved a target that is not a session root, so this write \
         would land on an unrelated entity and report success"
    );
    world.get_mut::<T>(root)
}

/// Insert one component into the canonical direct/test session-world root.
///
/// Provider activations should insert a complete prepared bundle through the
/// shell. This helper exists for small direct hosts and focused tests that
/// intentionally assemble the same root one component at a time.
pub fn insert_session_world_component<T: Component>(world: &mut World, component: T) -> Entity {
    let active_scope = world
        .get_resource::<ActiveSessionScope>()
        .and_then(ActiveSessionScope::current);
    let gated = world.contains_resource::<SessionGatedSimulation>();
    let entity = match unique_session_world_root(world) {
        Some((entity, owner)) => {
            assert!(
                !gated || active_scope == Some(owner),
                "cannot insert session-world state into stale root {owner:?} while {active_scope:?} is active"
            );
            entity
        }
        None => {
            let owner = active_scope.unwrap_or(SessionScopeId(0));
            world
                .spawn((Name::new("direct session world"), SessionRoot(owner)))
                .id()
        }
    };
    world.entity_mut(entity).insert(component);
    entity
}

/// Signal that a session scope has retired.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopeRetired(pub SessionScopeId);

/// Signal that a session scope has become the live one, before anything has
/// been built for it.
///
/// ⭐⭐ THE EDGE THAT MAKES CLEANUP HYGIENE. A process-global resource that
/// mirrors one live session is dangerous only if the NEXT session can read the
/// previous one's value. Re-establishing it when a session BEGINS closes that
/// without giving every reader an ownership check, because the value a session
/// reads is one its own activation wrote.
///
/// ⛔ Retirement alone could not do this. It is a cleanup that must happen, and
/// "must happen" is exactly the property a scheduling change, an abnormal exit
/// or a delayed frame can take away — which it did: a retired Smash match left
/// state that Ambition then read as its own.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionScopeActivated(pub SessionScopeId);

/// Stable schedule seam for exact scope retirement.
///
/// ⭐⭐ THE ORDER IS AN OWNERSHIP RULE, not three systems that happen to be
/// chained: an authority governing a scope stands down BEFORE the world it
/// governs is removed. Read the other way round and the authority observes its
/// own world vanishing underneath it, which is indistinguishable from
/// corruption — and that is precisely how a retired Smash match came to
/// poison the rollback timeline the next game would inherit.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionScopeSet {
    /// Authorities that GOVERN the retiring scope stand down: the rollback
    /// timeline, and anything else holding a claim over that session's world.
    ///
    /// ⛔ Scheduling here is hygiene, not correctness. An authority that misses
    /// this seam must still be inert for the next scope, because it names its
    /// owner and the next scope is not it.
    RetireAuthority,
    /// Exact retirement of entities owned by the retired session.
    Cleanup,
    /// A newly live scope re-establishes the process-global state that mirrors
    /// one session, BEFORE any provider builds that session's world.
    ///
    /// ⭐ This seam is why the retirement work above is hygiene. Whatever a
    /// skipped or abnormal teardown left standing is overwritten here by the
    /// session about to read it.
    Activate,
    /// Presentation systems may materialize activation-owned visuals after the
    /// provider has published its session world.
    Presentation,
}

/// `Commands` extensions that make captured session ownership explicit at each
/// spawn site.
pub trait SpawnSessionScopedExt {
    /// Spawn with the captured session owner. [`SessionSpawnScope::UNSCOPED`]
    /// deliberately creates process-/frontend-resident state.
    fn spawn_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Spawn with one explicit session identity.
    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Spawn an entity owned by both the active authored room and the captured
    /// gameplay session.
    fn spawn_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Populate an entity someone else allocated, giving it the same session
    /// ownership [`Self::spawn_session_scoped`] would have.
    ///
    /// The construction executor allocates a planned entity's root itself so a
    /// recipe cannot choose or commandeer one, which means recipes insert onto
    /// an entity rather than spawning it. These are the insert-shaped siblings
    /// of the spawn helpers above.
    fn insert_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_>;

    /// Populate an allocated entity as owned by both the active authored room
    /// and the captured gameplay session.
    fn insert_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_>;
}

impl SpawnSessionScopedExt for Commands<'_, '_> {
    fn spawn_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.spawn(bundle);
        scope.apply_to(&mut entity);
        entity
    }

    fn spawn_in_session<B: Bundle>(
        &mut self,
        scope: SessionScopeId,
        bundle: B,
    ) -> EntityCommands<'_> {
        self.spawn_session_scoped(SessionSpawnScope::scoped(scope), bundle)
    }

    fn spawn_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.spawn((RoomScopedEntity, bundle));
        scope.apply_to(&mut entity);
        entity
    }

    fn insert_session_scoped<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.entity(entity);
        entity.insert(bundle);
        scope.apply_to(&mut entity);
        entity
    }

    fn insert_room_in_session<B: Bundle>(
        &mut self,
        scope: SessionSpawnScope,
        entity: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let mut entity = self.entity(entity);
        entity.insert((RoomScopedEntity, bundle));
        scope.apply_to(&mut entity);
        entity
    }
}

/// Despawn every entity owned by a retired scope and clear the current pointer
/// when it still names that scope.
pub fn despawn_retired_session_entities(
    mut commands: Commands,
    mut retired: MessageReader<SessionScopeRetired>,
    mut active: ResMut<ActiveSessionScope>,
    scoped: Query<(Entity, &SessionScopedEntity)>,
) {
    for SessionScopeRetired(scope) in retired.read().copied() {
        for (entity, owner) in &scoped {
            if owner.0 == scope {
                commands.entity(entity).despawn();
            }
        }
        active.clear_if_current(scope);
    }
}

/// Installs session identity, retirement messages, and exact cleanup.
pub struct SessionScopePlugin;

impl Plugin for SessionScopePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveSessionScope>()
            .add_message::<SessionScopeRetired>()
            .add_message::<SessionScopeActivated>()
            .configure_sets(
                Update,
                // ⛔⛤ **THE RETIRING SCOPE FINISHES DYING BEFORE THE NEW ONE
                // IS BORN, AND THE ORDER USED TO BE THE OTHER WAY.**
                //
                // MEASURED 2026-09-13: a shell route swap emits `RouteDeactivated`
                // and `RouteActivated` from ONE run of
                // `translate_shell_session_lifecycle`, so a handoff retires and
                // activates in the SAME frame. With `Cleanup` last, the incoming
                // session's provider built its room while the OUTGOING scope's
                // placements were still live — the world log's
                // `session-end activation=2 / session-start activation=3` frame
                // ended with `room-refused central_hub_complex :: 18x Duplicated`,
                // the entire room, because every root it minted duplicated one the
                // sweep had not taken yet.
                //
                // ⛔ And the same order let `reset_session_scoped_resources_on_retire`
                // REMOVE `SessionMechanics` after the activation that installed it,
                // so the frozen generation registries died in the frame they were
                // published.
                //
                // ⇒ Both are the same defect: work belonging to the DEAD scope was
                // scheduled after work belonging to the LIVE one. The fix is the
                // order, not a guard at either site.
                (
                    SessionScopeSet::RetireAuthority,
                    SessionScopeSet::Cleanup,
                    SessionScopeSet::Activate,
                    SessionScopeSet::Presentation,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                despawn_retired_session_entities.in_set(SessionScopeSet::Cleanup),
            );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod settle_tests {
    use super::*;
    use crate::lifecycle::markers::RoomVisual;

    /// A world that is already there settles in ZERO frames.
    ///
    /// They disagree only about when.
    #[test]
    fn a_build_time_root_settles_immediately() {
        let mut app = App::new();
        let entity = insert_session_world_component(app.world_mut(), RoomVisual);
        assert_eq!(settle_until_session_world(&mut app, 8), Ok(0));
        assert_eq!(session_world_entity(app.world()), Some(entity));
    }

    /// A world that never arrives is an ERROR, not a hang and not a panic.
    ///
    /// the caller this replaces reads `.expect("active session RoomSet")`
    /// three lines after its update loop, so a session that never activated
    /// surfaced as a panic naming the component rather than the barrier. An
    /// exhausted budget is a fact the caller can report.
    #[test]
    fn a_world_that_never_arrives_reports_the_budget_it_spent() {
        let mut app = App::new();
        assert_eq!(settle_until_session_world(&mut app, 4), Err(4));
    }

    /// A LATE root is found, which is the whole point.
    #[test]
    fn a_root_that_appears_on_a_later_frame_is_waited_for() {
        #[derive(Resource, Default)]
        struct Frames(u32);

        let mut app = App::new();
        app.init_resource::<Frames>();
        app.add_systems(
            Update,
            |mut frames: ResMut<Frames>, mut commands: Commands| {
                frames.0 += 1;
                if frames.0 == 3 {
                    commands.spawn((SessionRoot(SessionScopeId(0)), RoomVisual));
                }
            },
        );
        assert_eq!(settle_until_session_world(&mut app, 16), Ok(3));
    }
}
