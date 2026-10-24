//! Mary-O scenery props — the flagpole and the warp pipes, drawn from their
//! generated construction sheets instead of flat placeholder quads.
//!
//! The goal pole and the pipes are `ae::Block`s: the pole is the level-end
//! grab target (`flag.rs`) and the pipes are solid warp geometry (`warp_through
//! _secret_pipe`). Their COLLISION lives on those blocks and never moves — this
//! module only supplies the LOOK. Each block is made visually transparent (a
//! zero-alpha `art_color`, so the shared block-art never draws a smear over it)
//! and a decorative `PropSpec` is layered on top, resolved to the matching
//! `super_mary_o_*` construction sheet.
//!
//! The sheets are CONTENT, not carried in any lean asset catalog: they are the
//! rows of the pack's `data/prop_sheets.ron`, which
//! `register_mary_o_construction_props` registers, and the engine loads them
//! with the rest of the art. A headless / `--no-assets` build loads none.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::world::rooms::{PropDraw, PropSpec};

/// The pipe's mouth lip (the wide rim you stand on) — a distinct sheet from the
/// shaft so a tall pipe reads as a mouth on a body, not a stretched tube.
pub const PIPE_TOP_SPRITE: &str = "super_mary_o_pipe_top";
/// The pipe's shaft, beneath the lip.
pub const PIPE_BODY_SPRITE: &str = "super_mary_o_pipe_body";
/// The flagpole's shaft.
pub const POLE_BODY_SPRITE: &str = "super_mary_o_flag_pole_body";
/// The finial capping the pole.
pub const POLE_TOP_SPRITE: &str = "super_mary_o_flag_pole_top";
/// The banner hanging off the pole (its own row is a 4-frame wave).
pub const FLAG_SPRITE: &str = "super_mary_o_flag";
/// The clockwork lift: the `visual` a level's `MovingPlatform` names to be
/// drawn as it.
pub const LIFT_SPRITE: &str = "super_mary_o_lift";

/// The top of a pit of lava: the wave, and the light in the air over it.
pub const LAVA_TOP_SPRITE: &str = "super_mary_o_lava_top";
/// The lava under the top.
pub const LAVA_BODY_SPRITE: &str = "super_mary_o_lava_body";
/// The switch at the end of a castle: a button, up (`idle`) and down (`pressed`).
pub const SWITCH_SPRITE: &str = "super_mary_o_switch";
/// The gate past the switch: a portcullis, one row for each position from down
/// (`idle`) to up in the wall (`open`).
pub const GATE_SPRITE: &str = "super_mary_o_gate";
/// A damage volume whose name starts with this is a pit of lava: the level
/// draws it as lava (`lava_pools_of`), and not as a ball of fire.
pub const LAVA_PREFIX: &str = "lava";

/// A fully transparent block-art override. Applied to the pole/pipe collision
/// blocks so the shared per-`BlockKind` art draws nothing behind the decorative
/// prop — the prop IS the art, the block is only its hitbox.
pub const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// One decorative prop for a rectangular region given by a block's top-left
/// `min` and `size`. Blocks are min-anchored; a `PropSpec` is CENTER-anchored,
/// so convert once here.
pub fn prop_over(id: &str, kind: &str, min: ae::Vec2, size: ae::Vec2) -> PropSpec {
    PropSpec {
        id: id.to_string(),
        name: id.to_string(),
        kind: kind.to_string(),
        pos: ae::Vec2::new(min.x + size.x * 0.5, min.y + size.y * 0.5),
        size,
        flip_y: false,
        draw: PropDraw::Decoration,
    }
}

/// A prop that is part of the BUILT WORLD a body goes INSIDE, optionally
/// mirrored — what a warp pipe's art needs.
///
/// [`PropDraw::Enclosure`] makes the art fill the authored box exactly, so the
/// pipe's lip lands on the surface a body actually stands on, AND puts it in
/// front of the cast so it can swallow a body sliding through. The mirror is
/// what makes one pipe-head sheet serve both a pipe standing on the ground and
/// one hanging from a ceiling.
pub fn pipe_prop(id: &str, kind: &str, min: ae::Vec2, size: ae::Vec2, flip_y: bool) -> PropSpec {
    PropSpec {
        flip_y,
        draw: PropDraw::Enclosure,
        ..prop_over(id, kind, min, size)
    }
}

/// A prop that is BUILT WORLD but not something you get inside — a flagpole
/// shaft, a girder.
///
/// [`PropDraw::Structure`] fills the authored box exactly (the flagpole shaft
/// was drawn EIGHTEEN tiles wide because character sizing derives its width from
/// the box's LONGEST side, and the shaft's box is nine tiles TALL), while
/// staying behind the cast — a body climbing the pole has to be visible on it.
pub fn structure_prop(id: &str, kind: &str, min: ae::Vec2, size: ae::Vec2) -> PropSpec {
    PropSpec {
        draw: PropDraw::Structure,
        ..prop_over(id, kind, min, size)
    }
}

/// Register the construction sheets: the rows of the pack's
/// `data/prop_sheets.ron`, keyed by the sheet target name (which is also the
/// `PropSpec.kind` the level authors). They load with the rest of the art.
pub fn register_mary_o_construction_props(app: &mut App) {
    use ambition_platformer2d::game_assets::PropSheetsAppExt;
    app.register_prop_sheet_rows(
        ambition_platformer2d::sprite_sheet::prop_sheets::lowered_prop_sheets(crate::pack::PACK.prepared())
            .expect("the demo's pack states its prop sheets (`data/prop_sheets.ron`)"),
    );
    app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::prop_sheets(&crate::pack::PACK));
    // How the lift is laid on its platform: the pack's `platform_looks.ron`.
    {
        use ambition_platformer2d::render::rendering::moving_platforms::PlatformLooksAppExt;
        app.register_platform_look_rows(
            ambition_platformer2d::sprite_sheet::prop_sheets::lowered_platform_looks(crate::pack::PACK.prepared())
                .expect("the demo's pack states its platform looks (`data/platform_looks.ron`)"),
        );
        app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::platform_looks(&crate::pack::PACK));
    }
    // A loading zone of Mary-O has no picture and no nameplate. Her doors are
    // pipes and poles; a zone is the sky of a level that is a secret, or the
    // place where a trip ends.
    {
        use ambition_platformer2d::render::rendering::gate_portal_visuals::HiddenLoadingZonesAppExt;
        for room in crate::authored_world().rooms {
            for zone in room.loading_zones {
                app.hide_loading_zone(zone.id);
            }
        }
    }
}

/// Each pit of lava of a room: the name and the box of each damage volume the
/// level names as lava. The top of the box is the top of the lava.
pub fn lava_pools_of(room: &ambition_platformer2d::world::rooms::RoomSpec) -> Vec<(String, ae::Aabb)> {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
    let mut pools: Vec<_> = room
        .placements
        .iter()
        .filter(|placement| {
            matches!(placement.schema, PlacementSchema::Hazard(_)) && placement.name.starts_with(LAVA_PREFIX)
        })
        .map(|placement| (placement.name.clone(), placement.aabb))
        .collect();
    // Sorted so the props a level produces do not depend on placement order.
    pools.sort_by(|a, b| a.0.cmp(&b.0));
    pools
}

/// On the visual of a pit of lava: its picture is the lava props, so the
/// engine's picture of a hazard is hidden.
#[derive(Component)]
pub struct LavaVisual;

/// In a room of Mary-O a damage volume is fire: each one is drawn as the fire
/// orb, and not as the spikes that are the engine's picture of a hazard. A
/// pit of lava is drawn by its props, so its own visual is hidden.
///
/// A bar of fire is a row of damage volumes that the level puts on paths
/// about one pivot. Their art is the `fire_orb` entity sprite of the sprite
/// renderer; this names it on each hazard's visual, one time.
pub fn dress_fire(
    mut commands: Commands,
    rooms: Option<ambition_platformer2d::world::rooms::LiveRoomSpecs>,
    subject: Option<Res<ambition_platformer2d::platformer::markers::ControlledSubject>>,
    visuals: Query<
        (Entity, &ambition_platformer2d::render::rendering::FeatureVisual),
        (
            Without<ambition_platformer2d::render::rendering::EntityArt>,
            Without<LavaVisual>,
        ),
    >,
) {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
    use ambition_platformer2d::render::rendering::EntityArt;
    use ambition_platformer2d::sprite_sheet::game_assets::EntitySprite;
    let room = subject
        .and_then(|subject| subject.0)
        .and_then(|body| rooms.as_ref()?.spec_of(body));
    let Some(room) = room.filter(|room| room.metadata.mode.as_deref() == Some(crate::MARY_O_MODE)) else {
        return;
    };
    for (entity, visual) in &visuals {
        let hazard = room.placements.iter().find(|placement| {
            matches!(placement.schema, PlacementSchema::Hazard(_)) && placement.id.as_str() == visual.id
        });
        match hazard {
            Some(placement) if placement.name.starts_with(LAVA_PREFIX) => {
                commands.entity(entity).try_insert((LavaVisual, Visibility::Hidden));
            }
            Some(_) => {
                commands.entity(entity).try_insert(EntityArt(EntitySprite::FireOrb));
            }
            None => {}
        }
    }
}
