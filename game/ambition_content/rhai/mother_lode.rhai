import "ambition/boss_beats.rhai" as beats;

const FAULT_SPEED = 390.0;
const FAULT_LIFETIME = 1.35;
const FAULT_OFFSET = 48.0;
const DROP_HEIGHT = 410.0;
const DROP_SPACING = 118.0;
const DROP_DELAY = 0.30;

fn declare() {
    #{
        provider: "ambition", key: "mother_lode",
        schemas: [
            beats::schema("mother_lode.beat"),
            #{ key: "mother_lode.seam", fields: [
                #{ tag: 1, name: "anchor", kind: "vec2" },
                #{ tag: 2, name: "regions", kind: #{ seq: 3, of: beats::box_kind() } },
                #{ tag: 3, name: "pinned", kind: "bool" },
                #{ tag: 4, name: "transition", kind: "bool" },
            ] },
        ],
        entries: [#{
            key: "perform", phase: "boss_conduct",
            trigger: #{ port: "boss_conduct", selector: "mother_lode" },
            writes: ["mother_lode.beat", "mother_lode.seam"],
            requests: ["attack_mark", "held_damage_box", "drawn_row", "projectile",
                       "sound", "burst", "camera_shake"],
            limit: 14,
        }],
    }
}

fn low_band(a, b, floor, height) {
    beats::box_between(min(a, b), floor - height, max(a, b), floor)
}

fn regions_for(key, x, floor, target_x, room_width) {
    switch key {
        "lode_hammer" => [
            low_band(x - 135.0, x - 46.0, floor, 44.0),
            low_band(x + 46.0, x + 135.0, floor, 44.0),
        ],
        "lode_fault" => {
            let reach = global::FAULT_OFFSET + global::FAULT_SPEED * global::FAULT_LIFETIME + 17.0;
            [
                low_band(max(0.0, x - reach), x - 31.0, floor, 26.0),
                low_band(x + 31.0, min(room_width, x + reach), floor, 26.0),
            ]
        }
        "lode_shed" => {
            let center = max(x - 534.0, min(x + 434.0, target_x));
            let top = floor - global::DROP_HEIGHT;
            let regions = [];
            for offset in [-global::DROP_SPACING, 0.0, global::DROP_SPACING] {
                let lane_x = center + offset;
                regions.push(beats::box_between(lane_x - 24.0, top, lane_x + 24.0, floor));
            }
            regions
        }
        "lode_split" => {
            let offset = if target_x < x { -108.0 } else { 108.0 };
            let gap = x + offset;
            [
                low_band(max(0.0, x - 448.0), gap - 76.0, floor, 58.0),
                low_band(gap + 76.0, min(room_width, x + 448.0), floor, 58.0),
            ]
        }
        _ => [],
    }
}

fn rows_for(key) {
    switch key {
        "lode_hammer" => ["hammer_lift", 0.86, "side_sweep", 0.34],
        "lode_fault" => ["fault_coil", 0.80, "fault", 0.36],
        "lode_shed" => ["shed_tell", 1.0, "shed", 0.52],
        "lode_split" => ["split_tell", 1.19, "split", 0.38],
    }
}

fn minerals(at, count, speed, color) {
    request("burst", #{ at: at, count: count, speed: speed, color: color, kind: "spark" })
}

fn floor_impact(regions, anchor, split) {
    let floor = anchor[1];
    let requests = [
        request("sound", #{ cue: "boss.gnu_ton.stomp", at: anchor }),
        request("camera_shake", #{ amplitude_px: if split { 7.0 } else { 4.0 } }),
    ];
    for box in regions {
        if split {
            for offset in [-0.65, 0.0, 0.65] {
                let x = box.center[0] + box.half[0] * offset;
                requests.push(minerals([x, floor - 24.0], 10, 174.0, [0.89, 0.51, 1.0, 1.0]));
            }
        } else {
            requests.push(minerals([box.center[0], floor - 12.0], 14, 140.0, [1.0, 0.65, 0.28, 1.0]));
        }
    }
    requests
}

fn fault(frame, regions, anchor) {
    let requests = [];
    if frame.beat.stage == "warning" {
        for i in 0..regions.len() {
            requests.push(beats::show_box(i, regions[i], frame.progress, false));
        }
    } else if frame.entered {
        for side in [-1.0, 1.0] {
            let start = [anchor[0] + side * global::FAULT_OFFSET, anchor[1] - 14.0];
            requests.push(request("projectile", #{
                origin: start, dir: [side, 0.0], speed: global::FAULT_SPEED,
                half_extent: [17.0, 12.0], max_lifetime: global::FAULT_LIFETIME, damage: 2,
                visual_id: "lode_fault_wave",
            }));
            requests.push(minerals(start, 13, 145.0, [0.45, 0.98, 0.77, 1.0]));
        }
        requests.push(request("sound", #{ cue: "boss.gnu_ton.stomp", at: anchor }));
        requests.push(request("camera_shake", #{ amplitude_px: 3.0 }));
    }
    requests
}

fn shed(frame, regions, dt) {
    let requests = [];
    let warning = frame.beat.stage == "warning";
    let age = frame.beat.age;
    for i in 0..regions.len() {
        let lane = regions[i];
        let at = [lane.center[0], lane.center[1] - lane.half[1]];
        let delay = if i == 1 { 0.0 } else { global::DROP_DELAY };
        let launched = !warning && age >= delay;
        // Show the column through its fall. The center crystal falls first.
        if warning || age < delay + 0.63 {
            let progress = if warning { frame.progress } else { max(0.0, min(1.0, (age - delay) / 0.63)) };
            requests.push(beats::show_box(i, lane, progress, launched));
        }
        if launched && ((frame.entered && delay == 0.0) || age - dt < delay) {
            requests.push(request("projectile", #{
                origin: at, dir: [0.0, 1.0], speed: 340.0, gravity: 1100.0,
                half_extent: [18.0, 24.0], max_lifetime: 0.68, damage: 2,
                visual_id: "lode_falling_crystal",
            }));
            requests.push(minerals(at, 12, 120.0, [0.78, 0.48, 1.0, 1.0]));
        }
    }
    if !warning && frame.entered {
        requests.push(request("sound", #{ cue: "boss.trex.tail_whip", at: regions[1].center }));
    }
    requests
}

fn perform() {
    let boss = this.trigger;
    let frame = beats::advance(this.state.beat, boss, this.dt);
    let beat = frame.beat;
    let seam = this.state.seam;
    let requests = this.requests;
    if frame.started && boss.room != () {
        let x = boss.position[0];
        let feet = boss.position[1] + boss.size[1] * 0.5;
        let floor = if boss.hall == () || boss.driven { feet } else { boss.hall.floor };
        let target_x = if boss.target_body == () || boss.driven { x + boss.facing * 120.0 }
                       else { boss.target_body.position[0] };
        seam.anchor = [x, floor];
        seam.regions = regions_for(beat.key, x, floor, target_x, boss.room[0]);
    }

    let rows = rows_for(beat.key);
    let warning = beat.stage == "warning";
    let strike = beat.stage == "strike";
    if rows != () && (warning || strike) && seam.regions.len() > 0 {
        if warning {
            requests.push(beats::pose(rows[0], rows[1] * frame.progress));
        } else {
            requests.push(beats::pose(rows[2], min(rows[3], beat.age)));
        }
        seam.pinned = true;
        switch beat.key {
            "lode_fault" => { requests += fault(frame, seam.regions, seam.anchor); }
            "lode_shed" => { requests += shed(frame, seam.regions, this.dt); }
            _ => {
                let split = beat.key == "lode_split";
                let boxes = beats::boxes(frame, seam.regions, false, if split { 3 } else { 2 }, 0.76);
                requests += boxes.requests;
                if boxes.fired != () {
                    requests += floor_impact(seam.regions, seam.anchor, split);
                }
            }
        }
    } else if rows != () && beat.stage == "recovery" && beat.age < 0.91 {
        requests.push(beats::pose("core_open", beat.age));
    } else if seam.pinned {
        requests.push(beats::pose("", 0.0));
        seam.pinned = false;
    }

    if boss.alive && boss.awake && boss.between_phases && !seam.transition {
        requests.push(minerals(boss.position, 22, 172.0, [0.84, 0.53, 1.0, 1.0]));
    }
    seam.transition = boss.between_phases;
    this.state.beat = beat;
    this.state.seam = seam;
    this.requests = requests;
}
