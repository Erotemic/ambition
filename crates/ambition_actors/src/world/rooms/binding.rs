//! What a room REFERENCES, resolved at construction time.
//!
//! A `RoomSpec` is full of ids that point at something else: a patrol brain
//! names a kinematic path, a ground item names a held-item spec, an enemy spawn
//! names a character archetype. Each of those was resolved by its own consumer,
//! at its own moment, with its own way of shrugging — the patrol brain falls
//! back to passive, the ground item is "skipped at spawn rather than erroring"
//! (its own doc comment says so), the archetype falls back to a default. A room
//! could therefore be authored entirely of typos and still construct, quietly,
//! into something that merely looked under-populated.
//!
//! This module resolves all of them in one pass, before construction mutates
//! anything, and returns ONE
//! [`BindingReport`](ambition_platformer_primitives::binding::BindingReport).
//!
//! # Why it lives here and not in a game
//!
//! `game/ambition_content` has had a cross-content validator for a while, and
//! its opening line states this module's thesis exactly: catch content typos
//! "instead of letting string ids silently fall back or never fire". But it
//! reads raw LDtk JSON, so it only ever served the one game with an `.ldtk`
//! file. Mary-O builds `RoomSpec`s in Rust, Sanic builds its course, Outlander
//! authors a ridge from outside the workspace — and none of them got any of it.
//!
//! Resolving the world IR instead of the authoring file is what makes this an
//! ENGINE capability: every provider gets it, whatever backend produced the
//! room (ADR 0021), including backends that do not exist yet.

use ambition_platformer_primitives::binding::{
    BindingLedger, BindingReport, Namespace, Ref, Resolver,
};

use crate::rooms::RoomSpec;

/// The kinematic paths a room declares. Room-scoped: `patrol_a` in one room has
/// nothing to do with `patrol_a` in another.
pub struct KinematicPathId;

impl Namespace for KinematicPathId {
    const NAME: &'static str = "kinematic path";
}

/// The character archetypes a catalog knows.
pub struct CharacterId;

impl Namespace for CharacterId {
    const NAME: &'static str = "character";
}

/// The held-item specs the item registry knows.
pub struct HeldItemId;

impl Namespace for HeldItemId {
    const NAME: &'static str = "held item";
}

/// The resolvers a room sweep has on hand.
///
/// Paths come from the room itself, so they are always checked. The catalog-backed
/// namespaces are optional because construction is legitimately performed in
/// contexts that have no catalog (a geometry-only fixture, an early boot). An
/// absent resolver means NOT CHECKED, and [`RoomBindings::checked`] says which
/// namespaces were — "we did not look" must not read like "we looked and it was
/// fine", which is the whole failure this boundary exists to prevent.
#[derive(Default)]
pub struct RoomBindings {
    characters: Option<Resolver<CharacterId>>,
    held_items: Option<Resolver<HeldItemId>>,
}

impl RoomBindings {
    /// Check character-archetype references against `ids`.
    pub fn with_characters<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.characters = Some(Resolver::new(ids));
        self
    }

    /// Check held-item references against `ids`.
    pub fn with_held_items<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.held_items = Some(Resolver::new(ids));
        self
    }

    /// Which namespaces this sweep can actually decide, in report order. A caller
    /// that wants full coverage asserts on this rather than on an empty report.
    pub fn checked(&self) -> Vec<&'static str> {
        let mut names = vec![KinematicPathId::NAME];
        if self.characters.is_some() {
            names.push(CharacterId::NAME);
        }
        if self.held_items.is_some() {
            names.push(HeldItemId::NAME);
        }
        names.sort_unstable();
        names
    }

    /// Resolve every reference `room` declares that this sweep can decide.
    ///
    /// One pass, one report: a room with a bad patrol path AND a bad pickup id
    /// names both, so fixing content is one edit session rather than a sequence
    /// of run-crash-fix cycles.
    pub fn sweep(&self, room: &RoomSpec) -> BindingReport {
        let mut ledger = BindingLedger::new();
        let paths = room_paths(room);

        for enemy in &room.enemy_spawns {
            match &enemy.payload {
                ambition_entity_catalog::placements::CharacterBrain::Patrol {
                    path_id: Some(path_id),
                } => {
                    ledger.resolve(
                        &paths,
                        &Ref::new(path_id),
                        format!("patrol brain of `{}`", enemy.id),
                    );
                }
                ambition_entity_catalog::placements::CharacterBrain::Custom(archetype) => {
                    if let Some(characters) = &self.characters {
                        ledger.resolve(
                            characters,
                            &Ref::new(archetype),
                            format!("enemy spawn `{}`", enemy.id),
                        );
                    }
                }
                _ => {}
            }
        }

        if let Some(held_items) = &self.held_items {
            for item in &room.ground_items {
                ledger.resolve(
                    held_items,
                    &Ref::new(&item.held_item),
                    format!("ground item `{}`", item.id),
                );
            }
        }

        ledger.finish()
    }

    /// Resolve character-archetype references a provider declares OUTSIDE the
    /// room spec — through content staging, a summon, or a scripted spawn.
    ///
    /// Mary-O and Sanic stage their enemies as spawn requests rather than authored
    /// `enemy_spawns`, so [`Self::sweep`] never sees those brain keys. They are the
    /// ones that need checking most: `CharacterRoster::spec_for_brain` cannot
    /// fail, so a misspelled key becomes the generic `combatant` fallback and the
    /// demo quietly ships the wrong enemy with the right name.
    ///
    /// Each item is `(archetype id, who declared it)`.
    pub fn sweep_characters<I, S, D>(&self, refs: I) -> BindingReport
    where
        I: IntoIterator<Item = (S, D)>,
        S: AsRef<str>,
        D: Into<String>,
    {
        let mut ledger = BindingLedger::new();
        if let Some(characters) = &self.characters {
            for (archetype, declared_by) in refs {
                ledger.resolve(characters, &Ref::new(archetype.as_ref()), declared_by);
            }
        }
        ledger.finish()
    }
}

/// The room's paths, under every spelling each answers to.
///
/// `KinematicPathSpec::matches_id` accepts the authored id and the display name,
/// so the resolver must too — otherwise this sweep would report references that
/// the runtime resolves perfectly well, which is worse than not checking.
fn room_paths(room: &RoomSpec) -> Resolver<KinematicPathId> {
    Resolver::with_aliases(
        room.kinematic_paths
            .iter()
            .enumerate()
            .flat_map(|(slot, path)| path.aliases().map(move |alias| (alias, slot))),
    )
}
