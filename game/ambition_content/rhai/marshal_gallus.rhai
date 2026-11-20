// Marshal Gallus's moves (`docs/planning/game/leadensaddle.md`): each is a
// `Special` of his pattern (`boss_profiles.ron`, "marshal_gallus"). The
// pattern says which move and when; this script says what each one throws.
// The spur kick is the body's own `side_sweep` strike, not a move here.
//
// The corral's floor is 64 over the room's bottom; its loading docks stand
// 224 wide at each side, so the tumbleweeds start at the docks' inner faces.
// Room-local coordinates, +Y down.
const FLOOR_INSET = 64.0;
const DOCK_W = 240.0;

// The quickdraw: three shots, each aimed where you are as it leaves.
const DRAW_SHOTS = 3;
const DRAW_INTERVAL_S = 0.14;
const SHOT_SPEED = 640.0;
// The gun's muzzle, from his body's centre, before facing.
const MUZZLE = [70.0, -18.0];

// The dynamite: three sticks lobbed to land round you, each a blast.
const STICKS = 3;
const STICK_INTERVAL_S = 0.18;
const STICK_FLIGHT_S = 0.95;
const STICK_GRAVITY = 900.0;
const STICK_SPREAD = 96.0;

// The tumbleweeds his whistle drives in from both docks, a pair each wave.
const WEED_INTERVAL_S = 1.05;
const WEED_SPEED = 360.0;
const WEED_GRAVITY = 1100.0;

// The whistle: rings of sound along the floor, both ways.
const RING_SPEED = 430.0;

// High noon: he marks where you stand as the warning starts, and fires down
// that line when it ends.
const NOON_SHOTS = 6;
const NOON_INTERVAL_S = 0.05;
const NOON_SPEED = 1500.0;

const WARN_INTERVAL_S = 0.25;

fn declare() {
    #{ provider: "ambition", key: "marshal_gallus",
       schemas: [
           #{ key: "marshal_gallus.draw", fields: [
               #{ tag: 1, name: "cooldown_s", kind: "f32" },
               #{ tag: 2, name: "shots", kind: "u32" },
           ] },
           #{ key: "marshal_gallus.sticks", fields: [
               #{ tag: 1, name: "cooldown_s", kind: "f32" },
               #{ tag: 2, name: "thrown", kind: "u32" },
           ] },
           #{ key: "marshal_gallus.weeds", fields: [
               #{ tag: 1, name: "cooldown_s", kind: "f32" },
               #{ tag: 2, name: "waves", kind: "u32" },
           ] },
           #{ key: "marshal_gallus.whistle", fields: [
               #{ tag: 1, name: "blown", kind: "bool" },
               #{ tag: 2, name: "cooldown_s", kind: "f32" },
           ] },
           #{ key: "marshal_gallus.rope", fields: [
               #{ tag: 1, name: "thrown", kind: "bool" },
               #{ tag: 2, name: "cooldown_s", kind: "f32" },
           ] },
           #{ key: "marshal_gallus.haul", fields: [
               #{ tag: 1, name: "pulled", kind: "bool" },
           ] },
           #{ key: "marshal_gallus.noon", fields: [
               #{ tag: 1, name: "marked", kind: "bool" },
               #{ tag: 2, name: "mark_x", kind: "f32" },
               #{ tag: 3, name: "mark_y", kind: "f32" },
               #{ tag: 4, name: "cooldown_s", kind: "f32" },
               #{ tag: 5, name: "shots", kind: "u32" },
           ] },
       ],
       entries: [
           #{ key: "quickdraw", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_quickdraw" },
              writes: ["marshal_gallus.draw"], requests: ["projectile", "effect", "sound"], limit: 6, idle: "reset" },
           #{ key: "dynamite", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_dynamite" },
              writes: ["marshal_gallus.sticks"], requests: ["projectile", "effect", "sound"], limit: 6, idle: "reset" },
           #{ key: "tumbleweeds", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_tumbleweeds" },
              writes: ["marshal_gallus.weeds"], requests: ["projectile", "effect", "sound"], limit: 6, idle: "reset" },
           #{ key: "whistle", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_whistle" },
              writes: ["marshal_gallus.whistle"], requests: ["projectile", "effect", "sound"], limit: 8, idle: "reset" },
           #{ key: "lasso", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_lasso" },
              writes: ["marshal_gallus.rope"], requests: ["spawn_module_entity", "effect", "sound"], limit: 4, idle: "reset" },
           #{ key: "haul", phase: "module_entity_tick",
              trigger: #{ port: "module_entity_tick", selector: "marshal_gallus.lasso_loop" },
              writes: ["marshal_gallus.haul"], requests: ["launch_toucher", "effect", "end_module_entity"], limit: 6 },
           #{ key: "high_noon", phase: "technique_execution",
              trigger: #{ port: "boss_special", selector: "gallus_high_noon" },
              writes: ["marshal_gallus.noon"], requests: ["projectile", "effect", "sound"], limit: 8, idle: "reset" },
       ],
    }
}

fn floor_y(caster) {
    caster.room_size[1] - global::FLOOR_INSET
}

fn muzzle(caster) {
    [caster.position[0] + global::MUZZLE[0] * caster.facing, caster.position[1] + global::MUZZLE[1]]
}

fn aim(from, to) {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    if dx == 0.0 && dy == 0.0 { [1.0, 0.0] } else { [dx, dy] }
}

fn quickdraw() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let draw = this.state.draw;
    draw.cooldown_s = max(0.0, draw.cooldown_s - this.dt);
    let gun = muzzle(caster);
    if caster.telegraphing && draw.cooldown_s <= 0.0 {
        this.requests.push(request("effect", #{ at: gun, fx: "alert_ping", scale: 0.9 }));
        draw.cooldown_s = global::WARN_INTERVAL_S;
    }
    if caster.pressed && type_of(caster.target) != "()" {
        if draw.shots == 0 { draw.cooldown_s = 0.0; }
        if draw.cooldown_s <= 0.0 && draw.shots < global::DRAW_SHOTS {
            this.requests.push(request("projectile", #{
                origin: gun, dir: aim(gun, caster.target), speed: global::SHOT_SPEED,
                damage: 1, max_lifetime: 1.6, half_extent: [8.0, 4.0], visual_id: "pistol_round",
            }));
            this.requests.push(request("sound", #{ cue: "enemy.pirate.pistol_shoot", at: gun }));
            draw.shots += 1;
            draw.cooldown_s = global::DRAW_INTERVAL_S;
        }
    }
    this.state.draw = draw;
}

fn dynamite() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" || type_of(caster.target) == "()" { return; }
    let sticks = this.state.sticks;
    sticks.cooldown_s = max(0.0, sticks.cooldown_s - this.dt);
    let floor = floor_y(caster);
    if caster.telegraphing && sticks.cooldown_s <= 0.0 {
        this.requests.push(request("effect", #{ at: [caster.target[0], floor - 12.0], fx: "alert_ping", scale: 1.1 }));
        sticks.cooldown_s = global::WARN_INTERVAL_S;
    }
    if caster.pressed {
        if sticks.thrown == 0 { sticks.cooldown_s = 0.0; }
        if sticks.cooldown_s <= 0.0 && sticks.thrown < global::STICKS {
            // Thrown to land at `goal` after the flight time: v = d/T - g T / 2.
            let from = [caster.position[0], caster.position[1] - 40.0];
            let offset = (sticks.thrown.to_float() - 1.0) * global::STICK_SPREAD;
            let width = caster.room_size[0];
            let goal_x = max(global::DOCK_W, min(width - global::DOCK_W, caster.target[0] + offset));
            let t = global::STICK_FLIGHT_S;
            let vx = (goal_x - from[0]) / t;
            let vy = (floor - 8.0 - from[1]) / t - 0.5 * global::STICK_GRAVITY * t;
            let speed = sqrt(vx * vx + vy * vy);
            this.requests.push(request("projectile", #{
                origin: from, dir: [vx, vy], speed: speed, gravity: global::STICK_GRAVITY,
                damage: 2, max_lifetime: t + 0.6, half_extent: [10.0, 10.0],
                splash_half_extent: 52.0, visual_id: "gallus_dynamite",
            }));
            sticks.thrown += 1;
            sticks.cooldown_s = global::STICK_INTERVAL_S;
        }
    }
    this.state.sticks = sticks;
}

fn tumbleweeds() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let weeds = this.state.weeds;
    weeds.cooldown_s = max(0.0, weeds.cooldown_s - this.dt);
    let width = caster.room_size[0];
    let floor = floor_y(caster);
    let starts = [[global::DOCK_W + 24.0, 1.0], [width - global::DOCK_W - 24.0, -1.0]];
    if caster.telegraphing && weeds.cooldown_s <= 0.0 {
        for s in starts {
            this.requests.push(request("effect", #{ at: [s[0], floor - 24.0], fx: "wind_curl", scale: 1.2 }));
        }
        weeds.cooldown_s = global::WARN_INTERVAL_S * 2.0;
    }
    if caster.pressed {
        if weeds.waves == 0 { weeds.cooldown_s = 0.0; }
        if weeds.cooldown_s <= 0.0 {
            for s in starts {
                this.requests.push(request("projectile", #{
                    origin: [s[0], floor - 26.0], dir: [s[1], -0.45], speed: global::WEED_SPEED,
                    gravity: global::WEED_GRAVITY, bounces: 12, damage: 1, max_lifetime: 3.4,
                    half_extent: [18.0, 18.0], visual_id: "gallus_tumbleweed",
                }));
                this.requests.push(request("effect", #{ at: [s[0], floor - 20.0], fx: "leaf_swirl", scale: 1.0 }));
            }
            weeds.waves += 1;
            weeds.cooldown_s = global::WEED_INTERVAL_S;
        }
    }
    this.state.weeds = weeds;
}

fn whistle() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let blow = this.state.whistle;
    blow.cooldown_s = max(0.0, blow.cooldown_s - this.dt);
    let floor = floor_y(caster);
    let chest = [caster.position[0] + 20.0 * caster.facing, caster.position[1] - 50.0];
    if caster.telegraphing && blow.cooldown_s <= 0.0 {
        this.requests.push(request("effect", #{ at: chest, fx: "wind_curl", scale: 0.8 }));
        blow.cooldown_s = global::WARN_INTERVAL_S;
    }
    if caster.pressed && !blow.blown {
        for side in [-1.0, 1.0] {
            this.requests.push(request("projectile", #{
                origin: [caster.position[0] + side * 50.0, floor - 16.0], dir: [side, 0.0], speed: global::RING_SPEED,
                damage: 1, max_lifetime: 2.0, half_extent: [14.0, 16.0], visual_id: "sound_ring",
            }));
        }
        this.requests.push(request("effect", #{ at: chest, fx: "phase_ripple", scale: 1.6 }));
        this.requests.push(request("sound", #{ cue: "enemy.chicken.notice", at: chest }));
        blow.blown = true;
    }
    this.state.whistle = blow;
}

fn lasso() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" || type_of(caster.target) == "()" { return; }
    let rope = this.state.rope;
    rope.cooldown_s = max(0.0, rope.cooldown_s - this.dt);
    if caster.telegraphing && rope.cooldown_s <= 0.0 {
        this.requests.push(request("effect", #{ at: caster.target, fx: "alert_ping", scale: 1.0 }));
        rope.cooldown_s = global::WARN_INTERVAL_S;
    }
    if caster.pressed && !rope.thrown {
        // The loop drops where you are; whoever is in it is hauled to him.
        this.requests.push(request("spawn_module_entity", #{
            kind: "marshal_gallus.lasso_loop", at: caster.target, lifetime_s: 0.3,
            half_extents: [44.0, 40.0], params: #{ pull_x: caster.position[0] },
        }));
        this.requests.push(request("effect", #{ at: caster.target, fx: "wind_curl", scale: 1.4 }));
        this.requests.push(request("sound", #{ cue: "boss.trex.tail_whip", at: caster.target }));
        rope.thrown = true;
    }
    this.state.rope = rope;
}

fn haul() {
    let lasso = this.state.haul;
    if lasso.pulled { return; }
    let pull_x = this.trigger.params.pull_x;
    let touchers = this.trigger.touchers;
    for i in 0..touchers.len() {
        let body = touchers[i];
        let dx = pull_x - body.position[0];
        // The Marshal himself, when the loop drops at his feet.
        if abs(dx) < 60.0 { continue; }
        // Hauled to stop a stride short of him: a short throw from near, a
        // long one from across the yard.
        let speed = max(-760.0, min(760.0, dx * 2.4));
        this.requests.push(request("launch_toucher", #{ toucher: i, velocity: [speed, -300.0] }));
        this.requests.push(request("effect", #{ at: body.position, fx: "dash_streak", scale: 1.0 }));
        lasso.pulled = true;
    }
    if lasso.pulled { this.requests.push(request("end_module_entity", #{})); }
    this.state.haul = lasso;
}

fn high_noon() {
    let caster = this.trigger;
    if !caster.alive || type_of(caster.room_size) == "()" { return; }
    let noon = this.state.noon;
    noon.cooldown_s = max(0.0, noon.cooldown_s - this.dt);
    if caster.telegraphing {
        if !noon.marked && type_of(caster.target) != "()" {
            noon.marked = true;
            noon.mark_x = caster.target[0];
            noon.mark_y = caster.target[1];
            this.requests.push(request("sound", #{ cue: "enemy.chicken.tell", at: caster.target }));
        }
        if noon.marked && noon.cooldown_s <= 0.0 {
            this.requests.push(request("effect", #{ at: [noon.mark_x, noon.mark_y], fx: "alert_ping", scale: 1.4 }));
            noon.cooldown_s = global::WARN_INTERVAL_S * 0.8;
        }
    }
    if caster.pressed && noon.marked {
        if noon.shots == 0 { noon.cooldown_s = 0.0; }
        if noon.cooldown_s <= 0.0 && noon.shots < global::NOON_SHOTS {
            let gun = muzzle(caster);
            this.requests.push(request("projectile", #{
                origin: gun, dir: aim(gun, [noon.mark_x, noon.mark_y]), speed: global::NOON_SPEED,
                damage: 2, max_lifetime: 1.2, half_extent: [10.0, 5.0], visual_id: "pistol_round",
            }));
            if noon.shots % 2 == 0 {
                this.requests.push(request("sound", #{ cue: "enemy.pirate.pistol_shoot", at: gun }));
            }
            noon.shots += 1;
            noon.cooldown_s = global::NOON_INTERVAL_S;
        }
    }
    this.state.noon = noon;
}
