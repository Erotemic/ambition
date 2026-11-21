// This policy uses the level's activation boxes and changes the pan response.
// Edit response_hz while the game runs. The view owns the counter and elapsed time.
fn declare() {
    #{ provider: "ambition", key: "camera",
       schemas: [#{ key: "camera_view", attachment: "view", fields: [
           #{ tag: 1, name: "frames", kind: "u32" },
           #{ tag: 2, name: "elapsed", kind: "f32" }
       ] }],
       entries: [#{ key: "follow", phase: "camera_policy",
           trigger: #{ port: "camera_policy", selector: "view" },
           writes: ["camera_view"], requests: ["camera_intent"] }]
    }
}

fn follow() {
    let selected = "";
    let priority = -2147483648;
    for region in this.trigger.regions {
        let was = !this.trigger.reset && region.id == this.trigger.previous_region;
        let margin = if was { -region.exit_margin } else { region.enter_margin };
        let inside = was && region.latched;
        for body in this.trigger.subjects {
            let at = body.physical_position;
            if region.sample == "feet" {
                let extent = (abs(body.down[0])*body.size[0] + abs(body.down[1])*body.size[1])*0.5;
                at = [at[0]+body.down[0]*extent, at[1]+body.down[1]*extent];
            }
            let half = if region.sample == "full_body" { [body.size[0]*0.5, body.size[1]*0.5] } else { [0.0,0.0] };
            inside = inside || (at[0]-half[0] >= region.activation_min[0]+margin && at[0]+half[0] <= region.activation_max[0]-margin
                && at[1]-half[1] >= region.activation_min[1]+margin && at[1]+half[1] <= region.activation_max[1]-margin);
        }
        if inside && (region.priority > priority || (region.priority == priority && (selected == "" || region.id < selected))) {
            selected = region.id;
            priority = region.priority;
        }
    }
    this.state.camera_view.frames += 1;
    this.state.camera_view.elapsed += this.dt;
    let regions = if selected == "" { [] } else { [#{ id: selected, weight: 1.0 }] };
    this.requests.push(request("camera_intent", #{ regions: regions, response_hz: 7.0 }));
}
