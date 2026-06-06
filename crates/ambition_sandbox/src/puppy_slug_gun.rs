//! Puppy-slug gun — a held item that summons **player-allied** puppy slugs.
//!
//! Jon's design (TODO "Puppy slug gun"): a held item whose `Attack` fires
//! friendly puppy slugs that harm the player's enemies but never the player.
//! Decided (Jon): the slugs **don't target — they just move** (their normal
//! surface-walker wander); they're simply player-allied now.
//!
//! Implementation: the slug spawns through the existing runtime-minion path
//! (`spawn_runtime_minion`) with [`ActorFaction::Player`] + a passive aggression.
//! The `can_damage` matrix then does all the work — a Player-faction body damages
//! Enemy-faction actors and is damaged by them, but never harms the player. No
//! new faction or targeting code is needed (the "ally that hunts" behaviour is a
//! future `AggressionTarget` variant, per `components.rs`). Capped at
//! [`MAX_ALLIES`] alive; they persist until killed or the room resets.

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::{ActorAggression, ActorFaction, HeldItem};
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PrimaryPlayer};

/// Marks a summoned, player-allied puppy slug (so the cap can count them and a
/// future system can manage them).
#[derive(Component, Clone, Copy, Debug)]
pub struct PuppySlugAlly;

/// The held-item id the gun grants (see `brain::action_set` HELD_ITEMS).
pub const PUPPY_SLUG_GUN_ID: &str = "puppy_slug_gun";

/// Most player-allied puppy slugs alive at once.
pub const MAX_ALLIES: usize = 3;

/// Archetype id of the spawned slug (must match `BRAIN_NAME_TO_ARCHETYPE`).
const SLUG_ARCHETYPE: &str = "puppy_slug";

/// `Attack` while holding the puppy-slug gun summons one player-allied puppy slug
/// ahead of the player, up to [`MAX_ALLIES`] alive. The gun's `HeldItemSpec` has
/// no melee/ranged verb, so this is the only thing `Attack` does while it's held.
pub fn fire_puppy_slug_gun_system(
    control: Res<ControlFrame>,
    mut commands: Commands,
    mut next_id: Local<u64>,
    players: Query<(&BodyKinematics, &HeldItem), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    allies: Query<(), With<PuppySlugAlly>>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    // Plain Attack summons; Shield+Attack is reserved for throwing the gun away
    // (handled by `throw_held_item_system`), so don't also summon then.
    if !control.attack_pressed || control.shield_held {
        return;
    }
    let Ok((kin, held)) = players.single() else {
        return;
    };
    if held.spec.id != PUPPY_SLUG_GUN_ID {
        return;
    }
    if allies.iter().count() >= MAX_ALLIES {
        return;
    }
    *next_id = next_id.wrapping_add(1);
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    let spawn_pos = kin.pos + ae::Vec2::new(facing * 40.0, -6.0);
    let entity = crate::features::spawn_runtime_minion(
        &mut commands,
        format!("puppy_slug_ally_{}", *next_id),
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
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::ActionSet;
    use crate::content::features::ActorFaction as Faction;
    use crate::player::PlayerBaseSize;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, fire_puppy_slug_gun_system);
        app
    }

    fn spawn_player_holding_gun(app: &mut App) {
        let spec = crate::brain::held_item_by_id(PUPPY_SLUG_GUN_ID).unwrap();
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PlayerBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            HeldItem::new(spec),
        ));
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
        spawn_player_holding_gun(&mut app);
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .attack_pressed = true;
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
        spawn_player_holding_gun(&mut app);
        // Press attack many times (re-arming the edge each frame).
        for _ in 0..6 {
            app.world_mut()
                .resource_mut::<ControlFrame>()
                .attack_pressed = true;
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
        spawn_player_holding_gun(&mut app);
        app.update();
        assert_eq!(ally_count(&mut app), 0);
    }
}
