//! The name of a level over the pipe that goes to it.
//!
//! Presentation only, and content-owned. A pipe whose role is `Warp` goes to a
//! level (`WarpPipe`); the words over it are that level's title, less the word
//! WORLD ("1-3"). No level authors the words: a pipe that is put in a level
//! has them.
//!
//! The words are dark, with a white edge on each side. A level of Mary-O is
//! pale or dark by its biome, and white words with no edge could not be read
//! on the pale ones (Jon, 2026-10-12: "a darker text probably with a white
//! outline so it always stands out even if the background changes").
//!
//! A sign is a world label of the engine (`WorldLabel`, `StaticWorldLabel`),
//! of the family of a plate on a door: the engine gives it its font, in the
//! weight of a name that is read at a glance, and its place in the view. The
//! edge is copies of the words behind them, as an actor's nameplate has; the
//! engine gives a font to the label only, so
//! [`edges_take_the_font_of_their_sign`] gives the same font to its edge.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionScopeSet, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d::render::rendering::{
    RoomVisual, StaticWorldLabel, WorldLabel, WorldLabelFamily, WorldLabelLayoutSettings,
};
use ambition_platformer2d::world::rooms::LiveRoomSpecs;

/// Where the copies of the edge are, in parts of `pipe_signs.edge_px` (her
/// rules file): the four sides and the four corners.
const EDGE_AT: [(f32, f32); 8] =
    [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)];

/// One per live room that has its signs.
#[derive(Component)]
struct PresentedPipeSigns;

/// A copy of the words of a sign, behind them: a part of its edge.
#[derive(Component)]
struct PipeSignEdge;

/// The words over a pipe to the level whose title is `title`, or whose id is
/// `id` when it has no title.
pub fn sign_words(title: Option<&str>, id: &str) -> String {
    let title = title.map(str::trim).filter(|title| !title.is_empty()).unwrap_or(id);
    title.strip_prefix("WORLD ").unwrap_or(title).to_string()
}

/// The systems of the signs. A plugin, so that the two roads that install the
/// game (`MaryOPlugin` and the provider) add them one time, and no resource in
/// the world of the simulation says that they are there.
struct PipeSignsPlugin;

impl Plugin for PipeSignsPlugin {
    fn build(&self, app: &mut App) {
        add_systems(app);
    }
}

pub fn install(app: &mut App) {
    if !app.is_plugin_added::<PipeSignsPlugin>() {
        app.add_plugins(PipeSignsPlugin);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            // With the room's own visuals, and only where the engine draws
            // world labels: a composition with no view draws no sign.
            present_pipe_signs
                .in_set(SessionScopeSet::Presentation)
                .run_if(resource_exists::<WorldLabelLayoutSettings>),
            edges_take_the_font_of_their_sign,
        ),
    );
}

/// Give each live room the signs of its pipes to other levels, one time.
fn present_pipe_signs(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    presented: Query<&InRoomInstance, With<PresentedPipeSigns>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    // No session: no room is live, and there is nothing to put a sign in.
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    // How the signs look is `pipe_signs` in her rules file.
    let look = crate::rules::mary_o_rules().pipe_signs;
    let rgb = |[r, g, b]: [f32; 3]| Color::srgb(r, g, b);
    let (ink, edge) = (rgb(look.ink), rgb(look.edge));
    for (room, definition) in rooms.live_rooms() {
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let spec = rooms.rooms().spec(definition);
        let scope = session_scope.in_room(Some(room));
        for warp in crate::warp_pipes_for_room(&spec.id) {
            let title = rooms
                .rooms()
                .rooms
                .iter()
                .find(|level| level.id == warp.room)
                .and_then(|level| level.metadata.title.as_deref());
            let words = sign_words(title, &warp.room);
            let mouth = warp.mouth.aabb;
            let at = ae::Vec2::new((mouth.min.x + mouth.max.x) * 0.5, mouth.min.y - look.rise_px);
            let anchor = world_to_bevy(&spec.world, at, WORLD_Z_PLAYER + 8.0);
            let font = TextFont { font_size: FontSize::Px(look.size_px), ..default() };
            commands
                .spawn_session_scoped(
                    scope,
                    (
                        Text2d::new(words.clone()),
                        font.clone(),
                        TextColor(ink),
                        Transform::from_translation(anchor),
                        RoomVisual,
                        WorldLabel::new(format!("mary_o_pipe_sign:{}", warp.mouth.name), WorldLabelFamily::Fixture, anchor)
                            .with_colors(ink, Some(edge)),
                        StaticWorldLabel,
                    ),
                )
                .with_children(|sign| {
                    for (dx, dy) in EDGE_AT {
                        sign.spawn((
                            Text2d::new(words.clone()),
                            font.clone(),
                            TextColor(edge),
                            Transform::from_xyz(dx * look.edge_px, dy * look.edge_px, -0.1),
                            PipeSignEdge,
                        ));
                    }
                });
        }
        commands.spawn_session_scoped(scope, (PresentedPipeSigns, RoomVisual));
    }
}

/// The engine gives a world label its font after it is spawned. The edge of a
/// sign is not a label: it takes the font of the sign it is a part of.
fn edges_take_the_font_of_their_sign(
    signs: Query<(&TextFont, &Children), (With<WorldLabel>, Changed<TextFont>)>,
    mut edges: Query<&mut TextFont, (With<PipeSignEdge>, Without<WorldLabel>)>,
) {
    for (font, children) in &signs {
        for child in children.iter() {
            if let Ok(mut edge) = edges.get_mut(child) {
                *edge = font.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_of_a_sign_are_the_title_of_the_level_less_the_word_world() {
        assert_eq!(sign_words(Some("WORLD 1-3"), "mary_o_1_3"), "1-3");
        assert_eq!(sign_words(Some("WARP ZONE"), "mary_o_warp"), "WARP ZONE");
        assert_eq!(sign_words(None, "mary_o_9_9"), "mary_o_9_9");
        assert_eq!(sign_words(Some("  "), "mary_o_9_9"), "mary_o_9_9");
    }
}
