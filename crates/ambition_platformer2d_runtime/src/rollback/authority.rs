//! Rollback authority, owned by exactly one gameplay session.
//!
//! # Three lifetimes
//!
//! ```text
//! Process
//!   └── Gameplay session          (SessionScopeId)
//!         └── Rollback timeline   (RollbackTimelineGeneration)
//! ```
//!
//! Keeping them apart is the whole job of this module, because the word
//! "session" was doing both of the inner two and the confusion had a symptom:
//! quit a Smash match to the title, start Ambition, and the doors stopped
//! working. Smash's timeline observed its own world being retired, read that as
//! a mid-session content disappearance, and invalidated itself — and the
//! invalidation was a process-global value the next game read as its own.
//!
//! # The one rule
//!
//! ```text
//! previous.owner == incoming.owner   -> the diagnosis CARRIES
//! previous.owner != incoming.owner   -> a FRESH authority
//! ```
//!
//! A new timeline inside one gameplay session inherits that session's health,
//! so a desync cannot launder itself by restarting or rebasing GGRS (AC23). A
//! new gameplay session inherits nothing, because nothing session A's timeline
//! discovered is a fact about session B's world.
//!
//! [`ActiveRollbackAuthority::installed`] is the only place that rule is
//! written. No caller chooses between preserving and clearing.
//!
//! # Ownership, not cleanup
//!
//! Every read names a scope ([`ActiveRollbackAuthority::confirmation_for`]),
//! and an authority that does not govern that scope answers
//! [`RollbackConfirmationState::Unavailable`] — never the other scope's health.
//! Retirement removing the resource is hygiene; the owner check is what makes a
//! survivor harmless.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{LiveSessionScope, SessionScopeId};

use crate::{PreparedContentIdentity, SnapshotSchemaFingerprint};

/// Backend-neutral availability/health of confirmation authority for a rollback
/// host. Room/lifecycle policy only needs to know whether a speculative intent
/// may be promoted to host-side work.
///
/// ⛔ NOT A RESOURCE. It is the ANSWER [`ActiveRollbackAuthority`] gives to a
/// scope that asked, and it was a bare process-global resource for exactly as
/// long as it took one game's answer to become another game's.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RollbackConfirmationState {
    #[default]
    Unavailable,
    Healthy,
    Unhealthy,
}

impl RollbackConfirmationState {
    pub fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

/// One rollback timeline within one gameplay session.
///
/// Monotonic for the whole process. Frame numbers restart at zero for every
/// GGRS session, so a stopped-and-restarted timeline is otherwise
/// indistinguishable from the one it replaced; host-side journals and traces
/// use this to discard work from timelines that no longer exist.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RollbackTimelineGeneration(pub u64);

/// Health of ONE rollback timeline.
///
/// Named for the timeline rather than "the session" on purpose: it is the fact
/// that carries across a rebase within a gameplay session and must not carry
/// across gameplay sessions, and the old name said neither.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RollbackTimelineStatus {
    pub mismatch_frames: Vec<i32>,
    pub invalidation: Option<String>,
}

impl RollbackTimelineStatus {
    /// Whether this timeline may still authorize confirmed host-side effects.
    ///
    /// Keeping the predicate on the status itself gives every host-side gate the
    /// same answer instead of re-deriving a subtly different idea of "healthy".
    pub fn is_healthy(&self) -> bool {
        self.invalidation.is_none() && self.mismatch_frames.is_empty()
    }

    /// The status a NEW timeline of the SAME gameplay session starts from. (AC23)
    ///
    /// So the diagnostic CARRIES. An unhealthy timeline hands its reason to the
    /// timeline that replaces it, and the only way to clear it is to say so
    /// ([`Self::acknowledge_and_clear`]).
    ///
    /// ⛔ `mismatch_frames` does NOT carry, and that is not an oversight: frame
    /// numbers restart at zero for every GGRS session, so carrying them forward
    /// would report a mismatch at frames the new timeline has not reached yet.
    /// The reason survives as prose, which is the part a reader acts on.
    pub fn carried_from(previous: Option<&Self>) -> Self {
        let Some(previous) = previous else {
            return Self::default();
        };
        let inherited = previous.invalidation.clone().or_else(|| {
            (!previous.mismatch_frames.is_empty()).then(|| {
                format!(
                    "GGRS sync-test checksum mismatch at frames {:?} on the PREVIOUS timeline",
                    previous.mismatch_frames
                )
            })
        });
        Self {
            mismatch_frames: Vec::new(),
            invalidation: inherited,
        }
    }

    /// Clear an inherited diagnostic DELIBERATELY.
    ///
    /// The escape hatch, named for what it is. A tool that has shown the
    /// divergence to a human and been told to carry on calls this; nothing on
    /// the ordinary install path does.
    pub fn acknowledge_and_clear(&mut self) {
        self.mismatch_frames.clear();
        self.invalidation = None;
    }
}

/// What a live timeline promised about the world it rewinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackTimelineContract {
    pub content: Option<PreparedContentIdentity>,
    pub schema: SnapshotSchemaFingerprint,
}

/// Whether a timeline is currently running under this authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Timeline {
    /// A rollback session resource exists and is speculating.
    Live,
    /// The timeline stopped, but the gameplay session that owned it did not.
    /// A diagnosis recorded before it stopped still refuses confirmed work,
    /// which is what stops a desync being laundered by a teardown.
    StoodDown,
}

/// THE rollback authority: whose it is, which timeline, and what it says.
///
/// One resource, because the four facts underneath are one fact. Splitting them
/// is what allowed a status with no owner to outlive the contract that produced
/// it.
#[derive(Resource, Clone, Debug)]
pub struct ActiveRollbackAuthority {
    owner: Option<SessionScopeId>,
    generation: RollbackTimelineGeneration,
    timeline: Timeline,
    contract: RollbackTimelineContract,
    status: RollbackTimelineStatus,
}

impl ActiveRollbackAuthority {
    /// Install a timeline for `owner`, given whatever authority preceded it.
    ///
    /// ⭐⭐ THE ONE RULE, WRITTEN ONCE. Same gameplay session: the outgoing
    /// timeline's diagnosis and generation counter carry forward. Different
    /// gameplay session: a fresh authority. A caller supplies who and what, and
    /// never whether.
    pub fn installed(
        previous: Option<&Self>,
        owner: Option<SessionScopeId>,
        contract: RollbackTimelineContract,
    ) -> Self {
        let same_session = previous.is_some_and(|previous| previous.owner == owner);
        Self {
            owner,
            // The counter is monotonic per process even across gameplay
            // sessions: two timelines must never share an identity, or a
            // journal cannot tell whose work it is holding.
            generation: RollbackTimelineGeneration(
                previous
                    .map(|previous| previous.generation.0)
                    .unwrap_or(0)
                    .wrapping_add(1),
            ),
            timeline: Timeline::Live,
            contract,
            status: RollbackTimelineStatus::carried_from(
                same_session.then(|| &previous.expect("same_session implies Some").status),
            ),
        }
    }

    /// The gameplay session this authority governs.
    pub fn owner(&self) -> Option<SessionScopeId> {
        self.owner
    }

    pub fn generation(&self) -> RollbackTimelineGeneration {
        self.generation
    }

    pub fn contract(&self) -> &RollbackTimelineContract {
        &self.contract
    }

    /// Record the content identity a live timeline has now observed.
    ///
    /// The narrow mutation on purpose: a contract's SCHEMA is fixed at install,
    /// and its content may only be filled in once, when a timeline installed
    /// before its world first sees one.
    pub fn adopt_content(&mut self, content: PreparedContentIdentity) {
        self.contract.content = Some(content);
    }

    /// Adopt the world a timeline installed before its session world first sees.
    ///
    /// The ownership sibling of [`Self::adopt_content`] and permitted for the
    /// same reason: a fixture may rebase frame zero onto a world it is about to
    /// build. An authority that already names an owner never re-adopts —
    /// that is a different session, and a different session gets its own.
    pub fn adopt_owner(&mut self, owner: SessionScopeId) {
        debug_assert!(self.owner.is_none(), "an owned authority never re-adopts");
        if self.owner.is_none() {
            self.owner = Some(owner);
        }
    }

    pub fn status(&self) -> &RollbackTimelineStatus {
        &self.status
    }

    /// Whether `scope` may read this authority at all.
    pub fn governs(&self, scope: Option<SessionScopeId>) -> bool {
        self.owner == scope
    }

    /// Confirmation state AS SEEN BY `scope`.
    ///
    /// ⛔⛔ A SCOPE THIS AUTHORITY DOES NOT GOVERN GETS `Unavailable`, NEVER
    /// `Unhealthy`. "Not mine" and "mine and broken" are different answers, and
    /// conflating them is the bug: a scope that reads a stranger's invalidation
    /// as its own refuses every confirmed transition forever.
    pub fn confirmation_for(&self, scope: Option<SessionScopeId>) -> RollbackConfirmationState {
        if !self.governs(scope) {
            return RollbackConfirmationState::Unavailable;
        }
        if !self.status.is_healthy() {
            return RollbackConfirmationState::Unhealthy;
        }
        match self.timeline {
            Timeline::Live => RollbackConfirmationState::Healthy,
            Timeline::StoodDown => RollbackConfirmationState::Unavailable,
        }
    }

    /// The timeline stopped; the gameplay session that owned it did not.
    pub fn stand_down_timeline(&mut self) {
        self.timeline = Timeline::StoodDown;
    }

    /// Record a divergence on this timeline.
    pub fn invalidate(&mut self, reason: String) {
        self.status.invalidation = Some(reason);
    }

    /// Record sync-test checksum mismatches on this timeline.
    pub fn record_mismatch(&mut self, frames: impl IntoIterator<Item = i32>) {
        self.status.mismatch_frames.extend(frames);
    }

    /// Clear an inherited diagnostic DELIBERATELY. See
    /// [`RollbackTimelineStatus::acknowledge_and_clear`].
    pub fn acknowledge_and_clear(&mut self) {
        self.status.acknowledge_and_clear();
    }
}

/// One remembered rollback failure. Process lifetime, ZERO gameplay authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackDiagnostic {
    /// The gameplay session whose timeline failed. `None` for an unscoped
    /// harness.
    pub scope: Option<SessionScopeId>,
    pub generation: RollbackTimelineGeneration,
    pub reason: String,
}

/// What went wrong on timelines this process has run.
///
/// ⭐ "Smash desynced because …" is worth keeping after Smash has ended, and it
/// is worth keeping precisely because it can no longer block anything. Health
/// that outlives its session is contamination; a RECORD that outlives its
/// session is a diagnostic. They were the same value until this split.
///
/// ⛔ Nothing may gate gameplay on this. If a check would read it to decide
/// whether work may proceed, it wants [`ActiveRollbackAuthority`] instead.
#[derive(Resource, Clone, Debug, Default)]
pub struct RollbackDiagnosticHistory {
    entries: Vec<RollbackDiagnostic>,
}

impl RollbackDiagnosticHistory {
    /// How many failures this process has seen, across every gameplay session.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[RollbackDiagnostic] {
        &self.entries
    }

    /// The most recent failure, whichever session it belonged to.
    pub fn last(&self) -> Option<&RollbackDiagnostic> {
        self.entries.last()
    }

    pub fn record(&mut self, diagnostic: RollbackDiagnostic) {
        self.entries.push(diagnostic);
    }
}

/// Confirmation authority for the ACTIVE gameplay session.
///
/// ⭐⭐ THE ONLY GAMEPLAY-SIDE READ. There is no way to reach the state without
/// naming a scope, and the only scope this parameter names is the live one — so
/// a consumer cannot accidentally read a retired session's health, and does not
/// have to remember to check.
///
/// A scope mismatch means "authority for the live session is not installed
/// yet", which during activation is simply true and resolves on its own. It
/// never means "use the previous session's".
#[derive(SystemParam)]
pub struct SessionRollbackConfirmation<'w, 's> {
    authority: Option<Res<'w, ActiveRollbackAuthority>>,
    scope: LiveSessionScope<'w, 's>,
}

impl SessionRollbackConfirmation<'_, '_> {
    pub fn state(&self) -> RollbackConfirmationState {
        let Some(authority) = self.authority.as_deref() else {
            return RollbackConfirmationState::Unavailable;
        };
        authority.confirmation_for(self.scope.get())
    }

    pub fn is_healthy(&self) -> bool {
        self.state().is_healthy()
    }

    /// Who the installed authority belongs to, and whose is being asked for.
    ///
    /// ⭐ FOR DIAGNOSTICS, and the one line that would have named this bug on
    /// sight: the refusal log printed only `Unhealthy`, which reads as "this
    /// session broke". `owner=Some(0) live=Some(1)` reads as what it was.
    pub fn ownership(&self) -> (Option<SessionScopeId>, Option<SessionScopeId>) {
        (
            self.authority
                .as_deref()
                .and_then(ActiveRollbackAuthority::owner),
            self.scope.get(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> RollbackTimelineContract {
        RollbackTimelineContract {
            content: None,
            schema: SnapshotSchemaFingerprint::from_bytes([7u8; 32]),
        }
    }

    const A: Option<SessionScopeId> = Some(SessionScopeId(1));
    const B: Option<SessionScopeId> = Some(SessionScopeId(2));

    /// ⭐⭐ SAME SCOPE, NEW TIMELINE: the diagnosis CARRIES (AC23).
    ///
    /// This test must fail if somebody later changes installation to return a
    /// default status — which is the shape every "just clear it when going
    /// home" fix takes.
    #[test]
    fn a_rebase_within_one_gameplay_session_inherits_its_invalidation() {
        let mut first = ActiveRollbackAuthority::installed(None, A, contract());
        first.invalidate("checksum diverged at frame 12".to_owned());

        let second = ActiveRollbackAuthority::installed(Some(&first), A, contract());

        assert_eq!(
            second.status().invalidation.as_deref(),
            Some("checksum diverged at frame 12"),
            "a new timeline of the SAME gameplay session inherits its diagnosis"
        );
        assert_eq!(
            second.confirmation_for(A),
            RollbackConfirmationState::Unhealthy
        );
        assert_ne!(
            second.generation(),
            first.generation(),
            "a rebase is a different timeline"
        );
    }

    /// ⭐⭐ DIFFERENT SCOPE: nothing carries.
    #[test]
    fn a_new_gameplay_session_inherits_nothing_from_the_previous_one() {
        let mut retired = ActiveRollbackAuthority::installed(None, A, contract());
        retired.invalidate("smash desynced".to_owned());

        let fresh = ActiveRollbackAuthority::installed(Some(&retired), B, contract());

        assert_eq!(fresh.status().invalidation, None);
        assert_eq!(
            fresh.confirmation_for(B),
            RollbackConfirmationState::Healthy
        );
    }

    /// ⛔⛔ A STALE AUTHORITY IS INERT, NOT UNHEALTHY.
    ///
    /// The distinction the bug turned on: session B reading A's `Unhealthy` as
    /// its own refuses every transition forever, while reading "not installed
    /// yet" resolves on the next frame.
    #[test]
    fn a_retired_scopes_authority_answers_unavailable_to_everyone_else() {
        let mut retired = ActiveRollbackAuthority::installed(None, A, contract());
        retired.invalidate("smash desynced".to_owned());

        assert_eq!(
            retired.confirmation_for(B),
            RollbackConfirmationState::Unavailable,
            "B is not this authority's scope, so this authority says nothing to B"
        );
        assert_eq!(
            retired.confirmation_for(None),
            RollbackConfirmationState::Unavailable,
            "and nothing to the title screen either"
        );
        assert_eq!(
            retired.confirmation_for(A),
            RollbackConfirmationState::Unhealthy,
            "while still refusing its OWN scope, which is the point of keeping it"
        );
    }

    /// A stopped timeline is unavailable; a stopped INVALIDATED one still refuses.
    #[test]
    fn standing_down_a_healthy_timeline_differs_from_standing_down_a_broken_one() {
        let mut healthy = ActiveRollbackAuthority::installed(None, A, contract());
        healthy.stand_down_timeline();
        assert_eq!(
            healthy.confirmation_for(A),
            RollbackConfirmationState::Unavailable
        );

        let mut broken = ActiveRollbackAuthority::installed(None, A, contract());
        broken.invalidate("desync".to_owned());
        broken.stand_down_timeline();
        assert_eq!(
            broken.confirmation_for(A),
            RollbackConfirmationState::Unhealthy,
            "a teardown may not launder a divergence"
        );
    }

    /// History remembers what authority forgets.
    #[test]
    fn diagnostics_outlive_the_session_they_describe() {
        let mut history = RollbackDiagnosticHistory::default();
        history.record(RollbackDiagnostic {
            scope: A,
            generation: RollbackTimelineGeneration(1),
            reason: "smash desynced".to_owned(),
        });
        assert_eq!(history.len(), 1);
        assert_eq!(history.last().map(|entry| entry.scope), Some(A));
    }
}

/// AC23 — a timeline's diagnosis survives its replacement WITHIN one gameplay
/// session. These moved here with [`RollbackTimelineStatus`]; the cross-session
/// half of the rule is in the module tests above.
#[cfg(test)]
mod ac23_tests {
    use super::*;

    /// A new session inherits an unhealthy timeline's reason. (AC23)
    #[test]
    fn an_invalidated_session_hands_its_reason_to_its_replacement() {
        let previous = RollbackTimelineStatus {
            mismatch_frames: Vec::new(),
            invalidation: Some("room reconstructed under a live timeline".to_string()),
        };
        let carried = RollbackTimelineStatus::carried_from(Some(&previous));
        assert!(
            !carried.is_healthy(),
            "an inherited invalidation was reported healthy"
        );
        assert_eq!(
            carried.invalidation.as_deref(),
            Some("room reconstructed under a live timeline"),
            "the replacement session came up clean, so the divergence was \
             laundered by the install"
        );
    }

    /// A checksum mismatch carries as PROSE, not as frame numbers.
    ///
    /// Frames restart at zero for every GGRS session, so carrying the numbers
    /// would report a mismatch at frames the new timeline has not reached.
    #[test]
    fn a_mismatch_carries_its_reason_but_not_its_frame_numbers() {
        let previous = RollbackTimelineStatus {
            mismatch_frames: vec![41, 42],
            invalidation: None,
        };
        let carried = RollbackTimelineStatus::carried_from(Some(&previous));
        assert!(
            carried.mismatch_frames.is_empty(),
            "frame numbers from a dead timeline were carried into a live one, so \
             the new session reports a mismatch at frames it has not reached"
        );
        let reason = carried
            .invalidation
            .expect("the mismatch survives as prose");
        assert!(
            reason.contains("41"),
            "the reason lost the evidence: {reason}"
        );
        assert!(
            reason.contains("PREVIOUS"),
            "the reason does not say the mismatch belongs to the old timeline: {reason}"
        );
    }

    /// A HEALTHY session installs clean, which is the ordinary case and must not
    /// acquire a phantom diagnostic.
    #[test]
    fn a_healthy_session_installs_clean() {
        let previous = RollbackTimelineStatus::default();
        assert!(
            previous.is_healthy(),
            "the default session status is not healthy"
        );
        assert_eq!(
            RollbackTimelineStatus::carried_from(Some(&previous)),
            RollbackTimelineStatus::default()
        );
        assert_eq!(
            RollbackTimelineStatus::carried_from(None),
            RollbackTimelineStatus::default()
        );
    }

    /// Clearing is possible, but only by SAYING SO.
    #[test]
    fn a_diagnostic_can_be_cleared_only_deliberately() {
        let mut status = RollbackTimelineStatus {
            mismatch_frames: vec![7],
            invalidation: Some("diverged".to_string()),
        };
        status.acknowledge_and_clear();
        assert_eq!(status, RollbackTimelineStatus::default());
    }
}
