//! Conversation continuity, tested beside the rules rather than in the
//! interaction module's test file — where these lived while the systems did.

use bevy::prelude::*;

use ambition_characters::actor::BodyCombat;
use ambition_characters::brain::ScriptedControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::CenteredAabb;

use super::{
    break_dialogue_on_hit_or_separation, project_conversation_hold, ActiveConversation,
    ConversationInputOwner, HeldByConversation,
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
    // The break can BARK, so its output channel exists here as it does in the
    // production schedule. Registering it in the fixture rather than wrapping the
    // writer in `Option` — that waiver would answer "may this be absent" when the
    // question is who owns registering it.
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
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
            ConversationInputOwner::Primary,
        ));
    (app, initiator, npc)
}

/// A conversation through `node`, opened on `tick`.
///
/// ⚠ the tick is the fixture's whole subject in the tests below: it is what
/// tells one visit to an NPC from the next, and what a stamped narrative end is
/// matched against.
fn live_at(node: &str, tick: u64) -> super::LiveConversation {
    super::LiveConversation {
        opened_at: tick,
        ..super::LiveConversation::for_test(None, None, node, ConversationInputOwner::Primary)
    }
}

fn talking(app: &App) -> bool {
    app.world().resource::<ActiveConversation>().is_live()
}

/// **A resimulated tick does not close a conversation on the strength of a text
/// box from another timeline.**
///
/// ⛔ **the sim used to POLL `DialogState::active()`** to learn the Yarn runner
/// had run out of lines. `DialogState` is not rewound — deliberately, because
/// rewinding a typewriter would stutter the box — so on a resimulated tick that
/// read returned the LIVE runner rather than the runner as it was. A rewind to
/// before the conversation ended would close it again immediately, and the
/// resimulation would not reproduce the history it exists to reproduce.
///
/// ⭐ **the end is a STAMPED RECORD now**: presentation observes the runner
/// finishing once and writes down which conversation instance ended and the tick
/// it applies from. A tick before that tick changes nothing, which is exactly
/// what a resimulated tick must do.
///
/// ⚠ **what this does NOT claim**: that the Yarn runner is deterministic. It is
/// content running outside the simulation, so WHICH tick it finishes on is still
/// presentation's answer. What is now true is that every replay of that tick
/// agrees with the original run.
#[test]
fn a_conversation_survives_a_tick_before_the_narrative_end_applies() {
    use super::ObservedNarrativeEnd;

    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ObservedNarrativeEnd>();
    app.insert_resource(ambition_time::SimTick(7));
    app.add_systems(Update, super::close_conversation_on_narrative_end);
    let live = live_at("chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());

    // The narrative finished while the simulation was at tick 9, so the end
    // applies from 10. This tick is 7 — a resimulated tick BEFORE it.
    app.world_mut()
        .resource_mut::<ObservedNarrativeEnd>()
        .record(&live, 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "a tick before the end applies must leave the conversation alone — \
         polling the view here is what let a rewind close a conversation that, \
         in the timeline being replayed, was still going"
    );

    // ...and the tick it applies from ends it, whether that is the original run
    // or the fourth replay of it.
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 10;
    app.update();
    assert!(
        !app.world().resource::<ActiveConversation>().is_live(),
        "the runner finishing still ends the conversation — this is a change of \
         MECHANISM, not of behaviour"
    );
}

/// **THE REWIND, both halves.** (GPT 5.6, 2026-08-07, finding 2)
///
/// ⛔ the message this replaced was cleared on rollback, and the system that
/// wrote it — presentation, watching the live runner — does not execute between
/// resimulated ticks. So a rewind across the end tick DROPPED it: every replayed
/// tick after it ran with a conversation the original timeline had already
/// finished, holding a body and capturing a seat, and presentation re-observed
/// the end afterwards at a different simulation time.
///
/// ⭐ the record is not rollback state, so the replay is told the same thing the
/// original run was told, and reaches the same answer on the same tick.
#[test]
fn a_rewind_past_the_end_replays_it_at_the_same_tick() {
    use super::ObservedNarrativeEnd;

    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ObservedNarrativeEnd>();
    app.insert_resource(ambition_time::SimTick(12));
    app.add_systems(Update, super::close_conversation_on_narrative_end);
    let live = live_at("chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());
    app.world_mut()
        .resource_mut::<ObservedNarrativeEnd>()
        .record(&live, 10);
    app.update();
    assert!(!app.world().resource::<ActiveConversation>().is_live());

    // THE REWIND: the authority is rollback state, so tick 8 restores the
    // conversation. The record is NOT, so it is still there to be replayed.
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live.clone());
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 8;
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "tick 8 is before the end, so the replayed timeline still has a live \
         conversation — closing it here would put the hold and the input capture \
         two ticks early"
    );

    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 10;
    app.update();
    assert!(
        !app.world().resource::<ActiveConversation>().is_live(),
        "and the replay ends it on the SAME tick the original run did — a \
         narrative end that lands at a different simulation time is a different \
         history"
    );
}

/// **An end from the previous conversation does not close the next one.**
///
/// ⛔ the poison. A bare marker would close whatever is live when it happens to
/// be read, so a player who finished one conversation and immediately started
/// another could have the second one closed by the first one's ending.
///
/// ⚠ **and the node id alone is not enough**, which is why the record names the
/// tick the conversation OPENED on: talk to the same NPC twice and both
/// conversations are `"chat"`.
#[test]
fn an_end_from_the_previous_conversation_does_not_close_the_next_one() {
    use super::ObservedNarrativeEnd;

    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ObservedNarrativeEnd>();
    app.insert_resource(ambition_time::SimTick(30));
    app.add_systems(Update, super::close_conversation_on_narrative_end);

    // A different node entirely.
    let first = live_at("first_chat", 5);
    app.world_mut()
        .resource_mut::<ActiveConversation>()
        .open(live_at("second_chat", 20));
    app.world_mut()
        .resource_mut::<ObservedNarrativeEnd>()
        .record(&first, 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "the FIRST conversation's ending closed the SECOND one — an end has to \
         name what it is ending or it is just a global 'stop whatever is running'"
    );

    // And the SAME node, talked to twice — which a node id alone cannot tell
    // apart.
    app.world_mut()
        .resource_mut::<ObservedNarrativeEnd>()
        .record(&live_at("second_chat", 5), 10);
    app.update();
    assert!(
        app.world().resource::<ActiveConversation>().is_live(),
        "an end for the PREVIOUS visit to this NPC closed the current one: two \
         conversations through one node are two conversations"
    );
}

/// **A REWIND DOES NOT RESTART THE TEXT BOX.** (GPT 5.6, 2026-08-07, finding 2)
///
/// ⛔ opening the runner used to be a `DialogState::start` call inside the
/// INTERACTION system, which runs in the sim schedule. `DialogState` is left out
/// of rollback so a rewind does not stutter the typewriter — and a rewind across
/// the tick somebody pressed Interact replays that system, so the snapshot did
/// not stutter the box and the replay did: line, options and reveal reset, and a
/// second `runner.start_node` enqueued.
///
/// ⭐ the projection recognises the conversation it already opened, because a
/// restored authority carries the same `opened_at`. A conversation opened on a
/// DIFFERENT tick is a different conversation and does open the box, which is
/// the other half of the same rule.
#[test]
fn replaying_the_opening_tick_does_not_reopen_the_box() {
    let mut app = App::new();
    app.init_resource::<ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogState>();
    app.add_systems(Update, super::open_dialog_ui_when_the_conversation_starts);

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

/// **A conversation the world keeps running through can be broken.**
///
/// The three cases Jon named (design:
/// `docs/planning/engine/dialogue-continuity.md`): standing and talking holds;
/// being knocked about ends it; falling away from the other body ends it.
///
/// ⭐ the struck case is driven from the NPC's body, not the player's. The rule
/// is symmetric — *"both characters"* — and a test that only ever hits the
/// player would pass against a player-centric implementation.
#[test]
fn a_conversation_breaks_on_knockback_or_on_the_bodies_separating() {
    // Standing and talking: nothing breaks.
    let (mut app, _, _) = talking_app();
    app.update();
    assert!(talking(&app), "two bodies standing together keep talking");

    // ⭐ the NPC is knocked about — not the player.
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

    // ⚠ and damage that does NOT move you leaves it alone: a poison tick is not
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

/// **A conversation holds the body it is talking to, and lets go afterwards.**
///
/// ⛔ the release is the half that bites: a stranded `ScriptedControl` is a
/// permanently frozen NPC, and a conversation can end in more ways than the
/// break rule (the Yarn runner finishing, a room swap, a teardown). So the
/// projection asks the authority rather than remembering what it inserted.
#[test]
fn a_conversation_blanks_the_npcs_brain_and_releases_it_when_it_ends() {
    let (mut app, initiator, npc) = talking_app();

    app.update();
    assert!(
        app.world().get::<ScriptedControl>(npc).is_some(),
        "the NPC stops answering its brain while it is talking — otherwise it \
         wanders off mid-sentence now that the world keeps running"
    );
    assert!(
        app.world().get::<ScriptedControl>(initiator).is_none(),
        "the TALKER is not marked: `DIALOGUE_CONTEXT` already neutralised their \
         input, and a second mechanism on the same body would race the death beat"
    );

    // The conversation ends by any route — here, the runner finishing.
    app.world_mut().resource_mut::<ActiveConversation>().close();
    app.update();
    assert!(
        app.world().get::<ScriptedControl>(npc).is_none(),
        "and it gets its brain back; a stranded marker is a frozen NPC forever"
    );
    assert!(
        app.world().get::<HeldByConversation>(npc).is_none(),
        "the claim is released with the marker it claimed"
    );
}

/// **A rewind must not be able to strand the hold half-applied.**
///
/// ⛔ **the hold is written by TWO components with different rollback
/// authority**, and that is the defect this pins (GPT 5.6 review through
/// `c32e690`, finding 1). `ScriptedControl` is rollback-registered;
/// [`HeldByConversation`] is not, and neither is `DialogState` — which used to
/// be what these systems read. Both run in `sim_schedule()`, which under a
/// rollback host IS the GGRS schedule, so both resimulate.
///
/// So a rewind past the insert does exactly what this test does by hand: GGRS
/// restores the registered component and leaves the unregistered marker behind,
/// carried in from a future that no longer happened. What the resimulated tick
/// then sees is a body that is already "held" — and an insert gated on that
/// marker declines to restore the control override it stands for.
///
/// ⭐ **the assertion is about the PAIR, not about either component.** A hold
/// that is a marker plus an override has to move as one thing or it is not one
/// thing.
#[test]
fn a_rewind_cannot_leave_a_conversation_holding_a_body_it_no_longer_controls() {
    let (mut app, _, npc) = talking_app();

    app.update();
    assert!(
        app.world().get::<ScriptedControl>(npc).is_some(),
        "precondition: the conversation took the hold in the first place"
    );

    // **THE REWIND.** Not a simulated one — this is precisely what a GGRS
    // `LoadWorld` does with a snapshot taken before the insert: every
    // rollback-registered component is restored to its snapshot state (so the
    // override goes away), and everything else is left exactly as the abandoned
    // future left it (so the marker stays).
    app.world_mut().entity_mut(npc).remove::<ScriptedControl>();
    assert!(
        app.world().get::<HeldByConversation>(npc).is_some(),
        "precondition: the unregistered marker is what survives the rewind"
    );

    // The resimulated tick.
    app.update();

    assert!(
        app.world().get::<ScriptedControl>(npc).is_some(),
        "after a rewind the NPC is still marked as held by the conversation, so \
         the hold has to still BE a hold — otherwise it is holding station on \
         the strength of a marker while its brain drives it away"
    );
}

/// **The conversation's reconcile never strips another claimant's control.**
///
/// ⛔ `ScriptedControl` has six owners now — the death beat, the flagpole, act
/// clear, versus, seating, and this. The projection sweeps bodies it does not
/// hold, so the question "could that sweep take somebody else's override" has to
/// have an answer that is checked rather than reasoned about.
///
/// It does not, because the sweep is scoped by [`HeldByConversation`] and only
/// this module ever writes that. ⭐ **the second half is the poison**: a body
/// wearing a STALE conversation marker — the exact thing a rewind leaves behind
/// — alongside another claimant's override is the case where a marker-blind
/// sweep would do damage, and it is the case a test written only from the happy
/// path would never construct.
#[test]
fn a_conversation_hold_never_strips_another_claimants_control() {
    let (mut app, _, _) = talking_app();

    // Somebody else's held body: a death beat's, with no conversation marker.
    let dying = body(&mut app, ae::Vec2::new(900.0, 900.0));
    app.world_mut().entity_mut(dying).insert(ScriptedControl);

    app.update();
    assert!(
        app.world().get::<ScriptedControl>(dying).is_some(),
        "the conversation swept a body it never claimed — every other owner of \
         `ScriptedControl` marks the body a PLAYER is driving, and taking one \
         back mid-death-beat unfreezes a corpse"
    );

    // ⛔ THE POISON: a stale conversation marker on that same body, which is
    // what a rewind past a hold leaves behind. The sweep MUST clear the marker
    // it owns — and the override goes with it, because the pair is the hold.
    // What it must not do is leave the marker sitting there for the next tick to
    // reason about.
    app.world_mut().entity_mut(dying).insert(HeldByConversation);
    app.update();
    assert!(
        app.world().get::<HeldByConversation>(dying).is_none(),
        "a stale claim is cleared rather than left to accumulate — the marker is \
         a projection, so anything the authority does not name loses it"
    );
}
