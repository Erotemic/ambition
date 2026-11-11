//! The card of an area: entering a room of another area shows that area's
//! name across the view, with its subtitle under it, the way a boss's name
//! card does, and then it fades.
//!
//! The areas are the game's ([`AreaCards`], published from its content). A
//! room in no area shows nothing and leaves the last area standing, so a
//! side room does not make its own area's card show again on the way back.
//! Each local view follows the room of its own subject.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_platformer2d_shared_tangle::body::PrimaryBody;
use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sim_view::{LocalView, ResolvedViewSubject, ViewPlacement};

/// What a card says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AreaCard {
    pub name: String,
    pub subtitle: Option<String>,
}

/// The game's areas, by the rooms each holds.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct AreaCards {
    /// Rooms named exactly.
    pub rooms: HashMap<String, AreaCard>,
    /// Rooms by the start of their id, the longest first.
    pub prefixes: Vec<(String, AreaCard)>,
}

impl AreaCards {
    /// The area a room id is in: the room named exactly, or else the longest
    /// prefix it starts with.
    pub fn of(&self, room: &str) -> Option<&AreaCard> {
        self.rooms.get(room).or_else(|| {
            self.prefixes
                .iter()
                .filter(|(prefix, _)| room.starts_with(prefix.as_str()))
                .max_by_key(|(prefix, _)| prefix.len())
                .map(|(_, card)| card)
        })
    }
}

/// How long the card takes to come in, stays, and takes to go, in seconds.
const FADE_IN_S: f32 = 0.6;
const HOLD_S: f32 = 2.4;
const FADE_OUT_S: f32 = 1.0;

/// How far the card has come in `age` seconds after its area was entered.
fn shown(age: f32) -> f32 {
    if age < FADE_IN_S {
        age / FADE_IN_S
    } else if age < FADE_IN_S + HOLD_S {
        1.0
    } else {
        (1.0 - (age - FADE_IN_S - HOLD_S) / FADE_OUT_S).max(0.0)
    }
}

/// One card's root node: the card, and the local view it is drawn over
/// (`None`: the whole gameplay area, in a composition with no local views).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AreaCardRoot {
    card: AreaCard,
    view: Option<Entity>,
}

/// A text of the card, which fades with it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaCardPart {
    Name,
    Rule,
    Subtitle,
}

fn part_color(part: AreaCardPart, alpha: f32) -> Color {
    match part {
        AreaCardPart::Name => Color::srgba(0.97, 0.94, 0.88, alpha),
        AreaCardPart::Rule => Color::srgba(0.97, 0.94, 0.88, 0.6 * alpha),
        AreaCardPart::Subtitle => Color::srgba(0.82, 0.86, 0.90, alpha),
    }
}

/// Where a card sits: across its view, in the upper part.
fn place(node: &mut Node, rect: Option<(Vec2, Vec2)>) {
    let (left, right, top, width) = match rect {
        Some((min, size)) => (Val::Px(min.x), Val::Auto, Val::Px(min.y + 0.18 * size.y), Val::Px(size.x)),
        None => (Val::Px(0.0), Val::Px(0.0), Val::Percent(18.0), Val::Auto),
    };
    node.left = left;
    node.right = right;
    node.top = top;
    node.width = width;
}

/// The area each view was last in, and how long ago it entered it.
#[derive(Default)]
pub struct AreaCardMemory(HashMap<Option<Entity>, (AreaCard, f32)>);

/// Show, over each local view, the card of the area its subject has just
/// entered.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn show_area_card(
    mut commands: Commands,
    mut memory: Local<AreaCardMemory>,
    cards: Option<Res<AreaCards>>,
    rooms: Option<LiveRoomSpecs>,
    primary: Query<Entity, With<PrimaryBody>>,
    views: Query<(Entity, &ResolvedViewSubject, Option<&ViewPlacement>), With<LocalView>>,
    mut drawn: Query<(Entity, &AreaCardRoot, &mut Node, Option<&Children>)>,
    mut parts: Query<(&AreaCardPart, Option<&mut TextColor>, Option<&mut BackgroundColor>), Without<AreaCardRoot>>,
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    ui_fonts: Option<Res<UiFonts>>,
    active_session: Option<Res<ActiveSessionScope>>,
    time: Res<Time>,
) {
    let (Some(cards), Some(rooms)) = (cards, rooms) else {
        return;
    };
    let rect_of = |placement: Option<&ViewPlacement>| {
        let gameplay = presentation.as_deref()?.gameplay_rect;
        Some(placement.copied().unwrap_or_default().carve(gameplay.min, gameplay.size()))
    };
    // Each view, the body it follows, and where it is drawn.
    let mut subjects: Vec<(Option<Entity>, Option<Entity>, Option<(Vec2, Vec2)>)> = Vec::new();
    if views.is_empty() {
        subjects.push((None, primary.single().ok(), None));
    } else {
        for (view, subject, placement) in &views {
            subjects.push((Some(view), subject.0.or_else(|| primary.single().ok()), rect_of(placement)));
        }
    }
    let dt = time.delta_secs();
    let mut wanted: Vec<(AreaCardRoot, f32, Option<(Vec2, Vec2)>)> = Vec::new();
    for (view, body, rect) in subjects {
        let here = body.and_then(|body| rooms.spec_of(body)).and_then(|spec| cards.of(&spec.id));
        let entry = memory.0.entry(view);
        let (card, age) = match (entry, here) {
            (std::collections::hash_map::Entry::Occupied(mut last), Some(here)) if last.get().0 != *here => {
                last.insert((here.clone(), 0.0));
                last.into_mut()
            }
            (std::collections::hash_map::Entry::Vacant(empty), Some(here)) => empty.insert((here.clone(), 0.0)),
            (std::collections::hash_map::Entry::Occupied(last), _) => {
                let last = last.into_mut();
                last.1 += dt;
                last
            }
            (std::collections::hash_map::Entry::Vacant(_), None) => continue,
        };
        let alpha = shown(*age);
        if alpha > 0.0 {
            wanted.push((AreaCardRoot { card: card.clone(), view }, alpha, rect));
        }
    }
    let fonts_changed = ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed());
    let mut kept = vec![false; wanted.len()];
    for (entity, root, mut node, children) in &mut drawn {
        let found = wanted.iter().position(|(want, ..)| want == root).filter(|at| !kept[*at] && !fonts_changed);
        let Some(at) = found else {
            commands.entity(entity).despawn();
            continue;
        };
        kept[at] = true;
        let (_, alpha, rect) = &wanted[at];
        place(&mut node, *rect);
        for child in children.into_iter().flatten() {
            let Ok((part, text, background)) = parts.get_mut(*child) else {
                continue;
            };
            if let Some(mut text) = text {
                text.set_if_neq(TextColor(part_color(*part, *alpha)));
            }
            // Only the rule is a filled box; a text's own background stays clear.
            if let (AreaCardPart::Rule, Some(mut background)) = (part, background) {
                background.set_if_neq(BackgroundColor(part_color(*part, *alpha)));
            }
        }
    }
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    let font = |size: f32, weight: UiFontWeight| {
        ui_fonts.as_deref().map(|fonts| fonts.text_font(size, weight)).unwrap_or(TextFont {
            font_size: FontSize::Px(size),
            ..default()
        })
    };
    for ((root, alpha, rect), kept) in wanted.into_iter().zip(kept) {
        if kept {
            continue;
        }
        let name = root.card.name.to_uppercase();
        let subtitle = root.card.subtitle.clone();
        let mut node = Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        };
        place(&mut node, rect);
        let shadow = TextShadow { offset: Vec2::new(0.0, 3.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) };
        commands
            .spawn_session_scoped(session_scope, (node, ZIndex(35), Name::new("Area card"), root))
            .with_children(|card| {
                card.spawn((
                    AreaCardPart::Name,
                    Text::new(name),
                    font(52.0, UiFontWeight::Semibold),
                    TextColor(part_color(AreaCardPart::Name, alpha)),
                    shadow,
                ));
                card.spawn((
                    AreaCardPart::Rule,
                    Node { width: Val::Px(320.0), height: Val::Px(2.0), ..default() },
                    BackgroundColor(part_color(AreaCardPart::Rule, alpha)),
                ));
                if let Some(subtitle) = subtitle {
                    card.spawn((
                        AreaCardPart::Subtitle,
                        Text::new(subtitle),
                        font(24.0, UiFontWeight::Regular),
                        TextColor(part_color(AreaCardPart::Subtitle, alpha)),
                        shadow,
                    ));
                }
            });
    }
}
