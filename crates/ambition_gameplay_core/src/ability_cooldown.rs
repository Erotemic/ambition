//! Shared cooldown for the movement abilities (Blink, Grapple) so they read as
//! deliberate verbs instead of spammable teleports.
//!
//! Stored as a per-player [`AbilityCooldown`] component, lazily inserted on first
//! use (like [`crate::abilities::traversal::mark_recall::PlayerMark`]), so it stays per-player for the
//! future multiplayer split. A single shared timer is fine: the player only holds
//! one ability at a time, so only the equipped ability's cooldown is ever in play.

use bevy::prelude::*;

use crate::actor::{PlayerEntity, PrimaryPlayer};

/// Per-player movement-ability cooldown (seconds remaining until the next use).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AbilityCooldown {
    pub remaining: f32,
}

impl AbilityCooldown {
    pub fn ready(&self) -> bool {
        self.remaining <= 0.0
    }

    pub fn trigger(&mut self, seconds: f32) {
        self.remaining = seconds;
    }
}

/// Returns `true` and arms the cooldown if the ability may fire now; returns
/// `false` (blocking the fire) while it's still running. Pass the player's
/// optional cooldown component (from the ability's query) and a `Commands` so the
/// component is lazily inserted the first time an ability is used.
pub fn try_use_ability(
    cooldown: &mut Option<Mut<AbilityCooldown>>,
    commands: &mut Commands,
    player: Entity,
    seconds: f32,
) -> bool {
    match cooldown {
        Some(cd) => {
            if !cd.ready() {
                return false;
            }
            cd.trigger(seconds);
            true
        }
        None => {
            commands
                .entity(player)
                .insert(AbilityCooldown { remaining: seconds });
            true
        }
    }
}

/// Tick the player's ability cooldown down by scaled dt, so bullet-time / pause
/// slow it the same way they slow everything else.
pub fn tick_ability_cooldown(
    time: Res<ambition_time::WorldTime>,
    mut players: Query<&mut AbilityCooldown, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.scaled_dt;
    for mut cd in &mut players {
        if cd.remaining > 0.0 {
            cd.remaining = (cd.remaining - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_ticks_down_to_ready() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            scaled_dt: 0.1,
            ..Default::default()
        });
        app.add_systems(Update, tick_ability_cooldown);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                AbilityCooldown { remaining: 0.5 },
            ))
            .id();

        app.update(); // 0.5 -> 0.4
        assert!(
            !app.world().get::<AbilityCooldown>(player).unwrap().ready(),
            "still cooling down after one tick",
        );
        for _ in 0..5 {
            app.update();
        }
        assert!(
            app.world().get::<AbilityCooldown>(player).unwrap().ready(),
            "ready again after enough ticks elapse",
        );
    }

    #[derive(Resource, Default)]
    struct UseLog(Vec<bool>);

    fn use_once_system(
        mut commands: Commands,
        mut players: Query<(Entity, Option<&mut AbilityCooldown>), With<PlayerEntity>>,
        mut log: ResMut<UseLog>,
    ) {
        if let Ok((player, mut cd)) = players.single_mut() {
            log.0
                .push(try_use_ability(&mut cd, &mut commands, player, 0.5));
        }
    }

    #[test]
    fn first_use_inserts_and_allows_then_armed_use_is_blocked() {
        let mut app = App::new();
        app.init_resource::<UseLog>();
        app.add_systems(Update, use_once_system);
        let player = app.world_mut().spawn(PlayerEntity).id();

        app.update(); // None -> insert + allow
        assert!(
            app.world().get::<AbilityCooldown>(player).is_some(),
            "first use lazily inserts the cooldown component",
        );
        app.update(); // Some(armed) -> blocked (no tick system here)
        assert_eq!(
            app.world().resource::<UseLog>().0,
            vec![true, false],
            "first use allowed, second (still armed) blocked",
        );
    }
}
