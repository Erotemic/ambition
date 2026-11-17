// The sentinel observes its target and decides through the body's control path.
fn declare() {
    #{ provider: "pulse_forge", key: "sentinel",
       schemas: [#{ key: "sentinel.clock", fields: [
           #{tag: 1, name: "ticks", kind: "u32"}
       ]}],
       entries: [#{key: "think", phase: "actor_think",
           trigger: #{port: "actor_think", selector: "pulse_forge.sentinel"},
           writes: ["sentinel.clock"], requests: ["velocity", "fire"], limit: 2
       }]
    }
}

fn think() {
    let clock = this.state.clock;
    clock.ticks += 1;
    this.state.clock = clock;
    if type_of(this.trigger.target) == "()" { return; }
    let dx = this.trigger.target[0] - this.trigger.position[0];
    let run = 0.0;
    if abs(dx) > 96.0 { run = if dx > 0.0 { 48.0 } else { -48.0 }; }
    this.requests.push(request("velocity", #{velocity: [run, this.trigger.velocity[1]]}));
    if abs(dx) < 200.0 && clock.ticks % 30 == 0 {
        this.requests.push(request("fire", #{direction: [if dx >= 0.0 { 1.0 } else { -1.0 }, 0.0]}));
    }
}
