// The Laden Swallow's brain, in Rhai: the native module's rule and numbers
// (src/laden_swallow.rs). The module's declaration stays the game's; this
// file is the code of its `think` entry when a run names it:
//
//     AMBITION_EXTENSION_MODULES=game/ambition_demo_mary_o_modules/rhai/laden_swallow.rhai
//
// It decides in `actor_think`, before the control gate: a coconut straight
// down on its placement's own tick of the period, while it is over its
// target. It keeps no state: the tick and the placement say it all. The
// ports it reads and asks for: docs/sdk/script-ports.md (`actor_think`,
// `fire`).
const DROP_EVERY_TICKS = 150;
const DROP_REACH = 150.0;

fn think() {
    let body = this.trigger;
    if type_of(body.target) == "()" || type_of(body.placement) == "()" {
        return;
    }
    let over = abs(body.position[0] - body.target[0]) <= global::DROP_REACH
        && body.position[1] < body.target[1];
    if over && drops_on(this.tick, body.placement) {
        this.requests.push(request("fire", #{ direction: [0.0, 1.0] }));
    }
}

// Each placement has its own tick of the period, so two birds do not drop
// as one: the sum of its id's bytes.
fn drops_on(tick, placement) {
    let offset = 0;
    for c in placement.chars() {
        offset += c.to_int();
    }
    (tick + offset % global::DROP_EVERY_TICKS) % global::DROP_EVERY_TICKS == 0
}
