// Hobford's sheep are cloud sheep (Jon, 2026-11-20): a sheep that is killed
// rises into the air as a cloud. The game ships this script; it is the rule.
//
// A body with the `cloud_sheep` tag that dies plays its death row (the
// fleece swells into a cumulus and lifts it), and as that row fades the
// cloud it became is made where it is: a body of `npc_sheep_cloud`. Each
// cloud (`sheep_cloud` tag) rises for a while, slowing as it goes, then
// hangs in the sky, bobbing, and drifts with the wind.
//
// What each room remembers between ticks is its record: the sheep that are
// dying and for how long, the sheep already made clouds, and each cloud's
// age.

// How long after the kill the cloud is made (s): the sheep's death row is
// 0.84 s and it fades from 0.6 s, so the cloud comes as it goes.
const BECOME_SECONDS = 0.6;
// The cloud's box, and how far over the sheep's middle it is made (px).
const CLOUD_HALF = [30.0, 18.0];
const CLOUD_LIFT = 10.0;
// How fast it rises at first, and how long it rises (px/s, s). It slows to
// a stop over that time, so it climbs about RISE_SPEED * RISE_SECONDS / 2.
const RISE_SPEED = 50.0;
const RISE_SECONDS = 8.0;
// The wind it drifts on once it is up (px/s, east), and its bob.
const WIND = 14.0;
const BOB_SPEED = 5.0;
const BOB_RATE = 0.7;
// The most sheep and clouds a room's record holds (its sequences' bound).
const MOST = 16;

fn declare() {
    #{
        provider: "ambition",
        key: "cloud_sheep",
        schemas: [
            #{ key: "cloud_sheep.flock", attachment: "body",
               fields: [
                   #{ tag: 1, name: "dying", kind: "seq 16 body" },
                   #{ tag: 2, name: "dying_for", kind: "seq 16 f32" },
                   #{ tag: 3, name: "clouded", kind: "seq 16 body" },
               ] },
            #{ key: "cloud_sheep.sky", attachment: "body",
               fields: [
                   #{ tag: 1, name: "clouds", kind: "seq 16 body" },
                   #{ tag: 2, name: "ages", kind: "seq 16 f32" },
               ] },
        ],
        entries: [
            #{ key: "become", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "cloud_sheep" },
               writes: ["cloud_sheep.flock"], requests: ["create_actors", "story_command"],
               limit: 2, idle: "skip_if_initial" },
            #{ key: "made", phase: "actor_think",
               trigger: #{ port: "action_result", selector: "made" } },
            #{ key: "drift", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "sheep_cloud" },
               writes: ["cloud_sheep.sky"], requests: ["steer"], limit: 16 },
        ],
    }
}

fn index_of(list, id) {
    for (item, n) in list {
        if item == id { return n; }
    }
    -1
}

fn become() {
    let room = this.trigger;
    let record = this.state.flock;
    let present = room.members.map(|member| member.id);
    // A sheep no longer in the room is forgotten.
    let dying = [];
    let dying_for = [];
    let clouded = record.clouded.filter(|id| id in present);
    let made = [];
    for member in room.members {
        if member.alive || member.id in clouded { continue; }
        let n = index_of(record.dying, member.id);
        let age = room.dt;
        if n >= 0 { age += record.dying_for[n]; }
        if dying.len() + clouded.len() >= global::MOST { continue; }
        if age < global::BECOME_SECONDS {
            dying.push(member.id);
            dying_for.push(age);
            continue;
        }
        clouded.push(member.id);
        made.push(#{
            key: "cloud_" + made.len(), character: "npc_sheep_cloud", allegiance: "neutral",
            at: [member.position[0], member.position[1] - global::CLOUD_LIFT],
            half_extents: global::CLOUD_HALF,
        });
    }
    if !made.is_empty() {
        this.requests.push(request("create_actors", #{ actors: made,
            reply: #{ request: 1, selector: "ambition::cloud_sheep/made" } }));
        this.requests.push(request("story_command", #{ line: "world.set_flag hobford_sheep_clouded true" }));
    }
    record.dying = dying;
    record.dying_for = dying_for;
    record.clouded = clouded;
    this.state.flock = record;
}

// A refused cloud is a fault to see, not a sheep quietly gone.
fn made() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
}

fn drift() {
    let room = this.trigger;
    let record = this.state.sky;
    let clouds = [];
    let ages = [];
    for member in room.members {
        if member.driven { continue; }
        if clouds.len() >= global::MOST { continue; }
        let n = index_of(record.clouds, member.id);
        let age = room.dt;
        if n >= 0 { age += record.ages[n]; }
        clouds.push(member.id);
        ages.push(age);
        // It rises, slowing to a stop; the wind takes it as it slows.
        let rising = if age < global::RISE_SECONDS { 1.0 - age / global::RISE_SECONDS } else { 0.0 };
        let up = global::RISE_SPEED * rising;
        let bob = global::BOB_SPEED * sin(age * global::BOB_RATE) * (1.0 - rising);
        let wind = global::WIND * (1.0 - rising);
        this.requests.push(request("steer", #{ body: member.id, velocity: [wind, bob - up] }));
    }
    record.clouds = clouds;
    record.ages = ages;
    this.state.sky = record;
}
