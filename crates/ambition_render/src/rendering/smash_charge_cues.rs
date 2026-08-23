//! The two AUDIBLE beats of a held smash: the latch and the lock.
//!
//! A charge has three readings — latched, building, loaded — and the pulse on
//! the body overlay carries the middle one continuously. The other two are
//! EDGES, and an edge wants a sound: the mechanical latch the instant the hold
//! takes, and a short higher lock the instant it reaches maximum. Each fires
//! exactly once per charge.
//!
//! The edge is detected HERE, against the previous frame's published fraction,
//! rather than in the simulation. That is deliberate and it is what keeps the
//! cue safe under rollback: a resimulated tick re-publishes the same fraction,
//! and this pass runs once per rendered FRAME off the read-model, so a rewind
//! cannot make a latch fire twice. Nothing here is rollback state.
//!
//! Both roads are read through one key, because a charge is a fact about a
//! BODY and the fighter that has one may be either an id-keyed actor (every
//! seat in the Smash demo) or a player-bodied entity (the exploration road).

use bevy::prelude::*;

use ambition_sfx::{ids, SfxMessage, SfxWriter};

/// Which body a remembered charge belongs to.
///
/// Two variants because the two presentation roads key their read-models
/// differently, not because a charge means anything different on either. One
/// map keyed by this beats two maps that have to be kept in step.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum ChargeKey {
    /// An id-keyed actor — every seated fighter.
    Feature(String),
    /// A player-bodied entity, whose read-model rides the entity itself.
    Body(Entity),
}

/// What this pass remembers about one charge in flight.
#[derive(Default)]
struct ChargeMemory {
    /// The lock has already sounded for THIS charge. Holding past maximum buys
    /// nothing, so it must not keep saying so.
    locked: bool,
}

/// Live charges, by body. Presentation-only: no rollback weight, and an entry
/// dies with the charge that made it.
#[derive(Default)]
pub struct SmashChargeCueState {
    live: std::collections::HashMap<ChargeKey, ChargeMemory>,
}

/// The fraction at or above which a charge counts as LOADED.
///
/// Not `>= 1.0` exactly: the fraction is a ratio of accumulated hold to the
/// authored maximum, and a cue that waits for the last float ulp is a cue that
/// sometimes never fires.
const LOADED_FRACTION: f32 = 0.999;

/// Sound the latch and the lock for every body holding a smash.
pub fn emit_smash_charge_cues(
    mut state: Local<SmashChargeCueState>,
    anim_frames: Res<ambition_sim_view::ActorAnimIndex>,
    poses: Query<(Entity, &ambition_sim_view::BodyPoseView)>,
    mut sfx: SfxWriter,
) {
    // Collected into one stream so the edge rule below is written once. The
    // iteration order of the actor index is a hash order, which is safe here
    // for the reason the index's own note gives: this is presentation, no sim
    // state reads it, and the cues it emits are unordered anyway.
    let actors = anim_frames.iter().filter_map(|(id, frame)| {
        frame
            .smash_charge
            .map(|charge| (ChargeKey::Feature(id.to_string()), frame.pos, charge))
    });
    let bodies = poses.iter().filter_map(|(entity, pose)| {
        pose.smash_charge
            .map(|charge| (ChargeKey::Body(entity), pose.pos, charge))
    });

    let mut still_charging: std::collections::HashSet<ChargeKey> = std::collections::HashSet::new();
    for (key, pos, charge) in actors.chain(bodies) {
        let fresh = !state.live.contains_key(&key);
        if fresh {
            // THE LATCH — the hold took. This is the edge a player needs to
            // hear, because until it lands they cannot tell a held button from
            // a dropped input.
            sfx.write(SfxMessage::Play {
                id: ids::PLAYER_SMASH_CHARGE_LATCH,
                pos,
            });
        }
        let memory = state.live.entry(key.clone()).or_default();
        if !memory.locked && charge >= LOADED_FRACTION {
            memory.locked = true;
            sfx.write(SfxMessage::Play {
                id: ids::PLAYER_SMASH_CHARGE_LOADED,
                pos,
            });
        }
        still_charging.insert(key);
    }
    // A charge that ended — released, cancelled, or whose body left the
    // world — is forgotten, so the NEXT one latches again. Retaining on
    // presence rather than clearing on a release event is what makes a
    // despawned fighter cost nothing.
    state.live.retain(|key, _| still_charging.contains(key));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sfx::OwnedSfxMessage;
    use ambition_sim_view::{ActorAnimFrame, ActorAnimIndex};

    /// The latch sounds once when the hold takes, the lock sounds once when it
    /// fills, and holding past maximum says nothing more.
    #[test]
    fn each_beat_sounds_once_per_charge() {
        let mut app = harness();

        // Not charging: silence.
        set_charge(&mut app, None);
        assert!(cues(&mut app).is_empty());

        // The hold takes.
        set_charge(&mut app, Some(0.0));
        assert_eq!(cues(&mut app), vec![ids::PLAYER_SMASH_CHARGE_LATCH]);

        // Building: the pulse carries this beat, not the audio.
        for f in [0.2, 0.5, 0.9] {
            set_charge(&mut app, Some(f));
            assert!(cues(&mut app).is_empty(), "building must stay quiet at {f}");
        }

        // Loaded.
        set_charge(&mut app, Some(1.0));
        assert_eq!(cues(&mut app), vec![ids::PLAYER_SMASH_CHARGE_LOADED]);

        // Held past maximum: nothing more to say.
        for _ in 0..4 {
            set_charge(&mut app, Some(1.0));
            assert!(cues(&mut app).is_empty());
        }
    }

    /// A released charge is forgotten, so the next one latches again. Without
    /// the retain the second smash of a match would be silent.
    #[test]
    fn the_next_charge_latches_again() {
        let mut app = harness();
        set_charge(&mut app, Some(0.0));
        assert_eq!(cues(&mut app), vec![ids::PLAYER_SMASH_CHARGE_LATCH]);

        set_charge(&mut app, None);
        assert!(cues(&mut app).is_empty(), "a release is not a cue");

        set_charge(&mut app, Some(0.0));
        assert_eq!(cues(&mut app), vec![ids::PLAYER_SMASH_CHARGE_LATCH]);
    }

    /// A charge that starts already full still says both things, in order —
    /// the frame budget is not a place to lose the latch.
    #[test]
    fn a_charge_that_arrives_full_latches_and_locks() {
        let mut app = harness();
        set_charge(&mut app, Some(1.0));
        assert_eq!(
            cues(&mut app),
            vec![
                ids::PLAYER_SMASH_CHARGE_LATCH,
                ids::PLAYER_SMASH_CHARGE_LOADED
            ]
        );
    }

    /// Two fighters charging at once are two charges, not one.
    #[test]
    fn each_body_latches_for_itself() {
        let mut app = harness();
        set_charges(&mut app, &[("seat_0", 0.0), ("seat_1", 0.0)]);
        assert_eq!(cues(&mut app).len(), 2);

        // Only one of them fills.
        set_charges(&mut app, &[("seat_0", 1.0), ("seat_1", 0.4)]);
        assert_eq!(cues(&mut app), vec![ids::PLAYER_SMASH_CHARGE_LOADED]);
    }

    fn harness() -> App {
        let mut app = App::new();
        app.init_resource::<ActorAnimIndex>();
        app.add_message::<OwnedSfxMessage>();
        app.add_systems(Update, emit_smash_charge_cues);
        app
    }

    fn set_charge(app: &mut App, charge: Option<f32>) {
        match charge {
            Some(charge) => set_charges(app, &[("seat_0", charge)]),
            None => set_charges(app, &[]),
        }
    }

    /// Rebuild the actor index the way the sim pass does — mark, write, sweep —
    /// so a body that stops charging really loses its row.
    fn set_charges(app: &mut App, charges: &[(&str, f32)]) {
        let rows: Vec<(String, ActorAnimFrame)> = charges
            .iter()
            .map(|(id, charge)| {
                (
                    (*id).to_string(),
                    ActorAnimFrame {
                        anim: ambition_sprite_sheet::character::CharacterAnim::Idle,
                        pos: ambition_platformer2d_core::Vec2::ZERO,
                        facing: 1.0,
                        clip: None,
                        smash_charge: Some(*charge),
                    },
                )
            })
            .collect();
        *app.world_mut().resource_mut::<ActorAnimIndex>() = ActorAnimIndex::from_rows(rows);
        app.update();
    }

    fn cues(app: &mut App) -> Vec<ambition_sfx::SfxId> {
        app.world_mut()
            .resource_mut::<Messages<OwnedSfxMessage>>()
            .drain()
            .filter_map(|m| match m.request {
                SfxMessage::Play { id, .. } => Some(id),
                _ => None,
            })
            .collect()
    }
}
