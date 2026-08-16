//! **What a death MEANS for the run** (ADR 0033).
//!
//! The engine publishes the FACT that a participant died and then does nothing.
//! What happens next is stated here, once, by the game.
//!
//! # ⭐ Once by the game, not once by the BINARY
//!
//! Three games in the shipped host state death rules, and a bare
//! `Resource` made that a race the last `Plugin::build` won — Mary-O's
//! three-second level replay governed every Smash match, whose own doc says an
//! arena wants [`LevelReset::Never`]. So a declaration names the rooms it
//! governs ([`DeathRulesScope`]), they all live in one
//! [`DeclaredDeathRules`], and a room its game did not author reads
//! [`DeathRules::default`] rather than a stranger's answer.
//!
//! ⚠ **not [`DeathPolicy`](ambition_characters::actor::DeathPolicy)**, which
//! answers a different question — *does a full damage meter kill this body* —
//! and lives beside `BodyHealth` because it travels with the health component.
//! This answers *what happens after it does*.
//!
//! # Why the level reset is not a death consequence
//!
//! Jon, on co-op: *"Say we make maryo 2 player like NSMB, the level reset would
//! only happen if a player dies and all other players are also dead."*
//!
//! Hanging the reset off the individual death is player-centric, and in a
//! single-player game the mistake is INVISIBLE — with a roster of one, "this
//! participant died" and "no participant remains" are the same event. So the
//! reset asks a question about the ROSTER ([`LevelReset`]) and single player
//! falls out as the one-element case, by the same value co-op uses.
//!
//! # What is deliberately not here yet
//!
//! There is no per-participant FATE axis (bubble-revive, respawn at a
//! checkpoint, wait for a teammate). Nothing in the workspace has two
//! participants in one level yet, and the axis is unanswerable until something
//! does. Jon: *"we could build a new on death policy if we ever want something
//! more elaborate."* Grow this then; do not pre-shape it now.

use bevy::prelude::*;

/// **One game's death rules.** Stated once, beside its other rules.
///
/// ⛔ **not a resource, and that is deliberate.** Three games in this binary
/// each state their own, so a bare global would be answered by whichever plugin
/// was built last — which is exactly what happened: the shell composes Sanic
/// then Mary-O, so every Smash match in the shipped host ran under Mary-O's
/// three-second level-replay rules. A game declares these into
/// [`DeclaredDeathRules`] under the [`DeathRulesScope`] it governs, and the
/// rooms it did not author read [`DeathRules::default`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathRules {
    /// Seconds a participant's death holds before its consequence runs — the
    /// window content may fill with presentation.
    ///
    /// ⛔ **this does NOT freeze the world**, and must never grow into that. In
    /// NSMB the other player is still playing while this one's death animation
    /// runs, so an interlude that held the world would be wrong the moment a
    /// second participant existed. It is strictly THIS participant's window. A
    /// game that wants the world to hold still claims a time scale, which is a
    /// separate statement anything can make.
    pub interlude: f32,
    /// When the level goes back to how it started.
    pub level_reset: LevelReset,
}

impl Default for DeathRules {
    /// The conservative answer for a room whose game said nothing: hold for
    /// nothing, reset nothing.
    ///
    /// ⭐ **and it is the right answer for a versus arena**, which is why an
    /// unclaimed room is safe. A stage that has no level to put back wants
    /// [`LevelReset::Never`], and a match ruleset that owns its own respawn
    /// (Smash's stocks) wants no interlude in front of it.
    fn default() -> Self {
        Self {
            interlude: 0.0,
            level_reset: LevelReset::Never,
        }
    }
}

/// **Which rooms a game's death rules govern.**
///
/// ⭐ **the third noun of the demo-hosting seam.** That seam
/// (`ambition_platformer2d_runtime::mode_scope`) already scopes a hosted game's
/// SYSTEMS (`in_mode` / `in_base_mode`) and its ENTITIES (`ModeScopedEntity`) to
/// the rooms tagged with its mode. Its RULES had no such word, so each game
/// inserted a process-global instead and the last plugin built won the whole
/// binary. These variants are the same three answers `<Demo>RulesPlugin` already
/// gives when it decides how to gate its systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeathRulesScope {
    /// Rooms tagged with this game mode — a HOSTED game. The mirror of
    /// `in_mode(mode)`, which is how every one of that game's systems is gated.
    Mode(&'static str),
    /// Rooms carrying no mode tag — the host's own. The mirror of
    /// `in_base_mode`.
    UntaggedRooms,
    /// Every room in the process — a STANDALONE game whose one ruleset IS the
    /// binary. The mirror of a `RulesPlugin::global()` that gates no system, and
    /// the reason a demo's own test harness need not tag its fixture rooms.
    EveryRoom,
}

/// **Every game's death rules in this binary, and who governs where.**
///
/// ⚠ **the collection is the point.** One resource per game is the shape that
/// cannot be composed: the type is the key, so the second insert silently
/// overwrites the first and the loser's rooms run under the winner's rules.
/// Here a second declaration is a different KEY, and a second declaration of the
/// SAME key is a contradiction that panics at build time rather than picking a
/// winner.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct DeclaredDeathRules {
    /// Declaration order is not consulted — [`Self::governing`] resolves by
    /// specificity — so this is a list rather than a map purely to keep the
    /// scope key `Copy`.
    declarations: Vec<(DeathRulesScope, DeathRules)>,
}

impl DeclaredDeathRules {
    /// State the rules for one scope.
    ///
    /// ⛔ **panics on a second declaration of the same scope.** Two games
    /// claiming one set of rooms is not a precedence question with a defensible
    /// answer; it is a composition mistake, and the version of this that picked
    /// a winner is the defect this type exists to delete.
    pub fn declare(&mut self, scope: DeathRulesScope, rules: DeathRules) {
        if let Some((_, existing)) = self
            .declarations
            .iter()
            .find(|(declared, _)| *declared == scope)
        {
            panic!(
                "{scope:?} already has death rules ({existing:?}); a second \
                 declaration ({rules:?}) would mean two games govern the same \
                 rooms. Declare the narrower scope the second game actually owns.",
            );
        }
        self.declarations.push((scope, rules));
    }

    /// ⭐ **THE ONE PLACE the question "whose rules govern a death here?" is
    /// answered.** `mode` is the active room's mode tag.
    ///
    /// Most specific first: the room's own mode, then the untagged-room
    /// declaration for an untagged room, then a standalone game's whole-process
    /// claim. A room nobody claimed reads [`DeathRules::default`] — a foreign
    /// game's rules are never the fallback, which is the whole fix.
    pub fn governing(&self, mode: Option<&str>) -> DeathRules {
        let find = |wanted: DeathRulesScope| {
            self.declarations
                .iter()
                .find(|(scope, _)| *scope == wanted)
                .map(|(_, rules)| *rules)
        };
        let own = match mode {
            Some(mode) => self
                .declarations
                .iter()
                .find(|(scope, _)| matches!(scope, DeathRulesScope::Mode(m) if *m == mode))
                .map(|(_, rules)| *rules),
            None => find(DeathRulesScope::UntaggedRooms),
        };
        own.or_else(|| find(DeathRulesScope::EveryRoom))
            .unwrap_or_default()
    }

    /// Every declaration, for diagnostics and for the composition guard that
    /// reads them all at once.
    pub fn iter(&self) -> impl Iterator<Item = (DeathRulesScope, DeathRules)> + '_ {
        self.declarations.iter().copied()
    }
}

/// Declare a game's death rules at app-build time.
pub trait DeathRulesAppExt {
    /// See [`DeclaredDeathRules::declare`].
    fn declare_death_rules(&mut self, scope: DeathRulesScope, rules: DeathRules) -> &mut Self;
}

impl DeathRulesAppExt for App {
    fn declare_death_rules(&mut self, scope: DeathRulesScope, rules: DeathRules) -> &mut Self {
        self.world_mut()
            .get_resource_or_insert_with(DeclaredDeathRules::default)
            .declare(scope, rules);
        self
    }
}

impl DeathRules {
    /// The classic single-player answer, and the one most games want: hold for
    /// the death beat, then put the level back.
    ///
    /// Named as a constructor because "very very easy to opt in" is a design
    /// requirement (Jon), not a nicety — a game should not have to know the
    /// roster vocabulary to get the behaviour every platformer has.
    pub fn replay_level_after(interlude: f32) -> Self {
        Self {
            interlude,
            level_reset: LevelReset::WhenNoParticipantRemains,
        }
    }
}

/// **When does the LEVEL go back?** A question about the roster, never about one
/// death.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelReset {
    /// Never on a death. A versus round, an arena, a multiplayer level that
    /// outlives its participants — the ruleset acts on the death itself and the
    /// level is none of death's business.
    #[default]
    Never,
    /// When the participant who died was the last one standing.
    ///
    /// ⭐ **NSMB and single-player Mary-O are this same value.** Co-op resets
    /// when a player dies and every other player is already dead; a roster of one
    /// meets that condition on the first death.
    WhenNoParticipantRemains,
}

/// **This participant's attempt is over, and the world must not act on the
/// body.**
///
/// While it is held: no control frame, no hurtbox, nothing teleports the body,
/// nothing heals it, nothing resets its anim, and **the world's reset gates skip
/// it**.
///
/// ⛔ **that last clause is the whole defect this component exists to close.**
/// The blast-zone gate is a POSITION TEST that re-fires every tick a body is
/// past the margin. The ACTOR path has always known this — its gate is written
/// `em.health.alive() && …`, with a comment saying so — and the PLAYER path
/// never got the same guard. Measured 2026-08-09: one fall into a Mary-O pit
/// re-flagged the reset on 192 of the 192 frames of the death beat, and in the
/// hosted app each of those was a full room-feature reset while the death music
/// played.
///
/// ⭐ **and it makes "she dies where she died" free.** The pose pin, the anim
/// re-arm, the spent-life latch and the scripted control/immunity grants in
/// Mary-O's death beat all existed to claw the body back from a respawn that had
/// already happened. Nothing moves her now, so there is nothing to pin.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutOfPlay;

/// **The open window between a participant's death and its consequence.**
///
/// Rides the BODY rather than the level, because it is that participant's
/// window: in co-op one player's death beat plays while the other is still
/// running the level.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct DeathInterlude {
    /// Seconds left before the consequence runs. The window closes at zero.
    pub remaining: f32,
    /// This window still owes the game its consequence. Armed when the window
    /// opens, spent once when it closes.
    ///
    /// ⛔ **the debt is why the window is not simply REMOVED when it closes**,
    /// and dropping it cost a rollback desync inside an hour. "Has the
    /// consequence already run for this death" has to be answerable from
    /// SNAPSHOT STATE: the level reset it triggers is a message written late in
    /// one frame and consumed early in the next, so a rewind across that
    /// boundary re-simulates a frame whose cause has already been deleted — the
    /// request is lost and the two branches diverge.
    ///
    /// As a debt the window carries, it rides the same snapshot as the rest of
    /// the window, and spending it is idempotent because it is a STATE rather
    /// than an observation of the previous frame. (Mary-O's deleted beat carried
    /// exactly this field, for exactly this reason, and its doc said so. It was
    /// right; only its owner was wrong.)
    pub consequence_pending: bool,
}

impl DeathInterlude {
    /// Is the window still open? While it is, the death row plays and the
    /// consequence has not run.
    pub fn open(&self) -> bool {
        self.remaining > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A room reads its own game's rules, and an unclaimed room reads the
    /// engine default — never a stranger's.**
    ///
    /// The table is the shipped host's shape: one untagged-room claim and two
    /// mode claims, with a third mode (the Smash arena) that claims nothing.
    #[test]
    fn an_unclaimed_room_reads_the_default_rather_than_another_games_rules() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(
            DeathRulesScope::UntaggedRooms,
            DeathRules::replay_level_after(0.0),
        );
        declared.declare(
            DeathRulesScope::Mode("mary_o"),
            DeathRules::replay_level_after(3.2),
        );

        assert_eq!(declared.governing(Some("mary_o")).interlude, 3.2);
        assert_eq!(declared.governing(None).interlude, 0.0);
        assert_eq!(
            declared.governing(None).level_reset,
            LevelReset::WhenNoParticipantRemains,
        );
        // The arena. Both declarations above reset the level; it must not.
        assert_eq!(declared.governing(Some("smash")), DeathRules::default());
        assert_eq!(
            declared.governing(Some("smash")).level_reset,
            LevelReset::Never,
        );
    }

    /// **A standalone game's claim is the whole process, including its own
    /// mode-tagged rooms and any untagged fixture.**
    ///
    /// This is what `<Demo>RulesPlugin::global()` means, and it is why a
    /// rules-only harness need not tag its rooms.
    #[test]
    fn a_standalone_games_rules_reach_every_room_it_loads() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(DeathRulesScope::EveryRoom, DeathRules::replay_level_after(3.2));

        assert_eq!(declared.governing(None).interlude, 3.2);
        assert_eq!(declared.governing(Some("mary_o")).interlude, 3.2);
    }

    /// **A mode's own claim outranks a whole-process one**, so a composition
    /// that somehow held both still gives the narrower answer.
    #[test]
    fn the_narrower_claim_wins() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(DeathRulesScope::EveryRoom, DeathRules::replay_level_after(9.0));
        declared.declare(
            DeathRulesScope::Mode("mary_o"),
            DeathRules::replay_level_after(3.2),
        );

        assert_eq!(declared.governing(Some("mary_o")).interlude, 3.2);
    }

    /// ⛔ **two games claiming one set of rooms is a build-time contradiction,
    /// not a precedence question.**
    ///
    /// The version that picked a winner is the defect this type replaced: three
    /// `insert_resource(DeathRules)` calls, and whichever plugin the shell built
    /// last governed the binary. Silently choosing is what made that invisible.
    #[test]
    #[should_panic(expected = "already has death rules")]
    fn a_second_claim_on_the_same_rooms_is_refused() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(
            DeathRulesScope::Mode("mary_o"),
            DeathRules::replay_level_after(3.2),
        );
        declared.declare(
            DeathRulesScope::Mode("mary_o"),
            DeathRules::replay_level_after(0.0),
        );
    }
}
