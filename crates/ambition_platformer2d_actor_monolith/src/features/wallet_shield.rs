//! Where a wallet absorbs a hit.
//!
//! Two facts decide it. The character a body wears states `wallet_shield`
//! (Sanic's rings), and the rules that govern the room let a wallet absorb a
//! hit ([`WalletShieldRule`]). The marker the damage resolvers read,
//! [`BodyWalletShield`], is derived from both every tick, before any hit is
//! resolved. So a game says "rings absorb a hit in my rooms" with one
//! declaration, and the body's shield follows its character and its room.

use bevy::prelude::*;

use ambition_characters::actor::{BodyWalletShield, WornCharacter};
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, PlayerInputSet, SimScheduleExt,
};

use crate::session::governing_rules::GoverningRules;

/// A game's rule that a wallet absorbs a hit in the rooms it governs, for a
/// body whose character states `wallet_shield`. Declared with
/// [`DeclareRulesExt::declare_rules`](ambition_combat::scoped_rules::DeclareRulesExt::declare_rules).
/// In a room that no game gave this rule, no wallet absorbs a hit, because no
/// game there shows what the spent wallet becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalletShieldRule;

/// Put [`BodyWalletShield`] on exactly the bodies whose wallet absorbs a hit
/// here, and take it off every other body.
///
/// Derived state: the worn id and the active room are canonical, and the
/// prepared cast and the declared rules do not change, so this writes the same
/// answer again after a rewind.
pub fn project_wallet_shields(
    mut commands: Commands,
    rule: GoverningRules<WalletShieldRule>,
    cast: Option<Res<PreparedCharacterRegistry>>,
    bodies: Query<(Entity, Option<&WornCharacter>, Has<BodyWalletShield>)>,
) {
    let governs = rule.get().is_some();
    for (entity, worn, shielded) in &bodies {
        let shields = governs
            && worn
                .and_then(|worn| cast.as_deref()?.get(worn.id()))
                .is_some_and(|character| character.wallet_shield);
        match (shields, shielded) {
            (true, false) => {
                commands.entity(entity).try_insert(BodyWalletShield);
            }
            (false, true) => {
                commands.entity(entity).try_remove::<BodyWalletShield>();
            }
            _ => {}
        }
    }
}

/// Schedules [`project_wallet_shields`] after the persona phase, which can
/// change what a body wears, and before any hit is resolved.
pub struct WalletShieldPlugin;

impl Plugin for WalletShieldPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            project_wallet_shields
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .after(PlayerInputSet::Persona)
                .before(ambition_damage::PlayerHitResolutionSet),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::scoped_rules::{DeclareRulesExt, RulesScope};

    /// A body that wears `shielded` (its row states `wallet_shield`) or
    /// `plain` (it does not), in an app whose only room is tagged `mode`.
    fn app(mode: &str) -> App {
        let mut app = App::new();
        let mut cast = PreparedCharacterRegistry::default();
        for (id, wallet_shield) in [("shielded", true), ("plain", false)] {
            let mut definition =
                ambition_characters::actor::definition::CharacterDefinition::new(id, id, "demo");
            definition.wallet_shield = wallet_shield;
            cast.insert_prepared(
                crate::character_runtime::prepare_and_finalize_for_test(
                    definition,
                    &ambition_characters::prepared::CharacterBindings::default(),
                )
                .prepared,
            );
        }
        app.insert_resource(cast);
        enter_room_of(&mut app, mode);
        app.declare_rules(RulesScope::Mode("rings"), WalletShieldRule);
        app.add_systems(Update, project_wallet_shields);
        app
    }

    /// Make the session's one room a room of `mode`.
    fn enter_room_of(app: &mut App, mode: &str) {
        let world = ambition_platformer2d_core::World::new(
            "room",
            ambition_platformer2d_core::Vec2::new(320.0, 240.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        );
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new("room", world);
        room.metadata.mode = Some(mode.to_owned());
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
                "room",
                vec![room],
                Vec::new(),
            ),
        );
    }

    fn shielded_after_a_tick(app: &mut App, wears: &str) -> bool {
        let body = app.world_mut().spawn(WornCharacter::new(wears)).id();
        app.update();
        app.world().get::<BodyWalletShield>(body).is_some()
    }

    /// The character states the shield and the room's rules let it absorb:
    /// both, or no shield.
    #[test]
    fn a_wallet_absorbs_only_for_a_shielded_character_where_the_rule_governs() {
        assert!(
            shielded_after_a_tick(&mut app("rings"), "shielded"),
            "a shielded character in a room the rule governs"
        );
        assert!(
            !shielded_after_a_tick(&mut app("rings"), "plain"),
            "a character that does not state the shield"
        );
        assert!(
            !shielded_after_a_tick(&mut app("other_game"), "shielded"),
            "a room that no game gave the rule"
        );
    }

    /// Leaving the rule's rooms takes the shield away, although the body
    /// still wears the same character.
    #[test]
    fn the_shield_leaves_with_the_rule() {
        let mut app = app("rings");
        let body = app.world_mut().spawn(WornCharacter::new("shielded")).id();
        app.update();
        assert!(app.world().get::<BodyWalletShield>(body).is_some(), "premise");
        enter_room_of(&mut app, "other_game");
        app.update();
        assert!(
            app.world().get::<BodyWalletShield>(body).is_none(),
            "the body kept its shield where no rule lets a wallet absorb a hit"
        );
    }
}
