// Lightning from the outstretched hand: a short cone of crackling arcs, held
// for as long as the slot is held and paid for with mana each second.
//
// The body holds out its near hand (the `zap` row, above the hips: the legs
// walk under it) and five arcs leave that hand in a cone toward the aim.
// Each arc is a ray (it hurts what it meets and stops at a wall) drawn as a
// held channel of lightning. Every pulse the arcs take new paths in the cone,
// and an arc that meets a body hits it again.
const MANA_PER_S = 24.0;
// How far the arcs reach: a short-range weapon, about two bodies away.
const REACH = 118.0;
// Half the cone's angle, in radians (about 28 degrees each side).
const CONE = 0.48;
// Each arc strays this far (radians) from its place in the cone each pulse.
const STRAY = 0.14;
// Seconds between pulses: each pulse is a new generation of arcs, and a new
// generation hits a body again.
const PULSE_S = 0.18;
const DAMAGE = 1;
// A light push: arcs that crackle, not a beam that shoves.
const KNOCKBACK = 0.25;
// An arc is asked for again each tick it crackles; this only outlives one.
const ARC_LIFETIME_S = 0.1;
const ARC_WIDTH = 5.0;
const JUMP_CHANCE = 0.35;
const JUMP_RANGE = 96.0;
const JUMP_DAMAGE = 1;
// How far the aim may lift or drop the cone from level (a slope).
const MAX_SLOPE = 0.9;
// Pulses between crackles of sound.
const CRACKLE_EVERY = 3;

fn declare() {
    #{
        provider: "ambition", key: "lightning",
        schemas: [#{ key: "lightning.cadence", fields: [
            #{ tag: 1, name: "crackling", kind: "bool" },
            #{ tag: 2, name: "generation", kind: "u32" },
            #{ tag: 3, name: "since_pulse", kind: "f32" },
            #{ tag: 4, name: "crackled", kind: "f32" },
            #{ tag: 5, name: "dry", kind: "bool" },
        ] }],
        entries: [#{ key: "zap", phase: "technique_execution",
            trigger: #{ port: "technique_use", selector: "lightning" },
            writes: ["lightning.cadence"],
            requests: ["spend_mana", "held_ray", "drawn_row", "sound"], limit: 10 }],
    }
}

// A number in -1..1 from a generation and an arc: the same on every peer and
// after every rewind.
fn stray_of(generation, arc) {
    let x = (generation * 7919 + arc * 104729 + 13) % 1009;
    (x / 504.0) - 1.0
}

fn zap() {
    let c = this.state.cadence;
    let user = this.trigger.user;
    let cost = global::MANA_PER_S * this.dt;
    let wants = user.pressed || this.trigger.held;
    if !wants { c.dry = false; }
    let can_pay = user.names_spawns && type_of(user.mana) != "()" && user.mana >= cost;
    if !wants || c.dry || !can_pay {
        c.dry = wants;
        if c.crackling {
            c.crackling = false;
            c.since_pulse = 0.0;
            c.crackled = 0.0;
            // The body's own row back.
            this.requests.push(request("drawn_row", #{ on: "body", names: [] }));
        }
        this.state.cadence = c;
        return;
    }
    let pulse = false;
    if c.crackling {
        c.since_pulse += this.dt;
        c.crackled += this.dt;
        if c.since_pulse >= global::PULSE_S {
            c.since_pulse -= global::PULSE_S;
            c.generation += 1;
            pulse = true;
        }
    } else {
        c.crackling = true;
        c.since_pulse = 0.0;
        c.crackled = 0.0;
        c.generation += 1;
        pulse = true;
        this.requests.push(request("sound", #{ cue: "boss.wolfenstein.zap", at: () }));
    }
    this.requests.push(request("spend_mana", #{ amount: cost }));

    // The cone's middle: the aim, on the side the body faces, lifted or
    // dropped no more than the outstretched arm can point.
    let side = if user.facing < 0.0 { -1.0 } else { 1.0 };
    let aim = user.aim_local;
    let slope = 0.0;
    if abs(aim[0]) > 0.001 {
        slope = aim[1] / abs(aim[0]);
    } else if abs(aim[1]) > 0.001 {
        slope = if aim[1] > 0.0 { global::MAX_SLOPE } else { -global::MAX_SLOPE };
    }
    slope = max(-global::MAX_SLOPE, min(global::MAX_SLOPE, slope));
    let middle = atan(slope);
    let rows = ["zap", "shoot"];
    let hand = #{ part: #{ part: "hand", rows: rows, elapsed: c.crackled, looping: true,
                           landmark: "hand_near" } };
    for arc in 0..5 {
        let place = (arc - 2) / 2.0;
        let angle = middle + place * global::CONE + global::STRAY * stray_of(c.generation, arc);
        let lx = side * cos(angle);
        let ly = sin(angle);
        let dir = [user.frame_side[0] * lx + user.frame_down[0] * ly,
                   user.frame_side[1] * lx + user.frame_down[1] * ly];
        // The outer arcs fall short of the middle ones, and each pulse they
        // reach a little farther or less far: a cone, not a fan of sticks.
        let reach = global::REACH * (0.78 + 0.22 * (1.0 - abs(place))
                                     + 0.08 * stray_of(c.generation + 31, arc));
        this.requests.push(request("held_ray", #{
            slot: arc, generation: c.generation, from: hand, toward: dir,
            reach: reach, width: global::ARC_WIDTH, damage: global::DAMAGE,
            knockback: global::KNOCKBACK, lifetime_s: global::ARC_LIFETIME_S, look: "force",
            jump: #{ chance: global::JUMP_CHANCE, range: global::JUMP_RANGE, damage: global::JUMP_DAMAGE },
        }));
    }
    this.requests.push(request("drawn_row", #{ on: "upper_body", names: rows,
                                               elapsed: c.crackled, looping: true }));
    if pulse && c.generation % global::CRACKLE_EVERY == 0 {
        this.requests.push(request("sound", #{ cue: "boss.wolfenstein.zap", at: () }));
    }
    this.state.cadence = c;
}
