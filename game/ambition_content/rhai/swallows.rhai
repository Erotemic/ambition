// Swallows fetch coconuts from the primeval trail and let them go over what
// they hunt (runtime authoring audit P4). The game ships this script; it is
// the rule, not a copy of one.
//
// An African swallow lifts a coconut alone. A European swallow carries half
// of one, so two lift it together on a sling: the one that takes it holds it,
// and its partner supports the other end. One invocation decides for every
// swallow of a room (`group_think`), so two birds never both count on one
// coconut: a pair decides once for both, a coconut taken this tick is no one
// else's, and a bird already flying in a pair is not asked again.
//
// What each bird is doing between ticks is the room's record: its goal (a
// coconut, by identity), its phase and how long it has been in it.

// How much of a coconut each kind lifts.
const CAPACITY = #{ european_swallow: 0.5, african_swallow: 1.0 };
// How long it strains at a coconut too heavy for it, how long it waits there
// for a partner, and how long after it gives up before it tries again (s).
const ATTEMPT_SECONDS = 0.8;
const WAIT_SECONDS = 4.0;
const RETRY_SECONDS = 3.0;
// How near another swallow must be to share its goal, and how near the
// coconut each must be to take it.
const PARTNER_REACH = 600.0;
const JOIN_RADIUS = 24.0;
// Half the sling's span, and how hard a bird keeps its place in the pair.
const HALF_SPAN = 24.0;
const FORMATION_GAIN = 4.0;
// It goes for a coconut at rest no farther from it than this (px).
const FETCH_REACH = 900.0;
// How fast it flies to a coconut, and how fast with one: it is a five ounce
// bird (px/s; its own speed is 290).
const FETCH_SPEED = 250.0;
const LADEN_SPEED = 185.0;
// It carries the coconut this far over its target (px).
const CARRY_OVER = 150.0;
// It lets go within this of the place over its target, side to side, and no
// lower over it than LET_GO_OVER (px). A coconut that falls that far arrives
// faster than the speed it hurts at.
const LET_GO_WITHIN = 18.0;
const LET_GO_OVER = 90.0;
// It aims where its target will be after this long (s): about the fall.
const LEAD_SECONDS = 0.35;

const APPROACH = 0;
const STRAIN = 1;
const WAIT = 2;
const DEPART = 3;

fn declare() {
    #{
        provider: "ambition",
        key: "swallows",
        schemas: [
            #{ key: "swallows.flight", attachment: "body",
               fields: [
                   #{ tag: 1, name: "birds", kind: "seq 16 body" },
                   #{ tag: 2, name: "goals", kind: "seq 16 opt item" },
                   #{ tag: 3, name: "phases", kind: "seq 16 u32" },
                   #{ tag: 4, name: "elapsed", kind: "seq 16 f32" },
               ] },
        ],
        entries: [
            #{ key: "fetch", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "swallow" },
               writes: ["swallows.flight"], requests: ["steer", "take", "release"],
               limit: 32, idle: "skip_if_initial" },
        ],
    }
}

fn length(v) { sqrt(v[0] * v[0] + v[1] * v[1]) }
fn sub(a, b) { [a[0] - b[0], a[1] - b[1]] }
fn add(a, b) { [a[0] + b[0], a[1] + b[1]] }
fn scale(v, k) { [v[0] * k, v[1] * k] }
fn distance(a, b) { length(sub(a, b)) }
fn distance_squared(a, b) { let d = sub(a, b); d[0] * d[0] + d[1] * d[1] }

// As glam's `normalize_or_zero`.
fn normalize_or_zero(v) {
    let rcp = 1.0 / length(v);
    if rcp > 0.0 && rcp < 1.0e38 { [v[0] * rcp, v[1] * rcp] } else { [0.0, 0.0] }
}

// As glam's `clamp_length_max`.
fn clamp_length_max(v, most) {
    let length_sq = v[0] * v[0] + v[1] * v[1];
    if length_sq > most * most {
        let len = sqrt(length_sq);
        [most * (v[0] / len), most * (v[1] / len)]
    } else {
        v
    }
}

// Steer the load over where the target will be, or let it go there: `()` to
// let go, else the velocity to carry it at.
fn laden(load, target, target_velocity) {
    let over = [target[0] + target_velocity[0] * global::LEAD_SECONDS, target[1] - global::CARRY_OVER];
    if abs(load[0] - over[0]) <= global::LET_GO_WITHIN && target[1] - load[1] >= global::LET_GO_OVER {
        return ();
    }
    scale(normalize_or_zero(sub(over, load)), global::LADEN_SPEED)
}

fn index_of(list, id) {
    for (item, n) in list {
        if item == id { return n; }
    }
    -1
}

fn fetch() {
    let room = this.trigger;
    let dt = room.dt;
    let record = this.state.flight;

    // The swallows of the room, each with what it was doing.
    let birds = [];
    let flights = [];
    for member in room.members {
        if !(member.form in global::CAPACITY) { continue; }
        birds.push(#{
            id: member.id,
            position: member.position,
            velocity: member.velocity,
            height: member.size[1],
            capacity: global::CAPACITY[member.form],
            available: member.alive && !member.driven,
            alive: member.alive,
            held: member.held,
            supporter: member.supporter,
            target: member.target,
            target_velocity: if type_of(member.target_velocity) == "()" { [0.0, 0.0] } else { member.target_velocity },
        });
        let n = index_of(record.birds, member.id);
        flights.push(if n < 0 {
            #{ goal: (), phase: global::APPROACH, elapsed: 0.0 }
        } else {
            #{ goal: record.goals[n], phase: record.phases[n], elapsed: record.elapsed[n] }
        });
    }
    let nuts = room.items.filter(|item| item.kind == "coconut" && item.at_rest);
    let busy = [];
    let taken = [];

    // An existing pair makes one decision for both control frames.
    for i in 0..birds.len() {
        let bird = birds[i];
        if type_of(bird.supporter) == "()" { continue; }
        let j = -1;
        for (other, n) in birds { if other.id == bird.supporter { j = n; } }
        busy.push(bird.id);
        busy.push(bird.supporter);
        let anchor = if j < 0 {
            [bird.position[0], bird.position[1] + (bird.height * 0.5 + 8.0)]
        } else {
            let helper = birds[j];
            let tallest = if bird.height > helper.height { bird.height } else { helper.height };
            add(scale(add(bird.position, helper.position), 0.5), [0.0, tallest * 0.5 + 8.0])
        };
        let intact = j >= 0
            && bird.available && birds[j].available
            && bird.held == "coconut" && type_of(birds[j].held) == "()"
            && bird.capacity + birds[j].capacity >= 1.0
            && distance(bird.position, birds[j].position) <= global::HALF_SPAN * 4.0 + global::JOIN_RADIUS;
        let decision = if type_of(bird.target) == "()" { [0.0, 0.0] } else { laden(anchor, bird.target, bird.target_velocity) };
        if !intact || type_of(decision) == "()" {
            this.requests.push(request("release", #{
                body: bird.id, thrown: intact, velocity: [bird.velocity[0] * 0.25, 0.0],
            }));
            flights[i] = #{ goal: flights[i].goal, phase: global::DEPART, elapsed: 0.0 };
            if j >= 0 {
                flights[j] = #{ goal: flights[j].goal, phase: global::DEPART, elapsed: 0.0 };
            }
        } else if j >= 0 {
            let centre = scale(add(bird.position, birds[j].position), 0.5);
            for pair in [[i, -1.0], [j, 1.0]] {
                let k = pair[0];
                let formation = add(centre, [pair[1] * global::HALF_SPAN, 0.0]);
                let corrected = add(decision, scale(sub(formation, birds[k].position), global::FORMATION_GAIN));
                this.requests.push(request("steer", #{
                    body: birds[k].id, velocity: clamp_length_max(corrected, global::FETCH_SPEED),
                }));
            }
        }
    }

    // Commit nearby arrivals to the same load before either can take custody.
    for i in 0..birds.len() {
        let bird = birds[i];
        if bird.id in busy || !bird.available || type_of(bird.held) != "()" { continue; }
        let flight = flights[i];
        flight.elapsed += dt;
        if flight.phase == global::DEPART {
            if flight.elapsed < global::RETRY_SECONDS {
                flights[i] = flight;
                continue;
            }
            flight = #{ goal: (), phase: global::APPROACH, elapsed: 0.0 };
        }
        let valid = nuts.filter(|nut| distance(nut.position, bird.position) <= global::FETCH_REACH);
        let committed = valid.filter(|nut| nut.id == flight.goal);
        if committed.is_empty() {
            let goal = ();
            for (other, n) in birds {
                if n == i || !other.available || type_of(other.held) != "()" { continue; }
                if flights[n].phase == global::DEPART { continue; }
                if distance(other.position, bird.position) > global::PARTNER_REACH { continue; }
                let theirs = valid.filter(|nut| nut.id == flights[n].goal);
                if !theirs.is_empty() {
                    goal = theirs[0].id;
                    break;
                }
            }
            if type_of(goal) == "()" {
                let best = ();
                for nut in valid {
                    if type_of(best) == "()" { best = nut; continue; }
                    let d = distance_squared(nut.position, bird.position);
                    let e = distance_squared(best.position, bird.position);
                    if d < e || (d == e && nut.id < best.id) { best = nut; }
                }
                if type_of(best) != "()" { goal = best.id; }
            }
            flight = #{ goal: goal, phase: global::APPROACH, elapsed: 0.0 };
        }
        flights[i] = flight;
    }

    for i in 0..birds.len() {
        let bird = birds[i];
        if bird.id in busy { continue; }
        if bird.held == "coconut" {
            if bird.alive && !bird.available { continue; }
            let from = [bird.position[0], bird.position[1] + (bird.height * 0.5 + 8.0)];
            let decision = if type_of(bird.target) == "()" { "none" } else { laden(from, bird.target, bird.target_velocity) };
            if !bird.alive || bird.capacity < 1.0 || type_of(decision) == "()" {
                this.requests.push(request("release", #{
                    body: bird.id, thrown: bird.alive && bird.capacity >= 1.0,
                    velocity: [bird.velocity[0] * 0.25, 0.0],
                }));
                flights[i] = #{ goal: flights[i].goal, phase: global::DEPART, elapsed: 0.0 };
            } else if type_of(decision) == "array" {
                this.requests.push(request("steer", #{ body: bird.id, velocity: decision }));
            }
            continue;
        }
        if !bird.available || type_of(bird.held) != "()" { continue; }
        let flight = flights[i];
        if flight.phase == global::DEPART {
            if flight.elapsed < global::ATTEMPT_SECONDS {
                let away = 1.0;
                for nut in nuts {
                    if nut.id == flight.goal {
                        away = if bird.position[0] < nut.position[0] { -1.0 } else { 1.0 };
                        break;
                    }
                }
                this.requests.push(request("steer", #{
                    body: bird.id, velocity: scale(normalize_or_zero([away, -1.0]), global::FETCH_SPEED),
                }));
            }
            continue;
        }
        let nut = ();
        for candidate in nuts {
            if candidate.id == flight.goal && !(candidate.id in taken) { nut = candidate; break; }
        }
        if type_of(nut) == "()" { continue; }
        let approach = [nut.position[0], nut.position[1] - (bird.height * 0.5 + 4.0)];
        let arrived = distance(bird.position, approach) <= global::JOIN_RADIUS;
        let partner = -1;
        for (other, n) in birds {
            if n == i || !other.available || type_of(other.held) != "()" || other.id in busy { continue; }
            if flights[n].phase == global::DEPART || flights[n].goal != flight.goal { continue; }
            if distance(other.position, bird.position) > global::PARTNER_REACH { continue; }
            if other.capacity + bird.capacity < 1.0 { continue; }
            partner = n;
            break;
        }
        let joined = if partner >= 0 && distance(birds[partner].position, approach) <= global::JOIN_RADIUS { partner } else { -1 };
        if arrived && (bird.capacity >= 1.0 || joined >= 0) {
            let supporter = if bird.capacity < 1.0 { birds[joined].id } else { () };
            this.requests.push(request("take", #{ body: bird.id, item: nut.id, supporter: supporter }));
            taken.push(nut.id);
            busy.push(bird.id);
            if bird.capacity < 1.0 { busy.push(birds[joined].id); }
            continue;
        }
        let aim = approach;
        if arrived && bird.capacity < 1.0 {
            if flight.phase == global::APPROACH {
                flight.phase = global::STRAIN;
                flight.elapsed = 0.0;
            } else if flight.phase == global::STRAIN && flight.elapsed >= global::ATTEMPT_SECONDS {
                if partner >= 0 {
                    flight.phase = global::WAIT;
                    flight.elapsed = 0.0;
                } else {
                    flight.phase = global::DEPART;
                    flight.elapsed = 0.0;
                }
            } else if flight.phase == global::WAIT && (partner < 0 || flight.elapsed >= global::WAIT_SECONDS) {
                flight.phase = global::DEPART;
                flight.elapsed = 0.0;
            }
            if flight.phase == global::STRAIN {
                aim[1] -= 8.0 * abs(sin(flight.elapsed * 12.0));
            }
        } else if flight.elapsed > global::WAIT_SECONDS + global::FETCH_REACH / (if global::FETCH_SPEED > 1.0 { global::FETCH_SPEED } else { 1.0 }) {
            flight.phase = global::DEPART;
            flight.elapsed = 0.0;
        }
        flights[i] = flight;
        this.requests.push(request("steer", #{
            body: bird.id,
            velocity: clamp_length_max(scale(sub(aim, bird.position), global::FORMATION_GAIN), global::FETCH_SPEED),
        }));
    }

    record.birds = [];
    record.goals = [];
    record.phases = [];
    record.elapsed = [];
    for (flight, i) in flights {
        if type_of(flight.goal) == "()" && flight.phase == global::APPROACH && flight.elapsed == 0.0 {
            continue;
        }
        record.birds.push(birds[i].id);
        record.goals.push(flight.goal);
        record.phases.push(flight.phase);
        record.elapsed.push(flight.elapsed);
    }
    this.state.flight = record;
}
