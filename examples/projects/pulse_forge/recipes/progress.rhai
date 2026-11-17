// Keep each authored body's technique ticks in its durable save record.
fn declare() {
    #{ provider: "pulse_forge", key: "progress",
       schemas: [#{ key: "progress.ticks", attachment: "body", save: "durable",
           fields: [#{tag: 1, name: "ticks", kind: "u32"}]
       }],
       entries: [#{key: "remember", phase: "technique_execution",
           trigger: #{port: "technique_use", selector: "pulse_forge"},
           writes: ["progress.ticks"], idle: "invoke"
       }]
    }
}

fn remember() {
    if this.dt <= 0.0 { return; }
    let p = this.state.ticks;
    p.ticks += 1;
    this.state.ticks = p;
}
