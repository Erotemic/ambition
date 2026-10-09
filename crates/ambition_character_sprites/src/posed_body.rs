//! Derive body and sprite geometry from per-pose sprite-sheet metrics.
//!
//! A sheet row supplies the occupied hurtbox rectangle in pixels.
//! [`SpritePosedBody::world_per_pixel`] scales that rectangle and the full frame into
//! world units so collision size, render size, and sprite offset stay coherent. Resizes
//! keep the gravity-side foot face fixed. The simulation uses authored
//! [`ActorAnimOverride`] rather than presentation locomotion selection, and resolves
//! geometry before movement.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::character::sheets::SpritePosedBody;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

use ambition_combat::components::{ActorRenderSize, ActorSpriteOffset};

// ⛔⛤ **`PosedBodyGeometry` / `posed_body_geometry` / `authored_body_pixel_size`
// MOVED DOWN TO `ambition_sprite_sheet` — 2026-09-21.** They are a pure
// projection of the BAKED SHEET REGISTRY (`record_for_sheet_key`), which that
// crate owns, and nothing about them is an ECS concern. The move is not
// tidying: `ambition_body_seed` builds a character's INITIAL body and cannot
// see this crate, so while the resolution lived here the seed had to derive
// its own answer from the catalog join — two derivations of one authored
// scale, which is the defect this move exists to make unrepresentable. Both
// roads now ask one function.
pub use ambition_sprite_sheet::character::sheets::{
    authored_body_pixel_size, posed_body_geometry, PosedBodyGeometry,
};

/// The art-to-world scale that `id`'s catalog row names in `posed_body`, or
/// `None` when the row states no posed body.
///
/// Registration asks this once to build the body, and anything else that needs
/// the scale (a level sizing a gap to a body) asks the same function, so there
/// is one derivation of each scale.
///
/// A reference to a character outside the catalog, a row with no sheet or no
/// standing height where the scale needs one, or a cycle of references is a
/// refusal: the body would otherwise be built at an invented size. A sheet with
/// no baked art (a headless fixture) reads 1.0, because nothing resolves a body
/// from absent art.
pub fn posed_body_world_per_pixel(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalogData,
    id: &str,
) -> Option<f32> {
    use ambition_characters::actor::character_catalog::PosedBodyScale;
    let row = |id: &str| {
        catalog.characters.get(id).unwrap_or_else(|| {
            panic!("a posed body scale names `{id}`, which is not in the same catalog")
        })
    };
    let sheet = |id: &str| {
        row(id).manifest_target().unwrap_or_else(|| {
            panic!("a posed body scale names `{id}`, whose row states no sheet")
        })
    };
    // The idle body in sheet pixels; `None` when the art is not baked.
    let idle = |id: &str| {
        posed_body_geometry(sheet(id), CharacterAnim::Idle, 1.0)
            .map(|geometry| geometry.collision)
            .filter(|pixels| pixels.x > 0.0 && pixels.y > 0.0)
    };
    let mut visited = Vec::new();
    let mut current = id.to_string();
    let scale = loop {
        if visited.contains(&current) {
            panic!("the posed body scales of {visited:?} name each other in a cycle");
        }
        visited.push(current.clone());
        let Some(scale) = row(&current).posed_body.clone() else {
            if current == id {
                return None;
            }
            panic!("`{id}` takes its body scale from `{current}`, whose row states no posed_body");
        };
        match scale {
            PosedBodyScale::SameAs(other) => current = other,
            scale => break scale,
        }
    };
    let world_per_pixel = match scale {
        PosedBodyScale::OwnHeight => {
            let entry = row(&current);
            let height = entry
                .standing_height
                .or_else(|| entry.body_kind.default_standing_height())
                .unwrap_or_else(|| {
                    panic!("`{current}` scales its body by its standing height and states none")
                });
            idle(&current).map(|pixels| height / pixels.y)
        }
        PosedBodyScale::Width(width) => idle(&current).map(|pixels| width / pixels.x),
        PosedBodyScale::TimesAsWideAs(times, other) => {
            let other_scale = posed_body_world_per_pixel(catalog, &other).unwrap_or_else(|| {
                panic!("`{current}` takes its width from `{other}`, whose row states no posed_body")
            });
            match (idle(&other), idle(&current)) {
                (Some(ruler), Some(pixels)) => Some(times * ruler.x * other_scale / pixels.x),
                _ => None,
            }
        }
        // The one scale that does not ask the art: it holds without a baked sheet.
        PosedBodyScale::WorldPerPixel(world_per_pixel) => return Some(world_per_pixel),
        PosedBodyScale::SameAs(_) => unreachable!("followed above"),
    };
    Some(world_per_pixel.unwrap_or(1.0))
}

/// The semantic body rig `id`'s sheet publishes (`<sheet>_body_rig.ron`), in
/// world units at the scale the body is drawn at, or `None` when the sheet
/// publishes no rig or the row states no scale.
///
/// The scale is the row's `posed_body`, else its standing height (the row's,
/// else its body kind's default) over the sheet's idle body: the scale
/// `sprite_body_collision_for_sheet` draws the sheet at. A row sized only by
/// its placement box has no character-wide scale. A rig without the body's own
/// scale would put a hand where a body of some other size keeps it, so such a
/// row gets no rig rather than a guess.
///
/// # Panics
///
/// When the published rig does not parse. It is generated by the sprite
/// publisher; a file this build cannot read is a stale or broken publish, and
/// the body would otherwise lose its rig without a word.
pub fn published_body_rig(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalogData,
    id: &str,
) -> Option<ambition_characters::actor::BodyRigDefinition> {
    let sheet = catalog.characters.get(id)?.manifest_target()?;
    let text = ambition_sprite_sheet::baked_body_rigs::baked_body_rig(sheet)?;
    let world_per_pixel = posed_body_world_per_pixel(catalog, id).or_else(|| {
        let row = catalog.characters.get(id)?;
        let height = row
            .standing_height
            .or_else(|| row.body_kind.default_standing_height())?;
        ambition_sprite_sheet::character::catalog_join::standing_world_per_pixel(sheet, height)
    })?;
    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
        .unwrap_or_else(|error| panic!("`{id}`'s published body rig `{sheet}`: {error}"));
    Some(rig.scaled(world_per_pixel))
}

/// The hurtbox a row's `hurtbox_insets` states, built on the idle body its
/// `posed_body` scales, or `None` when the row states no insets.
///
/// ⛔ **A STANCE MOVES THE CENTRE, AND THE VOLUME IS PLACED AT THE CENTRE.**
/// The insets are fractions of a box, but they are baked to world offsets here
/// and `hurtbox_world_aabb` puts them at `pos`. A crouch halves the box and
/// slides `pos` toward the feet, so a volume measured against the STANDING box
/// would hang a quarter of the standing height through the floor. So the crouch
/// gets its own profile, inset from the box a crouching body wears, which is
/// asked of `BodyMode::shape` so that the two cannot disagree about what
/// crouching means.
///
/// # Panics
///
/// When the row states insets and no `posed_body`, or its sheet has no baked
/// idle body: there is then no box to inset from. Both catalog readers refuse
/// the first (`validator::findings`), so only a catalog that skipped them gets
/// here.
pub fn posed_body_inset_hurtboxes(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalogData,
    id: &str,
) -> Option<ambition_entity_catalog::HurtboxDoc> {
    let row = catalog.characters.get(id)?;
    let insets = row.hurtbox_insets?;
    let world_per_pixel = posed_body_world_per_pixel(catalog, id).unwrap_or_else(|| {
        panic!("`{id}` insets its hurtbox from a body its row does not scale: state a posed_body")
    });
    let pixels = row
        .manifest_target()
        .and_then(|sheet| posed_body_geometry(sheet, CharacterAnim::Idle, 1.0))
        .map(|geometry| geometry.collision)
        .filter(|pixels| pixels.x > 0.0 && pixels.y > 0.0)
        .unwrap_or_else(|| panic!("`{id}` insets its hurtbox from a sheet with no idle body"));
    let standing = pixels * world_per_pixel;
    Some(ambition_entity_catalog::HurtboxDoc {
        default: Some(inset_timeline(insets, standing)),
        poses: std::iter::once((
            ambition_combat::hurtbox_resolution::POSE_CROUCH.to_string(),
            inset_timeline(
                insets,
                ae::player_state::BodyMode::Crouching.shape(standing).size,
            ),
        ))
        .collect(),
        moves: Default::default(),
    })
}

/// One volume inset from a body box of size `body`, in that box's frame.
fn inset_timeline(
    insets: ambition_characters::actor::character_catalog::BodyInsets,
    body: ae::Vec2,
) -> ambition_entity_catalog::HurtboxTimeline {
    // World +y is down, so a lower hurtbox centre has a positive y offset.
    let offset = ae::Vec2::new(
        ((insets.left + (1.0 - insets.right)) * 0.5 - 0.5) * body.x,
        ((insets.top + (1.0 - insets.bottom)) * 0.5 - 0.5) * body.y,
    );
    let half_extents = ae::Vec2::new(
        (1.0 - insets.left - insets.right) * 0.5 * body.x,
        (1.0 - insets.top - insets.bottom) * 0.5 * body.y,
    );
    ambition_entity_catalog::HurtboxTimeline {
        keyframes: vec![ambition_entity_catalog::HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![ambition_entity_catalog::HurtboxVolume {
                shape: ambition_entity_catalog::VolumeShape::Rect {
                    offset: (offset.x, offset.y),
                    half_extents: (half_extents.x, half_extents.y),
                },
            }],
        }],
    }
}

/// Keep every [`SpritePosedBody`] actor's collision box, sprite quad, and quad
/// offset equal to what its sheet says about the pose it is showing.
///
/// A projection of the pose only. The standing identity box (`BodyBaseSize`)
/// and the initial quad are published with `SpritePosedBody` itself, when a
/// character's body is granted, so nothing here constructs or restores them.
///
/// Runs in the sim so the box is authoritative in a headless build, and writes
/// nothing when the geometry is unchanged — the common case, since a pose holds
/// for many ticks and `ActorRenderSize` feeds a change-detecting render index.
pub fn sync_sprite_posed_bodies(
    mut bodies: Query<(
        &SpritePosedBody,
        Option<&ActorAnimOverride>,
        &mut ae::BodyKinematics,
        &mut ActorRenderSize,
        &mut ActorSpriteOffset,
        // The STANCE, which composes with the pose rather than competing with
        // it. Absent  a body that never body-modes, and the pose IS the box.
        Option<&ae::BodyModeState>,
        // The body's OWN resolved frame, the one movement and contact read.
        // Absent only on a bare fixture body, which stands under the default.
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    for (posed, pinned, mut kin, mut render_size, mut offset, body_mode, frame) in &mut bodies {
        let anim = pinned.map_or(CharacterAnim::Idle, |o| o.0);
        let Some(geometry) = posed_body_geometry(&posed.target, anim, posed.world_per_pixel) else {
            continue;
        };
        // The pose says how big the body IS; the MODE says what it is doing with
        // it, and the box is the composition of the two. The stance must be
        // re-applied here rather than left to the transition: the transition
        // writes the shorter box once, on the tick the mode changes, while this
        // pass writes every tick — so publishing `geometry.collision` alone
        // would stand a crouching body back up inside its own crouch.
        //
        // `BodyMode::shape` is the same function the stance transition uses, so
        // the two cannot disagree about what crouching means. It applies to the
        // POSE's rectangle, not to `base_size`, so a body whose silhouette
        // changes shape crouches from the height it is actually showing.
        let posed_collision = body_mode.map_or(geometry.collision, |mode| {
            mode.body_mode.shape(geometry.collision).size
        });
        // In a reversed or horizontal-gravity room a resize anchored to world +y
        // pushed the body into or off its own support. The module's contract is
        // that the +gravity face stays planted; the direction has to be the
        // body's, not the default's.
        let gravity_dir = frame.map_or(ae::DEFAULT_GRAVITY_DIR, |frame| frame.down());
        if kin.size != posed_collision {
            // Feet-anchored, through the engine's one feet-planted resize op: hold the +gravity
            // face and move the centre by half the change, so a withdraw/emerge never drives the
            // body through the ground it is standing on.
            ae::resize_feet_planted(&mut kin, posed_collision, gravity_dir);
        }
        if render_size.0 != geometry.render {
            render_size.0 = geometry.render;
        }
        // The stance moved the body's CENTRE without moving its FEET, and the
        // quad is placed relative to that centre — so the placement owes the
        // same shift back, or the art is drawn where a STANDING body's centre
        // would have put it: a quarter of the body's height into the floor for
        // a half-height crouch.
        //
        // `geometry.sprite_offset` answers "where does the frame go so the
        // POSE's rectangle lands on the box", and that box is the one the SHEET
        // measured. `resize_feet_planted` then slid the centre by half the
        // stance shrink along gravity, so the same term reverses it. Both facts
        // are published from here, by the one pass that knows both.
        let stance_shift = gravity_dir * ((geometry.collision - posed_collision) * 0.5);
        let sprite_offset = geometry.sprite_offset - stance_shift;
        if offset.0 != sprite_offset {
            offset.0 = sprite_offset;
        }
    }
}

#[cfg(test)]
mod tests;
