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
            ] },
        ],
        entries: [#{
            key: "perform", phase: "boss_conduct",
            trigger: #{ port: "boss_conduct", selector: "gram_reaper_boss" },
            writes: ["gram_reaper.beat", "gram_reaper.cut"],
            requests: ["attack_mark", "held_damage_box", "drawn_row", "sound", "burst"],
            limit: 10,
        }],
    }
}

fn clamp_x(x, hall, margin) {
    let safe_margin = min(margin, (hall.right - hall.left) * 0.5);
    max(hall.left + safe_margin, min(hall.right - safe_margin, x))
}

fn cuts_for(key, boss) {
    let x = boss.position[0];
    let y = boss.position[1];
    let hall = if boss.hall == () {
        #{ left: 0.0, right: boss.room[0], floor: y + boss.size[1] * 0.5 }
    } else { boss.hall };
    let target_x = if boss.driven || boss.target_body == () { x + boss.side * 120.0 }
                   else { boss.target_body.position[0] };
    let side = if target_x < x { -1.0 } else { 1.0 };
    let floor = if boss.driven { min(hall.floor, y + boss.size[1] * 0.5) } else { hall.floor };

    switch key {
        "reaper_graph_cut" => {
            let near = x - side * 18.0;
            let far = x + side * 238.0;
            [beats::box_between(max(hall.left, min(near, far)), floor - 52.0,
                                min(hall.right, max(near, far)), floor)]
        }
        "reaper_projection" => {
            let axis = clamp_x(target_x, hall, 58.0);
            [beats::box_between(axis - 52.0, max(0.0, floor - 330.0), axis + 52.0, floor)]
        }
        "reaper_edge_volley" => {
            let origin = clamp_x(target_x - side * 128.0, hall, 193.0);
            let regions = [];
            for i in 0..3 {
                let lane = origin + side * 126.0 * (i.to_float() - 1.0);
                regions.push(beats::box_between(lane - 57.0, floor - 84.0, lane + 57.0, floor));
            }
            regions
        }
        "reaper_null_space" => {
            let axis = clamp_x(x, hall, 206.0);
            [
                beats::box_between(hall.left, floor - 58.0, axis - 132.0, floor),
                beats::box_between(axis + 132.0, floor - 58.0, hall.right, floor),
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

fn impact(requests, key, center, floor) {
    let at = [center[0], min(floor - 12.0, center[1] + 18.0)];
    requests.push(request("burst", #{
        at: at, count: 11, speed: 195.0, color: [0.82, 0.51, 1.0, 1.0], kind: "spark",
    }));
    let cue = switch key {
        "reaper_projection" => "boss.gram_reaper.projection_fire",
        "reaper_edge_volley" => "boss.gram_reaper.edge_fire",
        "reaper_null_space" => "enemy.gram_reaper.notice",
        _ => "enemy.gram_reaper.swing",
    };
    requests.push(request("sound", #{ cue: cue, at: at }));
    requests
}

fn perform() {
    let boss = this.trigger;
    let frame = beats::advance(this.state.beat, boss, this.dt);
    let beat = frame.beat;
    let cut = this.state.cut;
    let requests = this.requests;
    if frame.started {
        cut.regions = if boss.room == () { [] } else { cuts_for(beat.key, boss) };
    }

    let pose = pose_for(beat.key);
    if beat.stage == "idle" || pose == () {
        if boss.alive && boss.awake && !boss.driven && boss.between_phases && !cut.transition {
            requests.push(request("burst", #{
                at: boss.position, count: 22, speed: 235.0,
                color: [0.68, 0.39, 1.0, 1.0], kind: "spark",
            }));
        }
        if cut.pinned { requests.push(beats::pose("", 0.0)); }
        cut.regions = [];
        cut.pinned = false;
    } else if beat.stage == "warning" || beat.stage == "strike" {
        let live = beat.stage == "strike";
        let row_time = if live { pose.strike_s + (pose.recovery_s - pose.strike_s) * frame.progress }
                       else { min(pose.strike_s - 0.015, pose.strike_s * frame.progress) };
        requests.push(beats::pose(pose.name, row_time));
        cut.pinned = true;
        let heavy = beat.key == "reaper_null_space" || beat.key == "reaper_projection";
        let damage = if heavy || boss.enraged { 2 } else { 1 };
        let in_order = beat.key == "reaper_edge_volley";
        let boxes = beats::boxes(frame, cut.regions, in_order, damage, 0.8);
        requests += boxes.requests;
        if boxes.fired != () {
            let box = cut.regions[boxes.fired];
            let floor_y = box.center[1] + box.half[1];
            let at = if beat.key == "reaper_null_space" {
                [boss.position[0], floor_y - 30.0]
            } else { [box.center[0], floor_y - min(34.0, box.half[1])] };
            requests = impact(requests, beat.key, at, floor_y);
        }
    } else if beat.age < 0.48 {
        let elapsed = pose.recovery_s + (pose.end_s - pose.recovery_s) * beat.age / 0.48;
        requests.push(beats::pose(pose.name, elapsed));
    } else if cut.pinned {
        requests.push(beats::pose("", 0.0));
        cut.pinned = false;
    }
    cut.transition = boss.between_phases;
    this.state.beat = beat;
    this.state.cut = cut;
    this.requests = requests;
}
