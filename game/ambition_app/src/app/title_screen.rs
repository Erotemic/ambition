//! The backdrop of the title screen: the scene of the game the cursor is on.
//!
//! The launcher is a flat menu, and it was a flat menu on black. Behind it
//! now is the parallax scene of the highlighted game (the hub for Ambition,
//! the open sky for Sanic, the starlight kingdom for Mary-O, the eclipse for
//! Smash Siblings, the cove at night for Exit). Its four layers drift at four
//! speeds, and a move of the cursor fades one scene into the next.
//!
//! Presentation only, and host-owned: the launcher's rows, cursor and input
//! are `ambition_game_shell`'s, and this reads them. The art is the published
//! parallax layers of each biome. Nothing here is drawn by code.
//!
//! A device with no budget for parallax (`ParallaxBudget::enabled`) has no
//! backdrop. The menu keeps its title-screen shape there, on its usual dark
//! window.
//!
//! The menu itself is the flat renderer's (`ambition_menu`), which draws each
//! menu of the app the same way. [`style_the_launcher_menu`] gives the one on
//! the title screen its shape: a narrow glass window with round corners, rows
//! with round corners and a thin edge, and an edge on the cursor's row that
//! breathes. It changes the shape and the edges only. The fill of a row and
//! the colour of its text stay the renderer's, so the cursor is gold here as
//! it is in each other menu.

use bevy::prelude::*;

use ambition_platformer2d::game_shell::{
    BasicShellUiRoot, LauncherTab, ShellExperienceRegistry, ShellLauncherState,
};
use ambition_platformer2d::menu::render::bevy_ui::{AmbitionMenuControlKind, BevyUiMenuPanel, BevyUiMenuTab};
use ambition_platformer2d::menu::{MenuControlKind, MenuVisualState};
use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::sprite_sheet::game_assets::ParallaxTheme;

/// The layers of a scene from the far one to the near one, and how much each
/// one drifts: the near ones more, as they do when a camera moves.
const LAYERS: [(&str, f32); 4] =
    [("sky", 0.15), ("far_backplate", 0.4), ("near_background", 0.75), ("foreground_atmosphere", 1.0)];
/// How wide a layer is, in parts of a hundred of the view's width. A layer is
/// square, and a 16:9 view of this width shows the middle band of it, as the
/// game's camera does.
const LAYER_VW: f32 = 132.0;
/// How far the nearest layer drifts to each side, in the same unit.
const DRIFT_VW: f32 = 9.0;
/// The seconds of one drift there and back.
const DRIFT_PERIOD_S: f32 = 80.0;
/// The seconds one scene takes to fade into the next.
const FADE_S: f32 = 0.9;

/// The scrim the menu puts over what is behind it, on the title screen: the
/// backdrop is the point of the screen, so it is dimmed only a little.
const SCRIM: Color = Color::srgba(0.0, 0.0, 0.02, 0.20);
/// The menu's window, on the title screen: dark glass over the backdrop.
const WINDOW: Color = Color::srgba(0.035, 0.045, 0.085, 0.70);

/// The width of the window, in parts of a hundred of the view's height. Text
/// is sized by the height of the view, so the window is too: a row keeps its
/// shape on a view of any aspect. On a 16:9 view this is 54 of its width.
const WINDOW_WIDTH_VH: f32 = 96.0;
/// The most of the view's width the window takes (a tall, narrow view).
const WINDOW_MOST_PERCENT: f32 = 94.0;
const WINDOW_RADIUS_VH: f32 = 2.4;
const WINDOW_EDGE_VH: f32 = 0.28;
const WINDOW_EDGE: Color = Color::srgba(0.78, 0.85, 1.0, 0.30);
const ROW_RADIUS_VH: f32 = 1.6;
const ROW_EDGE_VH: f32 = 0.32;
/// The edge of a row the cursor is not on.
const ROW_EDGE: Color = Color::srgba(0.72, 0.80, 1.0, 0.22);
/// The edge of the cursor's row at its brightest. Its alpha goes between
/// [`CURSOR_EDGE_LOW`] and 1 once each [`CURSOR_BREATH_S`].
const CURSOR_EDGE: Color = Color::srgb(1.0, 0.98, 0.86);
const CURSOR_EDGE_LOW: f32 = 0.45;
const CURSOR_BREATH_S: f32 = 1.8;

/// The scene behind the row of `route`. `None` is the Exit row.
///
/// The host names each provider's scene: it is the one composition that has
/// all of them. A route it does not know gets the hub.
fn scene_of(route: Option<&str>) -> ParallaxTheme {
    match route {
        None => ParallaxTheme::Cove,
        Some(route) if route.starts_with("sanic") => ParallaxTheme::OpenSky,
        Some(route) if route.starts_with("mary_o") => ParallaxTheme::Starlight,
        Some(route) if route.starts_with("smash") || route.starts_with("versus") => ParallaxTheme::Eclipse,
        Some(_) => ParallaxTheme::Hub,
    }
}

/// The root of the backdrop: one for the title screen, behind the menu.
#[derive(Component)]
pub struct TitleBackdrop;

/// One scene of the backdrop. `shown` goes to `wanted` over [`FADE_S`]; a
/// scene that is not wanted and not shown is taken away.
#[derive(Component)]
pub struct TitleScene {
    pub theme: ParallaxTheme,
    shown: f32,
    wanted: f32,
}

/// One layer of a scene, and how much it drifts.
#[derive(Component)]
struct TitleLayer {
    depth: f32,
}

pub struct TitleScreenPlugin;

impl Plugin for TitleScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (show_the_scene_of_the_cursor, fade_and_drift_the_scenes).chain().run_if(the_launcher_is_up),
                take_the_backdrop_away.run_if(not(the_launcher_is_up)),
            )
                // An app that does not draw has no backdrop.
                .run_if(resource_exists::<AssetServer>)
                .run_if(resource_exists::<Time>)
                .run_if(resource_exists::<ShellLauncherState>)
                .run_if(resource_exists::<ShellExperienceRegistry>),
        );
        // After the menu is made (`Update`) and before its layout, so a menu
        // is never drawn for one frame in the renderer's shape: a change of
        // tab makes the menu again, and that frame would show as a blink.
        app.add_systems(
            PostUpdate,
            style_the_launcher_menu
                .before(bevy::ui::UiSystems::Layout)
                .run_if(the_launcher_is_up)
                .run_if(resource_exists::<Time>),
        );
    }
}

fn the_launcher_is_up(launcher: Option<Res<ShellLauncherState>>) -> bool {
    launcher.is_some_and(|launcher| launcher.active)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where a layer is at `seconds`: in the middle of the view, and to one side
/// of it by its drift.
fn layer_left(depth: f32, seconds: f32) -> Val {
    let swing = (seconds * std::f32::consts::TAU / DRIFT_PERIOD_S).sin();
    Val::Vw(-(LAYER_VW - 100.0) * 0.5 + depth * DRIFT_VW * swing)
}

/// Want the scene of the row the cursor is on, and make it if it is not there.
fn show_the_scene_of_the_cursor(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    registry: Res<ShellExperienceRegistry>,
    quality: Option<Res<ResolvedVisualQuality>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    roots: Query<Entity, With<TitleBackdrop>>,
    mut scenes: Query<&mut TitleScene>,
    mut order: Local<i32>,
) {
    let budget = quality.as_deref().map(|quality| &quality.budget.parallax);
    if budget.is_some_and(|budget| !budget.enabled) {
        return;
    }
    // The settings tab has no game under the cursor: the scene stays.
    let wanted = match launcher.tab {
        LauncherTab::Home => {
            let entries = registry.launch_entries();
            let route = entries.iter().filter(|entry| entry.available).nth(launcher.selected).map(|entry| entry.route_id.as_str().to_owned());
            Some(scene_of(route.as_deref()))
        }
        LauncherTab::Settings => scenes.iter().find(|scene| scene.wanted > 0.5).map(|scene| scene.theme),
    }
    .unwrap_or(ParallaxTheme::Hub);

    let root = roots.iter().next().unwrap_or_else(|| {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                // Behind the menu (1000) and behind each control on the screen.
                GlobalZIndex(-100),
                TitleBackdrop,
                Name::new("title backdrop"),
            ))
            .id()
    });

    let mut present = false;
    for mut scene in &mut scenes {
        let is_wanted = scene.theme == wanted;
        present |= is_wanted;
        scene.wanted = if is_wanted { 1.0 } else { 0.0 };
    }
    if present {
        return;
    }
    let subdir = budget.map_or("backgrounds/parallax_layers", |budget| budget.resolution_scale.parallax_subdir());
    // The first scene of the screen is there at once: there is nothing to fade from.
    let shown = if scenes.is_empty() { 1.0 } else { 0.0 };
    *order += 1;
    let seconds = time.elapsed_secs();
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            // The newest scene is in front, so it fades in over the one before.
            ZIndex(*order),
            TitleScene { theme: wanted, shown, wanted: 1.0 },
            Name::new(format!("title scene: {}", wanted.key())),
        ))
        .with_children(|scene| {
            for (layer, depth) in LAYERS {
                let mut image = ImageNode::new(asset_server.load(format!("{subdir}/{}_{layer}.png", wanted.key())));
                image.color = Color::WHITE.with_alpha(smoothstep(shown));
                scene.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Vw(LAYER_VW),
                        aspect_ratio: Some(1.0),
                        left: layer_left(depth, seconds),
                        // Its middle on the middle of the view.
                        top: Val::Percent(50.0),
                        margin: UiRect::top(Val::Vw(-LAYER_VW * 0.5)),
                        ..default()
                    },
                    image,
                    TitleLayer { depth },
                ));
            }
        });
    });
}

/// Move each scene to the alpha it is wanted at, and each layer along its drift.
fn fade_and_drift_the_scenes(
    mut commands: Commands,
    time: Res<Time>,
    mut scenes: Query<(Entity, &mut TitleScene)>,
    mut layers: Query<(&ChildOf, &TitleLayer, &mut Node, &mut ImageNode)>,
) {
    let step = time.delta_secs() / FADE_S;
    for (entity, mut scene) in &mut scenes {
        scene.shown = if scene.wanted > scene.shown { (scene.shown + step).min(1.0) } else { (scene.shown - step).max(0.0) };
        if scene.shown <= 0.0 && scene.wanted <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
    let seconds = time.elapsed_secs();
    for (parent, layer, mut node, mut image) in &mut layers {
        let Ok((_, scene)) = scenes.get(parent.parent()) else {
            continue;
        };
        node.left = layer_left(layer.depth, seconds);
        image.color = Color::WHITE.with_alpha(smoothstep(scene.shown));
    }
}

/// The launcher is gone (a game is on, or a card): so is its backdrop.
fn take_the_backdrop_away(mut commands: Commands, roots: Query<Entity, With<TitleBackdrop>>) {
    for root in &roots {
        commands.entity(root).try_despawn();
    }
}

/// `entity` is a part of the launcher's menu: one of the `steps` entities
/// above it is the launcher's root.
fn in_the_launcher(entity: Entity, steps: usize, parents: &Query<&ChildOf>, roots: &Query<(), With<BasicShellUiRoot>>) -> bool {
    parents.iter_ancestors(entity).take(steps).any(|above| roots.contains(above))
}

/// The alpha of the cursor's edge at `seconds`.
fn cursor_edge_alpha(seconds: f32) -> f32 {
    let breath = 0.5 + 0.5 * (seconds * std::f32::consts::TAU / CURSOR_BREATH_S).sin();
    CURSOR_EDGE_LOW + (1.0 - CURSOR_EDGE_LOW) * breath
}

/// Give the launcher's menu the shape of a title screen (see the module doc).
///
/// Each write is only on a difference, so a menu at rest changes one thing in
/// a frame: the edge of the cursor's row.
fn style_the_launcher_menu(
    mut commands: Commands,
    time: Res<Time>,
    backdrops: Query<(), With<TitleBackdrop>>,
    roots: Query<(), With<BasicShellUiRoot>>,
    root_ids: Query<Entity, With<BasicShellUiRoot>>,
    parents: Query<&ChildOf>,
    windows: Query<Entity, With<BevyUiMenuPanel>>,
    tabs: Query<(Entity, &BevyUiMenuTab)>,
    rows: Query<(Entity, &MenuVisualState, &AmbitionMenuControlKind)>,
    mut nodes: Query<&mut Node>,
    mut fills: Query<&mut BackgroundColor>,
    mut edges: Query<&mut BorderColor>,
) {
    let mut edge = |entity: Entity, colour: Color| {
        let wanted = BorderColor::all(colour);
        match edges.get_mut(entity) {
            Ok(mut edge) if *edge != wanted => *edge = wanted,
            Ok(_) => {}
            Err(_) => {
                commands.entity(entity).try_insert(wanted);
            }
        }
    };
    let has_backdrop = !backdrops.is_empty();

    // With a backdrop behind it, the menu dims it less and the window lets it
    // through. With none, the scrim and the window are as the renderer made them.
    if has_backdrop {
        for root in &root_ids {
            if let Ok(mut scrim) = fills.get_mut(root) {
                if scrim.0 != SCRIM {
                    scrim.0 = SCRIM;
                }
            }
        }
    }

    for window in &windows {
        if !in_the_launcher(window, 1, &parents, &roots) {
            continue;
        }
        if has_backdrop {
            if let Ok(mut fill) = fills.get_mut(window) {
                if fill.0 != WINDOW {
                    fill.0 = WINDOW;
                }
            }
        }
        if let Ok(mut node) = nodes.get_mut(window) {
            let radius = BorderRadius::all(Val::Vh(WINDOW_RADIUS_VH));
            let border = UiRect::all(Val::Vh(WINDOW_EDGE_VH));
            if node.width != Val::Vh(WINDOW_WIDTH_VH) || node.border_radius != radius || node.border != border {
                node.width = Val::Vh(WINDOW_WIDTH_VH);
                node.max_width = Val::Percent(WINDOW_MOST_PERCENT);
                node.border_radius = radius;
                node.border = border;
            }
        }
        edge(window, WINDOW_EDGE);
    }

    // The two end tabs are in the top corners of the window: they take its
    // corners, inside its edge.
    let last_tab = tabs.iter().filter(|(tab, _)| in_the_launcher(*tab, 3, &parents, &roots)).map(|(_, tab)| tab.index).max();
    for (entity, tab) in &tabs {
        if !in_the_launcher(entity, 3, &parents, &roots) {
            continue;
        }
        let corner = Val::Vh(WINDOW_RADIUS_VH - WINDOW_EDGE_VH);
        let mut radius = BorderRadius::ZERO;
        if tab.index == 0 {
            radius.top_left = corner;
        }
        if Some(tab.index) == last_tab {
            radius.top_right = corner;
        }
        if let Ok(mut node) = nodes.get_mut(entity) {
            if node.border_radius != radius {
                node.border_radius = radius;
            }
        }
    }

    let cursor_edge = CURSOR_EDGE.with_alpha(cursor_edge_alpha(time.elapsed_secs()));
    for (entity, state, kind) in &rows {
        // A scrollbar is not a row.
        if matches!(kind.0, MenuControlKind::Scrollbar) || !in_the_launcher(entity, 3, &parents, &roots) {
            continue;
        }
        if let Ok(mut node) = nodes.get_mut(entity) {
            let radius = BorderRadius::all(Val::Vh(ROW_RADIUS_VH));
            let border = UiRect::all(Val::Vh(ROW_EDGE_VH));
            if node.border_radius != radius || node.border != border {
                node.border_radius = radius;
                node.border = border;
            }
        }
        edge(entity, if state.focused && !state.disabled { cursor_edge } else { ROW_EDGE });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_row_of_the_launcher_has_a_scene_and_no_two_games_share_one() {
        let scenes = [
            scene_of(Some("ambition_gameplay")),
            scene_of(Some("sanic_gameplay")),
            scene_of(Some("mary_o_gameplay")),
            scene_of(Some("smash_select")),
            scene_of(None),
        ];
        for (i, a) in scenes.iter().enumerate() {
            for b in &scenes[i + 1..] {
                assert_ne!(a, b, "two rows of the launcher show one scene");
            }
        }
    }

    #[test]
    fn the_edge_of_the_cursor_breathes_and_never_goes_out() {
        let alphas: Vec<f32> = (0..=180).map(|hundredth| cursor_edge_alpha(hundredth as f32 * 0.01)).collect();
        let (low, high) = alphas.iter().fold((f32::MAX, f32::MIN), |(low, high), a| (low.min(*a), high.max(*a)));
        assert!(low >= CURSOR_EDGE_LOW - 1e-4, "the edge of the cursor went down to {low}");
        assert!(high > 0.99 && high <= 1.0, "the edge of the cursor went up to {high}");
    }

    #[test]
    fn a_layer_stays_over_the_whole_view_through_its_drift() {
        for (_, depth) in LAYERS {
            for tenth in 0..=800 {
                let Val::Vw(left) = layer_left(depth, tenth as f32 * 0.1) else {
                    panic!("a layer is placed in parts of the view's width");
                };
                assert!(left <= 0.0 && left + LAYER_VW >= 100.0, "a layer at depth {depth} leaves a gap at {left}");
            }
        }
    }
}
