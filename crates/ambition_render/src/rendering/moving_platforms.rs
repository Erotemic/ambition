//! A moving platform's picture, drawn the way every other room feature's is.
//!
//! This family derives the picture from the authoritative
//! `MovingPlatformSet`. The visual is not spawned inside the
//! room-construction transaction (`transaction::open` to
//! `transaction::close`).
//!
//! Every room feature is drawn reactively: each render family discovers its
//! own population, and [`super::features`] draws a marked rectangle for any
//! published id no family claims. Moving platforms follow the same model.
//!
//! Nothing here writes platform state. The set is only read, and the visuals
//! are reconciled to it. A restore that rewinds `MovingPlatformSet` is
//! followed on the next frame by matching visuals.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, RoomVisual, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::collision::MovingPlatformSet;
use ambition_platformer2d_world::platforms::MovingPlatformState;
use ambition_sprite_sheet::character::CharacterAnimator;
use ambition_sprite_sheet::game_assets::GameAssets;
use bevy::prelude::*;
use bevy::sprite::Anchor;

/// How a platform drawn as a sheet (`MovingPlatformState::visual`, a
/// registered prop sheet kind) looks on it: the row it loops, the span of the
/// frame (in frame pixels) its width covers, and the frame row (pixels from
/// the top) its top surface is. The Mockingbird's sharks: the back of the
/// shark is the platform, so a body stands on the saddle, not on the fin.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformLook {
    pub row: String,
    pub span_px: [f32; 2],
    pub top_px: f32,
    /// The sheet is drawn facing right (it is flipped to fly left).
    pub faces_right: bool,
}

/// The looks a game registers for its platform sheets, by kind.
#[derive(Resource, Default, Clone, Debug)]
pub struct PlatformLooks(pub std::collections::HashMap<String, PlatformLook>);

/// Register a [`PlatformLook`] from a content plugin's `build`.
pub trait PlatformLooksAppExt {
    fn register_platform_look(&mut self, kind: impl Into<String>, look: PlatformLook) -> &mut Self;
}

impl PlatformLooksAppExt for App {
    fn register_platform_look(&mut self, kind: impl Into<String>, look: PlatformLook) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<PlatformLooks>()
            .0
            .insert(kind.into(), look);
        self
    }
}

/// The quad and anchor a platform's sheet is drawn at, and where: the frame
/// scaled so its `span_px` is the platform's width, anchored so frame row
/// `top_px` (at the span's middle) is the platform's top centre.
fn sheet_geometry(look: &PlatformLook, frame_px: Vec2, platform: &MovingPlatformState) -> (Vec2, Anchor) {
    let span = (look.span_px[1] - look.span_px[0]).max(1.0);
    let k = platform.size.x / span;
    let anchor_px = Vec2::new((look.span_px[0] + look.span_px[1]) * 0.5, look.top_px);
    let anchor = Vec2::new(anchor_px.x / frame_px.x - 0.5, 0.5 - anchor_px.y / frame_px.y);
    (frame_px * k, Anchor(anchor))
}

/// The sheet `platform` is drawn as, its look and its art: `None` until its
/// look is registered and its art has loaded (it is drawn plain until then).
fn sheet_look<'a>(
    platform: &'a MovingPlatformState,
    looks: Option<&'a PlatformLooks>,
    assets: Option<&'a GameAssets>,
) -> Option<(&'a str, &'a PlatformLook, &'a ambition_sprite_sheet::character::CharacterSpriteAsset)> {
    let kind = platform.visual.as_deref()?;
    let look = looks?.0.get(kind)?;
    let asset = assets?.characters.prop_asset_for_kind(kind)?;
    Some((kind, look, asset))
}

/// Where a sheet-drawn platform's picture stands: its top centre.
fn sheet_point(platform: &MovingPlatformState) -> ae::Vec2 {
    ae::Vec2::new(platform.pos.x, platform.pos.y - platform.size.y * 0.5)
}

/// Whether the sheet is drawn flipped: it faces the way the platform flies.
fn sheet_flipped(look: &PlatformLook, platform: &MovingPlatformState) -> bool {
    match platform.heading_x() {
        Some(dir) => (dir < 0.0) == look.faces_right,
        None => !look.faces_right,
    }
}

/// The picture of one moving platform, tied to its index in the authoritative
/// [`MovingPlatformSet`].
///
/// The index is the identity: the set is a positional roster rebuilt by room
/// construction, so a platform has no id of its own. A room change replaces
/// the whole roster and all its visuals.
#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
    /// The sheet it is drawn as, when it is one: a slot whose platform now
    /// wants another look is drawn again.
    pub sheet: Option<String>,
}

/// Reconcile the moving-platform visuals against the authoritative set.
///
/// Spawns what is missing, retires what the set no longer has, and moves and
/// resizes the rest. Idempotent: it compares populations instead of reacting
/// to events, so it needs no change detection and cannot double-spawn during
/// a rollback resimulation.
pub fn sync_moving_platform_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    // The live room's geometry and its platforms, off one root.
    room: Single<
        (&ae::RoomGeometry, Option<&MovingPlatformSet>),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    mut existing: Query<(
        Entity,
        &MovingPlatformVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
    )>,
    looks: Option<Res<PlatformLooks>>,
    assets: Option<Res<GameAssets>>,
    time: Option<Res<Time>>,
) {
    let (world, platform_set) = *room;
    let platforms = platform_set.map_or(&[][..], |set| &set.0[..]);
    let dt = time.map_or(0.0, |time| time.delta_secs());
    // The sheet a platform is drawn as, when its look is registered and its
    // art has loaded; otherwise it is drawn plain until they are.
    let (looks, assets) = (looks.as_deref(), assets.as_deref());
    let sheet_of = |platform| sheet_look(platform, looks, assets);
    // Retire first, so a vanished index is not mistaken for a survivor when a
    // shorter roster reuses its slot.
    let mut drawn = vec![false; platforms.len()];
    for (entity, visual, mut transform, mut sprite, animator, anchor) in &mut existing {
        let Some(platform) = platforms.get(visual.index) else {
            commands.entity(entity).despawn();
            continue;
        };
        let sheet = sheet_of(platform);
        if sheet.map(|(kind, ..)| kind) != visual.sheet.as_deref() {
            // Its look changed (its art arrived, or the slot is another
            // platform's now): drawn again below.
            commands.entity(entity).despawn();
            continue;
        }
        drawn[visual.index] = true;
        match (sheet, animator) {
            (Some((_, look, _)), Some(mut animator)) => {
                transform.translation = world_to_bevy(&world.0, sheet_point(platform), WORLD_Z_BLOCK + 4.0);
                animator.request_clip([look.row.as_str()], ambition_sprite_sheet::character::CharacterAnim::Walk);
                super::actors::draw_animator_frame(
                    &mut sprite,
                    &mut animator,
                    anchor.map(|anchor| anchor.into_inner()),
                    dt,
                    sheet_flipped(look, platform),
                    super::actors::StanceSquash::NONE,
                );
            }
            _ => {
                transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
                sprite.custom_size = Some(Vec2::new(platform.size.x, platform.size.y));
            }
        }
    }

    // Spawning needs a session scope; retiring does not, so a mid-frame
    // teardown still clears the population.
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (index, platform) in platforms.iter().enumerate() {
        if drawn[index] {
            continue;
        }
        if let Some((kind, look, asset)) = sheet_of(platform) {
            let (render_size, anchor) = sheet_geometry(look, asset.spec.frame_pixels(), platform);
            let (mut sprite, anchor, animator) =
                ambition_sprite_sheet::character::build_character_presentation_with_render_size(
                    asset,
                    render_size,
                    anchor,
                );
            sprite.flip_x = sheet_flipped(look, platform);
            commands.spawn_session_scoped(
                session_scope,
                (
                    sprite,
                    anchor,
                    animator,
                    Transform::from_translation(world_to_bevy(
                        &world.0,
                        sheet_point(platform),
                        WORLD_Z_BLOCK + 4.0,
                    )),
                    Name::new(format!("Moving platform {index}: {} ({kind})", platform.name)),
                    MovingPlatformVisual { index, sheet: Some(kind.to_string()) },
                    RoomVisual,
                ),
            );
            continue;
        }
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(
                    Color::srgba(0.35, 0.74, 1.0, 0.92),
                    Vec2::new(platform.size.x, platform.size.y),
                ),
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    platform.pos,
                    WORLD_Z_BLOCK + 4.0,
                )),
                Name::new(format!("Moving platform {index}: {}", platform.name)),
                MovingPlatformVisual { index, sheet: None },
                RoomVisual,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    fn platform(name: &str, x: f32) -> MovingPlatformState {
        let mut state = MovingPlatformState::from_authored(
            ae::Vec2::new(x, 200.0),
            ae::Vec2::new(96.0, 16.0),
            240.0,
            130.0,
        );
        state.name = name.to_string();
        state
    }

    fn app_with_platforms(states: Vec<MovingPlatformState>) -> App {
        let mut app = App::new();
        app.init_resource::<ActiveSessionScope>();
        app.world_mut().resource_mut::<ActiveSessionScope>().begin();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            ae::RoomGeometry(ae::World::new(
                "moving platform fixture",
                ae::Vec2::new(1280.0, 720.0),
                ae::Vec2::ZERO,
                Vec::new(),
            )),
        );
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            MovingPlatformSet(states),
        );
        app.add_systems(Update, sync_moving_platform_visuals);
        app
    }

    fn visuals(app: &mut App) -> Vec<(usize, ae::Vec2)> {
        let mut q = app
            .world_mut()
            .query::<(&MovingPlatformVisual, &Transform)>();
        let world = app.world();
        let mut rows: Vec<(usize, ae::Vec2)> = q
            .iter(world)
            .map(|(visual, transform)| {
                (
                    visual.index,
                    ae::Vec2::new(transform.translation.x, transform.translation.y),
                )
            })
            .collect();
        rows.sort_by_key(|(index, _)| *index);
        rows
    }

    /// A platform gets its visual without the room construction transaction
    /// spawning one: the set exists, and the family draws it.
    #[test]
    fn a_platform_in_the_set_gets_a_visual_without_any_construction_commit() {
        let mut app = app_with_platforms(vec![platform("a", 100.0), platform("b", 400.0)]);
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 2, "one visual per platform in the set");
        assert_eq!(drawn[0].0, 0);
        assert_eq!(drawn[1].0, 1);
    }

    /// It follows the authoritative set instead of remembering.
    ///
    /// A reconcile keeps no state, so it cannot overwrite restored platform
    /// state after a cross-room restore. A platform moved by any means (a tick,
    /// a room change, a rollback restore) is followed.
    #[test]
    fn the_visual_follows_a_restored_set_instead_of_remembering_a_start() {
        let mut app = app_with_platforms(vec![platform("a", 100.0)]);
        app.update();
        let before = visuals(&mut app)[0].1;

        // A jump like a rollback restore or room change: the set says somewhere
        // else, with no event.
        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the fixture room has platforms").0[0].pos = ae::Vec2::new(900.0, 200.0);
        app.update();
        let after = visuals(&mut app)[0].1;

        assert!(
            (after.x - before.x).abs() > 100.0,
            "the visual must follow the authoritative set ({before:?} -> {after:?}); \
             a family that remembered its own start would still be at the old place"
        );
        assert_eq!(visuals(&mut app).len(), 1, "and it must not double-spawn");
    }

    /// A shorter roster retires the visuals it no longer has. A room change
    /// replaces the whole set; nothing may be left drawing the old room's
    /// platforms.
    #[test]
    fn a_platform_that_leaves_the_set_stops_being_drawn() {
        let mut app = app_with_platforms(vec![platform("a", 100.0), platform("b", 400.0)]);
        app.update();
        assert_eq!(visuals(&mut app).len(), 2);

        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the fixture room has platforms").0.pop();
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 1, "the departed platform's visual is retired");
        assert_eq!(drawn[0].0, 0);
    }
}
