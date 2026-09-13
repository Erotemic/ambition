//! Unit coverage for the session-teardown resource reset.

use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopeActivated, SessionScopeId, SessionScopeRetired,
};

use super::*;
use crate::abilities::traversal::possession::PossessionState;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_characters::control::SlotInteractionState;
use ambition_encounter::switches::SwitchActivationQueue;
use ambition_encounter::{EncounterRegistry, SwitchActivation};
use ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown;
use ambition_platformer2d_world::collision::MovingPlatformSet;

/// ⛔⛔ THIS LIST IS THE SECOND HALF OF `SessionScopedResources`, AND IT IS
/// HAND-KEPT WHERE THE OTHER HALF IS NOT. The `reset` function destructures the
/// `SystemParam` exhaustively, so a field added there and not reset is
/// `error[E0027]` at compile time. Nothing does that for THIS list: every field
/// is a plain `ResMut`, so a missing `init_resource` here is a RUNTIME panic —
/// *"Parameter `SessionScopedResources<'_>::<field>` failed validation: Resource
/// does not exist"* — in three tests at once, which is how `projectile_seq`
/// announced itself on 2026-09-05.
///
/// ⇒ Adding a field to that `SystemParam` is TWO edits, and only one of them is
/// guarded. The type system cannot enumerate a `SystemParam`'s fields, so this
/// stays a list; the compile error tells you about the reset, and this comment
/// is the only thing that tells you about the harness.
fn app_with_populated_mirrors() -> App {
    let mut app = App::new();
    app.add_message::<SessionScopeRetired>();
    app.add_message::<SessionScopeActivated>();
    app.init_resource::<MovingPlatformSet>();
    app.init_resource::<PossessionState>();
    app.init_resource::<ambition_platformer2d_shared_tangle::markers::ControlledSubject>();
    app.init_resource::<EncounterRegistry>();
    app.init_resource::<ambition_encounter::EncounterView>();
    app.init_resource::<BossEncounterRegistry>();
    app.init_resource::<ambition_persistence::quest::QuestRegistry>();
    app.init_resource::<RoomTransitionCooldown>();
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<SwitchActivationQueue>();
    app.init_resource::<crate::session::durable_horizon::SaveRestored>();
    // The occurrence ledger and its three checkpoint copies. Session-scoped for
    // the same reason as the rest: each is a statement about ONE live world.
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>();
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>();
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>();
    app.init_resource::<crate::items::pickup::minted_horizon::MintedItemBaseline>();
    app.init_resource::<ambition_persistence::quest::LastQuestRoom>();
    app.init_resource::<ambition_cutscene::LastCutsceneRoom>();
    app.init_resource::<ambition_projectiles::ProjectileSeqCounter>();
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    // ⛔ THE OWNER. `reset_session_scoped_resources_on_retire` refuses a
    // retirement belonging to a scope that is not the live one — see its body.
    // At its default NO scope is current, which is a quit-to-title and is the
    // case every arm below except the stale one exercises.
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>();
    app.init_resource::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>();
    app.init_resource::<ambition_cutscene::ActiveCutscene>();
    app.init_resource::<ambition_cutscene::CutsceneTriggerQueue>();
    app.init_resource::<ambition_conversation::ActiveConversation>();
    app.init_resource::<ambition_cutscene::CutsceneAdvanceRequest>();
    app.init_resource::<ambition_cutscene::CutsceneSkipHold>();
    app.add_systems(
        Update,
        (
            reset_session_scoped_resources_on_activation,
            reset_session_scoped_resources_on_retire,
        ),
    );

    // Populate the mirrors with distinctive session-A state.
    app.world_mut().resource_mut::<MovingPlatformSet>().0.push(
        ambition_platformer2d_world::platforms::MovingPlatformState::from_authored(
            ambition_platformer2d_core::Vec2::new(10.0, 20.0),
            ambition_platformer2d_core::Vec2::new(32.0, 8.0),
            48.0,
            30.0,
        ),
    );
    let ghost = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<PossessionState>().possessed = Some(ghost);
    app.world_mut()
        .resource_mut::<EncounterRegistry>()
        .ids
        .insert("wave_a".to_owned(), ghost);
    app.world_mut()
        .resource_mut::<RoomTransitionCooldown>()
        .remaining = 5.0;
    app.world_mut()
        .resource_mut::<SlotInteractionState>()
        .primary_mut()
        .interact_buffer_timer = 0.75;
    app.world_mut()
        .resource_mut::<SwitchActivationQueue>()
        .0
        .push(SwitchActivation {
            id: "session_a_switch".to_owned(),
            action: "reset".to_owned(),
            target_encounter: "session_a_encounter".to_owned(),
        });
    // Session A applied its save.
    app.world_mut()
        .resource_mut::<crate::session::durable_horizon::SaveRestored>()
        .0 = true;
    // Session A last announced this room to quests and cutscenes.
    app.world_mut()
        .resource_mut::<ambition_persistence::quest::LastQuestRoom>()
        .0 = Some("intro_wake_room".to_owned());
    app.world_mut()
        .resource_mut::<ambition_cutscene::LastCutsceneRoom>()
        .0 = Some("intro_wake_room".to_owned());
    // ...and its world had somewhere to put things.
    {
        let world = app.world_mut();
        world
            .resource_mut::<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>()
            .adopt_rows(
                [(
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("session_a_item"),
                    ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Placed {
                        room: "session_a_room".to_owned(),
                        at: ambition_platformer2d_core::Vec2::new(10.0, 20.0),
                    },
                )]
                .into_iter()
                .collect(),
            );
        let ledger = world
            .resource::<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>()
            .clone();
        world
            .resource_mut::<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>()
            .adopt(ledger);
        world
            .resource_mut::<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>()
            .adopt(
                [(
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("session_a_item"),
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("session_a_hand"),
                )]
                .into_iter()
                .collect(),
            );
        world
            .resource_mut::<crate::items::pickup::minted_horizon::MintedItemBaseline>()
            .adopt(
                [(
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement("session_a_mint"),
                    crate::items::pickup::minted_horizon::MintedItemDescription {
                        origin:
                            ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic {
                                parent: ambition_platformer2d_shared_tangle::sim_id::SimId::placement(
                                    "session_a_spawner",
                                ),
                                sequence: 0,
                            },
                        held_item: "axe".to_owned(),
                    },
                )]
                .into_iter()
                .collect(),
            );
    }
    app
}

/// Every ledger that describes ONE live world is empty.
fn the_four_ledgers_are_empty(app: &App) -> bool {
    app.world()
        .resource::<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>()
        .is_empty()
        && app
            .world()
            .resource::<ambition_platformer2d_shared_tangle::lifecycle::OccurrenceBaseline>()
            .remembered()
            .is_empty()
        && app
            .world()
            .resource::<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>()
            .is_empty()
        && app
            .world()
            .resource::<crate::items::pickup::minted_horizon::MintedItemBaseline>()
            .is_empty()
}

/// The save-applied latch dies with the world it describes.
///
/// it did not, and that is second carried risk. `SaveRestored` means *"the loaded save has
/// been applied to THIS WORLD"*, is set true in exactly one place, and was set false NOWHERE —
/// so the second session in a process returned early from both restores and inherited session
/// A's `AuthoredOccurrences`, `OccurrenceBaseline`, `CustodyBaseline` and `MintedItemBaseline`.
/// A consumed occurrence stayed consumed into a new game.
///
/// resetting the latch is the whole fix: `AuthoredOccurrences::adopt_rows`
/// REPLACES rather than merges, so session B's restore rewrites all four ledgers
/// from the save, empty or not. One value, not four.
#[test]
fn retirement_clears_the_save_applied_latch() {
    let mut app = app_with_populated_mirrors();

    // No retirement yet: session A's latch stands, so nothing re-applies the
    // save underneath a live world.
    app.update();
    assert!(
        app.world()
            .resource::<crate::session::durable_horizon::SaveRestored>()
            .0,
        "the latch must survive an ordinary frame; clearing it mid-session would \
         re-apply the save over live state"
    );

    app.world_mut()
        .write_message(SessionScopeRetired(SessionScopeId(0)));
    app.update();

    assert!(
        !app.world()
            .resource::<crate::session::durable_horizon::SaveRestored>()
            .0,
        "the world the save was applied to has retired, so the next session must \
         re-run its restore rather than inherit this one's ledgers"
    );
}

/// ⚠ "EVERY" IS THE COMPILER'S CLAIM, NOT THIS TEST'S. The assertions below are
/// a hand-picked SUBSET — the mirrors whose contents are easy to seed and read.
/// What makes the name true is `reset`'s exhaustive destructure of
/// `SessionScopedResources`: adding a resource to that `SystemParam` without
/// resetting it is `error[E0027]: pattern does not mention field`, verified by
/// adding one. ⇒ Do not grow this list to chase completeness; it would still be
/// a hand-kept list, and the structural guard already covers what it was
/// reaching for. Seed a mirror here when its VALUE is the interesting part.
#[test]
fn retirement_clears_every_session_scoped_mirror() {
    let mut app = app_with_populated_mirrors();

    // No retirement yet: mirrors keep their session-A state.
    app.update();
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_some());
    assert!(!app.world().resource::<EncounterRegistry>().ids.is_empty());
    assert!(app
        .world()
        .resource::<SlotInteractionState>()
        .primary()
        .buffered());
    assert_eq!(app.world().resource::<SwitchActivationQueue>().0.len(), 1);
    assert!(
        !the_four_ledgers_are_empty(&app),
        "the fixture seeded no world-describing ledger, so clearing them below \
         proves nothing"
    );

    // Retire the scope; the mirrors reset the same frame.
    app.world_mut()
        .write_message(SessionScopeRetired(SessionScopeId(0)));
    app.update();

    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "moving-platform mirror still holds session-A platforms after teardown"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "possession still points at a despawned session-A body after teardown"
    );
    assert!(
        app.world().resource::<EncounterRegistry>().ids.is_empty(),
        "encounter index still maps ids to dead session-A entities after teardown"
    );
    assert_eq!(
        app.world().resource::<RoomTransitionCooldown>().remaining,
        RoomTransitionCooldown::default().remaining,
        "transient room state carried across teardown"
    );
    assert!(
        !app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered(),
        "slot-level interaction buffer carried across teardown"
    );
    assert!(
        app.world().resource::<SwitchActivationQueue>().0.is_empty(),
        "pending switch activation carried across teardown"
    );
    assert!(
        the_four_ledgers_are_empty(&app),
        "a ledger describing the retired session's world survived teardown: it \
         says where objects are and who was holding them in a world that no \
         longer exists"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_persistence::quest::LastQuestRoom>()
            .0,
        None,
        "the quest room-entry memory survived teardown: a new game starting in \
         the room the last session ended in would skip its first RoomEntered"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_cutscene::LastCutsceneRoom>()
            .0,
        None,
        "the cutscene room-entry memory survived teardown: the first room's \
         trigger would not fire"
    );
}

#[test]
fn no_retirement_leaves_mirrors_untouched() {
    let mut app = app_with_populated_mirrors();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_some());
}

/// ⭐⭐ THE NEXT SESSION IS CLEAN EVEN IF RETIREMENT NEVER HAPPENED.
///
/// ⛔ NOTHING RETIRES SESSION A HERE, DELIBERATELY. The whole claim is that
/// activation is the correctness edge and retirement is hygiene, and a fixture
/// that retires A first cannot tell those apart — it would pass with the
/// activation reset deleted.
#[test]
fn activating_a_session_clears_what_a_skipped_teardown_left_behind() {
    let mut app = app_with_populated_mirrors();
    app.update();

    // Session A's mirrors are still standing: its retirement was delayed,
    // misordered, or lost to an abnormal exit.
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_some());
    assert!(app
        .world()
        .resource::<SlotInteractionState>()
        .primary()
        .buffered());

    // Session B begins.
    app.world_mut()
        .write_message(SessionScopeActivated(SessionScopeId(1)));
    app.update();

    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "session B inherited A's moving platforms"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "session B inherited a handle to a body that belonged to A — and Bevy \
         reuses entity ids, so that handle can come to name one of B's own"
    );
    assert!(
        !app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered(),
        "session B started with a buffered interact nobody pressed in it"
    );
    assert!(
        app.world().resource::<EncounterRegistry>().ids.is_empty(),
        "session B inherited A's encounter index, and its `specs_loaded` latch \
         would then suppress B's own repopulation"
    );
    assert_eq!(
        app.world().resource::<RoomTransitionCooldown>().remaining,
        0.0,
        "session B started inside A's room-transition cooldown, which refuses \
         every door for as long as it lasts"
    );
    assert!(
        app.world().resource::<SwitchActivationQueue>().0.is_empty(),
        "a switch activation produced in A was about to be delivered into B"
    );
    assert!(
        the_four_ledgers_are_empty(&app),
        "session B is about to build its first room against a ledger describing \
         A's world. A row saying an object is lying in one of A's rooms \
         SUPPRESSES that object where B authors it — the inherited ledger \
         deletes things from B's world. ⭐ This is the edge that matters: \
         `adopt_the_occurrence_ledger_at_activation` runs AFTER this reset, so a \
         LOAD re-seeds the cleared ledger from its own file on this same edge"
    );
    assert!(
        !app.world()
            .resource::<crate::session::durable_horizon::SaveRestored>()
            .0,
        "the latch still said A's save had been applied, so B would never \
         restore into its own world"
    );
}

/// ⛔⛤ **A DELAYED RETIREMENT OF A DEAD SCOPE MUST NOT WIPE THE LIVE ONE'S
/// AUTHORITIES — MEASURED MISSING BY A 2026-09-13 REVIEW.**
///
/// The shared lifecycle states this rule beside `ActiveSessionScope::clear_if_current`:
/// *"Retiring A after B activated must not clear B's spawn context."* Entity
/// cleanup obeys it, and so does the rollback session's. **This system did not
/// ask which scope retired at all** — any `SessionScopeRetired` reset all twenty
/// mirrors and removed `SessionMechanics`.
///
/// ⇒ So a retirement for A arriving after B became current wiped **B's** live
/// state — mechanics, occurrence ledgers, baselines, encounters, possession,
/// projectile counter, lifecycle intent — while `ActiveSessionScope` went on
/// correctly naming B. A session still being played, reset out from under itself.
///
/// ⭐ **THE SECOND HALF IS THE CONTROL AND IT IS NOT OPTIONAL**: a system that
/// refused EVERY retirement would pass the first half and delete the hygiene this
/// whole module exists for. Retiring the LIVE scope must still clear.
#[test]
fn a_stale_scopes_retirement_leaves_the_live_scopes_mirrors_alone() {
    let mut app = app_with_populated_mirrors();

    // Two scopes: A retired long ago, B is what the player is in.
    let (stale, live) = {
        let mut active = app
            .world_mut()
            .resource_mut::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>();
        let stale = active.begin();
        let live = active.begin();
        (stale, live)
    };
    assert_ne!(stale, live, "the fixture minted one scope twice");

    app.update();
    // ⚠ THE PREMISE: seeded state, or "unchanged" below is the empty set.
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(!the_four_ledgers_are_empty(&app));

    app.world_mut().write_message(SessionScopeRetired(stale));
    app.update();

    assert_eq!(
        app.world().resource::<MovingPlatformSet>().0.len(),
        1,
        "a retirement belonging to a scope that is NOT live cleared the live \
         scope's moving platforms"
    );
    assert!(
        app.world()
            .resource::<PossessionState>()
            .possessed
            .is_some(),
        "a stale scope's retirement dropped the LIVE session's possessed body"
    );
    assert!(
        !the_four_ledgers_are_empty(&app),
        "a stale scope's retirement wiped the ledgers describing the world the \
         player is standing in"
    );

    // ⭐ THE CONTROL: the live scope's own retirement still clears.
    app.world_mut().write_message(SessionScopeRetired(live));
    app.update();
    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "retiring the LIVE scope left its mirrors standing, so the assertion \
         above is about refusing every retirement rather than about ownership"
    );
    assert!(
        the_four_ledgers_are_empty(&app),
        "retiring the LIVE scope left its ledgers standing"
    );
}

/// ⛔⛤ **AMBIENT GRAVITY IS A MECHANIC THAT USED TO OUTLIVE THE SESSION THAT
/// FLIPPED IT — MEASURED BY A 2026-09-13 REVIEW.**
///
/// `BaseGravity` is canonical rollback state and gravity-flip switches write it.
/// Its only reset roads were `RoomReplayAdmitted` and a room TRANSITION's commit;
/// neither is a session edge. ⇒ *"flip gravity upward, quit, start a new
/// session"* began the next run upside down, until some later transition
/// happened to correct it — mechanical state crossing a lifecycle boundary.
///
/// ⭐ IT IS FIXED BY DECLARATION, NOT BY A NEW SYSTEM: `BaseGravity` is a member
/// of `SessionScopedResources`, so the exhaustive destructure in `reset` covers
/// it and the next resource added to that aggregate cannot skip it silently
/// either.
#[test]
fn ambient_gravity_does_not_outlive_the_session_that_flipped_it() {
    use ambition_platformer2d_shared_tangle::gravity::BaseGravity;

    let mut app = app_with_populated_mirrors();
    let flipped = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
    app.world_mut().insert_resource(BaseGravity { dir: flipped });

    app.update();
    // ⚠ THE PREMISE: an unflipped fixture makes "reset to default" vacuous.
    assert_eq!(
        app.world().resource::<BaseGravity>().dir,
        flipped,
        "the fixture never flipped gravity, so the assertions below compare the \
         default against itself"
    );

    app.world_mut()
        .write_message(SessionScopeRetired(SessionScopeId(0)));
    app.update();
    assert_eq!(
        app.world().resource::<BaseGravity>().dir,
        BaseGravity::default().dir,
        "the retired session's flipped ambient gravity survived teardown, so the \
         next session starts upside down"
    );

    // ⭐ AND THE ACTIVATION EDGE, which is the one that is CORRECTNESS rather
    // than hygiene: whatever an abnormal exit left standing is overwritten by
    // the session about to fall through it.
    app.world_mut().insert_resource(BaseGravity { dir: flipped });
    app.world_mut()
        .write_message(SessionScopeActivated(SessionScopeId(1)));
    app.update();
    assert_eq!(
        app.world().resource::<BaseGravity>().dir,
        BaseGravity::default().dir,
        "an activating session inherited the previous run's flipped gravity"
    );
}

/// ⛔⛤ **CUTSCENE STATE WHOSE DOCUMENTED LIFETIME IS ONE SESSION AND WHOSE
/// IMPLEMENTED LIFETIME WAS THE PROCESS — MEASURED BY TWO 2026-09-13 REVIEWS.**
///
/// `CutsceneTriggerQueue`: a trigger raised just before retirement and consumed
/// just after the next session begins plays A's cutscene in B. `ActiveCutscene`
/// is worse — while `is_playing()` holds, input declaration captures the cutscene
/// context, and `end_cutscene` writes the running script's `seen_flag` into the
/// CURRENT save, so A's stale playback finishing after B installed its file
/// writes A's narrative flag into B's.
///
/// ⛔ **AND `CutsceneAdvanceRequest`, WHICH THE FIRST FIX LEFT OUT** — the
/// completed dismiss/skip EDGES already across the simulation boundary, consumed
/// by `tick_active_cutscene` with `mem::take`. Clearing the playback alone does
/// not end A's authority: B's opening room auto-triggers its own cutscene and the
/// next tick spends A's skip on it.
///
/// ⚠ `ActiveCutscene` is not asserted here and that is this file's standing rule,
/// not an omission: "every" is the COMPILER's claim, made by `reset`'s exhaustive
/// destructure of `SessionScopedResources`. Seed a mirror here when its VALUE is
/// the interesting part; these two have values a reader can check in one line.
#[test]
fn a_session_does_not_inherit_the_previous_ones_queued_cutscene_or_pending_skip() {
    let mut app = app_with_populated_mirrors();
    app.world_mut()
        .resource_mut::<ambition_cutscene::CutsceneTriggerQueue>()
        .request("intro");
    app.world_mut()
        .insert_resource(ambition_cutscene::CutsceneAdvanceRequest {
            dismiss_dialogue: false,
            skip_cutscene: true,
        });

    app.update();
    assert_eq!(
        app.world()
            .resource::<ambition_cutscene::CutsceneTriggerQueue>()
            .0
            .len(),
        1,
        "the fixture queued no cutscene trigger"
    );

    app.world_mut()
        .write_message(SessionScopeRetired(SessionScopeId(0)));
    app.update();

    assert!(
        app.world()
            .resource::<ambition_cutscene::CutsceneTriggerQueue>()
            .0
            .is_empty(),
        "a cutscene trigger raised by the retired session survived it, so the \
         next session plays the previous run's scene"
    );
    // ⛔⛤ **THE PENDING INPUT, WHICH THE FIRST VERSION OF THIS FIX LEFT OUT.**
    // `skip_cutscene` is a COMPLETED edge already across the simulation boundary,
    // consumed by `tick_active_cutscene` with `mem::take`. With the playback
    // cleared and this left standing, B's opening room auto-triggers ITS cutscene
    // and the very next tick spends A's skip on it.
    assert!(
        !app.world()
            .resource::<ambition_cutscene::CutsceneAdvanceRequest>()
            .skip_cutscene,
        "a skip raised by the retired session survived it, so the next session's \
         opening cutscene is skipped by an input nobody gave it"
    );
}
