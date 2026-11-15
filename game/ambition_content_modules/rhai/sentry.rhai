// The sentry's turret, in Rhai: the native module's algorithm and numbers
// (src/sentry.rs). The module's declaration stays the game's; this file is
// the code of its `turret` entry when a developer run names it:
//
//     AMBITION_EXTENSION_MODULES=game/ambition_content_modules/rhai/sentry.rhai ./run_game.sh --reuse
//
// An edit takes effect while the game runs (docs/recipes/iterating-without-a-rebuild.md).
// `deploy` is not defined here, so it runs natively. The ports a request may
// name, with their fields: docs/sdk/script-ports.md.
const ARM_DELAY_S = 0.25;
const FIRE_INTERVAL_S = 0.55;
const RANGE = 480.0;
const BOLT_SPEED = 430.0;
const BOLT_DAMAGE = 2;
const BOLT_LIFETIME = 1.4;

// Each tick of a living turret: count down, and fire when ready.
fn turret() {
    let c = this.state.cadence;
    if !c.armed {
        c.armed = true;
        c.cooldown_s = global::ARM_DELAY_S;
    }
    c.cooldown_s -= this.dt;
    if !(c.cooldown_s > 0.0) {
        c.cooldown_s = this.fire(c.cooldown_s);
    }
    this.state.cadence = c;
}

// Fire at the nearest enemy in range; the cooldown that follows.
fn fire(cooldown) {
    let at = this.trigger.position;
    let enemy = this.trigger.nearest_enemy;
    if type_of(enemy) == "()" {
        return 0.0;
    }
    let dx = enemy[0] - at[0];
    let dy = enemy[1] - at[1];
    let len = sqrt(dx * dx + dy * dy);
    if len > global::RANGE {
        return 0.0;
    }
    let rcp = 1.0 / len;
    if !(rcp.is_finite() && rcp > 0.0) {
        return cooldown;
    }
    this.requests.push(request("projectile", #{
        origin: at, dir: [dx * rcp, dy * rcp], speed: global::BOLT_SPEED,
        damage: global::BOLT_DAMAGE, max_lifetime: global::BOLT_LIFETIME, half_extent: [7.0, 7.0],
    }));
    this.requests.push(request("sound", #{ cue: "world.rock.hit", at: at }));
    global::FIRE_INTERVAL_S
}
