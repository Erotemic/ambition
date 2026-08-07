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
    app.world_mut().resource_mut::<ActiveConversation>().open(
        Some(initiator),
        Some(npc),
        "chat",
        ConversationInputOwner::Primary,
    );
    (app, initiator, npc)
}

fn talking(app: &App) -> bool {
    app.world().resource::<ActiveConversation>().is_live()
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
