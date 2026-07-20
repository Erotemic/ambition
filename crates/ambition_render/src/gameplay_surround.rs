//! Surround presentation for a fixed-aspect gameplay viewport.
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! **This is not decoration.** A Bevy camera with a `viewport` set never clears
//! the display outside it, so a fixed-aspect profile leaves the surround
//! undefined until something paints it. The base fill below is that something;
//! the [`SurroundPolicy`] then decides how the surround READS, not whether it
//! exists.
//!
//! Render owns the drawing and nothing else — it never selects policy. It reads
//! the one resolved layout the host published and paints what that layout says
//! the gameplay camera does not cover.

use bevy::prelude::*;

use ambition_platformer_primitives::gameplay_presentation::{
    ResolvedGameplayPresentation, SurroundPolicy, SurroundRegion,
};

/// Behind every other UI layer: HUD, menus, dialogue and the touch overlay all
/// draw over the surround.
const SURROUND_Z: i32 = -10_000;

/// Root of the surround fill. One root so teardown is a single despawn.
#[derive(Component)]
struct GameplaySurroundRoot;

#[derive(Component, Clone, Copy)]
struct GameplaySurroundBar(SurroundRegion);

/// Draws the region a viewport-clipped gameplay camera leaves unpainted.
pub struct GameplaySurroundPlugin;

impl Plugin for GameplaySurroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_gameplay_surround);
    }
}

/// Base surround colour.
///
/// [`SurroundPolicy::GameAuthored`] and
/// [`SurroundPolicy::DecorativeWorldExtension`] still get the base fill: they
/// describe what a game draws ON TOP, and skipping the fill for them would
/// leave unpainted display when the game draws nothing.
fn surround_color(policy: SurroundPolicy) -> Color {
    match policy {
        SurroundPolicy::None | SurroundPolicy::Solid => Color::srgb(0.02, 0.02, 0.03),
        SurroundPolicy::GameAuthored | SurroundPolicy::DecorativeWorldExtension => {
            Color::srgb(0.04, 0.04, 0.06)
        }
    }
}

fn sync_gameplay_surround(
    mut commands: Commands,
    presentation: Res<ResolvedGameplayPresentation>,
    roots: Query<Entity, With<GameplaySurroundRoot>>,
    mut bars: Query<(&GameplaySurroundBar, &mut Node, &mut BackgroundColor)>,
) {
    let letterbox = presentation.letterbox_rects();

    // Full bleed: nothing is unpainted, so there is nothing to own.
    if letterbox.is_empty() {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }

    let color = surround_color(presentation.surround);

    if roots.is_empty() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                GlobalZIndex(SURROUND_Z),
                // The surround is scenery, never a pointer target: a click on a
                // bar belongs to whatever menu is open, not to the bar.
                Pickable::IGNORE,
                GameplaySurroundRoot,
                Name::new("GameplaySurround"),
            ))
            .with_children(|root| {
                for region in [
                    SurroundRegion::Left,
                    SurroundRegion::Right,
                    SurroundRegion::Top,
                    SurroundRegion::Bottom,
                ] {
                    root.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        BackgroundColor(color),
                        Pickable::IGNORE,
                        GameplaySurroundBar(region),
                    ));
                }
            });
        // The bars this frame carry default zero-size nodes; the next update
        // lays them out. One frame of unpainted surround at profile activation
        // is invisible behind the room transition that caused it.
        return;
    }

    for (bar, mut node, mut background) in &mut bars {
        let rect = letterbox
            .iter()
            .find(|named| named.region == bar.0)
            .map(|named| named.rect);
        match rect {
            Some(rect) => {
                node.left = Val::Px(rect.min.x);
                node.top = Val::Px(rect.min.y);
                node.width = Val::Px(rect.width());
                node.height = Val::Px(rect.height());
            }
            None => {
                // This side has no slack on the current display; collapse it
                // rather than despawning, so an aspect change re-uses the node.
                node.width = Val::Px(0.0);
                node.height = Val::Px(0.0);
            }
        }
        if background.0 != color {
            background.0 = color;
        }
    }
}

#[cfg(test)]
mod tests;
