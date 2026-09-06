//! Programmatic startup vanity-card segment.
//!
//! Rig solving and choreography are baked offline into
//! `assets/data/vanity_card_made_this_meme.ron`; runtime code only places baked
//! parts from one packed texture. The scene is scoped to its shell segment and
//! reports `ProgrammaticSegmentCompleted` when finished, so ordinary shell
//! teardown handles skip, route replacement, and failure.

use std::time::Duration;

use ambition_platformer2d::game_shell::{
    ActiveShellSequence, FrontendOwnedEntity, FrontendPresentationKind, ShellActivationId,
    ShellSegmentId, ShellSegmentScopedEntity, ShellSequenceCommand,
};
use bevy::prelude::*;
use serde::Deserialize;

/// Registered shell-segment kind for the vanity card.
pub const MADE_THIS_MEME_CARD_SEGMENT_KIND: &str = "ambition_vanity_card_made_this_meme";

/// The baked animation. Committed, and generated — see the module docs.
const MADE_THIS_MEME_RON: &str = include_str!("../../assets/data/vanity_card_made_this_meme.ron");

/// The `game://` asset source is the content crate's own `assets/` tree.
const ASSET_SOURCE: &str = "game://";

/// Card colours, matching the renderer's own (`render_author_vanity_dialog.py`).
const CARD_FILL: Color = Color::srgb(0.965, 0.969, 0.984);
const CARD_OUTLINE: Color = Color::srgb(0.824, 0.847, 0.894);
const BACKDROP: Color = Color::srgb(0.047, 0.055, 0.086);
const BUBBLE_FILL: Color = Color::srgb(1.0, 1.0, 1.0);
const BUBBLE_OUTLINE: Color = Color::srgb(0.659, 0.698, 0.776);
const TEXT_INK: Color = Color::srgb(0.125, 0.141, 0.180);
const CARD_MARGIN: f32 = 8.0;

#[derive(Deserialize)]
struct RigCard {
    canvas: (f32, f32),
    frame_ms: u64,
    sheet: String,
    parts: Vec<PartRect>,
    frames: Vec<CardFrame>,
}

#[derive(Deserialize)]
struct PartRect {
    #[allow(dead_code)]
    name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Deserialize)]
struct CardFrame {
    draws: Vec<Draw>,
    bubble: Option<Bubble>,
}

/// One part, on one frame: which sheet rect, where its CENTRE lands in canvas
/// units, how big it is there, and how far it is spun clockwise.
#[derive(Deserialize)]
struct Draw {
    part: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    deg: f32,
}

#[derive(Deserialize)]
struct Bubble {
    text: String,
    tail_x: f32,
    tail_y: f32,
    box_x: f32,
    box_y: f32,
}

fn card() -> &'static RigCard {
    use std::sync::OnceLock;
    static CARD: OnceLock<RigCard> = OnceLock::new();
    CARD.get_or_init(|| {
        ron::from_str(MADE_THIS_MEME_RON)
            .expect("vanity_card_made_this_meme.ron is generated and compiled in; it must parse")
    })
}

/// How long the card plays: its own frame count at its own frame rate.
pub fn made_this_meme_card_duration() -> Duration {
    let card = card();
    Duration::from_millis(card.frame_ms * card.frames.len() as u64)
}

pub struct MadeThisMemeCardPlugin;

impl Plugin for MadeThisMemeCardPlugin {
    fn build(&self, app: &mut App) {
        // The card exists only inside a shell sequence. Gate all of its systems so
        // shell-less compositions do not require shell resources or message types.
        app.add_systems(
            Update,
            (spawn_vanity_card, animate_vanity_card, fit_card_to_display)
                .run_if(resource_exists::<ActiveShellSequence>),
        );
    }
}

#[derive(Component)]
struct VanityCardRoot {
    activation_id: ShellActivationId,
    segment_id: ShellSegmentId,
    elapsed: Duration,
    completion_sent: bool,
    stage: Entity,
    slots: Vec<Entity>,
    bubble: Entity,
    bubble_text: Entity,
    bubble_tail: Entity,
}

/// The full-screen box the card is scaled to fit inside.
#[derive(Component)]
struct VanityCardViewport;

/// The canvas-sized node everything is laid out in, scaled to the viewport.
#[derive(Component)]
struct VanityCardStage;

fn spawn_vanity_card(
    mut commands: Commands,
    active: Res<ActiveShellSequence>,
    roots: Query<&VanityCardRoot>,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Some((activation_id, segment_id, kind)) = active
        .registered_segment()
        .map(|(activation_id, segment_id, kind)| (activation_id, segment_id.clone(), kind.clone()))
    else {
        return;
    };
    if kind.as_str() != MADE_THIS_MEME_CARD_SEGMENT_KIND {
        return;
    }
    if roots
        .iter()
        .any(|root| root.activation_id == activation_id && root.segment_id == segment_id)
    {
        return;
    }

    let card = card();
    let (canvas_w, canvas_h) = card.canvas;

    // ONE texture, and one layout naming every part's rect inside it.
    // ⛔ THROUGH THE STAMPED ROAD, not `assets.load` directly. A bare load lands
    // in `Assets<Image>` with no demand recorded, so the image-stage ledger
    // reports it as `demand=unknown` — the census sees the insertion and has
    // nothing to measure it against, which is the one shape that instrument
    // cannot explain.
    let sheet: Handle<Image> = ambition_sprite_sheet::game_assets::load_sheet_image(
        &assets,
        "vanity-card",
        format!("{ASSET_SOURCE}{}", card.sheet),
    );
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(1, 1));
    for part in &card.parts {
        layout.add_texture(URect::new(part.x, part.y, part.x + part.w, part.y + part.h));
    }
    let layout = layouts.add(layout);

    // As many part slots as the busiest frame needs, reused every frame. Parts
    // come and go (the robot blinks), so a slot is hidden rather than despawned:
    // spawning during playback would put command flushes on the animation path.
    let slot_count = card.frames.iter().map(|f| f.draws.len()).max().unwrap_or(0);

    let mut slots = Vec::with_capacity(slot_count);
    let mut stage = Entity::PLACEHOLDER;
    let mut bubble = Entity::PLACEHOLDER;
    let mut bubble_text = Entity::PLACEHOLDER;
    let mut bubble_tail = Entity::PLACEHOLDER;

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BACKDROP),
            GlobalZIndex(4_500),
            FrontendOwnedEntity::shell(activation_id, FrontendPresentationKind::StartupRoot),
            ShellSegmentScopedEntity {
                activation_id,
                segment_id: segment_id.clone(),
            },
            Name::new("Ambition Vanity Card Root"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                VanityCardViewport,
                Name::new("Ambition Vanity Card Viewport"),
            ))
            .with_children(|viewport| {
                let mut stage_cmds = viewport.spawn((
                    Node {
                        width: Val::Px(canvas_w),
                        height: Val::Px(canvas_h),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    VanityCardStage,
                    Name::new("Ambition Vanity Card Stage"),
                ));
                stage = stage_cmds.id();
                stage_cmds.with_children(|stage| {
                    // The comic panel the characters stand on.
                    stage.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(CARD_MARGIN),
                            top: Val::Px(CARD_MARGIN),
                            width: Val::Px(canvas_w - CARD_MARGIN * 2.0),
                            height: Val::Px(canvas_h - CARD_MARGIN * 2.0),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(22.0)),
                            ..default()
                        },
                        BackgroundColor(CARD_FILL),
                        BorderColor::all(CARD_OUTLINE),
                        Name::new("Ambition Vanity Card Panel"),
                    ));

                    for index in 0..slot_count {
                        slots.push(
                            stage
                                .spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    ImageNode::from_atlas_image(
                                        sheet.clone(),
                                        TextureAtlas {
                                            layout: layout.clone(),
                                            index: 0,
                                        },
                                    ),
                                    Visibility::Hidden,
                                    Name::new(format!("Ambition Vanity Part {index}")),
                                ))
                                .id(),
                        );
                    }

                    bubble_tail = stage
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            BackgroundColor(BUBBLE_FILL),
                            BorderColor::all(BUBBLE_OUTLINE),
                            Visibility::Hidden,
                            Name::new("Ambition Vanity Bubble Tail"),
                        ))
                        .id();

                    let mut bubble_cmds = stage.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            padding: UiRect::axes(Val::Px(13.0), Val::Px(9.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(13.0)),
                            max_width: Val::Px(186.0),
                            ..default()
                        },
                        BackgroundColor(BUBBLE_FILL),
                        BorderColor::all(BUBBLE_OUTLINE),
                        Visibility::Hidden,
                        Name::new("Ambition Vanity Speech Bubble"),
                    ));
                    bubble = bubble_cmds.id();
                    bubble_cmds.with_children(|bubble| {
                        bubble_text = bubble
                            .spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(18.0),
                                    ..default()
                                },
                                TextColor(TEXT_INK),
                                Name::new("Ambition Vanity Speech Text"),
                            ))
                            .id();
                    });
                });
            });
        })
        .id();

    commands.entity(root).insert(VanityCardRoot {
        activation_id,
        segment_id,
        elapsed: Duration::ZERO,
        completion_sent: false,
        stage,
        slots,
        bubble,
        bubble_text,
        bubble_tail,
    });
}

/// Scale the canvas-sized stage to fill whatever the card is being shown in.
///
/// Uniform scale, so the card letterboxes instead of stretching; every position
/// in the baked table then means the same thing at 640x360 and at 4K, and the
/// speech text scales with the art rather than growing out of its bubble.
///
/// the measurement is converted to LOGICAL pixels before it is compared to the
/// canvas. `ComputedNode::size()` is PHYSICAL, and the stage is sized in `Val::Px`
/// — which bevy_ui has already multiplied by the display scale factor — so
/// dividing the raw sizes asks "how many device pixels fit in a logical canvas"
/// and answers with a scale inflated by exactly that factor. A desktop at 1.0
/// cannot tell the difference, which is why every check of this passed; a phone
/// at 2.0-3.5 drew the card zoomed several times past the edges of the screen
/// .
fn fit_card_to_display(
    viewports: Query<(&ComputedNode, &Children), With<VanityCardViewport>>,
    mut stages: Query<&mut UiTransform, With<VanityCardStage>>,
) {
    let (canvas_w, canvas_h) = card().canvas;
    for (computed, children) in &viewports {
        let size = computed.size() * computed.inverse_scale_factor();
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let scale = (size.x / canvas_w).min(size.y / canvas_h);
        for child in children.iter() {
            if let Ok(mut transform) = stages.get_mut(child) {
                if transform.scale != Vec2::splat(scale) {
                    transform.scale = Vec2::splat(scale);
                }
            }
        }
    }
}

fn animate_vanity_card(
    time: Res<Time>,
    mut roots: Query<&mut VanityCardRoot>,
    mut nodes: Query<&mut Node>,
    mut transforms: Query<&mut UiTransform>,
    mut images: Query<&mut ImageNode>,
    mut visibilities: Query<&mut Visibility>,
    mut texts: Query<&mut Text>,
    mut commands: MessageWriter<ShellSequenceCommand>,
) {
    let card = card();
    let total = made_this_meme_card_duration();
    for mut root in &mut roots {
        root.elapsed = (root.elapsed + time.delta()).min(total);

        // The last frame is HELD rather than wrapping: the card ends on the
        // author holding the game, and a wrap to the empty first frame would
        // flash the stage bare on the way out.
        let index = ((root.elapsed.as_millis() as u64) / card.frame_ms.max(1)) as usize;
        let frame = &card.frames[index.min(card.frames.len() - 1)];

        for (slot, draw) in root.slots.iter().zip(frame.draws.iter()) {
            let Some(part) = card.parts.get(draw.part) else {
                continue;
            };
            if let Ok(mut node) = nodes.get_mut(*slot) {
                node.left = Val::Px(draw.x - draw.w / 2.0);
                node.top = Val::Px(draw.y - draw.h / 2.0);
                node.width = Val::Px(draw.w);
                node.height = Val::Px(draw.h);
            }
            if let Ok(mut transform) = transforms.get_mut(*slot) {
                transform.rotation = Rot2::degrees(draw.deg);
            }
            if let Ok(mut image) = images.get_mut(*slot) {
                if let Some(atlas) = image.texture_atlas.as_mut() {
                    if atlas.index != draw.part {
                        atlas.index = draw.part;
                    }
                }
                let _ = part;
            }
            if let Ok(mut visibility) = visibilities.get_mut(*slot) {
                *visibility = Visibility::Inherited;
            }
        }
        for slot in root.slots.iter().skip(frame.draws.len()) {
            if let Ok(mut visibility) = visibilities.get_mut(*slot) {
                *visibility = Visibility::Hidden;
            }
        }

        match &frame.bubble {
            Some(bubble) => {
                if let Ok(mut node) = nodes.get_mut(root.bubble) {
                    node.left = Val::Px(bubble.box_x);
                    node.top = Val::Px(bubble.box_y);
                }
                if let Ok(mut text) = texts.get_mut(root.bubble_text) {
                    if text.0 != bubble.text {
                        text.0 = bubble.text.clone();
                    }
                }
                // The tail is a thin quad from the bubble to the speaker's head:
                // a rectangle rotated onto the line between them, which is the
                // cheapest thing that still says WHO is talking.
                let from = Vec2::new(bubble.box_x + 24.0, bubble.box_y + 34.0);
                let to = Vec2::new(bubble.tail_x, bubble.tail_y);
                let span = to - from;
                let length = span.length().max(1.0);
                if let Ok(mut node) = nodes.get_mut(root.bubble_tail) {
                    node.left = Val::Px(from.x + span.x / 2.0 - length / 2.0);
                    node.top = Val::Px(from.y + span.y / 2.0 - 1.5);
                    node.width = Val::Px(length);
                    node.height = Val::Px(3.0);
                }
                if let Ok(mut transform) = transforms.get_mut(root.bubble_tail) {
                    transform.rotation = Rot2::radians(span.y.atan2(span.x));
                }
                for entity in [root.bubble, root.bubble_tail] {
                    if let Ok(mut visibility) = visibilities.get_mut(entity) {
                        *visibility = Visibility::Inherited;
                    }
                }
            }
            None => {
                for entity in [root.bubble, root.bubble_tail] {
                    if let Ok(mut visibility) = visibilities.get_mut(entity) {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
        }

        let _ = root.stage;

        if !root.completion_sent && root.elapsed >= total {
            commands.write(ShellSequenceCommand::ProgrammaticSegmentCompleted {
                activation_id: root.activation_id,
                segment_id: root.segment_id.clone(),
            });
            root.completion_sent = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The baked table has to be READABLE and internally consistent — every
    /// draw must name a part that exists, or the card silently loses limbs.
    #[test]
    fn every_baked_draw_names_a_part_that_exists() {
        let card = card();
        assert!(!card.frames.is_empty(), "the card must have frames");
        assert!(!card.parts.is_empty(), "the card must have parts");
        for (index, frame) in card.frames.iter().enumerate() {
            for draw in &frame.draws {
                assert!(
                    draw.part < card.parts.len(),
                    "frame {index} draws part {} of {}",
                    draw.part,
                    card.parts.len(),
                );
            }
        }
    }

    /// The card is a startup screen, so its length is a product fact rather than
    /// an implementation detail: long enough to read the joke, short enough that
    /// a player who has seen it is not stuck watching it.
    #[test]
    fn the_card_runs_for_a_plausible_startup_duration() {
        let total = made_this_meme_card_duration();
        assert!(
            total > Duration::from_secs(3) && total < Duration::from_secs(12),
            "unexpected vanity card length: {total:?}",
        );
    }

    /// Both speakers get their line, in the order the joke needs.
    #[test]
    fn the_robot_claims_it_first_and_the_author_claims_it_last() {
        let card = card();
        let lines: Vec<&str> = card
            .frames
            .iter()
            .filter_map(|frame| frame.bubble.as_ref())
            .map(|bubble| bubble.text.as_str())
            .collect();
        assert_eq!(
            lines.first().map(|line| line.to_lowercase()),
            Some("i made this.".to_string()),
        );
        assert!(
            lines
                .iter()
                .any(|line| line.to_lowercase().contains("you made this")),
            "the author has to ask: {lines:?}",
        );
        assert_eq!(
            lines.last().map(|line| line.to_lowercase()),
            Some("i made this.".to_string()),
        );
    }
}
