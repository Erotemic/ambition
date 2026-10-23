//! Item visuals: ground-item quads, the held-item sprite, and held-projectile
//! sprites. Pure consumers of the sim-built `sim_view` item snapshots, with no
//! live item or body queries.

use ambition_platformer2d_shared_tangle::binding::{
    log_unresolved, Namespace, ReportedOnce, Resolver, UnresolvedRef,
};
use ambition_platformer2d_shared_tangle::held_item_art::HeldItemSprite;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::world_item_art::WorldItemSprite;
use ambition_sim_view::{GroundItemsView, HeldItemView};
use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

// Presentation (visible build only).

/// Provider-contributed art, resolved once at startup: the ids that bound, and
/// the loaded `(image, display size)` for each, indexed by the binding's slot.
///
/// A miss gives a diagnosis the caller can print. The placeholder still draws,
/// so a blind run never goes black, and the run names what it could not find.
///
/// It does not check that the images arrive. `AssetServer::load` returns a
/// handle even for a missing path, so an id can bind to art that never draws.
/// [`report_unloadable_item_art`] checks that.
pub struct ArtBindings<N: Namespace> {
    ids: Resolver<N>,
    /// Parallel to the resolver's declaration slots.
    art: Vec<(Handle<Image>, Vec2)>,
}

impl<N: Namespace> Default for ArtBindings<N> {
    fn default() -> Self {
        Self {
            ids: Resolver::default(),
            art: Vec::new(),
        }
    }
}

impl<N: Namespace> ArtBindings<N> {
    /// Pair a manifest's resolver with the handles loaded from the same effective
    /// entry list, so slot `i` of one addresses slot `i` of the other.
    pub fn new(ids: Resolver<N>, art: impl IntoIterator<Item = (Handle<Image>, Vec2)>) -> Self {
        let art: Vec<_> = art.into_iter().collect();
        debug_assert_eq!(
            ids.ids().len(),
            art.len(),
            "resolver and handles must come from the same effective entry list",
        );
        Self { ids, art }
    }

    /// The art for `id`, or nothing.
    pub fn get(&self, id: &str) -> Option<(Handle<Image>, Vec2)> {
        let bound = self.ids.bind(id)?;
        Some(self.art[bound.slot()].clone())
    }

    /// Why `id` has no art, in full. Call once per distinct failure.
    pub fn explain(&self, id: &str, declared_by: impl Into<String>) -> UnresolvedRef {
        self.ids.explain(id, declared_by)
    }

    /// Every registered id beside its loaded image handle, for the pass that
    /// checks the files arrived (see [`report_unloadable_item_art`]).
    ///
    /// Uses `declarations`, not `ids`: the resolver is sorted for lookup and the
    /// handles are in declaration order, so zipping them would name the wrong id.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Handle<Image>)> {
        self.ids
            .declarations()
            .map(|(id, slot)| (id, &self.art[slot].0))
    }
}

/// Registered art ids whose image asset reached a terminal failed state.
///
/// The binding is still valid (manifest and consumer agree on the id), but
/// renderers must use their placeholder instead of a sprite whose handle
/// can never produce an image.
#[derive(Resource, Default)]
pub struct FailedItemArt {
    world: BTreeSet<String>,
    held: BTreeSet<String>,
}

/// Handles that have not yet reached a terminal load state.
///
/// Loaded and failed entries are removed, so the build-health watcher does not
/// probe every successful image forever.
#[derive(Default)]
pub struct ItemArtLoadWatch {
    world: BTreeMap<String, Handle<Image>>,
    held: BTreeMap<String, Handle<Image>>,
}

/// Resolve `id` through `art`, reporting a miss at most once per distinct
/// failure and paying for the diagnostic only then.
///
/// What every per-frame consumer of an [`ArtBindings`] wants: draw the
/// placeholder either way, report the problem the first time, and then stop.
fn resolve_art<N: Namespace>(
    art: Option<&ArtBindings<N>>,
    failed: Option<&BTreeSet<String>>,
    id: &str,
    declared_by: impl FnOnce() -> String,
    reported: &mut ReportedOnce,
    context: &str,
) -> Option<(Handle<Image>, Vec2)> {
    let art = art?;
    if failed.is_some_and(|failed| failed.contains(id)) {
        return None;
    }
    if let Some(found) = art.get(id) {
        return Some(found);
    }
    let declared_by = declared_by();
    if reported.first_sight(N::NAME, &declared_by, id) {
        log_unresolved(context, &art.explain(id, declared_by));
    }
    None
}

/// Re-arm the watch because the art resource was replaced: every id is
/// unsettled again, and failures from the old manifest do not apply.
fn reset_art_watch<N: Namespace>(
    art: Option<&ArtBindings<N>>,
    pending: &mut BTreeMap<String, Handle<Image>>,
    failed: &mut BTreeSet<String>,
) {
    pending.clear();
    failed.clear();
    if let Some(art) = art {
        pending.extend(
            art.entries()
                .map(|(id, image)| (id.to_owned(), image.clone())),
        );
    }
}

/// Drain the watch: settled handles leave it, and each failure is reported
/// once and recorded, so the render path uses the placeholder instead.
fn poll_art_watch(
    assets: &AssetServer,
    context: &str,
    pending: &mut BTreeMap<String, Handle<Image>>,
    failed: &mut BTreeSet<String>,
) {
    pending.retain(|id, image| match assets.load_state(image.id()) {
        bevy::asset::LoadState::Loaded => false,
        bevy::asset::LoadState::Failed(_) => {
            failed.insert(id.clone());
            let path = assets
                .get_path(image.id())
                .map(|path| path.to_string())
                .unwrap_or_else(|| "<no path>".to_owned());
            error!(
                "{context}: `{id}` is registered and bound, but its image `{path}` failed to load — \
                 the id is fine and the FILE is missing or unreadable (check the generator target)",
            );
            false
        }
        _ => true,
    });
}

/// Report a bound art id whose image never arrives, and stop drawing it.
///
/// A resolver cannot see this case. The id is registered and the manifest
/// names a path, but if no file exists, `AssetServer::load` still returns a
/// handle. An id namespace proves only that content agrees with content;
/// this checks that the file exists.
///
/// Reporting alone is not enough: a bound id with a failed image would still
/// take the sprite branch and draw nothing. Failed ids go into
/// [`FailedItemArt`], and the render path treats them as unresolved so the
/// placeholder quad draws.
///
/// Each entry is probed only until it settles (loaded or failed), so this is
/// free once a room's art is resolved.
pub fn report_unloadable_item_art(
    assets: Res<AssetServer>,
    world_art: Option<Res<WorldItemArt>>,
    held_art: Option<Res<HeldItemArt>>,
    mut failed: ResMut<FailedItemArt>,
    mut watch: Local<ItemArtLoadWatch>,
) {
    if world_art.as_ref().is_some_and(|art| art.is_changed()) {
        reset_art_watch(
            world_art.as_deref().map(|art| &art.0),
            &mut watch.world,
            &mut failed.world,
        );
    }
    if held_art.as_ref().is_some_and(|art| art.is_changed()) {
        reset_art_watch(
            held_art.as_deref().map(|art| &art.0),
            &mut watch.held,
            &mut failed.held,
        );
    }
    poll_art_watch(
        &assets,
        "world item art",
        &mut watch.world,
        &mut failed.world,
    );
    poll_art_watch(&assets, "held item art", &mut watch.held, &mut failed.held);
}

/// The z of a ground item: over a portal's frame and its window, as a body
/// is, so an item that crosses a portal is not covered by the portal's line
/// of light at the seam (measured with the seam gauge: at 8.0, under the
/// frame's 9.3, half the gauge was hidden as it crossed), and an item in
/// front of a window is not covered by the window. Under the bodies.
const GROUND_ITEM_Z: f32 = ambition_platformer2d_core::config::WORLD_Z_DUMMY - 0.2;

/// Marks a sprite entity visualizing a [`GroundItem`]: the item it draws.
#[derive(Component)]
pub struct GroundItemVisual(pub Entity);

/// Loaded held and inventory item art, resolved from every provider's
/// [`HeldItemArtManifest`](ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest):
/// held-item spec id to `(image, on-screen display size)`. The engine owns the
/// seam (this resource, [`build_held_item_art`], and the lookup in the sync
/// systems); each game contributes its own item images without a render
/// dependency. Absent or unmatched ids draw the placeholder quad.
#[derive(Resource, Default)]
pub struct HeldItemArt(pub ArtBindings<HeldItemSprite>);

/// Resolve every provider-contributed
/// [`HeldItemArtEntry`](ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtEntry)
/// (`id -> path + size` data) into loaded image handles, filling
/// [`HeldItemArt`]. The render half of the contribution seam: games declare
/// art without a render dependency, and the `AssetServer` work is here, so a
/// multi-game host's merged manifest binds every provider's items. It runs
/// when the manifest is added or changed (a content reload restates it). No
/// manifest gives no `HeldItemArt`, and every item draws the quad fallback.
pub fn build_held_item_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    manifest: Res<ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest>,
) {
    commands.insert_resource(HeldItemArt(ArtBindings::new(
        manifest.item_ids(),
        manifest.effective().into_iter().map(|entry| {
            (
                ambition_sprite_sheet::game_assets::load_sheet_image(
                    &assets,
                    "held-item",
                    entry.asset_path.clone(),
                ),
                Vec2::new(entry.size.x, entry.size.y),
            )
        }),
    )));
}

/// [`build_held_item_art`] as the render plugin schedules it: in `Update`,
/// when the manifest is added or changed, and before its readers.
pub(super) fn resolve_held_item_art_when_the_manifest_changes()
-> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::ScheduleSystem> {
    build_held_item_art
        .run_if(resource_exists_and_changed::<ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest>)
        .before(sync_ground_item_visuals)
        .before(report_unloadable_item_art)
}

/// Draw each ground item row as a kept sprite: one visual per item and live
/// room, its transform written only when the row moves it, and spawned again
/// only when its room or item changes or the art is replaced. Until
/// 2026-10-23 every visual was despawned and spawned again each frame.
pub fn sync_ground_item_visuals(
    mut commands: Commands,
    // Each row is placed by the geometry of its own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
    failed_art: Option<Res<FailedItemArt>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut visuals: Query<(Entity, &GroundItemVisual, &GroundItemDrawn, &mut Transform)>,
    grounds: Res<GroundItemsView>,
    // Report each missing id once.
    mut reported: Local<ReportedOnce>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        for (entity, ..) in &visuals {
            commands.entity(entity).despawn();
        }
        return;
    };
    // A replaced manifest is new content: earlier reports no longer apply,
    // and every visual is drawn again from it.
    let art_replaced = art.as_ref().is_some_and(|art| art.is_changed());
    if art_replaced {
        reported.clear();
    }
    // The rows to draw, each with its room and transform. A row whose live
    // room cannot be told is not drawn.
    let mut wanted: Vec<(&ambition_sim_view::GroundItemFact, LiveRoomInstance, Transform, bool)> = Vec::new();
    for ground in &grounds.0 {
        let Some((room, world)) = ground.room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let translation =
            ambition_platformer2d_core::config::world_to_bevy(&world.0, ground.pos, GROUND_ITEM_Z);
        let transform = Transform::from_translation(translation).with_rotation(Quat::from_rotation_z(ground.roll));
        wanted.push((ground, room, transform, false));
    }
    for (entity, link, drawn, mut transform) in &mut visuals {
        let kept = (!art_replaced)
            .then(|| {
                wanted.iter_mut().find(|(ground, room, _, taken)| {
                    !*taken && ground.item == link.0 && *room == drawn.room && ground.item_id == drawn.item_id
                })
            })
            .flatten();
        match kept {
            Some((_, _, wanted_transform, taken)) => {
                *taken = true;
                transform.set_if_neq(*wanted_transform);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (ground, room, transform, taken) in wanted {
        if taken {
            continue;
        }
        let bound = resolve_art(
            art.as_deref().map(|art| &art.0),
            failed_art.as_deref().map(|failed| &failed.held),
            ground.item_id.as_str(),
            || "ground item".to_owned(),
            &mut reported,
            "ground item visual",
        );
        // The placeholder still draws; the report makes the miss visible.
        let sprite = bound
            .map(|(image, size)| Sprite {
                image,
                custom_size: Some(size),
                ..default()
            })
            .unwrap_or_else(|| {
                Sprite::from_color(Color::srgb(0.72, 0.52, 0.30), ground.half_extent * 2.0)
            });
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                GroundItemVisual(ground.item),
                GroundItemDrawn {
                    room,
                    item_id: ground.item_id.clone(),
                },
                sprite,
                transform,
                Name::new("Ground item visual"),
            ),
        );
    }
}

/// What a kept ground item visual was spawned for: a row with another room or
/// another item id is drawn by a new visual.
#[derive(Component)]
pub struct GroundItemDrawn {
    room: LiveRoomInstance,
    item_id: String,
}

/// Marks a sprite entity visualizing a `WorldItem` (`ambition_world_items`).
#[derive(Component)]
pub struct WorldItemVisual;

/// Game-supplied art for walk-into world items, keyed by the `sprite` id a
/// `WorldItem` (`ambition_world_items`) carries, to `(image, on-screen display
/// size)`. The engine owns the seam (this resource and the lookup in
/// [`sync_world_item_visuals`]); each game fills it with its own pickup images
/// (for example Mary-O's star wand). Absent or unmatched ids draw the
/// row-tinted placeholder quad.
#[derive(Resource, Default)]
pub struct WorldItemArt(pub ArtBindings<WorldItemSprite>);

/// Resolve the provider-contributed
/// [`WorldItemArtManifest`](ambition_platformer2d_shared_tangle::world_item_art::WorldItemArtManifest)
/// (`id -> path + size` data from every game) into loaded image handles,
/// filling [`WorldItemArt`]. The render half of the contribution seam: games
/// declare pickup art without a render dependency, and a multi-game host's
/// merged manifest binds every provider's pickups. No manifest gives an empty
/// map (every item draws the quad fallback).
pub fn build_world_item_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    manifest: Option<
        Res<ambition_platformer2d_shared_tangle::world_item_art::WorldItemArtManifest>,
    >,
) {
    let art = match manifest {
        Some(manifest) => WorldItemArt(ArtBindings::new(
            manifest.sprite_ids(),
            manifest.effective().into_iter().map(|entry| {
                (
                    ambition_sprite_sheet::game_assets::load_sheet_image(
                        &assets,
                        "held-item",
                        entry.asset_path.clone(),
                    ),
                    Vec2::new(entry.size.x, entry.size.y),
                )
            }),
        )),
        None => WorldItemArt::default(),
    };
    commands.insert_resource(art);
}

/// [`build_world_item_art`] as the render plugin schedules it: in `Update`,
/// when the manifest is added or changed, and before its readers.
pub(super) fn resolve_world_item_art_when_the_manifest_changes()
-> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::ScheduleSystem> {
    build_world_item_art
        .run_if(resource_exists_and_changed::<ambition_platformer2d_shared_tangle::world_item_art::WorldItemArtManifest>)
        .before(sync_world_item_visuals)
        .before(report_unloadable_item_art)
}

/// A sprite per walk-into world item: the real image when its `sprite` id is
/// bound in [`WorldItemArt`], otherwise a quad tinted by the row it grants
/// (star wand gold, cinder beacon ember, unknown magenta). Runs while a row or a
/// drawn visual stands, like [`sync_ground_item_visuals`].
pub fn sync_world_item_visuals(
    mut commands: Commands,
    // Each row is placed by the geometry of its own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    art: Option<Res<WorldItemArt>>,
    failed_art: Option<Res<FailedItemArt>>,
    mut visuals: Query<(Entity, &WorldItemDrawn, &mut Transform), With<WorldItemVisual>>,
    items: Res<ambition_sim_view::WorldItemsView>,
    mut reported: Local<ReportedOnce>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        for (entity, ..) in &visuals {
            commands.entity(entity).despawn();
        }
        return;
    };
    // A replaced manifest is new content: earlier reports no longer apply, and
    // every visual is drawn again from it.
    let art_replaced = art.as_ref().is_some_and(|art| art.is_changed());
    if art_replaced {
        reported.clear();
    }
    // A visual is kept while a row of its room and look remains: rows have no
    // identity of their own, so two alike rows may trade visuals, which draws
    // the same picture. Its transform is written only when it moves.
    let mut kept: Vec<bool> = vec![false; items.0.len()];
    for (entity, drawn, mut transform) in &mut visuals {
        let row = (!art_replaced)
            .then(|| {
                items.0.iter().enumerate().find(|(index, item)| {
                    !kept[*index] && item.room == Some(drawn.room) && drawn.looks_like(item)
                })
            })
            .flatten()
            .and_then(|(index, item)| Some((index, item, rooms.in_room(drawn.room)?)));
        match row {
            Some((index, item, world)) => {
                kept[index] = true;
                let z = if item.emerging { -1.0 } else { 8.0 };
                let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, item.pos, z);
                transform.set_if_neq(Transform::from_translation(translation));
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (index, item) in items.0.iter().enumerate() {
        if kept[index] {
            continue;
        }
        // An emerging pickup draws behind the world, so it looks like it comes
        // out of its block. `WORLD_Z_BLOCK` is 0.0, so anything below it is
        // hidden by the geometry; a free item uses 8.0.
        let z = if item.emerging { -1.0 } else { 8.0 };
        let Some((room, world)) = item.room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let session_scope = session_scope.in_room(Some(room));
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, item.pos, z);
        // A bound sprite wins; otherwise the row-tinted quad. An item with no
        // sprite id is authored that way and reports nothing; an item with an
        // unregistered id is reported once.
        let bound = item.sprite.as_deref().and_then(|id| {
            resolve_art(
                art.as_deref().map(|art| &art.0),
                failed_art.as_deref().map(|failed| &failed.world),
                id,
                || format!("world item `{}`", item.row_id),
                &mut reported,
                "world item visual",
            )
        });
        let sprite = match bound {
            Some((image, size)) => Sprite {
                image,
                custom_size: Some(size),
                ..default()
            },
            None => {
                let color = match item.row_id.as_str() {
                    "star_wand" => Color::srgb(0.99, 0.84, 0.42),
                    "cinder_beacon" => Color::srgb(0.95, 0.55, 0.20),
                    _ => Color::srgb(0.90, 0.20, 0.80),
                };
                Sprite::from_color(color, item.half_extent * 2.0)
            }
        };
        commands.spawn_session_scoped(
            session_scope,
            (
                WorldItemVisual,
                WorldItemDrawn {
                    room,
                    row_id: item.row_id.clone(),
                    sprite: item.sprite.clone(),
                },
                sprite,
                Transform::from_translation(translation),
                Name::new("World item visual"),
            ),
        );
    }
}

/// What a kept world item visual draws: a row of another room or another look
/// is drawn by a new visual.
#[derive(Component)]
pub struct WorldItemDrawn {
    room: LiveRoomInstance,
    row_id: String,
    sprite: Option<String>,
}

impl WorldItemDrawn {
    fn looks_like(&self, item: &ambition_sim_view::WorldItemFact) -> bool {
        self.row_id == item.row_id && self.sprite == item.sprite
    }
}

use super::posed_hand::ITEM_REST_ANGLE;

/// The angle of an item with a grip in the hand of a holder that faces
/// right, in degrees up from level.
///
/// At rest it points forward and a little up. A swing takes it from behind
/// the strike to past it while the move strikes, and back to rest while the
/// move settles. A swing forward comes down from over the shoulder; a swing up
/// goes from low in front to over the head; a swing down goes from in front to
/// under the feet; a swing back is the forward swing on the other side.
pub fn item_swing_angle(swing: Option<ambition_sim_view::ItemSwingFact>) -> f32 {
    use ambition_characters::moveset_prefabs::ItemSwingDir;
    let Some(swing) = swing else {
        return ITEM_REST_ANGLE;
    };
    let (from, to) = match swing.dir {
        ItemSwingDir::Forward => (128.0, -46.0),
        ItemSwingDir::Up => (-20.0, 124.0),
        ItemSwingDir::Down => (40.0, -112.0),
        ItemSwingDir::Back => (52.0, 226.0),
    };
    // Fast at the start of the strike and slow at its end, as a swing is.
    let through = 1.0 - (1.0 - swing.strike) * (1.0 - swing.strike);
    let struck = from + (to - from) * through;
    struck + (ITEM_REST_ANGLE - struck) * (swing.settle * swing.settle * (3.0 - 2.0 * swing.settle))
}

/// One item to draw in a hand, from either view of the holders.
struct InHand<'a> {
    item_id: &'a str,
    /// The hand when the body's art has none: a fixed place on its box.
    hand: Vec2,
    facing: f32,
    ranged: bool,
    aim: Vec2,
    swing: Option<ambition_sim_view::ItemSwingFact>,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    holder: &'a ambition_sim_view::ItemHolder,
}

/// Marks the sprite shown in the player's hand for the currently held item.
#[derive(Component)]
pub struct HeldItemVisual;

/// What a held item visual draws: this item in this body's hand, in this live
/// room. A visual is kept while its holder holds the item there, and moved;
/// another item, holder or room is a new visual.
#[derive(Component)]
pub struct HeldItemDrawn {
    holder: Entity,
    room: LiveRoomInstance,
    item_id: String,
}

/// Draw a small quad in the controlled subject's hand for its held item,
/// tinted per item (axe, javelin). Kept per holder and item, placed each frame.
///
/// Keyed on [`ControlledSubject`] (the body with
/// `DrivingParticipant(PRIMARY)`), not `PrimaryPlayer`: while possessing, the
/// sprite draws on the driven body (its own `HeldItem`), not the vacated home
/// avatar. The blink reticle, camera, and nameplate use the same rule. Aim
/// comes from the subject's `ActorControl`, not raw device input.
pub fn sync_held_item_visual(
    mut commands: Commands,
    // Each row is placed by the geometry of its own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
    // Where the hand holds each item (`HeldItemArtEntry::grip`).
    manifest: Option<Res<ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest>>,
    failed_art: Option<Res<FailedItemArt>>,
    active_session: Option<Res<ActiveSessionScope>>,
    held_view: Res<HeldItemView>,
    // A living hostile is on the over-hand road. That road draws the items
    // its own catalog has (the gun-swords); an item it does not have is drawn
    // here, from the same art as in any other hand, so that an item is
    // registered one time.
    wielded_view: Res<ambition_sim_view::HostileWieldedItemsView>,
    wielded_catalog: Option<Res<super::wielded_item_visuals::WieldedItemVisualCatalog>>,
    // The hand of the pose each holder draws (`posed_hand`).
    hands: super::posed_hand::PosedHands,
    mut visuals: Query<(Entity, &HeldItemDrawn, &mut Transform, &mut Sprite), With<HeldItemVisual>>,
    mut reported: Local<ReportedOnce>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        for (entity, ..) in &visuals {
            commands.entity(entity).despawn();
        }
        return;
    };
    // A replaced manifest is new content: every visual is drawn again from it.
    let art_replaced = art.as_ref().is_some_and(|art| art.is_changed());
    if art_replaced {
        reported.clear();
    }
    // Each item to draw, with where and how it is turned this frame.
    let mut wanted: Vec<(InHand, LiveRoomInstance, Transform, bool, bool, bool)> = Vec::new();
    // One visual per driven holder. The view is ordered by `SimId`, so the
    // draw order is the same every run.
    let in_hand = held_view.0.iter().map(|held| {
        let facing = if held.facing >= 0.0 { 1.0 } else { -1.0 };
        InHand {
            item_id: held.item_id.as_str(),
            // In the hand: just in front, at hand height (y-down, so small +y).
            hand: held.pos + Vec2::new(facing * (held.size.x * 0.45 + 4.0), held.size.y * 0.06),
            facing: held.facing,
            ranged: held.ranged,
            aim: held.aim,
            swing: held.swing,
            room: held.room,
            holder: &held.holder,
        }
    });
    let over_hand = wielded_view
        .0
        .iter()
        .filter(|fact| wielded_catalog.as_deref().is_none_or(|catalog| catalog.get(&fact.item_id).is_none()))
        .map(|fact| {
            let aim = fact.aim_world - fact.hand_world;
            InHand {
                item_id: fact.item_id.as_str(),
                hand: fact.hand_world,
                facing: if aim.x >= 0.0 { 1.0 } else { -1.0 },
                ranged: fact.ranged,
                aim,
                swing: None,
                room: fact.room,
                holder: &fact.holder,
            }
        });
    for held in in_hand.chain(over_hand) {
        let facing = if held.facing >= 0.0 { 1.0 } else { -1.0 };
        let hand = held.hand;
        let Some((room, world)) = held.room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let mut translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, hand, 12.0);
        // The art's own hand, when the body is drawn from parts that have
        // one: the item is in it, in the frame that is drawn.
        let posed = hands.of(held.holder);
        if let Some(posed) = posed {
            translation.x = posed.at.x;
            translation.y = posed.at.y;
        }
        let grip = manifest.as_deref().and_then(|manifest| {
            manifest.effective().into_iter().find(|entry| entry.item_id == held.item_id).and_then(|entry| Some((entry.grip?, entry.size.x)))
        });

        // A ranged held item (the gun-sword) points where the subject aims,
        // the direction it fires. Melee and thrown items only flip with
        // facing. Aim is the subject's resolved frame (falls back to facing),
        // so a possessed body's item follows its own aim.
        let (rotation, flip_x, flip_y) = if held.ranged {
            let aim = if held.aim.length_squared() > 1e-4 {
                held.aim.normalize()
            } else {
                Vec2::new(held.facing, 0.0)
            };
            // World is y-down, render space y-up. Aiming left flips vertically,
            // so the gun stays upright.
            let angle = (-aim.y).atan2(aim.x);
            let rotation = Quat::from_rotation_z(angle);
            // In a hand of the art, the grip of the picture is in the hand,
            // and the rest of it points along the aim.
            if let (Some(_), Some((grip, length))) = (posed, grip) {
                translation += rotation * Vec3::new((0.5 - grip) * length, 0.0, 0.0);
            }
            (rotation, false, aim.x < 0.0)
        } else if let (Some(posed), Some((grip, length))) = (posed, grip) {
            // An item with a grip is held rigidly: it turns as the hand part
            // of the pose turns, so a swing is the character's own row. The
            // picture has its far end to the right; a mirrored body holds its
            // mirror image.
            let rotation = Quat::from_rotation_z(posed.angle());
            translation += rotation * Vec3::new((0.5 - grip) * length, 0.0, 0.0);
            (rotation, false, posed.mirrored)
        } else if let Some((grip, length)) = grip {
            // No hand in the art (a body that is not drawn from parts): the
            // fixed hand, and a turn by a formula.
            // An item with a grip points out of the hand, and a swing turns
            // it about the hand. The picture has its far end to the right;
            // for a holder that faces left it is mirrored and it turns the
            // other way.
            let turn = facing * item_swing_angle(held.swing).to_radians();
            let rotation = Quat::from_rotation_z(turn);
            let to_middle = rotation * Vec3::new(facing * (0.5 - grip) * length, 0.0, 0.0);
            translation += to_middle;
            (rotation, facing < 0.0, false)
        } else {
            (Quat::IDENTITY, facing < 0.0, false)
        };

        let transform = Transform::from_translation(translation).with_rotation(rotation);
        wanted.push((held, room, transform, flip_x, flip_y, false));
    }
    for (entity, drawn, mut transform, mut sprite) in &mut visuals {
        let kept = (!art_replaced)
            .then(|| {
                wanted.iter_mut().find(|(held, room, .., taken)| {
                    !*taken && held.holder.body == drawn.holder && *room == drawn.room && held.item_id == drawn.item_id
                })
            })
            .flatten();
        match kept {
            Some((_, _, wanted_transform, flip_x, flip_y, taken)) => {
                *taken = true;
                transform.set_if_neq(*wanted_transform);
                if (sprite.flip_x, sprite.flip_y) != (*flip_x, *flip_y) {
                    sprite.flip_x = *flip_x;
                    sprite.flip_y = *flip_y;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (held, room, transform, flip_x, flip_y, taken) in wanted {
        if taken {
            continue;
        }
        let bound = resolve_art(
            art.as_deref().map(|art| &art.0),
            failed_art.as_deref().map(|failed| &failed.held),
            held.item_id,
            || "held item".to_owned(),
            &mut reported,
            "held item visual",
        );

        let sprite = bound
            .map(|(image, size)| Sprite {
                image,
                custom_size: Some(size),
                flip_x,
                flip_y,
                ..default()
            })
            .unwrap_or_else(|| {
                let color = match held.item_id {
                    "axe" => Color::srgb(0.72, 0.52, 0.30),
                    "javelin" => Color::srgb(0.86, 0.84, 0.62),
                    _ => Color::srgb(0.82, 0.82, 0.82),
                };
                Sprite::from_color(color, Vec2::new(14.0, 28.0))
            });
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                HeldItemVisual,
                HeldItemDrawn {
                    holder: held.holder.body,
                    room,
                    item_id: held.item_id.to_owned(),
                },
                sprite,
                transform,
                Name::new("Held item visual"),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetApp, AssetPlugin};
    use std::time::{Duration, Instant};

    /// A content reload restates the held art manifest, and the next frame
    /// draws the new art: the handles are resolved again, not only at startup.
    #[test]
    fn a_changed_held_art_manifest_is_resolved_again() {
        use ambition_platformer2d_shared_tangle::held_item_art::{HeldItemArtAppExt, HeldItemArtEntry, HeldItemArtManifest};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .add_systems(Update, resolve_held_item_art_when_the_manifest_changes());
        let axe = |width: f32| HeldItemArtEntry::new("axe", "sprites/props/axe.png", Vec2::new(width, 18.0));
        app.register_held_item_art([axe(40.0)]);
        let drawn = |app: &App| app.world().get_resource::<HeldItemArt>().and_then(|art| art.0.get("axe")).map(|(_, size)| size.x);
        app.update();
        assert_eq!(drawn(&app), Some(40.0), "control: the first manifest is resolved");
        let ids = app
            .world_mut()
            .resource_mut::<HeldItemArtManifest>()
            .restate(["axe"], vec![axe(44.0)]);
        assert_eq!(ids, ["axe"]);
        app.update();
        assert_eq!(drawn(&app), Some(44.0), "the restated manifest was not resolved again");
    }

    /// A content reload restates a provider's pickup art, and the render
    /// draws the new art: the handles are resolved again, not only at startup.
    #[test]
    fn a_changed_world_art_manifest_is_resolved_again() {
        use ambition_platformer2d_shared_tangle::world_item_art::{WorldItemArtAppExt, WorldItemArtEntry, WorldItemArtManifest};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .add_systems(Update, resolve_world_item_art_when_the_manifest_changes());
        let wand = |width: f32| WorldItemArtEntry::new("wand", "sprites/props/wand.png", Vec2::new(width, 18.0));
        app.register_world_item_art([wand(24.0)]);
        let drawn = |app: &App| app.world().get_resource::<WorldItemArt>().and_then(|art| art.0.get("wand")).map(|(_, size)| size.x);
        app.update();
        assert_eq!(drawn(&app), Some(24.0), "control: the first manifest is resolved");
        let ids = app
            .world_mut()
            .resource_mut::<WorldItemArtManifest>()
            .restate(["wand"], vec![wand(32.0)]);
        assert_eq!(ids, ["wand"]);
        app.update();
        assert_eq!(drawn(&app), Some(32.0), "the restated manifest was not resolved again");
    }

    #[test]
    fn missing_image_asset_reaches_the_failed_art_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Image>()
            .init_resource::<FailedItemArt>()
            .add_systems(Update, report_unloadable_item_art);

        let image = app
            .world()
            .resource::<AssetServer>()
            .load::<Image>("__ambition_test_missing__/cinder_beacon.png");
        app.world_mut()
            .insert_resource(WorldItemArt(ArtBindings::new(
                Resolver::<WorldItemSprite>::new(["cinder_beacon"]),
                [(image, Vec2::splat(16.0))],
            )));

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline
            && !app
                .world()
                .resource::<FailedItemArt>()
                .world
                .contains("cinder_beacon")
        {
            app.update();
            std::thread::sleep(Duration::from_millis(1));
        }

        assert!(
            app.world()
                .resource::<FailedItemArt>()
                .world
                .contains("cinder_beacon"),
            "the AssetServer's terminal failure must reach the render fallback state"
        );
    }

    #[test]
    fn failed_bound_art_uses_the_placeholder_path() {
        let art = ArtBindings::new(
            Resolver::<WorldItemSprite>::new(["cinder_beacon"]),
            [(Handle::<Image>::default(), Vec2::splat(16.0))],
        );
        let failed = BTreeSet::from(["cinder_beacon".to_owned()]);
        let mut reported = ReportedOnce::default();

        assert!(
            resolve_art(
                Some(&art),
                Some(&failed),
                "cinder_beacon",
                || "world item `cinder_beacon`".to_owned(),
                &mut reported,
                "world item visual",
            )
            .is_none(),
            "a terminally failed image must select the caller's visible fallback"
        );
        assert!(
            resolve_art(
                Some(&art),
                None,
                "cinder_beacon",
                || "world item `cinder_beacon`".to_owned(),
                &mut reported,
                "world item visual",
            )
            .is_some(),
            "the logical binding remains valid when no asset failure is known"
        );
    }

    #[test]
    fn replacing_art_resets_failed_and_pending_state() {
        let art = ArtBindings::new(
            Resolver::<WorldItemSprite>::new(["new_art"]),
            [(Handle::<Image>::default(), Vec2::splat(16.0))],
        );
        let mut pending = BTreeMap::from([("old_art".to_owned(), Handle::<Image>::default())]);
        let mut failed = BTreeSet::from(["old_art".to_owned()]);

        reset_art_watch(Some(&art), &mut pending, &mut failed);

        assert!(failed.is_empty());
        assert_eq!(
            pending.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["new_art"]
        );
    }

    /// View half, cut V2c: a world item is drawn in its own live room. Two
    /// live rooms of different sizes, a row in each at one simulation
    /// position: each visual takes its own room's flip and is stamped with
    /// its room, so the room's band and retirement take it. A row whose room
    /// cannot be told is not drawn. Before V2c this road read the sole live
    /// room and drew nothing while two rooms were live.
    #[test]
    fn each_world_item_is_drawn_in_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let at = ambition_platformer2d_core::Vec2::new(100.0, 200.0);
        let size = |w: f32, h: f32| ambition_platformer2d_core::Vec2::new(w, h);
        let geometry = |size| {
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "item room",
                size,
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ))
        };
        let (big, small) = (size(800.0, 600.0), size(400.0, 300.0));
        let second = LiveRoomInstance::ACTIVATION.next();
        let row = |room| ambition_sim_view::WorldItemFact {
            pos: at,
            half_extent: size(8.0, 8.0),
            row_id: "grow_cap".to_string(),
            sprite: None,
            emerging: false,
            room,
        };
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry(big));
        ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(app.world_mut(), second, geometry(small));
        app.insert_resource(ambition_sim_view::WorldItemsView(vec![
            row(Some(LiveRoomInstance::ACTIVATION)),
            row(Some(second)),
            row(None),
        ]));
        app.add_systems(Update, sync_world_item_visuals);
        app.update();
        let mut drawn: Vec<(LiveRoomInstance, Vec2)> = app
            .world_mut()
            .query_filtered::<(&InRoomInstance, &Transform), With<WorldItemVisual>>()
            .iter(app.world())
            .map(|(room, transform)| (room.0, transform.translation.truncate()))
            .collect();
        drawn.sort_by_key(|(room, _)| *room);
        let flip = |room: ambition_platformer2d_core::Vec2| Vec2::new(at.x - room.x * 0.5, room.y * 0.5 - at.y);
        assert_eq!(
            drawn,
            vec![(LiveRoomInstance::ACTIVATION, flip(big)), (second, flip(small))],
            "a world item was not drawn in its own live room, or a row with no room was drawn"
        );
    }

    /// A ground item's visual is kept: a still row keeps its entity and its
    /// transform is not written, a moved row moves it, and a row that is gone
    /// takes its visual with it. Until 2026-10-23 every visual was spawned
    /// again each frame.
    #[test]
    fn a_ground_item_visual_is_kept_while_its_row_stays() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let geometry = ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "item room",
            ambition_platformer2d_core::Vec2::new(800.0, 600.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ));
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry);
        let item = app.world_mut().spawn_empty().id();
        let row = |x: f32| ambition_sim_view::GroundItemFact {
            item,
            pos: ambition_platformer2d_core::Vec2::new(x, 200.0),
            half_extent: ambition_platformer2d_core::Vec2::new(8.0, 8.0),
            item_id: "key".to_string(),
            room: Some(LiveRoomInstance::ACTIVATION),
            roll: 0.0,
        };
        app.insert_resource(GroundItemsView(vec![row(100.0)]));
        app.add_systems(Update, sync_ground_item_visuals);
        app.update();
        // The tick of the transform's last write: `is_changed` read from
        // outside a system is false after every `update`.
        let visual = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(Entity, Ref<Transform>), With<GroundItemVisual>>()
                .iter(app.world())
                .map(|(entity, transform)| (entity, transform.translation.x, transform.last_changed()))
                .collect::<Vec<_>>()
        };
        let first = visual(&mut app);
        assert_eq!(first.len(), 1, "one row, one visual");
        app.update();
        let still = visual(&mut app);
        assert_eq!((still[0].0, still[0].2), (first[0].0, first[0].2), "a still row keeps its visual and writes nothing");
        app.world_mut().resource_mut::<GroundItemsView>().0 = vec![row(140.0)];
        app.update();
        let moved = visual(&mut app);
        assert_eq!(moved[0].0, first[0].0, "a moved row keeps its visual");
        assert_eq!(moved[0].1 - first[0].1, 40.0, "and moves it");
        app.world_mut().resource_mut::<GroundItemsView>().0.clear();
        app.update();
        assert!(visual(&mut app).is_empty(), "a row that is gone takes its visual");
    }

    /// A held item's visual is kept while its holder holds it in the room: its
    /// entity survives a frame and is not written; a turn flips it in place,
    /// and an item put away takes it. It was despawned and spawned each frame.
    #[test]
    fn a_held_item_visual_is_kept_while_its_holder_holds_it() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let geometry = ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "item room",
            ambition_platformer2d_core::Vec2::new(800.0, 600.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ));
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry);
        let body = app.world_mut().spawn_empty().id();
        let row = |facing: f32| ambition_sim_view::HeldItemFact {
            pos: ambition_platformer2d_core::Vec2::new(100.0, 200.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 32.0),
            facing,
            item_id: "axe".to_string(),
            ranged: false,
            aim: ambition_platformer2d_core::Vec2::ZERO,
            swing: None,
            room: Some(LiveRoomInstance::ACTIVATION),
            holder: ambition_sim_view::ItemHolder { body, feature: None, hand: Default::default() },
        };
        app.insert_resource(HeldItemView(vec![row(1.0)]));
        app.init_resource::<ambition_sim_view::HostileWieldedItemsView>();
        app.add_systems(Update, sync_held_item_visual);
        app.update();
        let visual = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(Entity, Ref<Transform>, &Sprite), With<HeldItemVisual>>()
                .iter(app.world())
                .map(|(entity, transform, sprite)| (entity, transform.last_changed(), sprite.flip_x))
                .collect::<Vec<_>>()
        };
        let first = visual(&mut app);
        assert_eq!(first.len(), 1, "one held item, one visual");
        app.update();
        assert_eq!(visual(&mut app), first, "a held item keeps its visual and writes nothing");
        app.world_mut().resource_mut::<HeldItemView>().0 = vec![row(-1.0)];
        app.update();
        let turned = visual(&mut app);
        assert_eq!((turned[0].0, turned[0].2), (first[0].0, true), "a turn flips the same visual");
        app.world_mut().resource_mut::<HeldItemView>().0.clear();
        app.update();
        assert!(visual(&mut app).is_empty(), "an item put away takes its visual");
    }

    /// A world item's visual is kept while a row of its room and look stays:
    /// its entity survives a frame, and a row that is gone takes it.
    #[test]
    fn a_world_item_visual_is_kept_while_its_row_stays() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let geometry = ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
            "item room",
            ambition_platformer2d_core::Vec2::new(800.0, 600.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ));
        let mut app = App::new();
        let mut active = ActiveSessionScope::default();
        active.begin();
        app.insert_resource(active);
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), geometry);
        let row = |x: f32| ambition_sim_view::WorldItemFact {
            pos: ambition_platformer2d_core::Vec2::new(x, 200.0),
            half_extent: ambition_platformer2d_core::Vec2::new(8.0, 8.0),
            row_id: "grow_cap".to_string(),
            sprite: None,
            emerging: false,
            room: Some(LiveRoomInstance::ACTIVATION),
        };
        app.insert_resource(ambition_sim_view::WorldItemsView(vec![row(100.0)]));
        app.add_systems(Update, sync_world_item_visuals);
        app.update();
        let visual = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(Entity, &Transform), With<WorldItemVisual>>()
                .iter(app.world())
                .map(|(entity, transform)| (entity, transform.translation.x))
                .collect::<Vec<_>>()
        };
        let first = visual(&mut app);
        assert_eq!(first.len(), 1);
        app.world_mut().resource_mut::<ambition_sim_view::WorldItemsView>().0 = vec![row(130.0)];
        app.update();
        let moved = visual(&mut app);
        assert_eq!(moved, vec![(first[0].0, first[0].1 + 30.0)], "a moving row keeps its visual and moves it");
        app.world_mut().resource_mut::<ambition_sim_view::WorldItemsView>().0.clear();
        app.update();
        assert!(visual(&mut app).is_empty(), "a row that is gone takes its visual");
    }
}
