import "ambition/boss_beats.rhai" as beats;

fn declare() {
    #{
        provider: "ambition", key: "barrow_king",
        schemas: [
            beats::schema("barrow_king.beat"),
            #{ key: "barrow_king.cast", fields: [
                #{ tag: 1, name: "boxes", kind: #{ seq: 2, of: beats::box_kind() } },
                #{ tag: 2, name: "pose_pinned", kind: "bool" },
                #{ tag: 3, name: "phase_age", kind: "f32" },
                #{ tag: 4, name: "walk_age", kind: "f32" },
            ] },
        ],
        entries: [#{
            key: "perform", phase: "boss_conduct",
            trigger: #{ port: "boss_conduct", selector: "barrow_king" },
            writes: ["barrow_king.beat", "barrow_king.cast"],
            requests: ["attack_mark", "held_damage_box", "drawn_row", "sound", "burst", "camera_shake"],
            limit: 9,
        }],
    }
}

fn band(left, right, floor, height) {
    beats::box_between(min(left, right), floor - height, max(left, right), floor)
}

fn targets(boss, key) {
    let x = boss.position[0];
    let floor = boss.position[1] + boss.size[1] * 0.5;
    let aimed = if boss.target_body == () || boss.driven { x + boss.facing * 144.0 }
                else { boss.target_body.position[0] };
    let side = if aimed < x { -1.0 } else { 1.0 };
    let left = 144.0;
    let right = boss.room[0] - 368.0;
    let mark = max(left, min(right, aimed));
    let shoulder = boss.size[0] * 0.5;
    switch key {
        "barrow_reap" => [band(x + side * 12.0, x + side * (shoulder + 100.0), floor, 63.0)],
        "barrow_sceptre" => [band(x - shoulder - 116.0, x + shoulder + 116.0, floor, 38.0)],
        "barrow_brand" => [band(mark - 41.0, mark + 41.0, floor, 190.0)],
        "barrow_sentence" => {
            // Keep both seals inside the arena and leave a gap between them.
            let offset = if mark - 132.0 < left || (mark < x && mark + 132.0 <= right) { 132.0 } else { -132.0 };
            let echo = mark + offset;
            [band(mark - 40.0, mark + 40.0, floor, 190.0),
             band(echo - 40.0, echo + 40.0, floor, 190.0)]
        }
        _ => [],
    }
}

fn rows(key) {
    switch key {
        "barrow_reap" => ["reap_tell", 0.70, "reap", 0.34, 3, "enemy.decomposing_specter.swing"],
        "barrow_sceptre" => ["sceptre_tell", 0.84, "sceptre_slam", 0.40, 4, "boss.gnu_ton.stomp"],
        "barrow_brand" => ["brand_tell", 0.99, "brand", 0.40, 3, "projectile.energy_orb.charge"],
        "barrow_sentence" => ["sentence_tell", 1.10, "sentence", 0.58, 4, "boss.gnu_ton.stomp"],
    }
}

fn perform() {
    let boss = this.trigger;
    let frame = beats::advance(this.state.beat, boss, this.dt);
    let beat = frame.beat;
    let cast = this.state.cast;
    let requests = this.requests;
    let moving = boss.alive && boss.awake && abs(boss.velocity[0]) > 10.0;
    cast.walk_age = if moving { (cast.walk_age + this.dt) % 0.88 } else { 0.0 };

    if frame.started && boss.room != () {
        cast.boxes = targets(boss, beat.key);
    }
    let animation = rows(beat.key);
    if boss.alive && boss.awake && boss.between_phases {
        requests.push(beats::pose("coronation", cast.phase_age));
        cast.phase_age += this.dt;
        cast.pose_pinned = true;
    } else {
        cast.phase_age = 0.0;
        if animation != () && (beat.stage == "warning" || beat.stage == "strike") {
            if beat.stage == "warning" {
                requests.push(beats::pose(animation[0], animation[1] * frame.progress));
            } else {
                requests.push(beats::pose(animation[2], animation[3] * frame.progress));
            }
            cast.pose_pinned = true;
            let boxes = beats::boxes(frame, cast.boxes, beat.key == "barrow_sentence", animation[4], 0.67);
            requests += boxes.requests;
            if boxes.fired != () {
                let box = cast.boxes[boxes.fired];
                let at = [box.center[0], box.center[1] + box.half[1] - 8.0];
                requests.push(request("sound", #{ cue: animation[5], at: at }));
                requests.push(request("burst", #{
                    at: at, count: 13, speed: 136.0, color: [0.40, 0.98, 0.80, 1.0], kind: "spark",
                }));
                if beat.key == "barrow_sceptre" {
                    requests.push(request("camera_shake", #{ amplitude_px: 4.0 }));
                }
            }
        } else if moving {
            requests.push(beats::pose("walk", cast.walk_age));
            cast.pose_pinned = true;
        } else if animation != () && beat.stage == "recovery" && beat.age < 0.68 {
            requests.push(beats::pose("spent", beat.age * 1.38));
        } else if cast.pose_pinned {
            requests.push(beats::pose("", 0.0));
            cast.pose_pinned = false;
        }
    }
    this.state.beat = beat;
    this.state.cast = cast;
    this.requests = requests;
}
