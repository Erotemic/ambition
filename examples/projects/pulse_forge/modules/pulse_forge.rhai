// The character declares pulse_forge on the Projectile slot.
// The body and each emitter keep separate records in the rollback state.
const CHARGE_PER_PULSE_S = 0.2;
const MAX_CHARGE_S = 1.0;
const PULSE_INTERVAL_S = 0.25;
const BOLT_SPEED = 240.0;

fn declare() {
    #{
        provider: "pulse_forge",
        key: "emitter",
        schemas: [
            #{ key: "emitter.charge", fields: [
                #{ tag: 1, name: "seconds", kind: "f32" },
            ] },
            #{ key: "emitter.cadence", fields: [
                #{ tag: 1, name: "fired", kind: "u32" },
                #{ tag: 2, name: "cooldown_s", kind: "f32" },
            ] },
        ],
        entries: [
            #{ key: "charge", phase: "technique_execution",
               trigger: #{ port: "technique_use", selector: "pulse_forge" },
               writes: ["emitter.charge"], requests: ["spend_mana", "spawn_module_entity"], limit: 2,
               idle: "reset" },
            #{ key: "pulse", phase: "module_entity_tick",
               trigger: #{ port: "module_entity_tick", selector: "pulse_forge.emitter" },
               writes: ["emitter.cadence"],
               requests: ["projectile", "end_module_entity"], limit: 4 },
        ],
    }
}

fn charge() {
    let c = this.state.charge;
    if this.trigger.held || this.trigger.user.pressed {
        c.seconds = min(c.seconds + this.dt, global::MAX_CHARGE_S);
    }
    if this.trigger.released {
        let pulses = min(5, (c.seconds / global::CHARGE_PER_PULSE_S).to_int());
        if pulses > 0 && this.trigger.user.names_spawns {
            this.requests.push(request("spend_mana", #{ amount: pulses * 12.0 }));
            this.requests.push(request("spawn_module_entity", #{
                kind: "pulse_forge.emitter", at: this.trigger.user.position,
                lifetime_s: 3.0, params: #{ pulses: pulses },
            }));
        }
        c.seconds = 0.0;
    }
    this.state.charge = c;
}

fn pulse() {
    let c = this.state.cadence;
    if c.fired >= this.trigger.params.pulses {
        this.requests.push(request("end_module_entity", #{}));
        return;
    }
    c.cooldown_s -= this.dt;
    if c.cooldown_s <= 0.0 {
        for direction in [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]] {
            this.requests.push(request("projectile", #{
                origin: this.trigger.position, dir: direction,
                speed: global::BOLT_SPEED, damage: 2,
                max_lifetime: 1.0, half_extent: [5.0, 5.0],
            }));
        }
        c.fired += 1;
        c.cooldown_s = global::PULSE_INTERVAL_S;
    }
    this.state.cadence = c;
}
