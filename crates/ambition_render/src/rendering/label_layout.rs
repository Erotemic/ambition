//! ONE ranked placement pass over every world-space text label.
//!
//! ## Why this exists (queue row AC12)
//!
//! Spacing within a family cannot stop a cross-family overlap. So placement moves here, and
//! every label — whoever spawns it — participates by carrying a [`WorldLabel`].
//!
//! ## The two mechanisms, and why they are different
//!
//! Label vs label → DISPLACEMENT, in rank order. The ranking is [`WorldLabelFamily`]'s
//! declaration order, and it is chosen so that the family which yields is the one that can yield
//! *without anything visibly jumping*: an actor plate already moves every frame with its actor, so
//! nudging it costs nothing.
//!
//! Label vs the CONTROLLED SUBJECT → FADE, never displacement. A body you are driving walks
//! under a static sign constantly; nudging the sign out of its way would make the sign twitch
//! across the screen every time. Dimming it keeps the sign legible, keeps the body visible, and
//! is stable.
//!
//! Note the subject is *whoever is driving*, read from
//! [`ControlledBodiesView`] — not "the player". A possessed enemy and both
//! fighters in a couch match get the same protection: a rule that singles out
//! "the player" stops being a rule about bodies.

use ambition_platformer2d_core as ae;
use ambition_sim_view::ControlledBodiesView;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;

use crate::ui_fonts::{UiFontWeight, UiFonts};

/// Which family a world-space label belongs to. Declaration order IS the
/// ranking: an earlier variant is placed first and never yields to a later
/// one.
///
/// The order is not a value judgement about which text matters more. It is
/// ordered by how expensive yielding is: the families that cannot move without
/// visibly twitching are placed first, and the one that is already in motion
/// absorbs the displacement.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldLabelFamily {
    /// Authored room signage (an LDtk `DebugLabel`, a chest's name). Static,
    /// hand-placed by a designer, and therefore never displaced by this pass
    /// unless another sign is already sitting where it wants to be.
    Signage,
    /// A plate naming a static world fixture — a door, a non-door loading
    /// zone. Static, so it yields only to signage.
    Fixture,
    /// A plate naming an actor. It tracks a moving body, so it yields to
    /// everything static: a nudge here is indistinguishable from the motion the
    /// label already has.
    Actor,
    /// A line somebody just said — a speech bubble.
    ///
    /// LAST, so it yields to the actor plate rather than the other way
    /// round, on this module's own test: *which family can move without
    /// anything visibly jumping?* A plate is permanent furniture attached to a
    /// body; displacing it means it hops up and back down once per taunt, on an
    /// element the eye is using to keep track of who is who. A bubble is born
    /// in motion — it rises through `SPEECH_BUBBLE_BASE_RISE` for its whole
    /// ~2.2s life and fades while it does — and it is gone before the frame
    /// stops being interesting. There is no reading of "displacement is
    /// invisible here" under which the plate is the better candidate.
    Speech,
}

/// Marks a `Text2d` entity as a world-space label and carries everything the
/// placement pass needs.
///
/// This pass is the single writer of the label's `Transform`, `Visibility`
/// and `TextColor`. The owning system writes only into this component — the
/// anchor it wants and the opacity it wants — and never touches the transform
/// itself. Two writers sharing one placement is how a label ends up drifting:
/// a pass that reads back the transform it moved last frame accumulates its own
/// correction.
#[derive(Component, Clone, Debug)]
pub struct WorldLabel {
    /// Stable view identity of the labeled thing. Used only as the final
    /// deterministic tiebreak — the actor plates' source index iterates in
    /// hash order, and a placement that depends on that order is a placement
    /// that flickers.
    pub owner_id: String,
    pub family: WorldLabelFamily,
    /// Where the owner wants the label, in Bevy world space, including Z. The
    /// pass always places FROM here, never from the current transform.
    pub anchor: Vec3,
    /// The opacity the owner asked for (a nameplate's rank fade; 1.0 for
    /// static signage). The pass may only reduce it.
    pub owner_opacity: f32,
    /// The label's colour at full opacity.
    pub text_color: Color,
    /// The colour for outline/shadow children, at full opacity. `None` for
    /// labels drawn without an outline pass.
    pub outline_color: Option<Color>,
    /// The opacity actually drawn last frame, eased toward the resolved target.
    ///
    /// Presentation state owned by the pass, never by an owner. Without it the
    /// subject fade is a hard cut: walk under a 400px sign and the whole thing
    /// snaps to a fifth of its alpha and back, once per step near the edge.
    /// Easing costs one float and removes the pop.
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
            // Starts dark so a label eases IN rather than appearing at full
            // strength on the frame its room loads.
            rendered_opacity: 0.0,
        }
    }

    pub fn with_colors(mut self, text_color: Color, outline_color: Option<Color>) -> Self {
        self.text_color = text_color;
        self.outline_color = outline_color;
        self
    }
}

/// Marks a world label the ROOM spawned once, whose per-view copies the mirror
/// below owns.
///
/// it exists to keep the mirror off the nameplates. Actor and door plates
/// carry [`WorldLabel`] too, and `sync_actor_nameplates` already builds one per
/// view itself — including an outline-child subtree the mirror has no business
/// cloning. Static signage and fixture plates are spawned once at room load by
/// code that has no view in scope, so they are the family that needs mirroring,
/// and this says which ones those are.
#[derive(Component, Clone, Copy, Debug)]
pub struct StaticWorldLabel;

/// A mirrored copy of a static world label, naming the label it was copied from.
///
/// The link is what makes the copy's life derivative: when the room despawns the
/// root, the copy goes with it rather than lingering as a label naming nothing.
#[derive(Component, Clone, Copy, Debug)]
pub struct MirroredWorldLabel {
    pub root: Entity,
}

/// ONE DRAWN COPY OF EVERY STATIC WORLD LABEL PER LIVE VIEW.
///
/// the reason is that one entity cannot hold two views' transforms. A sign
/// is ranked against its view's focus, displaced by whatever else that view is
/// drawing, and dimmed when that view's controlled body walks under it. Two views
/// legitimately want the same sign at two positions and two opacities, so naming
/// which view a single shared entity serves could not have made it correct —
/// there is no value it could hold that is right for both.
///
/// a second view is a COUNT, not a special case, and the single-view case stays exactly one
/// entity. The label the room spawned is CLAIMED by the lowest-id view rather than being
/// demoted to an un-drawn template; a template would have made the one-view game allocate two
/// entities per sign to draw one. Views past the first get copies.
///
/// the claim is keyed on `LocalViewId`, not on query order. Which entity is
/// "the root's view" has to be the same answer on every frame and every run;
/// archetype iteration is neither.
///
/// The root is the one exception, and it is RE-KEYED onto the surviving lowest view: a reset, not a
/// removal.
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
        // No observation seam in this composition, so nothing presents and there
        // is nothing to mirror. `ambition_sim_view::ViewsOnHand` calls the
        // no-views case quiet for exactly this reason.
        return;
    };

    // Retract BEFORE spawning, so a view that went away takes its whole set with
    // it rather than being counted as still-mirrored below.
    let live: std::collections::HashSet<Entity> = ordered.iter().map(|(_, view)| *view).collect();
    let mut mirrored: std::collections::HashSet<(Entity, Entity)> =
        std::collections::HashSet::new();
    for (entity, copy, key) in &copies {
        let root_is_gone = roots.get(copy.root).is_err();
        // `key.0 == root_view` is the re-key case: the view this copy served has
        // become the root's own view, so the root already draws it and the copy
        // is a duplicate.
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
            // Its own ease state. A copy that appears mid-session fades in like
            // any other new label instead of inheriting whatever opacity the
            // root happened to be drawn at for a different view.
            copied_label.rendered_opacity = 0.0;
            commands.spawn_session_scoped(
                session_scope,
                (
                    text.clone(),
                    font.clone(),
                    TextColor(color.0),
                    // From the ANCHOR, never from the root's transform: the
                    // root's has already been displaced by ITS view's placement.
                    Transform::from_translation(label.anchor),
                    copied_label,
                    StaticWorldLabel,
                    MirroredWorldLabel { root },
                    ambition_sim_view::PresentedForView(*view),
                    super::primitives::RoomVisual,
                    // no `Name`. `entity.name` is registered for rollback
                    // and the coverage contract sweeps any entity carrying a type
                    // the rollback knows about, so labelling these would enlist a
                    // whole view's presentation set in the sim sweep.
                ),
            );
        }
    }
}

/// Tunables for the placement pass.
#[derive(Resource, Clone, Debug)]
pub struct WorldLabelLayoutSettings {
    /// Off-switch. Disabled, every label draws at its owner's anchor and
    /// opacity — the pre-AC12 behaviour, so a game can decline the policy.
    pub enabled: bool,
    /// Empty space required between two labels' boxes, world px.
    pub padding_px: f32,
    /// How far a label may travel from its anchor before it is hidden instead,
    /// world px, upward (+Y in Bevy space). A label that has walked far from
    /// the thing it names has stopped naming it.
    ///
    /// A label now lifts to exactly clear whatever is in its way, and this is the one number that
    /// says how far is too far. Sized for four lines of world text in one cluster — a four-fighter
    /// free-for-all's taunts, the widest supported match.
    pub max_displacement_px: f32,
    /// Opacity multiplier for a label overlapping a driven body. Low enough
    /// that the body reads through it, high enough that the label is still
    /// legible on a dark background — it yields, it does not vanish.
    pub occluded_opacity: f32,
    /// Seconds for an opacity change to reach ~63% of its new target. Zero
    /// makes every change a hard cut.
    pub opacity_ease_secs: f32,
    /// How much of a driven body a label must cover before it yields, as a
    /// fraction of that body's box area. Guards against a one-pixel graze
    /// dimming a whole sign.
    pub min_body_coverage: f32,
    /// Fallback advance width per character, as a fraction of font size, used
    /// only until Bevy's text pipeline has measured the label for real.
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
    /// The subject fade keys off this rather than off bare intersection,
    /// because a label whose bottom edge grazes a head by a pixel should not
    /// dim a 400px sign — and while walking along under one, a graze is what
    /// you get repeatedly, so bare intersection would flicker.
    ///
    /// the intersection is clamped per axis, not taken as `sumHalf - |delta|`.
    /// That penetration-depth form is the one every AABB routine reaches for and
    /// it is WRONG here: a 300px-wide label fully containing a 32px-wide body
    /// reports 316px of x-overlap instead of 32, which read as total coverage
    /// for exactly the case this guard exists to judge.
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
    /// labels are placed first, matching the nameplate ranking that already
    /// exists.
    pub distance_sq: f32,
    pub anchor: Vec2,
    /// Full drawn size of the text.
    pub size: Vec2,
    pub owner_opacity: f32,
    /// Resolved position — `Some(anchor)` unless the label had to yield, and
    /// `None` when there was nowhere to stand at all.
    ///
    /// an `Option`, not a position plus a "did it fit" flag, and that is the
    /// whole point: a flag sitting next to a stale coordinate is a value the
    /// apply phase can read without noticing it must not. It did — an
    /// unplaceable label had `placed == anchor`, so the transform snapped back
    /// INTO the collision it had just lost and eased its opacity to zero from
    /// there, stacking visibly for the length of the fade. `None` makes that
    /// unwriteable.
    pub placed: Option<Vec2>,
    /// Resolved opacity. Zero means "could not be placed" or "the owner asked
    /// for nothing".
    pub opacity: f32,
}

/// The whole policy, as a pure function over boxes.
///
/// Kept pure because the premise of this pass is geometric and a premise
/// asserted in a comment is a premise nobody checks: the tests below construct
/// overlapping boxes and assert the resolved ones do not overlap, rather than
/// stating in prose that they do not.
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
            // An already-invisible label reserves no space. Letting it push a
            // visible neighbour aside would be a hidden label with visible
            // consequences.
            continue;
        }

        let half = label.size * 0.5;
        let mut candidate = LabelBox {
            center: label.anchor,
            half,
        };
        let mut resolved = None;
        // Lift to just clear whatever is actually in the way. Each pass
        // rises above the HIGHEST box it currently overlaps, which strictly
        // raises the highest blocker it can still meet — so this settles in at
        // most one pass per already-placed label, and the bound says so rather
        // than trusting the float arithmetic to.
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
                // Nowhere to stand. Hiding beats stacking: the wall of garbled
                // text this pass exists to prevent is exactly what "place it
                // anyway" produces — and `None` is what makes that true of the
                // TRANSITION as well, not only of the steady state.
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
/// The fallback measurement below works in logical pixels and this pass has no
/// viewport, so a viewport-relative or rem-relative `FontSize` cannot be
/// resolved here. Nothing writes one today — every Ambition label size is
/// authored in pixels — and this is the seam that would have to learn the
/// viewport if one ever did, rather than silently measuring zero.
pub(crate) fn label_font_px(font: &TextFont) -> f32 {
    match font.font_size {
        FontSize::Px(px) => px,
        other => other.eval(Vec2::ZERO, bevy::text::RemSize::default().0),
    }
}

/// Measure a label. Prefers what Bevy's text pipeline actually laid out; falls
/// back to a per-character estimate on the first frame of a label's life, when
/// no layout has run yet.
///
/// The fallback deliberately errs wide (`ceil` on the character count is not
/// enough — proportional fonts vary), because a measurement that is too small
/// produces the exact defect this module exists to remove.
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
/// Read from the sim's [`ControlledBodiesView`] rather than from render
/// entities — render reads the view, never the sim components directly, so a
/// presentation pass cannot start depending on simulation layout.
///
/// This deliberately does NOT use `NameplateIndex`'s `controlled` flag, which
/// is the obvious-looking source and is wrong: that index only carries rows
/// keyed by `FeatureId`, and the home avatar has none — so the flag is true
/// only while possessing a feature actor. Built on it, this rule would have
/// protected every body except the one you normally play, and it would have
/// looked like it worked.
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
/// The rule, stated (queue row AC20): all world text is ONE family, and the
/// weight carries the role. Signage is prose a designer wrote — a sentence —
/// so it is Regular. A plate is a NAME, read at a glance against busy art, so
/// it is Semibold.
///
/// Two typefaces on one screen, and the accident happened to look deliberate — mono reads as
/// machine annotation next to strings like `MAP_OFFICIAL:`. It is not chosen mono either way:
/// the project's only monospace asset is the *debug* HUD font, which is the wrong signal for
/// shipped world signage.
fn font_weight_for(family: WorldLabelFamily) -> UiFontWeight {
    match family {
        // A spoken line is prose — a sentence — so it takes the same weight the
        // rule gives a designer's authored sentence, not a plate's.
        WorldLabelFamily::Signage | WorldLabelFamily::Speech => UiFontWeight::Regular,
        WorldLabelFamily::Fixture | WorldLabelFamily::Actor => UiFontWeight::Semibold,
    }
}

/// Keep every world label on its family's typeface.
///
/// A separate system from placement on purpose — and it runs every frame
/// rather than once at spawn, which fixes a race the spawn-time resolution
/// could not: fonts load asynchronously, so a label spawned before its font
/// arrived kept Bevy's fallback forever. Assignment is guarded on inequality
/// because writing `TextFont` re-runs text layout.
pub fn apply_world_label_fonts(
    ui_fonts: Option<Res<UiFonts>>,
    mut labels: Query<(&WorldLabel, &mut TextFont)>,
) {
    let Some(fonts) = ui_fonts.as_deref() else {
        return;
    };
    for (label, mut font) in &mut labels {
        let wanted = fonts
            .text_font(font.font_size, font_weight_for(label.family))
            .font;
        if font.font != wanted {
            font.font = wanted;
        }
    }
}

/// The pass. Places every [`WorldLabel`] and writes the result — once per
/// view, over that view's own labels.
///
/// Silent-wrong, in a seam whose every other refusal is loud, and it produced a plausible-looking
/// layout that was ordered by nothing.
///
/// iterating VIEWS deletes that fallback rather than repairing it. Each
/// iteration holds a real [`CameraViewState`](ambition_sim_view::CameraViewState)
/// — the view's own — so there is no branch left in which a focus has to be
/// invented. A view with no camera draws for nobody and costs a pass; two
/// cameras on one view share one set, which is correct.
#[allow(clippy::type_complexity)]
pub fn layout_world_labels(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    settings: Res<WorldLabelLayoutSettings>,
    time: Res<Time>,
    // This was `PresentedViewState`, which resolves the ONE view a single main camera presents
    // and refuses when there are several — the right answer for a diagnostic that must name one
    // view, and the wrong shape for a draw system, which owes every view a picture.
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
        // The policy is DECLINED, not per-view: every label draws at its owner's
        // anchor and opacity, which is view-independent by construction.
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

    // Producing no projection is the honest answer. It cannot happen in a composed host — the view
    // is spawned at plugin BUILD time — so reaching this means the composition has no observation
    // seam and there is genuinely nobody to lay text out for.
    if views.is_empty() {
        return;
    }

    // Which view a given label belongs to is the SAME question as which view a
    // camera presents, so it is the same answer: `ambition_sim_view::ViewsOnHand`
    // states it once. A label that names its view is that view's; an unkeyed one
    // in a single-view composition is the only view's — which is what keeps
    // hand-spawned probes and any label the mirror has not reached yet drawing
    // exactly as they did; and an unkeyed one with several views is refused.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, _)| view));

    // View-independent: whoever is driving is driving in every view.
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

        // Placements are keyed by owner id, which is unique per label family
        // member WITHIN A VIEW — the same sign in two views is two entities
        // carrying the same id, which is why this map is rebuilt per view rather
        // than once for the whole world.
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
///
/// Split out of [`layout_world_labels`] only because the borrow it needs — the
/// label query, mutably, while a map borrowed from this view's placements is
/// alive — does not survive being written inline inside the loop that also
/// iterates the query immutably.
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
            // Unplaceable. The ease exists to stop the subject fade popping; it has no business
            // smoothing a disappearance whose whole purpose is that the text is not on screen.
            label.rendered_opacity = 0.0;
            *visibility = Visibility::Hidden;
            *text_color = TextColor(with_opacity(label.text_color, 0.0));
            paint_outlines(outline_colors, children, &label, 0.0);
            continue;
        };
        transform.translation = placed.extend(label.anchor.z);
        // Position snaps, opacity eases. A displaced label has to be where it
        // belongs THIS frame (its anchor is already moving with its actor);
        // only the fade would read as a pop.
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
/// Frame-rate independent on purpose: a fixed per-frame step would fade twice
/// as fast on a 120Hz display as on a 60Hz one, which is the classic way a
/// "subtle" transition becomes a flicker on somebody else's machine.
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
/// It was, for one commit, and that made the AC12/AC20 policy true of exactly one composition.
/// `spawn_room_visuals` — which lives in the GENERIC
/// [`SessionRoomVisualsPlugin`](crate::platformer_presentation::SessionRoomVisualsPlugin), not
/// in Ambition — spawns signage and fixture labels; the systems that give those components
/// meaning were installed only by
/// [`ActorNameplatePresentationPlugin`](super::nameplates::ActorNameplatePresentationPlugin),
/// which the demos and the external consumer do not add.
///
/// Adding it twice is a no-op rather than a crash, for the same reason
/// `AmbitionLoadPlugin` is: a full app composes room visuals AND nameplates, and
/// both legitimately need it. The guard is a marker resource, not
/// `is_plugin_added::<Self>()`, because Bevy has already registered the name by
/// the time `build` runs.
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
        // `chain()` is load-bearing here, for its SYNC POINTS. The mirror
        // spawns a view's copies and re-keys the roots through `Commands`; the
        // placement pass immediately after selects labels BY that key. Ordered
        // without the flush between them, every copy would be placed one frame
        // after it appeared, and a re-keyed root would spend that frame belonging
        // to a view that no longer exists.
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
        // Which camera may DRAW what the pass above placed. Registered here
        // because this plugin is what every composition that spawns a per-view
        // projection already installs — nameplates and room signage both — so the
        // isolation lands wherever the projections do, instead of being true of
        // the one composition that remembered it.
        //
        // and it is deliberately NOT gated on a session. A composition whose
        // session ended must still get its retraction pass; the systems above
        // decline by having nothing to iterate, and so does this one.
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

    /// The premise the whole module rests on, asserted rather than described: two labels FROM
    /// DIFFERENT FAMILIES that start on top of each other do not end on top of each other.
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
        // The premise: they DO overlap where their owners want them.
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
        // The static, authored label held its ground; the moving one yielded.
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
        // And it reports NO position, which is the half that matters to the
        // apply phase: given the anchor back it would snap the transform into
        // the collision the label just lost and fade out from there — stacking
        // for the length of the ease, which is what "hiding beats stacking"
        // exists to forbid.
        assert_eq!(plate.placed, None);
    }

    /// A body somebody is driving is never shoved aside and never covered: the
    /// label dims instead of moving, so a sign does not twitch every time the
    /// subject walks under it.
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

    /// A label that barely clips the top of a body must NOT dim it. Walking
    /// under a sign produces exactly this contact repeatedly, so a bare
    /// intersection test would make the sign strobe.
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

    /// The ease must depend on elapsed TIME, not on frames, or the fade runs
    /// at double speed on a 120Hz display.
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

    /// TWO VIEWS, ONE ROOM, ONE SIMULATION — TWO LAYOUTS.
    ///
    /// Everything below runs against ONE world holding ONE pair of overlapping labels per view;
    /// the only thing that differs between the two views is where each is looking.
    ///
    /// What these tests pin is only that N views produce N correct projections for any N.
    mod two_views_one_room_tests {
        use super::*;
        use ambition_sim_view::{CameraViewState, LocalView, LocalViewId, PresentedForView};
        use bevy::ecs::system::RunSystemOnce as _;

        /// 800x600, so the world→Bevy flip (`size.y * 0.5 - p.y`) is arithmetic
        /// anyone can check by hand rather than a number copied from a run.
        fn room() -> ae::RoomGeometry {
            ae::RoomGeometry(ae::World::new(
                "two views",
                ae::Vec2::new(800.0, 600.0),
                ae::Vec2::new(50.0, 50.0),
                Vec::new(),
            ))
        }

        /// Bevy-space (0, 0, 40) and (0, 5, 40): close enough that the two
        /// labels' boxes overlap, so ONE of them must yield and which one is
        /// decided purely by distance to the view's focus.
        const ANCHOR_A: Vec3 = Vec3::new(0.0, 0.0, 40.0);
        const ANCHOR_B: Vec3 = Vec3::new(0.0, 5.0, 40.0);

        /// World-space camera targets that land far BELOW and far ABOVE the two
        /// anchors once flipped: `300.0 - 1300.0 = -1000` and
        /// `300.0 - (-700.0) = +1000`.
        const TARGET_BELOW: ae::Vec2 = ae::Vec2::new(400.0, 1300.0);
        const TARGET_ABOVE: ae::Vec2 = ae::Vec2::new(400.0, -700.0);

        fn settings() -> WorldLabelLayoutSettings {
            WorldLabelLayoutSettings {
                padding_px: 0.0,
                // Room for several lifts of a 15px-tall label, so no expectation
                // below is really an assertion about the budget running out.
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

        /// Ten characters at font size 12 measure 60x15 through the pre-layout
        /// estimate (`10 * 12 * 0.5`, `1 * 12 * 1.25`), so each box is 30 wide
        /// and 7.5 tall from centre — the numbers every expectation below is
        /// derived from.
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

        /// EACH VIEW'S LABELS ARE PLACED BY ITS OWN FRAMING.
        ///
        /// So a second view did not produce a second layout; it produced one layout, ordered by
        /// distance to the world origin, written over both views' entities.
        ///
        /// the assertion is on VALUES, not on inequality. "the two views
        /// differ" would pass for a pair that differ and are both wrong. Each
        /// number below is derived from the box arithmetic: with 60x15 labels at
        /// y=0 and y=5, the nearer label holds its anchor and the farther one
        /// lifts to exactly clear it — the 15px sum of half-heights above the
        /// nearer label's centre.
        ///
        /// and the falsifier is inside the test. The second run swaps only
        /// the two views' camera targets — same spawn order, same entities, same
        /// anchors — and the two layouts must swap with them. A pass that keys
        /// off label or view iteration order instead of the view's own focus
        /// passes the first run and fails this one.
        #[test]
        fn each_view_lays_out_its_own_labels_against_its_own_focus() {
            // Looking from below, "a" (y=0) is nearer, so it holds its anchor and
            // "b" lifts from 5 to 15 — 7.5 above a's top edge, its own half-height.
            let looking_from_below = [0.0, 15.0];
            // Looking from above, "b" (y=5) is nearer, so it holds ITS anchor and
            // "a" lifts from 0 to 20.
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

        /// THE ONE-VIEW GAME IS UNCHANGED, INCLUDING FOR LABELS THAT NAME NO
        /// VIEW.
        ///
        /// Authored signage is spawned at room load, which has no view in scope,
        /// and two demo tests spawn bare `WorldLabel` probes by hand. All of them
        /// arrive unkeyed. `ViewsOnHand`'s rule — the only view is the honest
        /// answer for anything that names none — is what keeps them drawn, and it
        /// is the whole of "a second view is a count, not a rewrite".
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
            // Identical in every respect except that it names no view.
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

        /// A RETIRED VIEW TAKES ITS PROJECTIONS WITH IT — DESPAWNED AS A
        /// SET.
        ///
        /// One authored sign is ONE authoritative thing; what is duplicated is
        /// its per-view PROJECTION, and a projection whose view is gone has
        /// nothing left to be a projection of.
        ///
        /// Retract by despawning; only the ROOT is re-keyed, which is a reset rather than a
        /// removal.
        ///
        /// this is the adaptive shape arriving early on purpose: creating the
        /// second projection and retiring it have to be equally ordinary, because
        /// a layout that adapts does both while the room stays loaded.
        #[test]
        fn a_retired_view_takes_its_label_projections_with_it() {
            let mut world = World::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
                &mut world,
                room(),
            );

            let first = spawn_view(&mut world, 0, TARGET_BELOW);
            let second = spawn_view(&mut world, 1, TARGET_ABOVE);
            // ONE authored sign, spawned the way room load spawns it: no view in
            // scope, so it arrives unkeyed and the mirror decides.
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
