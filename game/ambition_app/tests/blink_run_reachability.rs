//! Room-level verification that `blink_run` actually requires + delivers the Blink
//! ability. Blink is a precise 150px teleport (15+ ... well, 5 unit tests in
//! blink.rs); this proves the LDtk content (GroundItem held_item="blink" + low
//! ceiling + hazard gaps) wires up to it: grab the ability, then blink across two
//! 112px death gaps in a corridor too low to jump. Driven through the public
//! action/observation API; asserts only on player position.
//!
//! Sequence: walk onto the ability (Attack to grab) → for each gap, walk to its
//! near edge and Attack with aim right (a blink carries the player 150px across).

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};

fn walk_to_then_blink(sim: &mut Platformer2dSimHarness, edge: f32) {
    for _ in 0..80 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= edge
        {
            break;
        }
    }
    sim.step(AgentAction {
        attack: true,
        aim_x: 1.0,
        ..base()
    }); // blink right
    for _ in 0..30 {
        sim.step(base());
    }
}

#[test]
fn blink_run_blinks_across_the_death_gaps() {
    let mut sim = fixed_60hz_room_sim("blink_run");
    assert!(
        sim.observation().player_pos.0 < 110.0,
        "spawns at the start of the corridor, got x={}",
        sim.observation().player_pos.0
    );

    // Grab the Blink ability off the start floor (x[120,160]).
    let mut grabbed = false;
    for _ in 0..60 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= 115.0
        {
            grabbed = true;
            break;
        }
    }
    assert!(grabbed, "should reach the blink pickup");
    sim.step(AgentAction {
        attack: true,
        ..base()
    }); // Attack while overlapping = grab
    sim.step(base());

    // Gap 1: hazard x[288,400]. A body 30 wide stops short of it at x 273,
    // and a blink carries 150, so it blinks from x 266 onto the mid floor
    // x[400,608].
    walk_to_then_blink(&mut sim, 266.0);
    let after1 = sim.observation();
    assert!(
        after1.player_pos.0 > 360.0 && after1.resets == 0,
        "should have blinked across gap 1 onto the mid floor (x={}, resets={}); a \
         jump can't clear it under the low ceiling and walking falls onto the hazard",
        after1.player_pos.0,
        after1.resets,
    );

    // Gap 2: hazard x[608,720]; from x 586 onto the end floor x[720,1008].
    walk_to_then_blink(&mut sim, 586.0);
    let after2 = sim.observation();
    assert!(
        after2.player_pos.0 > 720.0 && after2.resets == 0,
        "should have blinked across gap 2 to the end floor (x={}, resets={})",
        after2.player_pos.0,
        after2.resets,
    );
}
