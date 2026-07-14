//! Nameplates: presentation-only world-space labels above actors and doors.
//!
//! This intentionally lives in `ambition_render` rather than gameplay. Actor
//! identity, door names, and bounds are simulation/content state, but deciding
//! whether / how a human-facing label is drawn is view policy. The system keeps
//! one ECS visual entity per labeled source and only toggles visibility,
//! transform, and opacity each frame, so the rules can grow without becoming a
//! debug-overlay respawn loop.

use std::collections::{HashMap, HashSet};

use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::NameplateIndex;
use ambition_world::rooms::{ActiveRoomMetadata, RoomNameplatePolicy};
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};

use super::camera::CameraViewState;
use super::primitives::RoomVisual;

/// Presentation policy for world nameplates.
///
/// The default policy ranks all eligible labels by distance to
/// [`CameraViewState::target_world`], draws the first five at full opacity,
/// fades the sixth label, and reaches zero opacity at the seventh. Later
/// candidates are hidden. Active-room metadata may override the rank thresholds
/// from LDtk level fields. This keeps the selection rule local and
/// easy to tune without changing the actor/door collection code.
#[derive(Resource, Clone, Debug)]
pub struct ActorNameplateSettings {
    /// Global off-switch for the presentation surface.
    pub enabled: bool,
    /// Number of nearest eligible labels drawn at full configured opacity.
    pub full_opacity_count: usize,
    /// Ranked candidate count where opacity reaches zero. Candidates after this
    /// rank are hidden entirely.
    pub fade_out_count: usize,
    /// Optional world-pixel cutoff from the focus point. `None` means no cutoff.
    pub max_distance_px: Option<f32>,
    /// Gap between the source's rendered top edge and the text baseline.
    pub vertical_gap_px: f32,
    /// Font size in Bevy text points.
    pub font_size: f32,
    /// Absolute Bevy Z layer for the text root.
    pub z: f32,
    /// Main text color before rank-opacity is applied.
    pub text_color: Color,
    /// Shadow/outline text color before rank-opacity is applied.
    pub outline_color: Color,
    /// World-space pixel offset used for the four outline samples.
    pub outline_offset_px: f32,
}

impl Default for ActorNameplateSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            full_opacity_count: 5,
            fade_out_count: 7,
            max_distance_px: None,
            vertical_gap_px: 10.0,
            font_size: 10.0,
            z: WORLD_Z_PLAYER + 18.0,
            text_color: Color::srgba(0.94, 0.98, 1.0, 1.0),
            outline_color: Color::srgba(0.0, 0.0, 0.0, 0.72),
            outline_offset_px: 0.9,
        }
    }
}

/// Marker on any room visual that should participate in the nameplate policy.
///
/// Actor labels are collected directly from actor ECS components because their
/// render bounds are dynamic. Static door visuals carry this source component so
/// they can share the same ranking/fade/render machinery without adding door
/// special cases to gameplay.
#[derive(Component, Clone, Debug)]
pub struct DoorNameplateSource {
    pub id: String,
    pub label: String,
    pub center_world: ae::Vec2,
    pub size_world: ae::Vec2,
}

impl DoorNameplateSource {
    pub fn new(id: impl Into<String>, label: impl Into<String>, aabb: ae::Aabb) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            center_world: aabb.center(),
            size_world: aabb.half_size() * 2.0,
        }
    }
}

/// Marker on the root `Text2d` entity for a nameplate. `owner_id` is the
/// labeled source's STABLE id (an actor's feature id / a door's zone id) —
/// the view identity, never a sim `Entity` (E4 slice 16).
#[derive(Component, Clone, Debug)]
pub struct ActorNameplateVisual {
    pub owner_id: String,
    pub label: String,
}

/// Marker on outline child text entities. Kept separate so future style systems
/// can adjust only the shadow pass without inspecting hierarchy.
#[derive(Component, Clone, Copy, Debug)]
pub struct ActorNameplateOutlineVisual;

/// System set for nameplates. Downstream presentation code can order
/// before/after this set without naming the concrete sync system.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActorNameplateSet;

/// Render-layer plugin for player-facing actor/door labels.
pub struct ActorNameplatePresentationPlugin;

impl Plugin for ActorNameplatePresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorNameplateSettings>()
            .configure_sets(
                Update,
                ActorNameplateSet
                    .after(super::actors::sync_visuals)
                    .after(super::camera::camera_follow),
            )
            .add_systems(
                Update,
                sync_actor_nameplates
                    .in_set(ActorNameplateSet)
                    .run_if(ambition_platformer_primitives::lifecycle::session_world_exists),
            );
    }
}

#[derive(Clone, Debug)]
struct NameplateCandidate {
    owner_id: String,
    label: String,
    anchor_world: ae::Vec2,
    distance_sq: f32,
    opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedNameplateRankPolicy {
    full_opacity_count: usize,
    fade_out_count: usize,
}

impl ActorNameplateSettings {
    fn resolve_rank_policy(
        &self,
        room_policy: Option<&RoomNameplatePolicy>,
    ) -> ResolvedNameplateRankPolicy {
        ResolvedNameplateRankPolicy {
            full_opacity_count: room_policy
                .and_then(|policy| policy.full_opacity_count)
                .unwrap_or(self.full_opacity_count),
            fade_out_count: room_policy
                .and_then(|policy| policy.fade_out_count)
                .unwrap_or(self.fade_out_count),
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_actor_nameplates(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_engine_core::RoomGeometry>,
    settings: Res<ActorNameplateSettings>,
    active_session: Option<Res<ActiveSessionScope>>,
    active_metadata: Option<ambition_platformer_primitives::lifecycle::SessionWorldRef<ActiveRoomMetadata>>,
    camera: Option<Res<CameraViewState>>,
    // Sim-built nameplate read-model (E4 slices 5+16): label / geometry /
    // liveness / controlled-body facts per actor id. Doors stay render-side
    // sources below.
    nameplate_index: Option<Res<NameplateIndex>>,
    ui_fonts: Option<Res<UiFonts>>,
    mut nameplate_queries: ParamSet<(
        Query<(Entity, &DoorNameplateSource, Option<&Visibility>)>,
        Query<(
            Entity,
            &ActorNameplateVisual,
            &mut Transform,
            &mut Visibility,
            &mut TextColor,
            &Children,
        )>,
        Query<&mut TextColor, With<ActorNameplateOutlineVisual>>,
    )>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    if !settings.enabled {
        let mut nameplates = nameplate_queries.p1();
        hide_all_nameplates(&mut nameplates);
        return;
    }

    let rank_policy = settings.resolve_rank_policy(
        active_metadata
            .as_deref()
            .map(|active| &active.0.nameplate_policy),
    );
    let focus_world = camera
        .as_deref()
        .map_or(ae::Vec2::ZERO, |camera| camera.target_world);
    let mut source_ids = HashSet::new();
    let mut candidates = Vec::new();

    if let Some(index) = nameplate_index.as_deref() {
        collect_actor_candidates(
            &settings,
            index,
            focus_world,
            &mut source_ids,
            &mut candidates,
        );
    }
    {
        let door_sources = nameplate_queries.p0();
        collect_door_candidates(
            &settings,
            focus_world,
            &door_sources,
            &mut source_ids,
            &mut candidates,
        );
    }

    candidates.sort_by(|a, b| {
        a.distance_sq
            .total_cmp(&b.distance_sq)
            .then_with(|| a.label.cmp(&b.label))
    });
    apply_rank_opacity(rank_policy, &mut candidates);

    let visible_candidates: HashMap<String, NameplateCandidate> = candidates
        .into_iter()
        .take(rank_policy.fade_out_count)
        .map(|candidate| (candidate.owner_id.clone(), candidate))
        .collect();

    let mut existing_visible = HashSet::new();
    let mut outline_color_updates = Vec::new();
    {
        let mut nameplates = nameplate_queries.p1();
        for (entity, plate, mut transform, mut visibility, mut text_color, children) in
            &mut nameplates
        {
            if let Some(candidate) = visible_candidates.get(&plate.owner_id) {
                if plate.label != candidate.label {
                    // Name changes are rare. Rebuild the small text subtree so
                    // the root and outline children stay identical without
                    // relying on Text2d internals.
                    commands.entity(entity).despawn();
                    continue;
                }
                existing_visible.insert(plate.owner_id.clone());
                transform.translation = world_to_bevy(&world.0, candidate.anchor_world, settings.z);
                let visible = candidate.opacity > 0.0;
                *visibility = if visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                *text_color = TextColor(color_with_opacity(settings.text_color, candidate.opacity));
                for child in children.iter() {
                    outline_color_updates.push((child, candidate.opacity));
                }
            } else if source_ids.contains(plate.owner_id.as_str()) {
                *visibility = Visibility::Hidden;
            } else {
                commands.entity(entity).despawn();
            }
        }
    }

    {
        let mut outline_colors = nameplate_queries.p2();
        for (child, opacity) in outline_color_updates {
            if let Ok(mut outline_color) = outline_colors.get_mut(child) {
                *outline_color = TextColor(color_with_opacity(settings.outline_color, opacity));
            }
        }
    }

    let font = nameplate_font(ui_fonts.as_deref(), settings.font_size);
    for candidate in visible_candidates.values() {
        if !existing_visible.contains(candidate.owner_id.as_str()) {
            spawn_actor_nameplate(
                &mut commands,
                session_scope,
                &world.0,
                &settings,
                &font,
                candidate,
            );
        }
    }
}

fn collect_actor_candidates(
    settings: &ActorNameplateSettings,
    index: &NameplateIndex,
    focus_world: ae::Vec2,
    source_ids: &mut HashSet<String>,
    candidates: &mut Vec<NameplateCandidate>,
) {
    for (id, fact) in index.iter() {
        source_ids.insert(id.to_string());
        // The controlled subject's own plate is suppressed (the body the
        // local player is driving) — resolved sim-side into the fact.
        if fact.controlled {
            continue;
        }
        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            id.to_string(),
            fact.label.clone(),
            fact.center,
            fact.size,
        );
    }
}

fn collect_door_candidates(
    settings: &ActorNameplateSettings,
    focus_world: ae::Vec2,
    door_sources: &Query<(Entity, &DoorNameplateSource, Option<&Visibility>)>,
    source_ids: &mut HashSet<String>,
    candidates: &mut Vec<NameplateCandidate>,
) {
    for (_entity, source, visibility) in door_sources.iter() {
        source_ids.insert(source.id.clone());
        if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden) {
            continue;
        }
        if source.label.trim().is_empty() {
            continue;
        }

        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            source.id.clone(),
            source.label.clone(),
            source.center_world,
            source.size_world,
        );
    }
}

fn push_candidate_if_in_range(
    settings: &ActorNameplateSettings,
    focus_world: ae::Vec2,
    candidates: &mut Vec<NameplateCandidate>,
    owner_id: String,
    label: String,
    center: ae::Vec2,
    size: ae::Vec2,
) {
    let distance_sq = (center - focus_world).length_squared();
    if let Some(max_distance) = settings.max_distance_px {
        if distance_sq > max_distance.max(0.0).powi(2) {
            return;
        }
    }

    candidates.push(NameplateCandidate {
        owner_id,
        label,
        anchor_world: nameplate_anchor(center, size, settings.vertical_gap_px),
        distance_sq,
        opacity: 1.0,
    });
}

fn apply_rank_opacity(policy: ResolvedNameplateRankPolicy, candidates: &mut [NameplateCandidate]) {
    for (rank_index, candidate) in candidates.iter_mut().enumerate() {
        candidate.opacity =
            rank_opacity(rank_index, policy.full_opacity_count, policy.fade_out_count);
    }
}

fn rank_opacity(rank_index: usize, full_opacity_count: usize, fade_out_count: usize) -> f32 {
    let rank = rank_index + 1;
    if rank <= full_opacity_count {
        return 1.0;
    }
    if fade_out_count <= full_opacity_count || rank >= fade_out_count {
        return 0.0;
    }
    let fade_span = (fade_out_count - full_opacity_count) as f32;
    let remaining = (fade_out_count - rank) as f32;
    (remaining / fade_span).clamp(0.0, 1.0)
}

fn hide_all_nameplates(
    nameplates: &mut Query<(
        Entity,
        &ActorNameplateVisual,
        &mut Transform,
        &mut Visibility,
        &mut TextColor,
        &Children,
    )>,
) {
    for (_, _, _, mut visibility, _, _) in nameplates.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}

fn nameplate_anchor(center: ae::Vec2, size: ae::Vec2, vertical_gap_px: f32) -> ae::Vec2 {
    // Ambition world coordinates are +Y down. The label's anchor sits above the
    // rendered source box, so subtract half-height and the configured gap.
    ae::Vec2::new(center.x, center.y - size.y * 0.5 - vertical_gap_px.max(0.0))
}

fn nameplate_font(ui_fonts: Option<&UiFonts>, font_size: f32) -> TextFont {
    ui_fonts
        .map(|fonts| fonts.text_font(font_size, UiFontWeight::Semibold))
        .unwrap_or(TextFont {
            font_size,
            ..default()
        })
}

fn spawn_actor_nameplate(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    settings: &ActorNameplateSettings,
    font: &TextFont,
    candidate: &NameplateCandidate,
) {
    let text = candidate.label.clone();
    let outline_offsets = outline_offsets(settings.outline_offset_px);
    let text_color = color_with_opacity(settings.text_color, candidate.opacity);
    let outline_color = color_with_opacity(settings.outline_color, candidate.opacity);
    commands
        .spawn_session_scoped(
            session_scope,
            (
                Text2d::new(text.clone()),
                font.clone(),
                TextColor(text_color),
                Transform::from_translation(world_to_bevy(
                    world,
                    candidate.anchor_world,
                    settings.z,
                )),
                if candidate.opacity > 0.0 {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ActorNameplateVisual {
                    owner_id: candidate.owner_id.clone(),
                    label: text.clone(),
                },
                RoomVisual,
                Name::new(format!("Nameplate: {text}")),
            ),
        )
        .with_children(|parent| {
            for offset in outline_offsets {
                parent.spawn((
                    Text2d::new(text.clone()),
                    font.clone(),
                    TextColor(outline_color),
                    Transform::from_xyz(offset.x, offset.y, -0.1),
                    ActorNameplateOutlineVisual,
                    Name::new("Nameplate outline"),
                ));
            }
        });
}

fn color_with_opacity(color: Color, opacity: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        srgba.red,
        srgba.green,
        srgba.blue,
        srgba.alpha * opacity.clamp(0.0, 1.0),
    )
}

fn outline_offsets(offset_px: f32) -> [Vec2; 4] {
    let offset = offset_px.max(0.0);
    [
        Vec2::new(-offset, 0.0),
        Vec2::new(offset, 0.0),
        Vec2::new(0.0, -offset),
        Vec2::new(0.0, offset),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_shows_five_full_and_fades_to_zero_at_seven() {
        let settings = ActorNameplateSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.full_opacity_count, 5);
        assert_eq!(settings.fade_out_count, 7);
        assert_eq!(settings.max_distance_px, None);
        assert_eq!(rank_opacity(0, 5, 7), 1.0);
        assert_eq!(rank_opacity(4, 5, 7), 1.0);
        assert_eq!(rank_opacity(5, 5, 7), 0.5);
        assert_eq!(rank_opacity(6, 5, 7), 0.0);
    }

    #[test]
    fn active_room_policy_overrides_rank_thresholds() {
        let settings = ActorNameplateSettings::default();
        let policy = RoomNameplatePolicy {
            full_opacity_count: Some(100),
            fade_out_count: Some(120),
        };
        assert_eq!(
            settings.resolve_rank_policy(Some(&policy)),
            ResolvedNameplateRankPolicy {
                full_opacity_count: 100,
                fade_out_count: 120,
            }
        );
    }

    #[test]
    fn anchor_sits_above_source_in_y_down_world_space() {
        let anchor = nameplate_anchor(ae::Vec2::new(20.0, 100.0), ae::Vec2::new(30.0, 40.0), 10.0);
        assert_eq!(anchor, ae::Vec2::new(20.0, 70.0));
    }
}
