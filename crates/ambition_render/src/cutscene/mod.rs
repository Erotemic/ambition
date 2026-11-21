//! Sandbox cutscene presentation overlay.
//!
//! The cutscene script format and runtime stepper live in
//! [`ambition_cutscene`] (data and logic, plus the playback resource
//! `ActiveCutscene`). The cutscene runtime seam
//! drives them (triggers, queue drain, tick, save-flag effects). Scripts and
//! bindings are content (`ambition_content`).
//!
//! This module is presentation only: it reads `ActiveCutscene` and draws the
//! screen-space overlay (banner card, fade sheet, skip-hold progress bar). A
//! cutscene's dialogue line is not drawn here: it is published on
//! `DialogView` and drawn by the game's one dialogue box.

use ambition_cutscene::ActiveCutscene;
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────
// Presentation overlay
// ─────────────────────────────────────────────────────────────────────
//
// Two surfaces show narrative text:
//
// - The dialogue box (`DialogView`, the game's presenter): every line that
//   waits for the player, a conversation's or a cutscene's
//   (`CutsceneBeat::Dialogue`), with the answering seat's controls.
//
// - Cutscene overlay (this module): `CutsceneBeat::Banner` (timed,
//   auto-advances), the fade, and the skip-hold progress bar. Owned by
//   [`sync_cutscene_ui`].
//
// - Speech bubbles (`crate::fx::update_speech_bubbles`): world-space quote
//   bubbles that anything can fire with
//   `VfxMessage::SpeechBubble { pos, text }` (for example enemies shouting
//   when hit). No UI input and no pause; the line floats up and fades. The
//   cutscene overlay never owns it.
//
// Both run in the presentation half; headless and RL builds skip them.

/// Root entity for cutscene screen-space presentation: the card panel, and
/// the fade sheet, which is a second root, not a child (see
/// [`sync_cutscene_ui`]). Despawned and respawned each frame; they exist
/// only while a cutscene plays.
#[derive(Component)]
pub struct CutsceneOverlayRoot;

/// What the cutscene overlay shows, as [`sync_cutscene_ui`] draws it: the
/// banner's text (not its countdown), and the fade and skip hold to the
/// precision they are drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawnCutscene {
    banner: Option<String>,
    fade: u8,
    skip: u8,
}

impl DrawnCutscene {
    fn of(presentation: &ambition_cutscene::CutscenePresentation, skip_progress: f32) -> Self {
        Self {
            banner: presentation.banner.as_ref().map(|(text, _seconds)| text.clone()),
            fade: (presentation.fade_alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
            skip: (skip_progress.clamp(0.0, 1.0) * 100.0).round() as u8,
        }
    }
}

/// Run condition of [`sync_cutscene_ui`]: a cutscene plays, or its overlay is
/// still drawn and is taken away on this run.
pub fn a_cutscene_plays_or_is_drawn(active: Res<ActiveCutscene>, overlays: Query<(), With<CutsceneOverlayRoot>>) -> bool {
    active.is_playing() || !overlays.is_empty()
}

/// Where a cutscene's overlay goes on a split screen: the view of the seat
/// that answers it, as its whole view (the fade) and its reading region (the
/// cards). `None`: over the whole window, as for a cutscene of the whole world
/// or a shared view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayRegion {
    view: Rect,
    reading: Rect,
}

/// Build or refresh the cutscene UI overlay, like the dialogue presenter:
/// despawn last frame's overlay and spawn this frame's from `ActiveCutscene`.
/// Each playing cutscene draws its own (SIMULATION-ISLANDS N4): a room's
/// over the view of the seat that answers it, when the screen is split.
///
/// Layout:
/// - Banner beats: centered card near the top, no input prompt
///   (auto-advances after the beat's timer).
/// - Fade beats: a black sheet at the beat's current ramp value, under the
///   cards.
/// - Skip-hold progress: thin bar near the bottom-right, only while
///   the answering seat holds back.
pub fn sync_cutscene_ui(
    mut commands: Commands,
    active: Res<ActiveCutscene>,
    overlays: Query<Entity, With<CutsceneOverlayRoot>>,
    ui_fonts: Option<Res<crate::ui_fonts::UiFonts>>,
    presentation: Option<
        Res<ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation>,
    >,
    // The seat that answers each room's cutscene, and its view.
    (surface, drivers, rooms): (
        crate::ui_surface::ParticipantSurface,
        Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    ),
    mut drawn: Local<Vec<(DrawnCutscene, Option<OverlayRegion>)>>,
) {
    let region = |cutscene: &ambition_cutscene::PlayingCutscene| {
        cutscene.room?;
        if surface.view_count() < 2 {
            return None;
        }
        let seat = cutscene.answering_seat(
            drivers.iter().map(|(driver, body)| (driver.0, rooms.of(body).map(|at| at.ordinal()))),
            ambition_characters::control::PlayerSlot::PRIMARY,
        );
        let owner = Some(ambition_input::InputOwner::Participant(ambition_input::ParticipantId(seat.0)));
        Some(OverlayRegion { view: surface.gameplay_rect(owner)?, reading: surface.rect(owner)? })
    };
    // Rebuilt only when what it shows changes: the runtime republishes the
    // presentation each tick (a banner's countdown, which is not drawn), and a
    // rebuild lays out and measures the whole overlay again.
    let showing: Vec<(DrawnCutscene, Option<OverlayRegion>)> = active
        .iter()
        .map(|cutscene| (DrawnCutscene::of(&cutscene.presentation, cutscene.skip_progress()), region(cutscene)))
        .collect();
    let restyled = ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed())
        || presentation.as_ref().is_some_and(|presentation| presentation.is_changed());
    if !restyled && *drawn == showing && overlays.is_empty() == showing.is_empty() {
        return;
    }
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    for (cutscene, (_, region)) in active.iter().zip(&showing) {
        spawn_cutscene_overlay(&mut commands, cutscene, *region, ui_fonts.as_deref(), presentation.as_deref(), &surface);
    }
    *drawn = showing;
}

/// Spawn one cutscene's overlay roots.
fn spawn_cutscene_overlay(
    commands: &mut Commands,
    cutscene: &ambition_cutscene::PlayingCutscene,
    region: Option<OverlayRegion>,
    ui_fonts: Option<&crate::ui_fonts::UiFonts>,
    presentation: Option<&ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation>,
    surface: &crate::ui_surface::ParticipantSurface,
) {
    use bevy::ui::{
        AlignItems, BorderRadius, FlexDirection, JustifyContent, Node, PositionType, UiRect, Val,
        ZIndex,
    };

    let cutscene_font = |font_size: f32, weight: crate::ui_fonts::UiFontWeight| {
        ui_fonts
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            })
    };
    let fill = || Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    };

    let banner = cutscene.presentation.banner.as_ref();
    let skip_progress = cutscene.skip_progress();
    let fade_alpha = cutscene.presentation.fade_alpha.clamp(0.0, 1.0);

    // The fade is its own root. The card root below is placed inside the
    // reading rect (`place_in_reading_rect`), a sub-region of the window, so a
    // sheet parented there would darken only that rectangle. It sits under the
    // dialogue box (45) and the banner, so a line spoken over a fade stays
    // readable. On a split screen it darkens its own view only.
    if fade_alpha > 0.001 {
        let mut node = fill();
        if let Some(region) = region {
            surface.place(&mut node, region.view);
        }
        commands.spawn((
            node,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, fade_alpha)),
            ZIndex(44),
            Name::new("Cutscene Fade Sheet"),
            CutsceneOverlayRoot,
        ));
    }

    // Return early when there is nothing to show (for example between beats
    // during a CameraPan). The runtime stays active in `ActiveCutscene`.
    if banner.is_none() && skip_progress <= 0.01 {
        return;
    }

    let mut node = Node {
        padding: UiRect::all(Val::Px(24.0)),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::FlexStart,
        align_items: AlignItems::Center,
        ..fill()
    };
    match region {
        Some(region) => surface.place(&mut node, region.reading),
        None => crate::reading_layout::place_in_reading_rect(&mut node, presentation),
    }
    commands
        .spawn((
            node,
            ZIndex(50),
            Name::new("Cutscene Overlay Root"),
            CutsceneOverlayRoot,
            // A full-screen layout box with nothing to press. Picked, it would
            // sit over a pause menu and take every click and tap meant for it.
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|root| {
            // Top: banner card. The runtime advances it on its own timer; the
            // player does not press anything.
            if let Some((banner_text, _seconds)) = banner {
                root.spawn((
                    Node {
                        max_width: Val::Px(720.0),
                        padding: UiRect::axes(Val::Px(22.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.92)),
                    BorderColor::all(Color::srgba(0.84, 0.72, 0.40, 0.78)),
                    Name::new("Cutscene Banner"),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(banner_text.clone()),
                        cutscene_font(18.0, crate::ui_fonts::UiFontWeight::Semibold),
                        TextColor(Color::srgba(0.96, 0.90, 0.74, 1.0)),
                    ));
                });
            }
        });

    // Skip-hold progress bar: bottom-right of the window (of its own view, on
    // a split screen), a separate root so it does not affect the main column
    // layout. Spawned only while skip is held.
    if skip_progress > 0.01 {
        let fill_pct = (skip_progress * 100.0).clamp(0.0, 100.0);
        let (mut node, inner_width) = match region {
            Some(region) => {
                let mut node = Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::FlexEnd,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                };
                surface.place(&mut node, region.view);
                (node, Val::Px(220.0))
            }
            None => (
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    bottom: Val::Px(24.0),
                    width: Val::Px(220.0),
                    ..default()
                },
                Val::Percent(100.0),
            ),
        };
        node.row_gap = Val::Px(4.0);
        commands
            .spawn((node, ZIndex(51), Name::new("Cutscene Skip Progress"), CutsceneOverlayRoot, bevy::picking::Pickable::IGNORE))
            .with_children(|root| {
                root.spawn(Node { width: inner_width, flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() })
                    .with_children(|column| {
                        column.spawn((
                            Text::new(format!("hold to skip … {fill_pct:>3.0}%")),
                            cutscene_font(12.0, crate::ui_fonts::UiFontWeight::Regular),
                            TextColor(Color::srgba(0.86, 0.86, 0.92, 0.92)),
                        ));
                        column.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(6.0),
                                border_radius: BorderRadius::all(Val::Px(3.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.20, 0.22, 0.28, 0.85)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: Val::Percent(fill_pct),
                                    height: Val::Percent(100.0),
                                    border_radius: BorderRadius::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96)),
                            ));
                        });
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_cutscene::{CutsceneBeat, CutsceneRuntime, CutsceneScript};

    /// Drive `sync_cutscene_ui` once over a cutscene sitting `elapsed` into a
    /// fade, and report the alpha of the sheet it drew, if any.
    fn drawn_fade_alpha(from: f32, to: f32, seconds: f32, elapsed: f32) -> Option<f32> {
        let script = CutsceneScript::new(
            "fade",
            vec![CutsceneBeat::Fade {
                from_alpha: from,
                to_alpha: to,
                seconds,
            }],
        );
        let mut runtime = CutsceneRuntime::new(script);
        let _ = runtime.tick(elapsed, false);
        let active = ActiveCutscene::playing(runtime, None);

        let mut app = App::new();
        app.insert_resource(active).add_systems(Update, sync_cutscene_ui);
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<(&Name, &BackgroundColor)>();
        q.iter(world)
            .find(|(name, _)| name.as_str() == "Cutscene Fade Sheet")
            .map(|(_, color)| color.0.alpha())
    }

    /// A fade beat draws a black sheet at its ramp value. `fade_alpha` must have
    /// a consumer; otherwise a fade is only a timer the player waits out.
    #[test]
    fn a_fade_beat_draws_a_black_sheet_at_the_ramp_value() {
        // Up from black, halfway through.
        let half = drawn_fade_alpha(1.0, 0.0, 0.8, 0.4).expect("a fade draws a sheet");
        assert!(
            (half - 0.5).abs() < 1e-5,
            "the sheet was drawn at {half} halfway through a fade up from black"
        );

        // The control is a clear screen: a renderer that always spawned a sheet
        // would pass the arm above, and a transparent sheet still costs a UI
        // node every frame.
        assert_eq!(
            drawn_fade_alpha(1.0, 0.0, 0.8, 0.8),
            None,
            "a completed fade up left a sheet behind"
        );
    }

    /// The overlay is rebuilt only when what it shows changes: a banner
    /// counting down keeps its cards, and a new line replaces them.
    #[test]
    fn the_overlay_is_kept_while_what_it_shows_is_the_same() {
        let script = CutsceneScript::new(
            "banner",
            vec![CutsceneBeat::Banner {
                text: "Chapter one".to_string(),
                seconds: 2.0,
            }],
        );
        let runtime = CutsceneRuntime::new(script);
        let mut app = App::new();
        app.insert_resource(ActiveCutscene::playing(runtime, None)).add_systems(Update, sync_cutscene_ui);
        app.update();
        let roots = |app: &mut App| {
            let world = app.world_mut();
            let mut roots: Vec<Entity> = world.query_filtered::<Entity, With<CutsceneOverlayRoot>>().iter(world).collect();
            roots.sort();
            roots
        };
        let first = roots(&mut app);
        assert!(!first.is_empty(), "a banner draws an overlay");
        // The countdown moves; the drawn text does not.
        if let Some((_, seconds)) = app.world_mut().resource_mut::<ActiveCutscene>().iter_mut().next().unwrap().presentation.banner.as_mut() {
            *seconds = 1.5;
        }
        app.update();
        assert_eq!(roots(&mut app), first, "a banner counting down rebuilt its overlay");
        app.world_mut().resource_mut::<ActiveCutscene>().iter_mut().next().unwrap().presentation.banner = Some(("Chapter two".to_string(), 2.0));
        app.update();
        let second = roots(&mut app);
        assert!(!second.is_empty() && second != first, "a new banner draws a new overlay");
    }
}
