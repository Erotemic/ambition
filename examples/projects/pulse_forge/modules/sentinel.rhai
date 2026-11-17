// Tagged bodies patrol together through the shared control path.
fn declare() {
    #{ provider: "pulse_forge", key: "sentinel",
       schemas: [#{ key: "sentinel.patrol", fields: [
           #{tag: 1, name: "origin", kind: "opt vec2"},
           #{tag: 2, name: "elapsed_s", kind: "f32"},
           #{tag: 3, name: "direction", kind: "i32", initial: 1},
           #{tag: 4, name: "turns", kind: "u32"}
       ]}],
       entries: [#{key: "think", phase: "actor_think",
           trigger: #{port: "group_think", selector: "pulse_forge.sentinels"},
           writes: ["sentinel.patrol"], requests: ["steer"], limit: 8
       }]
    }
}

fn think() {
    if this.trigger.dt <= 0.0 { return; }
    let anchor = ();
    for member in this.trigger.members {
        if member.alive && !member.driven { anchor = member.position; break; }
    }
    if type_of(anchor) == "()" { return; }
    let p = this.state.patrol;
    if type_of(p.origin) == "()" { p.origin = anchor; }
    p.elapsed_s += this.trigger.dt;
    let outside = (anchor[0] > p.origin[0] + 80.0 && p.direction > 0)
        || (anchor[0] < p.origin[0] - 80.0 && p.direction < 0);
    if outside || p.elapsed_s >= 2.0 {
        p.direction = -p.direction;
        p.turns += 1;
        p.elapsed_s = 0.0;
    }
    for member in this.trigger.members {
        if member.alive && !member.driven {
            this.requests.push(request("steer", #{body: member.id, velocity: [p.direction * 48.0, 0.0]}));
        }
    }
    this.state.patrol = p;
}
