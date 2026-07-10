//! Shared cooldown for the movement abilities (Blink, Grapple) so they read as
//! deliberate verbs instead of spammable teleports.
//!
//! Stored as a per-BODY [`AbilityCooldown`] component, lazily inserted on first
//! use. A single shared timer per body is fine: a body holds one ability at a
//! time, so only the equipped ability's cooldown is ever in play.
//!
//! **Body-generic, not player-only (S5/S6 fold, refactor-chain R6a).** `blink`
//! and `grapple` already act on the `ControlledSubject` — any body the human is
//! driving — and arm the cooldown on THAT body. The tick used to filter
//! `With<PlayerEntity>, With<PrimaryPlayer>`, so a possessed actor's armed
//! cooldown never counted down and it could never blink again. The fold fixed a
//! real bug rather than merely relaxing a filter.

use bevy::prelude::*;

/// A body's movement-ability cooldown (seconds remaining until the next use).
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
/// `false` (blocking the fire) while it's still running. Pass the acting BODY's
/// optional cooldown component (from the ability's query) and a `Commands` so the
/// component is lazily inserted the first time that body uses an ability.
pub fn try_use_ability(
    cooldown: &mut Option<Mut<AbilityCooldown>>,
    commands: &mut Commands,
    body: Entity,
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
                .entity(body)
                .insert(AbilityCooldown { remaining: seconds });
            true
        }
    }
}

/// Tick EVERY body's ability cooldown down by scaled dt, so bullet-time / pause
/// slow it the same way they slow everything else.
///
/// No `With<PrimaryPlayer>` filter: a body only carries this component once it has
/// actually used an ability, and `blink`/`grapple` arm it on the
/// `ControlledSubject` — which may be a possessed actor. Filtering to the home
/// avatar left a possessed body's cooldown armed forever.
pub fn tick_ability_cooldown(
    time: Res<ambition_time::WorldTime>,
    mut bodies: Query<&mut AbilityCooldown>,
) {
    let dt = time.scaled_dt;
    for mut cd in &mut bodies {
        if cd.remaining > 0.0 {
            cd.remaining = (cd.remaining - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{PlayerEntity, PrimaryPlayer};

    /// The bug the S5/S6 fold found: `blink`/`grapple` act on the
    /// `ControlledSubject`, so a POSSESSED actor arms this cooldown on itself —
    /// and the old `With<PlayerEntity>, With<PrimaryPlayer>` tick filter never
    /// counted it down. The possessed body could blink exactly once, ever.
    #[test]
    fn a_possessed_body_cooldown_ticks_down_too() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            scaled_dt: 0.1,
            ..Default::default()
        });
        app.add_systems(Update, tick_ability_cooldown);
        // No `PlayerEntity` / `PrimaryPlayer`: an ordinary actor body, driven.
        let possessed = app
            .world_mut()
            .spawn(AbilityCooldown { remaining: 0.5 })
            .id();

        for _ in 0..6 {
            app.update();
        }
        assert!(
            app.world()
                .get::<AbilityCooldown>(possessed)
                .unwrap()
                .ready(),
            "a possessed body's cooldown must expire like the home avatar's",
        );
    }

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
