// The cavern floor is 176 px above the bottom of the room.
const FLOOR_INSET = 176.0;
const MARK_INTERVAL = 0.16;

fn declare() {
    #{
        provider: "ambition", key: "mother_lode",
        schemas: [
            #{ key: "mother_lode.fault", fields: [
                #{ tag: 1, name: "released", kind: "bool" },
            ] },
            #{ key: "mother_lode.shed", fields: [
                #{ tag: 1, name: "marked", kind: "bool" },
                #{ tag: 2, name: "released", kind: "bool" },
                #{ tag: 3, name: "x", kind: "f32" },
                #{ tag: 4, name: "until_mark", kind: "f32" },
            ] },
        ],
        entries: [
            #{ key: "fault", phase: "technique_execution",
               trigger: #{ port: "boss_special", selector: "lode_fault" },
               writes: ["mother_lode.fault"], requests: ["projectile", "effect", "sound"], limit: 5, idle: "reset" },
            #{ key: "shed", phase: "technique_execution",
               trigger: #{ port: "boss_special", selector: "lode_shed" },
               writes: ["mother_lode.shed"], requests: ["projectile", "effect", "sound"], limit: 6, idle: "reset" },
        ],
    }
}

fn fault() {
    let caster = this.trigger;
    if !caster.alive || !caster.pressed || type_of(caster.room_size) == "()" { return; }
    let state = this.state.fault;
    if state.released { return; }
    let floor = caster.room_size[1] - global::FLOOR_INSET;
    for side in [-1.0, 1.0] {
        let start = [caster.position[0] + side * 48.0, floor - 14.0];
        this.requests.push(request("projectile", #{
            origin: start, dir: [side, 0.0], speed: 390.0,
            half_extent: [17.0, 12.0], max_lifetime: 1.35, damage: 2,
            visual_id: "lode_fault_wave",
        }));
        this.requests.push(request("effect", #{ at: start, fx: "phase_ripple", scale: 1.34 }));
    }
    this.requests.push(request("sound", #{ cue: "boss.gnu_ton.stomp", at: [caster.position[0], floor] }));
    state.released = true;
    this.state.fault = state;
}

fn shed() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let state = this.state.shed;
    let floor = caster.room_size[1] - global::FLOOR_INSET;
    // Fix the three impact locations at the start of the warning.
    if caster.telegraphing && !state.marked && type_of(caster.target) != "()" {
        state.x = max(414.0, min(1530.0, caster.target[0]));
        state.marked = true;
    }
    if !state.marked { return; }
    if caster.telegraphing {
        state.until_mark -= this.dt;
        if state.until_mark <= 0.0 {
            for offset in [-118.0, 0.0, 118.0] {
                let x = state.x + offset;
                this.requests.push(request("effect", #{ at: [x, floor - 10.0], fx: "alert_ping", scale: 1.08 }));
                this.requests.push(request("effect", #{ at: [x, 260.0], fx: "phase_ripple", scale: 0.57 }));
            }
            state.until_mark = global::MARK_INTERVAL;
        }
    }
    if caster.pressed && !state.released {
        for offset in [-118.0, 0.0, 118.0] {
            let at = [state.x + offset, 260.0];
            this.requests.push(request("projectile", #{
                origin: at, dir: [0.0, 1.0], speed: 270.0, gravity: 1100.0,
                half_extent: [18.0, 24.0], max_lifetime: 1.25, damage: 2,
                visual_id: "lode_falling_crystal",
            }));
        }
        this.requests.push(request("sound", #{ cue: "boss.trex.tail_whip", at: caster.position }));
        state.released = true;
    }
    this.state.shed = state;
}
