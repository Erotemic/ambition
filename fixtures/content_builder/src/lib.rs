//! A move authored from OUTSIDE the engine, by a crate that cannot build a game.
//!
//! ⭐⭐ **THE POINT IS THE DEPENDENCY LIST, NOT THE MOVE.** Fast-iteration packet
//! I1 asks whether authoring a move is a pure value computation. Before it, the
//! primitives lived in `ambition_characters`, which links Bevy, so the answer was
//! no by construction — and several demos reached them through the
//! `ambition_platformer2d` umbrella, which links the whole engine. This crate
//! authors a real multi-window move with a technique reference against
//! `ambition_entity_catalog` alone.
//!
//! ⛔ **A WORKSPACE MEMBER COULD NOT MAKE THIS CLAIM.** Cargo unifies features
//! and shares one lockfile across a workspace, so a member's `cargo tree` shows
//! whatever the union resolved and "I do not need Bevy" is unfalsifiable there.
//! This crate declares its own `[workspace]` and resolves independently, which is
//! what lets `the_authoring_closure_has_no_engine_in_it` be a real measurement.
//!
//! ⚠ **AND THE MOVE HAS TO BE A REAL ONE.** A fixture that built a `MoveSpec`
//! literal would compile against this closure while proving nothing about the
//! BUILDERS — the thing I1 moved. So it goes through `strike` (which authors the
//! startup/active/recovery timeline), `on_hit` (the technique reference) and
//! `charge` (a second window shape), and asserts the emitted values.

use ambition_content_pack::artifact::{ArtifactSection, ContentArtifact};
use ambition_entity_catalog::authoring::{charge, on_hit, strike, Charge, Strike};
use ambition_entity_catalog::move_section::{
    encode, MoveSectionData, MOVE_SECTION_KIND, MOVE_SECTION_VERSION,
};
use ambition_entity_catalog::{MoveSpec, MovesetContract, WindowTag};

/// The technique a landed hit asks the ruleset for. A KEY, never a handler — the
/// authoring crate cannot see a handler and must not need to.
pub const REBOUND_TECHNIQUE: &str = "pogo_bounce";

/// One authored move: a chargeable smash that offers a rebound where it lands.
pub fn a_chargeable_smash() -> MoveSpec {
    let m = strike(Strike {
        id: "outside_smash_forward",
        clip: "smash_forward",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (18.0, 0.0),
        half_extents: (20.0, 14.0),
        damage: 14,
        knockback: 120.0,
        knockback_growth: 0.8,
        launch_dir: None,
        on_hit: None,
    });
    let m = on_hit(m, REBOUND_TECHNIQUE);
    charge(
        m,
        Charge {
            hold_at_s: 0.10,
            max_hold_s: 1.0,
            // Committed, not banked — the field with no default, because it is
            // the one that most changes what a fighter IS.
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.4,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ THE BUILDERS RAN, not a literal. Each assertion names a field only a
    /// builder sets, so a fixture that quietly stopped calling them would fail
    /// rather than pass with a hand-written spec.
    #[test]
    fn an_outside_author_emits_a_real_multi_window_move() {
        let m = a_chargeable_smash();
        assert_eq!(m.id, "outside_smash_forward");

        let active: Vec<_> = m
            .windows
            .iter()
            .filter(|w| w.tag == WindowTag::Active)
            .collect();
        assert_eq!(active.len(), 1, "`strike` authors exactly one Active window");
        assert!(
            m.windows.len() >= 2,
            "a strike is startup/active/recovery, so one window means the \
             timeline builder did not run: {:?}",
            m.windows.iter().map(|w| w.tag.clone()).collect::<Vec<_>>()
        );

        let volume = active[0]
            .volumes
            .first()
            .expect("the Active window carries the swing's volume");
        assert_eq!(volume.damage, 14);
        assert_eq!(
            volume.on_hit.as_ref().map(|e| e.key.as_str()),
            Some(REBOUND_TECHNIQUE),
            "the technique reference did not reach the volume, so `on_hit` is \
             not what authored this"
        );

        let charge = m
            .smash_charge
            .as_ref()
            .expect("`charge` authors the hold, and nothing else here can");
        assert_eq!(charge.max_hold_s, 1.0);
    }

    /// ⛔ THE ANTI-VACUITY ARM. If the builders ever emitted an empty timeline the
    /// assertions above would still describe it in prose; this one refuses a move
    /// that spends time doing nothing.
    #[test]
    fn the_authored_move_occupies_real_time() {
        let m = a_chargeable_smash();
        assert!(
            m.duration_s > 0.5,
            "the authored smash lasts {}s, which is shorter than its own \
             startup — the timeline builder did not run",
            m.duration_s
        );
    }
}

/// Emit a loadable artifact carrying this crate's authored move table — I2's
/// "have the independent Rust builder emit this artifact".
///
/// ⭐ THE BUILDER NEVER SEES A HOST. It produces text; whether a host will ADMIT
/// that text is the host's question, asked with `ContentArtifact::admit`. Those
/// being separable is what lets content be built on a machine that has never
/// compiled the engine.
pub fn emit_artifact() -> String {
    let mut verbs = std::collections::BTreeMap::new();
    verbs.insert("attack_forward".to_string(), "outside_smash_forward".to_string());
    let mut data = MoveSectionData::new();
    data.insert(
        "outside_fighter".to_string(),
        MovesetContract {
            verbs,
            moves: vec![a_chargeable_smash()],
        },
    );
    ContentArtifact::new(vec![ArtifactSection {
        kind: MOVE_SECTION_KIND.to_string(),
        section_version: MOVE_SECTION_VERSION,
        payload: encode(&data).expect("the authored table encodes"),
    }])
    .to_ron()
    .expect("the artifact serializes")
}

#[cfg(test)]
mod artifact_tests {
    use super::*;

    /// ⛔ WHAT AN OUTSIDE AUTHOR EMITS IS SOMETHING A HOST WOULD ADMIT. Emitting
    /// text nothing accepts is the failure this arm exists to catch, and it is
    /// invisible to a round-trip test that only reads its own output.
    #[test]
    fn the_emitted_artifact_is_one_a_host_would_admit() {
        let text = emit_artifact();
        let parsed = ContentArtifact::parse(&text).expect("the emitted text parses");
        assert_eq!(
            parsed.admit(&|kind| (kind == MOVE_SECTION_KIND)
                .then_some(MOVE_SECTION_VERSION)),
            vec![],
            "the builder emitted an artifact a host would refuse"
        );
        let section = parsed
            .section(MOVE_SECTION_KIND)
            .expect("it carries a move section");
        let back = ambition_entity_catalog::move_section::decode(&section.payload)
            .expect("the payload decodes");
        assert_eq!(
            back["outside_fighter"].moves[0],
            a_chargeable_smash(),
            "the move the builder authored is not the move the artifact carries"
        );
    }
}

/// A capture kit authored from outside the engine: the grab, the pummel, the
/// throw.
///
/// ⭐⭐ **THIS IS THE ARM THE I1 MOVE ACTUALLY BOUGHT, AND IT COULD NOT HAVE BEEN
/// WRITTEN BEFORE IT.** `smash_capture` and `smash_repertoire` — the vocabulary
/// EVERY shipped fighter's table is built from — lived in `ambition_characters`,
/// which links Bevy, so an outside author could reach `strike` and `charge` and
/// nothing a real roster uses. The fixture above proved the timeline builders
/// were pure; it could not prove the technique families were, because they were
/// not reachable.
///
/// ⚠ THE PARAMS ARE THE POINT, not the three calls. Each of these authors an
/// `EffectRef` whose payload is a serialized parameter struct, and that
/// serialization is where "pure value" stops being a claim about imports and
/// becomes a claim about what crosses the wire to a host.
pub fn a_capture_kit() -> ambition_entity_catalog::smash_capture::SmashCaptureRepertoire {
    use ambition_entity_catalog::smash_capture as capture;
    capture::SmashCaptureRepertoire {
        cues: capture::CaptureCues::GENERIC,
        grab: capture::author_standing_grab(
            capture::grab_shell("outside_grab", "grab", 0.07, 0.05, 0.2),
            capture::CaptureAttemptParams {
                offset: (16.0, 0.0),
                half_extents: (14.0, 12.0),
                hold_offset: (18.0, 2.0),
            },
        ),
        pummel: capture::author_pummel(
            capture::capture_beat("outside_pummel", "attack", 0.18),
            0.08,
            capture::CapturePummelParams { damage: 3 },
        ),
        forward_throw: capture::author_throw(
            capture::capture_beat("outside_fthrow", "attack", 0.26),
            0.12,
            capture::CaptureThrowParams {
                damage: 9,
                knockback: 150.0,
                knockback_growth: 0.9,
                launch_dir: (1.0, 0.35),
            },
        ),
        back_throw: None,
        up_throw: None,
        down_throw: None,
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    use ambition_entity_catalog::smash_capture as capture;
    use ambition_entity_catalog::MoveEventKind;

    /// ⛔ THE TECHNIQUE KEYS AND THEIR PARAMETERS BOTH REACH THE SPEC. A kit whose
    /// three moves carried no effect events would satisfy any test that only
    /// counted moves, and it is exactly what a builder that stopped calling the
    /// `author_*` functions would produce.
    #[test]
    fn an_outside_author_emits_a_real_capture_kit() {
        let kit = a_capture_kit();
        let keys = |spec: &MoveSpec| -> Vec<String> {
            spec.events
                .iter()
                .filter_map(|e| match &e.kind {
                    MoveEventKind::Effect(r) => Some(r.key.clone()),
                    _ => None,
                })
                .collect()
        };
        // ⛔ THE GRAB'S TECHNIQUE IS A WINDOW `sustain_effect`, NOT AN EVENT, and
        // asserting it as an event is what this arm caught first. An instant is
        // the wrong shape for a capture attempt: the reach has to be LIVE for the
        // whole Active window or a body that walks in on frame two is not caught.
        let sustained: Vec<&str> = kit
            .grab
            .windows
            .iter()
            .filter_map(|w| w.sustain_effect.as_ref().map(|e| e.key.as_str()))
            .collect();
        assert_eq!(
            sustained,
            vec![capture::CAPTURE_ATTEMPT],
            "the grab sustains no capture attempt, so it would play, cost its \
             recovery, and catch nobody"
        );
        assert!(
            keys(&kit.pummel).contains(&capture::CAPTURE_PUMMEL.to_string()),
            "the pummel asks for no capture technique: {:?}",
            keys(&kit.pummel)
        );

        // ⭐ AND THE THROW'S PARAMETERS SURVIVE, read back the way a host reads
        // them. A key with an empty payload would pass the arms above.
        let effect = kit
            .forward_throw
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Effect(r) if r.key == capture::CAPTURE_THROW => Some(r),
                _ => None,
            })
            .expect("the throw carries its technique reference");
        let params: ambition_entity_catalog::smash_capture::CaptureThrowParams =
            effect.params.hydrate().expect("the throw parameters read back");
        assert_eq!(params.damage, 9);
        assert_eq!(params.launch_dir, (1.0, 0.35));
    }
}
