//! Always-on player HUD: health, mana, and money meters (visible build).
//!
//! A small bottom-left overlay drawn with Bevy UI: a red **health** bar, a
//! blue **mana** bar, and a gold **money** readout. Distinct from the
//! debug/quest text HUD (`app/hud.rs`) — this is the player-facing status
//! widget that's always on screen.
//!
//! Mana is a real spendable resource: the sim's
//! the player mana regeneration system refills the `BodyMana`
//! meter over time so charge attacks / the fireball (which already spend it
//! via the projectile spawner) draw it down and it recovers. Money is fed by
//! `PickupKind::Currency` collection crediting the body wallet. This module
//! is a pure consumer of the sim-built
//! [`ambition_sim_view::PlayerHudFacts`] snapshot (E4 slices
//! 5+6+16) — it never queries live body clusters.

/// The DECLARED-HUD renderer: whatever the active route's game said its HUD
/// reads. This module's widget is Ambition's own fixed HP/MP/$ row, which is
/// precisely why a second game needed a seam.
pub mod declared;

use bevy::prelude::*;

use ambition_platformer_primitives::{
    gameplay_presentation::{
        ActiveHudDeclaration, ResolvedGameplayPresentation, ScreenOccluder, SurroundRegion,
    },
    lifecycle::{ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt},
    markers::{PlayerEntity, PrimaryPlayer},
};
use ambition_sim_view::PlayerHudFacts;

/// Bar width / height in logical px.
const BAR_W: f32 = 168.0;
const BAR_H: f32 = 13.0;

/// Where the HUD sits when it overlays gameplay: top-left, clear of the
/// bottom-left movement stick.
///
/// Public so an assembled test can tell the two placements apart by NAME. On a
/// widely pillarboxed display the overlay anchor happens to land in the
/// surround anyway, so "is it clear of the gameplay rect" cannot distinguish
/// "placed in the region it asked for" from "never moved" — the anchor can.
pub const OVERLAY_ANCHOR: Vec2 = Vec2::new(16.0, 34.0);

/// Breathing room between the HUD and the edges of whatever region holds it.
pub const HUD_MARGIN: f32 = 12.0;

/// What the HUD needs to be legible. Below this a surround region is refused
/// rather than squeezed — a clipped health bar is worse than one over the
/// world.
const HUD_MIN: Vec2 = Vec2::new(BAR_W + HUD_MARGIN * 2.0, 96.0);

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
    active_session: Option<Res<ActiveSessionScope>>,
    players: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    existing: Query<(), With<PlayerHudRoot>>,
) {
    if !existing.is_empty() || players.is_empty() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        // A shell host can retain a player for one deferred teardown frame.
        // Never materialize new gameplay UI without a live session owner.
        return;
    };
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
        .spawn_session_scoped(
            session_scope,
            (
                PlayerHudRoot,
                Node {
                    position_type: PositionType::Absolute,
                    // Overlay anchor to start; `place_player_hud` moves it into
                    // the reserved surround on the first frame if the active
                    // profile offers one.
                    left: Val::Px(OVERLAY_ANCHOR.x),
                    top: Val::Px(OVERLAY_ANCHOR.y),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    ..default()
                },
                Name::new("Player HUD"),
                // Generic screen occupancy, read off this node's own computed
                // layout. The HUD is not placed by the resolver, so unlike the
                // touch clusters it really is a producer: it says what it is,
                // and the host derives where it is.
                ScreenOccluder::hud(),
            ),
        )
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

/// Put the HUD in the reserved surround when the active profile offers one.
///
/// The proving consumer for [`ResolvedControlRegions::hud`]: a fixed-aspect
/// profile that reserves surround for HUD was, until this existed, reserving it
/// for nobody — the bars stayed pinned over the gameplay rectangle while a
/// whole column sat empty beside them.
///
/// The whole author API is the three lines below: ask the resolved layout for a
/// named region, take it if the HUD fits, otherwise keep overlaying. No
/// responsive framework, no layout negotiation — a HUD knows its own size.
///
/// [`ResolvedControlRegions::hud`]:
///     ambition_platformer_primitives::gameplay_presentation::ResolvedControlRegions::hud
pub fn place_player_hud(
    presentation: Res<ResolvedGameplayPresentation>,
    mut roots: Query<&mut Node, With<PlayerHudRoot>>,
) {
    // Left surround: these are status bars, and they read left-to-right from
    // the same edge they occupy when overlaying.
    let region = presentation
        .prefers_surround_hud()
        .then(|| presentation.hud_region(SurroundRegion::Left))
        .flatten()
        .filter(|rect| rect.width() >= HUD_MIN.x && rect.height() >= HUD_MIN.y);

    let anchor = match region {
        Some(rect) => rect.min + Vec2::splat(HUD_MARGIN),
        None => OVERLAY_ANCHOR,
    };
    for mut node in &mut roots {
        if node.left != Val::Px(anchor.x) {
            node.left = Val::Px(anchor.x);
        }
        if node.top != Val::Px(anchor.y) {
            node.top = Val::Px(anchor.y);
        }
    }
}

/// This built-in HP/MP/$ row is AMBITION's own HUD (see the module docs). Hide it
/// whenever the active route's game declared its OWN HUD — Sanic's rings,
/// Mary-O's score/coins/lives — so the vitals bars never overlay a game that has
/// no health or mana (Jon bug #36). Ambition declares no custom HUD, so its
/// built-in row stays visible; a game that genuinely wants vitals can declare a
/// health slot of its own.
///
/// Presentation-only (a `Node.display` toggle), so it is outside any sim/rollback
/// concern.
pub fn toggle_builtin_hud_for_declared_games(
    active: Res<ActiveHudDeclaration>,
    mut roots: Query<&mut Node, With<PlayerHudRoot>>,
) {
    let want = if active.0.is_none() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut roots {
        if node.display != want {
            node.display = want;
        }
    }
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
    use ambition_platformer_primitives::lifecycle::{
        SessionScopePlugin, SessionScopeRetired, SessionScopedEntity,
    };

    use ambition_platformer_primitives::gameplay_presentation::{
        profiles, resolve_gameplay_presentation, ControlFootprints, GameplayPresentationInput,
        PresentationEnvironment, ScreenInsets, ScreenRect,
    };

    /// Resolve a real declared profile at a real display size.
    fn layout(
        display: Vec2,
        profiles: ambition_platformer_primitives::gameplay_presentation::GameplayPresentationProfiles,
        environment: PresentationEnvironment,
    ) -> ResolvedGameplayPresentation {
        resolve_gameplay_presentation(GameplayPresentationInput {
            display_px: display,
            safe_area_insets: ScreenInsets::ZERO,
            profile: profiles.for_environment(environment),
            occlusions: &[],
            control_footprints: ControlFootprints::default(),
        })
    }

    fn placed_at(presentation: ResolvedGameplayPresentation) -> Vec2 {
        let mut app = App::new();
        app.insert_resource(presentation);
        app.world_mut().spawn((
            PlayerHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(OVERLAY_ANCHOR.x),
                top: Val::Px(OVERLAY_ANCHOR.y),
                ..default()
            },
        ));
        app.add_systems(Update, place_player_hud);
        app.update();

        let mut roots = app
            .world_mut()
            .query_filtered::<&Node, With<PlayerHudRoot>>();
        let node = roots.single(app.world()).expect("the HUD root");
        let px = |value| match value {
            Val::Px(px) => px,
            other => panic!("expected Px, got {other:?}"),
        };
        Vec2::new(px(node.left), px(node.top))
    }

    /// The proving vertical slice: a profile that reserves surround for HUD
    /// actually gets the HUD put there.
    ///
    /// Driven through Mary O's REAL declared profile rather than a hand-built
    /// one, because the claim being made is about a shipping game: its 4:3
    /// viewport was reserving a surround column that nothing ever moved into.
    #[test]
    fn a_reserved_surround_profile_puts_the_hud_in_the_surround() {
        // 16:9 leaves 4:3 gameplay 1440 wide, so each side surround is 240px —
        // room for the 168px bars plus margins.
        let display = Vec2::new(1920.0, 1080.0);
        let presentation = layout(
            display,
            profiles::fixed_four_by_three(),
            PresentationEnvironment::Desktop,
        );

        let region = presentation
            .hud_region(SurroundRegion::Left)
            .expect("a 4:3 viewport on 16:9 leaves a left HUD region");
        let anchor = placed_at(presentation.clone());

        assert_eq!(
            anchor,
            region.min + Vec2::splat(HUD_MARGIN),
            "the HUD must occupy the region it asked for",
        );
        let occupied = ScreenRect::from_min_size(anchor, Vec2::new(BAR_W, HUD_MIN.y));
        assert!(
            !occupied.overlaps(presentation.gameplay_rect),
            "and therefore stop covering the world: {occupied:?} vs {:?}",
            presentation.gameplay_rect,
        );
    }

    /// A full-bleed profile has no surround, so the HUD keeps overlaying —
    /// unchanged behavior for every game that did not ask for reserved HUD.
    #[test]
    fn a_full_bleed_profile_leaves_the_hud_overlaying() {
        let presentation = layout(
            Vec2::new(1920.0, 1080.0),
            profiles::adaptive_platformer(),
            PresentationEnvironment::Desktop,
        );
        assert!(presentation.hud_region(SurroundRegion::Left).is_none());
        assert_eq!(placed_at(presentation), OVERLAY_ANCHOR);
    }

    /// A surround too narrow to hold the bars is REFUSED, not squeezed. A
    /// clipped health bar is worse than one over the world.
    #[test]
    fn a_surround_too_narrow_for_the_hud_falls_back_to_overlay() {
        // Barely wider than 4:3: the gameplay rect takes 1365 of 1400, so
        // each side column is ~17px.
        let presentation = layout(
            Vec2::new(1400.0, 1024.0),
            profiles::fixed_four_by_three(),
            PresentationEnvironment::Desktop,
        );
        let region = presentation
            .hud_region(SurroundRegion::Left)
            .expect("there IS a region, just a narrow one");
        assert!(
            region.width() < HUD_MIN.x,
            "the fixture must actually be too narrow, got {}",
            region.width(),
        );
        assert_eq!(placed_at(presentation), OVERLAY_ANCHOR);
    }

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

    /// Jon bug #36: the built-in HP/MP/$ row is Ambition's own; it must hide when
    /// another game declares its own HUD, so vitals never overlay Sanic's rings or
    /// Mary-O's score.
    #[test]
    fn the_builtin_vitals_hud_hides_when_a_game_declares_its_own() {
        use ambition_platformer_primitives::gameplay_presentation::{HudDeclaration, HudSlotSpec};
        fn display_with(declaration: Option<HudDeclaration>) -> Display {
            let mut app = App::new();
            app.insert_resource(ActiveHudDeclaration(declaration));
            app.world_mut().spawn((PlayerHudRoot, Node::default()));
            app.add_systems(Update, toggle_builtin_hud_for_declared_games);
            app.update();
            let mut roots = app
                .world_mut()
                .query_filtered::<&Node, With<PlayerHudRoot>>();
            roots.single(app.world()).expect("the HUD root").display
        }
        // Ambition declares no custom HUD → its built-in vitals row shows.
        assert_eq!(display_with(None), Display::Flex);
        // Sanic / Mary-O declare their own HUD → the vitals row hides, so it can
        // never overlay the game's own readouts.
        assert_eq!(
            display_with(Some(HudDeclaration::new().slot(HudSlotSpec::new("rings")))),
            Display::None,
        );
    }

    #[test]
    fn hud_root_retires_with_its_exact_gameplay_session() {
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin);
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, SessionScopedEntity(scope)));
        app.add_systems(Update, spawn_player_hud);

        app.update();

        let mut owners = app
            .world_mut()
            .query_filtered::<&SessionScopedEntity, With<PlayerHudRoot>>();
        let hud_owners: Vec<_> = owners.iter(app.world()).copied().collect();
        assert_eq!(hud_owners, vec![SessionScopedEntity(scope)]);

        app.world_mut().write_message(SessionScopeRetired(scope));
        app.update();

        let mut roots = app.world_mut().query::<&PlayerHudRoot>();
        assert_eq!(roots.iter(app.world()).count(), 0);
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
