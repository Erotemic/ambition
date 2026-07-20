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

use ambition_engine_core as ae;
use ambition_platformer_primitives::gameplay_presentation::{
    NamedScreenRect, ResolvedGameplayPresentation, SurroundPolicy, SurroundRegion,
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
        // After the layout this paints against is resolved. Unordered, the
        // surround trails the resolve by a frame on every profile change —
        // including the very first one, which is the frame a fixed-aspect game
        // starts and the pillarboxes are at their most visible.
        app.add_systems(
            Update,
            sync_gameplay_surround.after(
                ambition_platformer_primitives::gameplay_presentation::GameplayPresentationSet,
            ),
        );
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
                    // Laid out AT SPAWN, not on the next update: the frame a
                    // fixed-aspect game starts is the frame the pillarboxes
                    // appear, and a one-frame flash of uncleared framebuffer
                    // is exactly the artifact this system exists to prevent.
                    root.spawn((
                        bar_node(&letterbox, region),
                        BackgroundColor(color),
                        Pickable::IGNORE,
                        GameplaySurroundBar(region),
                    ));
                }
            });
        return;
    }

    for (bar, mut node, mut background) in &mut bars {
        let next = bar_node(&letterbox, bar.0);
        if *node != next {
            *node = next;
        }
        if background.0 != color {
            background.0 = color;
        }
    }
}

/// The absolute node for one surround side.
///
/// A side with no slack on the current display collapses to zero rather than
/// despawning, so an aspect or display change re-uses the node instead of
/// churning the hierarchy.
fn bar_node(letterbox: &[NamedScreenRect], region: SurroundRegion) -> Node {
    let rect = letterbox
        .iter()
        .find(|named| named.region == region)
        .map(|named| named.rect);
    let (min, size) = match rect {
        Some(rect) => (rect.min, rect.size()),
        None => (ae::Vec2::ZERO, ae::Vec2::ZERO),
    };
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(min.x),
        top: Val::Px(min.y),
        width: Val::Px(size.x),
        height: Val::Px(size.y),
        ..default()
    }
}

#[cfg(test)]
mod tests;
