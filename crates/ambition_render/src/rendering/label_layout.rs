//! One ranked placement pass over every world-space text label.
//!
//! ## Why this exists (queue row AC12)
//!
//! Spacing within a family cannot stop a cross-family overlap. So placement
//! happens here, and every label takes part by carrying a [`WorldLabel`].
//!
//! ## The two mechanisms
//!
//! Label vs label: displacement, in rank order. The ranking is
//! [`WorldLabelFamily`]'s declaration order. The family that yields is the one
//! that can move without a visible jump: an actor plate already moves every
//! frame with its actor.
//!
//! Label vs the controlled subject: fade, never displacement. A driven body
//! walks under a static sign often; moving the sign would make it twitch.
//! Dimming keeps the sign legible, keeps the body visible, and is stable.
//!
//! The subject is whoever is driving, read from [`ControlledBodiesView`], not
//! "the player". A possessed enemy and both fighters in a couch match get the
//! same protection.

use ambition_platformer2d_core as ae;
use ambition_sim_view::ControlledBodiesView;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;

use crate::ui_fonts::{UiFontWeight, UiFonts};

/// Which family a world-space label belongs to. Declaration order is the
/// ranking: an earlier variant is placed first and never yields to a later
/// one.
///
/// The order is by the cost of yielding, not by importance. Families that
/// cannot move without a visible twitch go first; the family already in motion
/// absorbs the displacement.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldLabelFamily {
    /// Authored room signage (an LDtk `DebugLabel`, a chest's name). Static
    /// and hand-placed, so it is displaced only by another sign.
    Signage,
    /// A plate naming a static world fixture — a door, a non-door loading
    /// zone. Static, so it yields only to signage.
    Fixture,
    /// A plate naming an actor. It tracks a moving body, so it yields to
    /// everything static; a nudge looks like its normal motion.
    Actor,
    /// A line somebody just said: a speech bubble.
    ///
    /// Last, so it yields to the actor plate. A plate stays on a body, and
    /// displacing it makes it hop once per taunt, on an element the eye uses to
    /// track who is who. A bubble already rises (`SPEECH_BUBBLE_BASE_RISE`)
    /// and fades over its ~2.2 s life, so a nudge is not visible.
    Speech,
}

/// Marks a `Text2d` entity as a world-space label and carries everything the
/// placement pass needs.
///
/// This pass is the only writer of the label's `Transform`, `Visibility`, and
/// `TextColor`. The owning system writes only this component (the wanted
/// anchor and opacity). With two writers, a pass that reads back its own last
/// transform accumulates its correction and the label drifts.
#[derive(Component, Clone, Debug)]
pub struct WorldLabel {
    /// Stable view identity of the labeled thing. Used only as the final
    /// deterministic tiebreak, because the actor-plate source index iterates
    /// in hash order.
    pub owner_id: String,
    pub family: WorldLabelFamily,
    /// Where the owner wants the label, in Bevy world space, including Z. The
    /// pass always places from here, never from the current transform.
    pub anchor: Vec3,
    /// The opacity the owner asked for (a nameplate's rank fade; 1.0 for
    /// static signage). The pass may only reduce it.
    pub owner_opacity: f32,
    /// The label's colour at full opacity.
    pub text_color: Color,
    /// The colour for outline/shadow children, at full opacity. `None` for
    /// labels drawn without an outline pass.
    pub outline_color: Option<Color>,
    /// The opacity drawn last frame, eased toward the resolved target.
    ///
    /// Owned by the pass, never by an owner. Without it the subject fade is a
    /// hard cut that pops each step near the edge of a sign.
    pub rendered_opacity: f32,
}

impl WorldLabel {
    pub fn new(owner_id: impl Into<String>, family: WorldLabelFamily, anchor: Vec3) -> Self {
        Self {
            owner_id: owner_id.into(),
            family,
            anchor,
            owner_opacity: 1.0,
            text_color: Color::WHITE,
            outline_color: None,
            // Starts at zero so a label eases in when its room loads.
            rendered_opacity: 0.0,
        }
    }

    pub fn with_colors(mut self, text_color: Color, outline_color: Option<Color>) -> Self {
        self.text_color = text_color;
        self.outline_color = outline_color;
        self
    }
}

/// Marks a world label that the room spawned once. The mirror below owns its
/// per-view copies.
///
/// It keeps the mirror away from nameplates. Actor and door plates also carry
/// [`WorldLabel`], but `sync_actor_nameplates` builds them per view, with an
/// outline-child subtree. Static signage and fixture plates are spawned once at
/// room load without a view, so they need mirroring.
#[derive(Component, Clone, Copy, Debug)]
pub struct StaticWorldLabel;

/// A mirrored copy of a static world label, naming its source label. When the
/// room despawns the root, the copy goes with it.
#[derive(Component, Clone, Copy, Debug)]
pub struct MirroredWorldLabel {
    pub root: Entity,
}

/// One drawn copy of every static world label per live view.
///
/// One entity cannot hold two views' transforms. A sign is ranked against its
/// view's focus, displaced by that view's other labels, and dimmed when that
/// view's controlled body walks under it. No single value is right for two
/// views.
///
/// The lowest-id view claims the room-spawned label, so a one-view game keeps
/// one entity per sign. Views past the first get copies.
///
/// The claim is keyed on `LocalViewId`, not query order, so it is the same on
/// every frame and run.
///
/// When the root's view goes away, the root is re-keyed onto the new lowest
/// view, not removed.
pub fn mirror_static_world_labels_per_view(
    mut commands: Commands,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::LocalView>>,
    roots: Query<
        (
            Entity,
            &Text2d,
            &TextFont,
            &TextColor,
            &WorldLabel,
            Option<&ambition_sim_view::PresentedForView>,
        ),
        (With<StaticWorldLabel>, Without<MirroredWorldLabel>),
    >,
    copies: Query<
        (
            Entity,
            &MirroredWorldLabel,
            &ambition_sim_view::PresentedForView,
        ),
        With<StaticWorldLabel>,
    >,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt as _;

    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };

    let mut ordered: Vec<(ambition_sim_view::LocalViewId, Entity)> =
        views.iter().map(|(view, id)| (*id, view)).collect();
    ordered.sort_by_key(|(id, _)| *id);
    let Some((_, root_view)) = ordered.first().copied() else {
        // No observation seam in this composition, so nothing to mirror.
        // `ambition_sim_view::ViewsOnHand` treats no views as quiet too.
        return;
    };

    // Retract before spawning, so a view that went away takes its whole set
    // with it.
    let live: std::collections::HashSet<Entity> = ordered.iter().map(|(_, view)| *view).collect();
    let mut mirrored: std::collections::HashSet<(Entity, Entity)> =
        std::collections::HashSet::new();
    for (entity, copy, key) in &copies {
        let root_is_gone = roots.get(copy.root).is_err();
        // `key.0 == root_view` is the re-key case: the root already draws for
        // that view, so the copy is a duplicate.
        if root_is_gone || !live.contains(&key.0) || key.0 == root_view {
            commands.entity(entity).despawn();
            continue;
        }
        mirrored.insert((copy.root, key.0));
    }

    for (root, text, font, color, label, key) in &roots {
        if key.map(|key| key.0) != Some(root_view) {
            commands
                .entity(root)
                .insert(ambition_sim_view::PresentedForView(root_view));
        }
        for (_, view) in ordered.iter().skip(1) {
            if mirrored.contains(&(root, *view)) {
                continue;
            }
            let mut copied_label = label.clone();
            // Own ease state: a new copy fades in instead of inheriting the root's
            // opacity for a different view.
            copied_label.rendered_opacity = 0.0;
            commands.spawn_session_scoped(
                session_scope,
                (
                    text.clone(),
                    font.clone(),
                    TextColor(color.0),
                    // From the anchor, not the root's transform, which is already
                    // displaced by its own view's placement.
                    Transform::from_translation(label.anchor),
                    copied_label,
                    StaticWorldLabel,
                    MirroredWorldLabel { root },
                    ambition_sim_view::PresentedForView(*view),
                    super::primitives::RoomVisual,
                    // No `Name`: `entity.name` is registered for rollback, so it
                    // would enlist a whole view's presentation set in the sim sweep.
                ),
            );
        }
    }
}

/// Tunables for the placement pass.
#[derive(Resource, Clone, Debug)]
pub struct WorldLabelLayoutSettings {
    /// Off-switch. When disabled, every label draws at its owner's anchor and
    /// opacity (the pre-AC12 behaviour).
    pub enabled: bool,
    /// Empty space required between two labels' boxes, world px.
    pub padding_px: f32,
    /// How far a label may move up from its anchor (+Y in Bevy space, world
    /// px) before it is hidden instead. Far from its owner, a label no
    /// longer names it. Sized for four lines of world text in one cluster
    /// (a four-fighter match's taunts).
    pub max_displacement_px: f32,
    /// Opacity multiplier for a label that overlaps a driven body. Low enough
    /// that the body shows through, high enough that the label stays legible.
    pub occluded_opacity: f32,
    /// Seconds for an opacity change to reach ~63% of its new target. Zero
    /// makes every change a hard cut.
    pub opacity_ease_secs: f32,
    /// How much of a driven body's box area a label must cover before it
    /// yields. Stops a one-pixel graze from dimming a whole sign.
    pub min_body_coverage: f32,
    /// Fallback advance width per character, as a fraction of font size. Used
    /// only until Bevy's text pipeline measures the label.
    pub fallback_advance_ratio: f32,
    /// Fallback line height as a fraction of font size, same caveat.
    pub fallback_line_ratio: f32,
}

impl Default for WorldLabelLayoutSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            padding_px: 3.0,
            max_displacement_px: 96.0,
            occluded_opacity: 0.3,
            opacity_ease_secs: 0.09,
            min_body_coverage: 0.05,
            fallback_advance_ratio: 0.5,
            fallback_line_ratio: 1.25,
        }
    }
}

/// System set for the placement pass. It must run after every family has
/// published its anchor for the frame.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorldLabelLayoutSet;

/// An axis-aligned box in Bevy world space, centred on `center`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LabelBox {
    pub center: Vec2,
    pub half: Vec2,
}

impl LabelBox {
    pub(crate) fn overlaps(&self, other: &LabelBox, padding: f32) -> bool {
        let pad = padding.max(0.0);
        (self.center.x - other.center.x).abs() < self.half.x + other.half.x + pad
            && (self.center.y - other.center.y).abs() < self.half.y + other.half.y + pad
    }

    /// What fraction of `body`'s area this box covers.
    ///
    /// The subject fade uses coverage, not bare intersection, so a pixel graze
    /// does not dim a large sign. Walking under a sign grazes it often, so bare
    /// intersection would flicker.
    ///
    /// The intersection is clamped per axis, not computed as
    /// `sumHalf - |delta|`. That penetration-depth form is wrong here: a 300px
    /// label containing a 32px body reports 316px of x-overlap instead of 32.
    pub(crate) fn coverage_of(&self, body: &LabelBox) -> f32 {
        let area = (body.half.x * 2.0) * (body.half.y * 2.0);
        if area <= 0.0 {
            return 0.0;
        }
        let axis = |a_center: f32, a_half: f32, b_center: f32, b_half: f32| {
            ((a_center + a_half).min(b_center + b_half)
                - (a_center - a_half).max(b_center - b_half))
            .max(0.0)
        };
        let x = axis(self.center.x, self.half.x, body.center.x, body.half.x);
        let y = axis(self.center.y, self.half.y, body.center.y, body.half.y);
        (x * y) / area
    }
}

/// One label's input to — and result from — the pure resolver.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelPlacement {
    pub owner_id: String,
    pub family: WorldLabelFamily,
    /// Rank key within a family: squared distance to the camera focus. Nearer
    /// labels are placed first, like the nameplate ranking.
    pub distance_sq: f32,
    pub anchor: Vec2,
    /// Full drawn size of the text.
    pub size: Vec2,
    pub owner_opacity: f32,
    /// Resolved position: `Some(anchor)` unless the label had to yield, and
    /// `None` when there was nowhere to put it.
    ///
    /// An `Option`, not a position plus a "fits" flag. With a flag, the apply
    /// phase could read a stale position: the label would snap back into the
    /// collision and fade out there, visibly stacked. `None` prevents that.
    pub placed: Option<Vec2>,
    /// Resolved opacity. Zero means the label could not be placed or the
    /// owner asked for zero.
    pub opacity: f32,
}

/// The whole policy, as a pure function over boxes.
///
/// Pure so the tests can build overlapping boxes and assert that the resolved
/// boxes do not overlap.
pub(crate) fn resolve_label_layout(
    labels: &mut [LabelPlacement],
    subjects: &[LabelBox],
    settings: &WorldLabelLayoutSettings,
) {
    labels.sort_by(|a, b| {
        a.family
            .cmp(&b.family)
            .then_with(|| a.distance_sq.total_cmp(&b.distance_sq))
            .then_with(|| a.owner_id.cmp(&b.owner_id))
    });

    let mut occupied: Vec<LabelBox> = Vec::with_capacity(labels.len());
    for label in labels.iter_mut() {
        label.placed = Some(label.anchor);
        label.opacity = label.owner_opacity;
        if label.opacity <= 0.0 {
            // An invisible label reserves no space, so it cannot push a visible
            // neighbour.
            continue;
        }

        let half = label.size * 0.5;
        let mut candidate = LabelBox {
            center: label.anchor,
            half,
        };
        let mut resolved = None;
        // Lift just above the highest box it overlaps. Each pass raises the
        // highest blocker it can still meet, so this settles in at most one
        // pass per placed label. The loop bound states that.
        for _ in 0..=occupied.len() {
            let blocked_to = occupied
                .iter()
                .filter(|placed| candidate.overlaps(placed, settings.padding_px))
                .map(|placed| placed.center.y + placed.half.y)
                .max_by(f32::total_cmp);
            let Some(top) = blocked_to else {
                resolved = Some(candidate);
                break;
            };
            let lifted = top + half.y + settings.padding_px;
            if lifted - label.anchor.y > settings.max_displacement_px {
                break;
            }
            candidate.center.y = lifted;
        }

        let placed = match resolved {
            Some(candidate) => {
                label.placed = Some(candidate.center);
                occupied.push(candidate);
                candidate.center
            }
            None => {
                // Nowhere to stand. Hide it rather than stack it. `None` makes this
                // true during the transition too, not only in the steady state.
                label.placed = None;
                label.opacity = 0.0;
                continue;
            }
        };

        let body = LabelBox {
            center: placed,
            half,
        };
        if subjects
            .iter()
            .any(|subject| body.coverage_of(subject) >= settings.min_body_coverage)
        {
            label.opacity *= settings.occluded_opacity.clamp(0.0, 1.0);
        }
    }
}

/// The pixel size a label's font asks for.
///
/// This pass has no viewport, so a viewport- or rem-relative `FontSize` cannot
/// be resolved here. All Ambition label sizes are in pixels. If that changes,
/// this function must learn the viewport.
pub(crate) fn label_font_px(font: &TextFont) -> f32 {
    match font.font_size {
        FontSize::Px(px) => px,
        other => other.eval(Vec2::ZERO, bevy::text::RemSize::default().0),
    }
}

/// Measure a label. Use Bevy's text layout when present; on a label's first
/// frame, before layout runs, use a per-character estimate.
///
/// The estimate errs wide (proportional fonts vary), because a size that is
/// too small causes the overlap this module removes.
pub(crate) fn label_size(
    measured: Option<Vec2>,
    text: &str,
    font_size: f32,
    settings: &WorldLabelLayoutSettings,
) -> Vec2 {
    if let Some(size) = measured {
        if size.x > 0.0 && size.y > 0.0 {
            return size;
        }
    }
    let widest_line = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let lines = text.lines().count().max(1) as f32;
    Vec2::new(
        widest_line * font_size * settings.fallback_advance_ratio,
        lines * font_size * settings.fallback_line_ratio,
    )
}

/// Bodies a label must not obscure: every body somebody is driving.
///
/// Read from the sim's [`ControlledBodiesView`], not from render entities,
/// so presentation does not depend on simulation layout.
///
/// Do not use `NameplateIndex`'s `controlled` flag. That index has only
/// `FeatureId` rows, and the home avatar has none, so the rule would protect
/// every body except the one you normally play.
fn controlled_body_boxes(view: Option<&ControlledBodiesView>, world: &ae::World) -> Vec<LabelBox> {
    let Some(view) = view else {
        return Vec::new();
    };
    view.0
        .iter()
        .map(|fact| LabelBox {
            center: ae::config::world_to_bevy(world, fact.center, 0.0).truncate(),
            half: Vec2::new(fact.size.x * 0.5, fact.size.y * 0.5),
        })
        .collect()
}

/// The typeface a family is drawn in.
///
/// Rule (queue row AC20): all world text uses one family, and the weight
/// carries the role. Signage is designer prose, so it is Regular. A plate is a
/// name read at a glance, so it is Semibold. Mono is not used: the only
/// monospace asset is the debug HUD font.
fn font_weight_for(family: WorldLabelFamily) -> UiFontWeight {
    match family {
        // A spoken line is prose, so it takes signage's weight.
        WorldLabelFamily::Signage | WorldLabelFamily::Speech => UiFontWeight::Regular,
        WorldLabelFamily::Fixture | WorldLabelFamily::Actor => UiFontWeight::Semibold,
    }
}

/// Put every world label on its family's typeface.
///
/// This is the assignment, not a repair. Room signage
/// (`rendering::primitives`) spawns with a size and no source, because a room
/// load has no `UiFonts`. This pass gives it a face. Nameplates resolve at
/// spawn; for them this is one comparison.
///
/// Since Bevy 0.19 the source is a family, and
/// `load_font_assets_into_font_collection` marks a `TextFont` changed when its
/// family resolves. So a label spawned before its font loads is fixed
/// upstream. `a_label_spawned_before_its_font_still_ends_up_on_it` checks this.
///
/// Assign only on inequality: writing `TextFont` re-runs text layout, and a
/// `Mut` deref marks it changed.
pub fn apply_world_label_fonts(
    ui_fonts: Option<Res<UiFonts>>,
    mut labels: Query<(&WorldLabel, &mut TextFont)>,
) {
    let Some(fonts) = ui_fonts.as_deref() else {
        return;
    };
    for (label, mut font) in &mut labels {
        // Family and weight are one request. Assigning only the family draws
        // every plate at Regular. Guarded by
        // `a_label_spawned_before_its_font_still_ends_up_on_it`.
        let wanted = fonts.text_font(font.font_size, font_weight_for(label.family));
        if font.font != wanted.font || font.weight != wanted.weight {
            font.font = wanted.font;
            font.weight = wanted.weight;
        }
    }
}

/// The pass. Places every [`WorldLabel`] and writes the result, once per
/// view, over that view's own labels.
///
/// Each iteration has the view's own
/// [`CameraViewState`](ambition_sim_view::CameraViewState), so no focus is
/// invented. A view with no camera costs a pass; two cameras on one view
/// share one set.
#[allow(clippy::type_complexity)]
pub fn layout_world_labels(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    settings: Res<WorldLabelLayoutSettings>,
    time: Res<Time>,
    // A draw system draws every view. `PresentedViewState` resolves the one
    // view a single main camera shows and refuses when there are several.
    views: Query<(Entity, &ambition_sim_view::CameraViewState), With<ambition_sim_view::LocalView>>,
    controlled_bodies: Option<Res<ControlledBodiesView>>,
    mut labels: Query<(
        &mut WorldLabel,
        &Text2d,
        &TextFont,
        Option<&TextLayoutInfo>,
        &mut Transform,
        &mut Visibility,
        &mut TextColor,
        Option<&Children>,
        Option<&ambition_sim_view::PresentedForView>,
    )>,
    mut outline_colors: Query<&mut TextColor, Without<WorldLabel>>,
) {
    let ease = ease_fraction(settings.opacity_ease_secs, time.delta_secs());
    if !settings.enabled {
        // The policy is declined for all views: every label draws at its
        // owner's anchor and opacity.
        for (mut label, _, _, _, mut transform, mut visibility, mut text_color, children, _) in
            &mut labels
        {
            transform.translation = label.anchor;
            let opacity = label.owner_opacity;
            label.rendered_opacity = opacity;
            *visibility = visibility_for(opacity);
            *text_color = TextColor(with_opacity(label.text_color, opacity));
            paint_outlines(&mut outline_colors, children, &label, opacity);
        }
        return;
    }

    // No views: nobody to lay out text for. A composed host spawns the view
    // at plugin build time, so this means no observation seam.
    if views.is_empty() {
        return;
    }

    // A label's view is decided like a camera's view, by
    // `ambition_sim_view::ViewsOnHand`. A keyed label belongs to its view. An
    // unkeyed label belongs to the only view in a one-view composition (hand
    // probes, labels the mirror has not reached). With several views it is
    // refused.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, _)| view));

    // View-independent: a driven body is driven in every view.
    let subjects = controlled_body_boxes(controlled_bodies.as_deref(), &world.0);

    for (view_entity, view_state) in &views {
        let focus_bevy =
            ae::config::world_to_bevy(&world.0, view_state.target_world, 0.0).truncate();

        let mut placements: Vec<LabelPlacement> = Vec::new();
        for (label, text, font, layout, _, _, _, _, key) in &labels {
            if on_hand.drawn_for(key.copied()) != Some(view_entity) {
                continue;
            }
            let anchor = label.anchor.truncate();
            placements.push(LabelPlacement {
                owner_id: label.owner_id.clone(),
                family: label.family,
                distance_sq: anchor.distance_squared(focus_bevy),
                anchor,
                size: label_size(
                    layout.map(|layout| layout.size),
                    text.as_str(),
                    label_font_px(font),
                    &settings,
                ),
                owner_opacity: label.owner_opacity,
                placed: Some(anchor),
                opacity: label.owner_opacity,
            });
        }
        if placements.is_empty() {
            continue;
        }

        resolve_label_layout(&mut placements, &subjects, &settings);

        // Owner ids are unique only within a view: the same sign in two views
        // is two entities with one id. So rebuild this map per view.
        let resolved: std::collections::HashMap<&str, &LabelPlacement> = placements
            .iter()
            .map(|placement| (placement.owner_id.as_str(), placement))
            .collect();

        apply_view_layout(
            view_entity,
            &on_hand,
            &resolved,
            ease,
            &mut labels,
            &mut outline_colors,
        );
    }
}

/// Write one view's resolved layout onto that view's labels.
/// Split out of [`layout_world_labels`] only for borrows: the label query
/// is borrowed mutably while a map from this view's placements is alive.
#[allow(clippy::type_complexity)]
fn apply_view_layout(
    view_entity: Entity,
    on_hand: &ambition_sim_view::ViewsOnHand,
    resolved: &std::collections::HashMap<&str, &LabelPlacement>,
    ease: f32,
    labels: &mut Query<(
        &mut WorldLabel,
        &Text2d,
        &TextFont,
        Option<&TextLayoutInfo>,
        &mut Transform,
        &mut Visibility,
        &mut TextColor,
        Option<&Children>,
        Option<&ambition_sim_view::PresentedForView>,
    )>,
    outline_colors: &mut Query<&mut TextColor, Without<WorldLabel>>,
) {
    for (mut label, _, _, _, mut transform, mut visibility, mut text_color, children, key) in
        labels.iter_mut()
    {
        if on_hand.drawn_for(key.copied()) != Some(view_entity) {
            continue;
        }
        let Some(placement) = resolved.get(label.owner_id.as_str()) else {
            continue;
        };
        let Some(placed) = placement.placed else {
            // Unplaceable. Hide it at once; the ease is only for the subject fade.
            label.rendered_opacity = 0.0;
            *visibility = Visibility::Hidden;
            *text_color = TextColor(with_opacity(label.text_color, 0.0));
            paint_outlines(outline_colors, children, &label, 0.0);
            continue;
        };
        transform.translation = placed.extend(label.anchor.z);
        // Position snaps, opacity eases. A displaced label must be in place this
        // frame (its anchor moves with its actor); only a fade would pop.
        let opacity = label.rendered_opacity + (placement.opacity - label.rendered_opacity) * ease;
        let opacity = if (opacity - placement.opacity).abs() < 1.0e-3 {
            placement.opacity
        } else {
            opacity
        };
        label.rendered_opacity = opacity;
        *visibility = visibility_for(opacity);
        *text_color = TextColor(with_opacity(label.text_color, opacity));
        paint_outlines(outline_colors, children, &label, opacity);
    }
}

/// Per-frame blend fraction for an exponential ease with time constant `tau`.
///
/// Frame-rate independent: a fixed per-frame step would fade twice as fast
/// at 120 Hz as at 60 Hz.
fn ease_fraction(tau_secs: f32, delta_secs: f32) -> f32 {
    if tau_secs <= 0.0 || delta_secs <= 0.0 {
        return 1.0;
    }
    1.0 - (-delta_secs / tau_secs).exp()
}

fn visibility_for(opacity: f32) -> Visibility {
    if opacity > 0.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn paint_outlines(
    outline_colors: &mut Query<&mut TextColor, Without<WorldLabel>>,
    children: Option<&Children>,
    label: &WorldLabel,
    opacity: f32,
) {
    let (Some(children), Some(outline)) = (children, label.outline_color) else {
        return;
    };
    for child in children.iter() {
        if let Ok(mut color) = outline_colors.get_mut(child) {
            *color = TextColor(with_opacity(outline, opacity));
        }
    }
}

#[derive(Resource)]
struct WorldLabelLayoutInstalled;

/// The generic world-label capability: the settings, the placement pass, and
/// the typeface pass. Anything that spawns a [`WorldLabel`] needs this plugin,
/// and only this plugin.
///
/// ## Why it is not part of the nameplate plugin
///
/// `spawn_room_visuals`, in the generic
/// [`SessionRoomVisualsPlugin`](crate::platformer_presentation::SessionRoomVisualsPlugin),
/// spawns signage and fixture labels. If only
/// [`ActorNameplatePresentationPlugin`](super::nameplates::ActorNameplatePresentationPlugin)
/// installed this pass, demos and external consumers would not get the
/// AC12/AC20 policy.
///
/// Adding it twice is a no-op, like `AmbitionLoadPlugin`: a full app composes
/// room visuals and nameplates, and both need it. The guard is a marker
/// resource, because Bevy registers the plugin name before `build` runs.
pub struct WorldLabelLayoutPlugin;

impl Plugin for WorldLabelLayoutPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<WorldLabelLayoutInstalled>() {
            return;
        }
        app.insert_resource(WorldLabelLayoutInstalled);
        // `chain()` is required for its sync points. The mirror spawns copies and
        // re-keys roots through `Commands`; the placement pass then selects
        // labels by that key. Without the flush, copies place one frame late and a
        // re-keyed root spends a frame in a dead view.
        app.init_resource::<WorldLabelLayoutSettings>().add_systems(
            Update,
            (
                apply_world_label_fonts,
                mirror_static_world_labels_per_view,
                layout_world_labels,
            )
                .chain()
                .in_set(WorldLabelLayoutSet)
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Which camera may draw what the pass placed. Registered here because
        // every composition with per-view projections installs this plugin.
        //
        // Not gated on a session: an ended session still needs its retraction
        // pass. The systems do nothing when there is nothing to iterate.
        app.add_systems(
            PostUpdate,
            super::view_isolation::isolate_per_view_projections
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        );
    }
}

pub(crate) fn with_opacity(color: Color, opacity: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        srgba.red,
        srgba.green,
        srgba.blue,
        srgba.alpha * opacity.clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A label spawned before its font arrives still ends up on it.
    ///
    /// Room signage spawns with a size and no font, because a room load has no
    /// `UiFonts` and the resource appears only after the catalog resolves. So the
    /// label comes first and the font second. A label that misses its font shows
    /// missing glyphs.
    ///
    /// The first pass must leave it alone. An unresolvable `FontSource::Family`
    /// does not error, so naming it early claims a face that is not there.
    #[test]
    fn a_label_spawned_before_its_font_still_ends_up_on_it() {
        use crate::ui_fonts::{UiFonts, PRODUCT_FAMILY};
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_systems(Update, apply_world_label_fonts);

        // A plate, spawned while nothing has loaded.
        let plate = app
            .world_mut()
            .spawn((
                WorldLabel::new("sign", WorldLabelFamily::Fixture, Vec3::ZERO),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(plate).unwrap().font,
            FontSource::default(),
            "with no fonts loaded the pass must not name a family"
        );

        // The catalog resolves and the fonts arrive.
        app.world_mut().insert_resource(UiFonts {
            regular: Some(Handle::default()),
            semibold: Some(Handle::default()),
            mono: Some(Handle::default()),
        });
        app.update();

        let font = app.world().get::<TextFont>(plate).unwrap();
        assert_eq!(
            font.font,
            FontSource::Family(PRODUCT_FAMILY.into()),
            "a label that outlived the font load must end up on the product face"
        );
        assert_eq!(
            font.weight,
            FontWeight::SEMIBOLD,
            "a plate is a NAME read at a glance, so it takes the semibold weight"
        );
        assert_eq!(
            font.font_size,
            FontSize::Px(12.0),
            "the pass owns the FACE and must not touch the size the spawner chose"
        );
    }

    fn settings() -> WorldLabelLayoutSettings {
        WorldLabelLayoutSettings {
            padding_px: 0.0,
            max_displacement_px: 30.0,
            ..Default::default()
        }
    }

    fn label(id: &str, family: WorldLabelFamily, anchor: Vec2, size: Vec2) -> LabelPlacement {
        LabelPlacement {
            owner_id: id.to_string(),
            family,
            distance_sq: 0.0,
            anchor,
            size,
            owner_opacity: 1.0,
            placed: Some(anchor),
            opacity: 1.0,
        }
    }

    /// Two labels from different families that start on top of each other do
    /// not end on top of each other.
    #[test]
    fn a_nameplate_under_a_sign_is_moved_off_it() {
        let cfg = settings();
        let mut labels = vec![
            label(
                "plate",
                WorldLabelFamily::Actor,
                Vec2::new(0.0, 0.0),
                Vec2::new(60.0, 12.0),
            ),
            label(
                "sign",
                WorldLabelFamily::Signage,
                Vec2::new(0.0, 0.0),
                Vec2::new(300.0, 16.0),
            ),
        ];
        // Premise: they overlap where their owners want them.
        assert!(LabelBox {
            center: labels[0].anchor,
            half: labels[0].size * 0.5,
        }
        .overlaps(
            &LabelBox {
                center: labels[1].anchor,
                half: labels[1].size * 0.5,
            },
            0.0
        ));

        resolve_label_layout(&mut labels, &[], &cfg);

        let sign = labels.iter().find(|l| l.owner_id == "sign").unwrap();
        let plate = labels.iter().find(|l| l.owner_id == "plate").unwrap();
        // The static, authored label stays; the moving one yields.
        assert_eq!(sign.placed, Some(Vec2::new(0.0, 0.0)));
        assert_ne!(plate.placed, Some(plate.anchor));
        assert!(!LabelBox {
            center: sign.placed.unwrap(),
            half: sign.size * 0.5,
        }
        .overlaps(
            &LabelBox {
                center: plate.placed.unwrap(),
                half: plate.size * 0.5,
            },
            0.0
        ));
    }

    #[test]
    fn a_label_that_cannot_be_placed_is_hidden_rather_than_stacked() {
        let cfg = WorldLabelLayoutSettings {
            max_displacement_px: 0.0,
            ..settings()
        };
        let mut labels = vec![
            label(
                "sign",
                WorldLabelFamily::Signage,
                Vec2::ZERO,
                Vec2::new(100.0, 20.0),
            ),
            label(
                "plate",
                WorldLabelFamily::Actor,
                Vec2::ZERO,
                Vec2::new(100.0, 20.0),
            ),
        ];
        resolve_label_layout(&mut labels, &[], &cfg);
        let plate = labels.iter().find(|l| l.owner_id == "plate").unwrap();
        assert_eq!(plate.opacity, 0.0);
        // It reports no position. Given the anchor back, the apply phase would
        // snap into the collision and fade out there, visibly stacked.
        assert_eq!(plate.placed, None);
    }

    /// A driven body is never pushed aside and never covered: the label dims
    /// instead of moving, so a sign does not twitch when the subject walks
    /// under it.
    #[test]
    fn a_label_over_the_controlled_subject_fades_and_does_not_move() {
        let cfg = settings();
        let subject = LabelBox {
            center: Vec2::new(0.0, 0.0),
            half: Vec2::new(16.0, 24.0),
        };
        let mut labels = vec![label(
            "sign",
            WorldLabelFamily::Signage,
            Vec2::new(0.0, 4.0),
            Vec2::new(300.0, 16.0),
        )];
        resolve_label_layout(&mut labels, &[subject], &cfg);
        assert_eq!(labels[0].placed, Some(labels[0].anchor));
        assert!(labels[0].opacity > 0.0);
        assert!(labels[0].opacity < 1.0);
    }

    /// A label that barely clips the top of a body must not dim. Walking under
    /// a sign gives this contact often, so bare intersection would strobe.
    #[test]
    fn a_grazing_overlap_does_not_dim_the_label() {
        let cfg = settings();
        let subject = LabelBox {
            center: Vec2::new(0.0, 0.0),
            half: Vec2::new(16.0, 24.0),
        };
        // Sits one pixel into the top edge of a 32x48 body: 32*1/1536 ≈ 2%.
        let mut labels = vec![label(
            "sign",
            WorldLabelFamily::Signage,
            Vec2::new(0.0, 24.0 + 8.0 - 1.0),
            Vec2::new(300.0, 16.0),
        )];
        resolve_label_layout(&mut labels, &[subject], &cfg);
        assert_eq!(labels[0].opacity, 1.0);
    }

    #[test]
    fn a_label_clear_of_the_subject_keeps_full_opacity() {
        let cfg = settings();
        let subject = LabelBox {
            center: Vec2::new(0.0, 0.0),
            half: Vec2::new(16.0, 24.0),
        };
        let mut labels = vec![label(
            "sign",
            WorldLabelFamily::Signage,
            Vec2::new(0.0, 400.0),
            Vec2::new(300.0, 16.0),
        )];
        resolve_label_layout(&mut labels, &[subject], &cfg);
        assert_eq!(labels[0].opacity, 1.0);
    }

    /// Two labels identical in family and distance must not swap places from
    /// frame to frame. `NameplateIndex` iterates in hash order, so without the
    /// id tiebreak the resolved layout would depend on it.
    #[test]
    fn ties_break_on_id_so_the_layout_cannot_depend_on_hash_order() {
        let cfg = settings();
        let build = |order: [&str; 2]| {
            let mut labels: Vec<LabelPlacement> = order
                .iter()
                .map(|id| {
                    label(
                        id,
                        WorldLabelFamily::Actor,
                        Vec2::ZERO,
                        Vec2::new(40.0, 10.0),
                    )
                })
                .collect();
            resolve_label_layout(&mut labels, &[], &cfg);
            labels
                .iter()
                .map(|l| (l.owner_id.clone(), l.placed))
                .collect::<Vec<_>>()
        };
        assert_eq!(build(["alpha", "beta"]), build(["beta", "alpha"]));
    }

    #[test]
    fn an_invisible_label_reserves_no_space() {
        let cfg = settings();
        let mut hidden = label(
            "hidden",
            WorldLabelFamily::Signage,
            Vec2::ZERO,
            Vec2::new(300.0, 16.0),
        );
        hidden.owner_opacity = 0.0;
        let mut labels = vec![
            hidden,
            label(
                "plate",
                WorldLabelFamily::Actor,
                Vec2::ZERO,
                Vec2::new(60.0, 12.0),
            ),
        ];
        resolve_label_layout(&mut labels, &[], &cfg);
        let plate = labels.iter().find(|l| l.owner_id == "plate").unwrap();
        assert_eq!(plate.placed, Some(plate.anchor));
        assert_eq!(plate.opacity, 1.0);
    }

    /// The ease must depend on elapsed time, not frames, or the fade runs at
    /// double speed at 120 Hz.
    #[test]
    fn the_opacity_ease_is_frame_rate_independent() {
        let tau = 0.1;
        let one_step = ease_fraction(tau, 1.0 / 60.0);
        let two_half_steps = {
            let a = ease_fraction(tau, 1.0 / 120.0);
            // Two successive blends of `a` compose to 1 - (1-a)^2.
            1.0 - (1.0 - a) * (1.0 - a)
        };
        assert!((one_step - two_half_steps).abs() < 1.0e-6);
        // A zero time constant is a hard cut, not a divide by zero.
        assert_eq!(ease_fraction(0.0, 1.0 / 60.0), 1.0);
    }

    /// Two views, one room, one simulation: two layouts.
    ///
    /// One world holds one pair of overlapping labels per view; only each view's
    /// focus differs. These tests check that N views give N correct projections.
    mod two_views_one_room_tests {
        use super::*;
        use ambition_sim_view::{CameraViewState, LocalView, LocalViewId, PresentedForView};
        use bevy::ecs::system::RunSystemOnce as _;

        /// 800x600, so the world-to-Bevy flip (`size.y * 0.5 - p.y`) is easy to
        /// check by hand.
        fn room() -> ae::RoomGeometry {
            ae::RoomGeometry(ae::World::new(
                "two views",
                ae::Vec2::new(800.0, 600.0),
                ae::Vec2::new(50.0, 50.0),
                Vec::new(),
            ))
        }

        /// Bevy-space (0, 0, 40) and (0, 5, 40): the boxes overlap, so one label
        /// must yield, decided only by distance to the view's focus.
        const ANCHOR_A: Vec3 = Vec3::new(0.0, 0.0, 40.0);
        const ANCHOR_B: Vec3 = Vec3::new(0.0, 5.0, 40.0);

        /// World-space camera targets far below and far above the anchors after
        /// the flip: `300.0 - 1300.0 = -1000` and `300.0 - (-700.0) = +1000`.
        const TARGET_BELOW: ae::Vec2 = ae::Vec2::new(400.0, 1300.0);
        const TARGET_ABOVE: ae::Vec2 = ae::Vec2::new(400.0, -700.0);

        fn settings() -> WorldLabelLayoutSettings {
            WorldLabelLayoutSettings {
                padding_px: 0.0,
                // Room for several lifts of a 15px label, so no test depends on the
                // budget running out.
                max_displacement_px: 30.0,
                ..Default::default()
            }
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

        /// Ten characters at font size 12 measure 60x15 by the pre-layout
        /// estimate (`10 * 12 * 0.5`, `1 * 12 * 1.25`), so each box has half
        /// size 30 x 7.5. The expectations below come from these numbers.
        fn spawn_label(world: &mut World, id: &str, anchor: Vec3, view: Entity) -> Entity {
            world
                .spawn((
                    Text2d::new("abcdefghij"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_translation(anchor),
                    Visibility::Visible,
                    WorldLabel::new(id, WorldLabelFamily::Actor, anchor),
                    PresentedForView(view),
                ))
                .id()
        }

        /// Run the pass over two views and return each view's `[a_y, b_y]`.
        fn place(first_target: ae::Vec2, second_target: ae::Vec2) -> [[f32; 2]; 2] {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );
            world.insert_resource(settings());
            world.init_resource::<Time>();

            let first = spawn_view(&mut world, 0, first_target);
            let second = spawn_view(&mut world, 1, second_target);
            let entities = [first, second].map(|view| {
                [
                    spawn_label(&mut world, "a", ANCHOR_A, view),
                    spawn_label(&mut world, "b", ANCHOR_B, view),
                ]
            });

            world.run_system_once(layout_world_labels).expect(
                "layout_world_labels should run: the fixture provides the session world, \
                 the settings and the clock it reads",
            );

            entities.map(|per_view| {
                per_view.map(|entity| {
                    world
                        .entity(entity)
                        .get::<Transform>()
                        .expect("a label keeps its transform")
                        .translation
                        .y
                })
            })
        }

        /// Each view's labels are placed by its own framing.
        ///
        /// The test checks values, not only that the views differ. With 60x15
        /// labels at y=0 and y=5, the nearer label keeps its anchor and the other
        /// lifts to clear it: 15px above the nearer label's centre.
        ///
        /// The second run swaps only the camera targets, and the layouts must
        /// swap too. A pass keyed on iteration order fails that run.
        #[test]
        fn each_view_lays_out_its_own_labels_against_its_own_focus() {
            // From below, "a" (y=0) is nearer, so it stays and "b" lifts from 5 to
            // 15.
            let looking_from_below = [0.0, 15.0];
            // From above, "b" (y=5) is nearer, so it stays and "a" lifts from 0 to
            // 20.
            let looking_from_above = [20.0, 5.0];
            assert_ne!(
                looking_from_below, looking_from_above,
                "the fixture must give the two views genuinely different layouts, \
                 or nothing below can tell a per-view pass from a shared one"
            );

            assert_eq!(
                place(TARGET_BELOW, TARGET_ABOVE),
                [looking_from_below, looking_from_above],
                "each view must place ITS OWN labels against ITS OWN focus; one \
                 layout applied to both views' entities is the process-global this \
                 milestone deleted, restored as a loop invariant"
            );

            assert_eq!(
                place(TARGET_ABOVE, TARGET_BELOW),
                [looking_from_above, looking_from_below],
                "swapping only the two camera targets must swap the two layouts. It \
                 did not, so placement is following iteration order and the \
                 assertion above was passing for the wrong reason"
            );
        }

        /// The one-view game is unchanged, also for labels that name no view.
        ///
        /// Authored signage spawns at room load without a view, and two demo tests
        /// spawn bare `WorldLabel` probes. All arrive unkeyed. `ViewsOnHand` assigns
        /// them to the only view, so they stay drawn.
        #[test]
        fn an_unkeyed_label_is_laid_out_by_the_only_view() {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );
            world.insert_resource(settings());
            world.init_resource::<Time>();

            let view = spawn_view(&mut world, 0, TARGET_BELOW);
            let keyed = spawn_label(&mut world, "a", ANCHOR_A, view);
            // Identical except that it names no view.
            let unkeyed = spawn_label(&mut world, "b", ANCHOR_B, view);
            world.entity_mut(unkeyed).remove::<PresentedForView>();

            world
                .run_system_once(layout_world_labels)
                .expect("layout_world_labels should run");

            let at = |world: &World, entity: Entity| {
                world
                    .entity(entity)
                    .get::<Transform>()
                    .expect("a label keeps its transform")
                    .translation
                    .y
            };
            assert_eq!(
                [at(&world, keyed), at(&world, unkeyed)],
                [0.0, 15.0],
                "an unkeyed label must be laid out by the only view, exactly as a \
                 keyed one is. Skipping it would leave every authored sign in the \
                 game frozen at its spawn transform, drawn but never placed"
            );
        }

        /// A retired view takes its projections with it, despawned as a set.
        ///
        /// An authored sign is one thing; only its per-view projection is
        /// duplicated. A projection whose view is gone is despawned. Only the root is
        /// re-keyed. Creating and retiring projections must both be ordinary,
        /// because an adaptive layout does both while the room stays loaded.
        #[test]
        fn a_retired_view_takes_its_label_projections_with_it() {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );

            let first = spawn_view(&mut world, 0, TARGET_BELOW);
            let second = spawn_view(&mut world, 1, TARGET_ABOVE);
            // One authored sign, spawned like room load: no view, so it arrives
            // unkeyed and the mirror decides.
            let root = world
                .spawn((
                    Text2d::new("abcdefghij"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_translation(ANCHOR_A),
                    Visibility::Visible,
                    WorldLabel::new("signage:0", WorldLabelFamily::Signage, ANCHOR_A),
                    StaticWorldLabel,
                ))
                .id();

            fn projections(world: &mut World) -> Vec<Entity> {
                let mut query = world.query_filtered::<Entity, With<StaticWorldLabel>>();
                let mut found: Vec<Entity> = query.iter(world).collect();
                found.sort();
                found
            }

            world
                .run_system_once(mirror_static_world_labels_per_view)
                .expect("the mirror runs");
            assert_eq!(
                projections(&mut world).len(),
                2,
                "two views owe two projections of one authored sign — one entity \
                 cannot hold two views' transforms, which is the whole reason this \
                 mirror exists"
            );

            world.despawn(second);
            world
                .run_system_once(mirror_static_world_labels_per_view)
                .expect("the mirror runs");
            assert_eq!(
                projections(&mut world),
                vec![root],
                "retiring a view must despawn its projections as a SET, leaving \
                 exactly the root. A surviving copy is an entity the renderer still \
                 draws and no per-view query can ever reach again"
            );
            assert_eq!(
                world.entity(root).get::<PresentedForView>().copied(),
                Some(PresentedForView(first)),
                "the root is RE-KEYED, never un-keyed: stripping its key would drop \
                 the last projection of an authored sign out of the placement pass \
                 entirely, and the sign would freeze on screen"
            );
        }
    }

    #[test]
    fn a_measured_layout_beats_the_character_estimate() {
        let cfg = settings();
        let measured = Vec2::new(123.0, 17.0);
        assert_eq!(label_size(Some(measured), "whatever", 14.0, &cfg), measured);
        // Before the text pipeline has run, the estimate is used.
        let estimated = label_size(None, "abcd", 10.0, &cfg);
        assert_eq!(estimated.x, 4.0 * 10.0 * cfg.fallback_advance_ratio);
        // A zero measurement is the pipeline saying "not yet", not "empty".
        assert_eq!(label_size(Some(Vec2::ZERO), "abcd", 10.0, &cfg), estimated);
    }
}
