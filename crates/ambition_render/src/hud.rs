//! Always-on player HUD: health, mana, and money meters (visible build).
//!
//! A small bottom-left overlay drawn with Bevy UI: a red **health** bar, a
//! blue **mana** bar, and a gold **money** readout. Distinct from the
//! debug/quest text HUD (`app/hud.rs`) — this is the player-facing status
//! widget that's always on screen.
//!
//! Mana is a real spendable resource: [`regen_player_mana`] refills the
//! `BodyMana` meter over time so charge attacks / the fireball (which already
//! spend it via the projectile spawner) draw it down and it recovers. Money is
//! fed by `PickupKind::Currency` collection crediting [`ambition_gameplay_core::actor::BodyWallet`].

use bevy::prelude::*;

use ambition_gameplay_core::abilities::traversal::possession::ControlledSubject;
use ambition_gameplay_core::actor::BodyHealth;
use ambition_gameplay_core::actor::BodyMana;
use ambition_gameplay_core::actor::BodyWallet;
use ambition_gameplay_core::actor::{PlayerEntity, PrimaryPlayer};

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

/// The body the local player is driving this frame: the [`ControlledSubject`]
/// (home avatar during normal play, the possessed actor while possessing),
/// falling back to the primary player for the startup frame before the subject
/// resolver has run. HUD health/mana track THIS body, not a fixed
/// `PrimaryPlayer` marker — the same read model the camera and nameplates use.
fn controlled_body(
    controlled: Option<&ControlledSubject>,
    primary: &Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) -> Option<Entity> {
    controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok())
}

/// Mana slowly regenerates so it's a genuine spendable resource. Uses
/// `ResourceMeter::refill` (clamped) rather than the meter's own `regen_rate`
/// field so we don't change `BodyMana::default` (and any test that relies on
/// it). Scaled by sim dt, so bullet-time / pause slow it with the world.
///
/// Refills the *controlled subject's* mana — the body actually spending it on
/// charge attacks / the fireball — so possessing an actor regenerates that
/// actor's meter, not the vacated home avatar's.
pub fn regen_player_mana(
    time: Res<ambition_gameplay_core::WorldTime>,
    controlled: Option<Res<ControlledSubject>>,
    mut manas: Query<&mut BodyMana>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let Some(subject) = controlled_body(controlled.as_deref(), &primary) else {
        return;
    };
    if let Ok(mut mana) = manas.get_mut(subject) {
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
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<(&BodyHealth, &BodyMana, Option<&BodyWallet>)>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
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
    let Some(subject) = controlled_body(controlled.as_deref(), &primary) else {
        return;
    };
    let Ok((health, mana, wallet)) = bodies.get(subject) else {
        return;
    };
    let balance = wallet.map(|wallet| wallet.balance).unwrap_or(0);
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
        set_text_if_changed(
            &mut text,
            format!("HP {}/{}", health.current(), health.max()),
        );
    }
    if let Ok(mut text) = labels.p1().single_mut() {
        set_text_if_changed(&mut text, format!("MP {}", mana.meter.current as i32));
    }
    if let Ok(mut text) = labels.p2().single_mut() {
        set_text_if_changed(&mut text, format!("${balance}"));
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
    fn wallet_add_clamps_and_spend_respects_balance() {
        let mut wallet = BodyWallet::default();
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
    fn hud_tracks_the_controlled_body_for_every_stat_including_money() {
        use ambition_characters::actor::Health;

        let mut app = App::new();

        // Home avatar: full HP, a fat $42 purse. We should see NONE of this
        // while driving another body.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(Health::new(20)),
            BodyMana::default(),
            BodyWallet { balance: 42 },
        ));

        // A possessed actor with ITS OWN economy: wounded 3/10 HP, $7 in pocket.
        // Money is a body concern, so possessing it spends its purse, not ours.
        let mut actor_hp = BodyHealth::new(Health::new(10));
        actor_hp.damage(7);
        let actor = app
            .world_mut()
            .spawn((actor_hp, BodyMana::default(), BodyWallet { balance: 7 }))
            .id();

        // The player is DRIVING the actor.
        app.world_mut()
            .insert_resource(ControlledSubject(Some(actor)));

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
        assert_eq!(
            hp_text.as_deref(),
            Some("HP 3/10"),
            "HP bar must show the POSSESSED body's health, not the home avatar's"
        );
        assert_eq!(
            money_text.as_deref(),
            Some("$7"),
            "money is a body stat: the HUD shows the driven body's purse, not the home avatar's"
        );
    }

    #[test]
    fn mana_regenerates_over_time_but_clamps_to_max() {
        let mut app = App::new();
        app.insert_resource(ambition_gameplay_core::WorldTime {
            raw_dt: 1.0,
            scaled_dt: 1.0,
        });
        app.add_systems(Update, regen_player_mana);
        let player = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyMana::default()))
            .id();
        // Drain it, then let it tick back up.
        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .try_spend(60.0);
        let before = app.world().get::<BodyMana>(player).unwrap().meter.current;
        app.update();
        let after = app.world().get::<BodyMana>(player).unwrap().meter.current;
        assert!(
            after > before,
            "mana should regenerate ({before} -> {after})"
        );

        // Many ticks can't exceed max.
        for _ in 0..20 {
            app.update();
        }
        let m = app.world().get::<BodyMana>(player).unwrap().meter;
        assert!(m.current <= m.max + 1e-3, "mana clamps to max");
    }
}
