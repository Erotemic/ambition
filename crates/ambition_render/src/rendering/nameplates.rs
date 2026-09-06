//! Nameplates: presentation-only world-space labels above actors and doors.
//!
//! This intentionally lives in `ambition_render` rather than gameplay. Actor
//! identity, door names, and bounds are simulation/content state, but deciding
//! whether / how a human-facing label is drawn is view policy. The system keeps
//! one ECS visual entity per labeled source and only toggles visibility,
//! transform, and opacity each frame, so the rules can grow without becoming a
//! debug-overlay respawn loop.

use std::collections::{HashMap, HashSet};

use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::{ActiveRoomMetadata, RoomNameplatePolicy};
use ambition_sim_view::NameplateIndex;
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};

use super::label_layout::{WorldLabel, WorldLabelFamily};
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
///
/// It publishes candidates into the shared world-label placement pass; it
/// does not own that pass. The pass is
/// [`WorldLabelLayoutPlugin`](super::label_layout::WorldLabelLayoutPlugin),
/// which this plugin composes so a game that wants nameplates gets placement
/// without knowing the split — and which the generic room-visuals plugin adds
/// too, because signage is a world label whether or not anything in the
/// composition draws nameplates.
pub struct ActorNameplatePresentationPlugin;

impl Plugin for ActorNameplatePresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::label_layout::WorldLabelLayoutPlugin);
        app.init_resource::<ActorNameplateSettings>()
            .configure_sets(
                Update,
                ActorNameplateSet
                    .after(super::actors::sync_visuals)
                    .after(super::camera::camera_follow),
            )
            // The placement pass runs AFTER every family has published its
            // anchor for the frame. It is a hard ordering, not a preference:
            // placing before the plates move is placing against last frame.
            //
            // Configured HERE rather than in the layout plugin because it is a
            // fact about this family: only a composition that publishes actor
            // plates has an `ActorNameplateSet` for the pass to wait on.
            .configure_sets(
                Update,
                super::label_layout::WorldLabelLayoutSet.after(ActorNameplateSet),
            )
            .add_systems(
                Update,
                sync_actor_nameplates
                    .in_set(ActorNameplateSet)
                    .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
            );
    }
}

#[derive(Clone, Debug)]
struct NameplateCandidate {
    owner_id: String,
    label: String,
    /// Which placement family this plate belongs to. A door plate names a
    /// STATIC fixture, so it yields only to authored signage; an actor plate
    /// is already in motion and is the family that absorbs displacement.
    family: WorldLabelFamily,
    anchor_world: ae::Vec2,
    distance_sq: f32,
    opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedNameplateRankPolicy {
    full_opacity_count: usize,
    fade_out_count: usize,
    /// Does a body somebody is driving get a plate? See
    /// [`RoomNameplatePolicy::label_driven_bodies`] — the default is the
    /// exploration answer, and a room with a CAST overrides it.
    label_driven_bodies: bool,
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
            // ⛔ THE DEFAULT IS `false` — a driven body gets no plate — and that
            // is an EXPLORATION rule, right only while a room holds one driven
            // body. A room with a cast says so.
            label_driven_bodies: room_policy
                .and_then(|policy| policy.label_driven_bodies)
                .unwrap_or(false),
        }
    }
}

/// ONE SET OF PLATES PER VIEW.
///
/// WHICH PLATES ARE ON SCREEN IS A PROPERTY OF THE VIEW, NOT OF THE
/// ROOM. The policy ranks every candidate by distance to the camera's focus,
/// draws the nearest few, fades the next and hides the rest — so two views
/// looking at opposite ends of one room legitimately want two disjoint sets of
/// plates, at two different opacities, anchored by two different rankings. One
/// entity per labelled source could not express that; a second view would have
/// silently re-ranked the first view's plates out from under it.
///
/// so the plates are keyed by view, and a second view is a COUNT. A
/// one-view game builds exactly the plates it built before — same candidates,
/// same ranking, same anchors, same entity per visible source.
///
/// what is duplicated is the PLATE, never the thing it names. The
/// `NameplateIndex` row and the `DoorNameplateSource` on a room visual stay
/// singular — one authoritative source, N projections of it. Two views produce
/// two pictures of one door, never two doors.
#[allow(clippy::type_complexity)]
pub fn sync_actor_nameplates(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    settings: Res<ActorNameplateSettings>,
    active_session: Option<Res<ActiveSessionScope>>,
    active_metadata: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ActiveRoomMetadata>,
    >,
    // A draw system owes every view a picture, so it iterates them; `PresentedViewState`
    // answers the different question of which single view one camera shows, and refuses when
    // there are several.
    views: Query<(Entity, &ambition_sim_view::CameraViewState), With<ambition_sim_view::LocalView>>,
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
            &ambition_sim_view::PresentedForView,
            &mut WorldLabel,
        )>,
    )>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    if !settings.enabled {
        let mut nameplates = nameplate_queries.p1();
        for (_, _, _, mut label) in nameplates.iter_mut() {
            label.owner_opacity = 0.0;
        }
        return;
    }

    let rank_policy = settings.resolve_rank_policy(
        active_metadata
            .as_deref()
            .map(|active| &active.0.nameplate_policy),
    );

    // Every id that HAS a source this frame, across all views. It is what
    // separates "this plate's owner went away" (despawn it) from "this view
    // ranked it out" (hide it), and neither of those is a per-view question.
    let mut source_ids = HashSet::new();
    if let Some(index) = nameplate_index.as_deref() {
        for (id, _) in index.iter() {
            source_ids.insert(id.to_string());
        }
    }
    // Doors, snapshotted ONCE: every view needs them, and the `ParamSet` lends
    // out one of its queries at a time.
    let doors: Vec<DoorNameplateSource> = {
        let door_sources = nameplate_queries.p0();
        let mut doors = Vec::new();
        for (_entity, source, visibility) in door_sources.iter() {
            source_ids.insert(source.id.clone());
            if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden) {
                continue;
            }
            if source.label.trim().is_empty() {
                continue;
            }
            doors.push(source.clone());
        }
        doors
    };

    // What each view wants on screen, ranked against ITS OWN focus.
    let mut wanted: HashMap<Entity, HashMap<String, NameplateCandidate>> = HashMap::new();
    for (view_entity, view_state) in &views {
        let focus_world = view_state.target_world;
        let mut candidates = Vec::new();
        if let Some(index) = nameplate_index.as_deref() {
            collect_actor_candidates(&settings, rank_policy, index, focus_world, &mut candidates);
        }
        collect_door_candidates(&settings, focus_world, &doors, &mut candidates);

        candidates.sort_by(|a, b| {
            a.distance_sq
                .total_cmp(&b.distance_sq)
                .then_with(|| a.label.cmp(&b.label))
        });
        apply_rank_opacity(rank_policy, &mut candidates);

        wanted.insert(
            view_entity,
            candidates
                .into_iter()
                .take(rank_policy.fade_out_count)
                .map(|candidate| (candidate.owner_id.clone(), candidate))
                .collect(),
        );
    }

    // This system publishes each plate's WANTED anchor and opacity; the shared
    // placement pass (`label_layout`) owns the transform, visibility and
    // colour, because a plate has to be ranked against authored signage and
    // door plates too — not only against other actor plates (AC12).
    let mut existing_visible: HashSet<(Entity, String)> = HashSet::new();
    {
        let mut nameplates = nameplate_queries.p1();
        for (entity, plate, key, mut label) in &mut nameplates {
            let Some(view_wanted) = wanted.get(&key.0) else {
                commands.entity(entity).despawn();
                continue;
            };
            if let Some(candidate) = view_wanted.get(&plate.owner_id) {
                if plate.label != candidate.label {
                    // Name changes are rare. Rebuild the small text subtree so
                    // the root and outline children stay identical without
                    // relying on Text2d internals.
                    commands.entity(entity).despawn();
                    continue;
                }
                existing_visible.insert((key.0, plate.owner_id.clone()));
                label.family = candidate.family;
                label.anchor = world_to_bevy(&world.0, candidate.anchor_world, settings.z);
                label.owner_opacity = candidate.opacity;
                label.text_color = settings.text_color;
                label.outline_color = Some(settings.outline_color);
            } else if source_ids.contains(plate.owner_id.as_str()) {
                label.owner_opacity = 0.0;
            } else {
                commands.entity(entity).despawn();
            }
        }
    }

    let font = nameplate_font(ui_fonts.as_deref(), settings.font_size);
    for (view_entity, view_wanted) in &wanted {
        for candidate in view_wanted.values() {
            if existing_visible.contains(&(*view_entity, candidate.owner_id.clone())) {
                continue;
            }
            spawn_actor_nameplate(
                &mut commands,
                session_scope,
                &world.0,
                &settings,
                &font,
                candidate,
                *view_entity,
            );
        }
    }
}

fn collect_actor_candidates(
    settings: &ActorNameplateSettings,
    rank_policy: ResolvedNameplateRankPolicy,
    index: &NameplateIndex,
    focus_world: ae::Vec2,
    candidates: &mut Vec<NameplateCandidate>,
) {
    for (id, fact) in index.iter() {
        // ⭐⭐ WHETHER A DRIVEN BODY IS LABELLED IS THE ROOM'S CALL, not a
        // constant. Suppressing it is the exploration answer — a plate names a
        // body you are not inhabiting, so hiding it over the one you are is
        // honest with ONE driven body in the room. With a cast it renders as
        // "everyone is labelled except the human", and Jon named that on
        // 2026-08-24: *"This is player 1 centric behavior, and we should have
        // none of it."*
        if fact.driven && !rank_policy.label_driven_bodies {
            continue;
        }
        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            id.to_string(),
            fact.label.clone(),
            WorldLabelFamily::Actor,
            fact.center,
            fact.size,
        );
    }
}

fn collect_door_candidates(
    settings: &ActorNameplateSettings,
    focus_world: ae::Vec2,
    doors: &[DoorNameplateSource],
    candidates: &mut Vec<NameplateCandidate>,
) {
    for source in doors {
        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            source.id.clone(),
            source.label.clone(),
            WorldLabelFamily::Fixture,
            source.center_world,
            source.size_world,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_candidate_if_in_range(
    settings: &ActorNameplateSettings,
    focus_world: ae::Vec2,
    candidates: &mut Vec<NameplateCandidate>,
    owner_id: String,
    label: String,
    family: WorldLabelFamily,
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
        family,
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

fn nameplate_anchor(center: ae::Vec2, size: ae::Vec2, vertical_gap_px: f32) -> ae::Vec2 {
    // Ambition world coordinates are +Y down. The label's anchor sits above the
    // rendered source box, so subtract half-height and the configured gap.
    ae::Vec2::new(center.x, center.y - size.y * 0.5 - vertical_gap_px.max(0.0))
}

fn nameplate_font(ui_fonts: Option<&UiFonts>, font_size: f32) -> TextFont {
    ui_fonts
        .map(|fonts| fonts.text_font(font_size, UiFontWeight::Semibold))
        .unwrap_or(TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        })
}

/// Build one plate, for ONE view.
///
/// it carries no `Name`, and that is deliberate. `entity.name` is
/// registered for rollback, and the coverage contract derives its swept
/// population from *"an entity carrying even one type the rollback knows about
/// participates in rollback"* — so a debug label here would enlist every plate of
/// every view in the sim sweep. That has already happened once to the view entity
/// itself, where the ease state was immediately reported as an unrewound desync
/// risk. `ActorNameplateVisual::owner_id` plus `PresentedForView` is the
/// identity; the label is not worth the enlistment.
#[allow(clippy::too_many_arguments)]
fn spawn_actor_nameplate(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    settings: &ActorNameplateSettings,
    font: &TextFont,
    candidate: &NameplateCandidate,
    view: Entity,
) {
    let text = candidate.label.clone();
    let outline_offsets = outline_offsets(settings.outline_offset_px);
    let text_color = color_with_opacity(settings.text_color, candidate.opacity);
    let outline_color = color_with_opacity(settings.outline_color, candidate.opacity);
    let anchor = world_to_bevy(world, candidate.anchor_world, settings.z);
    commands
        .spawn_session_scoped(
            session_scope,
            (
                Text2d::new(text.clone()),
                font.clone(),
                TextColor(text_color),
                Transform::from_translation(anchor),
                if candidate.opacity > 0.0 {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ActorNameplateVisual {
                    owner_id: candidate.owner_id.clone(),
                    label: text.clone(),
                },
                {
                    let mut label =
                        WorldLabel::new(candidate.owner_id.clone(), candidate.family, anchor)
                            .with_colors(settings.text_color, Some(settings.outline_color));
                    label.owner_opacity = candidate.opacity;
                    label
                },
                RoomVisual,
                ambition_sim_view::PresentedForView(view),
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
            label_driven_bodies: None,
        };
        assert_eq!(
            settings.resolve_rank_policy(Some(&policy)),
            ResolvedNameplateRankPolicy {
                full_opacity_count: 100,
                fade_out_count: 120,
                label_driven_bodies: false,
            }
        );
    }

    /// ⭐⭐ A ROOM WITH A CAST LABELS EVERY FIGHTER THE SAME WAY.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS, Jon 2026-08-24: *"it looks like non-player 1
    /// gets a name over their head, whereas player 1 does not. This is player 1
    /// centric behavior, and we should have none of it."* The suppression was a
    /// CONSTANT — a driven body never got a plate — which reads as an honest
    /// relative rule with one driven body in the room and as pure
    /// player-centrism with four.
    #[test]
    fn a_room_with_a_cast_can_label_the_body_you_are_driving() {
        let settings = ActorNameplateSettings::default();
        // The default is the exploration answer, and it is still the default.
        assert!(!settings.resolve_rank_policy(None).label_driven_bodies);

        let cast = RoomNameplatePolicy {
            full_opacity_count: None,
            fade_out_count: None,
            label_driven_bodies: Some(true),
        };
        assert!(
            settings
                .resolve_rank_policy(Some(&cast))
                .label_driven_bodies,
            "a stage that declared its cast should be labelled uniformly still \
             hides the plate over whoever is playing"
        );

        // ⛔ AND THE OTHER UNIFORM ANSWER IS ONE VALUE AWAY — no plates at all —
        // which is the point of it being a knob rather than a second rule.
        let bare = RoomNameplatePolicy {
            full_opacity_count: None,
            fade_out_count: None,
            label_driven_bodies: Some(false),
        };
        assert!(
            !settings
                .resolve_rank_policy(Some(&bare))
                .label_driven_bodies
        );
    }

    #[test]
    fn anchor_sits_above_source_in_y_down_world_space() {
        let anchor = nameplate_anchor(ae::Vec2::new(20.0, 100.0), ae::Vec2::new(30.0, 40.0), 10.0);
        assert_eq!(anchor, ae::Vec2::new(20.0, 70.0));
    }

    /// TWO VIEWS, ONE ROOM, ONE SIMULATION — TWO SETS OF PLATES.
    ///
    /// the two-view split below is a FIXTURE, not a policy. It is the smallest world that
    /// can tell a per-view projection from a shared one.
    mod two_views_one_room_tests {
        use super::*;
        use ambition_sim_view::{CameraViewState, LocalView, LocalViewId, PresentedForView};
        use bevy::ecs::system::RunSystemOnce as _;

        const NEAR_FIRST: ae::Vec2 = ae::Vec2::new(100.0, 300.0);
        const NEAR_SECOND: ae::Vec2 = ae::Vec2::new(700.0, 300.0);

        fn room() -> ae::RoomGeometry {
            ae::RoomGeometry(ae::World::new(
                "two views",
                ae::Vec2::new(800.0, 600.0),
                ae::Vec2::new(50.0, 50.0),
                Vec::new(),
            ))
        }

        /// ONE door entity per door. The authoritative object stays
        /// singular; what the views get is one PROJECTION of it each.
        fn spawn_door(world: &mut World, id: &str, center: ae::Vec2) {
            world.spawn(DoorNameplateSource::new(
                id,
                id,
                ae::aabb_from_min_size(center - ae::Vec2::splat(20.0), ae::Vec2::splat(40.0)),
            ));
        }

        fn spawn_view(world: &mut World, id: u8, target_world: ae::Vec2) -> Entity {
            world
                .spawn((
                    LocalView,
                    LocalViewId(id),
                    CameraViewState {
                        target_world,
                        ..Default::default()
                    },
                ))
                .id()
        }

        /// Run the sync and read back, per view in spawn order, the opacity each
        /// door's plate was given.
        fn plate_opacities(first_target: ae::Vec2, second_target: ae::Vec2) -> [[f32; 2]; 2] {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );
            // One plate at full opacity, the next at exactly zero: the sharpest
            // ranking this policy can express, so the assertion is on values a
            // reader can derive rather than on a fade curve.
            world.insert_resource(ActorNameplateSettings {
                full_opacity_count: 1,
                fade_out_count: 2,
                ..Default::default()
            });
            spawn_door(&mut world, "first", NEAR_FIRST);
            spawn_door(&mut world, "second", NEAR_SECOND);
            let views = [
                spawn_view(&mut world, 0, first_target),
                spawn_view(&mut world, 1, second_target),
            ];

            world.run_system_once(sync_actor_nameplates).expect(
                "sync_actor_nameplates should run: the fixture provides the session \
                 world and the settings it reads",
            );

            let mut plates =
                world.query::<(&ActorNameplateVisual, &PresentedForView, &WorldLabel)>();
            let rows: Vec<(Entity, String, f32)> = plates
                .iter(&world)
                .map(|(plate, key, label)| (key.0, plate.owner_id.clone(), label.owner_opacity))
                .collect();
            assert_eq!(
                rows.len(),
                4,
                "two views owe two plates each — one projection of each door per \
                 view. Getting 2 means both views are still sharing one set, which \
                 is the defect: one entity cannot hold two views' rankings"
            );

            views.map(|view| {
                ["first", "second"].map(|id| {
                    rows.iter()
                        .find(|(plate_view, owner, _)| *plate_view == view && owner.as_str() == id)
                        .unwrap_or_else(|| panic!("view {view:?} has no plate for door {id}"))
                        .2
                })
            })
        }

        /// EACH VIEW RANKS THE ROOM AGAINST ITS OWN FOCUS.
        ///
        /// The policy draws the nearest few plates and fades the rest, ranked by distance to the
        /// camera's focus — so two views at opposite ends of one room want opposite answers.
        ///
        /// the assertion is on VALUES, not on inequality. "the two views
        /// differ" would pass for a pair that differ and are both wrong. Each view
        /// is checked against the opacity the rank policy gives the door it is
        /// actually looking at.
        ///
        /// and the falsifier is inside the test. The second run swaps only
        /// the two views' camera targets — same doors, same spawn order, same
        /// settings — and the two answers must swap with them. A sync that keys
        /// off view or door iteration order passes the first run and fails this.
        #[test]
        fn each_view_ranks_the_room_against_its_own_focus() {
            let looking_at_first = [1.0, 0.0];
            let looking_at_second = [0.0, 1.0];
            assert_ne!(
                looking_at_first, looking_at_second,
                "the fixture must give the two views genuinely different rankings, \
                 or nothing below can tell a per-view sync from a shared one"
            );

            assert_eq!(
                plate_opacities(NEAR_FIRST, NEAR_SECOND),
                [looking_at_first, looking_at_second],
                "each view must rank the room against ITS OWN focus; one ranking \
                 written over both views' plates is the process-global this \
                 milestone deleted"
            );

            assert_eq!(
                plate_opacities(NEAR_SECOND, NEAR_FIRST),
                [looking_at_second, looking_at_first],
                "swapping only the two camera targets must swap the two rankings. It \
                 did not, so the ranking follows iteration order and the assertion \
                 above was passing for the wrong reason"
            );
        }

        /// A RETIRED VIEW TAKES ITS PLATES WITH IT — DESPAWNED AS A SET.
        #[test]
        fn a_retired_view_takes_its_plates_with_it() {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );
            world.insert_resource(ActorNameplateSettings {
                full_opacity_count: 1,
                fade_out_count: 2,
                ..Default::default()
            });
            spawn_door(&mut world, "first", NEAR_FIRST);
            spawn_door(&mut world, "second", NEAR_SECOND);
            let first = spawn_view(&mut world, 0, NEAR_FIRST);
            let second = spawn_view(&mut world, 1, NEAR_SECOND);

            fn plate_views(world: &mut World) -> Vec<Entity> {
                let mut query = world.query::<(&ActorNameplateVisual, &PresentedForView)>();
                let mut found: Vec<Entity> = query.iter(world).map(|(_, key)| key.0).collect();
                found.sort();
                found
            }

            world
                .run_system_once(sync_actor_nameplates)
                .expect("the nameplate sync runs");
            let mut both = vec![first, first, second, second];
            both.sort();
            assert_eq!(plate_views(&mut world), both, "each view owes two plates");

            world.despawn(second);
            world
                .run_system_once(sync_actor_nameplates)
                .expect("the nameplate sync runs");
            assert_eq!(
                plate_views(&mut world),
                vec![first, first],
                "retiring a view must despawn its plates as a SET. A plate whose \
                 view is gone is drawn by the renderer and reachable by no per-view \
                 query, so nothing would ever place, fade or retire it again"
            );
        }
    }
}
