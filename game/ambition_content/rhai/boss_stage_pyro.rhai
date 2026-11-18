// Both concert stages have a floor 64 units above the room's bottom.
// The jets cross the side risers. The centre stays clear for the giant's climb.
const FLOOR_INSET = 64.0;
const JET_X = [0.1875, 0.3125, 0.6875, 0.8125];
const RISER_HEIGHT = [224.0, 96.0, 96.0, 224.0];
const WARNING_INTERVAL_S = 0.25;
const PULSE_INTERVAL_S = 0.12;
const JET_SPEED = 720.0;
const JET_LIFETIME_S = 0.65;
const MAX_PULSES = 10;

fn declare() {
    #{ provider: "ambition", key: "boss_stage_pyro",
       schemas: [#{ key: "boss_stage_pyro.beat", fields: [
           #{ tag: 1, name: "cooldown_s", kind: "f32" },
           #{ tag: 2, name: "pulses", kind: "u32" },
           #{ tag: 3, name: "firing", kind: "bool" },
       ] }],
       entries: [#{ key: "jets", phase: "technique_execution",
           trigger: #{ port: "boss_special", selector: "boss_stage_pyro" },
           writes: ["boss_stage_pyro.beat"],
           requests: ["projectile", "effect", "sound"], limit: 8, idle: "reset" }],
    }
}

fn jets() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let beat = this.state.beat;
    beat.cooldown_s = max(0.0, beat.cooldown_s - this.dt);
    let width = caster.room_size[0];
    let floor = caster.room_size[1] - global::FLOOR_INSET;
    if caster.telegraphing && beat.cooldown_s <= 0.0 {
        for jet in 0..4 {
            let at = [width * global::JET_X[jet], floor - global::RISER_HEIGHT[jet] - 20.0];
            this.requests.push(request("effect", #{
                at: at, fx: "alert_ping", scale: 1.25,
            }));
            this.requests.push(request("effect", #{
                at: at, fx: "ember_wisp", scale: 0.6,
            }));
        }
        beat.cooldown_s = global::WARNING_INTERVAL_S;
    }
    if caster.pressed {
        if !beat.firing { beat.firing = true; beat.cooldown_s = 0.0; }
        if beat.cooldown_s <= 0.0 && beat.pulses < global::MAX_PULSES {
            let pair = if beat.pulses % 2 == 0 { [0, 3] } else { [1, 2] };
            for jet in pair {
                let at = [width * global::JET_X[jet], floor - global::RISER_HEIGHT[jet] - 20.0];
                this.requests.push(request("projectile", #{
                    origin: at, dir: [0.0, -1.0], speed: global::JET_SPEED,
                    damage: 1, max_lifetime: global::JET_LIFETIME_S,
                    half_extent: [14.0, 18.0], visual_id: "stage_pyro",
                }));
                this.requests.push(request("effect", #{ at: at, fx: "ember_wisp", scale: 1.0 }));
            }
            if beat.pulses == 0 {
                this.requests.push(request("sound", #{ cue: "boss.mockingbird.missile_launch", at: [width * 0.5, floor] }));
            }
            beat.pulses += 1;
            beat.cooldown_s = global::PULSE_INTERVAL_S;
        }
    }
    this.state.beat = beat;
}
