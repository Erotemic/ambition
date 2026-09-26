//! Rules that one game states for the rooms it governs (ADR 0033).
//!
//! One binary can host several games: Ambition hosts Mary-O and Sanic in
//! rooms tagged with their modes. So a game's rule cannot be a global
//! resource. The type is the key, thus the second insert overwrites the first
//! and the rooms of one game run under the rules of the other. A
//! [`DeclaredRules`] keys each statement by the rooms it governs, and it
//! refuses a second statement for the same rooms when the app is built.
//!
//! The death rules and the dormancy rule use it. The next per-game rule is one
//! more `T`, not one more registry.

use bevy::prelude::*;

/// The rooms that one declaration governs. The variants mirror the
/// hosted-game mode scopes used for systems and entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulesScope {
    /// Rooms tagged with this game mode: a HOSTED game. The mirror of
    /// `in_mode(mode)`, which gates each system of that game.
    Mode(&'static str),
    /// Rooms with no mode tag: the rooms of the host. The mirror of
    /// `in_base_mode`.
    UntaggedRooms,
    /// Every room in the process: a STANDALONE game whose one ruleset IS the
    /// binary. The mirror of a `RulesPlugin::global()` that gates no system,
    /// and the reason a demo's own test harness need not tag its fixture rooms.
    EveryRoom,
}

/// Every game's statement of one kind of rule `T` in this binary, and the
/// rooms each statement governs.
///
/// Authored constants: each game states its rules once, when its plugin is
/// built, and no system writes them.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DeclaredRules<T> {
    /// [`Self::governing`] resolves by specificity, not by declaration order.
    /// This is a list rather than a map only to keep the scope key `Copy`.
    declarations: Vec<(RulesScope, T)>,
}

impl<T> Default for DeclaredRules<T> {
    fn default() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }
}

impl<T: Copy + std::fmt::Debug> DeclaredRules<T> {
    /// State the rules for one scope.
    ///
    /// Panics on a second declaration of the same scope. Two games that claim
    /// one set of rooms is not a precedence question with a correct answer; it
    /// is a composition mistake.
    pub fn declare(&mut self, scope: RulesScope, rules: T) {
        if let Some((_, existing)) = self
            .declarations
            .iter()
            .find(|(declared, _)| *declared == scope)
        {
            panic!(
                "{scope:?} already has {} ({existing:?}); a second \
                 declaration ({rules:?}) would mean two games govern the same \
                 rooms. Declare the narrower scope the second game actually owns.",
                std::any::type_name::<T>(),
            );
        }
        self.declarations.push((scope, rules));
    }

    /// THE ONE PLACE the question "whose rules govern here?" is answered.
    /// `mode` is the active room's mode tag.
    ///
    /// Most specific first: the room's own mode, then the untagged-room
    /// declaration for an untagged room, then a standalone game's
    /// whole-process claim. A room that no game claimed reads `None`: the rules
    /// of a different game are never the fallback.
    pub fn governing(&self, mode: Option<&str>) -> Option<T> {
        let find = |wanted: RulesScope| {
            self.declarations
                .iter()
                .find(|(scope, _)| *scope == wanted)
                .map(|(_, rules)| *rules)
        };
        let own = match mode {
            Some(mode) => self
                .declarations
                .iter()
                .find(|(scope, _)| matches!(scope, RulesScope::Mode(m) if *m == mode))
                .map(|(_, rules)| *rules),
            None => find(RulesScope::UntaggedRooms),
        };
        own.or_else(|| find(RulesScope::EveryRoom))
    }

    /// Every declaration, for diagnostics and for the composition guard that
    /// reads them all at once.
    pub fn iter(&self) -> impl Iterator<Item = (RulesScope, T)> + '_ {
        self.declarations.iter().copied()
    }
}

/// Declare a game's rules when the app is built.
pub trait DeclareRulesExt {
    /// See [`DeclaredRules::declare`].
    fn declare_rules<T>(&mut self, scope: RulesScope, rules: T) -> &mut Self
    where
        T: Copy + std::fmt::Debug + Send + Sync + 'static;
}

impl DeclareRulesExt for App {
    fn declare_rules<T>(&mut self, scope: RulesScope, rules: T) -> &mut Self
    where
        T: Copy + std::fmt::Debug + Send + Sync + 'static,
    {
        self.world_mut()
            .get_resource_or_insert_with(DeclaredRules::<T>::default)
            .declare(scope, rules);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room reads its own game's rules, and a room that no game claimed
    /// reads nothing, never the rules of a different game.
    ///
    /// The table is the shipped host's shape: one untagged-room claim and one
    /// mode claim, with a second mode (the Smash arena) that claims nothing.
    #[test]
    fn an_unclaimed_room_reads_no_rules_rather_than_another_games() {
        let mut declared = DeclaredRules::<u8>::default();
        declared.declare(RulesScope::UntaggedRooms, 1);
        declared.declare(RulesScope::Mode("mary_o"), 2);

        assert_eq!(declared.governing(Some("mary_o")), Some(2));
        assert_eq!(declared.governing(None), Some(1));
        assert_eq!(declared.governing(Some("smash")), None);
    }

    /// A standalone game's claim is the whole process, including its own
    /// mode-tagged rooms and any untagged fixture.
    ///
    /// This is what `<Demo>RulesPlugin::global()` means, and it is why a
    /// rules-only harness need not tag its rooms.
    #[test]
    fn a_standalone_games_rules_reach_every_room_it_loads() {
        let mut declared = DeclaredRules::<u8>::default();
        declared.declare(RulesScope::EveryRoom, 3);

        assert_eq!(declared.governing(None), Some(3));
        assert_eq!(declared.governing(Some("mary_o")), Some(3));
    }

    /// A mode's own claim outranks a whole-process one, so a composition that
    /// holds both still gives the narrower answer.
    #[test]
    fn the_narrower_claim_wins() {
        let mut declared = DeclaredRules::<u8>::default();
        declared.declare(RulesScope::EveryRoom, 9);
        declared.declare(RulesScope::Mode("mary_o"), 2);

        assert_eq!(declared.governing(Some("mary_o")), Some(2));
    }

    /// Two games that claim one set of rooms is a contradiction at build time,
    /// not a precedence question.
    #[test]
    #[should_panic(expected = "already has u8")]
    fn a_second_claim_on_the_same_rooms_is_refused() {
        let mut declared = DeclaredRules::<u8>::default();
        declared.declare(RulesScope::Mode("mary_o"), 2);
        declared.declare(RulesScope::Mode("mary_o"), 0);
    }

    /// Two kinds of rule are two keys: a game that states both does not
    /// collide with itself.
    #[test]
    fn each_kind_of_rule_is_its_own_declaration() {
        let mut app = App::new();
        app.declare_rules(RulesScope::Mode("mary_o"), 2u8);
        app.declare_rules(RulesScope::Mode("mary_o"), 7u16);

        let bytes = app.world().resource::<DeclaredRules<u8>>();
        let words = app.world().resource::<DeclaredRules<u16>>();
        assert_eq!(bytes.governing(Some("mary_o")), Some(2));
        assert_eq!(words.governing(Some("mary_o")), Some(7));
    }
}
