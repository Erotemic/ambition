use ambition_platformer2d_shared_tangle::{construction::SpawnOrigin, sim_id::SimId};

/// Provenance survives a snapshot round trip, which is what makes healing
/// possible at all for a blob-rebuilt entity: nothing else in the world can
/// still say where it came from.
#[test]
fn spawn_origin_round_trips_through_its_snapshot_codec() {
    use crate::codec::SnapshotState;

    let cases = [
        SpawnOrigin::Authored {
            source: "hall".into(),
            instance: "pickup_a".into(),
        },
        SpawnOrigin::ProviderStaged {
            provider: "duel".into(),
            room: "hall".into(),
            instance: "duel_red".into(),
        },
        SpawnOrigin::Dynamic {
            parent: SimId::placement("boss_1"),
            sequence: 9,
        },
        // A deeply-nested parent: the id it carries is opaque to the codec, so
        // a grammar with delimiters in it must survive verbatim rather than
        // being re-split on the way back.
        SpawnOrigin::Dynamic {
            parent: SimId::from_snapshot("placement:boss_1/3".to_string()),
            sequence: 0,
        },
    ];
    for origin in cases {
        let mut bytes = Vec::new();
        origin.encode(&mut bytes);
        let mut reader = crate::codec::Reader::new(&bytes);
        assert_eq!(
            SpawnOrigin::decode(&mut reader).as_ref(),
            Some(&origin),
            "provenance must survive the blob it is rebuilt from"
        );
    }
}

#[test]
fn attack_gesture_history_round_trips_through_rollback_codec() {
    use crate::{decode_state, encode_state};
    use ambition_characters::actor::attack_gesture::{
        AttackDir, AttackGestureIntent, AttackGestureState, AttackInputPhase, AttackPosture,
        AttackStrength, RecentAttackFlick,
    };

    let intent = AttackGestureIntent {
        direction: AttackDir::Forward,
        strength: AttackStrength::Smash,
        posture: AttackPosture::Airborne,
        phase: AttackInputPhase::Press,
    };
    // A DIFFERENT intent in the buffered slot, so a codec that wrote the two
    // fields in the wrong order would fail rather than round-trip.
    let buffered = AttackGestureIntent {
        direction: AttackDir::Down,
        strength: AttackStrength::Tilt,
        posture: AttackPosture::Grounded,
        phase: AttackInputPhase::Press,
    };
    let state = AttackGestureState {
        flick_armed: false,
        recent_flick: Some(RecentAttackFlick {
            direction: AttackDir::Forward,
            age_ticks: 2,
        }),
        active: Some(intent),
        buffered_press: Some(buffered),
        buffered_special: Some(
            ambition_characters::actor::attack_gesture::SpecialGestureIntent {
                direction: AttackDir::Up,
                posture: AttackPosture::Airborne,
            },
        ),
        // ⛔⛔ NON-DEFAULT ON PURPOSE. These two fields arrived with the
        // special-turn window and this file did not compile for however long —
        // nothing gates this crate's test target, and `cargo check -p
        // ambition_app --all-targets` does not build it. Filling them with
        // `0` / `0.0` would make the file compile and test nothing; a codec that
        // forgets to encode them fails here instead.
        special_turn_ticks: 3,
        prev_lateral_sign: -1.0,
    };
    let bytes = encode_state(&state);
    assert_eq!(decode_state::<AttackGestureState>(&bytes), Some(state));
}
