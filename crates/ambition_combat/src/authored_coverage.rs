//! ONE AUTHORITY FOR WHERE A MOVE HITS: the region the brain admits a swing
//! by is the region the strike will occupy.
//!
//! The strike spawn (`moveset::spawn_strike_volumes`) prefers, for every
//! vfx-tagged volume, the sprite's authored hit polygon for the move's clip
//! (`AuthoredAttackVolumeResolver`, placed by the feet pixel and the drawn
//! scale), and falls back to the RON shape. `MoveSpec::frame_data` knows only
//! the RON shape. Measured 2026-10-30 (pointed vs pugnacious, rung 9, three
//! runs): `polygon_air_forward`'s RON box sat at y 131..165 while its live
//! hitbox sat at y 79..125 — the brain admitted by one map and the body swung
//! by another, and `inside` whiffs were the result.
//!
//! This module derives the kit's coverage by the SAME per-volume rule, from
//! the same resolver, so the two cannot disagree. The caller supplies the
//! resolver as a closure over the body it already holds (character, size,
//! drawn quad, art floor); this module names none of them.

use ambition_entity_catalog::{MoveCoverage, MoveSpec, WindowTag};
use ambition_platformer2d_core as ae;

/// The body-local region (facing `+x`, `+y` toward the feet) this move's
/// Active volumes will occupy on this body, by the strike spawn's own rule:
/// `resolve(clip, window_start_s)` answers the authored volume for a
/// vfx-tagged volume (`None` when the sheet has no row for the clip), and the
/// RON shape stands for everything else. `None` when the move authors no
/// Active volume — the same `None` `frame_data` gives, which the scorer reads
/// as "no region", never as "cannot miss".
pub fn authored_coverage_of(
    spec: &MoveSpec,
    resolve: &dyn Fn(&str, f32) -> Option<ae::CombatVolume>,
) -> Option<MoveCoverage> {
    authored_coverage_over(spec, resolve, |_| true)
}

/// [`authored_coverage_of`] over the Active windows `keep` admits, by window
/// index. The witness of this module compares the FIRST Active window alone
/// against the volumes the body spawns when that window opens.
pub fn authored_coverage_over(
    spec: &MoveSpec,
    resolve: &dyn Fn(&str, f32) -> Option<ae::CombatVolume>,
    keep: impl Fn(usize) -> bool,
) -> Option<MoveCoverage> {
    let clip = spec.clip.clip.as_str();
    spec.windows
        .iter()
        .enumerate()
        .filter(|(index, w)| matches!(w.tag, WindowTag::Active) && keep(*index))
        .map(|(_, w)| w)
        .flat_map(|w| {
            w.volumes.iter().map(move |v| {
                let authored = v.vfx.as_ref().and_then(|_| resolve(clip, w.start_s));
                match authored {
                    Some(volume) => {
                        let b = volume.bounds();
                        MoveCoverage {
                            min: (b.min.x, b.min.y),
                            max: (b.max.x, b.max.y),
                        }
                    }
                    None => v.shape.coverage_box(),
                }
            })
        })
        .reduce(|a, b| MoveCoverage {
            min: (a.min.0.min(b.min.0), a.min.1.min(b.min.1)),
            max: (a.max.0.max(b.max.0), a.max.1.max(b.max.1)),
        })
}

/// Rewrite a kit candidate's region and reach from [`authored_coverage_of`].
/// Reach is the far side of the region along facing, as `frame_data` defines
/// it (`leading_edge_x` of the hittable volumes), so a hull that ends short
/// of its RON box shortens the admission exactly as far.
pub fn apply_authored_coverage(
    frames: &mut ambition_entity_catalog::MoveFrameData,
    spec: &MoveSpec,
    resolve: &dyn Fn(&str, f32) -> Option<ae::CombatVolume>,
) {
    if let Some(coverage) = authored_coverage_of(spec, resolve) {
        frames.reach = coverage.max.0.max(0.0);
        frames.coverage = Some(coverage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_with_one_blade() -> MoveSpec {
        let mut spec = ambition_characters::moveset_prefabs::simple_melee(&Default::default());
        spec.id = "blade".to_string();
        spec.clip.clip = "attack_side".to_string();
        for window in &mut spec.windows {
            for volume in &mut window.volumes {
                volume.vfx = Some("slash_arc".to_string());
            }
        }
        spec
    }

    /// The authored hull, when the sheet has one, REPLACES the RON box for a
    /// vfx-tagged volume; without a row the RON box stands.
    #[test]
    fn the_authored_hull_replaces_the_ron_box_only_where_the_sheet_has_a_row() {
        let spec = spec_with_one_blade();
        let ron = spec.frame_data().coverage.expect("the prefab authors a strike");
        let hull = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::new(40.0, -30.0), ae::Vec2::new(10.0, 5.0)));
        let with_row = authored_coverage_of(&spec, &|clip, _| (clip == "attack_side").then(|| hull.clone()))
            .expect("a region");
        assert_eq!((with_row.min, with_row.max), ((30.0, -35.0), (50.0, -25.0)));
        assert_ne!((with_row.min, with_row.max), (ron.min, ron.max));
        let without = authored_coverage_of(&spec, &|_, _| None).expect("a region");
        assert_eq!((without.min, without.max), (ron.min, ron.max));
    }

    /// Reach follows the region's far side, as `frame_data` defines reach.
    #[test]
    fn reach_follows_the_authored_region() {
        let spec = spec_with_one_blade();
        let mut frames = spec.frame_data();
        let hull = ae::CombatVolume::aabb(ae::Aabb::new(ae::Vec2::new(40.0, -30.0), ae::Vec2::new(10.0, 5.0)));
        apply_authored_coverage(&mut frames, &spec, &|_, _| Some(hull.clone()));
        assert_eq!(frames.reach, 50.0);
        assert_eq!(frames.coverage.map(|c| c.max.0), Some(50.0));
    }
}
