//! Fixture builders shared by preparation's own tests and the registration
//! tests one crate up.
//!
//! behind `test-support` for the same reason
//! [`crate::prepared::prepare_and_finalize_for_test`] is, and for no other:
//! the monolith's registration tests need the SAME `mary_o` this crate's
//! preparation tests use, and a fixture builder copied into two crates drifts
//! into two different characters that share a name. These build nothing but
//! authored data — there is no barrier to bypass here.

use std::collections::BTreeMap;

use crate::actor::definition::CharacterDefinition;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, MoveEvent, MoveEventKind, MoveGates, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};

/// A move that emits one cue and carries one strike sound on its hit volume.
pub fn slash(id: &str, cue: &str, strike: &str) -> MoveSpec {
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        shot_damage: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![MoveEvent {
            at_s: 0.1,
            kind: MoveEventKind::Sfx {
                cue: cue.to_string(),
            },
        }],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                // An ordinary hit, not a gust.
                shape: VolumeShape::Rect {
                    offset: (10.0, 0.0),
                    half_extents: (8.0, 8.0),
                },
                damage: 1,
                knockback: 0.0,
                knockback_growth: None,
                launch_dir: None,
                on_hit: None,
                vfx: Some("slash_arc".to_string()),
                hit_sfx: Some(strike.to_string()),
                reaction: None,
                guard: Default::default(),
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates::default(),
        start_impulse: None,
        charge_mult: 1.0,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        move_charge: None,
        repeat: None,
        flow: None,
    }
}

pub fn moveset_with(verbs: &[(&str, &str)], moves: Vec<MoveSpec>) -> MovesetContract {
    MovesetContract {
        verbs: verbs
            .iter()
            .map(|(v, m)| (v.to_string(), m.to_string()))
            .collect::<BTreeMap<_, _>>(),
        moves,
    }
}

pub fn mary_o() -> CharacterDefinition {
    CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
        .with_sheet("super_mary_o_spritesheet")
        .with_moveset(moveset_with(
            &[("attack", "stomp")],
            vec![slash("stomp", "mary_o.stomp", "mary_o.stomp.land")],
        ))
}


/// Body-complete fixture cast for tests that need registered characters but do
/// not care which creatures they are.
pub fn fixture_cast(ids: &[&str]) -> crate::prepared::PreparedCharacterRegistry {
    let mut registry = crate::prepared::PreparedCharacterRegistry::default();
    for id in ids {
        let mut definition = crate::actor::definition::CharacterDefinition::new(*id, *id, "test")
            .with_locomotion(crate::actor::CharacterLocomotion {
                run_speed: 155.0,
                move_style: crate::brain::MoveStyleSpec::Walk,
                ..Default::default()
            })
            .with_contact_damage(crate::actor::ContactDamage {
                strength: 0.70,
                amount: 1,
            })
            .with_autonomous_profile(crate::brain::BrainProfile {
                patrol_effort: 0.6774,
                chase_effort: 1.0,
                aggro_radius: 460.0,
                attack_range: 150.0,
                ..Default::default()
            });
        definition.vitals.max_health = Some(4);
        let finalized = crate::prepared::prepare_and_finalize_for_test(
            definition,
            &crate::prepared::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
    }
    registry
}
