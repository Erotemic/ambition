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
    BasicShellUiRoot, LauncherTab, ShellExperienceRegistry, ShellLaunchEntry, ShellLauncherState,
};
use ambition_platformer2d::menu::render::bevy_ui::{AmbitionMenuControlKind, BevyUiMenuPanel, BevyUiMenuTab};
use ambition_platformer2d::menu::{MenuControlKind, MenuVisualState};
use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::sprite_sheet::game_assets::ParallaxTheme;

/// The layers of a scene from the far one to the near one. How much each one
/// drifts is the look's (`TitleLook::layer_drift`).
const LAYER_IDS: [&str; 4] = ["sky", "far_backplate", "near_background", "foreground_atmosphere"];

/// The title screen's look: the drifting scene behind the menu and the
/// menu's window, as the host's pack states them (`data/title_look.ron`). An
/// edit is shown at the next start, with no Rust build.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleLook {
    /// How much each layer drifts, far to near ([`LAYER_IDS`]): the near
    /// ones more, as they do when a camera moves.
    pub layer_drift: [f32; 4],
    /// How wide a layer is, in parts of a hundred of the view's width. A
    /// layer is square, and a 16:9 view of this width shows the middle band
    /// of it, as the game's camera does.
    pub layer_vw: f32,
    /// How far the nearest layer drifts to each side, in the same unit.
    pub drift_vw: f32,
    /// The seconds of one drift there and back.
    pub drift_period_s: f32,
    /// The seconds one scene takes to fade into the next.
    pub fade_s: f32,
    /// The scrim over what is behind the menu (RGBA): the backdrop is the
    /// point of the screen, so it is dimmed only a little.
    pub scrim: [f32; 4],
    /// The menu's window (RGBA): dark glass over the backdrop.
    pub window: [f32; 4],
    /// The width of the window, in parts of a hundred of the view's height.
    /// Text is sized by the height of the view, so the window is too: a row
    /// keeps its shape on a view of any aspect.
    pub window_width_vh: f32,
    /// The most of the view's width the window takes (a tall, narrow view).
    pub window_most_percent: f32,
    pub window_radius_vh: f32,
    pub window_edge_vh: f32,
    pub window_edge: [f32; 4],
    pub row_radius_vh: f32,
    pub row_edge_vh: f32,
    /// The edge of a row the cursor is not on (RGBA).
    pub row_edge: [f32; 4],
    /// The edge of the cursor's row at its brightest (RGB). Its alpha goes
    /// between `cursor_edge_low` and 1 once each `cursor_breath_s`.
    pub cursor_edge: [f32; 3],
    pub cursor_edge_low: f32,
    pub cursor_breath_s: f32,
    /// How long the run-in's engine card holds before the title, if nobody
    /// presses confirm.
    pub engine_card_s: f32,
}

fn rgba([r, g, b, a]: [f32; 4]) -> Color {
    Color::srgba(r, g, b, a)
}

impl TitleLook {
    fn layers(&self) -> [(&'static str, f32); 4] {
        std::array::from_fn(|index| (LAYER_IDS[index], self.layer_drift[index]))
    }
    fn scrim(&self) -> Color {
        rgba(self.scrim)
    }
    fn window(&self) -> Color {
        rgba(self.window)
    }
    fn window_edge(&self) -> Color {
        rgba(self.window_edge)
    }
    fn row_edge(&self) -> Color {
        rgba(self.row_edge)
    }
    fn cursor_edge(&self) -> Color {
        let [r, g, b] = self.cursor_edge;
        Color::srgb(r, g, b)
    }

    /// Each value the title screen cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (field, value) in [
            ("drift_period_s", self.drift_period_s),
            ("fade_s", self.fade_s),
            ("cursor_breath_s", self.cursor_breath_s),
            ("engine_card_s", self.engine_card_s),
            ("window_width_vh", self.window_width_vh),
            ("window_most_percent", self.window_most_percent),
        ] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and more than 0"));
            }
        }
        for (field, value) in [
            ("drift_vw", self.drift_vw),
            ("window_radius_vh", self.window_radius_vh),
            ("window_edge_vh", self.window_edge_vh),
            ("row_radius_vh", self.row_radius_vh),
            ("row_edge_vh", self.row_edge_vh),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        // The window's inner corner is its radius less its edge.
        if self.window_edge_vh > self.window_radius_vh {
            problems.push(format!(
                "`window_edge_vh` is {}, more than `window_radius_vh` {}; the inner corner would be negative",
                self.window_edge_vh, self.window_radius_vh
            ));
        }
        // A layer wider than the view by less than its drift shows a gap at an edge.
        let deepest = self.layer_drift.iter().cloned().fold(0.0_f32, f32::max);
        if (self.layer_vw - 100.0) * 0.5 < deepest * self.drift_vw {
            problems.push(format!(
                "`layer_vw` {} leaves {} on each side, less than the deepest drift {}; a layer would show a gap",
                self.layer_vw,
                (self.layer_vw - 100.0) * 0.5,
                deepest * self.drift_vw
            ));
        }
        if !(0.0..=1.0).contains(&self.cursor_edge_low) {
            problems.push(format!("`cursor_edge_low` is {}; an alpha is from 0 to 1", self.cursor_edge_low));
        }
        let colours = [("scrim", &self.scrim[..]), ("window", &self.window[..]), ("window_edge", &self.window_edge[..]), ("row_edge", &self.row_edge[..]), ("cursor_edge", &self.cursor_edge[..])];
        for (field, channels) in colours {
            if channels.iter().any(|channel| !(0.0..=1.0).contains(channel)) {
                problems.push(format!("`{field}` is {channels:?}; each channel is from 0 to 1"));
            }
        }
        problems
    }
}

/// The authored FILE kind: the title screen's look.
pub const TITLE_LOOK_SCHEMA: &str = "title_look";

/// The schema, for the host pack's `content_pack! { schemas: [..] }`.
pub fn title_look_schema() -> ambition_platformer2d::content::SchemaRegistration {
    ambition_platformer2d::content::rules_schema(
        TITLE_LOOK_SCHEMA,
        1,
        "ambition_host",
        "The title screen's look: the drifting scene behind the launcher menu and the menu's window.",
        TitleLook::problems,
    )
}

/// The look the host's pack states.
pub fn title_look() -> TitleLook {
    *ambition_platformer2d::content::lowered_rules::<TitleLook>(super::host_pack::HOST_PACK.prepared(), TITLE_LOOK_SCHEMA)
        .expect("the host's pack states the title look (`data/title_look.ron`)")
}

/// The scene behind the row `entry`. `None` is the Exit row.
///
/// Each provider names the look of its row (`launcher_look`); this resolves
/// the name to a backdrop theme. A row with no look, or a look that is not a
/// theme, gets the hub. No route name is read.
fn scene_of(entry: Option<&ShellLaunchEntry>) -> ParallaxTheme {
    match entry {
        None => ParallaxTheme::Cove,
        Some(entry) => entry.look.as_deref().and_then(ParallaxTheme::from_key).unwrap_or(ParallaxTheme::Hub),
    }
}

/// The root of the backdrop: one for the title screen, behind the menu.
#[derive(Component)]
pub struct TitleBackdrop;

/// One scene of the backdrop. `shown` goes to `wanted` over `TitleLook::fade_s`; a
/// scene that is not wanted and not shown is taken away.
#[derive(Component)]
pub struct TitleScene {
    pub theme: ParallaxTheme,
    /// The folder of its images: the resolution of the quality tier it was
    /// made at.
    subdir: &'static str,
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
    let swing = (seconds * std::f32::consts::TAU / title_look().drift_period_s).sin();
    Val::Vw(-(title_look().layer_vw - 100.0) * 0.5 + depth * title_look().drift_vw * swing)
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
    // ⛔ No parallax: no backdrop. It returned and left the backdrop that
    // was there.
    if budget.is_some_and(|budget| !budget.enabled) {
        for root in &roots {
            commands.entity(root).try_despawn();
        }
        return;
    }
    let subdir = budget.map_or("backgrounds/parallax_layers", |budget| budget.resolution_scale.parallax_subdir());
    // The settings tab has no game under the cursor: the scene stays.
    let wanted = match launcher.tab {
        LauncherTab::Home => {
            let entries = registry.launch_entries();
            Some(scene_of(entries.iter().filter(|entry| entry.available).nth(launcher.selected)))
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

    // A scene of another quality tier is not the wanted scene: the scene of
    // the new tier fades in over it. It kept the images of the tier it was
    // made at.
    let mut present = false;
    for mut scene in &mut scenes {
        let is_wanted = scene.theme == wanted && scene.subdir == subdir;
        present |= is_wanted;
        scene.wanted = if is_wanted { 1.0 } else { 0.0 };
    }
    if present {
        return;
    }
    // The first scene of the screen is there at once: there is nothing to fade from.
    let shown = if scenes.is_empty() { 1.0 } else { 0.0 };
    *order += 1;
    let seconds = time.elapsed_secs();
    commands.entity(root).with_children(|root| {
        root.spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            // The newest scene is in front, so it fades in over the one before.
            ZIndex(*order),
            TitleScene { theme: wanted, subdir, shown, wanted: 1.0 },
            Name::new(format!("title scene: {}", wanted.key())),
        ))
        .with_children(|scene| {
            for (layer, depth) in title_look().layers() {
                let mut image = ImageNode::new(asset_server.load(format!("{subdir}/{}_{layer}.png", wanted.key())));
                image.color = Color::WHITE.with_alpha(smoothstep(shown));
                scene.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Vw(title_look().layer_vw),
                        aspect_ratio: Some(1.0),
                        left: layer_left(depth, seconds),
                        // Its middle on the middle of the view.
                        top: Val::Percent(50.0),
                        margin: UiRect::top(Val::Vw(-title_look().layer_vw * 0.5)),
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
    let step = time.delta_secs() / title_look().fade_s;
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
    let breath = 0.5 + 0.5 * (seconds * std::f32::consts::TAU / title_look().cursor_breath_s).sin();
    title_look().cursor_edge_low + (1.0 - title_look().cursor_edge_low) * breath
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
                if scrim.0 != title_look().scrim() {
                    scrim.0 = title_look().scrim();
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
                if fill.0 != title_look().window() {
                    fill.0 = title_look().window();
                }
            }
        }
        if let Ok(mut node) = nodes.get_mut(window) {
            let radius = BorderRadius::all(Val::Vh(title_look().window_radius_vh));
            let border = UiRect::all(Val::Vh(title_look().window_edge_vh));
            if node.width != Val::Vh(title_look().window_width_vh) || node.border_radius != radius || node.border != border {
                node.width = Val::Vh(title_look().window_width_vh);
                node.max_width = Val::Percent(title_look().window_most_percent);
                node.border_radius = radius;
                node.border = border;
            }
        }
        edge(window, title_look().window_edge());
    }

    // The two end tabs are in the top corners of the window: they take its
    // corners, inside its edge.
    let last_tab = tabs.iter().filter(|(tab, _)| in_the_launcher(*tab, 3, &parents, &roots)).map(|(_, tab)| tab.index).max();
    for (entity, tab) in &tabs {
        if !in_the_launcher(entity, 3, &parents, &roots) {
            continue;
        }
        let corner = Val::Vh(title_look().window_radius_vh - title_look().window_edge_vh);
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

    let cursor_edge = title_look().cursor_edge().with_alpha(cursor_edge_alpha(time.elapsed_secs()));
    for (entity, state, kind) in &rows {
        // A scrollbar is not a row.
        if matches!(kind.0, MenuControlKind::Scrollbar) || !in_the_launcher(entity, 3, &parents, &roots) {
            continue;
        }
        if let Ok(mut node) = nodes.get_mut(entity) {
            let radius = BorderRadius::all(Val::Vh(title_look().row_radius_vh));
            let border = UiRect::all(Val::Vh(title_look().row_edge_vh));
            if node.border_radius != radius || node.border != border {
                node.border_radius = radius;
                node.border = border;
            }
        }
        edge(entity, if state.focused && !state.disabled { cursor_edge } else { title_look().row_edge() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::persistence::settings::TextureResolutionScale;

    /// The look is the host pack's `title_look.ron`: a layer rests on the
    /// middle of the view by the file's width, and the cursor's edge goes
    /// down to the file's lowest alpha.
    #[test]
    fn the_title_look_is_the_host_packs_file() {
        let look = title_look();
        assert_eq!(layer_left(1.0, 0.0), Val::Vw(-(look.layer_vw - 100.0) * 0.5), "a layer at rest is not on the file's width");
        let trough = cursor_edge_alpha(look.cursor_breath_s * 0.75);
        assert!((trough - look.cursor_edge_low).abs() < 1e-4, "the cursor's edge went down to {trough}, not the file's {}", look.cursor_edge_low);
    }

    fn launcher_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default(), TitleScreenPlugin));
        app.init_asset::<Image>();
        app.insert_resource(ShellLauncherState { active: true, ..default() });
        app.init_resource::<ShellExperienceRegistry>();
        app.insert_resource(ResolvedVisualQuality::default());
        app
    }

    /// (folder, wanted) of each scene, sorted.
    fn scenes(app: &mut App) -> Vec<(&'static str, bool)> {
        let world = app.world_mut();
        let mut query = world.query::<&TitleScene>();
        let mut scenes: Vec<_> = query.iter(world).map(|scene| (scene.subdir, scene.wanted > 0.5)).collect();
        scenes.sort();
        scenes
    }

    fn backdrops(app: &mut App) -> usize {
        let world = app.world_mut();
        world.query_filtered::<(), With<TitleBackdrop>>().iter(world).count()
    }

    /// ⭐ B1: a new quality tier makes the scene again at its resolution,
    /// and with no parallax the backdrop goes.
    #[test]
    fn the_backdrop_follows_the_quality_while_the_launcher_is_up() {
        let mut app = launcher_app();
        app.world_mut().resource_mut::<ResolvedVisualQuality>().budget.parallax.resolution_scale =
            TextureResolutionScale::Full;
        app.update();
        let full = TextureResolutionScale::Full.parallax_subdir();
        assert_eq!(scenes(&mut app), vec![(full, true)], "the premise: one scene");

        let quarter = TextureResolutionScale::Quarter.parallax_subdir();
        app.world_mut().resource_mut::<ResolvedVisualQuality>().budget.parallax.resolution_scale =
            TextureResolutionScale::Quarter;
        app.update();
        let mut expected = vec![(full, false), (quarter, true)];
        expected.sort();
        assert_eq!(scenes(&mut app), expected, "the scene of the new tier comes in over the old");

        app.world_mut().resource_mut::<ResolvedVisualQuality>().budget.parallax.enabled = false;
        app.update();
        assert_eq!(backdrops(&mut app), 0, "no parallax: no backdrop");
    }

    /// ⭐ IN7: a provider with a route name the host has never seen gets the
    /// look it names. The scene was chosen from route-name prefixes, so a new
    /// provider got the hub until this host was edited. The control: with no
    /// look, or a look that is no theme, the row gets the hub; the Exit row
    /// gets the cove.
    #[test]
    fn a_provider_names_the_look_of_its_row() {
        let entry = |look: Option<&str>| ShellLaunchEntry {
            route_id: "zz_new_route".into(),
            label: "New".into(),
            description: String::new(),
            available: true,
            unavailable_reason: None,
            look: look.map(str::to_owned),
        };
        assert_eq!(scene_of(Some(&entry(Some("open_sky")))), ParallaxTheme::OpenSky, "the look it names");
        assert_eq!(scene_of(Some(&entry(Some("starlight")))), ParallaxTheme::Starlight, "the look it names");
        assert_eq!(scene_of(Some(&entry(None))), ParallaxTheme::Hub, "the control: no look");
        assert_eq!(scene_of(Some(&entry(Some("no_such_look")))), ParallaxTheme::Hub, "the control: no theme");
        assert_eq!(scene_of(None), ParallaxTheme::Cove, "the Exit row");
    }

    #[test]
    fn the_edge_of_the_cursor_breathes_and_never_goes_out() {
        let alphas: Vec<f32> = (0..=180).map(|hundredth| cursor_edge_alpha(hundredth as f32 * 0.01)).collect();
        let (low, high) = alphas.iter().fold((f32::MAX, f32::MIN), |(low, high), a| (low.min(*a), high.max(*a)));
        assert!(low >= title_look().cursor_edge_low - 1e-4, "the edge of the cursor went down to {low}");
        assert!(high > 0.99 && high <= 1.0, "the edge of the cursor went up to {high}");
    }

    #[test]
    fn a_layer_stays_over_the_whole_view_through_its_drift() {
        for (_, depth) in title_look().layers() {
            for tenth in 0..=800 {
                let Val::Vw(left) = layer_left(depth, tenth as f32 * 0.1) else {
                    panic!("a layer is placed in parts of the view's width");
                };
                assert!(left <= 0.0 && left + title_look().layer_vw >= 100.0, "a layer at depth {depth} leaves a gap at {left}");
            }
        }
    }
}
