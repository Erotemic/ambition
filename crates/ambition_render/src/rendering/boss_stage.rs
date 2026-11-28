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
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, LiveRoomOf, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_sim_view::{BossRenderIndex, LocalView, PresentedForView, ResolvedViewSubject, ViewPlacement};

use super::camera::coverage::GameplayOutputs;
use super::retained_entities::RetainedEntities;
use super::view_isolation::lighting_layers;
use super::PresentedViewFrames;

/// Over the bodies, under the effects.
pub const BOSS_STAGE_Z: f32 = WORLD_Z_FX - 0.5;
/// The veils are this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;
/// The dark at its darkest: the room's shapes are only just there.
const DARKEST: f32 = 0.93;
/// The white of a full flash.
const BRIGHTEST: f32 = 0.75;

/// A veil for a boss's stage light.
#[derive(Component, Clone, Copy, Debug)]
#[require(super::primitives::AnimatedWorldSpriteTint)]
pub struct BossStageVeil;

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

/// Run condition of the passes that draw a boss's stage, card, finish and
/// bar: the boss read model holds a row, or held one last frame, the run
/// that puts each away. With no row each draws a lit, empty stage.
pub fn bosses_are_shown_or_were(index: Option<Res<BossRenderIndex>>, mut was: Local<bool>) -> bool {
    let now = index.is_some_and(|index| ambition_sim_view::RowView::has_rows(&*index));
    let run = now || *was;
    *was = now;
    run
}

/// Draw the stage lights from the room shown by each gameplay view.
pub(crate) fn light_the_boss_stage(
    mut commands: Commands,
    index: Option<Res<BossRenderIndex>>,
    outputs: GameplayOutputs,
    mut veils: Query<(&mut Transform, &mut Sprite), (With<BossStageVeil>, Without<Camera2d>)>,
    mut retained: Local<RetainedEntities<(Entity, bool)>>,
) {
    let mut wanted = std::collections::BTreeMap::new();
    if let Some(index) = index.as_deref() {
        for (view, (frame, _)) in outputs.view_bounds() {
            let (lit, flash) = index.iter()
                .filter(|(_, boss)| boss.room == Some(frame.room))
                .fold((1.0f32, 0.0f32), |(lit, flash), (_, boss)| {
                    (lit.min(boss.stage.lit), flash.max(boss.stage.flash))
                });
            let dark = Color::srgba(0.0, 0.0, 0.0, (1.0 - lit).clamp(0.0, 1.0) * DARKEST);
            let flash = Color::srgba(0.86, 0.92, 1.0, flash.clamp(0.0, 1.0) * BRIGHTEST);
            for (is_flash, color) in [(false, dark), (true, flash)] {
                if color.alpha() > 0.0 {
                    wanted.insert((view, is_flash), (frame.plane_center, color));
                }
            }
        }
    }

    retained.reconcile(&mut commands, |key| wanted.contains_key(key));
    for ((view, is_flash), (center, color)) in wanted {
        let at = center.extend(BOSS_STAGE_Z + if is_flash { 0.05 } else { 0.0 });
        let key = (view, is_flash);
        if let Some((mut transform, mut sprite)) = retained.get(&key).and_then(|entity| veils.get_mut(*entity).ok()) {
            if transform.translation != at {
                transform.translation = at;
            }
            if sprite.color != color {
                sprite.color = color;
            }
        } else {
            let entity = commands.spawn((
                BossStageVeil,
                PresentedForView(view),
                lighting_layers(),
                Sprite::from_color(color, Vec2::splat(VEIL_SIZE)),
                Transform::from_translation(at),
                Name::new(if is_flash { "Boss stage: flash" } else { "Boss stage: dark" }),
            )).id();
            retained.insert(key, entity);
        }
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
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    primary: PrimaryLiveRoom,
    frames: Option<Res<PresentedViewFrames>>,
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
        index.iter()
            .filter(|(_, boss)| boss.room == Some(here))
            .map(|(id, boss)| (id, boss, boss.stage.card.clamp(0.0, 1.0)))
            .filter(|(_, _, card)| *card > 0.0)
            .max_by(|a, b| a.2.total_cmp(&b.2).then_with(|| b.0.cmp(a.0)))
            .map(|(_, boss, card)| (boss, card))
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
                let here = frames.as_deref().and_then(|frames| frames.of(view))
                    .map(|frame| frame.room)
                    .or_else(|| subject.0.and_then(|body| rooms.room_of(body)));
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
    fn stage_lights_follow_the_room_each_view_frames() {
        use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, PARALLAX_BACKGROUND_LAYER};
        use ambition_sim_view::{LocalViewId, PresentsView};
        use bevy::camera::{visibility::RenderLayers, RenderTargetInfo};
        use crate::rendering::PresentedViewFrame;

        let mut app = App::new();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO));
        app.add_systems(Update, (light_the_boss_stage, super::super::view_isolation::isolate_per_view_projections).chain());
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let room_rows = [(first, 0.6, 0.0), (second, 1.0, 0.3)];
        let index = BossRenderIndex::from_rows(room_rows.map(|(room, lit, flash)| (
            "same_boss".into(),
            ambition_sim_view::BossRenderView {
                room: Some(room),
                name: "Sentinel".into(),
                behavior_id: "sentinel".into(),
                title: None,
                stage: ambition_sim_view::BossStage { lit, flash, card: 0.0 },
                awake: true,
                fighting: true,
                finish: 0.0,
            },
        )));
        assert_eq!(index.len(), 2);
        app.insert_resource(index);

        let world = app.world_mut();
        let placements = [(0, first, Vec2::new(100.0, 200.0)), (1, second, Vec2::new(600.0, 200.0))];
        let outputs = placements.map(|(id, room, at)| {
            let view = world.spawn((LocalView, LocalViewId(id))).id();
            let mut camera = Camera::default();
            camera.computed.target_info = Some(RenderTargetInfo { physical_size: UVec2::new(640, 360), scale_factor: 1.0 });
            let output = world.spawn((MainCamera, Camera2d, camera, PresentsView(view),
                RenderLayers::layer(0).with(PARALLAX_BACKGROUND_LAYER))).id();
            let frame = PresentedViewFrame { room, plane_center: at, roll_radians: 0.0, orthographic_scale: 1.0 };
            (view, output, frame)
        });
        let frames = outputs.map(|(view, _, frame)| (view, frame));
        world.insert_resource(PresentedViewFrames::of_views(frames.to_vec()));

        app.update();
        let world = app.world_mut();
        let mut veils = world.query_filtered::<(Entity, &PresentedForView, &Transform, &Sprite, &RenderLayers), With<BossStageVeil>>();
        let mut kept = Vec::new();
        for (entity, owner, transform, sprite, layers) in veils.iter(world) {
            let slot = outputs.iter().position(|(view, _, _)| owner.0 == *view).unwrap();
            let (_, camera, frame) = outputs[slot];
            let other_camera = outputs[1 - slot].1;
            assert_eq!(transform.translation.truncate(), frame.plane_center);
            assert!(layers.intersects(world.get::<RenderLayers>(camera).unwrap()));
            assert!(!layers.intersects(world.get::<RenderLayers>(other_camera).unwrap()));
            assert!(!layers.intersects(&RenderLayers::layer(0)));
            let (_, lit, flash) = room_rows[slot];
            let alpha = if flash > 0.0 { flash * BRIGHTEST } else { (1.0 - lit) * DARKEST };
            assert_eq!(sprite.color.alpha(), alpha);
            kept.push((owner.0, entity));
        }
        assert_eq!(kept.len(), 2);

        let [(leaving, ..), (staying, ..)] = outputs;
        let frame = world.resource_mut::<PresentedViewFrames>().into_inner().of_mut(leaving).unwrap();
        frame.room = second.next();
        app.update();
        let world = app.world_mut();
        let remaining = veils.iter(world).map(|(entity, owner, ..)| (owner.0, entity)).collect::<Vec<_>>();
        assert_eq!(remaining, kept.into_iter().filter(|(view, _)| *view == staying).collect::<Vec<_>>());
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
        let index = |card: f32| {
            let mut view = ambition_sim_view::BossRenderView {
                room: Some(LiveRoomInstance::ACTIVATION),
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
                    room: Some(if id == "warden" { first } else { second }),
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
