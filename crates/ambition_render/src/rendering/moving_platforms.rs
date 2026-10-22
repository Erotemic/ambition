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
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_PLAYER};
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
/// registered prop sheet kind) looks on it. A game states its looks in its
/// pack (`platform_looks`), so the type is the schema's row.
pub use ambition_sprite_sheet::prop_sheets::PlatformLook;

/// The row of a layer of `look`, and the depth it is drawn at: `None` for the
/// front layer of a look that has none.
fn look_layer(look: &PlatformLook, front: bool) -> Option<(&str, f32)> {
    if front {
        look.front_row.as_deref().map(|row| (row, FRONT_LAYER_Z))
    } else {
        Some((look.row.as_str(), BACK_LAYER_Z))
    }
}

/// Where a platform is drawn: over the blocks of the room, behind each body.
const BACK_LAYER_Z: f32 = WORLD_Z_BLOCK + 4.0;
/// Where the front row of a platform's sheet is drawn: in front of each body,
/// behind what is drawn over a body (its name and its health).
const FRONT_LAYER_Z: f32 = WORLD_Z_PLAYER + 6.0;

/// The looks a game registers for its platform sheets, by kind.
#[derive(Resource, Default, Clone, Debug)]
pub struct PlatformLooks(pub std::collections::HashMap<String, PlatformLook>);

impl PlatformLooks {
    /// Replace the looks one provider registered from its pack (`previous`,
    /// the kinds of its last registration) with `rows`, and return the kinds
    /// it now owns. A kind another provider registered is refused, and
    /// nothing changes: a reload must not take another provider's look.
    pub fn restate<'a>(
        &mut self,
        previous: impl IntoIterator<Item = &'a str>,
        rows: &ambition_sprite_sheet::prop_sheets::PlatformLookRows,
    ) -> Result<Vec<String>, String> {
        let previous: Vec<&str> = previous.into_iter().collect();
        let taken: Vec<&String> =
            rows.keys().filter(|kind| self.0.contains_key(*kind) && !previous.contains(&kind.as_str())).collect();
        if !taken.is_empty() {
            return Err(format!("platform look kind(s) {taken:?} belong to another provider"));
        }
        self.0.retain(|kind, _| !previous.contains(&kind.as_str()));
        self.0.extend(rows.iter().map(|(kind, look)| (kind.clone(), look.clone())));
        Ok(rows.keys().cloned().collect())
    }
}

/// Register [`PlatformLook`]s from a content plugin's `build`.
pub trait PlatformLooksAppExt {
    fn register_platform_look(&mut self, kind: impl Into<String>, look: PlatformLook) -> &mut Self;
    /// Register each row of a pack's `platform_looks`, and return the kinds,
    /// for a reload to restate ([`PlatformLooks::restate`]).
    fn register_platform_look_rows(&mut self, rows: &ambition_sprite_sheet::prop_sheets::PlatformLookRows) -> Vec<String>;
}

impl PlatformLooksAppExt for App {
    fn register_platform_look(&mut self, kind: impl Into<String>, look: PlatformLook) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<PlatformLooks>()
            .0
            .insert(kind.into(), look);
        self
    }

    fn register_platform_look_rows(&mut self, rows: &ambition_sprite_sheet::prop_sheets::PlatformLookRows) -> Vec<String> {
        for (kind, look) in rows {
            self.register_platform_look(kind.clone(), look.clone());
        }
        rows.keys().cloned().collect()
    }
}

/// The quad and anchor a platform's sheet is drawn at, and where: the frame
/// scaled so its `span_px` is the platform's width, anchored so frame row
/// `top_px` (at the span's middle) is the platform's top centre.
fn sheet_geometry(look: &PlatformLook, platform: &MovingPlatformState) -> (Vec2, Anchor) {
    let frame_px = Vec2::from(look.frame_px);
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
    let asset = assets?.characters.prop_asset_for_kind(look.sheet.as_deref().unwrap_or(kind))?;
    Some((kind, look, asset))
}

/// The `visual` of a platform that has no picture of its own: it is under the
/// picture of another thing (a step of the collision under the drawn ramp of
/// the cart). A platform with no `visual` is drawn as a plain rectangle, so
/// "no picture" needs a word.
pub const HIDDEN_PLATFORM_VISUAL: &str = "none";

fn is_hidden(platform: &MovingPlatformState) -> bool {
    platform.visual.as_deref() == Some(HIDDEN_PLATFORM_VISUAL)
}

/// Where a sheet-drawn platform's picture stands: its top centre.
fn sheet_point(platform: &MovingPlatformState) -> ae::Vec2 {
    ae::Vec2::new(platform.pos.x, platform.pos.y - platform.size.y * 0.5)
}

/// Whether the sheet is drawn flipped: it faces the way the platform flies
/// THROUGH THE AIR. `air_x` is how fast the room's air moves (its sky's
/// scroll, world px/s; `0.0` for a sky that stands still).
///
/// A platform slower than the air that carries it flies the other way through
/// it. The Mockingbird's sharks drift toward it at 105 px/s under a sky that
/// runs past at 800 px/s, so they fly away from it and lose ground: they face
/// away from it. A platform that does not move through the air faces right.
fn sheet_flipped(look: &PlatformLook, platform: &MovingPlatformState, air_x: f32) -> bool {
    let through_air = platform.velocity_x().unwrap_or(0.0) - air_x;
    if through_air == 0.0 {
        return !look.faces_right;
    }
    (through_air < 0.0) == look.faces_right
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
    /// This is the front layer of its sheet ([`PlatformLook::front_row`]).
    pub front: bool,
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
        (
            &ae::RoomGeometry,
            Option<&MovingPlatformSet>,
            Option<&ambition_platformer2d_world::rooms::LiveRoomDefinition>,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    // The room's authored sky: a sheet faces the way it flies through it.
    room_specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
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
    let (world, platform_set, definition) = *room;
    let air_x = definition
        .zip(room_specs.as_ref())
        .and_then(|(definition, specs)| specs.rooms().spec(*definition).metadata.visual_profile.sky_scroll_px_s)
        .map_or(0.0, |px_s| px_s as f32);
    let platforms = platform_set.map_or(&[][..], |set| &set.0[..]);
    let dt = time.map_or(0.0, |time| time.delta_secs());
    // The sheet a platform is drawn as, when its look is registered and its
    // art has loaded; otherwise it is drawn plain until they are.
    let (looks, assets) = (looks.as_deref(), assets.as_deref());
    let sheet_of = |platform| sheet_look(platform, looks, assets);
    // Retire first, so a vanished index is not mistaken for a survivor when a
    // shorter roster reuses its slot.
    // For each platform: its picture, and the front layer of its sheet.
    let mut drawn = vec![[false; 2]; platforms.len()];
    for (entity, visual, mut transform, mut sprite, animator, anchor) in &mut existing {
        let Some(platform) = platforms.get(visual.index).filter(|platform| !is_hidden(platform)) else {
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
        let layer = sheet.and_then(|(_, look, _)| look_layer(look, visual.front));
        if visual.front && layer.is_none() {
            // A front layer of a look that has none now.
            commands.entity(entity).despawn();
            continue;
        }
        drawn[visual.index][usize::from(visual.front)] = true;
        match (sheet.zip(layer), animator) {
            (Some(((_, look, _), (row, z))), Some(mut animator)) => {
                transform.translation = world_to_bevy(&world.0, sheet_point(platform), z);
                // A loop: no move plays a platform's row, so nothing ends it.
                animator.request_loop([row], ambition_sprite_sheet::character::CharacterAnim::Walk);
                // `&mut` through the `Mut` marks the components already.
                let _ = super::actors::draw_animator_frame(
                    &mut sprite,
                    &mut animator,
                    anchor.map(|anchor| anchor.into_inner()),
                    dt,
                    sheet_flipped(look, platform, air_x),
                    super::actors::StanceSquash::NONE,
                );
            }
            _ => {
                transform.translation = world_to_bevy(&world.0, platform.pos, BACK_LAYER_Z);
                // A plank turns; world +y is down and Bevy's is up, so the
                // turn is mirrored.
                transform.rotation = Quat::from_rotation_z(-platform.angle());
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
        if is_hidden(platform) {
            continue;
        }
        if let Some((kind, look, asset)) = sheet_of(platform) {
            for front in [false, true] {
                let Some((_, z)) = look_layer(look, front).filter(|_| !drawn[index][usize::from(front)]) else {
                    continue;
                };
                let (render_size, anchor) = sheet_geometry(look, platform);
                let (mut sprite, anchor, animator) =
                    ambition_sprite_sheet::character::build_character_presentation_with_render_size(
                        asset,
                        render_size,
                        anchor,
                    );
                sprite.flip_x = sheet_flipped(look, platform, air_x);
                let layer = if front { " front" } else { "" };
                commands.spawn_session_scoped(
                    session_scope,
                    (
                        sprite,
                        anchor,
                        animator,
                        Transform::from_translation(world_to_bevy(&world.0, sheet_point(platform), z)),
                        Name::new(format!("Moving platform {index}: {} ({kind}{layer})", platform.name)),
                        MovingPlatformVisual { index, sheet: Some(kind.to_string()), front },
                        RoomVisual,
                    ),
                );
            }
            continue;
        }
        if drawn[index][0] {
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
                    BACK_LAYER_Z,
                ))
                .with_rotation(Quat::from_rotation_z(-platform.angle())),
                Name::new(format!("Moving platform {index}: {}", platform.name)),
                MovingPlatformVisual { index, sheet: None, front: false },
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

    fn shark_look() -> PlatformLook {
        PlatformLook {
            sheet: None,
            row: "idle".into(),
            frame_px: [217.0, 125.0],
            span_px: [40.0, 176.0],
            top_px: 60.0,
            faces_right: true,
            front_row: None,
        }
    }

    /// A platform that sweeps `dx` at 130 px/s: right for a positive `dx`.
    fn flying(dx: f32) -> MovingPlatformState {
        MovingPlatformState::from_authored(ae::Vec2::new(400.0, 200.0), ae::Vec2::new(96.0, 16.0), dx, 130.0)
    }

    /// A sheet faces the way its platform flies through the air (Jon,
    /// 2026-10-08: the Mockingbird's sharks must face away from it).
    ///
    /// Measured before: the facing was the platform's own heading, so sharks
    /// that drift toward the Mockingbird faced it.
    #[test]
    fn a_sheet_faces_the_way_its_platform_flies_through_the_air() {
        let look = shark_look();
        let (right, left) = (flying(240.0), flying(-240.0));
        assert_eq!((right.velocity_x(), left.velocity_x()), (Some(130.0), Some(-130.0)), "premise");

        // Still air: it faces its own heading.
        assert!(!sheet_flipped(&look, &right, 0.0));
        assert!(sheet_flipped(&look, &left, 0.0), "in still air a sheet drawn facing right is flipped to fly left");

        // The sky runs left faster than the platform drifts left: through the
        // air the platform flies right, away from what holds the left side.
        assert!(!sheet_flipped(&look, &left, -800.0), "a shark that loses ground to the sky faces the way it flees");
        // A platform faster than the sky still faces its heading.
        assert!(sheet_flipped(&look, &left, -100.0));
        // The sky runs right: a platform that drifts right slower than it
        // flies left through it.
        assert!(sheet_flipped(&look, &right, 800.0));
    }

    /// A sheet-drawn platform loops its row for as long as it is drawn (Jon,
    /// 2026-10-08: the sharks played one cycle and stopped).
    ///
    /// Measured before: the row was asked for as a move's clip, which holds
    /// its last frame.
    #[test]
    fn a_sheet_drawn_platform_loops_its_row() {
        use ambition_sprite_sheet::character::{try_load_spec_for_target, CharacterSpriteAsset, SheetTuning};
        const SHARK: &str = "burning_flying_shark";

        let spec = try_load_spec_for_target(SHARK, &SheetTuning::new(1.0, 0)).expect("the shark's baked sheet");
        let slot = spec.clip_slot(["idle"]).expect("the shark's idle row");
        let idle = ambition_sprite_sheet::character::CharacterAnim::Idle;
        assert_eq!(spec.slot_for_anim(idle), slot, "premise: the idle row is the idle pose");
        let (frames, cycle_s) = (spec.frame_count(idle), spec.clip_seconds(idle));
        assert!(frames > 1, "premise: a row of one frame cannot show a loop");
        let page = ambition_sprite_sheet::character::CharacterSpritePage {
            texture: Handle::default(),
            layout: Handle::default(),
        };
        let asset = CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec,
            pages: vec![page],
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        };

        let mut app = app_with_platforms(vec![platform("shark", 400.0).with_visual(SHARK)]);
        let mut assets = GameAssets::default();
        assets.characters.props.insert(SHARK.to_string(), asset);
        app.insert_resource(assets);
        app.world_mut().get_resource_or_init::<PlatformLooks>().0.insert(SHARK.to_string(), shark_look());
        app.init_resource::<Time>();

        // Three cycles, at 60 Hz.
        let dt = 1.0 / 60.0;
        let mut seen = Vec::new();
        for _ in 0..(3.0 * cycle_s / dt) as usize {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(dt));
            app.update();
            let mut q = app.world_mut().query::<(&MovingPlatformVisual, &CharacterAnimator)>();
            if let Some((_, animator)) = q.iter(app.world()).next() {
                assert_eq!(animator.drawn_row(), Some(slot), "it draws the row of its look");
                seen.push(animator.frame);
            }
        }
        assert!(!seen.is_empty(), "premise: the platform is drawn as its sheet");
        let restarts = seen.windows(2).filter(|pair| pair[1] < pair[0]).count();
        assert!(restarts >= 2, "in three cycles the row started again {restarts} time(s): {seen:?}");
        let last_third = &seen[seen.len() * 2 / 3..];
        assert!(
            last_third.iter().any(|frame| *frame != last_third[0]),
            "the drawing stopped on frame {} in the third cycle",
            last_third[0]
        );
    }

    /// A platform whose `visual` is the word for no picture is not drawn: it
    /// is under the picture of another thing.
    #[test]
    fn a_hidden_platform_has_no_picture() {
        let mut app = app_with_platforms(vec![
            platform("a step", 100.0).with_visual(HIDDEN_PLATFORM_VISUAL),
            platform("plain", 400.0),
        ]);
        app.update();
        let drawn = visuals(&mut app);
        assert_eq!(drawn.len(), 1, "the hidden platform has no visual: {drawn:?}");
        assert_eq!(drawn[0].0, 1, "the plain one is drawn");
    }

    /// A platform is drawn at one size from a sheet of any size: the look is
    /// measured in the frame of the full-size sheet, and a reduced copy of the
    /// sheet has a smaller frame.
    #[test]
    fn the_size_of_the_picture_is_the_size_of_the_platform_and_not_of_the_sheet() {
        let look = shark_look();
        let platform = platform("shark", 400.0);
        let (size, anchor) = sheet_geometry(&look, &platform);
        // The span of the look (136 px) is the width of the platform (96).
        let k = 96.0 / 136.0;
        assert!((size - Vec2::new(217.0 * k, 125.0 * k)).length() < 1e-3, "{size:?}");
        assert!((anchor.0 - Vec2::new(108.0 / 217.0 - 0.5, 0.5 - 60.0 / 125.0)).length() < 1e-4, "{:?}", anchor.0);
    }

    /// A look with a front row draws two pictures of one platform: the row of
    /// the look behind each body, and the front row in front of each body
    /// (Jon, 2026-10-10: a body on the cart of the intro must be IN the cart,
    /// behind its near rail).
    #[test]
    fn a_front_row_is_drawn_in_front_of_a_body_and_the_row_behind_it() {
        use ambition_sprite_sheet::character::{try_load_spec_for_target, CharacterSpriteAsset, SheetTuning};
        const CART: &str = "intro_cart";

        let spec = try_load_spec_for_target(CART, &SheetTuning::new(1.0, 0)).expect("the cart's baked sheet");
        let (back, front) = (
            spec.clip_slot(["idle"]).expect("the cart's idle row"),
            spec.clip_slot(["front"]).expect("the cart's front row"),
        );
        assert_ne!(back, front, "premise: the two rows are two pictures");
        let page = ambition_sprite_sheet::character::CharacterSpritePage {
            texture: Handle::default(),
            layout: Handle::default(),
        };
        let asset = CharacterSpriteAsset {
            texture: Handle::default(),
            layout: Handle::default(),
            spec,
            pages: vec![page],
            requested_tier: Default::default(),
            resolved_tier: Default::default(),
            rigged: None,
        };
        let look = PlatformLook { front_row: Some("front".into()), ..shark_look() };

        let mut app = app_with_platforms(vec![platform("cart", 400.0).with_visual(CART)]);
        let mut assets = GameAssets::default();
        assets.characters.props.insert(CART.to_string(), asset);
        app.insert_resource(assets);
        app.world_mut().get_resource_or_init::<PlatformLooks>().0.insert(CART.to_string(), look);
        app.init_resource::<Time>();
        // The first update draws them, the second gives each its row.
        app.update();
        app.update();
        app.update();

        let mut q = app.world_mut().query::<(&MovingPlatformVisual, &Transform, &CharacterAnimator)>();
        let mut layers: Vec<(bool, f32, Option<usize>)> = q
            .iter(app.world())
            .map(|(visual, transform, animator)| (visual.front, transform.translation.z, animator.drawn_row()))
            .collect();
        layers.sort_by_key(|(front, ..)| *front);
        assert_eq!(layers.len(), 2, "one platform with a front row is two pictures, and no more: {layers:?}");
        let ((_, back_z, back_row), (_, front_z, front_row)) = (layers[0], layers[1]);
        assert_eq!((back_row, front_row), (Some(back), Some(front)), "each layer draws its own row");
        assert!(
            back_z < WORLD_Z_PLAYER && WORLD_Z_PLAYER < front_z,
            "a body (z {WORLD_Z_PLAYER}) is between the two layers: back {back_z}, front {front_z}"
        );
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
