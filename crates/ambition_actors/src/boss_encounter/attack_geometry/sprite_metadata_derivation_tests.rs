//! Tests for the pixel-frame -> world-space AABB derivation used by boss attack volumes.

use super::*;
use ambition_engine_core::AabbExt;
use ambition_sprite_sheet::{NamedPixelRect, PixelRect};

/// Centered pixel bbox at frame center → world AABB at world_center.
/// The 128×128 frame with a 64×64 bbox at (32, 32) should map to
/// a world AABB at world_center with half-size = (16, 16) when the
/// world_size is (64, 64) (1:1 px/world). Tests the basic
/// pixel-frame → world-space transform.
#[test]
fn world_aabb_from_centered_pixel_rect_lands_at_world_center() {
    let bbox = PixelRect {
        x: 32,
        y: 32,
        w: 64,
        h: 64,
    };
    let world = world_aabb_from_pixel_rect(
        bbox,
        128,
        128,
        ae::Vec2::new(100.0, 200.0),
        ae::Vec2::new(64.0, 64.0),
    );
    // Center of pixel rect = (64, 64) = frame center → world
    // center should be exactly the passed world_center.
    let center = world.center();
    assert!((center.x - 100.0).abs() < 1e-3);
    assert!((center.y - 200.0).abs() < 1e-3);
    // Half-size = (64*0.5 * 0.5_scale, 64*0.5 * 0.5_scale) =
    // 16 since scale = 64/128 = 0.5.
    let half = world.half_size();
    assert!((half.x - 16.0).abs() < 1e-3);
    assert!((half.y - 16.0).abs() < 1e-3);
}

/// Off-center bbox should land off-center in world too. A bbox
/// in the top-left quadrant of the frame should produce a world
/// AABB above-and-left of the world_center.
#[test]
fn world_aabb_from_off_center_bbox_translates_correctly() {
    let bbox = PixelRect {
        x: 0,
        y: 0,
        w: 32,
        h: 32,
    };
    let world = world_aabb_from_pixel_rect(
        bbox,
        128,
        128,
        ae::Vec2::new(500.0, 500.0),
        ae::Vec2::new(64.0, 64.0),
    );
    let center = world.center();
    // Frame center is (64, 64); bbox center is (16, 16); offset
    // (-48, -48). Scaled to world by 64/128 = 0.5 → (-24, -24).
    // World center = (500 - 24, 500 - 24) = (476, 476).
    assert!((center.x - 476.0).abs() < 1e-3);
    assert!((center.y - 476.0).abs() < 1e-3);
}

/// Multi-part metadata returns one world AABB per pixel part.
/// Verifies the "disjointed character pieces" path the user
/// asked for — three named rects yield three world AABBs in the
/// same order.
#[test]
fn world_space_body_aabbs_emits_one_per_named_part() {
    let parts = vec![
        NamedPixelRect {
            name: "head".to_string(),
            x: 56,
            y: 16,
            w: 16,
            h: 16,
        },
        NamedPixelRect {
            name: "body".to_string(),
            x: 48,
            y: 32,
            w: 32,
            h: 48,
        },
        NamedPixelRect {
            name: "left_hand".to_string(),
            x: 16,
            y: 64,
            w: 16,
            h: 16,
        },
    ];
    let aabbs = world_space_body_aabbs_from_parts(
        &parts,
        None,
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(128.0, 128.0),
    );
    assert_eq!(
        aabbs.len(),
        3,
        "multi-part should produce one AABB per named part",
    );
}

/// Empty parts + present bbox falls back to single-rect path.
#[test]
fn world_space_body_aabbs_falls_back_to_single_bbox() {
    let bbox = PixelRect {
        x: 16,
        y: 16,
        w: 96,
        h: 96,
    };
    let aabbs = world_space_body_aabbs_from_parts(
        &[],
        Some(bbox),
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(128.0, 128.0),
    );
    assert_eq!(
        aabbs.len(),
        1,
        "single bbox should produce exactly one AABB",
    );
}

/// Empty parts + no bbox returns an empty list (callers fall
/// back to the legacy combat_size path).
#[test]
fn world_space_body_aabbs_empty_when_no_metadata() {
    let aabbs = world_space_body_aabbs_from_parts(
        &[],
        None,
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(128.0, 128.0),
    );
    assert!(aabbs.is_empty());
}

/// `bounding_aabb` returns a tight envelope around a list of
/// AABBs. Verifies the combat_size derivation path collapses
/// multi-part bodies into one for movement / clamping.
#[test]
fn bounding_aabb_envelops_disjoint_parts() {
    let parts = vec![
        ae::Aabb::new(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 10.0)),
        ae::Aabb::new(ae::Vec2::new(50.0, 0.0), ae::Vec2::new(10.0, 10.0)),
    ];
    let bound = bounding_aabb(&parts).expect("non-empty input");
    // Parts span x=[-10,10] and x=[40,60]; envelope x=[-10,60].
    // Y is the same [-10, 10] for both.
    assert!((bound.center().x - 25.0).abs() < 1e-3);
    assert!((bound.center().y - 0.0).abs() < 1e-3);
    let half = bound.half_size();
    assert!((half.x - 35.0).abs() < 1e-3);
    assert!((half.y - 10.0).abs() < 1e-3);
}

#[test]
fn bounding_aabb_returns_none_for_empty_input() {
    assert!(bounding_aabb(&[]).is_none());
}

/// Doubling the spawn size doubles the derived world AABB on
/// both axes (with identical sprite metadata). Pins the
/// "boss in the intro is 2× larger" change — the intro arena's
/// BossSpawn went from 64×80 → 128×160 and the runtime's
/// combat_size derived from the SAME `body_pixel_bbox` MUST
/// scale 2× in both dimensions. If this test breaks the
/// sprite-metadata-driven body math diverged from the spawn
/// AABB.
/// End-to-end pin: `damageable_volumes` MUST return the
/// per-animation hurtbox when the boss's sprite metrics
/// carries one for the current animation. If a future change
/// breaks the wire (derive doesn't copy `animations`, the
/// consumer's lookup falls through silently, the
/// boss_animation_for_profile mapping drops, etc.) the cyan
/// debug box stops growing during attacks — which is the
/// exact regression the user just reported.
///
/// Builds a fake `ActorSpriteMetrics` with a clearly-distinct
/// per-animation hurtbox for `side_sweep`, sets
/// `attack_state.active_profile = Some(SideSweep)`, and
/// asserts the consumer returns an AABB matching the wide
/// `side_sweep` hurtbox (~128 wide) rather than the static
/// `body_pixel_bbox` (~106 wide).
#[test]
fn damageable_volumes_uses_per_animation_hurtbox_during_attack() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::{BossAttackProfile, BossAttackState};
    use ambition_sprite_sheet::{AnimationBox, AnimationMetrics, PixelRect};
    use std::collections::HashMap;

    // Build a sprite-metrics snapshot with a distinct
    // `side_sweep` hurtbox (much wider than the static body
    // bbox) so we can prove the consumer picked the
    // per-animation one.
    let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
    animations.insert(
        "side_sweep".to_string(),
        AnimationMetrics {
            hurtbox: Some(AnimationBox {
                parts: Vec::new(),
                bbox: Some(PixelRect {
                    x: 1,
                    y: 5,
                    w: 127,
                    h: 86,
                }),
                poly: Vec::new(),
                frames: Vec::new(),
            }),
            hitbox: None,
            frame_duration_secs: None,
        },
    );
    let metrics = ActorSpriteMetrics {
        frame_width: 128,
        frame_height: 128,
        body_pixel_bbox: Some(PixelRect {
            x: 8,
            y: 5,
            w: 106,
            h: 83,
        }),
        body_pixel_parts: Vec::new(),
        // Match the BOSS_SHEET render: `max(boss.size) * 1.6`
        // = `160 * 1.6` = `256` for a (128,160) spawn.
        sprite_render_size: ae::Vec2::new(256.0, 256.0),
        // Test fixture: zero offset keeps `boss.aabb()` centered
        // on `boss.pos` (the pre-offset behavior) so the
        // half-size assertion below doesn't have to factor in
        // body-center bias.
        combat_offset: ae::Vec2::ZERO,
        animations,
    };

    let mut behavior = BossBehaviorProfile::clockwork_warden();
    behavior.combat_size = Some(ae::Vec2::new(54.0, 56.0));
    let mut attack_state = BossAttackState::default();
    attack_state.active_profile = Some(BossAttackProfile::Strike("side_sweep".to_string()));

    let ctx = BossVolumeContext {
        pos: ae::Vec2::new(640.0, 656.0),
        size: ae::Vec2::new(128.0, 160.0),
        combat_size: ae::Vec2::new(54.0, 56.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: None,
        facing: 1.0,
    };
    let volumes = damageable_volumes(&ctx);
    assert_eq!(volumes.len(), 1);
    let half = volumes[0].half_size();
    // side_sweep hurtbox: 127 wide / 128 frame × 256 render =
    // 254 wide. Half = 127. Static body bbox at render scale
    // would give 106/2 * 2 = 106. So we expect half.x > 120 to
    // pin the per-animation path.
    assert!(
        half.x > 120.0,
        "expected per-animation side_sweep hurtbox (wider than static body); got half.x = {} (would be ~106 if falling back to body_pixel_bbox)",
        half.x,
    );
}

/// Pin the scale-to-render-size fix: when `sprite_render_size`
/// is 2× `ctx.size`, the cyan hurtbox must be 2× bigger than
/// when `sprite_render_size` is zeroed (legacy path). Without
/// this, the user's complaint — "in the sprites the box covers
/// the boss head, but in game it is the old boxes" — comes back
/// because the visible sprite renders 1.6× bigger than `boss.size`
/// but the hurtbox would scale by `boss.size` only.
#[test]
fn damageable_volumes_samples_per_frame_hurtbox_from_animation_elapsed() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::{BossAttackProfile, BossAttackState};
    use ambition_sprite_sheet::{
        AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
    };
    use std::collections::HashMap;

    let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
    animations.insert(
        "gnu_head_descent".to_string(),
        AnimationMetrics {
            frame_duration_secs: Some(0.1),
            hurtbox: Some(AnimationBox {
                // Coarse fallback is deliberately different from
                // the sampled frame so the assertion proves that
                // the per-frame path was taken.
                parts: vec![NamedPixelRect {
                    name: "head".to_string(),
                    x: 45,
                    y: 45,
                    w: 10,
                    h: 10,
                }],
                bbox: None,
                poly: Vec::new(),
                frames: vec![
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 10,
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 30,
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                ],
            }),
            hitbox: None,
        },
    );
    let metrics = ActorSpriteMetrics {
        frame_width: 100,
        frame_height: 100,
        body_pixel_bbox: None,
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(100.0, 100.0),
        combat_offset: ae::Vec2::ZERO,
        animations,
    };
    let behavior = BossBehaviorProfile::gnu_ton();
    let mut attack_state = BossAttackState::default();
    attack_state.active_profile = Some(BossAttackProfile::Strike("head_descent".to_string()));
    attack_state.active_elapsed = 0.15; // frame index 1 at 0.1s/frame.

    let ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(100.0, 100.0),
        combat_size: ae::Vec2::new(100.0, 100.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: None,
        facing: 1.0,
    };

    let volumes = damageable_volumes(&ctx);
    assert_eq!(volumes.len(), 1);
    assert!(
        (volumes[0].center().y - -15.0).abs() < 1e-3,
        "expected frame-1 head center y=-15, got {:?}",
        volumes[0]
    );
}

#[test]
fn animation_frame_sample_overrides_elapsed_frame_for_authored_boxes() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::{BossAttackProfile, BossAttackState};
    use ambition_sprite_sheet::{
        AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
    };
    use std::collections::HashMap;

    let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
    animations.insert(
        "gnu_head_descent".to_string(),
        AnimationMetrics {
            frame_duration_secs: Some(0.1),
            hurtbox: Some(AnimationBox {
                parts: Vec::new(),
                bbox: None,
                poly: Vec::new(),
                frames: vec![
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 10,
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 70,
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                ],
            }),
            hitbox: None,
        },
    );
    let metrics = ActorSpriteMetrics {
        frame_width: 100,
        frame_height: 100,
        body_pixel_bbox: None,
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(100.0, 100.0),
        combat_offset: ae::Vec2::ZERO,
        animations,
    };
    let behavior = BossBehaviorProfile::gnu_ton();
    let mut attack_state = BossAttackState::default();
    attack_state.active_profile = Some(BossAttackProfile::Strike("head_descent".to_string()));
    attack_state.active_elapsed = 0.15; // elapsed alone would pick frame 1.
    let visual_frame = BossAnimationFrameSample {
        profile: Some(BossAttackProfile::Strike("head_descent".to_string())),
        frame_index: 0,
        animation_key: Some("gnu_head_descent"),
    };

    let ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(100.0, 100.0),
        combat_size: ae::Vec2::new(100.0, 100.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: Some(&visual_frame),
        facing: 1.0,
    };

    let volumes = damageable_volumes(&ctx);
    assert_eq!(volumes.len(), 1);
    assert!(
        (volumes[0].center().y - -35.0).abs() < 1e-3,
        "expected visual frame 0 head center y=-35, got {:?}",
        volumes[0]
    );
}

#[test]
fn idle_rest_hurtbox_follows_the_live_animation_frame() {
    // Regression for the "GNU-ton head hurtbox locks to frame 0
    // while idle" bug. At rest there is no active/telegraph
    // profile, so `damageable_volumes` used to sample the rest
    // animation at elapsed 0 → frame 0 forever, even as the
    // rendered breathing pose bobbed. An idle `BossAnimationFrameSample`
    // (`profile: None`) now feeds the live frame index through.
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::BossAttackState;
    use ambition_sprite_sheet::{
        AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
    };
    use std::collections::HashMap;

    let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
    animations.insert(
        "rest".to_string(),
        AnimationMetrics {
            frame_duration_secs: Some(0.1),
            hurtbox: Some(AnimationBox {
                parts: Vec::new(),
                bbox: None,
                poly: Vec::new(),
                frames: vec![
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 10, // center 15 → world y -35
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 45,
                            y: 70, // center 75 → world y +25
                            w: 10,
                            h: 10,
                        }],
                        bbox: None,
                    },
                ],
            }),
            hitbox: None,
        },
    );
    let metrics = ActorSpriteMetrics {
        frame_width: 100,
        frame_height: 100,
        body_pixel_bbox: None,
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(100.0, 100.0),
        combat_offset: ae::Vec2::ZERO,
        animations,
    };
    let behavior = BossBehaviorProfile::gnu_ton();
    // Fully idle: no active or telegraph profile.
    let attack_state = BossAttackState::default();

    // Without a sample, elapsed 0 locks to frame 0 (y = -35).
    let ctx0 = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(100.0, 100.0),
        combat_size: ae::Vec2::new(100.0, 100.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: None,
        facing: 1.0,
    };
    let v0 = damageable_volumes(&ctx0);
    assert_eq!(v0.len(), 1);
    assert!(
        (v0[0].center().y - -35.0).abs() < 1e-3,
        "idle without a sample should hold frame 0 (y=-35), got {:?}",
        v0[0]
    );

    // An idle sample (profile None) at frame 1 bobs the hurtbox down.
    let idle_frame = BossAnimationFrameSample {
        profile: None,
        frame_index: 1,
        animation_key: Some("rest"),
    };
    let ctx1 = BossVolumeContext {
        animation_frame: Some(&idle_frame),
        ..ctx0
    };
    let v1 = damageable_volumes(&ctx1);
    assert_eq!(v1.len(), 1);
    assert!(
        (v1[0].center().y - 25.0).abs() < 1e-3,
        "idle sample frame 1 should move the head hurtbox to y=+25, got {:?}",
        v1[0]
    );
}

#[test]
fn gnu_head_descent_accepts_visual_row_alias_for_runtime_boxes() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::{BossAttackProfile, BossAttackState};
    use ambition_sprite_sheet::{
        AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
    };
    use std::collections::HashMap;

    let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
    animations.insert(
        "head_down".to_string(),
        AnimationMetrics {
            frame_duration_secs: Some(0.1),
            hurtbox: Some(AnimationBox {
                parts: Vec::new(),
                bbox: None,
                poly: Vec::new(),
                frames: vec![
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 40,
                            y: 10,
                            w: 20,
                            h: 20,
                        }],
                        bbox: None,
                    },
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head".to_string(),
                            x: 40,
                            y: 70,
                            w: 20,
                            h: 20,
                        }],
                        bbox: None,
                    },
                ],
            }),
            hitbox: Some(AnimationBox {
                parts: Vec::new(),
                bbox: None,
                poly: Vec::new(),
                frames: vec![
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head_hit".to_string(),
                            x: 35,
                            y: 5,
                            w: 30,
                            h: 30,
                        }],
                        bbox: None,
                    },
                    AnimationBoxFrame {
                        parts: vec![NamedPixelRect {
                            name: "head_hit".to_string(),
                            x: 35,
                            y: 65,
                            w: 30,
                            h: 30,
                        }],
                        bbox: None,
                    },
                ],
            }),
        },
    );

    let metrics = ActorSpriteMetrics {
        frame_width: 100,
        frame_height: 100,
        body_pixel_bbox: None,
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(100.0, 100.0),
        combat_offset: ae::Vec2::ZERO,
        animations,
    };
    let behavior = BossBehaviorProfile::gnu_ton();
    let mut attack_state = BossAttackState::default();
    attack_state.active_profile = Some(BossAttackProfile::Strike("head_descent".to_string()));
    attack_state.active_elapsed = 0.15;
    let ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(100.0, 100.0),
        combat_size: ae::Vec2::new(100.0, 100.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: None,
        facing: 1.0,
    };

    let hurt = damageable_volumes(&ctx);
    assert_eq!(hurt.len(), 1);
    assert!(
        (hurt[0].center().y - 30.0).abs() < 1e-3,
        "expected head_down alias hurtbox frame 1 at y=30, got {:?}",
        hurt[0]
    );

    let hit = active_attack_volumes(&ctx);
    assert_eq!(hit.len(), 1);
    assert!(
        (hit[0].center().y - 30.0).abs() < 1e-3,
        "expected head_down alias hitbox frame 1 at y=30, got {:?}",
        hit[0]
    );
}

#[test]
fn damageable_volumes_scales_to_sprite_render_size() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::BossAttackState;
    use ambition_engine_core::AabbExt;
    use ambition_sprite_sheet::PixelRect;
    use std::collections::HashMap;

    let bbox = PixelRect {
        x: 8,
        y: 5,
        w: 106,
        h: 83,
    };
    let behavior = BossBehaviorProfile::clockwork_warden();
    let attack_state = BossAttackState::default();

    let legacy_metrics = ActorSpriteMetrics {
        frame_width: 128,
        frame_height: 128,
        body_pixel_bbox: Some(bbox),
        body_pixel_parts: Vec::new(),
        // Zero render size → consumer falls back to ctx.size
        // (the pre-fix behavior).
        sprite_render_size: ae::Vec2::ZERO,
        combat_offset: ae::Vec2::ZERO,
        animations: HashMap::new(),
    };
    let render_metrics = ActorSpriteMetrics {
        frame_width: 128,
        frame_height: 128,
        body_pixel_bbox: Some(bbox),
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(256.0, 256.0),
        combat_offset: ae::Vec2::ZERO,
        animations: HashMap::new(),
    };

    let legacy_ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(128.0, 160.0),
        combat_size: ae::Vec2::new(54.0, 56.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&legacy_metrics),
        animation_frame: None,
        facing: 1.0,
    };
    let render_ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(128.0, 160.0),
        combat_size: ae::Vec2::new(54.0, 56.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&render_metrics),
        animation_frame: None,
        facing: 1.0,
    };
    let legacy = damageable_volumes(&legacy_ctx)[0];
    let render = damageable_volumes(&render_ctx)[0];

    // ctx.size = (128, 160) → scale (1, 1.25) → body half (53, 51.875).
    // sprite_render_size = (256, 256) → scale (2, 2) → body half (106, 83).
    // Render must be ~2× legacy on x and ≥1.5× on y.
    let lx = legacy.half_size().x;
    let rx = render.half_size().x;
    let ly = legacy.half_size().y;
    let ry = render.half_size().y;
    assert!(
        rx > lx * 1.8,
        "sprite_render_size scaling should ~2× the x half-extent; legacy={lx} render={rx}",
    );
    assert!(
        ry > ly * 1.5,
        "sprite_render_size scaling should ≥1.5× the y half-extent; legacy={ly} render={ry}",
    );
}

#[test]
fn world_space_body_aabbs_doubles_when_spawn_doubles() {
    let bbox = PixelRect {
        x: 8,
        y: 5,
        w: 106,
        h: 83,
    };
    let half_at_1x = world_space_body_aabbs_from_parts(
        &[],
        Some(bbox),
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(64.0, 80.0),
    )[0]
    .half_size();
    let half_at_2x = world_space_body_aabbs_from_parts(
        &[],
        Some(bbox),
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(128.0, 160.0),
    )[0]
    .half_size();
    let ratio_x = half_at_2x.x / half_at_1x.x;
    let ratio_y = half_at_2x.y / half_at_1x.y;
    assert!(
        (ratio_x - 2.0).abs() < 1e-3,
        "2× spawn must produce 2× x-extent; got ratio {ratio_x}",
    );
    assert!(
        (ratio_y - 2.0).abs() < 1e-3,
        "2× spawn must produce 2× y-extent; got ratio {ratio_y}",
    );
}

/// Larger world_size (e.g. boss lab boss at 150×185 vs intro at
/// 64×80) scales the body AABB proportionally — the same pixel
/// bbox yields a bigger world AABB. This is the scaling promise
/// of the sprite-metadata-driven approach: one source of body
/// shape, multiple sizes.
#[test]
fn world_space_body_aabbs_scales_with_world_size() {
    let bbox = PixelRect {
        x: 8,
        y: 8,
        w: 112,
        h: 112,
    };
    let small = world_space_body_aabbs_from_parts(
        &[],
        Some(bbox),
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(64.0, 80.0), // first_system_boss spawn size
    );
    let large = world_space_body_aabbs_from_parts(
        &[],
        Some(bbox),
        128,
        128,
        ae::Vec2::ZERO,
        ae::Vec2::new(150.0, 185.0), // boss-lab spawn size
    );
    let small_half = small[0].half_size();
    let large_half = large[0].half_size();
    // Same fraction of the frame → large should be roughly the
    // ratio (150/64, 185/80) bigger than small.
    let ratio_x = large_half.x / small_half.x;
    let ratio_y = large_half.y / small_half.y;
    assert!((ratio_x - 150.0 / 64.0).abs() < 1e-3);
    assert!((ratio_y - 185.0 / 80.0).abs() < 1e-3);
}

// ============================================================
// "Attack inside the boss doesn't connect" investigation
// (Jon's mockingbird report, 2026-06-21).
// ============================================================

/// Hypothesis check: "when the attack hitbox is completely inside the
/// enemy hurtbox the intersection isn't flagged." It IS flagged — the hit
/// path uses `strict_intersects`, which accepts full containment (see also
/// `geometry::strict_intersects_accepts_full_containment`). So containment
/// is NOT where the bug lives; this pins that at the damageable-volume level.
#[test]
fn attack_fully_inside_boss_volume_still_registers() {
    use crate::boss_encounter::behavior::BossBehaviorProfile;
    use ambition_characters::brain::BossAttackState;

    let behavior = BossBehaviorProfile::clockwork_warden();
    let attack_state = BossAttackState::default();
    let ctx = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(500.0, 185.0),
        combat_size: ae::Vec2::new(500.0, 185.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: None, // mockingbird has no authored body_metrics -> fallback box
        animation_frame: None,
        facing: 1.0,
    };
    let damageable = damageable_volumes(&ctx);
    // A tiny attack fully inside the boss volume — "I'm in the middle and swing".
    let attack = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 20.0));
    assert!(
        damageable.iter().any(|p| attack.strict_intersects(*p)),
        "an attack fully inside the boss volume must register (containment works)",
    );
}

/// The actual bug: a boss with NO authored `body_metrics` (the mockingbird)
/// falls back to the bare `combat_size` box. That box is SMALLER than the
/// visible sprite and carries no alignment offset, so attacks that visually
/// connect with the sprite but land outside the smaller combat box miss.
/// Mockingbird: `combat_size (500x185)` but the sprite frame is `576x216`.
/// An authored hurtbox covering the visible sprite (as GNU-ton has) fixes it.
#[test]
fn mockingbird_combat_size_fallback_undershoots_the_visible_sprite() {
    use crate::boss_encounter::behavior::{ActorSpriteMetrics, BossBehaviorProfile};
    use ambition_characters::brain::BossAttackState;
    use std::collections::HashMap;

    let behavior = BossBehaviorProfile::clockwork_warden();
    let attack_state = BossAttackState::default();

    // An attack on the visible upper body: inside the 216-tall sprite
    // (±108) but ABOVE the fallback combat box (±92.5).
    let attack = ae::Aabb::new(ae::Vec2::new(0.0, -103.0), ae::Vec2::new(4.0, 4.0));

    // Current state: no authored hurtbox -> combat_size fallback (±92.5 tall).
    let ctx_fallback = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(500.0, 185.0),
        combat_size: ae::Vec2::new(500.0, 185.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: None,
        animation_frame: None,
        facing: 1.0,
    };
    assert!(
        !damageable_volumes(&ctx_fallback)
            .iter()
            .any(|p| attack.strict_intersects(*p)),
        "BUG: the bare combat_size box undershoots the visible sprite, so a hit on \
         the visible upper body misses",
    );

    // Fix: an authored body hurtbox covering the visible 576x216 sprite.
    let metrics = ActorSpriteMetrics {
        frame_width: 576,
        frame_height: 216,
        body_pixel_bbox: Some(PixelRect {
            x: 0,
            y: 0,
            w: 576,
            h: 216,
        }),
        body_pixel_parts: Vec::new(),
        sprite_render_size: ae::Vec2::new(576.0, 216.0),
        combat_offset: ae::Vec2::ZERO,
        animations: HashMap::new(),
    };
    let ctx_authored = BossVolumeContext {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(500.0, 185.0),
        combat_size: ae::Vec2::new(500.0, 185.0),
        behavior: &behavior,
        attack_state: &attack_state,
        sprite_metrics: Some(&metrics),
        animation_frame: None,
        facing: 1.0,
    };
    assert!(
        damageable_volumes(&ctx_authored)
            .iter()
            .any(|p| attack.strict_intersects(*p)),
        "with an authored body hurtbox covering the visible sprite, the same hit lands",
    );
}

#[test]
fn mirror_x_if_flipped_reflects_about_axis_only_when_facing_left() {
    // The boss sprite flips horizontally to face the player, so an off-center
    // body's hit/hurt boxes must mirror with it. This pins the reflection:
    // center.x → 2*axis - center.x when facing < 0; size + y untouched; no-op
    // when facing right.
    let boxes = vec![ae::Aabb::new(
        ae::Vec2::new(70.0, 5.0),
        ae::Vec2::new(4.0, 3.0),
    )];
    let axis = 50.0;

    let right = mirror_x_if_flipped(boxes.clone(), axis, 1.0);
    assert!(
        (right[0].center().x - 70.0).abs() < 1e-6,
        "facing right is a no-op"
    );

    let left = mirror_x_if_flipped(boxes, axis, -1.0);
    // 70 reflected about 50 → 30 (the mirror side of the boss center).
    assert!((left[0].center().x - 30.0).abs() < 1e-6);
    assert!((left[0].center().y - 5.0).abs() < 1e-6, "y unchanged");
    assert!(
        (left[0].half_size().x - 4.0).abs() < 1e-6,
        "width unchanged"
    );
}
