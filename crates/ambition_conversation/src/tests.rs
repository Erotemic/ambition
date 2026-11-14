//! Conversation continuity, tested beside the rules rather than in the
//! interaction module's test file — where these lived while the systems did.

use bevy::prelude::*;
use ambition_input::InputOwner;

use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::CenteredAabb;

use ambition_characters::control::{ControlHold, ControlHolds};

use super::{
    break_dialogue_on_hit_or_separation, project_conversation_hold, ActiveConversation,
    HeldByConversation,
};

fn body(app: &mut App, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            CenteredAabb::from_center_size(at, ae::Vec2::new(24.0, 24.0)),
            BodyCombat::default(),
        ))
        .id()
}

/// Two bodies standing in each other's reach, mid-conversation, with both
/// systems chained as the sim schedule chains them.
fn talking_app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    // the break ASKS for a bark now rather than writing a bubble, so the
    // fixture registers the REQUEST channel and no VFX at all. What the cast
    // says in answer is tested where the cast lives.
    app.add_message::<super::ConversationCutBark>();
    app.add_systems(
        Update,
        (
            break_dialogue_on_hit_or_separation,
            project_conversation_hold,
        )
            .chain(),
    );
    let here = ae::Vec2::new(100.0, 100.0);
    let initiator = body(&mut app, here);
    let npc = body(&mut app, here);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(super::LiveConversation::for_test(
            Some(initiator),
            Some(npc),
            "chat",
            InputOwner::Primary,
        ));
    (app, initiator, npc)
}

/// A conversation through `node`, opened on `tick`.
///
/// the tick is the fixture's whole subject in the tests below: it is what
/// tells one visit to an NPC from the next, and what a stamped narrative end is
/// matched against.
fn live_at(node: &str, tick: u64) -> super::LiveConversation {
    super::LiveConversation {
        instance: super::ConversationInstanceId::mint(
            tick,
            node,
            None,
            None,
            &ambition_dialog::DialogueContext::scripted(),
        ),
        ..super::LiveConversation::for_test(None, None, node, InputOwner::Primary)
    }
}

fn talking(app: &App) -> bool {
    app.world().resource::<ActiveConversation>().is_live()
}

/// Open a conversation the way the interaction system opens one, with the
/// initiator wearing `worn`, and hand back the instance id it minted.
///
/// through [`super::DialogueDispatch`], not by hand. The whole subject
/// below is what the OPENING derives, and a fixture that minted an id itself
/// would be asserting about its own arguments. So the speaker id comes from
/// `speaker_id()` — which, for a body with neither an `ActorInteraction` nor an
/// `ActorIdentity` (the home avatar), is the character it is WEARING.
///
/// Everything else is pinned: the same tick, the same node, the same two
/// `SimId`s, the same listener.
fn instance_wearing(worn: &str) -> super::ConversationInstanceId {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    app.insert_resource(ambition_time::SimTick(100));

    let initiator = app
        .world_mut()
        .spawn((
            SimId::placement("player"),
            ambition_characters::actor::CharacterForm::new(worn),
        ))
        .id();
    let talker = app.world_mut().spawn(SimId::placement("npc_admiral")).id();

    let mut state: bevy::ecs::system::SystemState<super::DialogueDispatch> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    {
        let mut dialogue = state.get_mut(app.world_mut()).expect("dialogue params");
        let speaker = dialogue
            .speaker_id(initiator, None, None)
            .expect("a body with no authored identity speaks as the character it wears");
        assert_eq!(
            speaker, worn,
            "precondition: the WORN character is the speaker"
        );
        assert!(dialogue.open_between(
            initiator,
            talker,
            "chat",
            "The Admiral",
            &speaker,
            "npc_admiral",
            InputOwner::Primary,
        ));
    }
    state.apply(app.world_mut());

    app.world()
        .resource::<ActiveConversation>()
        .instance()
        .expect("the conversation opened")
        .clone()
}

/// A corrected timeline that re-wears the initiator is a DIFFERENT
/// conversation.
///
/// `CharacterForm` is rollback-owned and runtime-mutable — the rollback registration's own docs
/// discuss *"a rewind that restores an EARLIER `CharacterForm`"* — and it is what decides the
/// initiator's dialogue identity for a body with no authored one. So two authoritative openings can
/// agree on the tick, the node and BOTH bodies' `SimId`s while Yarn is entered with a different
/// `$speaker_id`. Under an id built from the four body facts alone those were one conversation, and
/// two authorities keyed on that id got it wrong in opposite directions:
///
/// * the text box's projection concluded "already attached" and left Yarn's
///   variable storage carrying the ABANDONED branch's `$speaker_id`, so content
///   branching on it ran as somebody else;
/// * every retained [`super::NarrativeInputLedger`] record is instance-gated, so
///   a grant or an ending observed under the old identity matched the corrected
///   conversation.
///
/// the invariant: if two authoritative openings can make Yarn observe
/// different narrative semantics, they must not have the same instance identity.
#[test]
fn two_worn_characters_are_two_conversations() {
    assert_ne!(
        instance_wearing("mary_o"),
        instance_wearing("sanic"),
        "a corrected timeline that re-wore the initiator opened a conversation \
         Yarn enters with a different $speaker_id, and the id called it the same \
         conversation — so the abandoned branch's narrative records apply to it \
         and the text box never re-enters the node under the identity that is \
         actually speaking"
    );
}

/// And the round trip still holds: a resimulation of the opening tick, with
/// the restored `CharacterForm`, re-mints an EQUAL id.
///
/// Without it every record from the original run stops matching its own conversation on the
/// first replayed tick.
#[test]
fn re_minting_from_the_restored_identity_is_equal() {
    assert_eq!(
        instance_wearing("mary_o"),
        instance_wearing("mary_o"),
        "the same opening, resimulated, minted a different id — a narrative \
         record from the original run would no longer find its own conversation"
    );
}

/// An app with the narrative-end half of the seam wired as the sim wires it:
/// the ledger releases at the head, the closer reads what it released.
///
/// the release is a real system, not a hand-poked resource. The whole
/// claim these tests make is about WHEN a record reaches the simulation, and a
/// fixture that reached into the ledger directly would be testing the container
/// rather than the seam.
fn narrative_app(tick: u64) -> App {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<super::NarrativeInputLedger<super::ConversationEnded>>();
    app.add_message::<super::ConversationEnded>();
    app.insert_resource(ambition_time::SimTick(tick));
    app.add_systems(
        Update,
        (
            super::release_narrative_inputs::<super::ConversationEnded>,
            super::close_conversation_on_narrative_end,
        )
            .chain(),
    );
    app
}

/// Write down that `live`'s narrative finished, effective from `from_tick` —
/// what `publish_the_narrative_end` does when it sees the runner go quiet.
fn record_end(app: &mut App, live: &super::LiveConversation, from_tick: u64) {
    app.world_mut()
        .resource_mut::<super::NarrativeInputLedger<super::ConversationEnded>>()
        .record(
            live.instance.clone(),
            from_tick,
            super::ConversationEnded {
                instance: live.instance.clone(),
            },
        );
}

fn set_tick(app: &mut App, tick: u64) {
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = tick;
}

/// A resimulated tick does not close a conversation on the strength of a text
/// box from another timeline.
///
/// `DialogState` is not rewound — deliberately, because rewinding a typewriter would stutter the
/// box — so on a resimulated tick that read returned the LIVE runner rather than the runner as it
/// was. A rewind to before the conversation ended would close it again immediately, and the
/// resimulation would not reproduce the history it exists to reproduce.
///
/// the end is a STAMPED RECORD now: presentation observes the runner finishing once and
/// writes down which conversation instance ended and the tick it applies from.
///
/// what this does NOT claim: that the Yarn runner is deterministic. It is
/// content running outside the simulation, so WHICH tick it finishes on is still
/// presentation's answer. What is now true is that every replay of that tick
/// agrees with the original run.
#[test]
fn a_conversation_survives_a_tick_before_the_narrative_end_applies() {
    let mut app = narrative_app(7);
    let live = live_at("chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());

    // The narrative finished while the simulation was at tick 9, so the end
    // applies from 10. This tick is 7 — a resimulated tick BEFORE it.
    record_end(&mut app, &live, 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "a tick before the end applies must leave the conversation alone — \
         polling the view here is what let a rewind close a conversation that, \
         in the timeline being replayed, was still going"
    );

    // ...and the tick it applies from ends it, whether that is the original run
    // or the fourth replay of it.
    set_tick(&mut app, 10);
    app.update();
    assert!(
        !app.world().resource::<ActiveConversation>().is_live(),
        "the runner finishing still ends the conversation — this is a change of \
         MECHANISM, not of behaviour"
    );
}

/// Rewinding across a recorded conversation end must replay that end on the
/// same simulation tick. The end record persists outside rollback while the
/// conversation authority itself rewinds.
#[test]
fn a_rewind_past_the_end_replays_it_at_the_same_tick() {
    let mut app = narrative_app(10);
    let live = live_at("chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());
    record_end(&mut app, &live, 10);
    app.update();
    assert!(!app.world().resource::<ActiveConversation>().is_live());

    // THE REWIND: the authority is rollback state, so tick 8 restores the
    // conversation. The record is NOT, so it is still there to be replayed.
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());
    set_tick(&mut app, 8);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "tick 8 is before the end, so the replayed timeline still has a live \
         conversation — closing it here would put the hold and the input capture \
         two ticks early"
    );

    set_tick(&mut app, 10);
    app.update();
    assert!(
        !app.world().resource::<ActiveConversation>().is_live(),
        "and the replay ends it on the SAME tick the original run did — a \
         narrative end that lands at a different simulation time is a different \
         history"
    );
}

/// An end from the previous conversation does not close the next one.
///
/// the poison. A bare marker would close whatever is live when it happens to
/// be read, so a player who finished one conversation and immediately started
/// another could have the second one closed by the first one's ending.
///
/// and the node id alone is not enough, which is why the record names the
/// tick the conversation OPENED on: talk to the same NPC twice and both
/// conversations are `"chat"`.
#[test]
fn an_end_from_the_previous_conversation_does_not_close_the_next_one() {
    let mut app = narrative_app(30);

    // A different node entirely.
    let first = live_at("first_chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("second_chat", 20));
    record_end(&mut app, &first, 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "the FIRST conversation's ending closed the SECOND one — an end has to \
         name what it is ending or it is just a global 'stop whatever is running'"
    );

    // And the SAME node, talked to twice — which a node id alone cannot tell
    // apart.
    record_end(&mut app, &live_at("second_chat", 5), 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "an end for the PREVIOUS visit to this NPC closed the current one: two \
         conversations through one node are two conversations"
    );
}

/// A REWIND DOES NOT RESTART THE TEXT BOX.
///
/// the projection recognises the conversation it already opened, because a
/// restored authority carries the same `opened_at`. A conversation opened on a
/// DIFFERENT tick is a different conversation and does open the box, which is
/// the other half of the same rule.
#[test]
fn replaying_the_opening_tick_does_not_reopen_the_box() {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogState>();
    app.add_systems(Update, super::project_the_dialog_ui_from_the_conversation);

    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("chat", 5));
    app.update();
    assert!(
        app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "the simulation decided a conversation exists and the box must follow it"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_dialog::DialogState>()
            .dialogue_id(),
        "chat"
    );

    // The player finished reading and the box closed. Now a rollback restores
    // the authority — same conversation, same `opened_at` — and replays the
    // tick it opened on.
    app.world_mut()
        .resource_mut::<ambition_dialog::DialogState>()
        .close();
    app.update();
    assert!(
        !app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "the replay reopened a text box the player already watched close — this \
         is the presentation side effect a replayable system must not have"
    );

    // Talking to the same NPC AGAIN is a different conversation, and it opens.
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("chat", 40));
    app.update();
    assert!(
        app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "a second visit through the same node is a second conversation, and \
         suppressing it would leave the player looking at nothing"
    );
}

#[test]
fn changing_the_input_owner_does_not_restart_the_narrative() {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogState>();
    app.add_systems(Update, super::project_the_dialog_ui_from_the_conversation);
    let mut live = live_at("chat", 5);
    app.world_mut().resource_mut::<ActiveConversation>().open(live.clone());
    app.update();
    app.world_mut().resource_mut::<ambition_dialog::DialogState>().close();
    live.input_owner = InputOwner::Participant(ambition_input::ParticipantId(1));
    app.world_mut().resource_mut::<ActiveConversation>().open(live);
    app.update();
    let dialogue = app.world().resource::<ambition_dialog::DialogState>();
    assert!(!dialogue.active());
    assert_eq!(dialogue.input_owner(), Some(InputOwner::Participant(ambition_input::ParticipantId(1))));
}

/// Two conversations may finish inside one rollback window; both completion
/// records must survive so rewinding past both replays both. Correctness cannot
/// depend on reading speed because conversations may be scripted or auto-advancing.
#[test]
fn two_narrative_ends_in_one_window_both_replay() {
    let mut app = narrative_app(100);

    let first = live_at("first_chat", 100);
    let second = live_at("second_chat", 104);

    // The original run: A opens at 100 and finishes (applies from 103); B opens
    // at 104 and finishes (applies from 106). Well inside an eight-frame window.
    record_end(&mut app, &first, 103);
    record_end(&mut app, &second, 106);

    // THE REWIND to 101, which restores the authority to the conversation
    // that was live then — A.
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(first.clone());
    set_tick(&mut app, 101);
    app.update();
    assert!(talking(&app), "101 is before A's end; A is still going");

    set_tick(&mut app, 103);
    app.update();
    assert!(
        !talking(&app),
        "the replay lost the FIRST conversation's ending because a second one \
         overwrote it — every observation the rollback window can still reach \
         has to survive, and one slot cannot hold two"
    );
}

/// A conversation whose authority disappears and comes back gets its box
/// back.
///
/// the presentation memo said "I projected this once", and what a
/// repairable projection needs is "I am currently attached to this instance".
/// Reachable under prediction: a predicted remote hit breaks the conversation,
/// presentation closes the box at the end of that frame, the real input arrives
/// and the correction restores the SAME conversation — and the memo, which is
/// never cleared, refuses to rebuild the box. The simulation goes on holding the
/// talker and capturing a seat while the player looks at nothing.
///
/// distinct from `replaying_the_opening_tick_does_not_reopen_the_box`, which
/// is the case where the box closes while the AUTHORITY stays live. Both must
/// hold: presentation follows the authority's existence, not its own history.
#[test]
fn a_conversation_restored_after_its_authority_vanished_is_projected_again() {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogState>();
    app.add_systems(Update, super::project_the_dialog_ui_from_the_conversation);

    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("chat", 5));
    app.update();
    assert!(
        app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "precondition: the box follows the authority"
    );

    // A predicted branch broke it. Presentation observes the absence and closes.
    app.world_mut().resource_mut::<ActiveConversation>().close();
    app.update();
    assert!(
        !app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "precondition: the box closed with the conversation"
    );

    // THE CORRECTION. The real input said the hit never landed, so the SAME
    // conversation instance is authoritative again.
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("chat", 5));
    app.update();
    assert!(
        app.world()
            .resource::<ambition_dialog::DialogState>()
            .active(),
        "the authority is live and there is no text box: presentation refused to \
         rebuild a conversation it had projected once before, so the simulation \
         holds the talker and captures the seat for a conversation nobody can see"
    );
}

/// A conversation the world keeps running through can be broken.
///
/// the struck case is driven from the NPC's body, not the player's. The rule
/// is symmetric — *"both characters"* — and a test that only ever hits the
/// player would pass against a player-centric implementation.
#[test]
fn a_conversation_breaks_on_knockback_or_on_the_bodies_separating() {
    // Standing and talking: nothing breaks.
    let (mut app, _, _) = talking_app();
    app.update();
    assert!(talking(&app), "two bodies standing together keep talking");

    // the NPC is knocked about — not the player.
    let (mut app, _, npc) = talking_app();
    app.world_mut().entity_mut(npc).insert(BodyCombat {
        recoil_lock_timer: 0.2,
        ..Default::default()
    });
    app.update();
    assert!(
        !talking(&app),
        "an NPC knocked off its feet mid-sentence has ended the conversation too"
    );

    // Separation: the other body falls away.
    let (mut app, _, npc) = talking_app();
    let far =
        CenteredAabb::from_center_size(ae::Vec2::new(100.0, 900.0), ae::Vec2::new(24.0, 24.0));
    app.world_mut().entity_mut(npc).insert(far);
    app.update();
    assert!(!talking(&app), "you fell away from the parrot");

    // and damage that does NOT move you leaves it alone: a poison tick is not
    // an interruption, which is the whole reason the signal is the recoil lock
    // rather than a health change.
    let (mut app, _, npc) = talking_app();
    app.world_mut().entity_mut(npc).insert(BodyCombat {
        hit_flash: 1.0,
        ..Default::default()
    });
    app.update();
    assert!(
        talking(&app),
        "being hurt without being MOVED does not interrupt a conversation"
    );
}

/// A conversation holds the body it is talking to, and lets go afterwards.
///
/// the release is the half that bites: a stranded `ControlHolds` is a
/// permanently frozen NPC, and a conversation can end in more ways than the
/// break rule (the Yarn runner finishing, a room swap, a teardown). So the
/// projection asks the authority rather than remembering what it inserted.
#[test]
fn a_conversation_blanks_the_npcs_brain_and_releases_it_when_it_ends() {
    let (mut app, initiator, npc) = talking_app();

    app.update();
    assert!(
        app.world().get::<ControlHolds>(npc).is_some(),
        "the NPC stops answering its brain while it is talking — otherwise it \
         wanders off mid-sentence now that the world keeps running"
    );
    assert!(
        app.world().get::<ControlHolds>(initiator).is_none(),
        "the TALKER is not marked: `DIALOGUE_CONTEXT` already neutralised their \
         input, and a second mechanism on the same body would race the death beat"
    );

    // The conversation ends by any route — here, the runner finishing.
    app.world_mut().resource_mut::<ActiveConversation>().close();
    app.update();
    assert!(
        app.world().get::<ControlHolds>(npc).is_none(),
        "and it gets its brain back; a stranded marker is a frozen NPC forever"
    );
    assert!(
        app.world().get::<HeldByConversation>(npc).is_none(),
        "the claim is released with the marker it claimed"
    );
}

/// A rewind must not be able to strand the hold half-applied.
///
/// Both run in `sim_schedule()`, which under a rollback host IS the GGRS schedule, so both
/// resimulate.
///
/// So a rewind past the insert does exactly what this test does by hand: GGRS
/// restores the registered component and leaves the unregistered marker behind,
/// carried in from a future that no longer happened. What the resimulated tick
/// then sees is a body that is already "held" — and an insert gated on that
/// marker declines to restore the control override it stands for.
///
/// the assertion is about the PAIR, not about either component. A hold
/// that is a marker plus an override has to move as one thing or it is not one
/// thing.
#[test]
fn a_rewind_cannot_leave_a_conversation_holding_a_body_it_no_longer_controls() {
    let (mut app, _, npc) = talking_app();

    app.update();
    assert!(
        app.world().get::<ControlHolds>(npc).is_some(),
        "precondition: the conversation took the hold in the first place"
    );

    // THE REWIND. Not a simulated one — this is precisely what a GGRS
    // `LoadWorld` does with a snapshot taken before the insert: every
    // rollback-registered component is restored to its snapshot state (so the
    // override goes away), and everything else is left exactly as the abandoned
    // future left it (so the marker stays).
    app.world_mut()
        .entity_mut(npc)
        .remove::<ControlHolds>();
    assert!(
        app.world().get::<HeldByConversation>(npc).is_some(),
        "precondition: the unregistered marker is what survives the rewind"
    );

    // The resimulated tick.
    app.update();

    assert!(
        app.world().get::<ControlHolds>(npc).is_some(),
        "after a rewind the NPC is still marked as held by the conversation, so \
         the hold has to still BE a hold — otherwise it is holding station on \
         the strength of a marker while its brain drives it away"
    );
}

/// The conversation's reconcile never strips another claimant's control.
///
/// `ControlHolds` has six owners now — the death beat, the flagpole, act
/// clear, versus, seating, and this. The projection sweeps bodies it does not
/// hold, so the question "could that sweep take somebody else's override" has to
/// have an answer that is checked rather than reasoned about.
///
/// It does not, because the sweep is scoped by [`HeldByConversation`] and only
/// this module ever writes that — and because the release clears one bit of
/// [`ControlHolds`] and cannot clear another. the second half is the
/// poison: a body wearing a STALE conversation marker — the exact thing a
/// rewind leaves behind — alongside another claimant's live hold is the case
/// where a marker-blind sweep would do damage, and it is the case a test written
/// only from the happy path would never construct.
#[test]
fn a_conversation_hold_never_strips_another_claimants_control() {
    let (mut app, _, _) = talking_app();

    // Somebody else's held body: a death beat's, with no conversation marker.
    // Held through its OWN claim, the way every authority holds a body now.
    let dying = body(&mut app, ae::Vec2::new(900.0, 900.0));
    app.world_mut()
        .entity_mut(dying)
        .insert(ControlHolds::only(ControlHold::Sequence));

    app.update();
    assert!(
        app.world().get::<ControlHolds>(dying).is_some(),
        "the conversation swept a body it never claimed — every other owner of \
         `ControlHolds` marks the body a PLAYER is driving, and taking one \
         back mid-death-beat unfreezes a corpse"
    );

    // THE POISON: a stale conversation marker on that same body, which is
    // what a rewind past a hold leaves behind. The sweep MUST clear the marker
    // it owns and release the bit it owns — and MUST leave the death beat's
    // hold, and the override that projects it, exactly where they were.
    app.world_mut().entity_mut(dying).insert(HeldByConversation);
    app.update();
    assert!(
        app.world().get::<HeldByConversation>(dying).is_none(),
        "a stale claim is cleared rather than left to accumulate — the marker is \
         a projection, so anything the authority does not name loses it"
    );
    assert!(
        app.world().get::<ControlHolds>(dying).is_some(),
        "clearing a STALE conversation marker took the death beat's hold with \
         it — the sweep released a bit it never claimed"
    );
    assert_eq!(
        app.world().get::<ControlHolds>(dying).copied(),
        Some(ControlHolds::only(ControlHold::Sequence)),
        "the death beat's claim did not survive somebody else's release"
    );
}

/// Three bodies in one conversation: the opening pair and one that joined
/// beside them, with the threat rule, the continuity rule and the hold
/// chained as the sim schedule chains them.
fn three_talking(nodes: &[&str]) -> (App, Entity, Entity, Entity) {
    let (mut app, initiator, npc) = talking_app();
    let mut index = ambition_dialog::DialogueNodeIndex::default();
    index.populate(nodes.iter().map(|node| node.to_string()));
    app.insert_resource(index);
    app.insert_resource(ambition_time::SimTick(40));
    app.add_systems(Update, super::interrupt_a_conversation_on_a_threat.before(break_dialogue_on_hit_or_separation));
    // Within earshot of the pair, and out of the talk reach of either.
    let third = body(&mut app, ae::Vec2::new(180.0, 100.0));
    assert!(app.world_mut().resource_mut::<ActiveConversation>().join(third));
    (app, initiator, npc, third)
}

fn members(app: &App) -> Vec<Entity> {
    app.world().resource::<ActiveConversation>().participants().collect()
}

/// A body that joins is held as the talker is, and is let go when it leaves.
#[test]
fn a_body_that_joins_is_held_and_let_go_when_it_leaves() {
    let (mut app, initiator, npc, third) = three_talking(&[]);
    app.update();
    assert_eq!(members(&app), vec![initiator, npc, third], "a third body in earshot stays in it");
    assert!(app.world().get::<HeldByConversation>(third).is_some(), "the joined body is held");
    assert!(app.world().get::<HeldByConversation>(initiator).is_none(), "the initiator is not held");

    assert_eq!(
        app.world_mut().resource_mut::<ActiveConversation>().leave(third),
        super::Leaving::Left,
        "two are left, so the pair talks on"
    );
    app.update();
    assert!(talking(&app));
    assert!(app.world().get::<HeldByConversation>(third).is_none(), "the body that left is let go");
    assert!(
        app.world().get::<ControlHolds>(third).is_none_or(|holds| !holds.holds(ControlHold::Conversation)),
        "and its conversation hold is released"
    );
}

/// Each member leaves by the rules on its own: struck, out of earshot, or
/// dead. The others talk on while two are left.
#[test]
fn a_member_struck_out_of_earshot_or_dead_leaves_and_the_rest_talk_on() {
    let (mut app, initiator, npc, third) = three_talking(&[]);
    app.world_mut().entity_mut(third).insert(BodyCombat { hitstun_timer: 0.2, ..Default::default() });
    app.update();
    assert_eq!(members(&app), vec![initiator, npc], "the struck body left; the pair talks on");

    let (mut app, initiator, npc, third) = three_talking(&[]);
    let far = CenteredAabb::from_center_size(ae::Vec2::new(100.0 + super::EARSHOT + 64.0, 100.0), ae::Vec2::new(24.0, 24.0));
    app.world_mut().entity_mut(third).insert(far);
    app.update();
    assert_eq!(members(&app), vec![initiator, npc], "the body out of earshot left");

    // The talker dies: the conversation goes on between the initiator and the
    // body that joined, with no talker.
    let (mut app, initiator, npc, third) = three_talking(&[]);
    let mut health = ambition_characters::actor::Health::new(10);
    health.current = 0;
    app.world_mut().entity_mut(npc).insert(ambition_characters::actor::BodyHealth::new(health));
    app.update();
    assert_eq!(members(&app), vec![initiator, third], "the dead talker left, and the others talk on");
    assert_eq!(app.world().resource::<ActiveConversation>().talker(), None);
    assert!(app.world().get::<HeldByConversation>(npc).is_none(), "the dead body is let go");

    // The initiator struck ends it for everybody.
    let (mut app, initiator, _, third) = three_talking(&[]);
    app.world_mut().entity_mut(initiator).insert(BodyCombat { hitstun_timer: 0.2, ..Default::default() });
    app.update();
    assert!(!talking(&app), "the initiator's leaving ends the conversation");
    assert!(app.world().get::<HeldByConversation>(third).is_none());
}

/// A hostile body that comes within earshot of a conversation whose node
/// has an interruption authored breaks in: it joins, held, and the narrative
/// goes on from `<node>__interrupted` as a new instance. A second hostile
/// joins without starting the interruption again; without the node, nothing
/// changes.
#[test]
fn a_threat_in_earshot_breaks_into_a_conversation_with_an_interruption_authored() {
    use ambition_combat::components::ActorDisposition;

    let (mut app, _, _, _) = three_talking(&["chat", "chat__interrupted"]);
    let opened = app.world().resource::<ActiveConversation>().instance().cloned().expect("live");
    let raider = body(&mut app, ae::Vec2::new(20.0, 100.0));
    app.world_mut().entity_mut(raider).insert(ActorDisposition::Hostile);
    app.update();
    let conversation = app.world().resource::<ActiveConversation>();
    assert_eq!(conversation.dialogue_id(), Some("chat__interrupted"), "the narrative goes on from the interruption");
    assert_eq!(conversation.live().map(|live| live.opened_at()), Some(40), "from the tick it broke in");
    assert_ne!(conversation.instance(), Some(&opened), "a new instance, so the text box starts the node");
    assert!(conversation.holds(raider), "the threat joined, and is held while it speaks");
    let interrupted = conversation.instance().cloned();

    let second = body(&mut app, ae::Vec2::new(10.0, 100.0));
    app.world_mut().entity_mut(second).insert(ActorDisposition::Hostile);
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 41;
    app.update();
    let conversation = app.world().resource::<ActiveConversation>();
    assert!(conversation.holds(second), "a second threat joins");
    assert_eq!(conversation.instance().cloned(), interrupted, "and does not start the interruption again");

    // No interruption authored: a threat in earshot changes nothing (it
    // breaks the conversation by striking somebody).
    let (mut app, initiator, npc, third) = three_talking(&["chat"]);
    let raider = body(&mut app, ae::Vec2::new(20.0, 100.0));
    app.world_mut().entity_mut(raider).insert(ActorDisposition::Hostile);
    app.update();
    assert_eq!(members(&app), vec![initiator, npc, third]);
    assert_eq!(app.world().resource::<ActiveConversation>().dialogue_id(), Some("chat"));
}

/// The rollback registration localizes a desync through every body the
/// conversation names, and a restore remaps every handle, the joined among
/// them.
#[test]
fn a_rollback_names_and_remaps_every_member() {
    use bevy::ecs::entity::{EntityHashMap, MapEntities};

    let (app, initiator, npc, third) = three_talking(&[]);
    let conversation = app.world().resource::<ActiveConversation>().clone();
    assert_eq!(conversation.referenced_entities(), vec![initiator, npc, third]);

    let mut world = World::new();
    let moved: Vec<Entity> = (0..3).map(|_| world.spawn_empty().id()).collect();
    let mut map = EntityHashMap::default();
    for (old, new) in [initiator, npc, third].into_iter().zip(moved.iter().copied()) {
        map.insert(old, new);
    }
    let mut restored = conversation.clone();
    restored.map_entities(&mut map);
    assert_eq!(restored.referenced_entities(), moved);
}
