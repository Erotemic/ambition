//! §7.11 runtime: the selection order, and the headless prohibition.

use super::*;
use ambition_entity_catalog::{HurtboxKeyframe, HurtboxTimeline, VolumeShape};
use std::collections::BTreeMap;

fn boxes(half_w: f32, half_h: f32) -> Vec<HurtboxVolume> {
    vec![HurtboxVolume {
        shape: VolumeShape::Rect {
            offset: (0.0, 0.0),
            half_extents: (half_w, half_h),
        },
    }]
}

fn timeline(frames: &[(f32, f32)]) -> HurtboxTimeline {
    HurtboxTimeline {
        keyframes: frames
            .iter()
            .map(|(at_s, half_h)| HurtboxKeyframe {
                at_s: *at_s,
                volumes: boxes(6.0, *half_h),
            })
            .collect(),
    }
}

fn half_height(resolved: &ResolvedHurtboxes) -> f32 {
    match resolved.volumes.first().expect("one volume").shape {
        VolumeShape::Rect { half_extents, .. } => half_extents.1,
        VolumeShape::Circle { radius, .. } => radius,
    }
}

/// A doc with all three sources authored, each a distinguishable height.
fn full_doc() -> HurtboxDoc {
    HurtboxDoc {
        default: Some(timeline(&[(0.0, 10.0)])),
        poses: BTreeMap::from([
            (POSE_HITSTUN.to_string(), timeline(&[(0.0, 20.0)])),
            (POSE_AIRBORNE.to_string(), timeline(&[(0.0, 30.0)])),
        ]),
        moves: BTreeMap::from([(
            "swat".to_string(),
            // A timeline, not a single box: the move's silhouette CHANGES.
            timeline(&[(0.0, 40.0), (0.2, 50.0)]),
        )]),
    }
}

/// The §4.11 precedence, and which source won.
#[test]
fn hurtbox_selection_prefers_move_override_then_pose_profile() {
    let doc = full_doc();

    // A move is active AND the body is in hitstun: the move override wins.
    let during_move = resolve_hurtboxes(&doc, Some(("swat", 0.05)), Some((POSE_HITSTUN, 0.5)));
    assert_eq!(during_move.source, HurtboxSelection::MoveOverride);
    assert_eq!(half_height(&during_move), 40.0);

    // No move: the pose profile answers.
    let hitstun = resolve_hurtboxes(&doc, None, Some((POSE_HITSTUN, 0.5)));
    assert_eq!(hitstun.source, HurtboxSelection::PoseProfile);
    assert_eq!(half_height(&hitstun), 20.0);

    // A pose with no authored profile falls through to the default, rather than
    // to nothing — an unauthored pose must not make a body invulnerable.
    let unauthored_pose = resolve_hurtboxes(&doc, None, Some(("crouch", 0.1)));
    assert_eq!(unauthored_pose.source, HurtboxSelection::Default);
    assert_eq!(half_height(&unauthored_pose), 10.0);

    // A MOVE with no authored override also falls through, and lands on the pose
    // rather than skipping straight to default.
    let unauthored_move =
        resolve_hurtboxes(&doc, Some(("uppercut", 0.1)), Some((POSE_AIRBORNE, 0.0)));
    assert_eq!(unauthored_move.source, HurtboxSelection::PoseProfile);
    assert_eq!(half_height(&unauthored_move), 30.0);
}

/// The move timeline is sampled on the MOVE clock, piecewise-constant.
#[test]
fn a_move_override_is_sampled_on_the_move_clock() {
    let doc = full_doc();
    for (t, expected) in [(0.0, 40.0), (0.19, 40.0), (0.2, 50.0), (5.0, 50.0)] {
        let resolved = resolve_hurtboxes(&doc, Some(("swat", t)), None);
        assert_eq!(
            half_height(&resolved),
            expected,
            "move clock {t}s must select the keyframe at or before it"
        );
    }
    // A tiny numerical underflow at state entry must not select a different
    // profile, so negative clamps to the first keyframe.
    assert_eq!(
        half_height(&resolve_hurtboxes(&doc, Some(("swat", -0.0001)), None)),
        40.0
    );
}

/// The prohibition, asserted. (§4.11)
///
/// The resolver is a pure function of an authored doc plus two clocks. There is no
/// asset, no `AssetServer`, no texture, no animator, and no renderer anywhere in
/// its inputs — `ambition_platformer2d_actor_monolith` cannot even name `ambition_render` — so a
/// headless rollout and a windowed playtest compute the same volumes by
/// construction rather than by care.
///
/// Hurtboxes do not get to repeat it.
#[test]
fn hurtboxes_exist_headless_without_a_decoded_sheet() {
    let doc = full_doc();
    // No App, no plugins, no assets, no window — and a full answer.
    let resolved = resolve_hurtboxes(&doc, Some(("swat", 0.25)), Some((POSE_AIRBORNE, 0.0)));
    assert_eq!(resolved.source, HurtboxSelection::MoveOverride);
    assert_eq!(half_height(&resolved), 50.0);
    assert_eq!(resolved.volumes.len(), 1);

    // And through the real ECS system, in an App with a MinimalPlugins-free world:
    // still no asset pipeline of any kind.
    let mut app = App::new();
    app.add_systems(Update, resolve_body_hurtboxes);
    let body = app
        .world_mut()
        .spawn((AuthoredHurtboxes(doc), ResolvedHurtboxes::default()))
        .id();
    app.update();
    let resolved = app
        .world()
        .get::<ResolvedHurtboxes>(body)
        .expect("resolved");
    assert_eq!(
        resolved.source,
        HurtboxSelection::Default,
        "no move and no pose clock: the authored default answers, headless"
    );
    assert_eq!(half_height(resolved), 10.0);
}

/// Multiple volumes per keyframe work from day one, even though early content
/// authors one rectangle — retrofitting this later means touching every character.
#[test]
fn a_keyframe_may_carry_several_volumes() {
    let doc = HurtboxDoc {
        default: Some(HurtboxTimeline {
            keyframes: vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![
                    HurtboxVolume {
                        shape: VolumeShape::Rect {
                            offset: (0.0, -8.0),
                            half_extents: (5.0, 8.0),
                        },
                    },
                    // A circle beside a rect: both shapes, one body.
                    HurtboxVolume {
                        shape: VolumeShape::Circle {
                            offset: (0.0, 6.0),
                            radius: 5.0,
                        },
                    },
                ],
            }],
        }),
        poses: BTreeMap::new(),
        moves: BTreeMap::new(),
    };
    let resolved = resolve_hurtboxes(&doc, None, None);
    assert_eq!(resolved.volumes.len(), 2);
}

/// An UNAUTHORED body is distinguishable from one that authored no volumes.
///
/// The first keeps its sprite-derived compatibility box; the second would be
/// invulnerable. Collapsing them is how a body silently stops being hittable.
#[test]
fn unauthored_is_not_the_same_as_authored_empty() {
    let nothing = resolve_hurtboxes(&HurtboxDoc::default(), None, None);
    assert_eq!(nothing.source, HurtboxSelection::Unauthored);
    assert!(nothing.volumes.is_empty());

    let authored_empty = resolve_hurtboxes(
        &HurtboxDoc {
            default: Some(HurtboxTimeline {
                keyframes: vec![HurtboxKeyframe {
                    at_s: 0.0,
                    volumes: vec![],
                }],
            }),
            poses: BTreeMap::new(),
            moves: BTreeMap::new(),
        },
        None,
        None,
    );
    assert_eq!(
        authored_empty.source,
        HurtboxSelection::Default,
        "an authored empty keyframe is a DECISION (invulnerable here), and must not \
         read as 'nobody authored anything'"
    );
    assert!(authored_empty.volumes.is_empty());
}

/// every documented pose must be REACHABLE, and every reachable pose
/// documented. The trap this pins is not a crash: a pose id that appears in
/// the vocabulary but that no simulation fact can produce lets content author a
/// profile for it, pass validation, and be silently ignored forever. Four ids
/// (`tumble`, `shield`, `ledge_hang`, `run`) sat in the module doc in exactly
/// that state, and `crouch` did too until it was given a fact.
///
/// The check is exhaustive over `body_pose`'s whole input space, so a new pose
/// name cannot land without a branch that produces it, and a deleted branch
/// cannot leave its name behind.
#[test]
fn the_pose_vocabulary_is_exactly_what_the_engine_can_write() {
    let mut reachable = std::collections::BTreeSet::new();
    for hitstun in [false, true] {
        for crouching in [false, true] {
            for airborne in [false, true] {
                reachable.insert(body_pose(hitstun, crouching, airborne));
            }
        }
    }
    let documented: std::collections::BTreeSet<&str> = BODY_POSES.iter().copied().collect();
    assert_eq!(
        reachable, documented,
        "⛔ a pose named in BODY_POSES that no fact produces is authored, valid, \
         and silently never selected"
    );
}

/// POISON: a body doing nothing must NOT read as crouching, or every idle silhouette in the game
/// changes.
#[test]
fn a_crouched_body_selects_its_crouch_profile() {
    assert_eq!(body_pose(false, true, false), POSE_CROUCH);
    assert_eq!(body_pose(false, false, false), POSE_IDLE);
    // Hitstun is a reaction the body did not choose; it outranks the stance.
    assert_eq!(body_pose(true, true, false), POSE_HITSTUN);
    // A stance change is what moved the silhouette, so it outranks mere altitude.
    assert_eq!(body_pose(false, true, true), POSE_CROUCH);
}
