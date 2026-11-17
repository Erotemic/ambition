// The equipped technique uses the same body and projectile path for each controller.
const VOLLEY_INTERVAL_S = 0.35;
const MANA_PER_VOLLEY = 9.0;
const BOLT_SPEED = 680.0;
const BOLT_DAMAGE = 3;
const BOLT_LIFETIME_S = 0.65;
const SPREAD = 0.18;

fn declare() {
    #{
        provider: "ambition", key: "lightning",
        schemas: [#{ key: "lightning.cadence", fields: [
            #{ tag: 1, name: "cooldown_s", kind: "f32" },
        ] }],
        entries: [#{ key: "volley", phase: "technique_execution",
            trigger: #{ port: "technique_use", selector: "lightning" },
            writes: ["lightning.cadence"],
            requests: ["spend_mana", "projectile", "sound"], limit: 5 }],
    }
}

fn volley() {
    let c = this.state.cadence;
    c.cooldown_s = max(0.0, c.cooldown_s - this.dt);
    let user = this.trigger.user;
    if (user.pressed || this.trigger.held) && c.cooldown_s <= 0.0
        && user.names_spawns && type_of(user.mana) != "()"
        && user.mana >= global::MANA_PER_VOLLEY {
        let aim = user.aim_local;
        let length = sqrt(aim[0] * aim[0] + aim[1] * aim[1]);
        let x = user.facing;
        let y = 0.0;
        if length > 0.001 { x = aim[0] / length; y = aim[1] / length; }
        this.requests.push(request("spend_mana", #{ amount: global::MANA_PER_VOLLEY }));
        for spread in [-global::SPREAD, 0.0, global::SPREAD] {
            let lx = x - y * spread;
            let ly = y + x * spread;
            let dir = [user.frame_side[0] * lx + user.frame_down[0] * ly,
                       user.frame_side[1] * lx + user.frame_down[1] * ly];
            this.requests.push(request("projectile", #{
                origin: user.position, dir: dir, speed: global::BOLT_SPEED,
                damage: global::BOLT_DAMAGE, max_lifetime: global::BOLT_LIFETIME_S,
                half_extent: [3.0, 3.0], visual_id: "lightning",
            }));
        }
        this.requests.push(request("sound", #{ cue: "boss.wolfenstein.zap", at: user.position }));
        c.cooldown_s = global::VOLLEY_INTERVAL_S;
    }
    this.state.cadence = c;
}
