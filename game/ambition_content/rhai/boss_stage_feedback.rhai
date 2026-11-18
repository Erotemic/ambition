// A climb starts a warning. The guitar then carries a short electrical pulse.
const WARNING_S = 1.1;
const LIVE_S = 0.25;
const RECOVERY_S = 4.0;
const APPROACH_DISTANCE = 72.0;
const RAY_WIDTH = 28.0;

fn declare() {
    #{ provider: "ambition", key: "boss_stage_feedback",
       schemas: [#{ key: "boss_stage_feedback.charge", fields: [
           #{ tag: 1, name: "phase", kind: "u32" },
           #{ tag: 2, name: "left_s", kind: "f32" },
           #{ tag: 3, name: "generation", kind: "u32" },
       ] }],
       entries: [#{ key: "feedback", phase: "boss_conduct",
           trigger: #{ port: "boss_conduct", selector: "bossofthegame" },
           reads: ["mount"], writes: ["boss_stage_feedback.charge"],
           requests: ["held_ray", "lightning", "sound"], limit: 8 }],
    }
}

fn feedback() {
    let boss = this.trigger;
    let charge = this.state.charge;
    if !boss.alive || !boss.awake || boss.invulnerable || boss.driven {
        charge.phase = 0;
        charge.left_s = 0.0;
        this.state.charge = charge;
        return;
    }
    let mount = this.observe.mount;
    if type_of(mount) == "()" { return; }
    let slopes = [];
    for i in 1..mount.slope.len() {
        let a = mount.slope[i - 1];
        let b = mount.slope[i];
        slopes.push([a[0], a[1], b[0], b[1]]);
    }
    if slopes.is_empty() { return; }
    charge.left_s = max(0.0, charge.left_s - this.dt);
    if charge.phase == 3 && charge.left_s <= 0.0 { charge.phase = 0; }
    if charge.phase == 0 && type_of(boss.target_body) != "()" {
        let target = boss.target_body;
        let feet = [target.position[0], target.position[1] + target.size[1] * 0.5];
        let near = false;
        for line in slopes {
            let dx = line[2] - line[0];
            let dy = line[3] - line[1];
            let t = max(0.0, min(1.0, ((feet[0] - line[0]) * dx + (feet[1] - line[1]) * dy) / (dx * dx + dy * dy)));
            let x = feet[0] - line[0] - dx * t;
            let y = feet[1] - line[1] - dy * t;
            if x * x + y * y <= global::APPROACH_DISTANCE * global::APPROACH_DISTANCE { near = true; }
        }
        if near {
            charge.phase = 1;
            charge.left_s = global::WARNING_S;
            charge.generation += 1;
        }
    }
    if charge.phase == 1 && charge.left_s <= 0.0 {
        charge.phase = 2;
        charge.left_s = global::LIVE_S;
        this.requests.push(request("sound", #{ cue: "boss.wolfenstein.zap", at: mount.position }));
    }
    if charge.phase == 2 && charge.left_s <= 0.0 {
        charge.phase = 3;
        charge.left_s = global::RECOVERY_S;
    }
    for i in 0..slopes.len() {
        let line = slopes[i];
        // The warning and the strike use the same live surface endpoints.
        let from = [line[0], line[1] - 12.0];
        let to = [line[2], line[3] - 12.0];
        if charge.phase == 1 {
            this.requests.push(request("lightning", #{
                key: "guitar_warning_" + i, from: #{ world: from }, to: #{ world: to },
                look: "force", glow: [0.9, 0.55, 0.12, 0.4], hold_s: 0.05,
            }));
        } else if charge.phase == 2 {
            let dx = to[0] - from[0];
            let dy = to[1] - from[1];
            this.requests.push(request("held_ray", #{
                slot: i, generation: charge.generation, from: #{ world: from },
                toward: [dx, dy], reach: sqrt(dx * dx + dy * dy),
                width: global::RAY_WIDTH, damage: 1, lifetime_s: global::LIVE_S,
                look: "force",
            }));
        }
    }
    this.state.charge = charge;
}
