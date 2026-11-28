//! Nameplates: presentation-only world-space labels above actors and doors.
//!
//! This lives in `ambition_render`, not gameplay. Actor identity, door names,
//! and bounds are simulation state; how a label is drawn is view policy. The
//! system keeps one entity per labeled source and per view, and only changes
//! visibility, transform, and opacity each frame.

use std::collections::{HashMap, HashSet};

use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, LiveRoomInstance, LiveRooms, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::RoomNameplatePolicy;
use ambition_sim_view::NameplateIndex;
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};

use super::label_layout::{with_opacity, WorldLabel, WorldLabelFamily, WorldLabelStyle};
use super::primitives::RoomVisual;

/// Presentation policy for world nameplates.
///
/// The default ranks eligible labels by distance to
/// [`CameraViewState::target_world`], draws the first five at full opacity,
/// fades the sixth, and reaches zero opacity at the seventh. Later candidates
/// are hidden. Active-room metadata (LDtk level fields) can override the rank
/// thresholds.
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

/// Marker on a room visual that takes part in the nameplate policy.
///
/// Actor labels come from the nameplate read model because their bounds are
/// dynamic. Static door visuals carry this component so they share the same
/// ranking, fade, and render path.
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

/// Marker on the root `Text2d` entity of a nameplate. `owner_id` is the
/// source's stable id (an actor's feature id or a door's zone id), never a sim
/// `Entity`.
#[derive(Component, Clone, Debug)]
pub struct ActorNameplateVisual {
    pub owner_id: String,
    pub label: String,
}

/// System set for nameplates, so other code can order against it.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActorNameplateSet;

/// Render-layer plugin for player-facing actor and door labels.
///
/// It publishes candidates into the shared world-label placement pass,
/// [`WorldLabelLayoutPlugin`](super::label_layout::WorldLabelLayoutPlugin).
/// It adds that plugin, and so does the generic room-visuals plugin, because
/// signage is a world label even without nameplates.
pub struct ActorNameplatePresentationPlugin;

impl Plugin for ActorNameplatePresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::label_layout::WorldLabelLayoutPlugin);
        app.init_resource::<ActorNameplateSettings>()
            .configure_sets(
                Update,
                ActorNameplateSet
                    .after(super::actors::sync_visuals)
                    .after(super::camera::ViewCameraSet),
            )
            // The placement pass must run after every family publishes its anchor,
            // or it places against last frame. This is configured here because
            // only a composition with actor plates has `ActorNameplateSet`.
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
/// One plate a view may show. It borrows its id and words from the frame's
/// sources (the nameplate index and the door snapshot), so ranking a room
/// full of bodies copies no string; a new plate copies them when spawned.
struct NameplateCandidate<'a> {
    owner_id: &'a str,
    label: &'a str,
    /// Placement family. A door plate names a static fixture and yields
    /// only to authored signage. An actor plate moves and absorbs
    /// displacement.
    family: WorldLabelFamily,
    anchor_world: ae::Vec2,
    distance_sq: f32,
    opacity: f32,
}

struct ViewNameplates<'a> {
    room: LiveRoomInstance,
    world: &'a ae::World,
    wanted: HashMap<&'a str, NameplateCandidate<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedNameplateRankPolicy {
    full_opacity_count: usize,
    fade_out_count: usize,
    /// Does a driven body get a plate? See
    /// [`RoomNameplatePolicy::label_driven_bodies`]. A room with a cast
    /// overrides the exploration default.
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
            // Default `false` (no plate on a driven body) is the exploration rule,
            // correct only with one driven body. A room with a cast overrides it.
            label_driven_bodies: room_policy
                .and_then(|policy| policy.label_driven_bodies)
                .unwrap_or(false),
        }
    }
}

/// One set of plates per view.
///
/// Which plates are on screen depends on the view. The policy ranks
/// candidates by distance to the camera focus, so two views at opposite ends
/// of a room want different plates and opacities. So plates are keyed by
/// view. A one-view game builds the same plates as before.
///
/// Each view projects the sources in the live room it frames.
#[allow(clippy::type_complexity)]
pub fn sync_actor_nameplates(
    mut commands: Commands,
    // Each view shows the plates of the live room it frames, placed by that
    // room's geometry and ranked by that room's policy.
    geometry: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    live: LiveRooms,
    settings: Res<ActorNameplateSettings>,
    active_session: Option<Res<ActiveSessionScope>>,
    room_specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    // A draw system draws every view, so it iterates them.
    // `PresentedViewState` answers a different question (which one view a
    // camera shows) and refuses when there are several.
    views: Query<
        (
            Entity,
            &ambition_sim_view::CameraViewState,
            Option<&ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot>,
        ),
        With<ambition_sim_view::LocalView>,
    >,
    nameplate_index: Option<Res<NameplateIndex>>,
    ui_fonts: Option<Res<UiFonts>>,
    doors: Query<(Entity, &DoorNameplateSource, Option<&Visibility>)>,
    mut plates: Query<(
        Entity,
        &ActorNameplateVisual,
        &ambition_sim_view::PresentedForView,
        &InRoomInstance,
        &mut WorldLabel,
    )>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    if !settings.enabled {
        for (_, _, _, _, mut label) in &mut plates {
            if label.owner_opacity != 0.0 {
                label.owner_opacity = 0.0;
            }
        }
        return;
    }

    let doors: Vec<_> = doors.iter().map(|(entity, source, visibility)| {
        let shown = !visibility.is_some_and(|visibility| *visibility == Visibility::Hidden)
            && !source.label.trim().is_empty();
        (live.of(entity), source, shown)
    }).collect();
    // Keep hidden plates only while their sources stay in the room.
    let mut source_ids = HashSet::new();
    if let Some(index) = nameplate_index.as_deref() {
        source_ids.extend(index.iter().map(|(id, fact)| (fact.room, id)));
    }
    source_ids.extend(doors.iter().map(|(room, source, _)| (*room, source.id.as_str())));

    let mut wanted: HashMap<Entity, ViewNameplates> = HashMap::new();
    for (view_entity, view_state, resolved) in &views {
        let Some((view_room, world)) = resolved
            .and_then(|resolved| resolved.frame())
            .map(|frame| frame.room)
            .or_else(|| live.sole())
            .and_then(|room| Some((room, &geometry.in_room(room)?.0)))
        else {
            continue;
        };
        let rank_policy = settings.resolve_rank_policy(room_specs.as_ref().and_then(|specs| {
            let definition = specs.definition_in(view_room)?;
            Some(&specs.rooms().spec(definition).metadata.nameplate_policy)
        }));
        let focus_world = view_state.target_world;
        let mut candidates = Vec::new();
        if let Some(index) = nameplate_index.as_deref() {
            collect_actor_candidates(&settings, rank_policy, index, view_room, focus_world, &mut candidates);
        }
        collect_door_candidates(&settings, view_room, focus_world, &doors, &mut candidates);

        candidates.sort_by(|a, b| {
            a.distance_sq
                .total_cmp(&b.distance_sq)
                .then_with(|| a.label.cmp(&b.label))
        });
        apply_rank_opacity(rank_policy, &mut candidates);

        wanted.insert(view_entity, ViewNameplates {
            room: view_room,
            world,
            wanted: candidates
                .into_iter()
                .take(rank_policy.fade_out_count)
                .map(|candidate| (candidate.owner_id, candidate))
                .collect(),
        });
    }

    // This system publishes each plate's wanted anchor and opacity. The shared
    // placement pass (`label_layout`) owns transform, visibility, and colour,
    // because plates are ranked against signage and door plates too (AC12).
    let style = WorldLabelStyle::outlined(settings.text_color, settings.outline_color, settings.outline_offset_px);
    let mut existing_visible: HashSet<(Entity, &str)> = HashSet::new();
    for (entity, plate, key, stamp, mut label) in &mut plates {
        let Some(view) = wanted.get(&key.0).filter(|view| view.room == stamp.0) else {
            commands.entity(entity).try_despawn();
            continue;
        };
        if let Some(candidate) = view.wanted.get(plate.owner_id.as_str()) {
            if plate.label != candidate.label {
                commands.entity(entity).try_despawn();
                continue;
            }
            existing_visible.insert((key.0, candidate.owner_id));
            let anchor = world_to_bevy(view.world, candidate.anchor_world, settings.z);
            if label.family != candidate.family
                || label.anchor != anchor
                || label.owner_opacity != candidate.opacity
                || label.style != style
            {
                label.family = candidate.family;
                label.anchor = anchor;
                label.owner_opacity = candidate.opacity;
                label.style = style;
            }
        } else if source_ids.contains(&(Some(stamp.0), plate.owner_id.as_str())) {
            if label.owner_opacity != 0.0 {
                label.owner_opacity = 0.0;
            }
        } else {
            commands.entity(entity).try_despawn();
        }
    }

    let font = nameplate_font(ui_fonts.as_deref(), settings.font_size);
    for (view_entity, view) in &wanted {
        for candidate in view.wanted.values() {
            if existing_visible.contains(&(*view_entity, candidate.owner_id)) {
                continue;
            }
            spawn_actor_nameplate(
                &mut commands,
                session_scope.in_room(Some(view.room)),
                view.world,
                &settings,
                &font,
                candidate,
                *view_entity,
            );
        }
    }
}

fn collect_actor_candidates<'a>(
    settings: &ActorNameplateSettings,
    rank_policy: ResolvedNameplateRankPolicy,
    index: &'a NameplateIndex,
    view_room: LiveRoomInstance,
    focus_world: ae::Vec2,
    candidates: &mut Vec<NameplateCandidate<'a>>,
) {
    for (id, fact) in index.iter() {
        // A body in another live room is not in this view's room.
        if fact.room != Some(view_room) {
            continue;
        }
        // The room decides whether a driven body is labelled. Hiding it is
        // correct with one driven body. With a cast it would label everyone
        // except the human, which is player-1-centric.
        if fact.driven && !rank_policy.label_driven_bodies {
            continue;
        }
        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            id,
            &fact.label,
            WorldLabelFamily::Actor,
            fact.center,
            fact.size,
        );
    }
}

fn collect_door_candidates<'a>(
    settings: &ActorNameplateSettings,
    view_room: LiveRoomInstance,
    focus_world: ae::Vec2,
    doors: &[(Option<LiveRoomInstance>, &'a DoorNameplateSource, bool)],
    candidates: &mut Vec<NameplateCandidate<'a>>,
) {
    for (room, source, shown) in doors {
        if !*shown || *room != Some(view_room) {
            continue;
        }
        push_candidate_if_in_range(
            settings,
            focus_world,
            candidates,
            &source.id,
            &source.label,
            WorldLabelFamily::Fixture,
            source.center_world,
            source.size_world,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_candidate_if_in_range<'a>(
    settings: &ActorNameplateSettings,
    focus_world: ae::Vec2,
    candidates: &mut Vec<NameplateCandidate<'a>>,
    owner_id: &'a str,
    label: &'a str,
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
    // World +Y is down. The anchor sits above the source box, so subtract
    // half the height and the gap.
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

/// Build one plate for one view.
///
/// It has no `Name` on purpose. `entity.name` is registered for rollback, and
/// the coverage contract includes any entity with a rollback-known type in the
/// sim sweep. A `Name` would enlist every plate of every view. The identity is
/// `ActorNameplateVisual::owner_id` plus `PresentedForView`.
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
    let text = candidate.label.to_string();
    let style = WorldLabelStyle::outlined(settings.text_color, settings.outline_color, settings.outline_offset_px);
    let text_color = with_opacity(settings.text_color, candidate.opacity);
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
                    owner_id: candidate.owner_id.to_string(),
                    label: text.clone(),
                },
                {
                    let mut label =
                        WorldLabel::new(candidate.owner_id.to_string(), candidate.family, anchor)
                            .with_style(style);
                    label.owner_opacity = candidate.opacity;
                    label
                },
                RoomVisual,
                ambition_sim_view::PresentedForView(view),
            ),
        );
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

    /// A room with a cast labels every fighter the same way.
    ///
    /// A constant suppression gave every body except player 1 a plate, which is
    /// player-1-centric with more than one driven body.
    #[test]
    fn a_room_with_a_cast_can_label_the_body_you_are_driving() {
        let settings = ActorNameplateSettings::default();
        // The exploration answer is still the default.
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

        // The other uniform answer (no plates at all) is one value away. That is
        // why it is a knob, not a second rule.
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

    /// Two views, one room, one simulation: two sets of plates.
    ///
    /// The two-view split is a fixture, not a policy. It is the smallest world
    /// that can tell a per-view projection from a shared one.
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

        /// One door entity per door. The source stays singular; each view gets
        /// its own projection.
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

        #[test]
        fn a_view_replaces_its_plate_when_it_frames_another_live_room() {
            use ambition_platformer2d_shared_tangle::lifecycle::{spawn_live_room, InRoomInstance, LiveRoomInstance};
            use ambition_sim_view::camera_snapshot::{ResolvedCameraFrame, ResolvedCameraSnapshot};

            let mut world = World::new();
            world.init_resource::<ActorNameplateSettings>();
            let east = LiveRoomInstance::ACTIVATION;
            let west = east.next();
            let [east_root, _] = [east, west].map(|id| spawn_live_room(&mut world, id, room()));
            let [_, west_door] = [(east, NEAR_FIRST), (west, NEAR_SECOND)].map(|(room, center)| {
                world.spawn((
                    InRoomInstance(room),
                    DoorNameplateSource::new("door", "Door", ae::Aabb::new(center, ae::Vec2::splat(20.0))),
                )).id()
            });
            let frame = |room| ResolvedCameraSnapshot(Some(ResolvedCameraFrame {
                room,
                snapshot: Default::default(),
                follow_world: Default::default(),
            }));
            let first = spawn_view(&mut world, 0, NEAR_FIRST);
            let second = spawn_view(&mut world, 1, NEAR_SECOND);
            world.entity_mut(first).insert(frame(east));
            world.entity_mut(second).insert(frame(west));
            let plates = |world: &mut World| {
                world.query_filtered::<(Entity, &PresentedForView, &InRoomInstance, &WorldLabel), With<ActorNameplateVisual>>()
                    .iter(world)
                    .map(|(entity, view, stamp, label)| (view.0, (entity, stamp.0, label.anchor)))
                    .collect::<HashMap<_, _>>()
            };
            world.run_system_once(sync_actor_nameplates).unwrap();
            let before = plates(&mut world);
            assert_eq!(before[&first].1, east);
            assert_eq!(before[&second].1, west);
            assert_ne!(before[&first].2, before[&second].2);

            world.entity_mut(first).insert(frame(west));
            world.run_system_once(sync_actor_nameplates).unwrap();
            let moved = plates(&mut world);
            assert_eq!(moved[&first].1, west);
            assert_eq!(moved[&first].2, before[&second].2);
            assert_eq!(moved[&second], before[&second]);
            assert!(world.get_entity(before[&first].0).is_err());

            world.despawn(east_root);
            world.run_system_once(sync_actor_nameplates).unwrap();
            assert_eq!(plates(&mut world), moved);

            world.despawn(west_door);
            world.run_system_once(sync_actor_nameplates).unwrap();
            assert!(plates(&mut world).is_empty());
        }

        /// Run the sync and read back, per view in spawn order, the opacity each
        /// door's plate was given.
        fn plate_opacities(first_target: ae::Vec2, second_target: ae::Vec2) -> [[f32; 2]; 2] {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
                &mut world,
                room(),
            );
            // One plate at full opacity and the next at zero: the sharpest ranking
            // the policy allows, so the expected values are easy to derive.
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

        /// Each view ranks the room against its own focus.
        ///
        /// The test checks values, not only that the views differ. Each view is
        /// compared with the opacity the rank policy gives the door it looks at.
        ///
        /// The second run swaps only the camera targets, and the answers must
        /// swap too. A sync that keys off iteration order fails that run.
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

        /// A retired view takes its plates with it.
        #[test]
        fn a_retired_view_takes_its_plates_with_it() {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
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

        /// View half, cut V2e: each view shows the plates of the live room it
        /// frames, placed by that room's geometry. Two live rooms (800 wide and
        /// 400 wide), a door in each at x = 100, a view framing each room.
        /// Each view has one plate, its own room's door, anchored by its own
        /// room's flip. Before V2e this road read the sole live room and drew
        /// no plate while two rooms were live.
        #[test]
        fn each_view_shows_the_plates_of_its_own_live_room() {
            use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
            let first = LiveRoomInstance::ACTIVATION;
            let second = first.next();
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(&mut world, room());
            ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
                &mut world,
                second,
                ae::RoomGeometry(ae::World::new("small", ae::Vec2::new(400.0, 300.0), ae::Vec2::new(50.0, 50.0), Vec::new())),
            );
            world.insert_resource(ActorNameplateSettings::default());
            for (id, room) in [("big_door", first), ("small_door", second)] {
                world.spawn((
                    DoorNameplateSource::new(
                        id,
                        id,
                        ae::aabb_from_min_size(ae::Vec2::new(80.0, 80.0), ae::Vec2::splat(40.0)),
                    ),
                    InRoomInstance(room),
                ));
            }
            let views = [first, second].map(|room| {
                let view = spawn_view(&mut world, if room == first { 0 } else { 1 }, ae::Vec2::new(100.0, 100.0));
                world.entity_mut(view).insert(ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot(Some(
                    ambition_sim_view::camera_snapshot::ResolvedCameraFrame {
                        snapshot: Default::default(),
                        follow_world: Default::default(),
                        room,
                    },
                )));
                view
            });
            world.run_system_once(sync_actor_nameplates).expect("the sync runs");
            let mut plates: Vec<(Entity, String, f32)> = world
                .query::<(&ActorNameplateVisual, &PresentedForView, &WorldLabel)>()
                .iter(&world)
                .map(|(plate, key, label)| (key.0, plate.owner_id.clone(), label.anchor.x))
                .collect();
            plates.sort_by(|a, b| a.1.cmp(&b.1));
            assert_eq!(
                plates,
                vec![
                    (views[0], "big_door".to_string(), 100.0 - 400.0),
                    (views[1], "small_door".to_string(), 100.0 - 200.0),
                ],
                "a view showed another live room's plate, or placed its own by another room"
            );
        }
    }
}
