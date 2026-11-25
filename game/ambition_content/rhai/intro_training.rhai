// The workshop lesson advances on a landing and a real hit. It has no time limit.
const LANDING_RADIUS = 34.0;
const LANDING_TOLERANCE = 3.0;
const HIT_SETTLE_SECONDS = 0.5;
const SPEAKING_DISTANCE = 280.0;
const TEACHING_STATION_RADIUS = 24.0;

fn declare() {
    #{
        provider: "ambition", key: "intro_training",
        schemas: [
            #{ key: "intro_training.lesson", attachment: "body", fields: [
                #{ tag: 1, name: "phase", kind: "u32" },
                #{ tag: 2, name: "was_struck", kind: "bool" },
                #{ tag: 3, name: "settled", kind: "f32" },
            ] },
        ],
        entries: [
            #{ key: "lesson", phase: "actor_think",
               trigger: #{ port: "story_tick", selector: "intro_wake_room" },
               writes: ["intro_training.lesson"],
               requests: ["story_command", "play_cutscene"], limit: 4 },
        ],
    }
}

fn command(line) { this.requests.push(request("story_command", #{ line: line })); }
fn scene(id) { this.requests.push(request("play_cutscene", #{ id: id })); }

fn lesson() {
    let room = this.trigger;
    let state = this.state.lesson;
    if !("intro_training_active" in room.flags) {
        if !room.busy && "intro_raid_ready" in room.flags && !("intro_raid_arrival_seen" in room.flags) && !("intro_creator_fragment_seen" in room.flags) {
            this.scene("intro_raid_at_the_door");
        }
        state.phase = 0;
        state.settled = 0.0;
        this.state.lesson = state;
        return;
    }
    if room.busy { return; }
    let learners = room.bodies.filter(|body| body.driven && body.alive);
    let teachers = room.bodies.filter(|body| body.form == "npc_creator" && body.alive);
    let bags = room.bodies.filter(|body| body.form == "practice_sandbag");
    if learners.is_empty() || teachers.is_empty() || bags.is_empty() { return; }
    let teacher = teachers[0];
    let bag = bags[0];
    let learner = learners[0];
    // The teacher must be in the same shot before a lesson opens a box.
    if abs(teacher.position[0] - learner.position[0]) > global::SPEAKING_DISTANCE { return; }
    let stations = room.marks.filter(|mark| mark.name == "intro_practice_station");
    if stations.is_empty() { throw "the workshop has no intro_practice_station mark"; }
    if !teacher.on_ground || abs(teacher.position[0] - stations[0].position[0]) > global::TEACHING_STATION_RADIUS { return; }
    if state.phase == 0 {
        state.phase = 1;
    }
    if state.phase == 1 {
        let marks = room.marks.filter(|mark| mark.name == "intro_training_landing");
        if marks.is_empty() { throw "the workshop has no intro_training_landing mark"; }
        let mark = marks[0].position;
        let feet = learner.position[1] + learner.size[1] * 0.5;
        if learner.on_ground && abs(feet - mark[1]) <= global::LANDING_TOLERANCE
            && abs(learner.position[0] - mark[0]) <= global::LANDING_RADIUS {
            state.was_struck = bag.struck_recently;
            state.phase = 2;
            this.command("world.set_flag intro_training_movement_done true");
            this.scene("intro_training_combat");
        }
    } else if state.phase == 2 {
        if bag.struck_recently && !state.was_struck {
            state.phase = 3;
            state.settled = 0.0;
        }
        state.was_struck = bag.struck_recently;
    } else if state.phase == 3 {
        state.settled += this.dt;
        if state.settled >= global::HIT_SETTLE_SECONDS && learner.on_ground {
            state.phase = 4;
            this.scene("intro_training_finished");
        }
    }
    this.state.lesson = state;
}
