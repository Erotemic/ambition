//! A portal pair carries what is drawn into its entry out of its exit: every
//! kind of drawable, by one road
//! (`docs/planning/engine/portals-a-body-in-two-places.md`, "One road for
//! every drawable").
//!
//! Selection is by what goes through; the drawing is by geometry.
//!
//! * A body's own drawable ([`crate::PortalSceneBody`]) goes through while its
//!   body is in transit: the host publishes the pair it straddles
//!   ([`crate::PortalTransitView`]). A body that only overlaps an opening (a
//!   giant whose arm's near line is over its back) is not in transit, and its
//!   picture stays.
//! * Any other drawable (worn gear, a held item, a flash, a trail, a slash,
//!   dust, a spark, a bolt) goes through an opening when it reaches past the
//!   entry plane, inside the opening, from the front: the rule a combat volume
//!   is cut by. It moves through square on from its own centre, or reaches
//!   from the point it says
//!   ([`ambition_platformer2d_shared_tangle::portal_passages::ReachesFrom`]: a
//!   swing, from the body that swings it), and then goes through in the wedge
//!   from that point through the opening's ends, as its hits do
//!   ([`ae::frame::thread::partition_reaching`]). No kind opts in, so a new
//!   kind is carried when it is made.
//! * A drawable that is part of another's picture ([`PortalDrawnWith`], and
//!   each child) goes with that one.
//!
//! Each pair a drawable goes through has a carry camera. While a drawable
//! goes through, it is taken off the world's layers and put on that camera's
//! layer only ([`CarriedLayers`], given back at the start of the next frame),
//! and the camera draws it into a texture. That texture is drawn back into
//! the world cut by the pair: its part in front of the entry where it is, and
//! its part past the entry, inside the opening, out of the exit
//! ([`ae::frame::thread::partition`], the one partition combat uses). A
//! sprite, a mesh, a particle or a text is drawn the same way.
//!
//! ⛔ The carry runs in `PostUpdate`, after every `Update` writer of a
//! drawable's transform, and BEFORE the assets' events, the cameras and the
//! transform propagation: a clip quad's material changed after the assets'
//! events reached the renderer one frame late, and on the frame a body
//! crossed its pieces were cut by the planes of the frame before (the robot
//! drawn whole, under the floor, at the exit). So it reads each drawable's
//! own `Transform` (a top-level drawable's is its place in the world) and its
//! bounds as last computed.

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::camera_layers::{
    PORTAL_CARRY_LAYER_BASE, PORTAL_CARRY_LAYERS, LIVE_ROOM_RENDER_LAYER_BASE, LIVE_ROOM_RENDER_LAYER_LAST,
};

use ae::frame::thread::{partition, partition_reaching, PieceBudget, PortalLink, VolumePiece};
use ambition_portal2d::{find_portal, PlacedPortal};

use crate::clip_material::{clip_piece_transform, clip_plane_render, ClipQuadPool, PortalClipMaterial, CLIP_PLANES};
use crate::{PortalBodyPiece, PortalFrames, PortalSceneBody, PortalWorldFrame};

/// The drawable is part of the picture of another (a body's parts, the parts
/// drawn over its limbs): it goes through a pair when, and as, that one does,
/// and never by its own geometry.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalDrawnWith(pub Entity);

/// The drawable goes through a pair this frame: it is drawn by that pair's
/// carry ([`carry_through_pairs`]). Kept until the frame it no longer goes
/// through, so the systems of the next frame's `Update` read it too (the
/// far-side compositor leaves a carried drawable to the carry).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalCarried;

/// Bookkeeping: the layers a carried drawable had before the carry put it on
/// its camera's layer. Given back at the start of each frame
/// ([`give_back_carried_layers`]), so the systems of a frame see a drawable's
/// own layers, and the carry decides again in `PostUpdate`.
#[derive(Component, Clone, Debug)]
pub struct CarriedLayers {
    original: Option<RenderLayers>,
    carry: RenderLayers,
}

/// On a carry camera ([`PortalCarryAtlas`]). The host gives it the world's
/// compositing space, as it gives a window's capture camera.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalCarryCamera {
    pub slot: usize,
}

/// The carry cameras and their textures. Made when a live room first has a
/// portal, before anything goes through: a target made on the frame it is
/// first needed is not drawn on that frame.
#[derive(Resource, Default)]
pub struct PortalCarryAtlas {
    slots: Vec<CarrySlot>,
    /// Texels on each side of a slot's texture.
    side: u32,
    /// World units on each side of what a slot's camera sees.
    span: f32,
}

struct CarrySlot {
    camera: Entity,
    image: Handle<Image>,
}

/// Whether the pairs carry what goes through them. On by default; off draws
/// each drawable whole on its side of a pair, to compare against (a
/// developer's A/B, `capture_scene --carry off`).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalCarrySelection {
    pub enabled: bool,
}

impl Default for PortalCarrySelection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// The carry slots made before the first need: two pairs carrying at once.
const WARM_SLOTS: usize = 2;
/// The clipped quads made before the first need: the pieces of two carries
/// through a pair each (here, two beside, through) and one more.
const WARM_QUADS: usize = 10;
/// How near (world px) two drawables' roots are to be one root: a swing's
/// owner and the body it is.
const ROOT_SLACK: f32 = 1.0;
/// The largest texture side of a carry, in texels.
const MAX_SIDE: u32 = 2048;
/// The camera order of the carries: after the impostor cameras (a carried
/// part-drawn body draws its composited cell) and before every window's
/// capture and view (they draw the carried pieces).
const CARRY_CAMERA_ORDER: isize = -1000;

/// Give each carried drawable its own layers back. The carry decides again,
/// late in this frame.
pub fn give_back_carried_layers(mut commands: Commands, mut carried: Query<(Entity, &CarriedLayers, Option<&mut RenderLayers>)>) {
    for (entity, layers, current) in &mut carried {
        // Another writer that changed the layers since owns them now.
        match (current, &layers.original) {
            (Some(mut current), Some(original)) if *current == layers.carry => *current = original.clone(),
            (Some(current), None) if *current == layers.carry => {
                commands.entity(entity).remove::<RenderLayers>();
            }
            _ => {}
        }
        commands.entity(entity).remove::<CarriedLayers>();
    }
}

/// What goes through one pair, in one live room, this frame.
struct Carry {
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    frame: PortalWorldFrame,
    /// The link it goes in by.
    link: u32,
    /// Where it comes from: the body's centre, or the first drawable's.
    root: Vec2,
    /// It reaches from `root` (a swing): it is cut by the wedge from there.
    reaching: bool,
    /// The body whose policy cuts it, when a body goes through.
    body: Option<Entity>,
    /// A host's picture goes in only as deep as its limb (`PortalHostReach`).
    reach: Option<f32>,
    /// The drawables it carries (each with its family).
    drawables: Vec<Entity>,
    /// Engine-space bounds of what it carries.
    min: Vec2,
    max: Vec2,
    /// The render z its part in front of the entry is drawn at.
    z: f32,
    /// The bodies it carries, for the trace: entity, centre, straddled channel.
    traced: Vec<(Entity, Vec2, ambition_portal2d::PortalChannel)>,
}

/// The links of a room's portals, each pair both ways.
fn room_links(portals: &[PlacedPortal]) -> Vec<PortalLink> {
    ambition_portal2d::linked_pairs(portals)
        .into_iter()
        .enumerate()
        .flat_map(|(index, pair)| PortalLink::both_ways(index, pair))
        .collect()
}

/// The engine-space bounds of a drawable: its local bounds placed by its
/// global transform.
fn engine_bounds(frame: &PortalWorldFrame, aabb: &Aabb, at: &Transform) -> (Vec2, Vec2) {
    let c = Vec3::from(aabb.center);
    let h = Vec3::from(aabb.half_extents);
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            let p = at.transform_point(c + Vec3::new(h.x * sx, h.y * sy, 0.0)).truncate();
            let p = ae::config::bevy_size_to_world(frame.size, p);
            min = min.min(p);
            max = max.max(p);
        }
    }
    (min, max)
}

/// A sprite's local bounds, as `bevy_sprite` computes its `Aabb`: its custom
/// size, its rect, its atlas cell or its image, placed by its anchor.
fn sprite_bounds(
    sprite: &Sprite,
    anchor: &bevy::sprite::Anchor,
    images: &Assets<Image>,
    atlases: Option<&Assets<TextureAtlasLayout>>,
) -> Option<Aabb> {
    let size = sprite.custom_size.or_else(|| sprite.rect.map(|rect| rect.size())).or_else(|| {
        match &sprite.texture_atlas {
            None => images.get(&sprite.image).map(Image::size_f32),
            Some(atlas) => atlas.texture_rect(atlases?).map(|rect| rect.size().as_vec2()),
        }
    })?;
    Some(Aabb { center: (-anchor.as_vec() * size).extend(0.0).into(), half_extents: (0.5 * size).extend(0.0).into() })
}

/// Is a drawable on these layers drawn by a world camera: the world layer,
/// or a live room's band? A drawable on another layer (the impostor atlas,
/// the parallax, a view's own layer) is not carried.
fn on_the_world(layers: Option<&RenderLayers>) -> bool {
    layers.is_none_or(|layers| {
        layers.iter().any(|layer| layer == 0 || (LIVE_ROOM_RENDER_LAYER_BASE..=LIVE_ROOM_RENDER_LAYER_LAST).contains(&layer))
    })
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct CarryAssets<'w> {
    images: Option<ResMut<'w, Assets<Image>>>,
    atlases: Option<Res<'w, Assets<TextureAtlasLayout>>>,
    meshes: Option<ResMut<'w, Assets<Mesh>>>,
    materials: Option<ResMut<'w, Assets<PortalClipMaterial>>>,
    quad: Option<Res<'w, crate::clip_material::PortalPieceQuad>>,
}

/// The texels a world unit takes on the screen: the densest main view's.
fn screen_density(cameras: &Query<(&Camera, &Projection), With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>>) -> Option<(f32, UVec2)> {
    cameras
        .iter()
        .filter_map(|(camera, projection)| {
            let physical = camera.physical_viewport_size()?;
            let Projection::Orthographic(ortho) = projection else {
                return None;
            };
            let area = ortho.area.size();
            (area.x > 0.0 && area.y > 0.0).then(|| ((physical.as_vec2() / area).max_element(), physical))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// The cameras the carry reads (the views', for their density) and drives.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CarryCameras<'w, 's> {
    main: Query<'w, 's, (&'static Camera, &'static Projection), With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>>,
    carry: Query<
        'w,
        's,
        (&'static PortalCarryCamera, &'static mut Camera, &'static mut Transform, &'static mut Projection),
        Without<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
    >,
}

/// A drawable's family and its layers.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CarryFamilies<'w, 's> {
    drawn_with: Query<'w, 's, (Entity, &'static PortalDrawnWith)>,
    children: Query<'w, 's, &'static Children>,
    placed: Query<'w, 's, (&'static Transform, Option<&'static Aabb>, &'static Visibility), Without<PortalCarryCamera>>,
    layers: Query<'w, 's, &'static mut RenderLayers>,
    were_carried: Query<'w, 's, Entity, With<PortalCarried>>,
}

/// Carry what goes through each pair this frame. See the module docs.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn carry_through_pairs(
    mut commands: Commands,
    frames: PortalFrames,
    portals: Query<(Entity, &PlacedPortal)>,
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
    passages: Option<Res<ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>>,
    mut atlas: ResMut<PortalCarryAtlas>,
    mut assets: CarryAssets,
    mut cameras: CarryCameras,
    bodies: Query<
        (
            Entity,
            &crate::PortalBodyView,
            &crate::PortalTransitView,
            Option<&crate::PortalHostReach>,
        ),
        With<PortalSceneBody>,
    >,
    // Every other drawable: its own geometry says.
    free: Query<
        (
            Entity,
            Option<&Aabb>,
            Option<(&Sprite, &bevy::sprite::Anchor)>,
            &Transform,
            &Visibility,
            Option<&ambition_platformer2d_shared_tangle::portal_passages::ReachesFrom>,
        ),
        (
            Or<(With<Sprite>, With<Mesh2d>, With<Text2d>)>,
            Without<ChildOf>,
            Without<PortalSceneBody>,
            Without<PortalDrawnWith>,
            Without<PortalBodyPiece>,
            Without<crate::PortalFarSidePiece>,
            Without<crate::PortalLightLine>,
            Without<PortalCarryCamera>,
        ),
    >,
    mut families: CarryFamilies,
    quads: Query<(), With<PortalBodyPiece>>,
    selection: Option<Res<PortalCarrySelection>>,
    mut pool: ResMut<CarryQuadPool>,
) {
    let by_room = frames.portals_by_room(portals.iter());
    let any_portal = by_room.rooms().any(|(_, list)| !list.is_empty())
        && selection.is_none_or(|selection| selection.enabled);
    let mut carried_now: Vec<Entity> = Vec::new();
    pool.0.begin(|entity| quads.contains(entity));
    let (Some(images), Some(meshes), Some(materials)) =
        (assets.images.as_deref_mut(), assets.meshes.as_deref_mut(), assets.materials.as_deref_mut())
    else {
        // No render stack (a headless host): nothing is drawn, so nothing is
        // carried.
        return;
    };
    let mesh = match assets.quad.as_deref() {
        Some(quad) => quad.0.clone(),
        None => meshes.add(Rectangle::default()),
    };
    // The cameras and their textures, made before the first need.
    if any_portal && atlas.slots.is_empty() {
        if let Some((density, physical)) = screen_density(&cameras.main) {
            let side = physical.max_element().next_power_of_two().clamp(256, MAX_SIDE);
            atlas.side = side;
            atlas.span = side as f32 / density.max(0.05);
            for slot in 0..WARM_SLOTS {
                let made = spawn_slot(&mut commands, images, slot, side, atlas.span);
                atlas.slots.push(made);
            }
            // The quads too: a quad made on the frame of its first piece was
            // not drawn on that frame.
            pool.0.warm(&mut commands, materials, &mesh, WARM_QUADS, PortalBodyPiece);
            pool.0.begin(|_| true);
        }
    }
    let convention = tuning.as_deref().map(|tuning| tuning.convention.map_convention()).unwrap_or_default();

    let mut carries: Vec<Carry> = Vec::new();
    if any_portal && !atlas.slots.is_empty() {
        // Bodies in transit: the pair their host says they straddle.
        for (visual, view, transit, reach) in &bodies {
            let Some(placement) = frames.of(visual) else {
                continue;
            };
            let all = by_room.in_room(Some(placement.room));
            let Some(straddled) = find_portal(all, transit.straddling) else {
                continue;
            };
            let links = room_links(all);
            let Some(link) = links.iter().find(|link| link.enter == straddled.aperture()) else {
                continue;
            };
            let Ok((at, aabb, shown)) = families.placed.get(visual) else {
                continue;
            };
            if *shown == Visibility::Hidden {
                continue;
            }
            let (min, max) = aabb.map_or((view.pos - view.size * 0.5, view.pos + view.size * 0.5), |aabb| {
                engine_bounds(&placement.frame, aabb, at)
            });
            let reach = reach.map(|reach| reach.0);
            let carry = carries
                .iter_mut()
                .find(|c| c.room == placement.room && c.link == link.key && c.reach.is_none() && reach.is_none());
            match carry {
                Some(carry) => {
                    carry.drawables.push(visual);
                    carry.min = carry.min.min(min);
                    carry.max = carry.max.max(max);
                    carry.traced.push((view.body, view.pos, transit.straddling));
                }
                None => carries.push(Carry {
                    room: placement.room,
                    frame: placement.frame,
                    link: link.key,
                    root: view.pos,
                    reaching: false,
                    body: Some(view.body),
                    reach,
                    drawables: vec![visual],
                    min,
                    max,
                    z: at.translation.z,
                    traced: vec![(view.body, view.pos, transit.straddling)],
                }),
            }
        }
        // Every other drawable, by its own geometry.
        for (drawable, aabb, sprite, at, shown, reaches_from) in &free {
            if *shown == Visibility::Hidden || !on_the_world(families.layers.get(drawable).ok()) {
                continue;
            }
            // Over the window: a drawable under it is covered by the pane.
            if at.translation.z <= crate::PORTAL_WINDOW_Z {
                continue;
            }
            let Some(placement) = frames.of(drawable) else {
                continue;
            };
            let all = by_room.in_room(Some(placement.room));
            if all.is_empty() {
                continue;
            }
            // A sprite's bounds as it is now (its `Aabb` is computed after the
            // carry: one frame old, and missing on the frame it is made).
            let bounds = sprite
                .and_then(|(sprite, anchor)| sprite_bounds(sprite, anchor, images, assets.atlases.as_deref()))
                .or_else(|| aabb.copied());
            let Some(aabb) = bounds else {
                continue;
            };
            let (min, max) = engine_bounds(&placement.frame, &aabb, at);
            // It reaches out from its own centre, or from the point it says.
            let root = reaches_from.map_or_else(
                || ae::config::bevy_size_to_world(placement.frame.size, at.translation.truncate()),
                |from| from.0,
            );
            let links = room_links(all);
            let budget = passages
                .as_deref()
                .map_or(Some(PieceBudget::SINGLE_HOP), |passages| passages.budget_for(None, Some(placement.room)));
            let Some(budget) = budget else {
                continue;
            };
            let corners = vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
            let cut = match reaches_from {
                Some(_) => partition_reaching(&ae::CombatVolume::convex(corners), root, &links, None, budget, convention),
                None => partition(&ae::CombatVolume::convex(corners), root, &links, None, budget, convention),
            };
            let Some(through) = cut.pieces.iter().find(|piece| piece.is_carried()) else {
                continue;
            };
            let link = through.path[0];
            let z = at.translation.z;
            // A carry is cut by one rule. Square-on drawables share one; a
            // swing shares only with what reaches from its root, or with the
            // body that swings it (a body that fits the opening is all in the
            // wedge from its centre, so the wedge cuts it as square on does).
            let reaching = reaches_from.is_some();
            let shares = |c: &Carry| {
                c.room == placement.room
                    && c.link == link
                    && c.reach.is_none()
                    && if reaching { c.root.distance(root) <= ROOT_SLACK && (c.reaching || c.body.is_some()) } else { !c.reaching }
            };
            match carries.iter_mut().find(|c| shares(c)) {
                Some(carry) => {
                    carry.drawables.push(drawable);
                    carry.min = carry.min.min(min);
                    carry.max = carry.max.max(max);
                    carry.z = carry.z.max(z);
                    if reaching {
                        carry.reaching = true;
                        carry.root = root;
                    }
                }
                None => carries.push(Carry {
                    room: placement.room,
                    frame: placement.frame,
                    link,
                    root,
                    reaching,
                    body: None,
                    reach: None,
                    drawables: vec![drawable],
                    min,
                    max,
                    z,
                    traced: Vec::new(),
                }),
            }
        }
    }

    // Each carry: its slot, its camera, its members' layers, its pieces.
    let mut used = vec![false; atlas.slots.len()];
    let fresh_quads_before = pool.0.made();
    for carry in carries.iter_mut().take(PORTAL_CARRY_LAYERS) {
        let slot = match used.iter().position(|taken| !taken) {
            Some(slot) => slot,
            None => {
                let slot = atlas.slots.len();
                let (side, span) = (atlas.side, atlas.span);
                let made = spawn_slot(&mut commands, images, slot, side, span);
                atlas.slots.push(made);
                used.push(false);
                slot
            }
        };
        used[slot] = true;
        let all = by_room.in_room(Some(carry.room));
        let mut links = room_links(all);
        let budget = match carry.reach {
            Some(depth) => {
                links.retain(|link| link.key == carry.link);
                if let Some(link) = links.first_mut() {
                    link.depth = Some(depth);
                }
                PieceBudget::SINGLE_HOP
            }
            None => match passages.as_deref() {
                Some(passages) => match passages.budget_for(carry.body, Some(carry.room)) {
                    Some(budget) => budget,
                    None => continue,
                },
                None => PieceBudget::SINGLE_HOP,
            },
        };
        // What the camera sees: a square of the slot's span about what is
        // carried (larger than the span is cut at its edge).
        let centre = (carry.min + carry.max) * 0.5;
        let span = atlas.span;
        let half = Vec2::splat(span * 0.5);
        let render_centre = carry.frame.to_render(centre, 0.0);
        let layer = PORTAL_CARRY_LAYER_BASE + slot;
        if let Ok((_, mut camera, mut transform, mut projection)) = cameras.carry.get_mut(atlas.slots[slot].camera) {
            camera.is_active = true;
            *transform = Transform::from_translation(render_centre);
            if let Projection::Orthographic(ortho) = &mut *projection {
                let fixed = matches!(ortho.scaling_mode, ScalingMode::Fixed { width, height } if width == span && height == span);
                if !fixed {
                    ortho.scaling_mode = ScalingMode::Fixed { width: span, height: span };
                }
            }
        }
        // The members and their families.
        let mut members: Vec<Entity> = Vec::new();
        let mut pending: Vec<Entity> = carry.drawables.clone();
        while let Some(entity) = pending.pop() {
            if members.contains(&entity) {
                continue;
            }
            members.push(entity);
            if let Ok(kids) = families.children.get(entity) {
                pending.extend(kids.iter());
            }
            pending.extend(families.drawn_with.iter().filter(|(_, with)| with.0 == entity).map(|(e, _)| e));
        }
        // The pieces: the part of the camera's square that holds what is
        // carried, cut by the pairs it reaches. (The whole square reached
        // openings that nothing carried is near.)
        let (lo, hi) = (carry.min.max(centre - half), carry.max.min(centre + half));
        let size = (hi - lo).max(Vec2::splat(1.0));
        let middle = (lo + hi) * 0.5;
        // Texture coordinates of that part: the camera's square spans the
        // texture, its top (least engine y) at the texture's top.
        let uv = Vec4::new(
            (lo.x - (centre.x - half.x)) / span,
            (lo.y - (centre.y - half.y)) / span,
            (hi.x - (centre.x - half.x)) / span,
            (hi.y - (centre.y - half.y)) / span,
        );
        let corners = vec![lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)];
        let first = links.iter().find(|link| link.key == carry.link).map(|link| link.key);
        let cut = if carry.reaching {
            partition_reaching(&ae::CombatVolume::convex(corners), carry.root, &links, first, budget, convention)
        } else {
            partition(&ae::CombatVolume::convex(corners), carry.root, &links, first, budget, convention)
        };
        let through = cut.pieces.iter().any(VolumePiece::is_carried);
        let image = atlas.slots[slot].image.clone();
        for piece in &cut.pieces {
            let bounds = piece.active_bounds();
            if bounds.len() > CLIP_PLANES {
                bevy::log::warn_once!("a carried piece is bounded by {} planes; it is drawn clipped by the first {CLIP_PLANES}", bounds.len());
            }
            let planes: Vec<Vec4> = bounds.iter().map(|plane| clip_plane_render(&carry.frame, plane.point, plane.inward)).collect();
            let (base, flip, name) = if piece.is_carried() {
                let copy = ambition_portal2d::copy_transform_of(&piece.to_piece);
                let base = Transform {
                    translation: carry.frame.to_render(piece.carry(middle), crate::PORTAL_EXIT_COPY_Z),
                    rotation: Quat::from_rotation_z(copy.rolled(0.0)),
                    scale: Vec3::ONE,
                };
                (base, copy.flip_x, "Portal carry piece (through)")
            } else {
                (Transform::from_translation(carry.frame.to_render(middle, carry.z)), false, "Portal carry piece (here)")
            };
            let pose = clip_piece_transform(&base, Vec2::ZERO, size);
            pool.0.draw(
                &mut commands,
                materials,
                &mesh,
                PortalClipMaterial::new(
                    uv,
                    Vec4::new(if flip { 1.0 } else { 0.0 }, 0.0, 0.0, 1.0),
                    Vec4::ONE,
                    &planes,
                    image.clone(),
                ),
                pose,
                (PortalBodyPiece, ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(carry.room)),
                name,
            );
        }
        // A quad made this frame may not be drawn on it: the members stay on
        // the world too, whole, for that frame.
        let handoff = pool.0.made() > fresh_quads_before;
        for member in &members {
            let own = families.layers.get(*member).ok().cloned();
            let own = own.as_ref();
            // A part drawn into an impostor, a parallax panel: not the world's.
            if !on_the_world(own) {
                continue;
            }
            let carry_layers = match (handoff, own) {
                (true, Some(own)) => own.clone().with(layer),
                (true, None) => RenderLayers::layer(0).with(layer),
                (false, _) => RenderLayers::layer(layer),
            };
            match families.layers.get_mut(*member) {
                Ok(mut current) => *current = carry_layers.clone(),
                Err(_) => {
                    commands.entity(*member).insert(carry_layers.clone());
                }
            }
            commands
                .entity(*member)
                .insert((CarriedLayers { original: own.cloned(), carry: carry_layers }, PortalCarried));
            carried_now.push(*member);
        }
        for (body, at, straddling) in &carry.traced {
            bevy::log::debug!(
                target: "ambition_portal::piece_trace",
                "piece body={:?} at={:.1},{:.1} straddling={} bounds={:.0},{:.0}..{:.0},{:.0} span={:.0} drawn={:.0},{:.0}..{:.0},{:.0} {}",
                body,
                at.x,
                at.y,
                straddling.name(),
                carry.min.x,
                carry.min.y,
                carry.max.x,
                carry.max.y,
                span,
                lo.x,
                lo.y,
                hi.x,
                hi.y,
                match (handoff, through) {
                    (true, _) => "pieces made: the body shows whole",
                    (false, true) => "cut",
                    (false, false) => "near piece only: nothing has crossed",
                }
            );
        }
    }
    // The cameras with nothing to carry rest.
    for (slot, taken) in used.iter().enumerate() {
        if !taken {
            if let Ok((_, mut camera, ..)) = cameras.carry.get_mut(atlas.slots[slot].camera) {
                if camera.is_active {
                    camera.is_active = false;
                }
            }
        }
    }
    for entity in &families.were_carried {
        if !carried_now.contains(&entity) {
            commands.entity(entity).remove::<PortalCarried>();
        }
    }
    pool.0.end(&mut commands);
}

/// One carry slot: a texture `side` texels square and a camera that draws
/// carry layer `slot` into it, `span` world units square.
fn spawn_slot(commands: &mut Commands, images: &mut Assets<Image>, slot: usize, side: u32, span: f32) -> CarrySlot {
    let image = images.add(Image::new_target_texture(side, side, TextureFormat::Rgba8UnormSrgb, None));
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: CARRY_CAMERA_ORDER - slot as isize,
                is_active: false,
                // A transparent clear: the texture holds what is carried and
                // nothing else, its colour premultiplied by its cover.
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            bevy::render::view::Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            RenderLayers::layer(PORTAL_CARRY_LAYER_BASE + slot),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed { width: span, height: span },
                ..OrthographicProjection::default_2d()
            }),
            PortalCarryCamera { slot },
            Name::new(format!("Portal carry camera {slot}")),
        ))
        .id();
    CarrySlot { camera, image }
}

/// The carry's clipped quads, kept from frame to frame.
#[derive(Resource, Default)]
pub struct CarryQuadPool(pub(crate) ClipQuadPool);
