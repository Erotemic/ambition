//! The `projectile_visuals` schema: a game's named projectile looks, as data
//! in its pack.
//!
//! The file is a map from the visual id a projectile carries to its
//! [`ProjectileArt`]. The render and the sim resolve the id through the
//! [`ProjectileVisualCatalog`](crate::visual::ProjectileVisualCatalog), so a
//! look edited here is drawn at the next start, or after a content reload,
//! with no Rust build.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};

use crate::visual::{ProjectileArt, ProjectileArtSource, ProjectileRenderSize};

/// The capability that owns this schema.
pub const PROJECTILES_CAPABILITY: &str = "projectiles";

/// The authored FILE kind: a game's projectile looks.
pub const PROJECTILE_VISUALS_SCHEMA: &str = "projectile_visuals";

/// A game's projectile looks, by visual id.
pub type ProjectileLooks = BTreeMap<String, ProjectileArt>;

/// A look's id as a message names it: the empty id is the look of a shot that
/// names none.
fn named(id: &str) -> String {
    if id.is_empty() { "the empty id (a shot that names no look)".to_string() } else { format!("`{id}`") }
}

/// The problem with `value` at `field`, unless it is finite and in range.
fn out_of_range(id: &str, field: &str, value: f32, zero_allowed: bool) -> Option<String> {
    let in_range = value.is_finite() && if zero_allowed { value >= 0.0 } else { value > 0.0 };
    (!in_range).then(|| {
        let range = if zero_allowed { "not negative" } else { "more than 0" };
        format!("{}: `{field}` is {value}; it is finite and {range}", named(id))
    })
}

/// Each look the renderer cannot draw, named by its id and field.
fn problems(looks: &ProjectileLooks) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, art) in looks {
        let sizes: Vec<(&str, f32, bool)> = match art.size {
            // `min` 0 is no clamp.
            ProjectileRenderSize::Body { min, scale } => vec![("size.min", min, true), ("size.scale", scale, false)],
            ProjectileRenderSize::FixedWidth(width) => vec![("size", width, false)],
            ProjectileRenderSize::BodyWidth { scale } => vec![("size.scale", scale, false)],
        };
        let burst = art.expiry_vfx.map(|burst| ("expiry_vfx.scale", burst.scale, false));
        problems.extend(
            sizes
                .into_iter()
                .chain(burst)
                .filter_map(|(field, value, zero_allowed)| out_of_range(id, field, value, zero_allowed)),
        );
        let tints = match &art.source {
            ProjectileArtSource::EnergyTinted { rgba } | ProjectileArtSource::SolidColor { rgba } => vec![("source.rgba", *rgba)],
            _ => Vec::new(),
        };
        for (field, rgba) in tints.into_iter().chain([("debug_tint", art.debug_tint)]) {
            if rgba.iter().any(|channel| !(0.0..=1.0).contains(channel)) {
                problems.push(format!("{}: `{field}` is {rgba:?}; each channel is from 0 to 1", named(id)));
            }
        }
        let names_a_file = match &art.source {
            ProjectileArtSource::Image { path } => !path.trim().is_empty(),
            ProjectileArtSource::Sheet { target, animation, .. } => !target.trim().is_empty() && !animation.trim().is_empty(),
            _ => true,
        };
        if !names_a_file {
            problems.push(format!("{}: the `source` names no image or sheet row", named(id)));
        }
        if art.label.trim().is_empty() {
            problems.push(format!("{}: the `label` is empty; traces and debug names read it", named(id)));
        }
    }
    problems
}

/// The schema.
pub fn projectile_visuals_schema() -> SchemaRegistration {
    rules_schema::<ProjectileLooks>(
        PROJECTILE_VISUALS_SCHEMA,
        1,
        PROJECTILES_CAPABILITY,
        "A game's projectile looks, by the visual id a projectile carries: the \
         source of the pixels, the size, the rotation, the debug tint, the trace \
         label and the effect at expiry.",
        problems,
    )
}

/// The looks the pack states, or `None` when it has no `projectile_visuals`
/// source.
pub fn lowered_projectile_looks(pack: &PreparedContentPack) -> Option<&ProjectileLooks> {
    lowered_rules::<ProjectileLooks>(pack, PROJECTILE_VISUALS_SCHEMA)
}
