//! Read-only evidence of the bodies and presentations present at the shutter.

use ambition_platformer2d as engine;
use bevy::prelude::*;
use serde_json::json;

pub fn snapshot(world: &World) -> serde_json::Value {
    use engine::characters::actor::{BodyHealth, WornCharacter};
    use engine::sprite_sheet::character::CharacterAnimator;
    use engine::render::rendering::actors::{BoundActorArt, BoundSpriteQuality, PlayerSpriteCharacter};
    let mut bodies = Vec::new();
    let mut visuals = Vec::new();
    for entity in world.iter_entities() {
        if let Some(worn) = entity.get::<WornCharacter>() {
            bodies.push(json!({"character_id": worn.id(),
                "stable_id": entity.get::<engine::observation::SimId>().map(|id| id.as_str()),
                "session": entity.get::<engine::platformer::lifecycle::SessionScopedEntity>().map(|scope| scope.0.0),
                "feature_id": entity.get::<engine::combat::components::FeatureId>().map(|id| &id.0),
                "alive": entity.get::<BodyHealth>().map(|health| health.alive()),
                "death": entity.get::<engine::combat::components::DeathBeat>().map(|beat| format!("{beat:?}")),
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
                "requested_pose": requested.map(|pose| format!("{pose:?}")),
                "animator_pose": format!("{:?}", animator.current),
                "resolved_row": row, "frame": animator.frame, "held": animator.clip_held,
                "quality_tier": quality.map(|q| format!("{:?}", q.scale)),
                "realization": quality.map(|q| format!("{:?}", q.realization)),
                "requested_clip": clip.map(|clip| format!("{clip:?}")),
            }));
        }
    }
    bodies.sort_by_key(|row| row.to_string());
    visuals.sort_by_key(|row| row.to_string());
    let content = world.get_resource::<ambition_content::pack::SelectedContentPack>()
        .map(|pack| pack.0.fingerprint.hex());
    json!({"schema": "ambition.art_runtime.v1", "tick": world.get_resource::<engine::time::SimTick>().map(|tick| tick.get()),
        "build": {"package": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION"), "revision": option_env!("AMBITION_BUILD_COMMIT")},
        "content_fingerprint": content,
        "sheet_generation": engine::sprite_sheet::baked_sheet_rons::current().id(),
        "bodies": bodies, "visuals": visuals})
}
