//! Always-on player HUD: health, mana, and money meters (visible build).
//!
//! A small bottom-left overlay drawn with Bevy UI: a red **health** bar, a
//! blue **mana** bar, and a gold **money** readout. Separate from the
//! debug/quest text HUD (`app/hud.rs`); this is the always-visible
//! player-facing status widget.
//!
//! Mana is a spendable resource where the experience declares it: the mana
//! regeneration system refills the body's banked Mana, so charge attacks and
//! the fireball draw it down and it recovers. Money comes from
//! `PickupKind::Currency` pickups credited to the body wallet. This module
//! only reads the sim-built [`ambition_sim_view::ViewHudFacts`] of each view;
//! it never queries live body clusters.
//!
//! Each local view has its own HUD (Q150), placed in that view's part of the
//! gameplay rectangle and showing the body that view follows.

/// The declared-HUD renderer: whatever the active route's game declared.
pub mod declared;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::{
    gameplay_presentation::{
        ActiveHudDeclaration, ResolvedGameplayPresentation, ScreenOccluder, SurroundRegion,
    },
    lifecycle::{ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt},
    markers::{PlayerEntity, PrimaryPlayer},
};
use ambition_characters::control::PlayerSlot;
use ambition_sim_view::{LocalView, LocalViewId, SharedViewHudFacts, ViewHudFacts, ViewPlacement};

/// Bar width / height in logical px.
const BAR_W: f32 = 168.0;
const BAR_H: f32 = 13.0;

/// Where the HUD sits when it overlays gameplay: top-left, clear of the
/// bottom-left movement stick.
///
/// Public so an assembled test can tell the two placements apart. On a wide
/// pillarboxed display the overlay anchor can land in the surround anyway, so
/// "clear of the gameplay rect" cannot tell "placed" from "never moved"; the
/// anchor can.
pub const OVERLAY_ANCHOR: Vec2 = Vec2::new(16.0, 34.0);

/// Breathing room between the HUD and the edges of whatever region holds it.
pub const HUD_MARGIN: f32 = 12.0;

/// What the HUD needs to be legible. A smaller surround region is refused,
/// not squeezed: a clipped health bar is worse than one over the world.
const HUD_MIN: Vec2 = Vec2::new(BAR_W + HUD_MARGIN * 2.0, 96.0);

/// Root container for the player HUD overlay.
#[derive(Component)]
pub struct PlayerHudRoot;

/// The local view a HUD node shows (Q150). The root and each node that
/// [`update_player_hud`] writes carry it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudOfView(pub Entity);

/// The seat a HUD shows, for the HUD of another participant on a shared view
/// ([`SharedViewHudFacts`]). A HUD without it shows its view's own subject.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudOfSeat(pub PlayerSlot);

/// The vertical distance between two HUDs whose views start at one point.
const HUD_STACK: f32 = HUD_MIN.y;

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

/// Spawn one HUD for each local view that has none, and one for each other
/// participant on a shared view ([`SharedViewHudFacts`]), from the first frame
/// a primary player exists. Remove the HUD of a view that closed and of a
/// participant who left the view.
pub fn spawn_player_hud(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    players: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    views: Query<(Entity, &LocalViewId, Option<&SharedViewHudFacts>), With<LocalView>>,
    existing: Query<(Entity, &HudOfView, Option<&HudOfSeat>), With<PlayerHudRoot>>,
) {
    let on_view = |view: Entity, seat: Option<PlayerSlot>| match (views.get(view), seat) {
        (Err(_), _) => false,
        (Ok(_), None) => true,
        (Ok((_, _, shared)), Some(seat)) => {
            shared.is_some_and(|shared| shared.0.iter().any(|(slot, _)| *slot == seat))
        }
    };
    for (root, of, seat) in &existing {
        if !on_view(of.0, seat.map(|seat| seat.0)) {
            commands.entity(root).try_despawn();
        }
    }
    if players.is_empty() {
        return;
    }
    let mut unserved: Vec<(LocalViewId, Entity, Option<PlayerSlot>)> = views
        .iter()
        .flat_map(|(view, id, shared)| {
            let seats = shared.map_or(Vec::new(), |shared| {
                shared.0.iter().map(|(slot, _)| Some(*slot)).collect()
            });
            std::iter::once(None)
                .chain(seats)
                .map(move |seat| (*id, view, seat))
        })
        .filter(|(_, view, seat)| {
            !existing
                .iter()
                .any(|(_, of, has)| of.0 == *view && has.map(|has| has.0) == *seat)
        })
        .collect();
    if unserved.is_empty() {
        return;
    }
    unserved.sort_unstable();
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        // A shell host can keep a player for one deferred teardown frame. Do
        // not create gameplay UI without a live session owner.
        return;
    };
    for (_, view, seat) in unserved {
        spawn_hud_of_view(&mut commands, session_scope, view, seat);
    }
}

fn spawn_hud_of_view(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    view: Entity,
    seat: Option<PlayerSlot>,
) {
    let of = HudOfView(view);
    // The HUD of another participant on a shared view says whose it is.
    let tag = move |entity: &mut EntityCommands| {
        if let Some(seat) = seat {
            entity.insert(HudOfSeat(seat));
        }
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

    let mut root = commands.spawn_session_scoped(
            session_scope,
            (
                PlayerHudRoot,
                of,
                Node {
                    position_type: PositionType::Absolute,
                    // Start at the overlay anchor; `place_player_hud` moves it into
                    // the reserved surround on the first frame if the profile has one.
                    left: Val::Px(OVERLAY_ANCHOR.x),
                    top: Val::Px(OVERLAY_ANCHOR.y),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    ..default()
                },
                Name::new("Player HUD"),
                // Generic screen occupancy from this node's computed layout. The
                // resolver does not place the HUD, so the HUD is a producer: it
                // says what it is, and the host derives where it is.
                ScreenOccluder::hud(),
            ),
        );
    tag(&mut root);
    root.with_children(|root| {
            // Health bar (red fill + HP label).
            root.spawn((bar_node(), BackgroundColor(track)))
                .with_children(|bar| {
                    tag(&mut bar.spawn((
                        HealthFill,
                        of,
                        fill_node.clone(),
                        BackgroundColor(Color::srgb(0.90, 0.26, 0.32)),
                    )));
                    tag(&mut bar.spawn((
                        HealthLabel,
                        of,
                        overlay_label(),
                        Text::new("HP"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.98, 0.96, 0.98)),
                    )));
                });
            // Mana bar (blue fill + MP label).
            root.spawn((bar_node(), BackgroundColor(track)))
                .with_children(|bar| {
                    tag(&mut bar.spawn((
                        ManaFill,
                        of,
                        fill_node.clone(),
                        BackgroundColor(Color::srgb(0.30, 0.58, 1.0)),
                    )));
                    tag(&mut bar.spawn((
                        ManaLabel,
                        of,
                        overlay_label(),
                        Text::new("MP"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.98, 1.0)),
                    )));
                });
            // Money readout.
            tag(&mut root.spawn((
                MoneyLabel,
                of,
                Text::new("$0"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.86, 0.42)),
            )));
        });
}

/// Put the HUD in the reserved surround when the active profile offers one.
///
/// Ask the resolved layout for a named region, use it if the HUD fits, and
/// otherwise keep overlaying. A HUD knows its own size.
///
/// [`ResolvedControlRegions::hud`]:
///     ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedControlRegions::hud
///
/// Each HUD is placed in the part of the gameplay rectangle its view takes.
/// The HUD of a view that starts where the gameplay rectangle starts uses the
/// surround or the overlay anchor, as above. Another view's HUD overlays at
/// the overlay anchor inside its view. HUDs that start at one point (of views
/// that start at one point, or of the participants on one shared view) are
/// stacked in view order and then in seat order, so no HUD covers another.
#[allow(clippy::type_complexity)]
pub fn place_player_hud(
    presentation: Res<ResolvedGameplayPresentation>,
    views: Query<(&LocalViewId, Option<&ViewPlacement>), With<LocalView>>,
    mut roots: Query<(Entity, &mut Node, Option<&HudOfView>, Option<&HudOfSeat>), With<PlayerHudRoot>>,
) {
    // Left surround: status bars read left to right from the edge they use
    // when overlaying.
    let region = presentation
        .prefers_surround_hud()
        .then(|| presentation.hud_region(SurroundRegion::Left))
        .flatten()
        .filter(|rect| rect.width() >= HUD_MIN.x && rect.height() >= HUD_MIN.y);

    let base = match region {
        Some(rect) => rect.min + Vec2::splat(HUD_MARGIN),
        None => OVERLAY_ANCHOR,
    };
    let gameplay = presentation.gameplay_rect;
    let offset_of = |placement: Option<&ViewPlacement>| {
        placement.copied().unwrap_or_default().carve(gameplay.min, gameplay.size()).0 - gameplay.min
    };
    // Each HUD of a live view: where its view starts, and its stacking order.
    let placed: Vec<(Entity, Vec2, (LocalViewId, Entity, Option<PlayerSlot>))> = roots
        .iter()
        .filter_map(|(root, _, of, seat)| {
            let of = of?;
            let (id, placement) = views.get(of.0).ok()?;
            Some((root, offset_of(placement), (*id, of.0, seat.map(|seat| seat.0))))
        })
        .collect();
    for (root, mut node, _, _) in &mut roots {
        let anchor = match placed.iter().find(|(placed, ..)| *placed == root) {
            None => base,
            Some((_, offset, order)) => {
                let below = placed
                    .iter()
                    .filter(|(_, other_offset, other_order)| {
                        other_offset == offset && other_order < order
                    })
                    .count();
                let start = if *offset == Vec2::ZERO {
                    base
                } else {
                    gameplay.min + *offset + OVERLAY_ANCHOR
                };
                start + Vec2::new(0.0, below as f32 * HUD_STACK)
            }
        };
        if node.left != Val::Px(anchor.x) {
            node.left = Val::Px(anchor.x);
        }
        if node.top != Val::Px(anchor.y) {
            node.top = Val::Px(anchor.y);
        }
    }
}

/// This built-in HP/MP/$ row is Ambition's own HUD (see the module docs).
/// Hide it when the active route's game declares its own HUD (Sanic's rings,
/// Mary-O's score, coins, and lives), so vitals bars never overlay a game with
/// no health or mana. Ambition declares no custom HUD, so its row stays. A
/// game that wants vitals can declare its own health slot.
///
/// Presentation only (a `Node.display` toggle), outside any sim or rollback
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

/// Mirror each view's meters into the HUD widgets of that view each frame:
/// bar widths follow the fractions, labels show the numbers.
///
/// Every stat is a body stat of the body the view follows
/// ([`ViewHudFacts`]), so while possessing another body the HUD shows that
/// body's HP, MP, and purse. The wallet is `Option` because not every body
/// has one; no wallet reads `$0`. A view whose body did not resolve
/// (`present == false`) holds its last drawn state.
#[allow(clippy::type_complexity)]
pub fn update_player_hud(
    views: Query<(&ViewHudFacts, Option<&SharedViewHudFacts>)>,
    mut fills: ParamSet<(
        Query<(&mut Node, &HudOfView, Option<&HudOfSeat>), With<HealthFill>>,
        Query<(&mut Node, &HudOfView, Option<&HudOfSeat>), With<ManaFill>>,
    )>,
    mut labels: ParamSet<(
        Query<(&mut Text, &HudOfView, Option<&HudOfSeat>), With<HealthLabel>>,
        Query<(&mut Text, &HudOfView, Option<&HudOfSeat>), With<ManaLabel>>,
        Query<(&mut Text, &HudOfView, Option<&HudOfSeat>), With<MoneyLabel>>,
    )>,
) {
    let facts_of = |of: &HudOfView, seat: Option<&HudOfSeat>| {
        let (own, shared) = views.get(of.0).ok()?;
        let facts = match seat {
            None => own.0,
            Some(seat) => shared?
                .0
                .iter()
                .find(|(slot, _)| *slot == seat.0)
                .map(|(_, facts)| *facts)?,
        };
        facts.present.then_some(facts)
    };
    for (mut node, of, seat) in &mut fills.p0() {
        if let Some(facts) = facts_of(of, seat) {
            let hp_frac = if facts.hp_max > 0 {
                (facts.hp_current as f32 / facts.hp_max as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            node.width = Val::Percent(hp_frac * 100.0);
        }
    }
    for (mut node, of, seat) in &mut fills.p1() {
        if let Some(facts) = facts_of(of, seat) {
            node.width = Val::Percent(facts.mana.map_or(0.0, |mana| mana.fraction()) * 100.0);
        }
    }
    for (mut text, of, seat) in &mut labels.p0() {
        if let Some(facts) = facts_of(of, seat) {
            set_text_if_changed(
                &mut text,
                format!("HP {}/{}", facts.hp_current, facts.hp_max),
            );
        }
    }
    for (mut text, of, seat) in &mut labels.p1() {
        if let Some(facts) = facts_of(of, seat) {
            let label = match facts.mana {
                Some(mana) => format!("MP {}", mana.current as i32),
                // A body with no Mana reads as such, not as an empty pool.
                None => "MP -".to_owned(),
            };
            set_text_if_changed(&mut text, label);
        }
    }
    for (mut text, of, seat) in &mut labels.p2() {
        if let Some(facts) = facts_of(of, seat) {
            set_text_if_changed(&mut text, format!("${}", facts.balance));
        }
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
    use ambition_platformer2d_shared_tangle::lifecycle::{
        SessionScopePlugin, SessionScopeRetired, SessionScopedEntity,
    };

    use ambition_platformer2d_shared_tangle::gameplay_presentation::{
        profiles, resolve_gameplay_presentation, ControlFootprints, GameplayPresentationInput,
        PresentationEnvironment, ScreenInsets, ScreenRect,
    };

    /// Resolve a real declared profile at a real display size.
    fn layout(
        display: Vec2,
        profiles: ambition_platformer2d_shared_tangle::gameplay_presentation::GameplayPresentationProfiles,
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

    /// A profile that reserves surround for the HUD gets the HUD placed there.
    #[test]
    fn a_reserved_surround_profile_puts_the_hud_in_the_surround() {
        // 16:9 leaves 4:3 gameplay 1440 wide, so each side surround is 240px:
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

    /// A full-bleed profile has no surround, so the HUD keeps overlaying, as
    /// before for every game that did not ask for reserved HUD space.
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

    /// A surround too narrow for the bars is refused, not squeezed. A clipped
    /// health bar is worse than one over the world.
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

        // The sim already resolved the followed body's meters into the view's
        // read model; the HUD only consumes it.
        let view = app
            .world_mut()
            .spawn(ViewHudFacts(ambition_sim_view::PlayerHudFacts {
                present: true,
                hp_current: 3,
                hp_max: 10,
                mana: Some(ambition_platformer2d_core::resources::ResourceLevel {
                    current: 12.0,
                    max: 60.0,
                }),
                balance: 7,
            }))
            .id();
        let of = HudOfView(view);

        // Minimal HUD widgets (just the labels this assertion reads).
        app.world_mut().spawn((HealthLabel, of, Text::new("")));
        app.world_mut().spawn((ManaLabel, of, Text::new("")));
        app.world_mut().spawn((MoneyLabel, of, Text::new("")));
        app.world_mut().spawn((HealthFill, of, Node::default()));
        app.world_mut().spawn((ManaFill, of, Node::default()));

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

    /// Another game declares its own HUD, so vitals never overlay Sanic's rings
    /// or Mary-O's score.
    #[test]
    fn the_builtin_vitals_hud_hides_when_a_game_declares_its_own() {
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            HudDeclaration, HudSlotSpec,
        };
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
        // Ambition declares no custom HUD, so its vitals row shows.
        assert_eq!(display_with(None), Display::Flex);
        // Sanic and Mary-O declare their own HUD, so the vitals row hides.
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
        app.world_mut().spawn((LocalView, LocalViewId::FIRST));
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
        let view = app.world_mut().spawn(ViewHudFacts::default()).id(); // present: false
        app.world_mut()
            .spawn((HealthLabel, HudOfView(view), Text::new("HP 5/5")));
        app.add_systems(Update, update_player_hud);
        app.update();
        let mut labels = app.world_mut().query::<&Text>();
        let text = labels.iter(app.world()).next().unwrap();
        assert_eq!(text.as_str(), "HP 5/5", "startup frames hold the HUD");
    }

    /// Q150: two views (a split for two live rooms) get two HUDs, each in its
    /// own column and each showing its own view's meters. The control is the
    /// first view's HUD: it keeps the overlay anchor it had alone. When the
    /// second view closes, its HUD goes with it.
    #[test]
    fn each_view_has_its_own_hud_in_its_own_column() {
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin);
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, SessionScopedEntity(scope)));
        let presentation = layout(
            Vec2::new(1920.0, 1080.0),
            profiles::adaptive_platformer(),
            PresentationEnvironment::Desktop,
        );
        let gameplay = presentation.gameplay_rect;
        app.insert_resource(presentation);
        let meters = |hp: i32, balance: i32| {
            ViewHudFacts(ambition_sim_view::PlayerHudFacts {
                present: true,
                hp_current: hp,
                hp_max: 5,
                mana: None,
                balance,
            })
        };
        let alice = app
            .world_mut()
            .spawn((LocalView, LocalViewId::FIRST, ViewPlacement::column(0, 2), meters(3, 7)))
            .id();
        let bob = app
            .world_mut()
            .spawn((LocalView, LocalViewId(1), ViewPlacement::column(1, 2), meters(5, 0)))
            .id();
        app.add_systems(Update, (spawn_player_hud, place_player_hud, update_player_hud).chain());
        app.update();
        app.update();

        let shown = |app: &mut App| {
            let mut roots = app
                .world_mut()
                .query_filtered::<(&HudOfView, &Node), With<PlayerHudRoot>>();
            let px = |value| match value {
                Val::Px(px) => px,
                other => panic!("expected Px, got {other:?}"),
            };
            let placed: std::collections::BTreeMap<Entity, Vec2> = roots
                .iter(app.world())
                .map(|(of, node)| (of.0, Vec2::new(px(node.left), px(node.top))))
                .collect();
            let mut labels = app
                .world_mut()
                .query_filtered::<(&HudOfView, &Text), With<HealthLabel>>();
            let words: std::collections::BTreeMap<Entity, String> = labels
                .iter(app.world())
                .map(|(of, text)| (of.0, text.as_str().to_owned()))
                .collect();
            (placed, words)
        };
        let column = gameplay.min + Vec2::new(gameplay.width() / 2.0, 0.0);
        assert_eq!(
            shown(&mut app),
            (
                [(alice, OVERLAY_ANCHOR), (bob, column + OVERLAY_ANCHOR)].into(),
                [(alice, "HP 3/5".to_owned()), (bob, "HP 5/5".to_owned())].into(),
            ),
            "(each HUD's view and anchor, each HUD's health words) with two views in two columns"
        );

        app.world_mut().entity_mut(bob).despawn();
        app.update();
        let (placed, _) = shown(&mut app);
        assert_eq!(
            placed,
            [(alice, OVERLAY_ANCHOR)].into(),
            "the HUD of a closed view goes with it"
        );
    }

    /// Q150 on a merged screen: Alice and Bob share one view, and each has a
    /// HUD on it. Bob's is stacked under Alice's and shows his meters; the
    /// control is Alice's, at the anchor it has alone. When Bob leaves the
    /// view, his HUD goes with him.
    #[test]
    fn each_participant_on_a_shared_view_has_a_hud() {
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin);
        let scope = app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, SessionScopedEntity(scope)));
        app.insert_resource(layout(
            Vec2::new(1920.0, 1080.0),
            profiles::adaptive_platformer(),
            PresentationEnvironment::Desktop,
        ));
        let meters = |hp: i32| ambition_sim_view::PlayerHudFacts {
            present: true,
            hp_current: hp,
            hp_max: 5,
            mana: None,
            balance: 0,
        };
        let bob = PlayerSlot(1);
        let view = app
            .world_mut()
            .spawn((
                LocalView,
                LocalViewId::FIRST,
                ViewHudFacts(meters(3)),
                SharedViewHudFacts(vec![(bob, meters(5))]),
            ))
            .id();
        app.add_systems(Update, (spawn_player_hud, place_player_hud, update_player_hud).chain());
        app.update();
        app.update();

        let shown = |app: &mut App| {
            let mut roots = app
                .world_mut()
                .query_filtered::<(&HudOfView, Option<&HudOfSeat>, &Node), With<PlayerHudRoot>>();
            let mut placed: Vec<(Option<u8>, Val)> = roots
                .iter(app.world())
                .map(|(of, seat, node)| {
                    assert_eq!(of.0, view, "a HUD of another view");
                    (seat.map(|seat| seat.0 .0), node.top)
                })
                .collect();
            placed.sort_by_key(|(seat, _)| *seat);
            let mut labels = app
                .world_mut()
                .query_filtered::<(Option<&HudOfSeat>, &Text), With<HealthLabel>>();
            let mut words: Vec<(Option<u8>, String)> = labels
                .iter(app.world())
                .map(|(seat, text)| (seat.map(|seat| seat.0 .0), text.as_str().to_owned()))
                .collect();
            words.sort();
            (placed, words)
        };
        assert_eq!(
            shown(&mut app),
            (
                vec![
                    (None, Val::Px(OVERLAY_ANCHOR.y)),
                    (Some(1), Val::Px(OVERLAY_ANCHOR.y + HUD_STACK)),
                ],
                vec![(None, "HP 3/5".to_owned()), (Some(1), "HP 5/5".to_owned())],
            ),
            "(each HUD's seat and top, each HUD's health words) on one shared view"
        );

        app.world_mut()
            .entity_mut(view)
            .insert(SharedViewHudFacts(Vec::new()));
        app.update();
        let (placed, _) = shown(&mut app);
        assert_eq!(
            placed,
            vec![(None, Val::Px(OVERLAY_ANCHOR.y))],
            "the HUD of a participant who left the view goes with him"
        );
    }

    /// Two views over one area (no placement: a second camera, or a view
    /// laid over the first) stack their HUDs in view order, so neither
    /// covers the other. The control is the first view's HUD at the anchor
    /// it has alone.
    #[test]
    fn huds_of_two_views_over_one_area_stack() {
        let mut app = App::new();
        app.insert_resource(layout(
            Vec2::new(1920.0, 1080.0),
            profiles::adaptive_platformer(),
            PresentationEnvironment::Desktop,
        ));
        let second = app.world_mut().spawn((LocalView, LocalViewId(1))).id();
        let first = app.world_mut().spawn((LocalView, LocalViewId::FIRST)).id();
        for view in [first, second] {
            app.world_mut().spawn((PlayerHudRoot, HudOfView(view), Node::default()));
        }
        app.add_systems(Update, place_player_hud);
        app.update();
        let mut roots = app
            .world_mut()
            .query_filtered::<(&HudOfView, &Node), With<PlayerHudRoot>>();
        let placed: std::collections::BTreeMap<Entity, (Val, Val)> = roots
            .iter(app.world())
            .map(|(of, node)| (of.0, (node.left, node.top)))
            .collect();
        assert_eq!(
            placed,
            [
                (first, (Val::Px(OVERLAY_ANCHOR.x), Val::Px(OVERLAY_ANCHOR.y))),
                (second, (Val::Px(OVERLAY_ANCHOR.x), Val::Px(OVERLAY_ANCHOR.y + HUD_STACK))),
            ]
            .into(),
            "(each view's HUD anchor) with two views over the whole gameplay area"
        );
    }
}
