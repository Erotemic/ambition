//! Startup presentation diagnostics for the real product shell.
//!
//! This is intentionally a READ-ONLY developer probe. It does not fix, gate, or
//! reorder simulation/presentation. The point is to observe the exact product
//! path that can produce a bad first visible frame and answer two independent
//! questions with one timeline:
//!
//! 1. Was the primary actor already drawable before its sim-derived
//!    [`BodyPoseView`] / sheet-authored geometry existed?
//! 2. Did the main camera still present an old/default projection before the
//!    current local view had a resolved camera snapshot?
//!
//! Samples are taken in `Last`, after the ordinary main-world schedules have
//! run and immediately before Bevy hands the world to rendering. The probe is
//! opt-in from the F3 developer UI: when disabled it does no frame sampling or
//! pre-roll collection. When enabled, a newly active session arms a bounded
//! startup trace; stderr output is a separate opt-in switch.

use std::collections::VecDeque;

use ambition_platformer2d::actor::{BodyKinematics, BodyPoseView};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
use ambition_platformer2d::platformer::camera_layers::MainCamera;
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionRoot,
};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::render::rendering::actors::PlayerSpriteCharacter;
use ambition_platformer2d::render::rendering::PlayerSpriteBaseline;
use ambition_platformer2d::presentation::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d::rollback::{RollbackExecutionStats, RollbackFrameCount};
use ambition_platformer2d::sim::SimTick;
use ambition_platformer2d::sim_view::{
    CameraViewState, FeatureViewIndex, LocalView, PresentedPose, PresentsView,
};
use ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot;
use ambition_platformer2d::sprite_sheet::character::CharacterAnimator;
use bevy::camera::Projection;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_inspector_egui::bevy_egui::{
    egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext,
};

const STARTUP_TRACE_FRAMES: u32 = 180;
const PRE_ROLL_FRAMES: usize = 12;
const ALWAYS_LOG_FIRST_SESSION_FRAMES: u64 = 24;
const TRACE_CAPACITY: usize = 180;

#[derive(Clone, Debug, PartialEq)]
struct PoseProbe {
    size: [f32; 2],
    base_size: [f32; 2],
    authored_render: Option<[f32; 2]>,
    authored_offset: Option<[f32; 2]>,
    hp_current: i32,
    hp_max: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct SpriteProbe {
    custom_size: Option<[f32; 2]>,
    transform_scale: [f32; 3],
    transform_translation: [f32; 3],
    image: String,
    atlas: Option<(String, usize)>,
    anchor: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct AnimatorProbe {
    current: String,
    frame: usize,
    trimmed: bool,
    render_basis: Option<[f32; 2]>,
    current_render: Option<[f32; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
struct BaselineProbe {
    standing_render: [f32; 2],
    standing_collision: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
struct PlayerProbe {
    entity: u64,
    has_player_entity: bool,
    has_player_visual: bool,
    worn: Option<String>,
    bound_character: Option<String>,
    sim_pos: Option<[f32; 2]>,
    sim_vel: Option<[f32; 2]>,
    sim_size: Option<[f32; 2]>,
    pose: Option<PoseProbe>,
    presented_delta: Option<[f32; 2]>,
    sprite: Option<SpriteProbe>,
    animator: Option<AnimatorProbe>,
    baseline: Option<BaselineProbe>,
}

#[derive(Clone, Debug, PartialEq)]
struct ViewProbe {
    entity: u64,
    resolved: bool,
    resolved_ortho: Option<f32>,
    resolved_center: Option<[f32; 2]>,
    resolved_target: Option<[f32; 2]>,
    resolved_follow: Option<[f32; 2]>,
    resolved_visible_view: Option<[f32; 2]>,
    applied_view_ortho: Option<f32>,
    applied_view_center: Option<[f32; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
struct PhysicalViewportProbe {
    position: [u32; 2],
    size: [u32; 2],
}

#[derive(Clone, Debug, PartialEq)]
struct GameplayLayoutProbe {
    display_min: [f32; 2],
    display_size: [f32; 2],
    gameplay_min: [f32; 2],
    gameplay_size: [f32; 2],
}

impl GameplayLayoutProbe {
    fn has_reduced_gameplay_rect(&self) -> bool {
        !approx_v2(self.display_min, self.gameplay_min)
            || !approx_v2(self.display_size, self.gameplay_size)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MainCameraProbe {
    entity: u64,
    presents_view: Option<u64>,
    orthographic_scale: Option<f32>,
    translation: [f32; 3],
    viewport: Option<PhysicalViewportProbe>,
}

#[derive(Clone, Debug, PartialEq)]
struct PresentationFingerprint {
    active_scope: Option<u64>,
    root_scopes: Vec<u64>,
    feature_view_rows: usize,
    layout: Option<GameplayLayoutProbe>,
    players: Vec<PlayerProbe>,
    views: Vec<ViewProbe>,
    cameras: Vec<MainCameraProbe>,
}

#[derive(Clone, Debug)]
struct PresentationSample {
    app_frame: u64,
    session_frame: Option<u64>,
    sim_tick: u64,
    rollback_frame: Option<i32>,
    advance_runs: Option<u64>,
    fingerprint: PresentationFingerprint,
    warnings: Vec<String>,
}

impl PresentationSample {
    fn compact_line(&self) -> String {
        let scope = self
            .fingerprint
            .active_scope
            .map_or_else(|| "-".to_owned(), |scope| scope.to_string());
        let sf = self
            .session_frame
            .map_or_else(|| "-".to_owned(), |frame| frame.to_string());
        let ggrs = self
            .rollback_frame
            .map_or_else(|| "-".to_owned(), |frame| frame.to_string());
        let runs = self
            .advance_runs
            .map_or_else(|| "-".to_owned(), |runs| runs.to_string());

        let (pose, sprite, scale) = self
            .fingerprint
            .players
            .first()
            .map(|player| {
                let pose = player
                    .pose
                    .as_ref()
                    .and_then(|pose| pose.authored_render)
                    .map_or_else(|| "-".to_owned(), fmt_v2);
                let sprite = player
                    .sprite
                    .as_ref()
                    .and_then(|sprite| sprite.custom_size)
                    .map_or_else(|| "-".to_owned(), fmt_v2);
                let scale = player.sprite.as_ref().map_or_else(
                    || "-".to_owned(),
                    |sprite| {
                        format!(
                            "{:.2},{:.2}",
                            sprite.transform_scale[0], sprite.transform_scale[1]
                        )
                    },
                );
                (pose, sprite, scale)
            })
            .unwrap_or_else(|| ("-".to_owned(), "-".to_owned(), "-".to_owned()));
        let body_pos = self
            .fingerprint
            .players
            .first()
            .and_then(|player| player.sim_pos)
            .map_or_else(|| "-".to_owned(), fmt_xy);

        let resolved = self
            .fingerprint
            .views
            .first()
            .and_then(|view| view.resolved_ortho)
            .map_or_else(|| "-".to_owned(), |value| format!("{value:.3}"));
        let applied = self
            .fingerprint
            .cameras
            .first()
            .and_then(|camera| camera.orthographic_scale)
            .map_or_else(|| "-".to_owned(), |value| format!("{value:.3}"));
        let resolved_center = self
            .fingerprint
            .views
            .first()
            .and_then(|view| view.resolved_center)
            .map_or_else(|| "-".to_owned(), fmt_xy);
        let applied_center = self
            .fingerprint
            .views
            .first()
            .and_then(|view| view.applied_view_center)
            .map_or_else(|| "-".to_owned(), fmt_xy);
        let target = self
            .fingerprint
            .views
            .first()
            .and_then(|view| view.resolved_target)
            .map_or_else(|| "-".to_owned(), fmt_xy);
        let follow = self
            .fingerprint
            .views
            .first()
            .and_then(|view| view.resolved_follow)
            .map_or_else(|| "-".to_owned(), fmt_xy);
        let camera_xy = self
            .fingerprint
            .cameras
            .first()
            .map_or_else(|| "-".to_owned(), |camera| {
                format!("{:.1},{:.1}", camera.translation[0], camera.translation[1])
            });
        let viewport = self
            .fingerprint
            .cameras
            .first()
            .map_or_else(|| "-".to_owned(), |camera| match &camera.viewport {
                Some(viewport) => format!(
                    "{}x{}@{},{}",
                    viewport.size[0], viewport.size[1], viewport.position[0], viewport.position[1]
                ),
                None => "full".to_owned(),
            });
        let gameplay_rect = self.fingerprint.layout.as_ref().map_or_else(
            || "-".to_owned(),
            |layout| {
                format!(
                    "{:.0}x{:.0}@{:.0},{:.0}",
                    layout.gameplay_size[0],
                    layout.gameplay_size[1],
                    layout.gameplay_min[0],
                    layout.gameplay_min[1]
                )
            },
        );

        format!(
            "F{:05} S{:>3} tick={:<4} ggrs={:<4} runs={:<3} scope={} fv={:<3} body={} pose={} sprite={} scale={} cam-res={} cam-applied={} center={}->{} target={} follow={} cam-xy={} vp={} layout={}{}",
            self.app_frame,
            sf,
            self.sim_tick,
            ggrs,
            runs,
            scope,
            self.fingerprint.feature_view_rows,
            body_pos,
            pose,
            sprite,
            scale,
            resolved,
            applied,
            resolved_center,
            applied_center,
            target,
            follow,
            camera_xy,
            viewport,
            gameplay_rect,
            if self.warnings.is_empty() { "" } else { "  !!" },
        )
    }
}

#[derive(Resource, Debug)]
pub(crate) struct PresentationProbeState {
    enabled: bool,
    log_to_stderr: bool,
    app_frame: u64,
    traced_scope: Option<u64>,
    scope_started_at_app_frame: Option<u64>,
    remaining: u32,
    last_sim_tick: Option<u64>,
    last_rollback_frame: Option<i32>,
    last_advance_runs: Option<u64>,
    last_fingerprint: Option<PresentationFingerprint>,
    pre_roll: VecDeque<PresentationSample>,
    samples: VecDeque<PresentationSample>,
}

impl Default for PresentationProbeState {
    fn default() -> Self {
        Self {
            enabled: false,
            log_to_stderr: false,
            app_frame: 0,
            traced_scope: None,
            scope_started_at_app_frame: None,
            remaining: 0,
            last_sim_tick: None,
            last_rollback_frame: None,
            last_advance_runs: None,
            last_fingerprint: None,
            pre_roll: VecDeque::with_capacity(PRE_ROLL_FRAMES),
            samples: VecDeque::with_capacity(TRACE_CAPACITY),
        }
    }
}

pub(crate) struct PresentationProbePlugin;

impl Plugin for PresentationProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PresentationProbeState>()
            .add_systems(Last, collect_presentation_probe)
            .add_systems(EguiPrimaryContextPass, presentation_probe_ui);
    }
}

fn v2(value: Vec2) -> [f32; 2] {
    [value.x, value.y]
}

fn v3(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

fn ae_v2(value: ambition_platformer2d::engine_core::Vec2) -> [f32; 2] {
    [value.x, value.y]
}

fn fmt_v2(value: [f32; 2]) -> String {
    format!("{:.1}x{:.1}", value[0], value[1])
}

fn fmt_xy(value: [f32; 2]) -> String {
    format!("({:.1},{:.1})", value[0], value[1])
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

fn approx_v2(a: [f32; 2], b: [f32; 2]) -> bool {
    approx(a[0], b[0]) && approx(a[1], b[1])
}

fn collect_players(world: &mut World) -> Vec<PlayerProbe> {
    let mut query = world.query_filtered::<(
        Entity,
        Option<&PlayerEntity>,
        Option<&PlayerVisual>,
        Option<&WornCharacter>,
        Option<&BodyKinematics>,
        Option<&BodyPoseView>,
        Option<&PresentedPose>,
        Option<&Sprite>,
        Option<&CharacterAnimator>,
        Option<&Transform>,
        Option<&PlayerSpriteCharacter>,
        Option<&PlayerSpriteBaseline>,
        Option<&Anchor>,
    ), With<PrimaryPlayer>>();

    let mut players = query
        .iter(world)
        .map(
            |(
                entity,
                player_entity,
                player_visual,
                worn,
                kinematics,
                pose,
                presented_pose,
                sprite,
                animator,
                transform,
                bound,
                baseline,
                anchor,
            )| PlayerProbe {
                entity: entity.to_bits(),
                has_player_entity: player_entity.is_some(),
                has_player_visual: player_visual.is_some(),
                worn: worn.map(|worn| worn.id().to_owned()),
                bound_character: bound.map(|bound| bound.id.clone()),
                sim_pos: kinematics.map(|body| ae_v2(body.pos)),
                sim_vel: kinematics.map(|body| ae_v2(body.vel)),
                sim_size: kinematics.map(|body| ae_v2(body.size)),
                pose: pose.map(|pose| PoseProbe {
                    size: ae_v2(pose.size),
                    base_size: ae_v2(pose.base_size),
                    authored_render: pose.authored_render.map(ae_v2),
                    authored_offset: pose.authored_offset.map(ae_v2),
                    hp_current: pose.hp_current,
                    hp_max: pose.hp_max,
                }),
                presented_delta: presented_pose.map(|pose| ae_v2(pose.delta())),
                sprite: sprite.map(|sprite| SpriteProbe {
                    custom_size: sprite.custom_size.map(v2),
                    transform_scale: transform.map_or([1.0, 1.0, 1.0], |transform| {
                        v3(transform.scale)
                    }),
                    transform_translation: transform.map_or([0.0, 0.0, 0.0], |transform| {
                        v3(transform.translation)
                    }),
                    image: format!("{:?}", sprite.image.id()),
                    atlas: sprite.texture_atlas.as_ref().map(|atlas| {
                        (format!("{:?}", atlas.layout.id()), atlas.index)
                    }),
                    anchor: anchor.map(|anchor| format!("{anchor:?}")),
                }),
                animator: animator.map(|animator| AnimatorProbe {
                    current: format!("{:?}", animator.current),
                    frame: animator.frame,
                    trimmed: animator.spec.is_trimmed(),
                    render_basis: animator.render_basis.as_ref().map(|basis| v2(basis.render_size)),
                    current_render: animator.current_render().map(|(size, _)| v2(size)),
                }),
                baseline: baseline.map(|baseline| BaselineProbe {
                    standing_render: v2(baseline.standing_render),
                    standing_collision: v2(baseline.standing_collision),
                }),
            },
        )
        .collect::<Vec<_>>();
    players.sort_by_key(|player| player.entity);
    players
}

fn collect_views(world: &mut World) -> Vec<ViewProbe> {
    let mut query = world.query_filtered::<(
        Entity,
        &ResolvedCameraSnapshot,
        Option<&CameraViewState>,
    ), With<LocalView>>();
    let mut views = query
        .iter(world)
        .map(|(entity, resolved, applied)| {
            let frame = resolved.frame();
            ViewProbe {
                entity: entity.to_bits(),
                resolved: frame.is_some(),
                resolved_ortho: frame.map(|frame| frame.snapshot.orthographic_scale),
                resolved_center: frame.map(|frame| ae_v2(frame.snapshot.center_world)),
                resolved_target: frame.map(|frame| ae_v2(frame.snapshot.target_world)),
                resolved_follow: frame.map(|frame| ae_v2(frame.follow_world)),
                resolved_visible_view: frame.map(|frame| ae_v2(frame.snapshot.visible_view)),
                applied_view_ortho: applied.map(|view| view.orthographic_scale),
                applied_view_center: applied.map(|view| ae_v2(view.center_world)),
            }
        })
        .collect::<Vec<_>>();
    views.sort_by_key(|view| view.entity);
    views
}

fn collect_cameras(world: &mut World) -> Vec<MainCameraProbe> {
    let mut query = world.query_filtered::<(
        Entity,
        &Transform,
        &Projection,
        &Camera,
        Option<&PresentsView>,
    ), With<MainCamera>>();
    let mut cameras = query
        .iter(world)
        .map(|(entity, transform, projection, camera, presents)| MainCameraProbe {
            entity: entity.to_bits(),
            presents_view: presents.map(|presents| presents.0.to_bits()),
            orthographic_scale: match projection {
                Projection::Orthographic(orthographic) => Some(orthographic.scale),
                _ => None,
            },
            translation: v3(transform.translation),
            viewport: camera.viewport.as_ref().map(|viewport| PhysicalViewportProbe {
                position: [viewport.physical_position.x, viewport.physical_position.y],
                size: [viewport.physical_size.x, viewport.physical_size.y],
            }),
        })
        .collect::<Vec<_>>();
    cameras.sort_by_key(|camera| camera.entity);
    cameras
}

fn diagnose(
    fingerprint: &PresentationFingerprint,
    advance_runs: Option<u64>,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if let Some(active) = fingerprint.active_scope {
        if !fingerprint.root_scopes.contains(&active) {
            warnings.push(format!(
                "SESSION: active scope {active} has no matching SessionRoot"
            ));
        }
    }

    if fingerprint.active_scope.is_some()
        && advance_runs == Some(0)
        && (!fingerprint.players.is_empty() || !fingerprint.cameras.is_empty())
    {
        warnings.push(
            "SESSION: gameplay presentation is drawable before the first rollback advance"
                .to_owned(),
        );
    }

    if fingerprint.players.len() > 1 {
        warnings.push(format!(
            "ACTOR: {} PrimaryPlayer bodies are visible to the probe",
            fingerprint.players.len()
        ));
    }

    if let Some(player) = fingerprint.players.first() {
        // A character presentation may be complete before BodyPoseView exists.
        // The invalid state is a drawable trimmed frame whose animator has not
        // initialized the render basis, checked below.
        if let (Some(worn), Some(bound)) = (&player.worn, &player.bound_character) {
            if worn != bound {
                warnings.push(format!(
                    "ACTOR: worn character {worn:?} but renderer is bound to {bound:?}"
                ));
            }
        }
        if let (Some(animator), Some(sprite)) = (&player.animator, &player.sprite) {
            if animator.trimmed && animator.render_basis.is_none() {
                warnings.push(
                    "ACTOR: trimmed sprite is drawable before CharacterAnimator render basis is initialized"
                        .to_owned(),
                );
            }
            if let (Some(expected), Some(custom)) = (animator.current_render, sprite.custom_size) {
                if !approx_v2(expected, custom) {
                    warnings.push(format!(
                        "ACTOR: animator current trim {} but Sprite.custom_size is {}",
                        fmt_v2(expected),
                        fmt_v2(custom)
                    ));
                }
            }
        }
        if let (Some(pose), Some(_sprite)) = (&player.pose, &player.sprite) {
            if let Some(sim_size) = player.sim_size {
                if !approx_v2(sim_size, pose.size) {
                    warnings.push(format!(
                        "ACTOR: sim body size {} but BodyPoseView size is {}",
                        fmt_v2(sim_size),
                        fmt_v2(pose.size)
                    ));
                }
            }
        }
    }

    if fingerprint.views.len() > 1 {
        warnings.push(format!(
            "CAMERA: {} LocalView entities make startup interpretation ambiguous",
            fingerprint.views.len()
        ));
    }
    if fingerprint.cameras.len() > 1 {
        warnings.push(format!(
            "CAMERA: {} MainCamera entities make startup interpretation ambiguous",
            fingerprint.cameras.len()
        ));
    }

    if let Some(camera) = fingerprint.cameras.first() {
        if fingerprint.active_scope.is_some()
            && fingerprint
            .layout
            .as_ref()
            .is_some_and(GameplayLayoutProbe::has_reduced_gameplay_rect)
            && camera.viewport.is_none()
        {
            warnings.push(
                "CAMERA: resolved gameplay layout is smaller than the display but MainCamera.viewport is still full-window"
                    .to_owned(),
            );
        }
    }

    if let (Some(view), Some(camera)) = (fingerprint.views.first(), fingerprint.cameras.first()) {
        if fingerprint.active_scope.is_some() && !view.resolved {
            warnings.push(format!(
                "CAMERA: active gameplay LocalView has no ResolvedCameraSnapshot while main camera still has ortho {:?}",
                camera.orthographic_scale
            ));
        }
        if let (Some(resolved), Some(applied)) = (view.resolved_ortho, camera.orthographic_scale) {
            if !approx(resolved, applied) {
                warnings.push(format!(
                    "CAMERA: resolved ortho {resolved:.3} but main camera projection is {applied:.3}"
                ));
            }
        }
        if let (Some(resolved), Some(applied)) = (view.resolved_ortho, view.applied_view_ortho) {
            if !approx(resolved, applied) {
                warnings.push(format!(
                    "CAMERA: resolved ortho {resolved:.3} but CameraViewState still says {applied:.3}"
                ));
            }
        }
        if let (Some(resolved), Some(applied)) = (view.resolved_center, view.applied_view_center) {
            if !approx_v2(resolved, applied) {
                warnings.push(format!(
                    "CAMERA: resolved center {} but CameraViewState still says {}",
                    fmt_xy(resolved),
                    fmt_xy(applied),
                ));
            }
        }
        if let (Some(presents), Some(view_entity)) = (camera.presents_view, Some(view.entity)) {
            if presents != view_entity {
                warnings.push(format!(
                    "CAMERA: main camera presents entity {presents} but sampled LocalView is {view_entity}"
                ));
            }
        }
    }

    warnings
}

fn collect_presentation_probe(world: &mut World) {
    // Keep the permanent developer probe essentially free when it is not in
    // use. In particular, do not build fingerprints or retain shell pre-roll
    // on ordinary runs merely because the plugin is installed.
    if !world
        .get_resource::<PresentationProbeState>()
        .is_some_and(|probe| probe.enabled)
    {
        return;
    }

    let active_scope = world
        .get_resource::<ActiveSessionScope>()
        .and_then(ActiveSessionScope::current)
        .map(|scope| scope.0);
    let sim_tick = world.get_resource::<SimTick>().map_or(0, |tick| tick.get());
    let rollback_frame = world
        .get_resource::<RollbackFrameCount>()
        .map(|frame| frame.0);
    let advance_runs = world
        .get_resource::<RollbackExecutionStats>()
        .map(|stats| stats.advance_runs);

    let mut roots = {
        let mut query = world.query::<&SessionRoot>();
        query.iter(world).map(|root| root.0.0).collect::<Vec<_>>()
    };
    roots.sort_unstable();

    let layout = world.get_resource::<ResolvedGameplayPresentation>().map(|layout| {
        GameplayLayoutProbe {
            display_min: ae_v2(layout.display_rect.min),
            display_size: ae_v2(layout.display_rect.size()),
            gameplay_min: ae_v2(layout.gameplay_rect.min),
            gameplay_size: ae_v2(layout.gameplay_rect.size()),
        }
    });

    let fingerprint = PresentationFingerprint {
        active_scope,
        root_scopes: roots,
        feature_view_rows: world.get_resource::<FeatureViewIndex>().map_or(0, FeatureViewIndex::len),
        layout,
        players: collect_players(world),
        views: collect_views(world),
        cameras: collect_cameras(world),
    };
    let warnings = diagnose(&fingerprint, advance_runs);

    let mut probe = world.resource_mut::<PresentationProbeState>();
    probe.app_frame = probe.app_frame.saturating_add(1);

    // Keep a short rolling shell/pre-session history. The camera flash reported
    // by the product can happen one render BEFORE ActiveSessionScope appears;
    // a probe that arms only after activation can prove every active frame is
    // correct while still missing the actual bad frame.
    if active_scope.is_none() {
        let sample = PresentationSample {
            app_frame: probe.app_frame,
            session_frame: None,
            sim_tick,
            rollback_frame,
            advance_runs,
            fingerprint,
            warnings,
        };
        if probe.pre_roll.len() == PRE_ROLL_FRAMES {
            probe.pre_roll.pop_front();
        }
        probe.pre_roll.push_back(sample);
        probe.traced_scope = None;
        probe.scope_started_at_app_frame = None;
        probe.remaining = 0;
        probe.last_sim_tick = None;
        probe.last_rollback_frame = None;
        probe.last_advance_runs = None;
        probe.last_fingerprint = None;
        return;
    }

    if active_scope != probe.traced_scope {
        let app_frame = probe.app_frame;
        probe.traced_scope = active_scope;
        probe.scope_started_at_app_frame = Some(app_frame);
        probe.remaining = STARTUP_TRACE_FRAMES;
        probe.last_sim_tick = None;
        probe.last_rollback_frame = None;
        probe.last_advance_runs = None;
        probe.last_fingerprint = None;
        probe.samples.clear();

        if probe.log_to_stderr && !probe.pre_roll.is_empty() {
            eprintln!("[presentation-probe] --- {} pre-roll frames before session activation ---", probe.pre_roll.len());
            for sample in &probe.pre_roll {
                eprintln!("[presentation-probe] PRE {}", sample.compact_line());
                for warning in &sample.warnings {
                    eprintln!("[presentation-probe]   {warning}");
                }
            }
        }
        while let Some(sample) = probe.pre_roll.pop_front() {
            if probe.samples.len() == TRACE_CAPACITY {
                probe.samples.pop_front();
            }
            probe.samples.push_back(sample);
        }
    }

    if probe.remaining == 0 {
        return;
    }

    let changed = probe.last_fingerprint.as_ref() != Some(&fingerprint);
    let tick_changed = probe.last_sim_tick != Some(sim_tick);
    let rollback_changed = probe.last_rollback_frame != rollback_frame;
    let advance_changed = probe.last_advance_runs != advance_runs;
    // A frozen frame-step world may render the identical state hundreds of
    // times while the developer reads the panel. Those repeats contain no new
    // evidence and must not consume the startup trace budget. In realtime, the
    // camera/actor transient changes the fingerprint; a new sim/GGRS frame is
    // also retained even when the visible state happens to be identical.
    if !changed && !tick_changed && !rollback_changed && !advance_changed {
        return;
    }

    let session_frame = probe
        .scope_started_at_app_frame
        .map(|start| probe.app_frame.saturating_sub(start));
    let sample = PresentationSample {
        app_frame: probe.app_frame,
        session_frame,
        sim_tick,
        rollback_frame,
        advance_runs,
        fingerprint: fingerprint.clone(),
        warnings,
    };

    let early = session_frame.is_some_and(|frame| frame < ALWAYS_LOG_FIRST_SESSION_FRAMES);
    if probe.log_to_stderr
        && (changed
            || tick_changed
            || rollback_changed
            || advance_changed
            || early
            || !sample.warnings.is_empty())
    {
        eprintln!("[presentation-probe] {}", sample.compact_line());
        for warning in &sample.warnings {
            eprintln!("[presentation-probe]   {warning}");
        }
    }

    if probe.samples.len() == TRACE_CAPACITY {
        probe.samples.pop_front();
    }
    probe.samples.push_back(sample);
    probe.last_sim_tick = Some(sim_tick);
    probe.last_rollback_frame = rollback_frame;
    probe.last_advance_runs = advance_runs;
    probe.last_fingerprint = Some(fingerprint);
    probe.remaining = probe.remaining.saturating_sub(1);
}

fn presentation_probe_ui(world: &mut World) {
    let inspector_visible = world
        .get_resource::<DeveloperTools>()
        .is_some_and(|tools| tools.inspector_visible);
    if !inspector_visible {
        return;
    }

    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let Some(probe) = world.get_resource::<PresentationProbeState>() else {
        return;
    };
    let mut enabled = probe.enabled;
    let mut log_to_stderr = probe.log_to_stderr;
    let remaining = probe.remaining;
    let latest = probe.samples.back().cloned();
    let recent = probe
        .samples
        .iter()
        .rev()
        .take(18)
        .cloned()
        .collect::<Vec<_>>();

    let mut clear = false;
    let mut rearm = false;

    egui::Window::new("Startup Presentation Probe")
        .default_width(560.0)
        .resizable(true)
        .show(egui_context.get_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut enabled, "Enable presentation probe");
                ui.add_enabled_ui(enabled, |ui| {
                    ui.checkbox(&mut log_to_stderr, "Print transitions to stderr");
                });
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    if ui.button("Re-arm 180 frames").clicked() {
                        rearm = true;
                    }
                });
                if ui.button("Clear trace").clicked() {
                    clear = true;
                }
                if enabled {
                    ui.label(format!("{remaining} startup frames remaining"));
                } else {
                    ui.label("probe disabled");
                }
            });
            ui.small(
                "Opt-in. When enabled, samples in Last after simulation/presentation main-world systems and before render extraction; new gameplay sessions arm automatically.",
            );

            ui.separator();
            ui.strong("Latest frame");
            if let Some(sample) = latest.as_ref() {
                ui.monospace(sample.compact_line());

                if sample.warnings.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "No probe invariant is currently violated.");
                } else {
                    for warning in &sample.warnings {
                        ui.colored_label(egui::Color32::YELLOW, warning);
                    }
                }

                if let Some(player) = sample.fingerprint.players.first() {
                    ui.collapsing("Actor detail", |ui| {
                        ui.monospace(format!(
                            "entity={} PlayerEntity={} PlayerVisual={} worn={:?} bound={:?}",
                            player.entity,
                            player.has_player_entity,
                            player.has_player_visual,
                            player.worn,
                            player.bound_character,
                        ));
                        ui.monospace(format!(
                            "sim pos={:?} vel={:?} size={:?} presented_delta={:?}",
                            player.sim_pos.map(fmt_xy),
                            player.sim_vel.map(fmt_xy),
                            player.sim_size.map(fmt_v2),
                            player.presented_delta.map(fmt_xy),
                        ));
                        if let Some(pose) = player.pose.as_ref() {
                            ui.monospace(format!(
                                "pose size={} base={} authored={} offset={:?} hp={}/{}",
                                fmt_v2(pose.size),
                                fmt_v2(pose.base_size),
                                pose.authored_render.map_or_else(|| "-".to_owned(), fmt_v2),
                                pose.authored_offset.map(fmt_v2),
                                pose.hp_current,
                                pose.hp_max,
                            ));
                        } else {
                            ui.monospace("BodyPoseView = NONE");
                        }
                        if let Some(sprite) = player.sprite.as_ref() {
                            ui.monospace(format!(
                                "sprite custom={:?} scale={:.3},{:.3},{:.3} image={} atlas={:?} anchor={:?}",
                                sprite.custom_size.map(fmt_v2),
                                sprite.transform_scale[0],
                                sprite.transform_scale[1],
                                sprite.transform_scale[2],
                                sprite.image,
                                sprite.atlas,
                                sprite.anchor,
                            ));
                        } else {
                            ui.monospace("Sprite = NONE");
                        }
                        if let Some(animator) = player.animator.as_ref() {
                            ui.monospace(format!(
                                "anim {:?} frame={} trimmed={} basis={:?} current_trim={:?}",
                                animator.current,
                                animator.frame,
                                animator.trimmed,
                                animator.render_basis.map(fmt_v2),
                                animator.current_render.map(fmt_v2),
                            ));
                        }
                        if let Some(baseline) = player.baseline.as_ref() {
                            ui.monospace(format!(
                                "baseline render={} collision={}",
                                fmt_v2(baseline.standing_render),
                                fmt_v2(baseline.standing_collision),
                            ));
                        }
                    });
                }

                ui.collapsing("Camera detail", |ui| {
                    if let Some(view) = sample.fingerprint.views.first() {
                        ui.monospace(format!(
                            "view={} resolved={} resolved ortho={:?} center={:?} target={:?} follow={:?} visible={:?}",
                            view.entity,
                            view.resolved,
                            view.resolved_ortho,
                            view.resolved_center,
                            view.resolved_target,
                            view.resolved_follow,
                            view.resolved_visible_view,
                        ));
                        ui.monospace(format!(
                            "CameraViewState ortho={:?} center={:?}",
                            view.applied_view_ortho, view.applied_view_center,
                        ));
                    } else {
                        ui.monospace("LocalView = NONE");
                    }
                    if let Some(camera) = sample.fingerprint.cameras.first() {
                        ui.monospace(format!(
                            "main camera={} presents={:?} ortho={:?} xyz={:.1},{:.1},{:.1} viewport={:?}",
                            camera.entity,
                            camera.presents_view,
                            camera.orthographic_scale,
                            camera.translation[0],
                            camera.translation[1],
                            camera.translation[2],
                            camera.viewport,
                        ));
                    } else {
                        ui.monospace("MainCamera = NONE");
                    }
                    if let Some(layout) = sample.fingerprint.layout.as_ref() {
                        ui.monospace(format!(
                            "layout display={:.0}x{:.0}@{:.0},{:.0} gameplay={:.0}x{:.0}@{:.0},{:.0}",
                            layout.display_size[0],
                            layout.display_size[1],
                            layout.display_min[0],
                            layout.display_min[1],
                            layout.gameplay_size[0],
                            layout.gameplay_size[1],
                            layout.gameplay_min[0],
                            layout.gameplay_min[1],
                        ));
                    }
                });
            } else if enabled {
                ui.label("No sample yet. Start Ambition through the title shell, or re-arm in gameplay.");
            } else {
                ui.label("Enable the probe to collect presentation samples.");
            }

            ui.separator();
            ui.strong("Recent rendered-frame sequence (newest first)");
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    for sample in &recent {
                        let line = sample.compact_line();
                        if sample.warnings.is_empty() {
                            ui.monospace(line);
                        } else {
                            ui.colored_label(egui::Color32::YELLOW, egui::RichText::new(line).monospace());
                        }
                    }
                });

            ui.separator();
            ui.small(
                "Interpretation: a trimmed drawable sprite with no animator basis is invalid construction; after the fix, frame-zero trim should already match Sprite.custom_size even before BodyPoseView. Camera center changing in ResolvedCameraSnapshot means the resolver authored the whole-room snap; resolved center staying stable while CameraViewState lags means presentation applied it late. A reduced layout with viewport=full identifies a physical-viewport handoff gap. The trace keeps 12 shell frames before activation so a pre-session flash is not lost.",
            );
        });

    if let Some(mut probe) = world.get_resource_mut::<PresentationProbeState>() {
        let was_enabled = probe.enabled;
        probe.enabled = enabled;
        probe.log_to_stderr = enabled && log_to_stderr;
        if !enabled {
            probe.pre_roll.clear();
            probe.remaining = 0;
            probe.traced_scope = None;
            probe.scope_started_at_app_frame = None;
            probe.last_sim_tick = None;
            probe.last_rollback_frame = None;
            probe.last_advance_runs = None;
            probe.last_fingerprint = None;
        } else if !was_enabled {
            let app_frame = probe.app_frame;
            probe.remaining = STARTUP_TRACE_FRAMES;
            probe.scope_started_at_app_frame = probe.traced_scope.map(|_| app_frame);
            probe.samples.clear();
            probe.pre_roll.clear();
            probe.last_sim_tick = None;
            probe.last_rollback_frame = None;
            probe.last_advance_runs = None;
            probe.last_fingerprint = None;
        }
        if clear {
            probe.samples.clear();
            probe.last_fingerprint = None;
        }
        if rearm {
            let app_frame = probe.app_frame;
            let traced_scope = probe.traced_scope;
            probe.remaining = STARTUP_TRACE_FRAMES;
            probe.scope_started_at_app_frame = traced_scope.map(|_| app_frame);
            probe.samples.clear();
            probe.last_sim_tick = None;
            probe.last_rollback_frame = None;
            probe.last_advance_runs = None;
            probe.last_fingerprint = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_fingerprint() -> PresentationFingerprint {
        PresentationFingerprint {
            active_scope: Some(7),
            root_scopes: vec![7],
            feature_view_rows: 0,
            layout: None,
            players: Vec::new(),
            views: Vec::new(),
            cameras: Vec::new(),
        }
    }

    #[test]
    fn probe_is_opt_in_by_default() {
        let probe = PresentationProbeState::default();
        assert!(!probe.enabled);
        assert!(!probe.log_to_stderr);
    }

    fn complete_trimmed_player_without_pose() -> PlayerProbe {
        PlayerProbe {
            entity: 1,
            has_player_entity: true,
            has_player_visual: true,
            worn: Some("robot_v3".to_owned()),
            bound_character: Some("robot_v3".to_owned()),
            sim_pos: Some([100.0, 200.0]),
            sim_vel: Some([0.0, 0.0]),
            sim_size: Some([48.0, 64.0]),
            pose: None,
            presented_delta: None,
            sprite: Some(SpriteProbe {
                custom_size: Some([37.4, 53.2]),
                transform_scale: [1.0, 1.0, 1.0],
                transform_translation: [0.0, 0.0, 0.0],
                image: "image".to_owned(),
                atlas: None,
                anchor: None,
            }),
            animator: Some(AnimatorProbe {
                current: "Idle".to_owned(),
                frame: 0,
                trimmed: true,
                render_basis: Some([135.0, 135.0]),
                current_render: Some([37.4, 53.2]),
            }),
            baseline: None,
        }
    }

    #[test]
    fn complete_trimmed_primary_before_pose_is_not_an_error() {
        let mut fingerprint = empty_fingerprint();
        fingerprint.players.push(complete_trimmed_player_without_pose());
        let warnings = diagnose(&fingerprint, Some(1));
        assert!(!warnings.iter().any(|warning| warning.contains("BodyPoseView")));
        assert!(!warnings.iter().any(|warning| warning.contains("render basis")));
    }

    #[test]
    fn trimmed_primary_without_render_basis_is_called_out() {
        let mut fingerprint = empty_fingerprint();
        let mut player = complete_trimmed_player_without_pose();
        player.animator.as_mut().unwrap().render_basis = None;
        player.animator.as_mut().unwrap().current_render = None;
        fingerprint.players.push(player);
        assert!(diagnose(&fingerprint, Some(1))
            .iter()
            .any(|warning| warning.contains("render basis")));
    }

    #[test]
    fn shell_camera_without_resolved_snapshot_is_not_an_error() {
        let mut fingerprint = empty_fingerprint();
        fingerprint.active_scope = None;
        fingerprint.root_scopes.clear();
        fingerprint.views.push(ViewProbe {
            entity: 10,
            resolved: false,
            resolved_ortho: None,
            resolved_center: None,
            resolved_target: None,
            resolved_follow: None,
            resolved_visible_view: None,
            applied_view_ortho: Some(1.0),
            applied_view_center: Some([0.0, 0.0]),
        });
        fingerprint.cameras.push(MainCameraProbe {
            entity: 11,
            presents_view: Some(10),
            orthographic_scale: Some(1.0),
            translation: [0.0, 0.0, 0.0],
            viewport: None,
        });
        assert!(!diagnose(&fingerprint, Some(0))
            .iter()
            .any(|warning| warning.contains("ResolvedCameraSnapshot")));
    }

    #[test]
    fn resolved_camera_not_yet_applied_is_called_out() {
        let mut fingerprint = empty_fingerprint();
        fingerprint.views.push(ViewProbe {
            entity: 10,
            resolved: true,
            resolved_ortho: Some(1.25),
            resolved_center: Some([100.0, 200.0]),
            resolved_target: Some([110.0, 205.0]),
            resolved_follow: Some([125.0, 210.0]),
            resolved_visible_view: Some([568.0, 320.0]),
            applied_view_ortho: Some(1.0),
            applied_view_center: Some([0.0, 0.0]),
        });
        fingerprint.cameras.push(MainCameraProbe {
            entity: 11,
            presents_view: Some(10),
            orthographic_scale: Some(1.0),
            translation: [0.0, 0.0, 0.0],
            viewport: None,
        });
        let warnings = diagnose(&fingerprint, Some(1));
        assert!(warnings.iter().any(|warning| warning.contains("main camera projection")));
        assert!(warnings.iter().any(|warning| warning.contains("CameraViewState")));
    }
}
