//! Always-on player HUD: health, mana, and money meters (visible build).
//!
//! A small bottom-left overlay drawn with Bevy UI: a red **health** bar, a
//! blue **mana** bar, and a gold **money** readout. Distinct from the
//! debug/quest text HUD (`app/hud.rs`) — this is the player-facing status
//! widget that's always on screen.
//!
//! Mana is a real spendable resource: the sim's
//! `ambition_actors::player::regen_player_mana` refills the `BodyMana`
//! meter over time so charge attacks / the fireball (which already spend it
//! via the projectile spawner) draw it down and it recovers. Money is fed by
//! `PickupKind::Currency` collection crediting the body wallet. This module
//! is a pure consumer of the sim-built
//! [`ambition_sim_view::PlayerHudFacts`] snapshot (E4 slices
//! 5+6+16) — it never queries live body clusters.

use bevy::prelude::*;

use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
use ambition_sim_view::PlayerHudFacts;

/// Bar width / height in logical px.
const BAR_W: f32 = 168.0;
const BAR_H: f32 = 13.0;

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
                // Top-left: the on-screen joystick lives bottom-left, so the
                // status bars sit up top out of its way.
                left: Val::Px(16.0),
                top: Val::Px(34.0),
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

/// Mirror the controlled body's health / mana / money into the HUD widgets each
/// frame: bar fill widths track the fractions, labels show the numbers.
///
/// Every stat is a BODY stat — health, mana, and the wallet all follow the
/// [`ControlledSubject`], so while possessing another body the HUD shows THAT
/// body's HP / MP / purse, not the vacated home avatar's. Economy is a body
/// concern (an NPC or merchant carries its own money and inventory), so the
/// wallet is just another cluster the driven body may hold. It's `Option` only
/// because not every body carries one yet; a body without a wallet reads `$0`.
pub fn update_player_hud(
    facts: Res<PlayerHudFacts>,
    mut fills: ParamSet<(
        Query<&mut Node, With<HealthFill>>,
        Query<&mut Node, With<ManaFill>>,
    )>,
    mut labels: ParamSet<(
        Query<&mut Text, With<HealthLabel>>,
        Query<&mut Text, With<ManaLabel>>,
        Query<&mut Text, With<MoneyLabel>>,
    )>,
) {
    if !facts.present {
        return;
    }
    let hp_frac = if facts.hp_max > 0 {
        (facts.hp_current as f32 / facts.hp_max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if let Ok(mut node) = fills.p0().single_mut() {
        node.width = Val::Percent(hp_frac * 100.0);
    }
    if let Ok(mut node) = fills.p1().single_mut() {
        node.width = Val::Percent(facts.mana_fraction * 100.0);
    }
    if let Ok(mut text) = labels.p0().single_mut() {
        set_text_if_changed(
            &mut text,
            format!("HP {}/{}", facts.hp_current, facts.hp_max),
        );
    }
    if let Ok(mut text) = labels.p1().single_mut() {
        set_text_if_changed(&mut text, format!("MP {}", facts.mana_current as i32));
    }
    if let Ok(mut text) = labels.p2().single_mut() {
        set_text_if_changed(&mut text, format!("${}", facts.balance));
    }
}

fn set_text_if_changed(text: &mut Text, next: String) {
    if text.as_str() != next.as_str() {
        **text = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_mirrors_the_sim_built_facts() {
        let mut app = App::new();

        // The sim already resolved the controlled body's meters into the
        // read-model — the HUD is a pure consumer (E4).
        app.insert_resource(PlayerHudFacts {
            present: true,
            hp_current: 3,
            hp_max: 10,
            mana_current: 12.0,
            mana_fraction: 0.2,
            balance: 7,
        });

        // Minimal HUD widgets (just the labels this assertion reads).
        app.world_mut().spawn((HealthLabel, Text::new("")));
        app.world_mut().spawn((ManaLabel, Text::new("")));
        app.world_mut().spawn((MoneyLabel, Text::new("")));
        app.world_mut().spawn((HealthFill, Node::default()));
        app.world_mut().spawn((ManaFill, Node::default()));

        app.add_systems(Update, update_player_hud);
        app.update();

        let mut labels = app
            .world_mut()
            .query::<(&Text, Option<&HealthLabel>, Option<&MoneyLabel>)>();
        let mut hp_text = None;
        let mut money_text = None;
        for (text, is_hp, is_money) in labels.iter(app.world()) {
            if is_hp.is_some() {
                hp_text = Some(text.as_str().to_string());
            }
            if is_money.is_some() {
                money_text = Some(text.as_str().to_string());
            }
        }
        assert_eq!(hp_text.as_deref(), Some("HP 3/10"));
        assert_eq!(money_text.as_deref(), Some("$7"));
    }

    #[test]
    fn hud_holds_last_state_when_no_body_resolved() {
        let mut app = App::new();
        app.insert_resource(PlayerHudFacts::default()); // present: false
        app.world_mut().spawn((HealthLabel, Text::new("HP 5/5")));
        app.add_systems(Update, update_player_hud);
        app.update();
        let mut labels = app.world_mut().query::<&Text>();
        let text = labels.iter(app.world()).next().unwrap();
        assert_eq!(text.as_str(), "HP 5/5", "startup frames hold the HUD");
    }
}
