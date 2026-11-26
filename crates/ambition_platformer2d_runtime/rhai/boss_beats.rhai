// The boss pattern owns time. This record tracks the edges of each beat.
fn schema(key) {
    #{ key: key, fields: [
        #{ tag: 1, name: "key", kind: "text 96" },
        #{ tag: 2, name: "stage", kind: "text 12", initial: "idle" },
        #{ tag: 3, name: "generation", kind: "u32" },
        #{ tag: 4, name: "duration", kind: "f32" },
        #{ tag: 5, name: "remaining", kind: "f32" },
        #{ tag: 6, name: "age", kind: "f32" },
        #{ tag: 7, name: "driven", kind: "bool" },
    ] }
}

fn advance(beat, boss, dt) {
    let started = false;
    if beat.driven != boss.driven {
        beat.key = "";
        beat.stage = "idle";
        beat.driven = boss.driven;
    }
    let previous = beat.stage;
    let previous_remaining = beat.remaining;
    let active = boss.active;
    let continued = active != () && active.key == beat.key &&
                    (previous == "warning" || previous == "strike");
    if !boss.alive || !boss.awake || boss.invulnerable || boss.between_phases {
        beat.stage = "idle";
        beat.key = "";
        beat.remaining = 0.0;
    } else if boss.telegraph != () {
        let warning = boss.telegraph;
        started = previous != "warning" || beat.key != warning.key || warning.remaining > beat.remaining;
        if started {
            beat.key = warning.key;
            beat.generation += 1;
        }
        beat.stage = "warning";
        beat.remaining = warning.remaining;
    } else if active != () && (continued || boss.driven) {
        if !continued {
            beat.key = active.key;
            beat.generation += 1;
            started = true;
        }
        beat.stage = "strike";
        beat.remaining = active.remaining;
    } else if beat.key != "" {
        beat.stage = "recovery";
        beat.remaining = 0.0;
    }

    let entered = started || previous != beat.stage;
    if entered {
        beat.duration = beat.remaining;
        beat.age = 0.0;
    } else {
        beat.age += dt;
    }
    let progress = fraction(beat, beat.remaining);
    let previous_progress = if entered { 0.0 } else { fraction(beat, previous_remaining) };
    #{ beat: beat, started: started, entered: entered, progress: progress,
       previous_progress: previous_progress }
}

fn fraction(beat, remaining) {
    if beat.duration <= 0.0 { return 0.0; }
    1.0 - min(1.0, remaining / beat.duration)
}

// Save the same rectangle for the warning and the strike.
fn box_kind() {
    #{ record: [
        #{ tag: 1, name: "center", kind: "vec2" },
        #{ tag: 2, name: "half", kind: "vec2" },
    ] }
}

fn box_between(left, top, right, bottom) {
    #{
        center: [(left + right) * 0.5, (top + bottom) * 0.5],
        half: [(right - left) * 0.5, (bottom - top) * 0.5],
    }
}

fn show_box(slot, box, progress, active) {
    request("attack_mark", #{
        slot: slot, center: box.center, half_extent: box.half,
        progress: progress, active: active,
    })
}

fn strike_box(slot, box, generation, damage, knockback) {
    request("held_damage_box", #{
        slot: slot, generation: generation, center: box.center, half_extent: box.half,
        damage: damage, knockback: knockback, lifetime_s: 0.08,
    })
}

fn slice_at(progress, count) {
    min(count - 1, floor(progress * count.to_float()).to_int())
}

// Play all rectangles together, or one per equal part of the strike.
fn boxes(frame, regions, in_order, damage, knockback) {
    let requests = [];
    let fired = ();
    let count = regions.len();
    let active = frame.beat.stage == "strike";
    if count == 0 || (!active && frame.beat.stage != "warning") {
        return #{ requests: requests, fired: fired };
    }

    let slice = slice_at(frame.progress, count);
    for i in 0..count {
        let box = regions[i];
        if box.half[0] <= 0.0 || box.half[1] <= 0.0 { continue; }
        let live = active && (!in_order || i == slice);
        let progress = if active && in_order {
            max(0.0, min(1.0, count.to_float() * frame.progress - i.to_float()))
        } else { frame.progress };
        if !active || live || (in_order && i > slice) {
            requests.push(show_box(i, box, progress, live));
        }
        if live {
            requests.push(strike_box(i, box, frame.beat.generation, damage, knockback));
        }
    }
    if active && (frame.entered || (in_order && slice != slice_at(frame.previous_progress, count))) {
        fired = if in_order { slice } else { 0 };
    }
    #{ requests: requests, fired: fired }
}

fn pose(row, elapsed) {
    let names = if row == "" { [] } else { [row] };
    request("drawn_row", #{ on: "body", names: names, elapsed: elapsed })
}
