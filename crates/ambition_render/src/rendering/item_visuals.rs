//! Item visuals (the item pickup presentation tail):
//! ground-item quads, the held-item sprite, and held-projectile sprites. Pure
//! consumers of the sim-built `sim_view` item snapshots (E4 slices 11+12+16)
//! — no live item/body queries.

use ambition_platformer_primitives::binding::{
    log_unresolved, Namespace, ReportedOnce, Resolver, UnresolvedRef,
};
use ambition_platformer_primitives::held_item_art::HeldItemSprite;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer_primitives::world_item_art::WorldItemSprite;
use ambition_sim_view::{GroundItemsView, HeldItemView, HeldShotsView};

const FIREBALL_ID: &str = "fireball";
use bevy::prelude::*;
use bevy::sprite::Anchor;

// --- Wielded / held gun-sword sprite ---------------------------------------
// The spinning-lasersword sprite is the WIELDED weapon + the player's held
// gun-sword shot (a `HeldProjectile`), distinct from the faction projectile
// pool (whose art now flows through the `ProjectileVisualCatalog`). These consumers
// keep their own static-frame helper.
//
// (Read from `lasersword_spritesheet.yaml`, row `idle`, frame 0.)
const LASERSWORD_SHEET_PATH: &str = "sprites/lasersword_spritesheet.png";
const LASERSWORD_LABEL_W: f32 = 110.0;
const LASERSWORD_FRAME_W: f32 = 169.0;
const LASERSWORD_FRAME_H: f32 = 44.0;
const LASERSWORD_IDLE_FRAME_X: f32 = LASERSWORD_LABEL_W;
const LASERSWORD_IDLE_FRAME_Y: f32 = 0.0;
/// Pommel anchor in the idle frame — rotation pivot of the sprite.
const LASERSWORD_POMMEL_X_PX: f32 = 14.0;
const LASERSWORD_POMMEL_Y_PX: f32 = 22.0;
const LASERSWORD_RENDER_WIDTH: f32 = 56.0;

/// Spritesheet path for the wielded / held gun-sword sprite.
pub const LASERSWORD_SHEET: &str = LASERSWORD_SHEET_PATH;

/// Build the lasersword sprite (idle frame, pommel-anchored) + its z-rotation
/// for a shot traveling at `vel` (world space, y-down). Used by the wielded
/// weapon and the player's held gun-sword shot so both render an identical
/// spinning sword aligned to its velocity.
///
/// AMBITION_REVIEW(spatial): Bevy +Y is up, sim +Y is down — flip Y for the
/// rotation; the pommel anchor is normalized from frame-local px (y negated).
pub fn lasersword_projectile_sprite(
    texture: Handle<Image>,
    vel: ambition_engine_core::Vec2,
) -> (Sprite, Anchor, Quat) {
    let bevy_dx = vel.x;
    let bevy_dy = -vel.y;
    let angle = if bevy_dx == 0.0 && bevy_dy == 0.0 {
        0.0
    } else {
        bevy_dy.atan2(bevy_dx)
    };
    let aspect = LASERSWORD_FRAME_W / LASERSWORD_FRAME_H;
    let render = Vec2::new(LASERSWORD_RENDER_WIDTH, LASERSWORD_RENDER_WIDTH / aspect);
    let anchor_x_norm = (LASERSWORD_POMMEL_X_PX - LASERSWORD_FRAME_W * 0.5) / LASERSWORD_FRAME_W;
    let anchor_y_norm = -(LASERSWORD_POMMEL_Y_PX - LASERSWORD_FRAME_H * 0.5) / LASERSWORD_FRAME_H;
    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(render);
    sprite.rect = Some(Rect::from_corners(
        Vec2::new(LASERSWORD_IDLE_FRAME_X, LASERSWORD_IDLE_FRAME_Y),
        Vec2::new(
            LASERSWORD_IDLE_FRAME_X + LASERSWORD_FRAME_W,
            LASERSWORD_IDLE_FRAME_Y + LASERSWORD_FRAME_H,
        ),
    ));
    (
        sprite,
        Anchor(Vec2::new(anchor_x_norm, anchor_y_norm)),
        Quat::from_rotation_z(angle),
    )
}

// Presentation (visible build only).

/// Provider-contributed art, resolved once at startup: the ids that bound, and
/// the loaded `(image, display size)` for each, indexed by the binding's slot.
///
/// This replaces the `HashMap<String, _>` both art resources used to be. The map
/// was not wrong about storage — it was wrong about FAILURE. Its `get` returned
/// `None` for "you misspelled the id" and for "no provider ever registered art",
/// and every caller collapsed both into the same placeholder quad.
///
/// A miss now yields a diagnosis the caller can print. The placeholder still
/// draws — that part was right, and a blind run must never go black — but the
/// run also names what it could not find.
///
/// It does NOT check that the images arrive. `AssetServer::load` returns a
/// handle for a path that does not exist, so an id can bind perfectly to art
/// that will never draw; that is the spark-blossom failure, and it belongs to
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
    ///
    /// Says nothing about WHY on purpose: these syncs run every frame, and the
    /// why — cloning every registered id, running a did-you-mean across all of
    /// them — costs orders of magnitude more than the lookup. A caller that has
    /// not yet reported this failure asks [`Self::explain`] for it; a caller
    /// that already has just draws the placeholder again, for free.
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

/// Resolve `id` through `art`, reporting a miss at most once per distinct
/// failure and paying for the diagnostic only then.
///
/// The shape every per-frame consumer of an [`ArtBindings`] wants: draw the
/// placeholder either way, say what is wrong the first time, and stop spending
/// anything on a defect already on the record.
fn resolve_art<N: Namespace>(
    art: Option<&ArtBindings<N>>,
    id: &str,
    declared_by: impl FnOnce() -> String,
    reported: &mut ReportedOnce,
    context: &str,
) -> Option<(Handle<Image>, Vec2)> {
    let art = art?;
    if let Some(found) = art.get(id) {
        return Some(found);
    }
    let declared_by = declared_by();
    if reported.first_sight(N::NAME, &declared_by, id) {
        log_unresolved(context, &art.explain(id, declared_by));
    }
    None
}

/// Say so when a bound art id's IMAGE never arrives.
///
/// This is the other half of the spark blossom, and the half a resolver cannot
/// see. That pickup drew nothing for weeks with its id correctly registered:
/// the manifest named `sprites/props/super_mary_o_spark_blossom.png`, no
/// generator produced the file, `AssetServer::load` handed back a handle
/// regardless — a handle is a promise, not a picture — and the binding resolved
/// perfectly into art that would never exist. An id namespace can only ever
/// prove that content agrees with content. Whether the FILE showed up is a
/// separate question, and this is where it gets asked.
///
/// Runs every frame and costs a load-state probe per registered entry until each
/// one settles. A failure is named once, with its path, because that is what a
/// reader needs: the id points at the manifest line, the path at the generator.
pub fn report_unloadable_item_art(
    assets: Res<AssetServer>,
    world_art: Option<Res<WorldItemArt>>,
    held_art: Option<Res<HeldItemArt>>,
    mut reported: Local<std::collections::BTreeSet<String>>,
) {
    let world = world_art.as_deref().map(|art| &art.0);
    let held = held_art.as_deref().map(|art| &art.0);
    let entries = world
        .into_iter()
        .flat_map(ArtBindings::entries)
        .map(|entry| ("world item art", entry))
        .chain(
            held.into_iter()
                .flat_map(ArtBindings::entries)
                .map(|entry| ("held item art", entry)),
        );
    for (context, (id, image)) in entries {
        if !matches!(assets.load_state(image), bevy::asset::LoadState::Failed(_)) {
            continue;
        }
        if !reported.insert(format!("{context}/{id}")) {
            continue;
        }
        let path = assets
            .get_path(image.id())
            .map(|path| path.to_string())
            .unwrap_or_else(|| "<no path>".to_owned());
        error!(
            "{context}: `{id}` is registered and bound, but its image `{path}` failed to load — \
             the id is fine and the FILE is missing or unreadable (check the generator target)",
        );
    }
}

/// Marks a sprite entity visualizing a [`GroundItem`].
#[derive(Component)]
pub struct GroundItemVisual;

/// Loaded held/inventory item art, resolved from every provider's
/// [`HeldItemArtManifest`](ambition_platformer_primitives::held_item_art::HeldItemArtManifest):
/// held-item spec id → `(image, on-screen display size)`. The engine owns the
/// SEAM (this resource + [`build_held_item_art`] + the resolve in the sync
/// systems); each game contributes its own props' images (axe / javelin /
/// gun-sword / wielded-gauntlet icons) without a render dependency, keeping asset
/// knowledge out of the reusable renderer. Absent / unmatched ⇒ the placeholder
/// quad.
#[derive(Resource, Default)]
pub struct HeldItemArt(pub ArtBindings<HeldItemSprite>);

/// Resolve every provider-contributed
/// [`HeldItemArtEntry`](ambition_platformer_primitives::held_item_art::HeldItemArtEntry)
/// (pure `id → path + size` data) into loaded image handles at startup, filling
/// [`HeldItemArt`]. The render half of the contribution seam: games declare their
/// held-item art without a render dependency; the resolution — and the
/// `AssetServer` — lives HERE, so a multi-game host's unioned manifest binds every
/// provider's props at once. Absent manifest ⇒ an empty map (every item draws the
/// quad fallback).
pub fn build_held_item_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    manifest: Option<Res<ambition_platformer_primitives::held_item_art::HeldItemArtManifest>>,
) {
    let art = match manifest {
        Some(manifest) => HeldItemArt(ArtBindings::new(
            manifest.item_ids(),
            manifest.effective().into_iter().map(|entry| {
                (
                    assets.load(entry.asset_path.clone()),
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
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
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
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, ground.pos, 8.0);
        let bound = resolve_art(
            art.as_deref().map(|art| &art.0),
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

/// Marks a sprite entity visualizing a [`WorldItem`](ambition_actors::items::world_item::WorldItem).
#[derive(Component)]
pub struct WorldItemVisual;

/// Game-supplied art for walk-into world items, keyed by the presentation `sprite`
/// id a [`WorldItem`](ambition_actors::items::world_item::WorldItem) carries →
/// `(image, on-screen display size)`. The engine owns the SEAM (this resource + the
/// resolve in [`sync_world_item_visuals`]); each game fills it at startup with its
/// own pickups' images (e.g. Mary-O's milk carton), keeping asset knowledge out of
/// the reusable renderer. Absent / unmatched ⇒ the row-tinted placeholder quad.
#[derive(Resource, Default)]
pub struct WorldItemArt(pub ArtBindings<WorldItemSprite>);

/// Resolve the provider-contributed
/// [`WorldItemArtManifest`](ambition_platformer_primitives::world_item_art::WorldItemArtManifest)
/// (pure `id → path + size` data every game registered at build time) into loaded
/// image handles, filling [`WorldItemArt`]. This is the render half of the
/// contribution seam: games declare their pickup art without a render dependency;
/// the resolution — and the `AssetServer` — lives HERE, so a multi-game host's
/// unioned manifest binds every provider's pickups at once. Absent manifest ⇒ an
/// empty map (every item draws the quad fallback).
pub fn build_world_item_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    manifest: Option<Res<ambition_platformer_primitives::world_item_art::WorldItemArtManifest>>,
) {
    let art = match manifest {
        Some(manifest) => WorldItemArt(ArtBindings::new(
            manifest.sprite_ids(),
            manifest.effective().into_iter().map(|entry| {
                (
                    assets.load(entry.asset_path.clone()),
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
/// grants (grow-cap = cream, spark-blossom = ember, unknown = magenta) — the
/// draw-blind fallback. Clear-and-rebuild each frame — few items — mirroring
/// [`sync_ground_item_visuals`].
pub fn sync_world_item_visuals(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    art: Option<Res<WorldItemArt>>,
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
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, item.pos, 8.0);
        // A real bound sprite wins; otherwise the row-tinted quad. An item that
        // declares NO sprite id is authored that way and reports nothing; an item
        // that declares one nobody registered is reported, once.
        let bound = item.sprite.as_deref().and_then(|id| {
            resolve_art(
                art.as_deref().map(|art| &art.0),
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
                    "grow_cap" => Color::srgb(0.95, 0.93, 0.82),
                    "spark_blossom" => Color::srgb(0.95, 0.55, 0.20),
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
/// Keyed on [`ControlledSubject`] (the body carrying `Brain::Player(PRIMARY)`),
/// not `PrimaryPlayer`: while possessing, the held-item sprite draws on the body
/// you are DRIVING (reading ITS own `HeldItem`), never lingering on the vacated
/// home avatar — the same rule the blink reticle, camera, and nameplate follow.
/// Aim comes from the subject's brain-resolved `ActorControl`, not raw device
/// input, so a possessed body's ranged item points where THAT body aims.
pub fn sync_held_item_visual(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    art: Option<Res<HeldItemArt>>,
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
    let Some(held) = held_view.0.as_ref() else {
        return;
    };
    let facing = if held.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the subject's hand: just in front at hand height (y-down → small +y).
    let hand = held.pos + Vec2::new(facing * (held.size.x * 0.45 + 4.0), held.size.y * 0.06);
    let translation = ambition_engine_core::config::world_to_bevy(&world.0, hand, 12.0);

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

/// Texture handles used by held-shot visuals. Kept alive in system-local state
/// so the per-frame clear/rebuild visual path does not also re-request and
/// repeatedly decode projectile sprite PNGs.
pub struct HeldProjectileVisualArt {
    lasersword: Handle<Image>,
    fireball: Handle<Image>,
}

impl HeldProjectileVisualArt {
    fn load(asset_server: &AssetServer) -> Self {
        Self {
            lasersword: asset_server.load(LASERSWORD_SHEET),
            fireball: asset_server.load(format!("sprites/props/gauntlet_{FIREBALL_ID}.png")),
        }
    }
}

/// Marks the streak sprite for an in-flight [`HeldProjectile`] (laser bolt).
#[derive(Component)]
pub struct HeldProjectileVisual;

/// Render each in-flight held shot. Fireballs draw as a glowing sphere; other
/// shots reuse the spinning lasersword sprite and rotate along travel.
pub fn sync_held_projectile_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<HeldProjectileVisual>>,
    shots: Res<HeldShotsView>,
    mut art: Local<Option<HeldProjectileVisualArt>>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let art = art.get_or_insert_with(|| HeldProjectileVisualArt::load(&asset_server));
    for shot in &shots.0 {
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, shot.pos, 9.5);
        if shot.fireball {
            // Fireball: a glowing ball, sized a touch over the contact box so the
            // fire visibly fills the space that hits. No rotation — it's radial.
            commands.spawn_session_scoped(
                session_scope,
                (
                    HeldProjectileVisual,
                    Sprite {
                        image: art.fireball.clone(),
                        custom_size: Some(Vec2::splat(30.0)),
                        ..default()
                    },
                    Transform::from_translation(translation),
                    Name::new("Fireball shot"),
                ),
            );
            continue;
        }
        let (sprite, anchor, rotation) =
            lasersword_projectile_sprite(art.lasersword.clone(), shot.vel);
        commands.spawn_session_scoped(
            session_scope,
            (
                HeldProjectileVisual,
                sprite,
                anchor,
                Transform {
                    translation,
                    rotation,
                    scale: Vec3::ONE,
                },
                Name::new("Gun-sword laser shot"),
            ),
        );
    }
}
