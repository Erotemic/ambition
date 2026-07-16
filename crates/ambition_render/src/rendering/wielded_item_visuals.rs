//! Generic over-hand visuals for wielded items.
//!
//! Simulation publishes [`HostileWieldedItemsView`] rows with an open item identity
//! and world-space hand/aim facts. A game registers presentation-only
//! [`WieldedItemVisualSpec`] rows in the App-local catalog; this renderer knows
//! no provider item names or asset paths.

use std::collections::BTreeMap;

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::HostileWieldedItemsView;

/// Presentation-only description of one wielded item's over-hand sprite.
#[derive(Clone, Debug, PartialEq)]
pub struct WieldedItemVisualSpec {
    pub texture_path: String,
    pub source_rect: Rect,
    pub grip_px: Vec2,
    pub width_per_wielder_height: f32,
    pub z_bias: f32,
    pub entity_name: String,
}

impl WieldedItemVisualSpec {
    pub fn validate(&self) -> Result<(), &'static str> {
        let size = self.source_rect.size();
        if self.texture_path.trim().is_empty() {
            return Err("texture_path must not be empty");
        }
        if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
            return Err("source_rect must have finite positive width and height");
        }
        if !self.grip_px.is_finite() {
            return Err("grip_px must be finite");
        }
        if !self.width_per_wielder_height.is_finite() || self.width_per_wielder_height <= 0.0 {
            return Err("width_per_wielder_height must be finite and positive");
        }
        if !self.z_bias.is_finite() {
            return Err("z_bias must be finite");
        }
        if self.entity_name.trim().is_empty() {
            return Err("entity_name must not be empty");
        }
        Ok(())
    }
}

/// App-local provider-authored map from held-item id to presentation spec.
#[derive(Resource, Clone, Debug, Default)]
pub struct WieldedItemVisualCatalog {
    specs: BTreeMap<String, WieldedItemVisualSpec>,
}

impl WieldedItemVisualCatalog {
    pub fn get(&self, item_id: &str) -> Option<&WieldedItemVisualSpec> {
        self.specs.get(item_id)
    }

    fn register(&mut self, item_id: String, spec: WieldedItemVisualSpec) {
        assert!(
            !item_id.trim().is_empty(),
            "wielded-item visual id is empty"
        );
        spec.validate()
            .unwrap_or_else(|message| panic!("invalid wielded-item visual {item_id:?}: {message}"));
        if let Some(existing) = self.specs.get(&item_id) {
            assert_eq!(
                existing, &spec,
                "conflicting wielded-item visual registration for {item_id:?}",
            );
            return;
        }
        self.specs.insert(item_id, spec);
    }
}

/// Composition-time registration sugar for provider-owned wielded-item art.
pub trait WieldedItemVisualAppExt {
    fn register_wielded_item_visual(
        &mut self,
        item_id: impl Into<String>,
        spec: WieldedItemVisualSpec,
    ) -> &mut Self;
}

impl WieldedItemVisualAppExt for App {
    fn register_wielded_item_visual(
        &mut self,
        item_id: impl Into<String>,
        spec: WieldedItemVisualSpec,
    ) -> &mut Self {
        self.init_resource::<WieldedItemVisualCatalog>();
        self.world_mut()
            .resource_mut::<WieldedItemVisualCatalog>()
            .register(item_id.into(), spec);
        self
    }
}

#[derive(Component)]
pub struct WieldedItemVisual;

/// Rebuild the provider-authored wielded-item overlays from the sim read model.
pub fn sync_wielded_item_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    catalog: Res<WieldedItemVisualCatalog>,
    wielded_items: Res<HostileWieldedItemsView>,
    existing: Query<Entity, With<WieldedItemVisual>>,
    mut textures: Local<BTreeMap<String, Handle<Image>>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    for fact in &wielded_items.0 {
        let Some(spec) = catalog.get(&fact.item_id) else {
            continue;
        };
        let texture = textures
            .entry(spec.texture_path.clone())
            .or_insert_with(|| asset_server.load(spec.texture_path.clone()))
            .clone();
        let frame_size = spec.source_rect.size();
        let anchor_x_norm = (spec.grip_px.x - frame_size.x * 0.5) / frame_size.x;
        // Image pixels grow downward while Bevy anchor Y grows upward.
        let anchor_y_norm = -(spec.grip_px.y - frame_size.y * 0.5) / frame_size.y;

        let width = spec.width_per_wielder_height * fact.wielder_height;
        let render_size = Vec2::new(width, width * frame_size.y / frame_size.x);
        let hand_world = fact.hand_world;
        let bevy_angle = dy_world_to_bevy_angle(
            fact.aim_world.x - hand_world.x,
            fact.aim_world.y - hand_world.y,
        );
        let translation = world_to_bevy(&world.0, hand_world, WORLD_Z_PLAYER + spec.z_bias);

        let mut sprite = Sprite::from_image(texture);
        sprite.custom_size = Some(render_size);
        sprite.rect = Some(spec.source_rect.clone());

        commands.spawn_session_scoped(
            session_scope,
            (
                sprite,
                Anchor(Vec2::new(anchor_x_norm, anchor_y_norm)),
                Transform {
                    translation,
                    rotation: Quat::from_rotation_z(bevy_angle),
                    scale: Vec3::ONE,
                },
                WieldedItemVisual,
                Name::new(spec.entity_name.clone()),
            ),
        );
    }
}

/// Convert world-space aim deltas (+Y down) to a Bevy Z rotation (+Y up).
fn dy_world_to_bevy_angle(dx_world: f32, dy_world: f32) -> f32 {
    (-dy_world).atan2(dx_world)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(path: &str) -> WieldedItemVisualSpec {
        WieldedItemVisualSpec {
            texture_path: path.into(),
            source_rect: Rect::from_corners(Vec2::new(10.0, 20.0), Vec2::new(40.0, 30.0)),
            grip_px: Vec2::new(5.0, 6.0),
            width_per_wielder_height: 0.75,
            z_bias: 0.5,
            entity_name: "Test wielded item".into(),
        }
    }

    #[test]
    fn catalog_accepts_identical_registration_and_rejects_conflicts() {
        let mut app = App::new();
        app.register_wielded_item_visual("wand", spec("sprites/wand.png"));
        app.register_wielded_item_visual("wand", spec("sprites/wand.png"));
        assert_eq!(
            app.world()
                .resource::<WieldedItemVisualCatalog>()
                .get("wand")
                .expect("registered visual")
                .texture_path,
            "sprites/wand.png",
        );
    }

    #[test]
    #[should_panic(expected = "conflicting wielded-item visual registration")]
    fn catalog_rejects_conflicting_registration() {
        let mut app = App::new();
        app.register_wielded_item_visual("wand", spec("sprites/wand.png"));
        app.register_wielded_item_visual("wand", spec("sprites/other.png"));
    }

    #[test]
    #[should_panic(expected = "width_per_wielder_height must be finite and positive")]
    fn catalog_rejects_non_finite_geometry() {
        let mut invalid = spec("sprites/wand.png");
        invalid.width_per_wielder_height = f32::NAN;
        let mut app = App::new();
        app.register_wielded_item_visual("wand", invalid);
    }

    #[test]
    fn aim_along_positive_x_is_zero_angle() {
        let angle = dy_world_to_bevy_angle(1.0, 0.0);
        assert!(angle.abs() < 1.0e-6, "got {angle}");
    }

    #[test]
    fn aim_along_negative_y_world_is_quarter_turn_up_in_bevy() {
        let angle = dy_world_to_bevy_angle(0.0, -1.0);
        assert!(
            (angle - std::f32::consts::FRAC_PI_2).abs() < 1.0e-5,
            "got {angle}",
        );
    }
}
