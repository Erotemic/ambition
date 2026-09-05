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
use ambition_platformer2d_world::rooms::RoomSet;
use ambition_sprite_sheet::game_assets::GameAssets;

use crate::rendering::{
    spawn_parallax_layers, spawn_room_visuals, PlayerVisualSchedulePlugin,
    PresentationVisualAnimationPlugin,
};

/// System set for this plugin's one-shot host-resident `Startup` work, so a game
/// can order its own presentation setup against it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformerPresentationSetupSet;

#[derive(Resource, Default)]
struct PresentedSessionScope(Option<SessionScopeId>);

/// The parallax half of the presentation memo, separate because a backdrop can
/// become possible LATER than the room it sits behind. See
/// `sync_session_room_visuals`.
#[derive(Resource, Default)]
struct PresentedParallaxScope(Option<SessionScopeId>);

/// The provider-agnostic per-session room presentation: whenever a fresh session scope goes
/// live, spawn the active `RoomSet` room's parallax layers and static visuals exactly once,
/// owned by that scope.
///
/// `PlatformerPresentationPlugin` includes it; a host with its own camera and
/// presentation stack (the Ambition shell host) adds JUST this plugin.
pub struct SessionRoomVisualsPlugin;

impl Plugin for SessionRoomVisualsPlugin {
    fn build(&self, app: &mut App) {
        // Room visuals include world labels, so this composition also installs their layout pass.
        // `WorldLabelLayoutPlugin` is idempotent.
        app.add_plugins(crate::rendering::WorldLabelLayoutPlugin);
        // Room/parallax passes consume the resolved quality budget; install its idempotent owner.
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        // The loader's OUTCOME, read by presentation: see `ParallaxThemeAttempts`.
        app.init_resource::<crate::rendering::ParallaxThemeAttempts>();
        app.add_systems(
            Update,
            sync_session_room_visuals.in_set(SessionScopeSet::Presentation),
        );
        // The composition that spawns blocks also applies authored per-block art overrides.
        app.add_systems(Update, crate::rendering::apply_block_art);
        // ⭐ THE HOST TELLS PORTAL PRESENTATION WHAT IT DRAWS. That crate's body
        // seams are the ONE decomposed scene body and the affordance body, so an
        // ordinary NPC behind an aperture is invisible to it while this crate
        // draws it above every pane.
        //
        // ⚠ GATED ON A PORTAL EXISTING, so a room with none does no work at all:
        // this writes a component per drawable per frame, and paying that in
        // every portal-free room would be a cost with no reader.
        #[cfg(feature = "portal_render")]
        app.add_systems(
            Update,
            crate::rendering::portal_compositing::publish_portal_compositing_candidates
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
        // Room-transition requests rebuild visuals through the presentation animation plugin.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
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

/// Spawn the active room once for legacy hosts that do not install the
/// gameplay-session lifecycle. Shell hosts wait for a real session activation.
fn spawn_initial_room_visuals(
    mut commands: Commands,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
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
    let spec = room_set.active_spec();
    spawn_parallax_layers(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        spec,
        *physics_settings,
        assets.as_deref(),
    );
}

/// Materialize the active session's room presentation exactly once. The scope
/// is captured before any spawn request, so route retirement owns every static
/// visual and parallax entity created here.
fn sync_session_room_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    mut presented: ResMut<PresentedSessionScope>,
    mut parallax_presented: ResMut<PresentedParallaxScope>,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
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
        presented.0 = None;
        parallax_presented.0 = None;
        return;
    };
    if presented.0 == Some(scope) && parallax_presented.0 == Some(scope) {
        return;
    }
    let Some(room_set) = room_set else {
        // Keep the scope unpresented so a provider that publishes its world on a
        // later frame is retried rather than permanently skipped.
        return;
    };
    let spec = room_set.active_spec();

    // ⛔⛔ TWO MEMOS, BECAUSE THE ROOM AND ITS BACKDROP BECOME POSSIBLE AT
    // DIFFERENT TIMES. `spawn_parallax_layers` early-returns when `GameAssets`
    // has no layers for the room's theme, and `GameAssets` loads ONE theme at
    // startup — every other theme lazy-loads via
    // `ensure_active_room_parallax_theme`. A single memo forced a choice
    // between two wrong answers: set it first and a session that activated the
    // frame BEFORE its theme arrived got no parallax and was never retried
    // (one shot, and it missed); defer it and the whole room waited on a
    // backdrop.
    //
    // ⭐ THIS IS WHY AMBITION HAD A BACKDROP AND SMASH DID NOT, on the same
    // assets and the same theme (`Hub`). The sandbox draws through
    // `spawn_initial_room_visuals`, which has no memo and simply tries again
    // next frame; the session path is memoized and did not.
    //
    // ⛔⛔ AND DEFERRING BOTH WAS THE WORSE HALF, which is what this split
    // fixes: `!theme_loaded` used to `return` before `spawn_room_visuals`, so
    // at a tier that wants parallax a late theme withheld EVERY static visual
    // and every authored room entity — not merely the backdrop. Potato never
    // showed it because `parallax.enabled` is false there, so the gate never
    // engaged. ⇒ Room presentation may not depend on a backdrop becoming
    // resident. Parallax waits alone.
    let spawn_scope = SessionSpawnScope::scoped(scope);

    if presented.0 != Some(scope) {
        presented.0 = Some(scope);
        spawn_room_visuals(
            &mut commands,
            spawn_scope,
            spec,
            *physics_settings,
            assets.as_deref(),
        );
    }

    if parallax_presented.0 == Some(scope) {
        return;
    }
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
            // ⭐ ASK WHETHER ANYTHING IS STILL COMING. The loader closes a theme
            // once it has tried it, so "not loaded" splits in two: not YET, which
            // is worth another frame, and resolved-to-nothing, which is not.
            // Retrying the second one is a question re-asked every frame for the
            // life of the session against a loader that has stopped answering.
            let nothing_is_coming = attempts
                .as_deref()
                .is_some_and(|attempts| attempts.attempted_without_art(theme));
            if !nothing_is_coming {
                // Leave the PARALLAX memo unset so the next frame retries — and
                // only that one. The room is already on screen.
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

    /// One room asking for a parallax theme that no `GameAssets` provides —
    /// which is the state a session is in on the frame it activates, before
    /// `ensure_active_room_parallax_theme` has loaded the theme.
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
        RoomSet::from_parts("late_theme_room", vec![room], Vec::new())
    }

    /// How many actual room visuals this session has on screen.
    ///
    /// ⛔⛔ THE MEMO IS NOT THE PROPERTY. `PresentedSessionScope` is a note the
    /// system leaves for itself; the defect was that the ROOM DID NOT DRAW.
    /// Deleting the `spawn_room_visuals` call while leaving `presented.0 =
    /// Some(scope)` in place would satisfy every memo assertion in this file, so
    /// the regression below counts entities instead.
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
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PresentedParallaxScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(Update, sync_session_room_visuals);

        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        app.world_mut()
            .spawn((SessionRoot(scope), room_set_wanting_a_theme()));
        (app, scope)
    }

    /// ⛔⛔ A LATE BACKDROP MUST NOT WITHHOLD THE ROOM.
    ///
    /// `!theme_loaded` used to `return` before `spawn_room_visuals`, so at any
    /// tier whose budget wants parallax — every tier above Potato — a room whose
    /// theme had not arrived yet presented NOTHING: no static visuals, no
    /// authored room entities, not merely no backdrop. Potato hid it because
    /// `parallax.enabled` is false there, so the gate never engaged and the one
    /// tier anybody measured headless looked correct.
    ///
    /// ⇒ Two memos. The room presents on the first frame it can; parallax keeps
    /// the retry that it needed, alone.
    #[test]
    fn the_room_presents_even_though_its_parallax_theme_has_not_arrived() {
        let (mut app, scope) = app_with_an_active_session();
        // No `GameAssets` at all, so no theme can be loaded — the strongest form
        // of the condition, and the one a fresh session actually starts in.
        app.update();

        assert_eq!(
            app.world().resource::<PresentedSessionScope>().0,
            Some(scope),
            "the room must be presented on the frame it activates, whatever the backdrop is doing",
        );
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
        // The parallax theme is still missing, because that is the state the
        // regression lived in: a room that presents only after a backdrop.
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
        RoomSet::from_parts("npc_room", vec![room], Vec::new())
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

    /// ⭐⭐ THE PLAYER-VISIBLE SYMPTOM, PINNED: AN AUTHORED ROOM NPC MUST NEVER
    /// WEAR THE UNCLAIMED-BODY PLACEHOLDER.
    ///
    /// This is the defect the parallax gate produced, stated as what a player
    /// saw rather than as which system returned early. `draw_unclaimed_feature_views`
    /// draws a magenta stand-in for any `FeatureViewIndex` row nothing claimed,
    /// after `UNCLAIMED_STAND_IN_GRACE_FRAMES` (5) consecutive frames — and while
    /// `sync_session_room_visuals` withheld `spawn_room_visuals` behind a theme
    /// that had not loaded, every interactable NPC in every room above Potato was
    /// unclaimed for exactly that long. Potato hid it because parallax is
    /// disabled there, so the gate never engaged.
    ///
    /// ⛔ THE CONTROL ARM IS NOT OPTIONAL, and it is the second id. A test that
    /// only asserts "no placeholder" passes just as well against a build where
    /// the stand-in never draws at all — which is most of the ways this could be
    /// wrong. `a_body_the_room_never_authored` has a view row and NO placement,
    /// so it MUST get one, and its appearance is what proves the grace clock ran
    /// and the drawing path was live for the NPC too.
    #[test]
    fn an_authored_room_npc_never_wears_the_unclaimed_placeholder() {
        const NPC: &str = "npc_room_greeter";
        const NOBODYS: &str = "a_body_the_room_never_authored";

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PresentedSessionScope>();
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
            ambition_platformer2d_core::RoomGeometry(room_set.active_spec().world.clone());
        app.world_mut()
            .spawn((SessionRoot(scope), room_set, geometry));

        // Two frames past the grace period, so a placeholder that is merely LATE
        // still has time to appear and be caught.
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

    /// The retry the original single memo existed to provide, kept: parallax is
    /// unsettled while the theme is missing, on every frame, not just the first.
    #[test]
    fn parallax_keeps_retrying_while_the_theme_is_missing() {
        let (mut app, scope) = app_with_an_active_session();
        app.update();
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<PresentedSessionScope>().0,
            Some(scope)
        );
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            None,
            "a missing theme must never settle the parallax memo",
        );
    }

    /// ⭐ A THEME THE LOADER TRIED AND FOUND EMPTY IS FINISHED, NOT RETRIED.
    ///
    /// ⛔ THE BRANCH THIS COVERS WAS UNREACHABLE. The code below the gate says a
    /// room whose theme legitimately has no art "is finished rather than retried
    /// every frame" — and `!theme_loaded` returned before it could ever run, so
    /// the sentence described nothing. It is reachable in shipped profiles:
    /// `WebStatic` / `BundledStatic` attempt an optional image only when it has
    /// an authored embedded candidate, and the generated parallax manifest
    /// authors entries without one, so the load yields zero handles and the
    /// loader closes the theme.
    ///
    /// The difference between this and
    /// `parallax_keeps_retrying_while_the_theme_is_missing` is the whole point:
    /// same missing theme, opposite answer, decided by whether anything is still
    /// coming. Both are asserted, because a system that settled unconditionally
    /// would pass one of them and lose every late backdrop in the game.
    #[test]
    fn a_theme_the_loader_resolved_to_nothing_settles_instead_of_retrying() {
        let (mut app, scope) = app_with_an_active_session();
        let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::from_room_metadata(
            &room_set_wanting_a_theme().active_spec().metadata,
        );
        let mut attempts = crate::rendering::ParallaxThemeAttempts::default();
        attempts.attempted.push(theme);
        attempts.without_art.push(theme);
        app.insert_resource(attempts);

        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<PresentedSessionScope>().0,
            Some(scope),
            "the room still presents",
        );
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            Some(scope),
            "nothing is coming, so parallax is settled rather than re-asked every \
             frame for the life of the session",
        );
    }

    /// ⚠ NON-VACUITY, and the reason this test exists beside the two above: if
    /// the room memo could never settle, the first test would pass for the wrong
    /// reason. A tier that does not want parallax settles BOTH on frame one.
    #[test]
    fn a_tier_that_wants_no_parallax_settles_both_memos_at_once() {
        let (mut app, scope) = app_with_an_active_session();
        let mut quality = crate::quality::ResolvedVisualQuality::default();
        quality.budget.parallax.enabled = false;
        app.insert_resource(quality);
        app.update();

        assert_eq!(
            app.world().resource::<PresentedSessionScope>().0,
            Some(scope)
        );
        assert_eq!(
            app.world().resource::<PresentedParallaxScope>().0,
            Some(scope),
            "nothing is coming, so parallax is finished rather than retried every frame",
        );
    }
}
