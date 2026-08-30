//! Puppy-slug gun — a held item that summons player-allied puppy slugs.
//!
//! friendly puppy slugs that harm the player's enemies but never the player.
//! Decided: the slugs don't target — they just move (their normal
//! surface-walker wander); they're simply player-allied now.
//!
//! Implementation: the slug spawns through the existing runtime-minion path
//! (`spawn_runtime_minion`) with [`ActorFaction::Player`] + a passive aggression.
//! The `can_damage` matrix then does all the work — a Player-faction body damages
//! Enemy-faction actors and is damaged by them, but never harms the player. No
//! new faction or targeting code is needed (the "ally that hunts" behaviour is a
//! future `AggressionTarget` variant, per `components.rs`). Capped at
//! [`MAX_ALLIES`] alive; they persist until killed or the room resets.

use ambition_characters::control::ActorControl;
use bevy::prelude::*;

use crate::features::HeldItem;
use ambition_combat::components::{ActorAggression, ActorFaction};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopedEntity, SessionSpawnScope};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

/// Marks a summoned, player-allied puppy slug (so the cap can count them and a
/// future system can manage them).
#[derive(Component, Clone, Copy, Debug)]
pub struct PuppySlugAlly;

/// The held-item id the gun grants (see `brain::action_set` HELD_ITEMS).
pub const PUPPY_SLUG_GUN_ID: &str = "puppy_slug_gun";

/// Most player-allied puppy slugs alive at once.
pub const MAX_ALLIES: usize = 3;

/// The CHARACTER the gun summons.
///
/// An id that resolves nothing lands on the generic `combatant` fallback, so the ally a player
/// summoned with their own weapon was not a puppy slug at all: wrong health, wrong speed, no
/// crawl, no cling.
///
/// the summon road resolves the prepared cast first now, so naming the
/// character is all this takes.
const SLUG_ARCHETYPE: &str = "npc_puppy_slug";

/// `Attack` while holding the puppy-slug gun summons one player-allied puppy slug
/// ahead of the player, up to [`MAX_ALLIES`] alive. The gun's `HeldItemSpec` has
/// no melee/ranged verb, so this is the only thing `Attack` does while it's held.
pub fn fire_puppy_slug_gun_system(
    mut commands: Commands,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // the summoned ally IS a character (`npc_puppy_slug`), so this road needs
    // the cast to build it as one. `Option`: a composition that registers nobody
    // is ordinary, and the empty registry is the honest value there.
    prepared: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
    players: Query<(
        &ActorControl,
        &BodyKinematics,
        &HeldItem,
        Option<&SessionScopedEntity>,
    )>,
    allies: Query<(), With<PuppySlugAlly>>,
    // ⛔⛔ THE SUMMONER'S OWN ROLLBACKED COUNTER, NOT A `Local<u64>`. A `Local`
    // does not rewind, so an abandoned predicted branch that summoned a slug
    // advanced it and the resimulation of that same tick minted a DIFFERENT
    // identity for the same summon — which is the dynamic-spawn rule this repo
    // already states: a dynamically spawned body takes
    // `(spawner SimId, per-spawner counter)`, and both halves rewind.
    mut summoner_identity: Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((control, kin, held, owner)) = players.get(subject) else {
        return;
    };
    let c = control.0;
    // Plain Attack summons; Shield+Attack is reserved for throwing the gun away
    // (handled by `throw_held_item_system`), so don't also summon then.
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != PUPPY_SLUG_GUN_ID {
        return;
    }
    if allies.iter().count() >= MAX_ALLIES {
        return;
    }
    // ⛔ THE PAIR IS ONE VALUE. A summoner with no identity mints neither half,
    // so "dynamic, parent unknown" stays unspellable — the same rule the thrown
    // item mint follows.
    let minted = summoner_identity
        .get_mut(subject)
        .ok()
        .map(|(id, mut counter)| {
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(id, counter.next())
        });
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    let spawn_pos = kin.pos + ae::Vec2::new(facing * 40.0, -6.0);
    let session_scope = SessionSpawnScope::new(owner.map(|owner| owner.0));
    let empty_cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    let entity = crate::features::spawn_runtime_minion(
        &mut commands,
        &character_catalog,
        &authored_sheets,
        prepared.as_deref().unwrap_or(&empty_cast),
        session_scope,
        minted
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_else(|| "puppy_slug_ally".to_string()),
        // Must be the catalog `display_name` ("Puppy Slug"), NOT a decorated label
        // — the character-sprite table is keyed by display_name and silently falls
        // back to the goblin sheet on a miss, so "Puppy Slug (ally)" rendered a
        // goblin (with the puppy-slug shader, which keys off the archetype). The
        // ally-ness is carried by `ActorFaction::Player` + `PuppySlugAlly`, not the
        // name. See the sprite-keying refactor in TODO.md.
        "Puppy Slug",
        spawn_pos,
        ae::Vec2::new(14.0, 12.0),
        SLUG_ARCHETYPE,
        // Synthetic "encounter" so room reset cleans summons up alongside other
        // feature entities; no real boss owns them.
        "player_summon",
        // Player-allied + passive: damages the player's enemies via the faction
        // matrix, never the player, and just wanders (no targeting).
        ActorFaction::Player,
        ActorAggression::passive(),
    );
    commands.entity(entity).insert(PuppySlugAlly);
    // The GUN is the caster's, not the summoned slug's.
    sfx.write_for(
        subject,
        ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            pos: kin.pos,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;
    use ambition_combat::ActorFaction as Faction;

    pub(super) fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
        );
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        // Summoned bodies size themselves from their sheets (U1 stage B); a
        // fixture authors none, and empty resolves as it always did.
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        app.insert_resource(crate::character_runtime::fixture_cast(&[SLUG_ARCHETYPE]));
        app.add_systems(Update, fire_puppy_slug_gun_system);
        app
    }

    fn ally_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<PuppySlugAlly>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn attack_with_the_gun_summons_a_player_allied_slug() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        assert_eq!(ally_count(&mut app), 1, "one ally summoned");
        // The summoned slug is Player-faction, i.e. on the player's side: the
        // damage loop keys off `is_player_side`, so it harms enemies (the other
        // side) and is harmed by them, but never the player (same side).
        let mut q = app
            .world_mut()
            .query_filtered::<&Faction, With<PuppySlugAlly>>();
        let faction = *q.iter(app.world()).next().expect("ally exists");
        assert_eq!(faction, Faction::Player);
        assert!(faction.is_player_side(), "ally is on the player's side");
        assert!(
            !Faction::Enemy.is_player_side(),
            "enemies are the other side (so the ally damages them)"
        );
    }

    #[test]
    fn summon_is_capped() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        // Press attack many times (re-arming the edge each frame).
        for _ in 0..6 {
            app.world_mut()
                .get_mut::<ActorControl>(player)
                .unwrap()
                .0
                .melee_pressed = true;
            app.update();
        }
        assert_eq!(
            ally_count(&mut app),
            MAX_ALLIES,
            "capped at MAX_ALLIES alive"
        );
    }

    #[test]
    fn no_summon_without_the_gun_or_without_attack() {
        // Holding the gun but not attacking → no summon.
        let mut app = test_app();
        spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        app.update();
        assert_eq!(ally_count(&mut app), 0);
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;

    /// ⭐⭐ EACH SUMMON GETS ITS OWN IDENTITY, AND IT COMES FROM ROLLBACK STATE.
    ///
    /// ⛔⛔ THE COUNTER WAS A `Local<u64>`, WHICH DOES NOT REWIND. A predicted
    /// branch that summoned a slug advanced it, the branch was abandoned, and
    /// the resimulation of that same tick minted a DIFFERENT identity for the
    /// same summon — a body whose id disagrees between two peers replaying the
    /// same inputs. The summoner's own `SimIdCounter` is rollback state, so both
    /// halves of `(spawner SimId, sequence)` rewind together.
    ///
    /// ⚠ THE DISTINCTNESS IS THE OBSERVABLE HALF. A rewind is not reachable from
    /// this fixture, but "three summons, three ids" fails immediately if the
    /// counter stops advancing or the identity stops being read — which is what
    /// a wrong replacement looks like.
    #[test]
    fn three_summons_carry_three_distinct_identities() {
        let mut app = super::tests::test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        app.world_mut().entity_mut(player).insert((
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("summoner"),
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
        ));
        for _ in 0..MAX_ALLIES {
            app.world_mut()
                .get_mut::<ActorControl>(player)
                .unwrap()
                .0
                .melee_pressed = true;
            app.update();
        }

        let ids: Vec<String> = {
            let mut q = app
                .world_mut()
                .query_filtered::<&ambition_combat::components::FeatureId, With<PuppySlugAlly>>();
            q.iter(app.world())
                .map(|id| id.as_str().to_string())
                .collect()
        };
        assert_eq!(
            ids.len(),
            MAX_ALLIES,
            "the fixture summoned {} slug(s); this test is about their ids, so \
             it observes nothing if they are not there: {ids:?}",
            ids.len()
        );
        let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "two summons share an identity: {ids:?}"
        );
        assert!(
            ids.iter().all(|id| id.contains("summoner")),
            "each summon is named UNDER its summoner, which is what makes the \
             pair rewind together: {ids:?}"
        );
    }
}
