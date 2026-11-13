//! The light about an actor ([`ActorGlow`]): one tinted sprite of a soft
//! round picture for each glowing body, at the body's drawn place.
//!
//! Cost: one more sprite for each glowing body. It uses the sprite pipeline
//! and batches with the other sprites. It needs no shader, no material and no
//! render target, so every quality tier draws it.
//!
//! The renderer builds a white radial mask once. It needs no content library.
//! The sprite colour gives the light its hue and strength.
//!
//! The glow is a top-level sprite that names its body (`PresentationOf`), like
//! the hit flash. So the shared roads apply with no opt-in: a portal pane
//! clips it, a portal pair carries it, a hide by a portal is settled for it,
//! and it is drawn only in the views of its body's room.
//!
//! [`grant_player_glows`] puts the active route's glow ([`ActivePlayerGlow`])
//! on each body a player drives (`PlayerVisual`). A glow that a game put on a
//! body itself is not changed by it.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ActivePlayerGlow, ActorGlow, GlowLayer, GlowLook,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    PlayerVisual, PresentationOf, SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// How far behind its body a [`GlowLayer::Behind`] glow is: less than the
/// depth between two layers of a room, so nothing comes between them.
const BEHIND_DEPTH: f32 = 0.06;
/// How far over its body a [`GlowLayer::Over`] glow is: in the body's own
/// band, under the hit flash (1.5), so a strike still reads.
const OVER_BIAS: f32 = 0.5;
const _: () = assert!(OVER_BIAS < super::BODY_DEPTH_BAND);

/// The app's glow mask. Insert this resource before startup to use another image.
#[derive(Resource, Clone, Debug)]
pub struct ActorGlowPicture(pub Handle<Image>);

impl FromWorld for ActorGlowPicture {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource_mut::<Assets<Image>>().add(soft_glow_image()))
    }
}

fn soft_glow_image() -> Image {
    const SIDE: u32 = 128;
    let center = (SIDE - 1) as f32 * 0.5;
    let radius = center - 1.0;
    let falloff = 3.2_f32;
    let floor = (-falloff).exp();
    let mut data = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let distance_squared = ((x as f32 - center).powi(2)
                + (y as f32 - center).powi(2)) / radius.powi(2);
            let alpha = (((-falloff * distance_squared).exp() - floor)
                / (1.0 - floor)).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (255.0 * alpha).round() as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// Startup: create the app's mask unless the host supplied one.
pub fn build_actor_glow_picture(mut commands: Commands) {
    commands.init_resource::<ActorGlowPicture>();
}

/// On a body's visual: the route's player glow put the [`ActorGlow`] there,
/// so [`grant_player_glows`] may change it or take it away.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerGlowGrant;

/// The sprite that draws one body's glow.
#[derive(Component, Clone, Copy, Debug)]
pub struct ActorGlowVisual;

/// Run condition: a glow is asked for, or one is drawn and may go.
pub fn glows_are_wanted_or_drawn(
    active: Option<Res<ActivePlayerGlow>>,
    glowing: Query<(), Or<(With<ActorGlow>, With<PlayerGlowGrant>)>>,
    drawn: Query<(), With<ActorGlowVisual>>,
) -> bool {
    active.is_some_and(|active| active.0.is_some()) || !glowing.is_empty() || !drawn.is_empty()
}

/// Give the active route's player glow to each body a player drives, and
/// take it away when the route has none.
pub fn grant_player_glows(
    mut commands: Commands,
    active: Option<Res<ActivePlayerGlow>>,
    players: Query<(Entity, Option<&ActorGlow>, Has<PlayerGlowGrant>), With<PlayerVisual>>,
) {
    let wanted = active.and_then(|active| active.0);
    for (body, glow, granted) in &players {
        match (wanted, glow) {
            (Some(wanted), None) => {
                commands.entity(body).try_insert((wanted, PlayerGlowGrant));
            }
            (Some(wanted), Some(glow)) if granted && *glow != wanted => {
                commands.entity(body).try_insert(wanted);
            }
            (None, _) if granted => {
                commands.entity(body).try_remove::<(ActorGlow, PlayerGlowGrant)>();
            }
            _ => {}
        }
    }
}

/// Place a glow from the standing body geometry. Shader targets can change
/// their canvas size without changing this geometry.
pub fn glow_placement(
    glow: &ActorGlow, body: &Transform, sprite: &Sprite, anchor: Option<&Anchor>,
    animator: Option<&ambition_sprite_sheet::character::CharacterAnimator>,
) -> Option<(Vec3, f32)> {
    let fallback_size = sprite.custom_size?;
    let mirrored = sprite.flip_x ^ animator.is_some_and(|animator| animator.draws_mirror_row());
    let facing = if mirrored { -1.0 } else { 1.0 };
    let (height, middle) = if let Some((animator, basis)) = animator.and_then(|animator| animator.render_basis.map(|basis| (animator, basis))) {
        let (height, mut middle) = match animator.spec.posed_body_geometry(ambition_sprite_sheet::character::CharacterAnim::Idle, 1.0) {
            Some(geometry) => {
                let scale = basis.render_size / geometry.render;
                let offset = Vec2::new(-geometry.sprite_offset.x, geometry.sprite_offset.y) * scale;
                (geometry.collision.y * scale.y, -basis.feet_anchor * basis.render_size + offset)
            }
            None => (basis.render_size.y, -basis.feet_anchor * basis.render_size),
        };
        middle.x *= facing;
        (height, middle)
    } else {
        (fallback_size.y, -anchor.map_or(Vec2::ZERO, |anchor| anchor.0) * fallback_size)
    };
    let offset = Vec2::new(glow.offset[0] * facing, glow.offset[1]) * height;
    let local = (middle + offset) * body.scale.truncate();
    let middle = body.translation.truncate() + (body.rotation * local.extend(0.0)).truncate();
    let z = body.translation.z + match glow.layer {
        GlowLayer::Behind => -BEHIND_DEPTH,
        GlowLayer::Over => OVER_BIAS,
    };
    Some((middle.extend(z), 2.0 * glow.radius.max(0.0) * height * body.scale.y.abs()))
}

fn passage_alpha(
    direct: Option<&ambition_sim_view::PassageLookView>,
    visual: Option<&super::primitives::FeatureVisual>,
    frames: Option<&ambition_sim_view::ActorAnimIndex>,
) -> f32 {
    direct.copied().or_else(|| visual.and_then(|visual| frames?.get(&visual.id)?.passage_look))
        .map_or(1.0, |look| look.alpha.clamp(0.0, 1.0))
}

/// The sprite of a glow on sim tick `tick`.
fn glow_sprite(glow: &ActorGlow, picture: &Handle<Image>, diameter: f32, tick: u64, alpha: f32) -> Sprite {
    match glow.look {
        GlowLook::SoftDisc => {
            let [r, g, b] = glow.light_color();
            Sprite {
                image: picture.clone(),
                custom_size: Some(Vec2::splat(diameter)),
                color: Color::srgba(r, g, b, glow.intensity_at(tick) * alpha),
                ..default()
            }
        }
    }
}

/// A glow shows unless something other than the portal hides its body.
fn glow_visibility(body_hidden_by_another: bool) -> Visibility {
    if body_hidden_by_another {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Draw each body's glow at its drawn place, show it while the body shows,
/// and take it away when the body or its [`ActorGlow`] goes.
///
/// The glow is a separate root, so it does not inherit its body's
/// visibility: this system mirrors it each frame.
#[allow(clippy::type_complexity)]
pub fn sync_actor_glows(
    mut commands: Commands,
    picture: Option<Res<ActorGlowPicture>>,
    tick: Option<Res<ambition_time::SimTick>>,
    frames: Option<Res<ambition_sim_view::ActorAnimIndex>>,
    bodies: Query<
        (
            Entity,
            &ActorGlow,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<Ref<Visibility>>,
            Option<&SessionScopedEntity>,
            Option<&ambition_sprite_sheet::character::CharacterAnimator>,
            Option<&ambition_sim_view::PassageLookView>,
            Option<&super::primitives::FeatureVisual>,
        ),
        Without<ActorGlowVisual>,
    >,
    mut glows: Query<(Entity, &PresentationOf, &mut Transform, &mut Sprite, &mut Visibility), (With<ActorGlowVisual>, Without<ActorGlow>)>,
) {
    let tick = tick.map_or(0, |tick| tick.0);
    let mut drawn: Vec<Entity> = Vec::new();
    for (entity, visual, mut transform, mut sprite, mut visibility) in &mut glows {
        let Ok((_, glow, body, body_sprite, anchor, body_visibility, _, animator, passage, feature)) = bodies.get(visual.0) else {
            commands.entity(entity).try_despawn();
            continue;
        };
        if drawn.contains(&visual.0) {
            // A second glow of one body (a spawn that raced a despawn).
            commands.entity(entity).try_despawn();
            continue;
        }
        drawn.push(visual.0);
        let Some((at, diameter)) = glow_placement(glow, body, body_sprite, anchor, animator) else {
            continue;
        };
        let wanted = glow_visibility(body_visibility.is_some_and(|shown| matches!(*shown, Visibility::Hidden)));
        if *visibility != wanted {
            *visibility = wanted;
        }
        transform.set_if_neq(Transform::from_translation(at));
        let now = glow_sprite(glow, &sprite.image, diameter, tick, passage_alpha(passage, feature, frames.as_deref()));
        if sprite.custom_size != now.custom_size || sprite.color != now.color {
            sprite.custom_size = now.custom_size;
            sprite.color = now.color;
        }
    }
    let Some(picture) = picture else {
        return;
    };
    for (body, glow, transform, sprite, anchor, body_visibility, session_owner, animator, passage, feature) in &bodies {
        if drawn.contains(&body) {
            continue;
        }
        let Some((at, diameter)) = glow_placement(glow, transform, sprite, anchor, animator) else {
            continue;
        };
        let scope = session_owner.map_or(SessionSpawnScope::UNSCOPED, |owner| SessionSpawnScope::scoped(owner.0));
        commands.spawn_session_scoped(
            scope,
            (
                glow_sprite(glow, &picture.0, diameter, tick, passage_alpha(passage, feature, frames.as_deref())),
                Transform::from_translation(at),
                // Set each frame above: the glow is a separate root and does
                // not inherit its body's visibility.
                glow_visibility(body_visibility.is_some_and(|shown| matches!(*shown, Visibility::Hidden))),
                ActorGlowVisual,
                // Which body this draws, in the shared spelling that portal
                // composition and view isolation read.
                PresentationOf(body),
                Name::new("Actor glow"),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        app.add_systems(Startup, build_actor_glow_picture);
        app.init_resource::<ActivePlayerGlow>();
        app.add_systems(Update, (grant_player_glows, sync_actor_glows).chain());
        app
    }

    #[test]
    fn a_glowing_body_has_a_ready_mask_without_a_content_library() {
        let mut app = app();
        app.world_mut().spawn((body_sprite(Vec3::ZERO), ActorGlow::soft(0.5, 0.6)));
        app.update();
        assert!(!app.world().contains_resource::<AssetServer>());
        let sprite = app.world_mut().query_filtered::<&Sprite, With<ActorGlowVisual>>()
            .single(app.world()).unwrap();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&sprite.image).expect("the glow has its image at spawn");
        let size = image.size();
        let data = image.data.as_ref().unwrap();
        let pixel = |x: u32, y: u32| {
            let index = ((y * size.x + x) * 4) as usize;
            &data[index..index + 4]
        };
        let center = pixel(size.x / 2, size.y / 2);
        assert_eq!(&center[..3], &[255, 255, 255]);
        assert!(center[3] > pixel(size.x / 4, size.y / 2)[3]);
        for edge in [pixel(0, 0), pixel(size.x / 2, 0), pixel(size.x - 1, size.y / 2)] {
            assert_eq!(edge[3], 0, "the mask has no visible rim");
        }
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
    }

    #[test]
    fn a_host_supplied_mask_is_kept_at_startup() {
        let mut app = app();
        let custom = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        app.insert_resource(ActorGlowPicture(custom.clone()));
        app.update();
        assert_eq!(app.world().resource::<ActorGlowPicture>().0, custom);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
    }

    /// A feet-anchored body sprite, 40 by 60, with its feet at `at`.
    fn body_sprite(at: Vec3) -> (Sprite, Anchor, Transform, Visibility) {
        (
            Sprite { custom_size: Some(Vec2::new(40.0, 60.0)), ..default() },
            Anchor(Vec2::new(0.0, -0.5)),
            Transform::from_translation(at),
            Visibility::Inherited,
        )
    }

    fn glows(app: &mut App) -> Vec<(Entity, Vec3, f32)> {
        app.world_mut()
            .query_filtered::<(&PresentationOf, &Transform, &Sprite), With<ActorGlowVisual>>()
            .iter(app.world())
            .map(|(glow, transform, sprite)| (glow.0, transform.translation, sprite.custom_size.unwrap().x))
            .collect()
    }

    #[test]
    fn the_tracking_glow_ignores_actor_tints_and_uses_the_passage_fade() {
        let mut app = app();
        let body = app.world_mut().spawn((body_sprite(Vec3::ZERO), ActorGlow::player_highlight())).id();
        let alpha = |app: &mut App| app.world_mut().query_filtered::<&Sprite, With<ActorGlowVisual>>()
            .single(app.world()).unwrap().color.alpha();
        app.update();
        let normal = alpha(&mut app);
        for tint in [Color::linear_rgba(8.0, 3.0, 1.0, 1.0), Color::srgba(0.1, 0.1, 0.1, 0.0)] {
            app.world_mut().get_mut::<Sprite>(body).unwrap().color = tint;
            app.update();
            assert_eq!(alpha(&mut app), normal);
        }
        app.world_mut().entity_mut(body).insert(ambition_sim_view::PassageLookView {
            scale: 1.0, alpha: 0.5, shade: 1.0, transit: 0.0,
        });
        app.update();
        assert_eq!(alpha(&mut app), normal * 0.5);
        app.world_mut().entity_mut(body).remove::<ambition_sim_view::PassageLookView>();
        app.update();
        assert_eq!(alpha(&mut app), normal);
    }

    /// A body that carries a glow gets one sprite that names it, about the
    /// middle of its drawn frame (not at its feet: that would read as a
    /// shadow), behind it. It follows the body, and it goes when the body
    /// loses the glow.
    #[test]
    fn a_glow_follows_its_body_and_goes_with_its_component() {
        let mut app = app();
        let body = app.world_mut().spawn((body_sprite(Vec3::new(100.0, 50.0, 20.0)), ActorGlow::soft(0.5, 0.6))).id();
        app.update();
        let found = glows(&mut app);
        assert_eq!(found.len(), 1, "one glowing body, one glow");
        let (owner, at, width) = found[0];
        assert_eq!(owner, body);
        assert_eq!(app.world_mut().query::<&PresentationOf>().single(app.world()).unwrap().0, body);
        assert_eq!(at.truncate(), Vec2::new(100.0, 80.0), "the middle of the frame, 30 over the feet");
        assert!(at.z < 20.0, "behind the body");
        assert_eq!(width, 60.0, "radius 0.5 of a 60 high frame");

        app.world_mut().get_mut::<Transform>(body).unwrap().translation = Vec3::new(-40.0, 10.0, 20.0);
        app.update();
        assert_eq!(glows(&mut app)[0].1.truncate(), Vec2::new(-40.0, 40.0), "the glow follows its body");

        app.world_mut().entity_mut(body).remove::<ActorGlow>();
        app.update();
        assert!(glows(&mut app).is_empty(), "a body with no glow has no glow sprite");
    }

    /// A hidden body has a hidden glow, and a despawned body none.
    #[test]
    fn a_glow_hides_and_goes_with_its_body() {
        let mut app = app();
        let body = app.world_mut().spawn((body_sprite(Vec3::ZERO), ActorGlow::soft(0.5, 0.6))).id();
        app.update();
        *app.world_mut().get_mut::<Visibility>(body).unwrap() = Visibility::Hidden;
        app.update();
        let shown = *app.world_mut().query_filtered::<&Visibility, With<ActorGlowVisual>>().single(app.world()).unwrap();
        assert_eq!(shown, Visibility::Hidden);
        app.world_mut().despawn(body);
        app.update();
        assert!(glows(&mut app).is_empty());
    }

    /// The route's player glow is put on the body a player drives and taken
    /// away when the route has none. A glow the game put on a body itself
    /// stays.
    #[test]
    fn the_route_s_player_glow_turns_the_player_s_glow_on_and_off() {
        let mut app = app();
        let player = app.world_mut().spawn((body_sprite(Vec3::ZERO), PlayerVisual)).id();
        let lamp = app.world_mut().spawn((body_sprite(Vec3::new(200.0, 0.0, 20.0)), ActorGlow::soft(1.0, 0.3))).id();
        app.update();
        assert_eq!(glows(&mut app).iter().map(|g| g.0).collect::<Vec<_>>(), vec![lamp], "off by default");

        app.world_mut().resource_mut::<ActivePlayerGlow>().0 = Some(ActorGlow::player_highlight());
        app.update();
        app.update();
        let mut owners: Vec<Entity> = glows(&mut app).iter().map(|g| g.0).collect();
        owners.sort();
        let mut both = vec![player, lamp];
        both.sort();
        assert_eq!(owners, both, "the player glows");

        app.world_mut().resource_mut::<ActivePlayerGlow>().0 = None;
        app.update();
        app.update();
        assert_eq!(glows(&mut app).iter().map(|g| g.0).collect::<Vec<_>>(), vec![lamp], "the player's glow goes, the lamp's stays");
        assert!(app.world().get::<ActorGlow>(player).is_none());
    }

    /// An offset is toward the body's facing.
    #[test]
    fn an_offset_turns_with_the_facing() {
        let glow = ActorGlow::soft(0.5, 0.5).with_offset([0.25, 0.0]);
        let (sprite, anchor, transform, _) = body_sprite(Vec3::ZERO);
        let right = glow_placement(&glow, &transform, &sprite, Some(&anchor), None).unwrap().0;
        let left = glow_placement(&glow, &transform, &Sprite { flip_x: true, ..sprite }, Some(&anchor), None).unwrap().0;
        assert_eq!(right.x, 15.0);
        assert_eq!(left.x, -15.0);
    }
}
