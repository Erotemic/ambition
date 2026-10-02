//! Cut-rope boss arena rules.
//!
//! The arena is authored in LDtk as ordinary `Prop` entities with kind
//! `cut_rope_rope` and `cut_rope_anvil`, plus a `BossSpawn` whose behavior id
//! is `smirking_behemoth_boss`. The mechanic is tied to authored level data,
//! not hard-coded coordinates: cutting the rope starts the anvil falling, and
//! the anvil impact sends the boss through the normal death pipeline.

#![allow(unused_imports)]
use bevy::prelude::*;
use bevy::sprite::Anchor;

/// This boss's claim on the encounter layer's priority music tier.
pub const CUT_ROPE_MUSIC_OWNER: &str = "cut_rope_boss";

use ambition_boss_encounter::BossConfig;
use ambition_boss_encounter::{BossClusterQueryData, BossClusterRef, BossRef};
use ambition_boss_encounter::{BossEncounterRegistry, ReleaseOnDeath};
use ambition_characters::brain::BossAttackState;
use ambition_characters::control::ActorControl;
use ambition_combat::components::{
    CenteredAabb, DamageableVolumes, FeatureId, FeatureName, PogoPolicy,
    PogoTargetVolumes, PostBossNpc,
};
use ambition_combat::{GameplayBanner, HitEvent, HitSource, RoomReplayAdmitted};
use ambition_platformer2d::world::rooms::{PropSpec, RoomSet};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_actor_spawn::actor_bundles::{EnemyActorBundle, FeatureRenderedBundle};
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_render::rendering::PropVisual;
use ambition_sfx::SfxMessage;
use ambition_sprite_sheet::character::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::{FireworksRequest, FxRequest, ParticleKind, VfxMessage, VfxWriter};

pub const CUT_ROPE_BOSS_ID: &str = "smirking_behemoth_boss";
pub const CUT_ROPE_VICTORY_NPC_ID: &str = "smirking_behemoth_victory_npc";
pub const CUT_ROPE_VICTORY_NPC_DIALOGUE_ID: &str = "smirking_behemoth_victory_npc";
const CUT_ROPE_ROOM_ID: &str = "you_have_to_cut_the_rope";
const CUT_ROPE_VICTORY_NPC_NAME: &str = "The Rope Appreciator";
const CUT_ROPE_VICTORY_NPC_W: f32 = 28.0;
const CUT_ROPE_VICTORY_NPC_H: f32 = 48.0;
const ROPE_KIND: &str = "cut_rope_rope";
const ANVIL_KIND: &str = "cut_rope_anvil";
const PIANO_KIND: &str = "cut_rope_piano";
const ANVIL_Z_OFFSET: f32 = 0.75;
const ROPE_SPARK_INTERVAL: f32 = 0.22;

pub fn is_cut_rope_boss(id: &str) -> bool {
    id == CUT_ROPE_BOSS_ID
}

// The replay request is the engine's generic
// `session::reset::RoomReplayRequested`; content emits it.

/// The player chose "try again".
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutRopeRoomReplayRequested;

/// Latched once the player chooses the replay option. The room reset waits
/// until the conversation is over, so the final NPC line stays visible until
/// the player dismisses it.
///
/// `refight` is latched when the replay is requested and taken by the next
/// admitted replay of the cut-rope room: the re-fight the player asked for.
/// ⚠ A request the lifecycle refuses leaves it latched for the next admitted
/// replay of that room.
///
/// This is rollback state because it spans ticks: the choice is made while the
/// last line is on screen, and the reset happens an unbounded number of ticks
/// later. Without it, a rewind across the choice would keep the intention and a
/// rewind across the reset would lose it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingCutRopeRoomReplay {
    pub requested: bool,
    pub refight: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CutRopeHeavyObjectKind {
    Anvil,
    Piano,
}

impl CutRopeHeavyObjectKind {
    const fn prop_kind(self) -> &'static str {
        match self {
            Self::Anvil => ANVIL_KIND,
            Self::Piano => PIANO_KIND,
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Anvil => "anvil",
            Self::Piano => "piano",
        }
    }
}

const CUT_ROPE_HEAVY_OBJECT_CYCLE: [CutRopeHeavyObjectKind; 2] =
    [CutRopeHeavyObjectKind::Anvil, CutRopeHeavyObjectKind::Piano];

/// Tracks which heavy object is currently hanging from the cut-rope trap.
///
/// It lives outside [`CutRopeBossArenaState`] so leaving and re-entering the
/// room rebuilds transient fall/rope state without changing the chosen prop.
/// The choice advances only on a room reset, so the variation is deterministic.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutRopeHeavyObjectCycle {
    index: usize,
}

impl Default for CutRopeHeavyObjectCycle {
    fn default() -> Self {
        Self { index: 0 }
    }
}

impl CutRopeHeavyObjectCycle {
    /// Canonical projection for the session checksum.
    ///
    /// The cycle index decides which prop the arena rebuilds, and
    /// `reset_cut_rope_boss_arena_on_room_reset` changes it with `advance()`, so it
    /// must hash its value, not only its presence.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_u64};
        let Self { index } = self;
        let mut bytes = Vec::new();
        put_u64(&mut bytes, *index as u64);
        checksum_bytes(&bytes)
    }

    fn current(&self) -> CutRopeHeavyObjectKind {
        CUT_ROPE_HEAVY_OBJECT_CYCLE[self.index % CUT_ROPE_HEAVY_OBJECT_CYCLE.len()]
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % CUT_ROPE_HEAVY_OBJECT_CYCLE.len();
    }

    /// Stable Yarn-facing id for the currently hung heavy object.
    pub fn current_dialogue_id(&self) -> &'static str {
        self.current().display_name()
    }
}

/// Convert a dialogue-authored replay choice into the engine's generic
/// [`RoomReplayRequested`](ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested)
/// once the conversation is over. `AmbitionBossContentPlugin` registers it in
/// the engine's `ContentDialogueFollowupSet` slot, so the host never names this
/// system. The conversation authority decides when the conversation is over.
pub fn emit_cut_rope_room_replay_after_the_conversation_ends(
    conversation: Res<ambition_conversation::ActiveConversation>,
    mut chosen: MessageReader<CutRopeRoomReplayRequested>,
    mut pending: ResMut<PendingCutRopeRoomReplay>,
    mut replay_requests: MessageWriter<
        ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested,
    >,
) {
    if chosen.read().next().is_some() {
        pending.requested = true;
    }
    if !pending.requested || conversation.is_live() {
        return;
    }
    pending.requested = false;
    pending.refight = true;
    replay_requests
        .write(ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested::manual());
}

/// Reset the Smirking Behemoth encounter so the room can be replayed in-place.
///
/// The boss's live state is entity-local, so the real reset happens on the
/// replay road: `sandbox_reset::admit_room_replay` names the controlled body
/// and emits `RoomReplayAdmitted`, and canonical construction rebuilds the
/// room. This helper only clears the persisted "cleared" record (so the
/// rebuilt boss is not marked defeated) and restores the intro music from the
/// read-only profile catalog. The victory NPC hides again as a result of that
/// same record, because its spawn gate reads the placement state (see
/// `victory.rs`).
///
/// "Cleared" is keyed by placement (the boss's `config.id`), so the caller
/// passes the cut-rope boss placement ids in the room.
pub fn reset_cut_rope_boss_attempt(
    registry: &BossEncounterRegistry,
    save: Option<&mut ambition_persistence::save::AmbitionGameSave>,
    music_request: Option<&mut ambition_encounter::EncounterMusicRequest>,
    // The live room replayed: the intro is claimed in its music only.
    room: Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>,
    placement_ids: &[String],
) {
    let intro_track = registry
        .profile(CUT_ROPE_BOSS_ID)
        .map(|profile| profile.encounter.music_intro.clone());
    if let Some(save) = save {
        let data = save.data_mut();
        for placement_id in placement_ids {
            data.set_boss(
                placement_id,
                ambition_persistence::save_data::PersistedEncounterState::Untouched,
            );
        }
        // The line above makes the post-boss conversation wait for the next kill.
        // `spawn_cut_rope_victory_npc` gates the NPC on `PayloadReleased` this frame
        // (a fresh kill) or the placement reading `Cleared` (a re-entry); see
        // `victory.rs`. Setting the placement to `Untouched` closes both.
        //
        // Do not add a separate "victory NPC seen" flag: the save record already
        // answers that question, and a second authority can disagree with it.
    }
    if let Some(music) = music_request {
        match intro_track.filter(|track| !track.is_empty()) {
            Some(track) => music.claim_priority(room, CUT_ROPE_MUSIC_OWNER, track),
            None => music.release_priority(room, CUT_ROPE_MUSIC_OWNER),
        }
    }
}

/// Release the cut-rope boss's music claim in each room that is not its arena.
///
/// `reset_cut_rope_boss_attempt` claims `CUT_ROPE_MUSIC_OWNER` for the intro
/// track on an admitted room replay, which is what a death is. Its only
/// release is inside that same one-shot. Without this system, a player who
/// dies here and walks out keeps the Smirking Behemoth's intro, and
/// `EncounterMusicRequest::desired_track` ranks that tier above room music.
///
/// A claim released only by the system that took it is released only while
/// that system runs, and a one-shot stops running. The generic boss system
/// follows the same rule: it has no run condition, so it reaches its "no boss
/// is fighting" arm every frame.
///
/// It releases only its own claim (`release_priority` is owner-checked), so a
/// conversation, a demo death cue or the generic boss owner keep theirs.
pub fn release_cut_rope_music_outside_its_room(
    // Every live room: the claim stays in each live room that is the boss's.
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_encounter::EncounterMusicRequest,
        >,
    >,
) {
    let Some(mut music) = music else {
        return;
    };
    music.release_priority_where(CUT_ROPE_MUSIC_OWNER, |room| {
        rooms
            .definition_named(room)
            .is_none_or(|definition| rooms.rooms().spec(definition).id != CUT_ROPE_ROOM_ID)
    });
}

/// On an admitted replay of the cut-rope room, reset the fight's per-attempt
/// content: the intro music, and, when the player asked for it ("try again"),
/// the re-fight.
///
/// ⭐ THE RE-FIGHT IS A SECOND ROAD WITH ITS OWN OWNER, NOT AN EXCEPTION TO THE
/// REPLAY RULE. A replay retracts a boss defeat only when the defeat fell
/// after the last checkpoint, for every family, on the generic boss road
/// (`ambition_boss_encounter::retract_boss_defeats_on_replay`, Q56). "Try
/// again" is a choice the content authored: the player asked to fight this
/// boss again. So it puts the cut-rope placements of the replayed room back
/// to `Untouched` whatever side of the checkpoint the defeat fell on. Its
/// reward behaviour is what it was: only the record. This system no longer
/// retracts a defeat on any other replay.
///
/// Keyed by the replay's live room (`RoomReplayAdmitted` names its subject's).
/// It read every live `BossConfig`, so with two live rooms a replay in one
/// retracted a cut-rope fight in the other.
pub fn reset_cut_rope_attempt_on_replay(
    // The admitted replay, not the request. This retracts a persisted defeat;
    // doing that on a request the lifecycle might refuse would retract a defeat
    // for a replay that never happens.
    mut replays: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    mut pending: ResMut<PendingCutRopeRoomReplay>,
    registry: Res<BossEncounterRegistry>,
    mut save: Option<ResMut<ambition_persistence::save::AmbitionGameSave>>,
    mut music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_encounter::EncounterMusicRequest,
        >,
    >,
    bosses: Query<(Entity, &BossConfig)>,
) {
    for replay in replays.read() {
        let replayed = replay.subject.as_ref().and_then(|subject| subject.room);
        let Some(definition) = rooms.definition_named(replayed) else {
            continue;
        };
        if rooms.rooms().spec(definition).id != CUT_ROPE_ROOM_ID {
            continue;
        }
        let replayed = replayed.or_else(|| rooms.live().sole());
        let placements: Vec<String> = if std::mem::take(&mut pending.refight) {
            bosses
                .iter()
                .filter(|(entity, config)| {
                    is_cut_rope_boss(&config.behavior.id) && rooms.live().of(*entity) == replayed
                })
                .map(|(_, config)| config.id.clone())
                .collect()
        } else {
            Vec::new()
        };
        reset_cut_rope_boss_attempt(
            &registry,
            save.as_deref_mut(),
            // `Single<&mut T>` derefs to `Mut<T>`; peel the extra
            // change-detection layer to `&mut T`.
            music.as_deref_mut().map(|m| &mut **m),
            replayed,
            &placements,
        );
    }
}

/// The state the behemoth is built with: it frees its swallowed victory NPC
/// when it dies (the generic `ReleaseOnDeath`). Its fight beyond its phases is
/// the `encounter_script` of its row in `boss_profiles.ron`. See
/// [`ambition_boss_encounter::BossBirthKit`].
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, _: &ae::BodyKinematics) {
    scope.insert(ReleaseOnDeath);
}

mod arena;
mod victory;
pub use arena::*;
pub use victory::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// The authored anvil is found by its kind, not by its caption.
    ///
    /// `PropSpec.name` is the LDtk display name, used only for entity naming and
    /// the debug overlay. Matching it would let a caption edit rename or steal the
    /// prop this fight is built on. Both cut-rope props have `kind == name`, so
    /// the test needs the decoy below.
    #[test]
    fn the_authored_anvil_is_found_by_kind_and_not_by_its_caption() {
        let props = vec![
            prop_spec("iid-rope", ROPE_KIND, "cut_rope_rope"),
            // The decoy comes before the real anvil, and that order is the test: `find`
            // returns the first match, so a decoy after the anvil would never be reached
            // and the test could not fail.
            prop_spec("iid-poster", "wall_poster", ANVIL_KIND),
            prop_spec("iid-anvil", ANVIL_KIND, "a lovely anvil"),
        ];
        let found = arena::authored_prop(&props, ANVIL_KIND).expect("the anvil is authored");
        assert_eq!(
            found.id, "iid-anvil",
            "a decoration CAPTIONED `cut_rope_anvil` was taken for the anvil the \
             fight drops on the boss"
        );
    }

    /// The heavy object keeps its identity across its own re-skin.
    ///
    /// `apply_cut_rope_heavy_object_sprite` writes `PropVisual.kind` when the
    /// trap cycles anvil → piano, so a `kind` match would need to list every skin.
    /// Keyed by the authored iid, the entity is the same whatever it is drawn as,
    /// and a third heavy object needs no new arm.
    #[test]
    fn the_heavy_object_is_the_same_prop_after_it_is_re_skinned() {
        let anvil = prop_spec("iid-anvil", ANVIL_KIND, "a lovely anvil");
        let mut visual = PropVisual {
            id: anvil.id.clone(),
            kind: anvil.kind.clone(),
            name: anvil.name.clone(),
            size: bevy::prelude::Vec2::new(anvil.size.x, anvil.size.y),
            draw: anvil.draw,
            flip_y: anvil.flip_y,
        };
        assert!(
            arena::is_heavy_object(&visual, &anvil),
            "it starts as the authored anvil"
        );

        // Exactly what the cycle does to it: only `kind` moves.
        visual.kind = PIANO_KIND.to_string();

        assert_ne!(
            visual.kind, anvil.kind,
            "anti-vacuity: if the re-skin left `kind` alone, the assertion below \
             would pass without exercising anything"
        );
        assert!(
            arena::is_heavy_object(&visual, &anvil),
            "the re-skin changed which prop the arena thinks this IS, so it would \
             stop moving and hiding the object it just dropped on the boss"
        );
    }

    fn prop_spec(id: &str, kind: &str, name: &str) -> PropSpec {
        PropSpec {
            id: id.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            pos: ae::Vec2::new(0.0, 0.0),
            size: ae::Vec2::new(16.0, 16.0),
            flip_y: false,
            draw: Default::default(),
        }
    }

    /// The boss's music claim is released when the player leaves its room.
    ///
    /// `reset_cut_rope_attempt_on_replay` claims `CUT_ROPE_MUSIC_OWNER` for the
    /// intro, and a death is a room replay. `desired_track` ranks that tier above
    /// room music, so a claim that outlived the room would win everywhere.
    ///
    /// The premise is asserted first: the claim must survive while the player is
    /// in the room, or an unconditional release would pass while silencing the
    /// fight.
    #[test]
    fn the_boss_music_claim_does_not_follow_the_player_out_of_the_room() {
        let mut music = ambition_encounter::EncounterMusicRequest::default();
        music.claim_priority(None, CUT_ROPE_MUSIC_OWNER, "smirking_behemoth_intro");
        assert_eq!(
            music.desired_track(None),
            Some("smirking_behemoth_intro"),
            "premise: the claim is what makes the boss track win"
        );

        // Elsewhere: the call the system makes when the active room is not this
        // boss's.
        //
        // This test pins the claim lifetime (a released claim stops winning, and
        // the release is owner-scoped). The room predicate of
        // `release_cut_rope_music_outside_its_room` has its witness in the app:
        // `the_cut_rope_music_claim_is_released_when_no_live_room_is_its_room`.
        music.release_priority(None, CUT_ROPE_MUSIC_OWNER);
        assert_eq!(
            music.desired_track(None),
            None,
            "the boss's music claim outlived its room, so it beats room music \
             everywhere the player goes"
        );
    }

    /// It releases only its own claim. `release_priority` is owner-checked, so a
    /// conversation cue or another boss holding the tier is untouched.
    #[test]
    fn releasing_the_cut_rope_claim_does_not_silence_another_owner() {
        let mut music = ambition_encounter::EncounterMusicRequest::default();
        music.claim_priority(None, "some_other_fight", "another_track");
        music.release_priority(None, CUT_ROPE_MUSIC_OWNER);
        assert_eq!(
            music.desired_track(None),
            Some("another_track"),
            "leaving the cut-rope room cancelled a claim it does not own"
        );
    }

    #[test]
    fn is_cut_rope_boss_matches_only_the_cut_rope_id() {
        assert!(is_cut_rope_boss(CUT_ROPE_BOSS_ID));
        assert!(!is_cut_rope_boss("gnu_ton_rider"));
        assert!(!is_cut_rope_boss(""));
    }

    #[test]
    fn heavy_object_cycle_alternates_anvil_and_piano_on_advance() {
        let mut cycle = CutRopeHeavyObjectCycle::default();
        assert_eq!(cycle.current_dialogue_id(), "anvil");
        cycle.advance();
        assert_eq!(cycle.current_dialogue_id(), "piano");
        cycle.advance();
        assert_eq!(cycle.current_dialogue_id(), "anvil", "two-step cycle wraps");
    }
}
