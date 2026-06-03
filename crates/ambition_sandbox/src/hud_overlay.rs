//! Always-on player HUD: health, mana, and money meters (visible build).
//!
//! A small bottom-left overlay drawn with Bevy UI: a red **health** bar, a
//! blue **mana** bar, and a gold **money** readout. Distinct from the
//! debug/quest text HUD (`app/hud.rs`) — this is the player-facing status
//! widget that's always on screen.
//!
//! Mana is a real spendable resource: [`regen_player_mana`] refills the
//! `PlayerMana` meter over time so charge attacks / the fireball (which already
//! spend it via the projectile spawner) draw it down and it recovers. Money is
//! fed by `PickupKind::Currency` collection crediting [`crate::player::PlayerWallet`].

use bevy::prelude::*;

use crate::player::{PlayerEntity, PlayerHealth, PlayerMana, PlayerWallet, PrimaryPlayer};

/// Bar width / height in logical px.
const BAR_W: f32 = 168.0;
const BAR_H: f32 = 13.0;
/// Mana regenerated per second (clamped to the meter max).
const MANA_REGEN_PER_SEC: f32 = 14.0;

/// Root container for the player HUD overlay.
#[derive(Component)]
pub struct PlayerHudRoot;

/// The colored fill inside the health bar (width = HP fraction).
#[derive(Component)]
pub struct HealthFill;
/// The colored fill inside the mana bar (width = mana fraction).
#[derive(Component)]
pub struct ManaFill;
/// "HP cur/max" overlay label.
#[derive(Component)]
pub struct HealthLabel;
/// "MP cur" overlay label.
#[derive(Component)]
pub struct ManaLabel;
/// "$balance" money readout.
#[derive(Component)]
pub struct MoneyLabel;

/// Mana slowly regenerates so it's a genuine spendable resource. Uses
/// `ResourceMeter::refill` (clamped) rather than the meter's own `regen_rate`
/// field so we don't change `PlayerMana::default` (and any test that relies on
/// it). Scaled by sim dt, so bullet-time / pause slow it with the world.
pub fn regen_player_mana(
    time: Res<crate::WorldTime>,
    mut players: Query<&mut PlayerMana, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for mut mana in &mut players {
        mana.meter.refill(MANA_REGEN_PER_SEC * dt);
    }
}

/// Spawn the HUD overlay once, the first frame a primary player exists.
pub fn spawn_player_hud(
    mut commands: Commands,
    players: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    existing: Query<(), With<PlayerHudRoot>>,
) {
    if !existing.is_empty() || players.is_empty() {
        return;
    }
    let track = Color::srgba(0.05, 0.06, 0.09, 0.85);
    let bar_node = || Node {
        width: Val::Px(BAR_W),
        height: Val::Px(BAR_H),
        ..default()
    };
    let fill_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    let overlay_label = || Node {
        position_type: PositionType::Absolute,
        left: Val::Px(6.0),
        top: Val::Px(0.0),
        ..default()
    };

    commands
        .spawn((
            PlayerHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            },
            Name::new("Player HUD"),
        ))
        .with_children(|root| {
            // Health bar (red fill + HP label).
            root.spawn((bar_node(), BackgroundColor(track)))
                .with_children(|bar| {
                    bar.spawn((
                        HealthFill,
                        fill_node.clone(),
                        BackgroundColor(Color::srgb(0.90, 0.26, 0.32)),
                    ));
                    bar.spawn((
                        HealthLabel,
                        overlay_label(),
                        Text::new("HP"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.98, 0.96, 0.98)),
                    ));
                });
            // Mana bar (blue fill + MP label).
            root.spawn((bar_node(), BackgroundColor(track)))
                .with_children(|bar| {
                    bar.spawn((
                        ManaFill,
                        fill_node.clone(),
                        BackgroundColor(Color::srgb(0.30, 0.58, 1.0)),
                    ));
                    bar.spawn((
                        ManaLabel,
                        overlay_label(),
                        Text::new("MP"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.98, 1.0)),
                    ));
                });
            // Money readout.
            root.spawn((
                MoneyLabel,
                Text::new("$0"),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.86, 0.42)),
            ));
        });
}

/// Mirror the primary player's health / mana / money into the HUD widgets each
/// frame: bar fill widths track the fractions, labels show the numbers.
pub fn update_player_hud(
    players: Query<
        (&PlayerHealth, &PlayerMana, &PlayerWallet),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    mut fills: ParamSet<(Query<&mut Node, With<HealthFill>>, Query<&mut Node, With<ManaFill>>)>,
    mut labels: ParamSet<(
        Query<&mut Text, With<HealthLabel>>,
        Query<&mut Text, With<ManaLabel>>,
        Query<&mut Text, With<MoneyLabel>>,
    )>,
) {
    let Ok((health, mana, wallet)) = players.single() else {
        return;
    };
    let hp_frac = if health.max() > 0 {
        (health.current() as f32 / health.max() as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mp_frac = mana.meter.fraction();
    if let Ok(mut node) = fills.p0().single_mut() {
        node.width = Val::Percent(hp_frac * 100.0);
    }
    if let Ok(mut node) = fills.p1().single_mut() {
        node.width = Val::Percent(mp_frac * 100.0);
    }
    if let Ok(mut text) = labels.p0().single_mut() {
        **text = format!("HP {}/{}", health.current(), health.max());
    }
    if let Ok(mut text) = labels.p1().single_mut() {
        **text = format!("MP {}", mana.meter.current as i32);
    }
    if let Ok(mut text) = labels.p2().single_mut() {
        **text = format!("${}", wallet.balance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_add_clamps_and_spend_respects_balance() {
        let mut wallet = PlayerWallet::default();
        assert_eq!(wallet.balance, 0);
        wallet.add(50);
        wallet.add(-100); // can't drive below zero
        assert_eq!(wallet.balance, 0);
        wallet.add(30);
        assert!(wallet.try_spend(20));
        assert_eq!(wallet.balance, 10);
        assert!(!wallet.try_spend(99), "can't overspend");
        assert_eq!(wallet.balance, 10);
    }

    #[test]
    fn mana_regenerates_over_time_but_clamps_to_max() {
        let mut app = App::new();
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0,
            scaled_dt: 1.0,
        });
        app.add_systems(Update, regen_player_mana);
        let player = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, PlayerMana::default()))
            .id();
        // Drain it, then let it tick back up.
        app.world_mut()
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .try_spend(60.0);
        let before = app.world().get::<PlayerMana>(player).unwrap().meter.current;
        app.update();
        let after = app.world().get::<PlayerMana>(player).unwrap().meter.current;
        assert!(after > before, "mana should regenerate ({before} -> {after})");

        // Many ticks can't exceed max.
        for _ in 0..20 {
            app.update();
        }
        let m = app.world().get::<PlayerMana>(player).unwrap().meter;
        assert!(m.current <= m.max + 1e-3, "mana clamps to max");
    }
}
