//! What a boss stages around it (`ambition.boss.stage`): the light of its
//! room, and its name card.
//!
//! The light is two veils over the camera's view while the camera is in the
//! boss's room: black for its dark, white for its flash. They lie over the
//! bodies and under the effects (`BOSS_STAGE_Z`), so sparks, lightning and
//! glowing eyes show through the dark. Compositing, not drawing: the same
//! with screen shaders off.
//!
//! The name card is the boss's name, large across a view, with its title
//! under it, at the alpha its encounter or conductor asks for. Each local view
//! shows the card of the boss in its own live room.

use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_platformer2d_core::config::WORLD_Z_FX;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, LiveRoomOf, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_sim_view::{BossRenderIndex, LocalView, ResolvedViewSubject, ViewPlacement};

/// Over the bodies, under the effects.
pub const BOSS_STAGE_Z: f32 = WORLD_Z_FX - 0.5;
/// The veils are this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;
/// The dark at its darkest: the room's shapes are only just there.
const DARKEST: f32 = 0.93;
/// The white of a full flash.
const BRIGHTEST: f32 = 0.75;

/// A veil of a boss's stage: its dark (`false`) or its flash (`true`).
#[derive(Component, Clone, Copy, Debug)]
pub struct BossStageVeil {
    flash: bool,
}

/// One name card's root node: the words it shows, and the local view it is
/// drawn over (`None`: the whole gameplay area, in a composition with no
/// local views).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BossNameCard {
    name: String,
    title: Option<String>,
    view: Option<Entity>,
}

/// Where a card sits: across its view, a little above the middle.
fn card_place(node: &mut Node, rect: Option<(Vec2, Vec2)>) {
    let (left, right, top, width) = match rect {
        Some((min, size)) => (Val::Px(min.x), Val::Auto, Val::Px(min.y + 0.3 * size.y), Val::Px(size.x)),
        None => (Val::Px(0.0), Val::Px(0.0), Val::Percent(30.0), Val::Auto),
    };
    if node.left != left {
        node.left = left;
    }
    if node.right != right {
        node.right = right;
    }
    if node.top != top {
        node.top = top;
    }
    if node.width != width {
        node.width = width;
    }
}


/// A part of the name card that follows how far it has come in.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossNameCardPart {
    Name,
    Rule,
    Title,
}

/// How each part of the card looks when it has come in `card` of the way.
fn name_text_color(card: f32) -> TextColor {
    TextColor(Color::srgba(0.97, 0.93, 0.86, card))
}
fn name_shadow(card: f32) -> TextShadow {
    TextShadow { offset: Vec2::new(0.0, 3.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85 * card) }
}
fn rule_width(card: f32) -> Val {
    Val::Px(420.0 * card)
}
fn rule_color(card: f32) -> BackgroundColor {
    BackgroundColor(Color::srgba(0.62, 0.86, 1.0, 0.8 * card))
}
fn title_text_color(card: f32) -> TextColor {
    TextColor(Color::srgba(0.70, 0.88, 1.0, card))
}
fn title_shadow(card: f32) -> TextShadow {
    TextShadow { offset: Vec2::new(0.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8 * card) }
}
/// It settles into place as it comes in.
fn card_rise(card: f32) -> UiRect {
    UiRect::top(Val::Px((1.0 - card) * 18.0))
}

/// The stage of the boss in the primary live room: the lowest light and the
/// brightest flash any boss there asks for.
fn stage_here(
    index: &BossRenderIndex,
    sprites: &Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: &LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: &PrimaryLiveRoom,
) -> Option<ambition_sim_view::BossStage> {
    let here = primary.get()?;
    sprites
        .iter()
        .filter(|(entity, _)| rooms.room_of(*entity) == Some(here))
        .filter_map(|(_, sprite)| index.get(&sprite.id))
        .map(|view| view.stage)
        .reduce(|a, b| ambition_sim_view::BossStage {
            lit: a.lit.min(b.lit),
            flash: a.flash.max(b.flash),
            card: a.card.max(b.card),
        })
}

/// Run condition of the passes that draw a boss's stage, card, finish and
/// bar: the boss read model holds a row, or held one last frame, the run
/// that puts each away. With no row each draws a lit, empty stage.
pub fn bosses_are_shown_or_were(index: Option<Res<BossRenderIndex>>, mut was: Local<bool>) -> bool {
    let now = index.is_some_and(|index| ambition_sim_view::RowView::has_rows(&*index));
    let run = now || *was;
    *was = now;
    run
}

/// Put the veils over the camera's view at the light the boss asks for.
pub fn light_the_boss_stage(
    mut commands: Commands,
    // `Option`: a composition with no sim read model has no boss to stage.
    index: Option<Res<BossRenderIndex>>,
    sprites: Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: PrimaryLiveRoom,
    cameras: Query<&Transform, (With<MainCamera>, Without<BossStageVeil>)>,
    mut veils: Query<(&BossStageVeil, &mut Transform, &mut Sprite)>,
) {
    let stage = index
        .as_deref()
        .and_then(|index| stage_here(index, &sprites, &rooms, &primary))
        .unwrap_or_default();
    let dark = Color::srgba(0.0, 0.0, 0.0, (1.0 - stage.lit).clamp(0.0, 1.0) * DARKEST);
    let flash = Color::srgba(0.86, 0.92, 1.0, stage.flash.clamp(0.0, 1.0) * BRIGHTEST);
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let at = camera.translation.truncate();
    if veils.is_empty() {
        for (is_flash, color) in [(false, dark), (true, flash)] {
            commands.spawn((
                BossStageVeil { flash: is_flash },
                Sprite { color, custom_size: Some(Vec2::splat(VEIL_SIZE)), ..Default::default() },
                Transform::from_translation(at.extend(BOSS_STAGE_Z + if is_flash { 0.05 } else { 0.0 })),
                // Apply camera-relative light once, over the portal windows.
                // Captures and carries must not treat it as world geometry.
                bevy::camera::visibility::RenderLayers::layer(
                    ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
                ),
                Name::new(if is_flash { "Boss stage: flash" } else { "Boss stage: dark" }),
            ));
        }
        return;
    }
    for (veil, mut transform, mut sprite) in &mut veils {
        if transform.translation.truncate() != at {
            transform.translation.x = at.x;
            transform.translation.y = at.y;
        }
        sprite.color = if veil.flash { flash } else { dark };
    }
}

/// Show, over each local view, the name card of the boss in that view's live
/// room while its card is up; with no local views, the primary live room's
/// over the whole gameplay area.
///
/// A card is kept while it shows the same boss over the same view: as it
/// comes in and goes, its colours, its rise and its rule are written in
/// place. Another boss, another view, or new fonts make a new card.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn show_boss_name_card(
    mut commands: Commands,
    mut cards: Query<(Entity, &BossNameCard, &mut Node, Option<&Children>)>,
    mut parts: Query<
        (
            &BossNameCardPart,
            Option<&mut TextColor>,
            Option<&mut TextShadow>,
            Option<&mut Node>,
            Option<&mut BackgroundColor>,
        ),
        Without<BossNameCard>,
    >,
    // A composition without the boss read model shows no card.
    index: Option<Res<BossRenderIndex>>,
    sprites: Query<(Entity, &super::primitives::FeatureVisual)>,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: PrimaryLiveRoom,
    views: Query<(Entity, &ResolvedViewSubject, Option<&ViewPlacement>), With<LocalView>>,
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    ui_fonts: Option<Res<UiFonts>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let session_scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    let card_in = |here: Option<LiveRoomInstance>| {
        let (Some(here), Some(index)) = (here, index.as_deref()) else {
            return None;
        };
        sprites
            .iter()
            .filter(|(entity, _)| rooms.room_of(*entity) == Some(here))
            .filter_map(|(_, sprite)| index.get(&sprite.id))
            .map(|view| (view, view.stage.card.clamp(0.0, 1.0)))
            .filter(|(_, card)| *card > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let rect_of = |placement: Option<&ViewPlacement>| {
        let gameplay = presentation.as_deref()?.gameplay_rect;
        Some(placement.copied().unwrap_or_default().carve(gameplay.min, gameplay.size()))
    };
    // Each card wanted on this frame: its words and view, how far it has
    // come in, and the rectangle of its view.
    let mut wanted: Vec<(BossNameCard, f32, Option<(Vec2, Vec2)>)> = Vec::new();
    if session_scope.is_some() {
        let mut want = |view: Option<Entity>, here, rect| {
            if let Some((boss, card)) = card_in(here) {
                wanted.push((
                    BossNameCard { name: boss.name.to_uppercase(), title: boss.title.clone(), view },
                    card,
                    rect,
                ));
            }
        };
        if views.is_empty() {
            want(None, primary.get(), None);
        } else {
            for (view, subject, placement) in &views {
                let here = subject.0.and_then(|body| rooms.room_of(body)).or_else(|| primary.get());
                want(Some(view), here, rect_of(placement));
            }
        }
    }
    let fonts_changed = ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed());
    let mut kept = vec![false; wanted.len()];
    for (entity, drawn, mut node, children) in &mut cards {
        let found = wanted.iter().position(|(want, ..)| want == drawn).filter(|at| !kept[*at] && !fonts_changed);
        let Some(at) = found else {
            commands.entity(entity).despawn();
            continue;
        };
        kept[at] = true;
        let (_, card, rect) = wanted[at];
        let rise = card_rise(card);
        if node.margin != rise {
            node.margin = rise;
        }
        card_place(&mut node, rect);
        for child in children.into_iter().flatten() {
            let Ok((part, text_color, shadow, node, background)) = parts.get_mut(*child) else {
                continue;
            };
            let (color, shade) = match part {
                BossNameCardPart::Name => (name_text_color(card), name_shadow(card)),
                BossNameCardPart::Title => (title_text_color(card), title_shadow(card)),
                BossNameCardPart::Rule => {
                    if let Some(mut node) = node {
                        if node.width != rule_width(card) {
                            node.width = rule_width(card);
                        }
                    }
                    if let Some(mut background) = background {
                        background.set_if_neq(rule_color(card));
                    }
                    continue;
                }
            };
            if let Some(mut text_color) = text_color {
                text_color.set_if_neq(color);
            }
            if let Some(mut shadow) = shadow {
                if shadow.color != shade.color {
                    shadow.color = shade.color;
                }
            }
        }
    }
    let Some(session_scope) = session_scope else {
        return;
    };
    let font = |size: f32, weight: UiFontWeight| {
        ui_fonts.as_deref().map(|fonts| fonts.text_font(size, weight)).unwrap_or(TextFont {
            font_size: FontSize::Px(size),
            ..default()
        })
    };
    for ((card_words, card, rect), kept) in wanted.into_iter().zip(kept) {
        if kept {
            continue;
        }
        let title = card_words.title.clone();
        let name = card_words.name.clone();
        let mut node = Node {
            position_type: PositionType::Absolute,
            margin: card_rise(card),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        };
        card_place(&mut node, rect);
        commands
            .spawn_session_scoped(session_scope, (node, ZIndex(36), Name::new("Boss name card"), card_words))
            .with_children(|root| {
                root.spawn((
                    BossNameCardPart::Name,
                    Text::new(name),
                    font(72.0, UiFontWeight::Semibold),
                    name_text_color(card),
                    name_shadow(card),
                ));
                // A rule under the name, as wide as the card has come in.
                root.spawn((
                    BossNameCardPart::Rule,
                    Node { width: rule_width(card), height: Val::Px(2.0), ..default() },
                    rule_color(card),
                ));
                if let Some(title) = title {
                    root.spawn((
                        BossNameCardPart::Title,
                        Text::new(title),
                        font(28.0, UiFontWeight::Regular),
                        title_text_color(card),
                        title_shadow(card),
                    ));
                }
            });
    }
}

#[cfg(test)]
mod name_card_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};

    #[test]
    fn stage_light_follows_the_view_without_entering_world_captures() {
        use ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER;
        use bevy::camera::visibility::RenderLayers;

        let mut app = App::new();
        let view_layers = RenderLayers::layer(0).with(PARALLAX_BACKGROUND_LAYER);
        let camera = app.world_mut().spawn((
            MainCamera,
            Transform::from_xyz(100.0, 200.0, 0.0),
            view_layers.clone(),
        )).id();
        app.add_systems(Update, light_the_boss_stage);
        for at in [Vec2::new(100.0, 200.0), Vec2::new(500.0, -300.0)] {
            app.world_mut().get_mut::<Transform>(camera).unwrap().translation = at.extend(0.0);
            app.update();
            let world = app.world_mut();
            let mut veils = world.query_filtered::<(&Transform, Option<&RenderLayers>), With<BossStageVeil>>();
            assert_eq!(veils.iter(world).count(), 2);
            for (transform, layers) in veils.iter(world) {
                let layers = layers.cloned().unwrap_or_default();
                assert_eq!(transform.translation.truncate(), at);
                assert!(layers.intersects(&view_layers));
                assert!(!layers.intersects(&RenderLayers::layer(0)), "view light must not enter portal captures or carries");
            }
        }
    }

    /// The name card is kept as it comes in: its words are not spawned and
    /// shaped again each frame, its colour follows how far it has come. When
    /// the conductor stops asking for it, it goes.
    #[test]
    fn the_name_card_is_kept_as_it_comes_in() {
        let geometry = ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "boss room",
            ambition_platformer2d_core::Vec2::new(800.0, 600.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ));
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry);
        app.world_mut()
            .spawn((super::super::primitives::FeatureVisual { id: "boss".to_string() }, InRoomInstance(LiveRoomInstance::ACTIVATION)));
        let index = |card: f32| {
            let mut view = ambition_sim_view::BossRenderView {
                name: "Smirking Behemoth".to_string(),
                behavior_id: "behemoth".to_string(),
                title: Some("of the rope".to_string()),
                stage: Default::default(),
                awake: true,
                fighting: true,
                finish: 0.0,
            };
            view.stage.card = card;
            BossRenderIndex::from_rows([("boss".to_string(), view)])
        };
        app.insert_resource(index(0.3));
        app.add_systems(Update, show_boss_name_card);
        app.update();
        let card = |app: &mut App| {
            app.world_mut().query_filtered::<Entity, With<BossNameCard>>().iter(app.world()).collect::<Vec<_>>()
        };
        let words = |app: &mut App| {
            app.world_mut()
                .query::<(Entity, &BossNameCardPart, &TextColor)>()
                .iter(app.world())
                .filter(|(_, part, _)| **part == BossNameCardPart::Name)
                .map(|(entity, _, color)| (entity, color.0.alpha()))
                .collect::<Vec<_>>()
        };
        let first = card(&mut app);
        let first_words = words(&mut app);
        assert_eq!(first.len(), 1, "one card");
        app.insert_resource(index(0.8));
        app.update();
        assert_eq!(card(&mut app), first, "the card coming in is the same card");
        let later = words(&mut app);
        assert_eq!(later[0].0, first_words[0].0, "and the same words");
        assert!(later[0].1 > first_words[0].1, "brighter as it comes in: {first_words:?} -> {later:?}");
        app.insert_resource(index(0.0));
        app.update();
        assert!(card(&mut app).is_empty(), "a card no longer asked for goes");
    }

    /// Each local view shows the card of the boss in its own live room: a
    /// second view in another boss's room shows that boss's card, and a view
    /// in a room whose boss shows none shows no card.
    #[test]
    fn each_view_shows_the_card_of_the_boss_in_its_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room;
        use ambition_sim_view::{LocalView, LocalViewId, ResolvedViewSubject, ViewPlacement};
        let geometry = |name: &str| {
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                name,
                ambition_platformer2d_core::Vec2::new(800.0, 600.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ))
        };
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        let first = LiveRoomInstance::ACTIVATION;
        let second = LiveRoomInstance::from_ordinal(1);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry("hall"));
        spawn_live_room(app.world_mut(), second, geometry("arena"));
        for (id, room) in [("warden", first), ("trex", second)] {
            app.world_mut()
                .spawn((super::super::primitives::FeatureVisual { id: id.to_string() }, InRoomInstance(room)));
        }
        let alice = app.world_mut().spawn(InRoomInstance(first)).id();
        let bob = app.world_mut().spawn(InRoomInstance(second)).id();
        let views = [(0, alice), (1, bob)].map(|(index, body)| {
            app.world_mut()
                .spawn((
                    LocalView,
                    LocalViewId(index as u8),
                    ViewPlacement::column(index, 2),
                    ResolvedViewSubject(Some(body)),
                ))
                .id()
        });
        let index = |cards: [(&str, f32); 2]| {
            BossRenderIndex::from_rows(cards.map(|(id, card)| {
                let mut view = ambition_sim_view::BossRenderView {
                    name: id.to_string(),
                    behavior_id: id.to_string(),
                    title: None,
                    stage: Default::default(),
                    awake: true,
                    fighting: true,
                    finish: 0.0,
                };
                view.stage.card = card;
                (id.to_string(), view)
            }))
        };
        app.add_systems(Update, show_boss_name_card);
        let shown = |app: &mut App| {
            let mut shown: Vec<(Option<Entity>, String)> = app
                .world_mut()
                .query::<&BossNameCard>()
                .iter(app.world())
                .map(|card| (card.view, card.name.clone()))
                .collect();
            shown.sort();
            shown
        };

        app.insert_resource(index([("warden", 0.0), ("trex", 0.6)]));
        app.update();
        assert_eq!(shown(&mut app), vec![(Some(views[1]), "TREX".to_string())], "only the second view's boss shows a card");

        app.insert_resource(index([("warden", 0.5), ("trex", 0.6)]));
        app.update();
        let mut both = vec![(Some(views[0]), "WARDEN".to_string()), (Some(views[1]), "TREX".to_string())];
        both.sort();
        assert_eq!(shown(&mut app), both, "each view shows its own room's boss");

        app.insert_resource(index([("warden", 0.5), ("trex", 0.0)]));
        app.update();
        assert_eq!(shown(&mut app), vec![(Some(views[0]), "WARDEN".to_string())], "the second view's card went with its boss's");
    }
}
