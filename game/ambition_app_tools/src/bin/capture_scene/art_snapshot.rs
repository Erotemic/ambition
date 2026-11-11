//! Read-only evidence of the bodies and presentations present at the shutter.

use ambition_platformer2d as engine;
use bevy::prelude::*;
use serde_json::json;

pub fn snapshot(world: &World) -> serde_json::Value {
    let sprite_content = world.resource::<ambition_platformer2d::sprite_sheet::SpriteContent>().clone();
    use engine::characters::actor::{BodyHealth, WornCharacter};
    use engine::sprite_sheet::character::CharacterAnimator;
    use engine::render::rendering::actors::{BoundActorArt, BoundSpriteQuality, PlayerSpriteCharacter};
    let carry_load = world.get_resource::<engine::portal_presentation::PortalCarryAtlas>().map(|atlas| json!({
        "families":atlas.load.families,"texture_bytes":atlas.load.texture_bytes,
        "reduced_resolution":atlas.load.reduced_resolution,"material_proxies":atlas.load.material_proxies,
        "budget_bytes":atlas.budget.max_texture_bytes,
    }));
    let mut bodies = Vec::new();
    let mut visuals = Vec::new();
    let mut views = Vec::new();
    let mut material_parts = Vec::new();
    let mut strikes = Vec::new();
    let mut items = Vec::new();
    let mut portal_drawables = Vec::new();
    let mut apertures = Vec::new();
    for entity in world.iter_entities() {
        if let Some(portal) = entity.get::<engine::portal::PlacedPortal>() {
            apertures.push(json!({"channel":portal.channel.name(), "position":portal.pos.to_array(),
                "normal":portal.normal.to_array(), "half_length":portal.half_length,
                "room":entity.get::<engine::platformer::lifecycle::InRoomInstance>().map(|room|format!("{:?}",room.0))}));
        }
        if let (Some(item), Some(body), Some(custody)) = (
            entity.get::<engine::item::GroundItem>(),
            entity.get::<engine::engine_core::BodyKinematics>(),
            entity.get::<engine::item::ItemCustody>(),
        ) {
            items.push(json!({"entity": format!("{:?}", entity.id()),
                "item_id": item.spec.id, "custody": format!("{custody:?}"),
                "position": body.pos.to_array(), "canonical_size": body.canonical_size.to_array(),
                "scale": body.material.scale(), "orthogonal": body.material.orthogonal().to_cols_array(),
                "stored": entity.contains::<engine::engine_core::BodyInStorage>()}));
        }
        if let Some(view) = entity.get::<engine::sim_view::camera_snapshot::CameraViewState>() {
            let frame = entity.get::<engine::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
                .and_then(|resolved| resolved.frame());
            views.push(json!({
                "center": [view.center_world.x, view.center_world.y],
                "visible_size": [view.visible_view.x, view.visible_view.y],
                "zoom": view.zoom_multiplier, "zone": view.active_camera_zone,
                "follow": frame.map(|frame| [frame.follow_world.x, frame.follow_world.y]),
                "room": frame.map(|frame| format!("{:?}", frame.room)),
            }));
        }
        if let Some(geometry) = world.get_resource::<engine::platformer::body::geometry::BodyGeometry>() {
            let room = entity.get::<engine::platformer::lifecycle::InRoomInstance>().map(|room| room.0);
            if let Some(body) = entity.get::<engine::engine_core::BodyKinematics>()
                .filter(|_| !entity.contains::<engine::engine_core::BodyInStorage>()) {
                let partition = match (body.canonical_box(), body.material_map()) {
                    (Ok(domain), Ok(to_world)) => partition_snapshot(geometry.body_pieces(
                        engine::engine_core::frame::thread::MaterialPrimitive { domain: &domain, to_world }, entity.id(), room)),
                    (domain, material) => json!({"numeric_error": format!("{domain:?} {material:?}")}),
                };
                material_parts.push(json!({"entity": format!("{:?}", entity.id()),
                    "stable_id": entity.get::<engine::observation::SimId>().map(|id| id.as_str()),
                    "position": body.pos.to_array(), "partition": partition}));
            }
            if let Some(hit) = entity.get::<engine::combat::strike::Hitbox>() {
                let owner = world.get_entity(hit.owner).ok();
                let room = owner.as_ref().and_then(|owner| owner.get::<engine::platformer::lifecycle::InRoomInstance>()).map(|room| room.0);
                strikes.push(json!({"entity": format!("{:?}", entity.id()), "owner": format!("{:?}", hit.owner),
                    "damage": hit.damage, "partition": partition_snapshot(engine::combat::hitbox::strike_pieces(
                        Some(geometry), hit, world.get::<engine::engine_core::BodyKinematics>(hit.owner), room))}));
            }
        }
        if entity.contains::<Sprite>()
            || entity.contains::<engine::platformer::camera_layers::MainCamera>()
            || entity.contains::<engine::portal_presentation::PortalSceneBody>()
            || entity.contains::<engine::portal_presentation::PortalBodyPiece>()
            || entity.contains::<engine::portal_presentation::PortalCarried>()
            || entity.contains::<engine::portal_presentation::PortalCarryCamera>()
            || entity.contains::<engine::render::rendering::actors::rigged::RiggedPresentationOf>()
            || entity.contains::<engine::render::rendering::actors::rigged::RiggedPartSlot>()
            || entity.contains::<engine::render::rendering::actors::rigged::RiggedImpostorCamera>() {
            portal_drawables.push(json!({"entity": format!("{:?}", entity.id()),
                "name": entity.get::<Name>().map(Name::as_str),
                "body_view": entity.get::<engine::portal_presentation::PortalBodyView>().map(|view| format!("{view:?}")),
                "components": entity.archetype().components().iter().filter_map(|id| world.components().get_info(*id)).map(|info| info.name().to_string()).collect::<Vec<_>>(),
                "parent": entity.get::<ChildOf>().map(|v| format!("{:?}", v.parent())),
                "rigged_body": entity.get::<engine::render::rendering::actors::rigged::RiggedPresentationOf>().map(|v| format!("{:?}", v.0)),
                "rigged_frame": entity.get::<engine::render::rendering::actors::rigged::RiggedPresentation>().and_then(|p| {
                    let root = entity.get::<engine::render::rendering::actors::rigged::RiggedPresentationOf>()?.0;
                    let animator = world.get::<CharacterAnimator>(root)?;
                    let row = animator.drawn_row().and_then(|slot| animator.spec.row_name(slot))?;
                    Some(json!({"impostor":format!("{:?}",p.impostor),"row":row,"frame":animator.frame,
                        "opacity":p.pages.flipbook.frame_opacity(row,animator.frame),"shown":p.shown.as_ref().map(|(_,at,generation)|format!("{at:?} {generation}"))}))
                }),
                "target": entity.get::<bevy::camera::RenderTarget>().map(|v| format!("{v:?}")),
                "target_image": entity.get::<bevy::camera::RenderTarget>().and_then(|target| {
                    let bevy::camera::RenderTarget::Image(target) = target else { return None; };
                    let image = world.get_resource::<Assets<Image>>()?.get(&target.handle)?;
                    Some(json!({"size":[image.texture_descriptor.size.width,image.texture_descriptor.size.height],
                        "format":format!("{:?}", image.texture_descriptor.format),
                        "render_attachment":image.texture_descriptor.usage.contains(bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT)}))
                }),
                "visibility": entity.get::<Visibility>().map(|v| format!("{v:?}")),
                "inherited_visibility": entity.get::<InheritedVisibility>().map(|v| v.get()),
                "view_visibility": entity.get::<ViewVisibility>().map(|v| v.get()),
                "sprite_class": entity.get::<bevy::camera::visibility::VisibilityClass>().map(|v| v.0.contains(&std::any::TypeId::of::<Sprite>())),
                "aabb": entity.get::<bevy::camera::primitives::Aabb>().map(|v| format!("{v:?}")),
                "global_transform": entity.get::<GlobalTransform>().map(|v| format!("{v:?}")),
                "sprite_color": entity.get::<Sprite>().map(|v| format!("{:?}", v.color)),
                "sprite": entity.get::<Sprite>().map(|v| format!("{v:?}")),
                "anchor": entity.get::<bevy::sprite::Anchor>().map(|v| format!("{v:?}")),
                "atlas_rect": entity.get::<Sprite>().and_then(|sprite| sprite.texture_atlas.as_ref())
                    .and_then(|atlas| world.get_resource::<Assets<TextureAtlasLayout>>()?.get(&atlas.layout)?.textures.get(atlas.index))
                    .map(|rect| format!("{rect:?}")),
                "visible_entities": entity.get::<bevy::camera::visibility::VisibleEntities>().map(|v| v.entities.values().flatten().map(|e| format!("{e:?}")).collect::<Vec<_>>()),
                "layers": entity.get::<bevy::camera::visibility::RenderLayers>().map(|v| v.iter().collect::<Vec<_>>()),
                "transform": entity.get::<Transform>().map(|v| format!("{v:?}")),
                "image": entity.get::<Sprite>().map(|v| format!("{:?}", v.image)),
                "sprite_size": entity.get::<Sprite>().and_then(|v| v.custom_size.map(|v| v.to_array())),
                "clip_material": entity.get::<bevy::sprite_render::MeshMaterial2d<engine::portal_presentation::PortalClipMaterial>>()
                    .and_then(|handle| world.get_resource::<Assets<engine::portal_presentation::PortalClipMaterial>>()?.get(&handle.0))
                    .map(|material| format!("{material:?}")),
                "mesh_positions": entity.get::<Mesh2d>().and_then(|handle| world.get_resource::<Assets<Mesh>>()?.get(&handle.0))
                    .and_then(|mesh| mesh.attribute(Mesh::ATTRIBUTE_POSITION)).map(|positions| format!("{positions:?}")),
                "camera": entity.get::<Camera>().map(|camera| json!({"active":camera.is_active,"order":camera.order})),
            }));
        }
        if let Some(worn) = entity.get::<WornCharacter>() {
            bodies.push(json!({"character_id": worn.id(),
                "stable_id": entity.get::<engine::observation::SimId>().map(|id| id.as_str()),
                "session": entity.get::<engine::platformer::lifecycle::SessionScopedEntity>().map(|scope| scope.0.0),
                "feature_id": entity.get::<engine::combat::components::FeatureId>().map(|id| &id.0),
                "limb": entity.get::<engine::characters::actor::Limb>().map(|limb| json!({
                    "owner":format!("{:?}",limb.of), "slot":limb.slot.as_str(),
                    "owner_id":world.get::<engine::observation::SimId>(limb.of).map(|id|id.as_str()),
                })),
                "alive": entity.get::<BodyHealth>().map(|health| health.alive()),
                "death": entity.get::<engine::combat::components::DeathBeat>().map(|beat| format!("{beat:?}")),
                "position": entity.get::<engine::engine_core::BodyKinematics>().map(|body| [body.pos.x, body.pos.y]),
                "facing": entity.get::<engine::engine_core::BodyKinematics>().map(|body| body.facing),
                "material": entity.get::<engine::engine_core::BodyKinematics>().map(|body| json!({
                    "scale": body.material.scale(),
                    "orthogonal": body.material.orthogonal().to_cols_array(),
                    "canonical_size": body.canonical_size.to_array(),
                    "velocity": body.vel.to_array(),
                })),
            }));
        }
        if let Some(animator) = entity.get::<CharacterAnimator>() {
            let quality = entity.get::<BoundSpriteQuality>();
            let character = entity.get::<BoundActorArt>().map(|art| art.0.as_str())
                .or_else(|| entity.get::<PlayerSpriteCharacter>().map(|art| art.id.as_str()));
            let row = animator.drawn_row().and_then(|slot| animator.spec.row_name(slot));
            let feature = entity.get::<engine::render::rendering::FeatureVisual>();
            let actor = feature.and_then(|feature| world.get_resource::<engine::sim_view::ActorAnimIndex>()?.get(&feature.id));
            let body_pose = entity.get::<engine::sim_view::BodyPoseView>();
            let requested = body_pose.map(|pose| pose.anim).or_else(|| actor.map(|frame| frame.anim));
            let clip = body_pose.and_then(|pose| pose.clip.as_ref()).or_else(|| actor.and_then(|frame| frame.clip.as_ref()));
            visuals.push(json!({"character_id": character, "target": animator.spec.target(),
                "manifest_fingerprint": engine::sprite_sheet::inspection::fingerprint(&animator.spec),
                "sheet_key": engine::sprite_sheet::inspection::sheet_key(&animator.spec),
                "stable_id": entity.get::<engine::observation::SimId>().map(|id| id.as_str()),
                "session": entity.get::<engine::platformer::lifecycle::SessionScopedEntity>().map(|scope| scope.0.0),
                "feature_id": entity.get::<engine::render::rendering::FeatureVisual>().map(|feature| &feature.id),
                "presented_facing": body_pose.map(|pose| pose.facing),
                "sprite_flip_x": entity.get::<Sprite>().map(|sprite| sprite.flip_x),
                "requested_pose": requested.map(|pose| format!("{pose:?}")),
                "animator_pose": format!("{:?}", animator.current),
                "resolved_row": row, "frame": animator.frame, "held": animator.clip_held,
                "quality_tier": quality.map(|q| format!("{:?}", q.scale)),
                "realization": quality.map(|q| format!("{:?}", q.realization)),
                "requested_clip": clip.map(|clip| format!("{clip:?}")),
            }));
        }
    }
    material_parts.sort_by_key(|row| row.to_string());
    items.sort_by_key(|row| row.to_string());
    apertures.sort_by_key(|row| row.to_string());
    strikes.sort_by_key(|row| row.to_string());
    bodies.sort_by_key(|row| row.to_string());
    visuals.sort_by_key(|row| row.to_string());
    views.sort_by_key(|row| row.to_string());
    let music = world.get_resource::<engine::audio::music::MusicDirectorState>().map(|director| json!({
        "mode": format!("{:?}", director.mode), "cue": director.active_cue_id,
        "base_track": world.get_resource::<engine::audio::library::MusicPlaybackState>().map(|state| state.active_track()),
        "state": director.current_state_id, "section": director.current_section_id,
        "section_seconds": director.seconds_in_mode, "loop_seconds": director.seconds_in_loop,
        "intent": world.get_resource::<engine::audio::music::MusicIntent>().map(|intent| json!({
            "adaptive": format!("{:?}", intent.adaptive), "tracks": intent.simple_track_candidates,
        })),
    }));
    let content = world.get_resource::<ambition_content::pack::SelectedContentPack>()
        .map(|pack| pack.0.fingerprint.hex());
    json!({"schema": "ambition.art_runtime.v1", "tick": world.get_resource::<engine::time::SimTick>().map(|tick| tick.get()),
        "build": {"package": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION"), "revision": option_env!("AMBITION_BUILD_COMMIT")},
        "content_fingerprint": content,
        "sheet_generation": engine::sprite_sheet::baked_sheet_rons::current(&sprite_content).id(),
        "portal_carry_load":carry_load,
        "apertures":apertures,
        "bodies": bodies, "items": items, "visuals": visuals, "views": views, "music": music, "material_parts": material_parts, "strikes": strikes, "portal_drawables": portal_drawables})
}

fn partition_snapshot(partition: engine::engine_core::frame::thread::Partition) -> serde_json::Value {
    match partition {
        Ok(parts) => json!({"parts": parts.iter().map(|piece| {
            let map = piece.transform();
            json!({"path": piece.path, "scale": map.scale(), "reflected": map.reflected(),
                "source": format!("{:?}", piece.source()),
                "source_region": piece.source_region().map(|points| points.iter().map(|p| p.to_array()).collect::<Vec<_>>()),
                "region": piece.region().map(|points| points.iter().map(|p| p.to_array()).collect::<Vec<_>>())})
        }).collect::<Vec<_>>()}),
        Err(refusal) => json!({"refusal": format!("{:?}", refusal.reason), "path": refusal.path, "link": refusal.link}),
    }
}
