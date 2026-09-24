//! Sandbox cutscene presentation overlay.
//!
//! The cutscene script format and runtime stepper live in
//! [`ambition_cutscene`] (data and logic, plus the playback resources
//! `ActiveCutscene` and `CutsceneAdvanceRequest`). The cutscene runtime seam
//! drives them (triggers, queue drain, tick, save-flag effects). Scripts and
//! bindings are content (`ambition_content`).
//!
//! This module is presentation only: it reads `ActiveCutscene` and draws the
//! screen-space overlay (banner / dialogue cards + skip-hold progress bar).

use ambition_cutscene::ActiveCutscene;
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────
// Presentation overlay
// ─────────────────────────────────────────────────────────────────────
//
// Two surfaces show narrative text:
//
// - Cutscene overlay (this module): a screen-space Bevy UI panel that draws
//   `CutsceneBeat::Dialogue` (waits for player input) and
//   `CutsceneBeat::Banner` (timed, auto-advances), plus the skip-hold
//   progress bar. Owned by [`sync_cutscene_ui`].
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

/// Build or refresh the cutscene UI overlay, like the dialogue presenter:
/// despawn last frame's overlay and spawn this frame's from `ActiveCutscene`
/// and `CutsceneAdvanceRequest`.
///
/// Layout:
/// - Banner beats: centered card near the top, no input prompt
///   (auto-advances after the beat's timer).
/// - Fade beats: a full-screen black sheet at the beat's current ramp value,
///   under the cards.
/// - Dialogue beats: speaker + body card near the bottom, with a
///   "Press Interact / Jump to continue" hint (acknowledge mode).
/// - Skip-hold progress: thin bar near the bottom-right, only while
///   the player is holding Reset (Backspace / pad-Select).
pub fn sync_cutscene_ui(
    mut commands: Commands,
    active: Res<ActiveCutscene>,
    skip_hold: Res<ambition_cutscene::CutsceneSkipHold>,
    overlays: Query<Entity, With<CutsceneOverlayRoot>>,
    ui_fonts: Option<Res<crate::ui_fonts::UiFonts>>,
    presentation: Option<
        Res<ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation>,
    >,
) {
    use bevy::ui::{
        AlignItems, BorderRadius, FlexDirection, JustifyContent, Node, PositionType, UiRect, Val,
        ZIndex,
    };

    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !active.is_playing() {
        return;
    }

    let cutscene_font = |font_size: f32, weight: crate::ui_fonts::UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            })
    };

    let banner = active.presentation.banner.as_ref();
    let dialogue = active.presentation.dialogue.as_ref();
    let skip_progress = skip_hold.progress();
    let fade_alpha = active.presentation.fade_alpha.clamp(0.0, 1.0);

    // The fade is its own root. The card root below is placed inside the
    // reading rect (`place_in_reading_rect`), a sub-region of the window, so a
    // sheet parented there would darken only that rectangle. It sits one layer
    // under the cards, so a line spoken over a fade stays readable.
    if fade_alpha > 0.001 {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, fade_alpha)),
            ZIndex(49),
            Name::new("Cutscene Fade Sheet"),
            CutsceneOverlayRoot,
        ));
    }

    // Return early when there is nothing to show (for example between beats
    // during a CameraPan). The runtime stays active in `ActiveCutscene`.
    if banner.is_none() && dialogue.is_none() && skip_progress <= 0.01 {
        return;
    }

    commands
        .spawn((
            // Full-screen with `SpaceBetween` puts the speaker line and the skip
            // meter at the bottom, under the stick and action cluster on a phone.
            {
                let mut node = Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                };
                crate::reading_layout::place_in_reading_rect(&mut node, presentation.as_deref());
                node
            },
            ZIndex(50),
            Name::new("Cutscene Overlay Root"),
            CutsceneOverlayRoot,
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
            } else {
                // Spacer, so the dialogue card stays at the bottom when no banner
                // shows.
                root.spawn(Node::default());
            }

            // Bottom: dialogue card. Waits for Interact or Jump, which
            // `populate_control_frame_from_actions` turns into
            // `request.dismiss_dialogue`.
            if let Some((speaker, text)) = dialogue {
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        max_width: Val::Px(960.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.030, 0.045, 0.95)),
                    BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.86)),
                    Name::new("Cutscene Dialogue Panel"),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(speaker.clone()),
                        cutscene_font(20.0, crate::ui_fonts::UiFontWeight::Semibold),
                        TextColor(Color::srgba(0.82, 0.94, 1.00, 1.0)),
                    ));
                    panel.spawn((
                        Text::new(text.clone()),
                        cutscene_font(16.0, crate::ui_fonts::UiFontWeight::Regular),
                        TextColor(Color::srgba(0.93, 0.96, 1.00, 1.0)),
                    ));
                    panel.spawn((
                        // Bindings live in
                        // `ambition_input::presets::ControlPreset::input_map`
                        // (Interact = E, Jump = Space/W by default). The hint names
                        // the semantic actions, so a rebound key stays accurate.
                        Text::new("Press Interact (E) or Jump (Space) to continue. Hold Backspace to skip."),
                        cutscene_font(12.0, crate::ui_fonts::UiFontWeight::Regular),
                        TextColor(Color::srgba(0.66, 0.76, 0.88, 0.96)),
                    ));
                });
            } else {
                root.spawn(Node::default());
            }
        });

    // Skip-hold progress bar: bottom-right, a separate root so it does not
    // affect the main column layout. Spawned only while skip is held.
    if skip_progress > 0.01 {
        let fill_pct = (skip_progress * 100.0).clamp(0.0, 100.0);
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    bottom: Val::Px(24.0),
                    width: Val::Px(220.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ZIndex(51),
                Name::new("Cutscene Skip Progress"),
                CutsceneOverlayRoot,
            ))
            .with_children(|root| {
                root.spawn((
                    Text::new(format!("hold to skip … {fill_pct:>3.0}%")),
                    cutscene_font(12.0, crate::ui_fonts::UiFontWeight::Regular),
                    TextColor(Color::srgba(0.86, 0.86, 0.92, 0.92)),
                ));
                root.spawn((
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_cutscene::{CutsceneBeat, CutsceneRuntime, CutsceneScript, CutsceneSkipHold};

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
        let mut active = ActiveCutscene {
            presentation: runtime.presentation(),
            runtime: Some(runtime),
        };
        // The projection is a cache that the renderer reads, so refresh it
        // like the gameplay tick does.
        active.presentation = active.runtime.as_ref().unwrap().presentation();

        let mut app = App::new();
        app.insert_resource(active)
            .insert_resource(CutsceneSkipHold::default())
            .add_systems(Update, sync_cutscene_ui);
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
}
