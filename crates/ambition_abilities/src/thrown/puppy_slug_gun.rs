//! Puppy-slug gun — a held item that summons player-allied puppy slugs.
//!
//! Friendly puppy slugs that harm the player's enemies but never the player.
//! Decided: the slugs don't target — they just move (their normal
//! surface-walker wander); they're simply player-allied now.
//!
//! The gun asks for each slug through the summon executor's one road
//! (`EffectRequest` → `Effect::Summon`), as a boss asks for a minion. It states
//! what it wants: a companion ([`SummonCalling::Companion`]) on the player's
//! side ([`HitSide::Player`]). The executor builds the body, gives it its
//! identity under the summoner, and marks it [`SummonedCompanion`]. The
//! `can_damage` matrix then does all the work — a Player-faction body damages
//! Enemy-faction actors and is damaged by them, but never harms the player.
//! Capped at [`MAX_ALLIES`] alive; they persist until killed or the room resets.

use ambition_characters::control::ActorControl;
use ambition_combat::components::SummonedCompanion;
use ambition_combat::held_items::HeldItem;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_vfx::{Effect, EffectRequest, HitSide, SummonCalling, SummonSpec};
use bevy::prelude::*;

/// The held-item id the gun grants (see `brain::action_set` HELD_ITEMS).
pub const PUPPY_SLUG_GUN_ID: &str = "puppy_slug_gun";

/// Most companions alive at once. The gun is the only thing that summons
/// companions, so this is the most player-allied puppy slugs alive at once.
pub const MAX_ALLIES: usize = 3;

/// The CHARACTER the gun summons.
///
/// An id that resolves nothing is refused by the executor's preflight, so the
/// summon spends nothing and builds nothing.
const SLUG_ARCHETYPE: &str = "npc_puppy_slug";

/// The kind a summoned slug's name starts with; the executor names each body
/// under the identity it derives for it.
const SLUG_KIND: &str = "puppy_slug_ally";

/// `Attack` while holding the puppy-slug gun summons one player-allied puppy slug
/// ahead of the player, up to [`MAX_ALLIES`] alive. The gun's `HeldItemSpec` has
/// no melee/ranged verb, so this is the only thing `Attack` does while it's held.
pub fn fire_puppy_slug_gun_system(
    // ⭐ EVERY DRIVEN BODY, not the one the primary seat happens to hold.
    // `ControlledSubject` is singular by construction, so a possessed body or a
    // second seat holding the same item simply never fired.
    driven: ambition_held_items::DrivenBodies,
    players: Query<(&ActorControl, &BodyKinematics, &HeldItem)>,
    companions: Query<(), With<SummonedCompanion>>,
    mut effects: MessageWriter<EffectRequest>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    // ⛔⛔ A QUERY CANNOT SEE A SUMMON THAT IS NOT BUILT YET, and every driven
    // body reads the cap on the same tick. Without this running tally, N seats
    // pressing Attack together each saw the same pre-tick count and every one of
    // them summoned — the cap exceeded by exactly the number of extra seats.
    // The executor builds this tick's requests later in the same tick (it runs
    // in `Combat`, after `PlayerSimulation`), so the next tick's query sees them.
    let mut summoned_this_tick = 0usize;
    for subject in driven.entities() {
        let Ok((control, kin, held)) = players.get(subject) else {
            continue;
        };
        let c = control.0;
        // Plain Attack summons; Shield+Attack is reserved for throwing the gun away
        // (handled by `throw_held_item_system`), so don't also summon then.
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != PUPPY_SLUG_GUN_ID {
            continue;
        }
        if companions.iter().count() + summoned_this_tick >= MAX_ALLIES {
            continue;
        }
        let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
        effects.write(EffectRequest {
            owner: subject,
            effect: Effect::Summon(SummonSpec {
                id: SLUG_KIND.into(),
                pos: kin.pos + ae::Vec2::new(facing * 40.0, -6.0),
                half_size: ae::Vec2::new(14.0, 12.0),
                character_id: SLUG_ARCHETYPE.into(),
                // Synthetic "encounter" so room reset cleans summons up alongside
                // other feature entities; no real boss owns them.
                encounter_id: "player_summon".into(),
                // On the player's side: damages the player's enemies via the
                // faction matrix, never the player, and just wanders.
                faction: HitSide::Player,
                calling: SummonCalling::Companion,
                ridden_by_summoner: None,
                health: None,
                keeps_contact_damage: true,
                holds: Vec::new(),
            }),
        });
        summoned_this_tick += 1;
        // The GUN is the caster's, not the summoned slug's.
        sfx.write_for(
            subject,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
                pos: kin.pos,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{spawn_primary_player_holding, spawn_seated_body_holding};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<EffectRequest>();
        app.add_systems(Update, fire_puppy_slug_gun_system);
        app
    }

    /// The summons the gun asked for on the last update: (summoner, spec).
    fn summons(app: &App) -> Vec<(Entity, ae::Vec2, HitSide, SummonCalling, String)> {
        let messages = app.world().resource::<Messages<EffectRequest>>();
        messages
            .iter_current_update_messages()
            .filter_map(|req| match &req.effect {
                Effect::Summon(s) => {
                    Some((req.owner, s.pos, s.faction, s.calling, s.character_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn press_attack(app: &mut App, body: Entity) {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }

    #[test]
    fn attack_with_the_gun_asks_for_a_player_side_companion_slug() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        press_attack(&mut app, player);
        app.update();
        let asked = summons(&app);
        assert_eq!(asked.len(), 1, "one slug asked for: {asked:?}");
        let (owner, _, side, calling, character) = &asked[0];
        assert_eq!(*owner, player, "the summoner is the body that fired");
        // The player's side is what makes the slug harm the player's enemies and
        // never the player; a companion is what makes it wander, not hunt.
        assert_eq!(*side, HitSide::Player);
        assert_eq!(*calling, SummonCalling::Companion);
        assert_eq!(character, SLUG_ARCHETYPE);
    }

    #[test]
    fn no_summon_while_the_cap_is_full() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        for _ in 0..MAX_ALLIES {
            app.world_mut().spawn(SummonedCompanion);
        }
        press_attack(&mut app, player);
        app.update();
        assert!(summons(&app).is_empty(), "the cap is full");
    }

    #[test]
    fn no_summon_without_attack_or_while_shielding() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, PUPPY_SLUG_GUN_ID);
        app.update();
        assert!(summons(&app).is_empty(), "holding is not firing");
        {
            let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
            control.0.melee_pressed = true;
            control.0.shield_held = true;
        }
        app.update();
        assert!(summons(&app).is_empty(), "Shield+Attack throws the gun instead");
    }

    /// ⭐⭐ A SECOND DRIVEN BODY SUMMONS ITS OWN SLUG.
    /// Same singular-`ControlledSubject` defect as the volley.
    #[test]
    fn two_driven_bodies_each_summon_their_own_slug() {
        let mut app = test_app();
        app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
        let a = spawn_seated_body_holding(&mut app, PUPPY_SLUG_GUN_ID, 0, "seat_a", ae::Vec2::new(100.0, 100.0));
        let b = spawn_seated_body_holding(&mut app, PUPPY_SLUG_GUN_ID, 1, "seat_b", ae::Vec2::new(900.0, 100.0));
        for body in [a, b] {
            press_attack(&mut app, body);
        }
        app.update();
        let asked = summons(&app);
        assert_eq!(asked.len(), 2, "one slug per summoning seat: {asked:?}");
        // Each slug is asked for beside ITS OWN summoner, so the positions are
        // what says two bodies cast rather than one body casting twice.
        for (owner, pos, ..) in &asked {
            let near_a = pos.x < 500.0;
            assert_eq!(*owner == a, near_a, "each slug spawns beside its own summoner: {asked:?}");
        }
    }

    /// ⛔⛔ THE CAP IS COUNTED FROM A QUERY, AND A QUERY CANNOT SEE THIS TICK'S
    /// SUMMONS. Looping over driven bodies made that reachable: with
    /// `MAX_ALLIES` seats pressing Attack on one tick, every one of them read the
    /// same pre-tick count of zero and summoned, so the cap was exceeded by the
    /// number of extra seats. The system has to count what IT has already
    /// asked for this tick.
    #[test]
    fn the_summon_cap_counts_this_ticks_summons_too() {
        let mut app = test_app();
        app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
        // One more seat than the cap allows, all firing on the same tick.
        let seats: Vec<_> = (0..=MAX_ALLIES)
            .map(|i| {
                spawn_seated_body_holding(
                    &mut app,
                    PUPPY_SLUG_GUN_ID,
                    i as u8,
                    &format!("seat_{i}"),
                    ae::Vec2::new(100.0 * i as f32, 100.0),
                )
            })
            .collect();
        for body in seats {
            press_attack(&mut app, body);
        }
        app.update();
        assert_eq!(
            summons(&app).len(),
            MAX_ALLIES,
            "{} seats summoned on one tick and the cap is {MAX_ALLIES}",
            MAX_ALLIES + 1
        );
    }
}
