#![cfg(feature = "rl_sim")]
//! MULTI-PARTY conversations, in a room that has nothing to do with the raid
//! they were built for: the coil lab, where Coulomba speaks in Nikobolt's
//! conversation.
//!
//! The player talks to Nikobolt. `conversation.join npc_science_coulomba`,
//! asked as a `.yarn` line asks it, brings Coulomba in: she is held and
//! stands while they talk. A strike that knocks her back takes her out of
//! it, and the player and Nikobolt talk on. `conversation.leave` takes her
//! out again after she joins a second time.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::{BodyCombat, CharacterForm};
use ambition_platformer2d::conversation::{ActiveConversation, HeldByConversation};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::authored_logic::{AuthoredArg, AuthoredAsk, CommandId, RunAuthoredCommand};
use bevy::prelude::Entity;

const LAB: &str = "eureka_coil_lab";
const NIKOBOLT: &str = "npc_science_nikobolt";
const COULOMBA: &str = "npc_science_coulomba";

fn wearing(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == character)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("the coil lab stages `{character}`"))
}

fn x_of(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world().get::<ae::BodyKinematics>(body).expect("a body").pos.x
}

fn conversation(sim: &ambition_app::Platformer2dSimHarness) -> ActiveConversation {
    sim.world().resource::<ActiveConversation>().clone()
}

/// Put `body` at `at`, standing.
fn stand_beside(sim: &mut ambition_app::Platformer2dSimHarness, body: Entity, at: ae::Vec2) {
    {
        let world = sim.world_mut();
        let mut bodies = world.query::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>();
        let (mut clusters, mut model) = bodies.get_mut(world, body).expect("a body");
        let mut clusters = clusters.as_clusters_mut();
        ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
    }
    sim.step(base());
}

/// Ask `conversation.<verb> <character>` as the Yarn line's ledger release
/// does, and step until it has been performed.
fn ask(sim: &mut ambition_app::Platformer2dSimHarness, verb: &str, character: &str) {
    sim.world_mut().write_message(RunAuthoredCommand::new(
        CommandId::new("conversation", verb),
        vec![AuthoredArg::Name(character.to_string())],
        AuthoredAsk::new("dialogue", "a third character joins a conversation"),
    ));
    sim.step_n(base(), 2);
}

#[test]
fn a_third_character_joins_a_conversation_and_leaves_it_by_the_rules() {
    let mut sim = fixed_60hz_room_sim(LAB);
    sim.step(base());
    let nikobolt = wearing(&mut sim, NIKOBOLT);
    let coulomba = wearing(&mut sim, COULOMBA);

    // The player steps up to Nikobolt and talks.
    let at = sim.world().get::<ae::BodyKinematics>(nikobolt).expect("a body").pos;
    sim.teleport_player((at.x - 40.0, at.y));
    sim.step(base());
    sim.step(ambition_app::AgentAction { interact: true, interact_held: true, ..base() });
    for _ in 0..10 {
        if conversation(&sim).talker() == Some(nikobolt) {
            break;
        }
        sim.step(base());
    }
    assert_eq!(conversation(&sim).talker(), Some(nikobolt), "premise: Interact beside Nikobolt opens a conversation with him");

    // Coulomba stands within earshot of them.
    stand_beside(&mut sim, coulomba, ae::Vec2::new(at.x + 64.0, at.y));
    assert!(!conversation(&sim).holds(coulomba), "premise: standing near is not being in it");

    ask(&mut sim, "join", COULOMBA);
    let joined = conversation(&sim);
    assert!(joined.holds(coulomba), "`conversation.join` brought Coulomba into the conversation");
    assert_eq!(joined.talker(), Some(nikobolt), "the conversation is still with Nikobolt");
    let stood_at = x_of(&sim, coulomba);
    sim.step_n(base(), 60);
    assert!(sim.world().get::<HeldByConversation>(coulomba).is_some(), "Coulomba is held while they talk");
    let drift = (x_of(&sim, coulomba) - stood_at).abs();
    assert!(drift < 8.0, "Coulomba walked {drift} px on while she was in the conversation");

    // A strike knocks her back: she leaves, and the player and Nikobolt talk on.
    sim.world_mut().get_mut::<BodyCombat>(coulomba).expect("she can be struck").hitstun_timer = 0.3;
    sim.step_n(base(), 2);
    let after = conversation(&sim);
    assert!(!after.holds(coulomba), "Coulomba, knocked back, is still in the conversation");
    assert_eq!(after.talker(), Some(nikobolt), "the player and Nikobolt should talk on after Coulomba left");
    sim.step_n(base(), 30);
    assert!(sim.world().get::<HeldByConversation>(coulomba).is_none(), "the hold on Coulomba was released");

    // Let go, she went back to her own plans. She comes back, joins again,
    // and `conversation.leave` takes her out. The knock is over by then; the
    // dialogue mode stops the gameplay systems that would run its timer
    // down, so the fixture ends it.
    sim.world_mut().get_mut::<BodyCombat>(coulomba).expect("she can be struck").hitstun_timer = 0.0;
    stand_beside(&mut sim, coulomba, ae::Vec2::new(at.x + 64.0, at.y));
    ask(&mut sim, "join", COULOMBA);
    assert!(conversation(&sim).holds(coulomba), "Coulomba joins a second time");
    ask(&mut sim, "leave", COULOMBA);
    let left = conversation(&sim);
    assert!(!left.holds(coulomba), "`conversation.leave` took Coulomba out");
    assert_eq!(left.talker(), Some(nikobolt), "and the conversation with Nikobolt goes on");
}
