//! The light about an actor ([`ActorGlow`]): one tinted sprite of a soft
//! round picture for each glowing body, at the body's drawn place.
//!
//! Cost: one more sprite for each glowing body. It uses the sprite pipeline
//! and batches with the other sprites. It needs no shader, no material and no
//! render target, so every quality tier draws it.
//!
//! The picture is authored art: `entities/soft_glow.png` of the sprite
//! renderer (`targets/props/entities.py`), white light that falls off to
//! nothing at its rim. The sprite colour gives it its hue and its strength.
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

use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ActivePlayerGlow, ActorGlow, GlowLayer, GlowLook,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    PlayerVisual, PresentationOf, SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// The published picture of a soft glow, under the sprite folder.
pub const SOFT_GLOW_PICTURE: &str = "sprites/entities/soft_glow.png";

/// How far behind its body a [`GlowLayer::Behind`] glow is: less than the
/// depth between two layers of a room, so nothing comes between them.
const BEHIND_DEPTH: f32 = 0.06;
/// How far over its body a [`GlowLayer::Over`] glow is: in the body's own
/// band, under the hit flash (1.5), so a strike still reads.
const OVER_BIAS: f32 = 0.5;
const _: () = assert!(OVER_BIAS < super::BODY_DEPTH_BAND);

/// The soft glow picture, loaded once. With no asset server (a headless
/// composition) there is none, and no glow is drawn.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActorGlowPicture(pub Handle<Image>);

/// Startup: load the soft glow picture.
pub fn load_actor_glow_picture(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    if let Some(asset_server) = asset_server {
        commands.insert_resource(ActorGlowPicture(asset_server.load(SOFT_GLOW_PICTURE)));
    }
}

/// On a body's visual: the route's player glow put the [`ActorGlow`] there,
/// so [`grant_player_glows`] may change it or take it away.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerGlowGrant;

/// The sprite that draws one body's glow.
#[derive(Component, Clone, Copy, Debug)]
pub struct ActorGlowVisual {
    pub body: Entity,
}

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

/// Where a body's glow is and how wide, from the body's drawn sprite. `None`
/// while the sprite has no drawn size (its first frame).
pub fn glow_placement(glow: &ActorGlow, body: &Transform, sprite: &Sprite, anchor: Option<&Anchor>) -> Option<(Vec3, f32)> {
    let size = sprite.custom_size?;
    // The middle of the drawn frame, from the point its anchor puts at the
    // body's origin; then the authored offset, mirrored with the facing.
    let frame_middle = -anchor.map_or(Vec2::ZERO, |anchor| anchor.0) * size;
    let facing = if sprite.flip_x { -1.0 } else { 1.0 };
    let offset = Vec2::new(glow.offset[0] * facing, glow.offset[1]) * size.y;
    let local = (frame_middle + offset) * body.scale.truncate();
    let middle = body.translation.truncate() + (body.rotation * local.extend(0.0)).truncate();
    let z = body.translation.z
        + match glow.layer {
            GlowLayer::Behind => -BEHIND_DEPTH,
            GlowLayer::Over => OVER_BIAS,
        };
    let diameter = 2.0 * glow.radius.max(0.0) * size.y * body.scale.y.abs();
    Some((middle.extend(z), diameter))
}

/// The sprite of a glow on sim tick `tick`.
fn glow_sprite(glow: &ActorGlow, picture: &Handle<Image>, diameter: f32, tick: u64) -> Sprite {
    match glow.look {
        GlowLook::SoftDisc => {
            let [r, g, b] = glow.light_color();
            Sprite {
                image: picture.clone(),
                custom_size: Some(Vec2::splat(diameter)),
                color: Color::srgba(r, g, b, glow.intensity_at(tick)),
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
/// visibility: this system mirrors it each frame. A body that the portal
/// alone hides is drawn by clipped pieces, and its glow stays `Visible`: the
/// glow is a compositing candidate itself, which the portal settles.
#[allow(clippy::type_complexity)]
pub fn sync_actor_glows(
    mut commands: Commands,
    picture: Option<Res<ActorGlowPicture>>,
    tick: Option<Res<ambition_time::SimTick>>,
    bodies: Query<
        (
            Entity,
            &ActorGlow,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<Ref<Visibility>>,
            Option<&SessionScopedEntity>,
            super::hit_flash::PortalHidIt,
        ),
        Without<ActorGlowVisual>,
    >,
    mut glows: Query<(Entity, &ActorGlowVisual, &mut Transform, &mut Sprite, &mut Visibility), Without<ActorGlow>>,
) {
    let tick = tick.map_or(0, |tick| tick.0);
    let mut drawn: Vec<Entity> = Vec::new();
    for (entity, visual, mut transform, mut sprite, mut visibility) in &mut glows {
        let Ok((_, glow, body, body_sprite, anchor, body_visibility, _, portal_hid_it)) = bodies.get(visual.body) else {
            commands.entity(entity).try_despawn();
            continue;
        };
        if drawn.contains(&visual.body) {
            // A second glow of one body (a spawn that raced a despawn).
            commands.entity(entity).try_despawn();
            continue;
        }
        drawn.push(visual.body);
        let Some((at, diameter)) = glow_placement(glow, body, body_sprite, anchor) else {
            continue;
        };
        let wanted = glow_visibility(body_visibility.is_some_and(|shown| {
            matches!(*shown, Visibility::Hidden) && !super::hit_flash::portal_alone_hides(portal_hid_it, &shown)
        }));
        if *visibility != wanted {
            *visibility = wanted;
        }
        transform.set_if_neq(Transform::from_translation(at));
        let now = glow_sprite(glow, &sprite.image, diameter, tick);
        if sprite.custom_size != now.custom_size || sprite.color != now.color {
            sprite.custom_size = now.custom_size;
            sprite.color = now.color;
        }
    }
    let Some(picture) = picture else {
        return;
    };
    for (body, glow, transform, sprite, anchor, body_visibility, session_owner, portal_hid_it) in &bodies {
        if drawn.contains(&body) {
            continue;
        }
        let Some((at, diameter)) = glow_placement(glow, transform, sprite, anchor) else {
            continue;
        };
        let scope = session_owner.map_or(SessionSpawnScope::UNSCOPED, |owner| SessionSpawnScope::scoped(owner.0));
        commands.spawn_session_scoped(
            scope,
            (
                glow_sprite(glow, &picture.0, diameter, tick),
                Transform::from_translation(at),
                // Set each frame above: the glow is a separate root and does
                // not inherit its body's visibility.
                glow_visibility(body_visibility.is_some_and(|shown| {
                    matches!(*shown, Visibility::Hidden) && !super::hit_flash::portal_alone_hides(portal_hid_it, &shown)
                })),
                ActorGlowVisual { body },
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
        app.insert_resource(ActorGlowPicture(Handle::default()));
        app.init_resource::<ActivePlayerGlow>();
        app.add_systems(Update, (grant_player_glows, sync_actor_glows).chain());
        app
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
            .query::<(&ActorGlowVisual, &Transform, &Sprite)>()
            .iter(app.world())
            .map(|(glow, transform, sprite)| (glow.body, transform.translation, sprite.custom_size.unwrap().x))
            .collect()
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
        let right = glow_placement(&glow, &transform, &sprite, Some(&anchor)).unwrap().0;
        let left = glow_placement(&glow, &transform, &Sprite { flip_x: true, ..sprite }, Some(&anchor)).unwrap().0;
        assert_eq!(right.x, 15.0);
        assert_eq!(left.x, -15.0);
    }
}
