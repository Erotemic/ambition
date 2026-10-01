//! Generic platformer room presentation.
//!
//! [`PlatformerPresentationPlugin`] installs the gameplay camera, room/parallax
//! visuals, sprite animation, and player-visual scheduling. Session room visuals
//! can also be installed independently by hosts with their own camera stack.
//! HUD, menus, audio, dev overlays, and game-specific presentation remain host
//! responsibilities. Missing art falls back to the renderer's ordinary block
//! representation.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopeSet, SessionSpawnScope,
};
use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
use ambition_sprite_sheet::game_assets::GameAssets;

use crate::rendering::{
    spawn_parallax_layers, PlayerVisualSchedulePlugin,
    PresentationVisualAnimationPlugin,
};

/// System set for this plugin's one-shot host-resident `Startup` work, so a game
/// can order its own presentation setup against it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformerPresentationSetupSet;

/// The parallax memo: the session whose backdrop is spawned. A backdrop can
/// become available later than its room, and the room's static visuals do not
/// wait for it (`present_live_room_visuals`). See `sync_session_room_visuals`.
#[derive(Resource, Default)]
struct PresentedParallaxScope(Option<SessionScopeId>);

/// Per-session room presentation, independent of the provider. Each live room
/// gets its static visuals, stamped with that room, and a new session scope
/// gets the live room's parallax layers once, owned by that scope.
///
/// `PlatformerPresentationPlugin` includes it. A host with its own camera and
/// presentation stack (the Ambition shell host) adds only this plugin.
pub struct SessionRoomVisualsPlugin;

impl Plugin for SessionRoomVisualsPlugin {
    fn build(&self, app: &mut App) {
        // Room visuals include world labels, so this composition also installs their layout pass.
        // `WorldLabelLayoutPlugin` is idempotent.
        app.add_plugins(crate::rendering::WorldLabelLayoutPlugin);
        // Room/parallax passes consume the resolved quality budget; install its idempotent owner.
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        // The loader's outcome, read by presentation: see `ParallaxThemeAttempts`.
        app.init_resource::<crate::rendering::ParallaxThemeAttempts>();
        app.add_systems(
            Update,
            (crate::rendering::present_live_room_visuals, sync_session_room_visuals)
                .in_set(SessionScopeSet::Presentation),
        );
        // The composition that spawns blocks also applies authored per-block art overrides.
        app.add_systems(Update, crate::rendering::apply_block_art);
        app.add_systems(Update, crate::rendering::build_filled_ground_meshes);
        // The host tells portal presentation what it draws. That crate sees only the
        // decomposed scene body and the affordance body, so an ordinary NPC behind an
        // aperture is invisible to it unless published here.
        //
        // Gated on a portal existing: this writes a component per drawable per frame.
        //
        // It reads `Sprite::custom_size` and `Anchor`, which the animators rewrite
        // per frame. So it runs after the animators (final geometry) and before
        // `PortalPresentationSet` (so the compositor reads this frame's rectangle).
        // The explicit edge also puts the command flush between them.
        #[cfg(feature = "portal_render")]
        app.add_systems(
            Update,
            crate::rendering::portal_compositing::publish_portal_compositing_candidates
                // After every writer of the sprite basis. Candidates are
                // `FeatureVisual` or `PlayerVisual`, so the NPC half needs
                // `animate_characters` and `animate_feature_sprites` too.
                // `sync_visuals` writes the pose; the animators write the basis.
                .after(crate::rendering::actors::sync_visuals)
                .after(crate::rendering::actors::animate_player)
                .after(crate::rendering::actors::animate_characters)
                .after(crate::rendering::actors::animate_feature_sprites)
                // After every body-owned drawable writer (clock bar, flash
                // silhouette, ball, wire), as one set edge. The set edge is also the
                // command flush, so a drawable spawned this frame is a candidate this
                // frame. See `BodyOwnedDrawableSync`.
                .after(crate::rendering::BodyOwnedDrawableSync)
                .before(ambition_portal2d_presentation::PortalPresentationSet)
                .run_if(bevy::prelude::any_with_component::<
                    ambition_portal2d_presentation::PlacedPortal,
                >),
        );
        // The layer-spawning composition owns active-room theme loading and the refresh that
        // materializes newly available/quality-changed parallax assets.
        app.add_systems(
            Update,
            (
                crate::rendering::ensure_active_room_parallax_theme,
                crate::rendering::refresh_parallax_layers_on_quality_change,
            )
                .chain()
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Each live view owns its own parallax layers because placement depends on that view's
        // camera and viewport. Chain mirror -> sync so spawned/re-keyed copies flush before sync,
        // and run after refresh so quality-driven respawns have settled. No session-world guard is
        // needed; these systems operate only on the view/layer entities that exist.
        app.add_systems(
            Update,
            (
                crate::rendering::mirror_parallax_layers_per_view,
                crate::rendering::sync_parallax_layers,
            )
                .chain()
                .after(crate::rendering::camera_follow)
                .after(crate::rendering::refresh_parallax_layers_on_quality_change),
        );
    }
}

/// See the module docs. The generic platformer presentation: a camera, the room's
/// static visuals, and the sprite/animation chain.
pub struct PlatformerPresentationPlugin;

impl Plugin for PlatformerPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.add_systems(
            Startup,
            (spawn_main_camera, spawn_initial_room_visuals)
                .chain()
                .in_set(PlatformerPresentationSetupSet),
        );
        app.add_plugins(SessionRoomVisualsPlugin);
        // A camera for each view the live-room split opens (V5).
        app.add_systems(Update, present_split_view_rigs);
        // Room-transition requests rebuild parallax through the presentation animation plugin.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
        ));
    }
}

/// The camera the live-room split gave one of its views (V5).
#[derive(Component, Clone, Copy, Debug)]
pub struct SplitViewCamera;

/// Give each view the live-room split opened ([`ambition_sim_view::SplitForLiveRoom`])
/// one camera, and retire the camera when its view closes.
///
/// The camera is the gameplay rig the single-view host spawns: the world and
/// parallax layers. Its own room band (V3) and view band are added by the
/// isolation passes. Its order is above the host's gameplay rig (0) and below
/// the front HUD (9), one per view id, so two rigs never share an order.
pub fn present_split_view_rigs(
    mut commands: Commands,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::SplitForLiveRoom>>,
    rigs: Query<(Entity, &ambition_sim_view::PresentsView), With<SplitViewCamera>>,
) {
    for (rig, presents) in &rigs {
        if !views.contains(presents.0) {
            commands.entity(rig).try_despawn();
        }
    }
    for (view, id) in &views {
        if rigs.iter().any(|(_, presents)| presents.0 == view) {
            continue;
        }
        commands.spawn((
            Camera2d,
            Camera {
                order: 1 + isize::from(id.0.min(7)),
                ..default()
            },
            MainCamera,
            bevy::camera::visibility::RenderLayers::layer(0)
                .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER),
            ambition_sim_view::PresentsView(view),
            SplitViewCamera,
            Name::new(format!("Split view camera {}", id.0)),
        ));
    }
}

/// Spawn the single-view gameplay camera and full-screen front-UI camera.
///
/// Gameplay may occupy a viewport, while UI targets the full display. Multi-view compositions own
/// their gameplay rigs and use this path only for the unambiguous full-screen UI camera.
fn spawn_main_camera(
    mut commands: Commands,
    // Bind camera→view at spawn so view ownership is composition state, not a per-frame guess.
    views: Query<Entity, With<ambition_sim_view::LocalView>>,
) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER);
    // This plugin owns exactly one gameplay rig. Spawn it only when `ViewsOnHand` can identify one
    // view; multi-view callers use `ambition_sim_view::compose_local_views` and must not receive an
    // extra unbound full-screen gameplay camera.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter());
    match on_hand.presented_by(None) {
        Some(view) => {
            let camera = commands
                .spawn((
                    Camera2d,
                    MainCamera,
                    layers,
                    ambition_sim_view::PresentsView(view),
                    Name::new("Main Camera"),
                ))
                .id();
            // The shared publisher rejects a second main-camera record rather than choosing a winner.
            ambition_platformer2d_shared_tangle::camera_layers::publish_main_camera(
                &mut commands,
                camera,
            );
        }
        None => bevy::log::info_once!(
            "several local views exist, so the shared presentation plugin spawned no              gameplay camera: the composition that asked for those views owns their rigs              (`ambition_sim_view::compose_local_views`)."
        ),
    }

    commands.spawn((
        Camera2d,
        Camera {
            order: 9,
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        ambition_platformer2d_shared_tangle::camera_layers::FrontHudCamera,
        bevy::ui::IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(
            ambition_platformer2d_shared_tangle::camera_layers::FRONT_HUD_LAYER,
        ),
        Name::new("Front HUD Camera"),
    ));
}

/// Spawn the active room's parallax once for legacy hosts that do not install
/// the gameplay-session lifecycle. Shell hosts wait for a real session
/// activation. The room's static visuals are `present_live_room_visuals`'s,
/// for every host.
fn spawn_initial_room_visuals(
    mut commands: Commands,
    room_set: Option<ambition_platformer2d_world::rooms::SoleLiveRoomSpec>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    if active_session.is_some() {
        return;
    }
    // No world installed (a minimal test app) → nothing to draw, and that is not
    // an error: the same shape every optional-resource system in the engine uses.
    let Some(room_set) = room_set else {
        return;
    };
    let spec = room_set.spec();
    spawn_parallax_layers(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
}

/// Materialize the active session's parallax exactly once. The scope is
/// captured before any spawn request, so route retirement owns every parallax
/// entity created here. The room's static visuals are
/// `present_live_room_visuals`'s.
fn sync_session_room_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    mut parallax_presented: ResMut<PresentedParallaxScope>,
    room_set: Option<ambition_platformer2d_world::rooms::SoleLiveRoomSpec>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    // What the theme loader has already tried and found empty — the difference
    // between "not yet" and "never", which this system cannot derive alone.
    attempts: Option<Res<crate::rendering::ParallaxThemeAttempts>>,
) {
    let Some(active_session) = active_session else {
        return;
    };
    let current = active_session.current();
    let Some(scope) = current else {
        parallax_presented.0 = None;
        return;
    };
    if parallax_presented.0 == Some(scope) {
        return;
    }
    let Some(room_set) = room_set else {
        // Keep the scope unpresented so a provider that publishes its world on a
        // later frame is retried rather than permanently skipped.
        return;
    };
    let spec = room_set.spec();

    // The room and its backdrop become available at different times.
    // `spawn_parallax_layers` returns early when `GameAssets` has no layers for
    // the theme. `GameAssets` loads one theme at startup;
    // `ensure_active_room_parallax_theme` lazy-loads the others. The room's
    // static visuals are another system's, so they do not wait on the backdrop.
    let spawn_scope = SessionSpawnScope::scoped(scope);

    let wants_parallax = quality
        .as_deref()
        .map(|q| q.budget.parallax.enabled)
        .unwrap_or(true);
    if wants_parallax {
        let theme =
            ambition_sprite_sheet::game_assets::ParallaxTheme::from_room_metadata(&spec.metadata);
        let theme_loaded = assets.as_deref().is_some_and(|a| {
            ambition_sprite_sheet::game_assets::ParallaxLayerAsset::ALL
                .iter()
                .any(|layer| a.parallax_layers.get(theme, *layer).is_some())
        });
        if !theme_loaded {
            // The loader closes a theme after it tries it, so "not loaded" is
            // either "not yet" (retry next frame) or "resolved to nothing" (do not
            // retry).
            let nothing_is_coming = attempts
                .as_deref()
                .is_some_and(|attempts| attempts.attempted_without_art(theme));
            if !nothing_is_coming {
                // Leave only the parallax memo unset so the next frame retries.
                // The room is already on screen.
                return;
            }
            // Settled with no layers to spawn: this room's theme legitimately has
            // no art on this asset profile.
            parallax_presented.0 = Some(scope);
            return;
        }
    }

    // Settled either way: a tier that disables parallax, or a room whose theme
    // legitimately has no art, is finished rather than retried every frame.
    parallax_presented.0 = Some(scope);
    spawn_parallax_layers(
        &mut commands,
        spawn_scope,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
    use ambition_platformer2d_world::rooms::RoomSet;

    /// One room that asks for a parallax theme that no `GameAssets` provides.
    /// This is the state on the activation frame, before
    /// `ensure_active_room_parallax_theme` loads the theme.
    fn room_set_wanting_a_theme() -> RoomSet {
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "late_theme_room",
            ambition_platformer2d_core::World::new(
                "late_theme_room",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.visual_profile.parallax_theme = Some("a_theme_nobody_loaded".to_string());
        RoomSet::from_parts_or_panic("late_theme_room", vec![room], Vec::new())
    }

    /// How many room visuals this session has on screen.
    ///
    /// The regression tests count entities, because a drawn room is the
    /// property under test. A memo assertion would pass against a build that
    /// remembers a room it never drew.
    fn room_visuals(app: &mut App) -> usize {
        let mut query = app.world_mut().query_filtered::<(), (
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomVisual>,
            With<ambition_platformer2d_shared_tangle::lifecycle::SessionScopedEntity>,
        )>();
        query.iter(app.world()).count()
    }

    fn app_with_an_active_session() -> (App, SessionScopeId) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(
            Update,
            (crate::rendering::present_live_room_visuals, sync_session_room_visuals),
        );

        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        let rooms = room_set_wanting_a_theme();
        // The live room root names which room of the set it is (OW1 cut 5e).
        let definition = rooms.activation_definition();
        app.world_mut().spawn((SessionRoot(scope), rooms));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope),
            definition,
        ));
        (app, scope)
    }

    /// A late backdrop must not withhold the room.
    ///
    /// At every tier above Potato the budget wants parallax, so a room whose
    /// theme has not arrived must still present its static visuals and authored
    /// entities. Potato disables parallax, so it cannot show this failure.
    /// The room presents on the first frame it can; only parallax retries.
    #[test]
    fn the_room_presents_even_though_its_parallax_theme_has_not_arrived() {
        let (mut app, _) = app_with_an_active_session();
        // No `GameAssets`, so no theme can load. A new session starts this way.
        app.update();

        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            None,
            "and parallax must stay unsettled so a later theme is still retried",
        );
        assert!(
            room_visuals(&mut app) > 0,
            "the room must have actually DRAWN — the memo above is the system's \
             note to itself, and asserting only the memo would pass against a \
             build with the `spawn_room_visuals` call deleted",
        );
    }

    /// A room whose only content is one authored NPC, plus the geometry the
    /// stand-in needs to place a rectangle in.
    fn room_set_with_one_npc(npc_id: &str) -> RoomSet {
        use ambition_entity_catalog::placements::{
            InteractableSpec, InteractionKindSpec, PlacementSchema,
        };
        let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
            "npc_room",
            ambition_platformer2d_core::World::new(
                "npc_room",
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        // The parallax theme is missing, which is the condition under test.
        room.metadata.visual_profile.parallax_theme = Some("a_theme_nobody_loaded".to_string());
        room.placements
            .push(ambition_platformer2d_world::placements::PlacementRecord::new(
                npc_id,
                PlacementSchema::Interactable(InteractableSpec::new(
                    "Talk",
                    InteractionKindSpec::Npc {
                        character_id: None,
                        dialogue_id: None,
                        patrol_radius: 0.0,
                        patrol_path_id: None,
                        brain_override: None,
                    },
                )),
                ambition_platformer2d_core::Aabb::new(
                    ambition_platformer2d_core::Vec2::new(64.0, 64.0),
                    ambition_platformer2d_core::Vec2::new(8.0, 16.0),
                ),
            ));
        RoomSet::from_parts_or_panic("npc_room", vec![room], Vec::new())
    }

    fn a_view() -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: ambition_platformer2d_core::Vec2::new(64.0, 64.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 32.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            visible: true,
            submerged: false,
            wire_anchor: None,
            grab_reach: None,
            line_anchor: None,
            limb_host: None,
            depth_plane: Default::default(),
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 10,
            hp_max: 10,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    fn placeholders(app: &mut App) -> Vec<String> {
        let mut query = app
            .world_mut()
            .query_filtered::<&crate::rendering::FeatureVisual, With<
                crate::rendering::UnclaimedBodyPlaceholder,
            >>();
        query
            .iter(app.world())
            .map(|visual| visual.id.clone())
            .collect()
    }

    /// An authored room NPC must never wear the unclaimed-body placeholder.
    ///
    /// `draw_unclaimed_feature_views` draws a magenta stand-in for any
    /// `FeatureViewIndex` row that nothing claims for
    /// `UNCLAIMED_STAND_IN_GRACE_FRAMES` (5) frames. If the room's visuals
    /// waited behind a missing parallax theme, every NPC would get the
    /// stand-in.
    ///
    /// The control arm is required. `a_body_the_room_never_authored` has a view
    /// row and no placement, so it must get a stand-in. That proves the grace
    /// clock ran and the draw path was live; without it, a build that never draws
    /// stand-ins would also pass.
    #[test]
    fn an_authored_room_npc_never_wears_the_unclaimed_placeholder() {
        const NPC: &str = "npc_room_greeter";
        const NOBODYS: &str = "a_body_the_room_never_authored";

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.init_resource::<crate::rendering::UnclaimedFeatureViews>();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            (NPC.to_string(), a_view()),
            (NOBODYS.to_string(), a_view()),
        ]));
        app.add_systems(
            Update,
            (
                crate::rendering::present_live_room_visuals,
                sync_session_room_visuals,
                crate::rendering::draw_unclaimed_feature_views,
            )
                .chain(),
        );

        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        let room_set = room_set_with_one_npc(NPC);
        let geometry =
            ambition_platformer2d_core::RoomGeometry(room_set.activation_spec().world.clone());
        let definition = room_set.activation_definition();
        app.world_mut().spawn((SessionRoot(scope), room_set));
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope),
            geometry,
            definition,
        ));

        // Two frames past the grace period, so a late placeholder is still caught.
        for _ in 0..(5 + 2) {
            app.update();
        }

        let standing = placeholders(&mut app);
        assert!(
            standing.contains(&NOBODYS.to_string()),
            "the CONTROL failed: a view row nothing draws must get a stand-in, or \
             this test proves nothing about the NPC. Got {standing:?}"
        );
        assert!(
            !standing.contains(&NPC.to_string()),
            "an authored room NPC wore the placeholder — the room spawner did not \
             claim it within {} frames. Got {standing:?}",
            5
        );
    }

    /// Parallax stays unsettled on every frame while the theme is missing.
    #[test]
    fn parallax_keeps_retrying_while_the_theme_is_missing() {
        let (mut app, _) = app_with_an_active_session();
        app.update();
        app.update();
        app.update();

        assert!(room_visuals(&mut app) > 0, "the room is drawn");
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            None,
            "a missing theme must never settle the parallax memo",
        );
    }

    /// A theme that the loader tried and found empty is settled, not retried.
    ///
    /// This happens in shipped profiles: `WebStatic` / `BundledStatic` attempt an
    /// optional image only when it has an authored embedded candidate. The
    /// generated parallax manifest has entries without one, so the load yields
    /// zero handles and the loader closes the theme.
    ///
    /// Compare with `parallax_keeps_retrying_while_the_theme_is_missing`: the same
    /// missing theme gives the opposite answer, decided by whether anything is
    /// still coming. A system that settled unconditionally would pass one test and
    /// fail the other.
    #[test]
    fn a_theme_the_loader_resolved_to_nothing_settles_instead_of_retrying() {
        let (mut app, scope) = app_with_an_active_session();
        let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::from_room_metadata(
            &room_set_wanting_a_theme().activation_spec().metadata,
        );
        let mut attempts = crate::rendering::ParallaxThemeAttempts::default();
        attempts.without_art.push(theme);
        app.insert_resource(attempts);

        app.update();
        app.update();

        assert!(room_visuals(&mut app) > 0, "the room still presents");
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            Some(scope),
            "nothing is coming, so parallax is settled rather than re-asked every \
             frame for the life of the session",
        );
    }

    /// View half, cut V4a: each live room gets its own static visuals, stamped
    /// with that room. Two live rooms: each has visuals, and one presentation
    /// marker. A second frame adds nothing. When one room's visuals are taken
    /// (as its retirement takes them), that room alone is drawn again. The
    /// control: one live room is drawn once.
    #[test]
    fn each_live_room_gets_its_own_room_visuals() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let room = |id: &str, width: f32| {
            let mut spec = ambition_platformer2d_world::rooms::RoomSpec::new(
                id,
                ambition_platformer2d_core::World::new(
                    id,
                    ambition_platformer2d_core::Vec2::new(width, 480.0),
                    ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                    Vec::new(),
                ),
            );
            spec.metadata.visual_profile.parallax_theme = Some("a_theme_nobody_loaded".to_string());
            spec
        };
        let set = || RoomSet::from_parts_or_panic("left", vec![room("left", 640.0), room("right", 320.0)], Vec::new());
        let build = |rooms: usize| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.init_resource::<PresentedParallaxScope>();
            app.init_resource::<PhysicsSandboxSettings>();
            app.add_systems(
                Update,
                (crate::rendering::present_live_room_visuals, sync_session_room_visuals),
            );
            let mut active = ActiveSessionScope::default();
            let scope = active.begin();
            app.insert_resource(active);
            let set = set();
            let definitions = [set.activation_definition(), set.definition_by_id("right").expect("right")];
            app.world_mut().spawn((SessionRoot(scope), set));
            let mut live = LiveRoomInstance::ACTIVATION;
            for definition in definitions.into_iter().take(rooms) {
                app.world_mut()
                    .spawn(ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope))
                    .insert((live, definition));
                live = live.next();
            }
            app
        };
        // Per live room: the visuals stamped with it, and its markers.
        let drawn = |app: &mut App| {
            let mut visuals = app.world_mut().query_filtered::<(&InRoomInstance, Has<crate::rendering::PresentedRoomVisuals>), With<
                ambition_platformer2d_shared_tangle::lifecycle::RoomVisual,
            >>();
            let mut per_room = std::collections::BTreeMap::<LiveRoomInstance, (usize, usize)>::new();
            for (room, marker) in visuals.iter(app.world()) {
                let entry = per_room.entry(room.0).or_default();
                if marker {
                    entry.1 += 1;
                } else {
                    entry.0 += 1;
                }
            }
            per_room
        };

        let mut app = build(1);
        app.update();
        app.update();
        let one = drawn(&mut app);
        assert!(
            matches!(one.get(&LiveRoomInstance::ACTIVATION), Some((visuals, 1)) if *visuals > 0) && one.len() == 1,
            "control: one live room is not drawn once: {one:?}"
        );

        let mut app = build(2);
        app.update();
        app.update();
        let two = drawn(&mut app);
        let other = LiveRoomInstance::ACTIVATION.next();
        assert!(
            two.len() == 2 && two.values().all(|(visuals, markers)| *visuals > 0 && *markers == 1),
            "each live room is not drawn once: {two:?}"
        );
        // The right room retires its visuals; it alone is drawn again.
        let taken: Vec<Entity> = {
            let mut stamped = app.world_mut().query::<(Entity, &InRoomInstance)>();
            stamped.iter(app.world()).filter(|(_, room)| room.0 == other).map(|(entity, _)| entity).collect()
        };
        for entity in taken {
            app.world_mut().despawn(entity);
        }
        app.update();
        assert_eq!(drawn(&mut app), two, "a room whose visuals were taken was not drawn again, or another room was drawn twice");
    }

    /// Non-vacuity check for the tests above: a tier that wants no parallax
    /// settles both memos on frame one. If the room memo could never settle, the
    /// first test would pass for the wrong reason.
    #[test]
    fn a_tier_that_wants_no_parallax_settles_both_memos_at_once() {
        let (mut app, scope) = app_with_an_active_session();
        let mut quality = crate::quality::ResolvedVisualQuality::default();
        quality.budget.parallax.enabled = false;
        app.insert_resource(quality);
        app.update();

        assert!(room_visuals(&mut app) > 0, "the room is drawn");
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            Some(scope),
            "nothing is coming, so parallax is finished rather than retried every frame",
        );
    }

    /// View half, cut V5: each view the live-room split opened gets one
    /// camera, which presents it and goes when it closes. The host's own view
    /// gets none from this pass; its rig is the host's.
    #[test]
    fn each_view_the_split_opened_gets_one_camera_until_it_closes() {
        use bevy::ecs::system::RunSystemOnce as _;
        let mut world = World::new();
        world.spawn((ambition_sim_view::LocalView, ambition_sim_view::LocalViewId(0)));
        let opened = world
            .spawn((
                ambition_sim_view::LocalView,
                ambition_sim_view::LocalViewId(1),
                ambition_sim_view::SplitForLiveRoom,
            ))
            .id();
        let rigs = |world: &mut World| {
            world
                .query_filtered::<&ambition_sim_view::PresentsView, With<SplitViewCamera>>()
                .iter(world)
                .map(|presents| presents.0)
                .collect::<Vec<_>>()
        };
        world.run_system_once(present_split_view_rigs).expect("the pass runs");
        world.run_system_once(present_split_view_rigs).expect("the pass runs");
        assert_eq!(rigs(&mut world), vec![opened], "the opened view has not exactly one camera");
        world.entity_mut(opened).despawn();
        world.run_system_once(present_split_view_rigs).expect("the pass runs");
        assert_eq!(rigs(&mut world), Vec::new(), "a camera outlived the view it presented");
    }
}
