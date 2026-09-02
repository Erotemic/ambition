//! Item visuals (the item pickup presentation tail):
//! ground-item quads, the held-item sprite, and held-projectile sprites. Pure
//! consumers of the sim-built `sim_view` item snapshots (E4 slices 11+12+16)
//! — no live item/body queries.

use ambition_platformer2d_shared_tangle::binding::{
    log_unresolved, Namespace, ReportedOnce, Resolver, UnresolvedRef,
};
use ambition_platformer2d_shared_tangle::held_item_art::HeldItemSprite;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::world_item_art::WorldItemSprite;
use ambition_sim_view::{GroundItemsView, HeldItemView};
use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

// Presentation (visible build only).

/// Provider-contributed art, resolved once at startup: the ids that bound, and
/// the loaded `(image, display size)` for each, indexed by the binding's slot.
///
/// A miss now yields a diagnosis the caller can print. The placeholder still
/// draws — that part was right, and a blind run must never go black — but the
/// run also names what it could not find.
///
/// It does NOT check that the images arrive. `AssetServer::load` returns a
/// handle for a path that does not exist, so an id can bind perfectly to art
/// that will never draw; that is the cinder beacon failure, and it belongs to
/// [`report_unloadable_item_art`].
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
    /// Pair a manifest's resolver with the handles loaded from the SAME effective
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

    /// Every registered id beside the image handle it loaded, for the pass that
    /// checks the FILES arrived — see [`report_unloadable_item_art`].
    ///
    /// Through `declarations`, not `ids`: the resolver is sorted for lookup and
    /// the handles are in declaration order, so zipping the two directly would
    /// name the wrong id for a failed image.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Handle<Image>)> {
        self.ids
            .declarations()
            .map(|(id, slot)| (id, &self.art[slot].0))
    }
}

/// Registered art ids whose image asset reached a terminal failed state.
///
/// The binding itself remains valid — the manifest and consumer agree on the
/// id — but renderers must use their visible placeholder instead of continuing
/// to submit a sprite backed by a handle that can never produce an image.
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
/// The shape every per-frame consumer of an [`ArtBindings`] wants: draw the
/// placeholder either way, say what is wrong the first time, and stop spending
/// anything on a defect already on the record.
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

/// Re-arm the watch because the art resource was replaced: every id is unsettled
/// again, and a failure recorded against the OLD manifest is not evidence about
/// the new one.
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

/// Drain the watch: anything that has settled leaves it, and anything that
/// settled as a FAILURE is named once and remembered so the render path can
/// choose the placeholder over a handle that will never produce a picture.
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

/// Say so when a bound art id's IMAGE never arrives — and stop drawing it.
///
/// This is the other half of the cinder beacon, and the half a resolver cannot
/// see. That pickup drew nothing for weeks with its id correctly registered: the
/// manifest named `sprites/props/super_mary_o_cinder_beacon.png`, no generator
/// produced the file, `AssetServer::load` handed back a handle regardless — a
/// handle is a promise, not a picture — and the binding resolved perfectly into
/// art that would never exist. An id namespace can only ever prove that content
/// agrees with content. Whether the FILE showed up is a separate question, and
/// this is where it gets asked.
///
/// Naming it is not enough on its own: a bound id whose image failed would still
/// take the sprite branch and draw nothing, which is the same invisible pickup
/// with a log line beside it. The ids that settle as failures land in
/// [`FailedItemArt`], and the render path treats them as unresolved so the
/// placeholder quad comes back. Draw blind, but visibly.
///
/// Each entry is probed only until it settles — loaded and failed handles both
/// leave the watch — so this costs nothing once a room's art is resolved.
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

/// Marks a sprite entity visualizing a [`GroundItem`].
#[derive(Component)]
pub struct GroundItemVisual;

/// Loaded held/inventory item art, resolved from every provider's
/// [`HeldItemArtManifest`](ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest):
/// held-item spec id → `(image, on-screen display size)`. The engine owns the
/// SEAM (this resource + [`build_held_item_art`] + the resolve in the sync
/// systems); each game contributes its own props' images (axe / javelin /
/// gun-sword / wielded-gauntlet icons) without a render dependency, keeping asset
/// knowledge out of the reusable renderer. Absent / unmatched  the placeholder
/// quad.
#[derive(Resource, Default)]
pub struct HeldItemArt(pub ArtBindings<HeldItemSprite>);

/// Resolve every provider-contributed
/// [`HeldItemArtEntry`](ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtEntry)
/// (pure `id → path + size` data) into loaded image handles at startup, filling
/// [`HeldItemArt`]. The render half of the contribution seam: games declare their
/// held-item art without a render dependency; the resolution — and the
/// `AssetServer` — lives HERE, so a multi-game host's unioned manifest binds every
/// provider's props at once. Absent manifest  an empty map (every item draws the
/// quad fallback).
pub fn build_held_item_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    manifest: Option<Res<ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest>>,
) {
    let art = match manifest {
        Some(manifest) => HeldItemArt(ArtBindings::new(
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
        )),
        None => HeldItemArt::default(),
    };
    commands.insert_resource(art);
}

pub fn sync_ground_item_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
    failed_art: Option<Res<FailedItemArt>>,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<GroundItemVisual>>,
    grounds: Res<GroundItemsView>,
    // This system rebuilds every frame, so a missing id would otherwise be
    // sixty identical log lines a second.
    mut reported: Local<ReportedOnce>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // A replaced manifest is different content, and "we already said that" about
    // content that no longer exists would silence a live defect.
    if art.as_ref().is_some_and(|art| art.is_changed()) {
        reported.clear();
    }
    for ground in &grounds.0 {
        let translation =
            ambition_platformer2d_core::config::world_to_bevy(&world.0, ground.pos, 8.0);
        let bound = resolve_art(
            art.as_deref().map(|art| &art.0),
            failed_art.as_deref().map(|failed| &failed.held),
            ground.item_id.as_str(),
            || "ground item".to_owned(),
            &mut reported,
            "ground item visual",
        );
        // The placeholder still draws; the ledger is what stops it being silent.
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
            session_scope,
            (
                GroundItemVisual,
                sprite,
                Transform::from_translation(translation),
                Name::new("Ground item visual"),
            ),
        );
    }
}

/// Marks a sprite entity visualizing a [`WorldItem`](ambition_platformer2d_actor_monolith::items::world_item::WorldItem).
#[derive(Component)]
pub struct WorldItemVisual;

/// Game-supplied art for walk-into world items, keyed by the presentation `sprite`
/// id a [`WorldItem`](ambition_platformer2d_actor_monolith::items::world_item::WorldItem) carries →
/// `(image, on-screen display size)`. The engine owns the SEAM (this resource + the
/// resolve in [`sync_world_item_visuals`]); each game fills it at startup with its
/// own pickups' images (e.g. Mary-O's star wand), keeping asset knowledge out of
/// the reusable renderer. Absent / unmatched  the row-tinted placeholder quad.
#[derive(Resource, Default)]
pub struct WorldItemArt(pub ArtBindings<WorldItemSprite>);

/// Resolve the provider-contributed
/// [`WorldItemArtManifest`](ambition_platformer2d_shared_tangle::world_item_art::WorldItemArtManifest)
/// (pure `id → path + size` data every game registered at build time) into loaded
/// image handles, filling [`WorldItemArt`]. This is the render half of the
/// contribution seam: games declare their pickup art without a render dependency;
/// the resolution — and the `AssetServer` — lives HERE, so a multi-game host's
/// unioned manifest binds every provider's pickups at once. Absent manifest  an
/// empty map (every item draws the quad fallback).
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

/// A sprite per walk-into world item: the real image when the item carries a
/// `sprite` id bound in [`WorldItemArt`], else a colored quad tinted by the row it
/// grants (star wand = gold, cinder beacon = ember, unknown = magenta) — the
/// draw-blind fallback. Clear-and-rebuild each frame — few items — mirroring
/// [`sync_ground_item_visuals`].
pub fn sync_world_item_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    art: Option<Res<WorldItemArt>>,
    failed_art: Option<Res<FailedItemArt>>,
    visuals: Query<Entity, With<WorldItemVisual>>,
    items: Res<ambition_sim_view::WorldItemsView>,
    mut reported: Local<ReportedOnce>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    if art.as_ref().is_some_and(|art| art.is_changed()) {
        reported.clear();
    }
    for item in &items.0 {
        // an emerging pickup draws BEHIND the world, so it reads as coming
        // out of the block that produced it instead of being pasted whole on top
        // of it. `WORLD_Z_BLOCK` is 0.0, so anything below it
        // is occluded by the geometry; a free item keeps the ordinary 8.0.
        let z = if item.emerging { -1.0 } else { 8.0 };
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, item.pos, z);
        // A real bound sprite wins; otherwise the row-tinted quad. An item that
        // declares NO sprite id is authored that way and reports nothing; an item
        // that declares one nobody registered is reported, once.
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
                sprite,
                Transform::from_translation(translation),
                Name::new("World item visual"),
            ),
        );
    }
}

/// Marks the sprite shown in the player's hand for the currently held item.
#[derive(Component)]
pub struct HeldItemVisual;

/// Draw a small quad in the CONTROLLED SUBJECT's hand for whatever they're
/// holding, tinted per item (axe / javelin). Clear-and-rebuild each frame.
///
/// Keyed on [`ControlledSubject`] (the body holding `DrivingParticipant(PRIMARY)`),
/// not `PrimaryPlayer`: while possessing, the held-item sprite draws on the body
/// you are DRIVING (reading ITS own `HeldItem`), never lingering on the vacated
/// home avatar — the same rule the blink reticle, camera, and nameplate follow.
/// Aim comes from the subject's brain-resolved `ActorControl`, not raw device
/// input, so a possessed body's ranged item points where THAT body aims.
pub fn sync_held_item_visual(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
    failed_art: Option<Res<FailedItemArt>>,
    active_session: Option<Res<ActiveSessionScope>>,
    held_view: Res<HeldItemView>,
    visuals: Query<Entity, With<HeldItemVisual>>,
    mut reported: Local<ReportedOnce>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // ⭐ ONE VISUAL PER DRIVEN HOLDER. This read a single `Option`, so a couch
    // match drew seat zero's weapon and nothing else; the view is plural now and
    // ordered by `SimId`, so the draw order is the same every run.
    for held in &held_view.0 {
        let facing = if held.facing >= 0.0 { 1.0 } else { -1.0 };
        // In the subject's hand: just in front at hand height (y-down → small +y).
        let hand = held.pos + Vec2::new(facing * (held.size.x * 0.45 + 4.0), held.size.y * 0.06);
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, hand, 12.0);

        // A ranged held item (the gun-sword) points where you're AIMING — the same
        // direction it fires — just like the pirates' wielded gun-sword. Melee /
        // thrown items keep the simple facing flip. Aim is the subject's
        // brain-resolved frame (screen-relative fallback to facing), so a possessed
        // body's item tracks ITS aim, not the home avatar's device stick.
        let (rotation, flip_x, flip_y) = if held.ranged {
            let aim = if held.aim.length_squared() > 1e-4 {
                held.aim.normalize()
            } else {
                Vec2::new(held.facing, 0.0)
            };
            // World is y-down, render space y-up. Aiming left flips vertically so
            // the gun stays upright instead of rotating upside-down.
            let angle = (-aim.y).atan2(aim.x);
            (Quat::from_rotation_z(angle), false, aim.x < 0.0)
        } else {
            (Quat::IDENTITY, facing < 0.0, false)
        };

        if art.as_ref().is_some_and(|art| art.is_changed()) {
            reported.clear();
        }
        let bound = resolve_art(
            art.as_deref().map(|art| &art.0),
            failed_art.as_deref().map(|failed| &failed.held),
            held.item_id.as_str(),
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
                let color = match held.item_id.as_str() {
                    "axe" => Color::srgb(0.72, 0.52, 0.30),
                    "javelin" => Color::srgb(0.86, 0.84, 0.62),
                    _ => Color::srgb(0.82, 0.82, 0.82),
                };
                Sprite::from_color(color, Vec2::new(14.0, 28.0))
            });
        commands.spawn_session_scoped(
            session_scope,
            (
                HeldItemVisual,
                sprite,
                Transform::from_translation(translation).with_rotation(rotation),
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
}
