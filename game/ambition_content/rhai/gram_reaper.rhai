import "ambition/boss_beats.rhai" as beats;

fn declare() {
    #{
        provider: "ambition", key: "gram_reaper",
        schemas: [
            beats::schema("gram_reaper.beat"),
            #{ key: "gram_reaper.cut", fields: [
                #{ tag: 1, name: "regions", kind: #{ seq: 3, of: beats::box_kind() } },
                #{ tag: 2, name: "pinned", kind: "bool" },
                #{ tag: 3, name: "transition", kind: "bool" },
                #{ tag: 4, name: "anchor", kind: "vec2" },
                #{ tag: 5, name: "ground_y", kind: "f32" },
                #{ tag: 6, name: "side", kind: "f32" },
                #{ tag: 7, name: "held", kind: "bool" },
            ] },
        ],
        entries: [#{
            key: "perform", phase: "boss_conduct",
            trigger: #{ port: "boss_conduct", selector: "gram_reaper_boss" },
            writes: ["gram_reaper.beat", "gram_reaper.cut"],
            requests: ["attack_mark", "held_damage_box", "drawn_row", "conducted_pose",
                       "boss_guard", "sound", "burst", "camera_shake", "lightning"],
            limit: 14,
        }],
    }
}

fn clamp_x(x, hall, margin) {
    let safe_margin = min(margin, (hall.right - hall.left) * 0.5);
    max(hall.left + safe_margin, min(hall.right - safe_margin, x))
}

fn sweep(hall, x, side, reach, height) {
    let near = x - side * 12.0;
    let far = x + side * reach;
    beats::box_between(max(hall.left, min(near, far)), hall.floor - height,
                       min(hall.right, max(near, far)), hall.floor)
}

fn cuts_for(key, hall, x, target_x, side) {
    let floor = hall.floor;
    switch key {
        "reaper_graph_cut" => [sweep(hall, x, side, 218.0, 36.0)],
        "reaper_projection" => {
            let axis = clamp_x(x + side * 118.0, hall, 45.0);
            [sweep(hall, x, side, 280.0, 44.0),
             beats::box_between(axis - 45.0, max(0.0, floor - 310.0), axis + 45.0, floor)]
        }
        "reaper_edge_volley" => {
            let origin = clamp_x(target_x - side * 128.0, hall, 183.0);
            let regions = [];
            for i in 0..3 {
                let lane = origin + side * 126.0 * (i.to_float() - 1.0);
                regions.push(beats::box_between(lane - 57.0, floor - 84.0, lane + 57.0, floor));
            }
            regions
        }
        "reaper_null_space" => {
            let axis = clamp_x(x - side * 88.0, hall, 104.0);
            [
                beats::box_between(hall.left, floor - 104.0, axis - 94.0, floor),
                beats::box_between(axis + 94.0, floor - 104.0, hall.right, floor),
            ]
        }
        _ => [],
    }
}

fn pose_for(key) {
    switch key {
        "reaper_graph_cut" => row("attack_side", 0.06, 6, 10, 15),
        "reaper_projection" => row("ortho_slash", 0.06, 6, 11, 15),
        "reaper_edge_volley" => row("edge_volley", 0.07, 5, 9, 11),
        "reaper_null_space" => row("enrage", 0.09, 6, 9, 9),
    }
}

fn row(name, frame_s, strike_frame, recovery_frame, last_frame) {
    #{ name: name, strike_s: frame_s * strike_frame.to_float(),
       recovery_s: frame_s * recovery_frame.to_float(), end_s: frame_s * (last_frame.to_float() + 0.5) }
}

fn impact(key, box, boss_x) {
    let floor = box.center[1] + box.half[1];
    let at = if key == "reaper_null_space" { [boss_x, floor - 30.0] }
             else { [box.center[0], floor - min(34.0, box.half[1])] };
    let cue = switch key {
        "reaper_projection" => "boss.gram_reaper.projection_fire",
        "reaper_edge_volley" => "boss.gram_reaper.edge_fire",
        "reaper_null_space" => "enemy.gram_reaper.notice",
        _ => "enemy.gram_reaper.swing",
    };
    [
        request("burst", #{ at: at, count: 15, speed: 220.0,
                            color: [0.84, 0.55, 1.0, 1.0], kind: "spark" }),
        request("sound", #{ cue: cue, at: at }),
        request("camera_shake", #{ amplitude_px: if key == "reaper_null_space" { 8.0 } else { 4.0 } }),
    ]
}

fn constellations(frame, regions) {
    let requests = [];
    let live = frame.beat.stage == "strike";
    let glow = if live { [0.78, 0.32, 1.0, 0.75] } else { [0.55, 0.40, 1.0, 0.46] };
    for i in 0..regions.len() {
        let box = regions[i];
        let outward = if i == 0 { -1.0 } else { 1.0 };
        let edge = box.center[0] - outward * box.half[0];
        let span = min(144.0, box.half[0] * 1.8);
        let y = box.center[1] + box.half[1];
        let inner = [edge + outward * span * 0.12, y - 20.0];
        let middle = [edge + outward * span * 0.48, y - 78.0];
        let outer = [edge + outward * span, y - 35.0];
        requests.push(link("reaper.outer." + i.to_string(), outer, middle, glow));
        if live || frame.progress >= 0.32 {
            requests.push(link("reaper.inner." + i.to_string(), middle, inner, glow));
        }
    }
    requests
}

fn link(key, from, to, glow) {
    request("lightning", #{ key: key, from: #{ world: from }, to: #{ world: to },
                           glow: glow, look: "spark_gap", hold_s: 0.05 })
}

fn held_pose(cut, y) {
    request("conducted_pose", #{
        pose: #{ position: [cut.anchor[0], y], velocity: [0.0, 0.0] },
        side: cut.side,
    })
}

fn perform() {
    let boss = this.trigger;
    let frame = beats::advance(this.state.beat, boss, this.dt);
    let beat = frame.beat;
    let cut = this.state.cut;
    let requests = this.requests;
    if frame.started && boss.room != () {
        let x = boss.position[0];
        let y = boss.position[1];
        let feet = y + boss.size[1] * 0.5;
        let hall = if boss.hall == () { #{ left: 0.0, right: boss.room[0], floor: feet } }
                   else { boss.hall };
        let target_x = if boss.driven || boss.target_body == () { x + boss.facing * 120.0 }
                       else { boss.target_body.position[0] };
        if boss.driven { hall.floor = min(hall.floor, feet); }
        cut.anchor = boss.position;
        cut.ground_y = hall.floor - boss.size[1] * 0.5 - 1.0;
        cut.side = if target_x < x { -1.0 } else { 1.0 };
        cut.regions = cuts_for(beat.key, hall, x, target_x, cut.side);
    }

    let pose = pose_for(beat.key);
    let warning = beat.stage == "warning";
    let strike = beat.stage == "strike";
    let open_s = 0.88;
    let lift_s = 0.20;
    let recovering = beat.stage == "recovery" && beat.age < open_s + lift_s;
    let open = recovering && beat.age < open_s;
    if boss.alive {
        requests.push(request("boss_guard", #{ guarded: !boss.driven && !open }));
    }
    if pose != () && (warning || strike || recovering) && !boss.driven && boss.room != () {
        let y = if warning {
            let descent = min(1.0, frame.progress / 0.50);
            cut.anchor[1] + (cut.ground_y - cut.anchor[1]) * descent
        } else if open || strike { cut.ground_y }
        else { cut.ground_y - 50.0 * min(1.0, (beat.age - open_s) / lift_s) };
        requests.push(held_pose(cut, y));
        cut.held = true;
    } else if cut.held {
        requests.push(request("conducted_pose", #{}));
        cut.held = false;
    }

    if pose != () && (warning || strike) {
        let row_time = if strike { pose.strike_s + (pose.recovery_s - pose.strike_s) * frame.progress }
                       else { min(pose.strike_s - 0.015, pose.strike_s * frame.progress) };
        requests.push(beats::pose(pose.name, row_time));
        cut.pinned = true;
        let damage = switch beat.key {
            "reaper_projection" => 7,
            "reaper_edge_volley" => 4,
            "reaper_null_space" => 11,
            _ => 8,
        };
        let in_order = beat.key == "reaper_edge_volley" || beat.key == "reaper_projection";
        // Clear the sweep area when the blade passes. The pose continues.
        if !strike || beat.key != "reaper_graph_cut" || beat.age < 0.18 {
            let boxes = beats::boxes(frame, cut.regions, in_order, damage, 0.8);
            requests += boxes.requests;
            if beat.key == "reaper_null_space" {
                requests += constellations(frame, cut.regions);
            }
            if boxes.fired != () {
                requests += impact(beat.key, cut.regions[boxes.fired], cut.anchor[0]);
            }
        }
    } else if pose != () && recovering {
        let recoil = if beat.key == "reaper_null_space" { pose_for("reaper_graph_cut") } else { pose };
        let stuck = min(recoil.recovery_s + 0.09, recoil.end_s);
        let elapsed = if open { stuck }
                      else { stuck + (recoil.end_s - stuck) * (beat.age - open_s) / lift_s };
        requests.push(beats::pose(recoil.name, elapsed));
    } else if cut.pinned {
        requests.push(beats::pose("", 0.0));
        cut.pinned = false;
    }
    if boss.alive && boss.awake && !boss.driven && boss.between_phases && !cut.transition {
        requests.push(request("burst", #{
            at: boss.position, count: 22, speed: 235.0,
            color: [0.68, 0.39, 1.0, 1.0], kind: "spark",
        }));
    }
    cut.transition = boss.between_phases;
    this.state.beat = beat;
    this.state.cut = cut;
    this.requests = requests;
}
