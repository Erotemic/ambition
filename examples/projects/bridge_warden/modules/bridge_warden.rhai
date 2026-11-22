// THE BRIDGE WARDEN. The yard makes a warden and two posts. The warden hears
// each post fall (a native defeat), counts it in the world, and once the
// world's count says both are down, lowers the bridge: a world value the
// save keeps. The bridge room's gate (`LockWall` gated by
// `world.value_is bridge.state lowered`) opens on that value, and the bridge
// room's own script reads the gate back. Edit this file while the game runs.

// How many fallen posts lower the bridge. Edit it and watch the next count.
fn posts_to_lower() { 2 }

fn declare() {
    #{provider: "bridge", key: "warden", schemas: [
        #{key: "yard", fields: [
            #{tag: 1, name: "made", kind: "seq 4 object"},
        ]},
        #{key: "ward", fields: [
            #{tag: 1, name: "downs", kind: "u32"},
            #{tag: 2, name: "gone", kind: "u32"},
            #{tag: 3, name: "asked", kind: "bool"},
            #{tag: 4, name: "log", kind: "text 256"},
        ]},
        #{key: "far", fields: [
            #{tag: 1, name: "seen", kind: "text 16"},
        ]},
    ], entries: [
        #{key: "yard", phase: "actor_think", trigger: #{port: "story_began", selector: "warden_yard"},
          writes: ["yard"], requests: ["create_actors"], limit: 1},
        #{key: "made", phase: "actor_think", trigger: #{port: "action_result", selector: "made"}, writes: ["yard"]},
        #{key: "down", phase: "actor_think", trigger: #{port: "object_defeated", selector: "defeated"},
          writes: ["ward"], requests: ["add_world_count"], limit: 1},
        #{key: "counted", phase: "actor_think", trigger: #{port: "action_result", selector: "counted"},
          reads: ["world_values"], writes: ["ward"], requests: ["set_world_value"], limit: 1},
        #{key: "lowered", phase: "actor_think", trigger: #{port: "action_result", selector: "lowered"}, writes: ["ward"]},
        #{key: "gone", phase: "actor_think", trigger: #{port: "object_retired", selector: "retired"}, writes: ["ward"]},
        #{key: "far", phase: "actor_think", trigger: #{port: "story_tick", selector: "warden_bridge"},
          reads: ["lock_walls"], writes: ["far"]},
    ]}
}

fn actor(key, character, x, allegiance) {
    #{key: key, character: character, at: [x, 300.0], allegiance: allegiance}
}

// The warden's watch over the posts it is made with: `north` and `south` are
// the posts' keys in the same request, so the binding is complete when the
// three are created, and no post can fall before the warden is listening.
fn ward() {
    #{key: "ward", references: #{north: #{local: "north"}, south: #{local: "south"}},
      methods: [#{name: "down", entry: "bridge::warden/down"}, #{name: "gone", entry: "bridge::warden/gone"}],
      subscriptions: [#{key: "north", source: "north", method: "down"}, #{key: "south", source: "south", method: "down"},
        #{key: "north_gone", source: "north", method: "gone"}, #{key: "south_gone", source: "south", method: "gone"}]}
}

// The yard, once per visit (`story_began`): the warden and the posts it keeps.
fn yard() {
    let warden = actor("warden", "bridge_keeper", 860.0, "neutral");
    warden.bindings = [ward()];
    this.requests.push(request("create_actors", #{reply: #{request: 1, selector: "bridge::warden/made"},
        actors: [warden, actor("north", "bridge_post", 560.0, "enemy"), actor("south", "bridge_post", 700.0, "enemy")]}));
}
// A refused yard is loud; a made one keeps what it made.
fn made() {
    if !this.trigger.terminal { return; }
    if this.trigger.code != "actors_created" { throw this.trigger.message; }
    this.state.yard.made = this.trigger.subjects.map(|subject| subject.object);
}

// The warden's methods: count each fallen post in the world, then lower the
// bridge on the count the world holds, hearing every outcome.
fn down() {
    this.state.ward.downs += 1;
    this.state.ward.log += "down;";
    this.requests.push(request("add_world_count", #{key: "bridge.posts_down", delta: 1,
        reply: #{request: this.state.ward.downs, selector: "bridge::warden/counted"}}));
}
fn count_of(values, key) {
    for row in values { if row.key == key { return row.value; } }
    0
}
fn counted() {
    if !this.trigger.terminal { return; }
    let ward = this.state.ward;
    ward.log += this.trigger.code + ";";
    if !ward.asked && count_of(this.observe.world_values.values, "bridge.posts_down") >= posts_to_lower() {
        ward.asked = true;
        this.requests.push(request("set_world_value", #{key: "bridge.state", value: "lowered", expect: 0,
            reply: #{request: 1, selector: "bridge::warden/lowered"}}));
    }
    this.state.ward = ward;
}
fn lowered() {
    if this.trigger.terminal { this.state.ward.log += this.trigger.code + ";"; }
}
fn gone() {
    this.state.ward.gone += 1;
    this.state.ward.log += "gone;";
}

// The bridge room reads its own gate: "S" while it stands, "o" once open.
fn far() {
    for wall in this.observe.lock_walls.walls {
        if wall.id == "bridge_gate" { this.state.far.seen = if wall.standing { "S" } else { "o" }; }
    }
}
