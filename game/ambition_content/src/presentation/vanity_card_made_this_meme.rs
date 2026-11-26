//! Startup comic that plays the published character poses on a UI stage.

use std::{collections::hash_map::RandomState, hash::BuildHasher, time::Duration};

use ambition_platformer2d::game_shell::{
    ActiveShellSequence, FrontendOwnedEntity, FrontendPresentationKind, ShellSegmentScopedEntity,
    ShellSequenceSet,
};
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use ambition_sprite_sheet::{
    SpriteContent,
    character::rigged::{PartDraw, RiggedSpriteAsset, blend_from},
    game_assets::load_sheet_image,
};
use bevy::prelude::*;

pub const MADE_THIS_MEME_CARD_SEGMENT_KIND: &str = "ambition_vanity_card_made_this_meme";

const CANVAS: Vec2 = Vec2::new(640.0, 360.0);
const FLOOR: f32 = 320.0;
const ARRIVE: f32 = 1.25;
const ROBOT_LINE: f32 = 1.48;
const QUESTION: f32 = 2.90;
const REACH: f32 = 3.80;
const TAKE: f32 = 4.32;
const RELEASE: f32 = 4.62;
const RETREAT_END: f32 = 5.80;
const DIRECTOR_LINE: f32 = 6.50;

pub fn made_this_meme_card_duration() -> Duration {
    Duration::from_millis(8_500)
}

pub struct MadeThisMemeCardPlugin;

impl Plugin for MadeThisMemeCardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_vanity_card, animate_vanity_card, fit_card_to_display)
                .chain()
                .in_set(ShellSequenceSet::Programmatic)
                .run_if(resource_exists::<ActiveShellSequence>)
                .run_if(card_is_active),
        );
    }
}

fn card_is_active(active: Res<ActiveShellSequence>) -> bool {
    active
        .registered_segment()
        .is_some_and(|(_, _, kind)| kind.as_str() == MADE_THIS_MEME_CARD_SEGMENT_KIND)
}

#[derive(Clone, Copy)]
enum Gift {
    Gamepad,
    ApplePie,
}

impl Gift {
    fn asset(self) -> (&'static str, Vec2) {
        match self {
            Self::Gamepad => (
                "content://ambition_shared/sprites/gamepad.png",
                Vec2::new(72.0, 35.44),
            ),
            Self::ApplePie => (
                "content://ambition_shared/sprites/apple_pie.png",
                Vec2::new(74.0, 41.11),
            ),
        }
    }
}

struct CardActor {
    rig: RiggedSpriteAsset,
    pages: Vec<Handle<Image>>,
    slots: Vec<Entity>,
    draws: Vec<PartDraw>,
    scratch: Vec<PartDraw>,
    scale: f32,
    feet: Vec2,
}

impl CardActor {
    fn new(
        rig: RiggedSpriteAsset,
        content: &SpriteContent,
        assets: &AssetServer,
        scale: f32,
    ) -> Self {
        let pages = rig
            .pages
            .iter()
            .map(|page| load_sheet_image(Some(content), assets, "vanity-card", page.clone()))
            .collect();
        let capacity = rig.max_draws();
        Self {
            rig,
            pages,
            slots: Vec::with_capacity(capacity),
            draws: Vec::with_capacity(capacity),
            scratch: Vec::with_capacity(capacity),
            scale,
            feet: Vec2::ZERO,
        }
    }

    fn sample(&mut self, row: &str, elapsed: f32) {
        if let Some((frame, phase)) = clip_phase(&self.rig, row, elapsed) {
            self.rig.tween_into(row, frame, phase, &mut self.draws);
        } else {
            self.draws.clear();
        }
    }

    fn sample_frame(&mut self, row: &str, frame: usize) {
        self.draws.clear();
        if let Some(draws) = self.rig.frame(row, frame) {
            self.draws.extend_from_slice(draws);
        }
    }

    fn blend(&mut self, row: &str, frame: usize, fraction: f32) {
        if let Some(from) = self.rig.frame(row, frame) {
            blend_from(from, fraction, &mut self.draws);
        }
    }

    fn carry_walk(&mut self, elapsed: f32) {
        self.sample("item_hold", elapsed);
        if let Some((frame, phase)) = clip_phase(&self.rig, "walk", elapsed) {
            self.rig
                .legs_under("walk", frame, phase, &mut self.draws, &mut self.scratch);
        }
    }

    fn track(&self, name: &str) -> Option<&PartDraw> {
        self.draws.iter().find(|draw| {
            draw.track
                .is_some_and(|track| self.rig.tracks()[usize::from(track)] == name)
        })
    }

    fn point(&self, track: &str) -> Vec2 {
        self.feet
            + self
                .track(track)
                .map_or(Vec2::ZERO, |draw| draw.at * self.scale)
    }

    fn tilt_head(&mut self, radians: f32) {
        let Some(head) = self.track("head").copied() else {
            return;
        };
        let turn = Rot2::radians(radians);
        for draw in &mut self.draws {
            if draw.track.is_some_and(|track| {
                ambition_sprite_sheet::character::rigged::is_head_track(
                    &self.rig.tracks()[usize::from(track)],
                )
            }) {
                draw.at = head.at + turn * (draw.at - head.at);
                draw.rotation += radians;
            }
        }
    }
}

fn clip_phase(rig: &RiggedSpriteAsset, row: &str, elapsed: f32) -> Option<(usize, f32)> {
    let clip = rig.clip(row)?;
    let frame = (elapsed / clip.frame_duration_s).rem_euclid(clip.frame_count() as f32);
    Some((frame.floor() as usize, frame.fract()))
}

#[derive(Component)]
struct VanityCardRoot {
    robot: CardActor,
    director: CardActor,
    gift: Entity,
    gift_size: Vec2,
    bubble: Entity,
    bubble_text: Entity,
    bubble_tail: Entity,
}

#[derive(Component)]
struct VanityCardViewport;

#[derive(Component)]
struct VanityCardStage;

const ROBOT: &str = "content://ambition_shared/sprites/player_robot_v3_closeup";
const DIRECTOR: &str = "content://ambition_shared/sprites/director_closeup";

fn spawn_vanity_card(
    mut commands: Commands,
    active: Res<ActiveShellSequence>,
    roots: Query<(), With<VanityCardRoot>>,
    assets: Res<AssetServer>,
    content: Option<Res<SpriteContent>>,
    fonts: Option<Res<UiFonts>>,
) {
    if !roots.is_empty() {
        return;
    }
    let Some((activation_id, segment_id, _)) = active.registered_segment() else {
        return;
    };
    let Some(content) = content.as_deref() else {
        return;
    };
    let (Some(robot), Some(director)) = (
        RiggedSpriteAsset::baked(content, ROBOT),
        RiggedSpriteAsset::baked(content, DIRECTOR),
    ) else {
        bevy::log::warn_once!("Publish startup_character_parts to draw the vanity card.");
        return;
    };
    let mut robot = CardActor::new(robot, content, &assets, 2.00);
    let mut director = CardActor::new(director, content, &assets, 1.94);

    let choice = if RandomState::new().hash_one(activation_id.0) & 1 == 0 {
        Gift::Gamepad
    } else {
        Gift::ApplePie
    };
    let (gift_path, gift_size) = choice.asset();
    let gift_image = load_sheet_image(Some(content), &assets, "vanity-card", gift_path);

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
            BackgroundColor(Color::srgb_u8(12, 14, 22)),
            GlobalZIndex(4_500),
            FrontendOwnedEntity::shell(activation_id, FrontendPresentationKind::StartupRoot),
            ShellSegmentScopedEntity {
                activation_id,
                segment_id: segment_id.clone(),
            },
            Name::new("Ambition Vanity Card Root"),
        ))
        .id();
    let viewport = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            VanityCardViewport,
            ChildOf(root),
        ))
        .id();
    let stage = commands
        .spawn((
            Node {
                width: Val::Px(CANVAS.x),
                height: Val::Px(CANVAS.y),
                flex_shrink: 0.0,
                ..default()
            },
            VanityCardStage,
            ChildOf(viewport),
        ))
        .id();
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Px(8.0),
            width: Val::Px(CANVAS.x - 16.0),
            height: Val::Px(CANVAS.y - 16.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(22.0)),
            ..default()
        },
        BackgroundColor(Color::srgb_u8(246, 247, 251)),
        BorderColor::all(Color::srgb_u8(210, 216, 228)),
        ChildOf(stage),
    ));
    for actor in [&mut robot, &mut director] {
        for _ in 0..actor.rig.max_draws() {
            actor.slots.push(
                commands
                    .spawn((
                        absolute_node(),
                        ImageNode::default().with_mode(NodeImageMode::Stretch),
                        Visibility::Hidden,
                        ChildOf(stage),
                    ))
                    .id(),
            );
        }
    }
    let gift = commands
        .spawn((absolute_node(), ImageNode::new(gift_image), ChildOf(stage)))
        .id();
    let bubble_tail = commands
        .spawn((
            absolute_node(),
            BackgroundColor(Color::srgb_u8(168, 178, 198)),
            Visibility::Hidden,
            ChildOf(stage),
        ))
        .id();
    let bubble = commands
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(13.0), Val::Px(9.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(13.0)),
                ..absolute_node()
            },
            BackgroundColor(Color::WHITE),
            BorderColor::all(Color::srgb_u8(168, 178, 198)),
            Visibility::Hidden,
            ChildOf(stage),
        ))
        .id();
    let bubble_text = commands
        .spawn((
            Text::new(""),
            fonts.as_deref().map_or_else(
                || TextFont::from_font_size(18.0),
                |fonts| fonts.text_font(18.0, UiFontWeight::Regular),
            ),
            TextColor(Color::srgb_u8(32, 36, 46)),
            ChildOf(bubble),
        ))
        .id();
    commands.entity(root).insert(VanityCardRoot {
        robot,
        director,
        gift,
        gift_size,
        bubble,
        bubble_text,
        bubble_tail,
    });
}

fn absolute_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

fn fit_card_to_display(
    viewports: Query<(&ComputedNode, &Children), With<VanityCardViewport>>,
    mut stages: Query<&mut UiTransform, With<VanityCardStage>>,
) {
    for (computed, children) in &viewports {
        // UI positions use logical pixels. The viewport size uses physical pixels.
        let size = computed.size() * computed.inverse_scale_factor();
        if size.x > 0.0 && size.y > 0.0 {
            let scale = (size / CANVAS).min_element();
            for child in children.iter() {
                if let Ok(mut transform) = stages.get_mut(child) {
                    transform.scale = Vec2::splat(scale);
                }
            }
        }
    }
}

fn ease(t: f32, start: f32, end: f32) -> f32 {
    let phase = ((t - start) / (end - start)).clamp(0.0, 1.0);
    phase * phase * (3.0 - 2.0 * phase)
}

fn pose_robot(robot: &mut CardActor, t: f32) {
    let approach = ease(t, 0.0, ARRIVE);
    let retreat = ease(t, RELEASE, RETREAT_END);
    robot.feet = Vec2::new(83.0 + 216.0 * approach - 94.0 * retreat, FLOOR);
    let gait = robot.rig.clip("walk").map_or(1.0, |clip| {
        clip.frame_duration_s * clip.frame_count() as f32
    });

    if t < ARRIVE {
        robot.carry_walk(gait * 3.0 * approach);
    } else if t < RELEASE {
        robot.sample("grab_hold", t - ARRIVE);
        robot.blend("item_hold", 0, ease(t, ARRIVE, ROBOT_LINE + 0.14));
    } else if t < RETREAT_END {
        robot.sample("walk", -gait * 1.5 * retreat);
        robot.blend("grab_hold", 0, ease(t, RELEASE, RELEASE + 0.29));
    } else {
        robot.sample("idle", t - RETREAT_END);
        robot.blend("walk", 4, ease(t, RETREAT_END, RETREAT_END + 0.17));
        robot.tilt_head(0.16 * ease(t, DIRECTOR_LINE + 0.3, DIRECTOR_LINE + 0.8));
    }
}

fn pose_director(director: &mut CardActor, t: f32) {
    director.feet = Vec2::new(458.0, FLOOR);
    if t < QUESTION {
        director.sample("idle", t);
    } else if t < REACH {
        director.sample("talk", t - QUESTION);
        let gesture = ease(t, QUESTION, QUESTION + 0.24) - ease(t, REACH - 0.22, REACH);
        director.blend("idle", 0, gesture);
    } else if t < RELEASE {
        director.sample_frame("receive_low", 0);
        director.blend("idle", 0, ease(t, REACH, TAKE));
    } else {
        director.sample("item_hold", t - RELEASE);
        director.blend("receive_low", 0, ease(t, RELEASE, RELEASE + 0.68));
        director.tilt_head(-0.29 * ease(t, RELEASE + 0.3, RETREAT_END + 0.35));
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct CardNodes<'w, 's> {
    nodes: Query<'w, 's, &'static mut Node>,
    transforms: Query<'w, 's, &'static mut UiTransform>,
    images: Query<'w, 's, &'static mut ImageNode>,
    visibility: Query<'w, 's, &'static mut Visibility>,
}

impl CardNodes<'_, '_> {
    fn place(&mut self, entity: Entity, center: Vec2, size: Vec2, rotation: Rot2) {
        if let Ok(mut node) = self.nodes.get_mut(entity) {
            node.left = Val::Px(center.x - size.x / 2.0);
            node.top = Val::Px(center.y - size.y / 2.0);
            node.width = Val::Px(size.x);
            node.height = Val::Px(size.y);
        }
        if let Ok(mut transform) = self.transforms.get_mut(entity) {
            transform.rotation = rotation;
        }
    }

    fn show(&mut self, entity: Entity, visible: bool) {
        if let Ok(mut visibility) = self.visibility.get_mut(entity) {
            *visibility = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    fn draw_actor(&mut self, actor: &CardActor) {
        let draws = actor.draws.iter().filter(|draw| {
            // Keep the Director's pen out of the hand that takes the gift.
            draw.track
                .is_none_or(|track| actor.rig.tracks()[usize::from(track)] != "pen")
        });
        let mut slots = actor.slots.iter().copied();
        for (draw, slot) in draws.zip(slots.by_ref()) {
            let part = actor.rig.parts[usize::from(draw.part)];
            let rotation = Rot2::radians(draw.rotation);
            let scale = draw.scale * actor.scale;
            let center = actor.feet
                + draw.at * actor.scale
                + rotation * ((part.size / 2.0 - part.pivot) * scale);
            self.place(slot, center, part.size * scale.abs(), rotation);
            if let Ok(mut image) = self.images.get_mut(slot) {
                let tint = draw.tint();
                image.image = actor.pages[usize::from(part.page)].clone();
                image.rect = Some(Rect::new(
                    part.rect.min.x as f32,
                    part.rect.min.y as f32,
                    part.rect.max.x as f32,
                    part.rect.max.y as f32,
                ));
                image.flip_x = scale.x < 0.0;
                image.flip_y = scale.y < 0.0;
                image.color = Color::srgba(tint.x, tint.y, tint.z, draw.opacity());
            }
            self.show(slot, true);
        }
        for unused in slots {
            self.show(unused, false);
        }
    }
}

fn animate_vanity_card(
    active: Res<ActiveShellSequence>,
    mut roots: Query<&mut VanityCardRoot>,
    mut ui: CardNodes,
    mut texts: Query<&mut Text>,
) {
    let Some(sequence) = active.runtime.as_ref() else {
        return;
    };
    let t = sequence.elapsed.as_secs_f32();
    for mut root in &mut roots {
        pose_robot(&mut root.robot, t);
        pose_director(&mut root.director, t);
        ui.draw_actor(&root.robot);
        ui.draw_actor(&root.director);

        let transfer = ease(t, TAKE, RELEASE);
        let from = root.robot.point("far_hand") + Vec2::new(22.0, -9.0);
        let to = root.director.point("far_hand") + Vec2::new(-17.0, 5.0);
        ui.place(
            root.gift,
            from.lerp(to, transfer),
            root.gift_size,
            Rot2::radians(-0.10 + 0.22 * transfer),
        );

        let speech = if (ROBOT_LINE..QUESTION).contains(&t) {
            Some((
                "I made this.",
                Vec2::new(158.0, 50.0),
                Vec2::new(88.0, 32.0),
                root.robot.point("head") + Vec2::new(-28.0, -58.0),
            ))
        } else {
            let line = if (QUESTION..TAKE).contains(&t) {
                Some("You made this?")
            } else {
                (t >= DIRECTOR_LINE).then_some("I made this.")
            };
            line.map(|line| {
                (
                    line,
                    Vec2::new(421.0, 30.0),
                    Vec2::new(30.0, 32.0),
                    root.director.point("head") - Vec2::new(0.0, 56.0),
                )
            })
        };
        ui.show(root.bubble, speech.is_some());
        ui.show(root.bubble_tail, speech.is_some());
        if let Some((line, position, tail_base, tip)) = speech {
            if let Ok(mut text) = texts.get_mut(root.bubble_text) {
                if text.0 != line {
                    text.0 = line.to_owned();
                }
            }
            if let Ok(mut node) = ui.nodes.get_mut(root.bubble) {
                node.left = Val::Px(position.x);
                node.top = Val::Px(position.y);
            }
            let base = position + tail_base;
            let tail = tip - base;
            ui.place(
                root.bubble_tail,
                base + tail / 2.0,
                Vec2::new(tail.length(), 2.0),
                Rot2::radians(tail.y.atan2(tail.x)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::game_shell::{
        ShellActivationId, ShellSegmentRole, ShellSegmentSpec, ShellSequenceRuntime,
        ShellSequenceSpec,
    };

    #[test]
    fn unrelated_routes_do_not_require_vanity_card_assets() {
        for segment in [
            None,
            Some(ShellSegmentSpec::text("text", "A text card")),
            Some(ShellSegmentSpec::registered(
                "other",
                ShellSegmentRole::Vanity,
                "another_card",
            )),
        ] {
            let mut app = App::new();
            app.insert_resource(ActiveShellSequence {
                activation_id: Some(ShellActivationId(1)),
                runtime: segment.map(|segment| {
                    ShellSequenceRuntime::new(ShellSequenceSpec {
                        segments: vec![segment],
                    })
                }),
            });
            app.add_plugins(MadeThisMemeCardPlugin);
            app.update();
        }
    }
}
