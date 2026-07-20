//! The persistent input participant — the person in front of a controller.
//!
//! A participant exists before, during, and after any gameplay session: it is
//! the entity that owns device/action state (leafwing `ActionState` +
//! `InputMap`, attached by the host), the declared input contexts, and —
//! through its [`ParticipantId`] → `PlayerSlot` correspondence — the seat that
//! `Brain::Player(slot)` reads. Possession, session relaunch, and actor death
//! never touch the participant; they only change which body interprets the
//! participant's deterministic `ControlFrame` downstream.
//!
//! Contexts are explicit ownership claims, not inferences: the surface that
//! owns a UI state (the shell sequence, the launcher, the session lifecycle)
//! declares a [`ContextClaim`] on the participant and retracts it when the
//! surface goes away. [`resolve_active_input_context`] reduces the claims to
//! one ordered answer per frame ([`ActiveInputContext`]) with priority +
//! capture semantics; nothing derives input ownership from `GameMode` or from
//! the presence of a controlled body.

use bevy::prelude::*;

/// Which seat at the machine. Maps 1:1 onto the sim-side `PlayerSlot`
/// (`ambition_characters::brain`): the participant with id N feeds
/// `SlotControls[N]`, which `Brain::Player(slot N)` consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticipantId(pub u8);

impl ParticipantId {
    /// The first (and, today, only) local seat.
    pub const PRIMARY: Self = Self(0);
}

/// The persistent participant entity marker. Spawned once by the host input
/// plugin at boot; never session-scoped, never despawned with a world.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputParticipant {
    pub id: ParticipantId,
}

impl InputParticipant {
    pub const fn primary() -> Self {
        Self {
            id: ParticipantId::PRIMARY,
        }
    }
}

/// An open, string-keyed context identity. Engine surfaces use the
/// well-known ids below; games and future surfaces (dialogue, vehicles,
/// dev overlays) mint their own without editing an engine enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputContextId(pub &'static str);

/// The startup acknowledgement surface (vanity/startup cards): one
/// semantic "continue" action, tap-anywhere friendly.
pub const STARTUP_ACKNOWLEDGE_CONTEXT: InputContextId = InputContextId("shell.startup_acknowledge");
/// The launcher / title menu.
pub const LAUNCHER_CONTEXT: InputContextId = InputContextId("shell.launcher");
/// A live gameplay session owns the participant's actions.
pub const GAMEPLAY_CONTEXT: InputContextId = InputContextId("gameplay");

/// Recommended claim priorities for the engine's own contexts. Higher wins.
/// Shell overlays outrank gameplay so a transient session/launcher overlap
/// (teardown, quit-to-title) resolves to the visible surface.
pub mod context_priority {
    pub const STARTUP_ACKNOWLEDGE: i32 = 300;
    pub const LAUNCHER: i32 = 200;
    pub const GAMEPLAY: i32 = 100;
}

/// One surface's claim over the participant's actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextClaim {
    pub id: InputContextId,
    /// Ordering among simultaneous claims; higher wins.
    pub priority: i32,
    /// A capturing claim blocks every lower-priority claim from receiving
    /// actions. A non-capturing claim (a future observing overlay) leaves
    /// the claims beneath it open.
    pub capture: bool,
}

impl ContextClaim {
    pub const fn capturing(id: InputContextId, priority: i32) -> Self {
        Self {
            id,
            priority,
            capture: true,
        }
    }
}

/// The participant's declared context claims. Surfaces `declare`/`retract`
/// (or `sync`) their own claim; nothing else writes here.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ParticipantContexts {
    claims: Vec<ContextClaim>,
}

impl ParticipantContexts {
    /// Upsert a claim by id. Idempotent for an unchanged claim.
    pub fn declare(&mut self, claim: ContextClaim) {
        match self.claims.iter_mut().find(|c| c.id == claim.id) {
            Some(existing) => *existing = claim,
            None => self.claims.push(claim),
        }
    }

    /// Remove a claim by id. Idempotent when absent.
    pub fn retract(&mut self, id: InputContextId) {
        self.claims.retain(|c| c.id != id);
    }

    /// Declare when `active`, retract when not. Returns whether the stored
    /// claims changed, so callers can avoid change-detection churn.
    pub fn sync(&mut self, claim: ContextClaim, active: bool) -> bool {
        let before = self.claims.clone();
        if active {
            self.declare(claim);
        } else {
            self.retract(claim.id);
        }
        before != self.claims
    }

    pub fn is_declared(&self, id: InputContextId) -> bool {
        self.claims.iter().any(|c| c.id == id)
    }

    /// Reduce the claims to the ordered open contexts: highest priority
    /// first, cut after the first capturing claim. Ties break by id so the
    /// answer is deterministic regardless of declaration order.
    pub fn resolved(&self) -> Vec<InputContextId> {
        let mut ordered = self.claims.clone();
        ordered.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.0.cmp(b.id.0)));
        let mut open = Vec::new();
        for claim in ordered {
            open.push(claim.id);
            if claim.capture {
                break;
            }
        }
        open
    }
}

/// The per-frame resolved answer to "which input context owns the primary
/// participant's actions". `owner` is the highest-priority claim; `open`
/// additionally lists non-capturing claims above it. Empty = disabled/no
/// target (no surface claims input; every routed output stays neutral).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActiveInputContext {
    open: Vec<InputContextId>,
}

impl ActiveInputContext {
    pub fn owner(&self) -> Option<InputContextId> {
        self.open.first().copied()
    }

    /// Whether actions may route to `id` this frame.
    pub fn allows(&self, id: InputContextId) -> bool {
        self.open.contains(&id)
    }

    /// Convenience for the most common gate.
    pub fn gameplay_owned(&self) -> bool {
        self.allows(GAMEPLAY_CONTEXT)
    }
}

/// Resolve the primary participant's claims into [`ActiveInputContext`].
/// Runs after every declaring surface (end of `InputSet::ResolveContext`),
/// before any router reads the answer (`InputSet::Route`).
pub fn resolve_active_input_context(
    participants: Query<(&InputParticipant, &ParticipantContexts)>,
    mut active: ResMut<ActiveInputContext>,
) {
    let open = participants
        .iter()
        .find(|(p, _)| p.id == ParticipantId::PRIMARY)
        .map(|(_, contexts)| contexts.resolved())
        .unwrap_or_default();
    if active.open != open {
        active.open = open;
    }
}

#[cfg(test)]
mod participant_tests {
    use super::*;

    const OVERLAY: InputContextId = InputContextId("test.overlay");

    #[test]
    fn claims_resolve_by_priority_and_cut_at_the_first_capture() {
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, 100));
        contexts.declare(ContextClaim::capturing(LAUNCHER_CONTEXT, 200));
        // The launcher captures: gameplay is closed while it is up.
        assert_eq!(contexts.resolved(), vec![LAUNCHER_CONTEXT]);

        // A non-capturing observer above the launcher leaves it open.
        contexts.declare(ContextClaim {
            id: OVERLAY,
            priority: 900,
            capture: false,
        });
        assert_eq!(contexts.resolved(), vec![OVERLAY, LAUNCHER_CONTEXT]);
    }

    #[test]
    fn retract_reopens_the_context_beneath() {
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, 100));
        contexts.declare(ContextClaim::capturing(LAUNCHER_CONTEXT, 200));
        contexts.retract(LAUNCHER_CONTEXT);
        assert_eq!(contexts.resolved(), vec![GAMEPLAY_CONTEXT]);
        // No claims at all = disabled: nothing owns input.
        contexts.retract(GAMEPLAY_CONTEXT);
        assert!(contexts.resolved().is_empty());
    }

    #[test]
    fn sync_reports_change_only_when_the_claims_actually_move() {
        let mut contexts = ParticipantContexts::default();
        let claim = ContextClaim::capturing(LAUNCHER_CONTEXT, 200);
        assert!(contexts.sync(claim, true), "first declare is a change");
        assert!(!contexts.sync(claim, true), "re-declaring unchanged is not");
        assert!(contexts.sync(claim, false), "retract is a change");
        assert!(!contexts.sync(claim, false), "re-retracting is not");
    }

    #[test]
    fn resolution_is_deterministic_under_priority_ties() {
        let a = InputContextId("test.a");
        let b = InputContextId("test.b");
        let mut declared_ab = ParticipantContexts::default();
        declared_ab.declare(ContextClaim::capturing(a, 100));
        declared_ab.declare(ContextClaim::capturing(b, 100));
        let mut declared_ba = ParticipantContexts::default();
        declared_ba.declare(ContextClaim::capturing(b, 100));
        declared_ba.declare(ContextClaim::capturing(a, 100));
        assert_eq!(declared_ab.resolved(), declared_ba.resolved());
    }

    #[test]
    fn the_resource_answers_owner_and_allows() {
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim {
            id: OVERLAY,
            priority: 900,
            capture: false,
        });
        contexts.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, 100));
        let active = ActiveInputContext {
            open: contexts.resolved(),
        };
        assert_eq!(active.owner(), Some(OVERLAY));
        assert!(active.allows(GAMEPLAY_CONTEXT) && active.gameplay_owned());
        assert!(!active.allows(LAUNCHER_CONTEXT));
    }
}
