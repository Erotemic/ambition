//! Game-scoped consequences of participant death (ADR 0033).
//!
//! The engine publishes the death fact; the owning game declares what follows.
//! [`DeathRulesScope`] selects which rooms a declaration governs, while
//! [`DeathPolicy`](ambition_characters::actor::DeathPolicy) separately controls
//! whether a body's full damage meter kills it. Level reset is roster-level
//! policy through [`LevelReset`], not a consequence attached to one participant.

use bevy::prelude::*;

/// One game's death rules, declared in [`DeclaredDeathRules`] under the
/// [`DeathRulesScope`] it governs. Unclaimed rooms use [`DeathRules::default`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathRules {
    /// Seconds a participant's death holds before its consequence runs — the
    /// window content may fill with presentation.
    ///
    /// this does NOT freeze the world, and must never grow into that. In
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
    /// and it is the right answer for a versus arena, which is why an
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

/// Rooms governed by one game's death rules. The variants mirror the existing
/// hosted-game mode scopes used for systems and entities.
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

/// Every game's death rules in this binary, and who governs where.
///
/// the collection is the point. One resource per game is the shape that
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
    /// panics on a second declaration of the same scope. Two games
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

    /// THE ONE PLACE the question "whose rules govern a death here?" is
    /// answered. `mode` is the active room's mode tag.
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
    /// requirement, not a nicety — a game should not have to know the
    /// roster vocabulary to get the behaviour every platformer has.
    pub fn replay_level_after(interlude: f32) -> Self {
        Self {
            interlude,
            level_reset: LevelReset::WhenNoParticipantRemains,
        }
    }
}

/// When does the LEVEL go back? A question about the roster, never about one
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
    /// NSMB and single-player Mary-O are this same value. Co-op resets
    /// when a player dies and every other player is already dead; a roster of one
    /// meets that condition on the first death.
    WhenNoParticipantRemains,
}

/// This participant's attempt is over, and the world must not act on the
/// body.
///
/// While it is held: no control frame, no hurtbox, nothing teleports the body,
/// nothing heals it, nothing resets its anim, and the world's reset gates skip
/// it.
///
/// The ACTOR path has always known this — its gate is written `em.health.alive() && …`, with a
/// comment saying so — and the PLAYER path never got the same guard.
///
/// and it makes "she dies where she died" free. The pose pin, the anim
/// re-arm, the spent-life latch and the scripted control/immunity grants in
/// Mary-O's death beat all existed to claw the body back from a respawn that had
/// already happened. Nothing moves her now, so there is nothing to pin.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutOfPlay;

/// The open window between a participant's death and its consequence.
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
    /// As a debt the window carries, it rides the same snapshot as the rest of the window, and
    /// spending it is idempotent because it is a STATE rather than an observation of the
    /// previous frame.
    pub consequence_pending: bool,
}

impl DeathInterlude {
    /// Is the window still open? While it is, the death row plays and the
    /// consequence has not run.
    pub fn open(&self) -> bool {
        self.remaining > 0.0
    }
}

/// Count the open windows down on the SIM clock, and play the death row for as
/// long as one is open.
///
/// ⭐ IT LIVES WITH THE COMPONENT IT TICKS. It sat in the monolith's session
/// module while `DeathInterlude` was declared here, which meant the crate that
/// OPENS a window could not advance one — so the stocks beat that needed the
/// same countdown built a second one rather than reach up a layer for this.
/// Nothing in it needs the monolith: a clock, the window, and the anim fact.
///
/// arming the death animation is the ENGINE's job now. `death_anim_timer`
/// had exactly one writer in the workspace — Mary-O's beat — which re-armed it
/// EVERY TICK because the engine's respawn called `BodyAnimFacts::reset()` on
/// the very frame she died. Nothing resets it out from under the interlude any
/// more, and every game's death row plays without each one discovering the timer
/// for itself. The anim view already reads it (`v.dead = death_anim_timer > 0.0`),
/// so "dead" and "out of play with a window open" are now the same fact.
pub fn tick_death_interlude(
    time: Res<ambition_time::WorldTime>,
    mut windows: Query<(
        &mut DeathInterlude,
        Option<&mut ambition_characters::actor::BodyAnimFacts>,
    )>,
) {
    let dt = time.sim_dt();
    for (mut window, anim) in &mut windows {
        if window.remaining > 0.0 {
            window.remaining = (window.remaining - dt).max(0.0);
        }
        // only while the window is OPEN. The component OUTLIVES the window
        // (it carries the consequence debt until the body restarts), so arming
        // unconditionally would hold every corpse in its death row forever.
        if let Some(mut anim) = anim {
            if window.open() {
                // At least one frame's worth, so the row is visible on the frame
                // the window closes rather than blinking out one tick early —
                // the same `.max(dt)` Mary-O's beat carried.
                anim.death_anim_timer = window.remaining.max(dt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room reads its own game's rules, and an unclaimed room reads the
    /// engine default — never a stranger's.
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

    /// A standalone game's claim is the whole process, including its own
    /// mode-tagged rooms and any untagged fixture.
    ///
    /// This is what `<Demo>RulesPlugin::global()` means, and it is why a
    /// rules-only harness need not tag its rooms.
    #[test]
    fn a_standalone_games_rules_reach_every_room_it_loads() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(
            DeathRulesScope::EveryRoom,
            DeathRules::replay_level_after(3.2),
        );

        assert_eq!(declared.governing(None).interlude, 3.2);
        assert_eq!(declared.governing(Some("mary_o")).interlude, 3.2);
    }

    /// A mode's own claim outranks a whole-process one, so a composition
    /// that somehow held both still gives the narrower answer.
    #[test]
    fn the_narrower_claim_wins() {
        let mut declared = DeclaredDeathRules::default();
        declared.declare(
            DeathRulesScope::EveryRoom,
            DeathRules::replay_level_after(9.0),
        );
        declared.declare(
            DeathRulesScope::Mode("mary_o"),
            DeathRules::replay_level_after(3.2),
        );

        assert_eq!(declared.governing(Some("mary_o")).interlude, 3.2);
    }

    /// two games claiming one set of rooms is a build-time contradiction,
    /// not a precedence question.
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
