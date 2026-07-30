//! ONE ranked placement pass over every world-space text label.
//!
//! ## Why this exists (queue row AC12)
//!
//! A screen carries at least three families of world text — authored signage
//! from the room file, actor nameplates, and door/fixture plates — and each
//! family used to place itself. `sync_actor_nameplates` ranked and faded
//! nameplates *against other nameplates*; the LDtk authoring tool
//! (`edit.space_debug_labels`) spaced DebugLabels *against other DebugLabels*.
//! Neither could see the other, so a signage label and an actor plate could be
//! drawn through each other and both passes would correctly report "no
//! overlaps found".
//!
//! That is not a positioning bug in any one family; it is the absence of a
//! placement MODEL. Spacing within a family cannot stop a cross-family overlap.
//! So placement moves here, and every label — whoever spawns it — participates
//! by carrying a [`WorldLabel`].
//!
//! ## The two mechanisms, and why they are different
//!
//! **Label vs label → DISPLACEMENT, in rank order.** The ranking is
//! [`WorldLabelFamily`]'s declaration order, and it is chosen so that the
//! family which yields is the one that can yield *without anything visibly
//! jumping*: an actor plate already moves every frame with its actor, so
//! nudging it costs nothing. A designer's authored sign is fixed in the world
//! and outranks everything — it never moves, so it never jitters.
//!
//! **Label vs the CONTROLLED SUBJECT → FADE, never displacement.** A body you
//! are driving walks under a static sign constantly; nudging the sign out of
//! its way would make the sign twitch across the screen every time. Dimming it
//! keeps the sign legible, keeps the body visible, and is stable. This is the
//! half that fixes "a signage label drawn across the player on the game's first
//! screen" — filed twice as a content bug, once per room, when it was one
//! placement problem with two symptoms.
//!
//! Note the subject is *whoever is driving*, read from
//! [`ControlledBodiesView`] — not "the player". A possessed enemy and both
//! fighters in a couch match get the same protection
//! ([[feedback-relativity-principle]]).

use ambition_engine_core as ae;
use ambition_sim_view::ControlledBodiesView;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;

use crate::ui_fonts::{UiFontWeight, UiFonts};

/// Which family a world-space label belongs to. **Declaration order IS the
/// ranking**: an earlier variant is placed first and never yields to a later
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
    /// A plate naming an actor. It tracks a moving body, so it is the family
    /// that yields: a nudge here is indistinguishable from the motion the
    /// label already has.
    Actor,
}

/// Marks a `Text2d` entity as a world-space label and carries everything the
/// placement pass needs.
///
/// **This pass is the single writer of the label's `Transform`, `Visibility`
/// and `TextColor`.** The owning system writes only into this component — the
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

/// Tunables for the placement pass.
#[derive(Resource, Clone, Debug)]
pub struct WorldLabelLayoutSettings {
    /// Off-switch. Disabled, every label draws at its owner's anchor and
    /// opacity — the pre-AC12 behaviour, so a game can decline the policy.
    pub enabled: bool,
    /// Empty space required between two labels' boxes, world px.
    pub padding_px: f32,
    /// One displacement step, world px. Applied upward (+Y in Bevy space).
    pub step_px: f32,
    /// How many steps a label may take before it is hidden instead. A label
    /// that has walked far from the thing it names has stopped naming it.
    pub max_steps: u32,
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
            step_px: 11.0,
            max_steps: 6,
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
    /// ⚠ the intersection is clamped per axis, not taken as `sumHalf - |delta|`.
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
    /// **`None` when there was nowhere to stand at all**.
    ///
    /// ⚠ an `Option`, not a position plus a "did it fit" flag, and that is the
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
        let mut resolved = None;
        for step in 0..=settings.max_steps {
            let candidate = LabelBox {
                center: label.anchor + Vec2::new(0.0, step as f32 * settings.step_px),
                half,
            };
            if !occupied
                .iter()
                .any(|placed| candidate.overlaps(placed, settings.padding_px))
            {
                resolved = Some(candidate);
                break;
            }
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
/// entities ([[feedback-render-reads-simview-not-sim-components]]).
///
/// ⚠ This deliberately does NOT use `NameplateIndex`'s `controlled` flag, which
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
/// **The rule, stated (queue row AC20): all world text is ONE family, and the
/// weight carries the role.** Signage is prose a designer wrote — a sentence —
/// so it is Regular. A plate is a NAME, read at a glance against busy art, so
/// it is Semibold.
///
/// What this replaces was not a different rule, it was no rule:
/// `spawn_world_label` built `TextFont { font_size, ..default() }` because it
/// had no `UiFonts` in scope, so signage rendered in Bevy's built-in fallback
/// (a mono face) while plates used the project's font. Two typefaces on one
/// screen, and the accident happened to look deliberate — mono reads as
/// machine annotation next to strings like `MAP_OFFICIAL:`. It is not chosen
/// mono either way: the project's only monospace asset is the *debug* HUD font,
/// which is the wrong signal for shipped world signage.
fn font_weight_for(family: WorldLabelFamily) -> UiFontWeight {
    match family {
        WorldLabelFamily::Signage => UiFontWeight::Regular,
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

/// The pass. Places every [`WorldLabel`] and writes the result.
#[allow(clippy::type_complexity)]
pub fn layout_world_labels(
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    settings: Res<WorldLabelLayoutSettings>,
    time: Res<Time>,
    camera: Option<Res<super::camera::CameraViewState>>,
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
    )>,
    mut outline_colors: Query<&mut TextColor, Without<WorldLabel>>,
) {
    let ease = ease_fraction(settings.opacity_ease_secs, time.delta_secs());
    if !settings.enabled {
        for (mut label, _, _, _, mut transform, mut visibility, mut text_color, children) in
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

    let focus_bevy = camera
        .as_deref()
        .map(|camera| ae::config::world_to_bevy(&world.0, camera.target_world, 0.0).truncate())
        .unwrap_or(Vec2::ZERO);

    let mut placements: Vec<LabelPlacement> = Vec::new();
    for (label, text, font, layout, _, _, _, _) in &labels {
        let anchor = label.anchor.truncate();
        placements.push(LabelPlacement {
            owner_id: label.owner_id.clone(),
            family: label.family,
            distance_sq: anchor.distance_squared(focus_bevy),
            anchor,
            size: label_size(
                layout.map(|layout| layout.size),
                text.as_str(),
                font.font_size,
                &settings,
            ),
            owner_opacity: label.owner_opacity,
            placed: Some(anchor),
            opacity: label.owner_opacity,
        });
    }

    let subjects = controlled_body_boxes(controlled_bodies.as_deref(), &world.0);
    resolve_label_layout(&mut placements, &subjects, &settings);

    // Placements are keyed by owner id, which is unique per label family
    // member. Look-up by id rather than by index because the resolver sorts.
    let resolved: std::collections::HashMap<&str, &LabelPlacement> = placements
        .iter()
        .map(|placement| (placement.owner_id.as_str(), placement))
        .collect();

    for (mut label, _, _, _, mut transform, mut visibility, mut text_color, children) in &mut labels
    {
        let Some(placement) = resolved.get(label.owner_id.as_str()) else {
            continue;
        };
        let Some(placed) = placement.placed else {
            // Unplaceable. Cut to hidden THIS frame and leave the transform
            // alone — a label that lost every candidate position must not be
            // moved into the collision it lost, and must not linger there
            // easing out for ~0.3s. The ease exists to stop the subject fade
            // popping; it has no business smoothing a disappearance whose
            // whole purpose is that the text is not on screen.
            label.rendered_opacity = 0.0;
            *visibility = Visibility::Hidden;
            *text_color = TextColor(with_opacity(label.text_color, 0.0));
            paint_outlines(&mut outline_colors, children, &label, 0.0);
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
        paint_outlines(&mut outline_colors, children, &label, opacity);
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

/// **The generic world-label capability**: the settings, the placement pass, and
/// the typeface pass. Anything that spawns a [`WorldLabel`] needs this plugin,
/// and only this plugin.
///
/// ## Why it is not part of the nameplate plugin
///
/// It was, for one commit, and that made the AC12/AC20 policy true of exactly
/// one composition. `spawn_room_visuals` — which lives in the GENERIC
/// [`SessionRoomVisualsPlugin`](crate::platformer_presentation::SessionRoomVisualsPlugin),
/// not in Ambition — spawns signage and fixture labels; the systems that give
/// those components meaning were installed only by
/// [`ActorNameplatePresentationPlugin`](super::nameplates::ActorNameplatePresentationPlugin),
/// which the demos and the external consumer do not add. So the external
/// consumer, Mary-O and Sanic kept drawing static labels at their raw anchors,
/// in Bevy's fallback typeface, with no subject fade — the mechanism existed and
/// one production composition still ran the old behaviour
/// ([[feedback-presentation-binding-fails-silently]]).
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
        app.init_resource::<WorldLabelLayoutSettings>().add_systems(
            Update,
            (apply_world_label_fonts, layout_world_labels)
                .chain()
                .in_set(WorldLabelLayoutSet)
                .run_if(ambition_platformer_primitives::lifecycle::session_world_exists),
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
            step_px: 10.0,
            max_steps: 3,
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

    /// The premise the whole module rests on, asserted rather than described:
    /// two labels FROM DIFFERENT FAMILIES that start on top of each other do
    /// not end on top of each other. This is the AC12 defect exactly — an
    /// actor nameplate under an authored sign.
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
            max_steps: 0,
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
