// A native lever asks a reusable barrier behavior to spend its own object.
const PRICE = 3;

fn declare() {
    #{provider: "ambition", key: "switch_lab", schemas: [
        #{key: "switch_lab.room", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "lever", kind: "opt object"},
            #{tag: 3, name: "ready", kind: "bool"},
            #{tag: 4, name: "left", kind: "f32"},
            #{tag: 5, name: "poll", kind: "u32"},
            #{tag: 6, name: "attachments", kind: "u32"},
        ]},
        #{key: "switch_lab.lever", fields: [
            #{tag: 1, name: "revision", kind: "u64"},
            #{tag: 2, name: "requests", kind: "u32"},
            #{tag: 3, name: "applied", kind: "u32"},
            #{tag: 4, name: "refused", kind: "u32"},
            #{tag: 5, name: "replies", kind: "u32"},
        ]},
        #{key: "switch_lab.barrier", fields: [
            #{tag: 1, name: "busy", kind: "bool"},
            #{tag: 2, name: "pending", kind: "u64"},
            #{tag: 3, name: "requests", kind: "u32"},
            #{tag: 4, name: "replies", kind: "u32"},
        ]},
    ], entries: [
        #{key: "room", phase: "actor_think", trigger: #{port: "story_tick", selector: "switch_lab"},
          writes: ["switch_lab.room"], requests: ["resolve_authored_objects", "call_object"], limit: 1},
        #{key: "located", phase: "actor_think", trigger: #{port: "action_result", selector: "located"},
          writes: ["switch_lab.room"], requests: ["attach_behaviors"], limit: 2},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["switch_lab.room"]},
        #{key: "activated", phase: "actor_think", trigger: #{port: "object_activated", selector: "activated"},
          reads: ["behavior", "object_activation"], writes: ["switch_lab.lever"], requests: ["call_object"], limit: 1},
        #{key: "reconcile", phase: "actor_think", trigger: #{port: "object_method", selector: "reconcile"},
          reads: ["behavior", "object_activation"], writes: ["switch_lab.lever"], requests: ["call_object", "complete_object_call"], limit: 2},
        #{key: "opened", phase: "actor_think", trigger: #{port: "action_result", selector: "opened"}, writes: ["switch_lab.lever"]},
        #{key: "open", phase: "actor_think", trigger: #{port: "object_method", selector: "open"},
          reads: ["behavior", "destructible"], writes: ["switch_lab.barrier"], requests: ["consume_self", "complete_object_call"], limit: 1},
        #{key: "spent", phase: "actor_think", trigger: #{port: "action_result", selector: "spent"},
          writes: ["switch_lab.barrier"], requests: ["complete_object_call"], limit: 1},
        #{key: "ack", phase: "actor_think", trigger: #{port: "action_result", selector: "ack"}},
    ]}
}
fn room() {
    if !this.state.room.started {
        this.state.room.started = true;
        this.requests.push(request("resolve_authored_objects", #{placements: ["switch_lab_toll", "switch_lab_barrier"],
            reply: #{request: 0, selector: "ambition::switch_lab/located"}}));
        return;
    }
    if !this.state.room.ready { return; }
    this.state.room.left -= this.dt;
    if this.state.room.left > 0.0 { return; }
    this.state.room.left = 0.5;
    this.state.room.poll += 1;
    this.requests.push(request("call_object", #{target: this.state.room.lever, binding: "lever", method: "reconcile",
        reply: #{request: this.state.room.poll, selector: "ambition::switch_lab/ack"}}));
}
fn located() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.code; }
    this.state.room.lever = this.trigger.subjects[0].object;
    this.requests.push(request("attach_behaviors", #{target: this.trigger.subjects[1].object,
        reply: #{request: 0, selector: "ambition::switch_lab/attached"}, bindings: [#{key: "barrier", params: #{price: global::PRICE},
            methods: [#{name: "open", entry: "ambition::switch_lab/open"}]}]}));
    this.requests.push(request("attach_behaviors", #{target: this.state.room.lever,
        reply: #{request: 1, selector: "ambition::switch_lab/attached"}, bindings: [#{key: "lever",
            references: #{source: this.state.room.lever, barrier: this.trigger.subjects[1].object},
            methods: [#{name: "activated", entry: "ambition::switch_lab/activated"}, #{name: "reconcile", entry: "ambition::switch_lab/reconcile"}],
            subscriptions: [#{key: "press", source: "source", method: "activated"}]}]}));
}
fn attached() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.code; }
    this.state.room.attachments += 1;
    this.state.room.ready = this.state.room.attachments == 2;
}
fn activation_request(activation, revision, barrier, number) {
    // Revision values are exact decimal text. Compare equality.
    if activation == () || activation.revision == revision || activation.actor == () { return (); }
    request("call_object", #{target: barrier, binding: "barrier", method: "open", data: #{actor: activation.actor},
        reply: #{request: number, selector: "ambition::switch_lab/opened"}})
}
fn activated() {
    let next = activation_request(this.observe.object_activation, this.state.lever.revision,
        this.observe.behavior.references.barrier, this.state.lever.requests + 1);
    if next == () { return; }
    this.state.lever.revision = this.observe.object_activation.revision;
    this.state.lever.requests += 1;
    this.requests.push(next);
}
fn reconcile() {
    let next = activation_request(this.observe.object_activation, this.state.lever.revision,
        this.observe.behavior.references.barrier, this.state.lever.requests + 1);
    if next != () {
        this.state.lever.revision = this.observe.object_activation.revision;
        this.state.lever.requests += 1;
        this.requests.push(next);
    }
    this.state.lever.replies += 1;
    this.requests.push(request("complete_object_call", #{sequence: this.trigger.sequence, code: "reconciled",
        reply: #{request: this.state.lever.replies, selector: "ambition::switch_lab/ack"}}));
}
fn opened() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage == "completed" { this.state.lever.applied += 1; }
    else { this.state.lever.refused += 1; }
}
fn open() {
    this.state.barrier.replies += 1;
    if this.state.barrier.busy || this.observe.destructible == () {
        this.requests.push(request("complete_object_call", #{sequence: this.trigger.sequence, accepted: false, code: "barrier_busy",
            reply: #{request: this.state.barrier.replies, selector: "ambition::switch_lab/ack"}}));
        return;
    }
    this.state.barrier.busy = true;
    this.state.barrier.pending = this.trigger.sequence;
    this.state.barrier.requests += 1;
    this.requests.push(request("consume_self", #{recipient: this.trigger.data.actor,
        exchange: #{pay_coins: this.observe.behavior.params.price},
        reply: #{request: this.state.barrier.requests, selector: "ambition::switch_lab/spent"}}));
}
fn spent() {
    if !this.trigger.terminal { return; }
    this.state.barrier.busy = false;
    this.state.barrier.replies += 1;
    this.requests.push(request("complete_object_call", #{sequence: this.state.barrier.pending,
        accepted: this.trigger.stage == "applied", code: this.trigger.code,
        reply: #{request: this.state.barrier.replies, selector: "ambition::switch_lab/ack"}}));
}
fn ack() { if this.trigger.stage == "rejected" || this.trigger.stage == "failed" { throw this.trigger.code; } }
