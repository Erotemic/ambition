//! The persistent input participant — the person in front of a controller.
//!
//! A participant exists before, during, and after any gameplay session: it is
//! the entity that owns device/action state (leafwing `ActionState` +
//! `InputMap`, attached by the host), the declared input contexts, and —
//! through its [`ParticipantId`] → `PlayerSlot` correspondence — the seat that
//! the body carrying `DrivingParticipant(slot)` reads. Possession, session
//! relaunch, and actor death
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
/// (`ambition_platformer2d::characters::brain`): the participant with id N feeds
/// `SlotControls[N]`, which the body carrying `DrivingParticipant(slot N)`
/// consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticipantId(pub u8);

impl ParticipantId {
    /// The first local seat.
    pub const PRIMARY: Self = Self(0);

    /// The second local seat — couch versus.
    pub const SECONDARY: Self = Self(1);

    // ⚠ these used to differ in KIND: primary "owned the global `ControlFrame`"
    // and secondary "wrote `SlotControls[1]` directly and never touched the
    // global frame". That asymmetry is what made half the input path
    // primary-only, and D175 removed it — every seat publishes through
    // `SlotControls` / `SeatRawFrames`, and the global frame is a device-edge
    // adapter for the local primary, not a routing table.

    /// The controller slot this seat drives. Participant ids and player slots
    /// are the same numbering on purpose: a seat IS a slot, and two maps that
    /// have to agree eventually disagree.
    pub const fn slot(self) -> u8 {
        self.0
    }
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

    pub const fn with_id(id: ParticipantId) -> Self {
        Self { id }
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

// ── in-session surfaces ─────────────────────────────────────────────────────
//
// These sit BETWEEN the shell and gameplay: they appear while a session is
// live, and a shell overlay still outranks them.
//
// an in-session surface is not the same fact as a stopped world. Pausing
// stops the world — global, `GameMode`, every seat. A surface owning a seat's
// input does not: one player reading a dialogue box while another keeps
// running is the ordinary state of a couch, and it is the thing this engine
// could not express before these ids existed.

/// A dev/tool overlay that has grabbed input. Above every in-session surface,
/// because a developer reaching for the inspector means it.
pub const DEBUG_CONTEXT: InputContextId = InputContextId("debug");
/// A scripted cutscene owns advance/skip.
pub const CUTSCENE_CONTEXT: InputContextId = InputContextId("cutscene");
/// An NPC conversation.
pub const DIALOGUE_CONTEXT: InputContextId = InputContextId("dialogue");
/// An inventory / equipment screen.
pub const INVENTORY_CONTEXT: InputContextId = InputContextId("inventory");
/// A character-select surface. Distinct from the launcher: a select screen is
/// reached FROM a launcher row and is a question, not a game.
pub const SELECT_CONTEXT: InputContextId = InputContextId("select");

/// The universal pause menu owns input while it is open.
///
/// Neither could consume the other's edge because they read different channels (`MenuControlFrame`
/// and `SeatMenuFrames`), and a demo cannot even NAME `ShellPauseMenu` (`basic_shell_presentation`
/// is not in `all_capabilities`, which is the oracle rule working as intended).
///
/// So the answer is not a feature edge from a demo to the shell — it is the
/// claim system that was already built for exactly this: the pause menu
/// DECLARES a capturing context, and any surface underneath asks whether it
/// still owns its seat. Neither side names the other.
pub const PAUSE_CONTEXT: InputContextId = InputContextId("pause");

/// Recommended claim priorities for the engine's own contexts. Higher wins.
/// Shell overlays outrank gameplay so a transient session/launcher overlap
/// (teardown, quit-to-title) resolves to the visible surface.
pub mod context_priority {
    pub const STARTUP_ACKNOWLEDGE: i32 = 300;
    pub const LAUNCHER: i32 = 200;
    /// Above the in-session surfaces: a developer opening a tool over a
    /// dialogue box wants the tool.
    pub const DEBUG: i32 = 195;
    /// Above dialogue: a cutscene that starts mid-conversation is the thing on
    /// screen.
    /// A pause menu opens OVER everything an experience is doing.
    ///
    /// Above cutscene, dialogue, select and gameplay — all four are things a
    /// player pauses out of — and below `DEBUG`, because an inspector that a
    /// pause could hide would be useless exactly when it is wanted.
    pub const PAUSE: i32 = 190;
    pub const CUTSCENE: i32 = 180;
    pub const DIALOGUE: i32 = 150;
    pub const INVENTORY: i32 = 140;
    pub const SELECT: i32 = 130;
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

/// The per-frame resolved answer to "which input context owns ONE
/// participant's actions". `owner` is the highest-priority claim; `open`
/// additionally lists non-capturing claims above it. Empty = disabled/no
/// target (no surface claims input; every routed output stays neutral).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

/// The empty answer, returned for a seat nobody has resolved. A seat with no
/// participant owns no context — which routes every output neutral — rather
/// than being an error, because "slot 3 has no pad" is the ordinary state of a
/// couch.
static NO_CONTEXT: ActiveInputContext = ActiveInputContext { open: Vec::new() };

/// Resolved input context for each participant seat.
///
/// Context answers where a seat's input routes; `GameMode` independently gates
/// whether gameplay runs. An unresolved seat owns no context.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatInputContexts {
    seats: std::collections::BTreeMap<u8, ActiveInputContext>,
}

impl SeatInputContexts {
    /// This seat's resolved answer. An unresolved seat owns nothing.
    pub fn for_seat(&self, slot: u8) -> &ActiveInputContext {
        self.seats.get(&slot).unwrap_or(&NO_CONTEXT)
    }

    /// The local primary seat: the device-edge `ControlFrame` adapter and the
    /// on-screen prompts. Consumers that are genuinely about the local primary
    /// — the touch overlay, the control-prompt HUD, the `ControlFrame` bridge —
    /// say so by calling this rather than by being the only reader of a global.
    pub fn primary(&self) -> &ActiveInputContext {
        self.for_seat(ParticipantId::PRIMARY.slot())
    }

    /// Whether this seat's actions may route to gameplay.
    pub fn gameplay_owned(&self, slot: u8) -> bool {
        self.for_seat(slot).gameplay_owned()
    }

    /// Every resolved seat, in slot order.
    pub fn seats(&self) -> impl Iterator<Item = (u8, &ActiveInputContext)> + '_ {
        self.seats.iter().map(|(slot, ctx)| (*slot, ctx))
    }

    pub fn set(&mut self, slot: u8, context: ActiveInputContext) {
        self.seats.insert(slot, context);
    }
}

/// Resolve EVERY participant's claims into [`SeatInputContexts`].
/// Runs after every declaring surface (end of `InputSet::ResolveContext`),
/// before any router reads the answer (`InputSet::Route`).
///
/// A seat whose participant has gone away is dropped rather than left holding
/// its last answer: a departed seat must not keep owning gameplay.
pub fn resolve_active_input_context(
    participants: Query<(&InputParticipant, &ParticipantContexts)>,
    mut active: ResMut<SeatInputContexts>,
) {
    let mut resolved = std::collections::BTreeMap::new();
    for (participant, contexts) in &participants {
        resolved.insert(
            participant.id.slot(),
            ActiveInputContext {
                open: contexts.resolved(),
            },
        );
    }
    if active.seats != resolved {
        active.seats = resolved;
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

    /// Two seats, two different answers, in one resolution pass.
    ///
    /// This is the whole point of keying the resolved context. The claims were
    /// always per-participant; the ANSWER was one global fold of seat 0, so a
    /// second seat could declare whatever it liked and no router could see it.
    fn resolve(app: &mut bevy::prelude::App) {
        use bevy::ecs::system::RunSystemOnce;
        app.world_mut()
            .run_system_once(resolve_active_input_context)
            .expect("resolver runs");
    }

    #[test]
    fn each_seat_resolves_its_own_context() {
        let mut app = bevy::prelude::App::new();
        app.init_resource::<SeatInputContexts>();

        let mut playing = ParticipantContexts::default();
        playing.declare(ContextClaim::capturing(
            GAMEPLAY_CONTEXT,
            context_priority::GAMEPLAY,
        ));
        let mut browsing = ParticipantContexts::default();
        browsing.declare(ContextClaim::capturing(
            LAUNCHER_CONTEXT,
            context_priority::LAUNCHER,
        ));

        app.world_mut()
            .spawn((InputParticipant::with_id(ParticipantId::PRIMARY), playing));
        let seat_one = app
            .world_mut()
            .spawn((
                InputParticipant::with_id(ParticipantId::SECONDARY),
                browsing,
            ))
            .id();
        resolve(&mut app);

        let seats = app.world().resource::<SeatInputContexts>();
        assert!(
            seats.gameplay_owned(0),
            "seat 0 is playing and must keep its gameplay routing"
        );
        assert!(
            !seats.gameplay_owned(1),
            "seat 1 is at the launcher — its own claim captures above gameplay"
        );
        assert_eq!(seats.for_seat(1).owner(), Some(LAUNCHER_CONTEXT));
        // A seat nobody has resolved owns nothing rather than inheriting seat 0.
        assert!(!seats.gameplay_owned(3) && seats.for_seat(3).owner().is_none());

        // A departed seat stops owning gameplay rather than holding its last
        // answer forever — otherwise unplugging a pad mid-match leaves a slot
        // that the router still believes is being driven.
        app.world_mut().despawn(seat_one);
        resolve(&mut app);
        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(seats.seats().count(), 1);
        assert!(seats.gameplay_owned(0), "seat 0 is unaffected by the exit");
    }
}
