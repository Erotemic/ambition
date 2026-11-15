--!strict
-- The sentry's turret, in Luau: the native module's algorithm and numbers
-- (game/ambition_content_modules/src/sentry.rs).
local ARM_DELAY_S = 0.25
local FIRE_INTERVAL_S = 0.55
local RANGE = 480.0
local BOLT_SPEED = 430.0
local BOLT_DAMAGE = 2
local BOLT_LIFETIME = 1.4

type Vec2 = { number }
type Request = { port: string, value: { [string]: any } }
type Inv = {
    tick: number,
    dt: number,
    trigger: { position: Vec2, remaining_s: number, nearest_enemy: Vec2? },
    state: { cadence: { armed: boolean, cooldown_s: number } },
    requests: { Request },
}

-- Fire at the nearest enemy in range; the cooldown that follows.
local function fire(inv: Inv, cooldown: number): number
    local at = inv.trigger.position
    local enemy = inv.trigger.nearest_enemy
    if enemy == nil then
        return 0
    end
    local dx = enemy[1] - at[1]
    local dy = enemy[2] - at[2]
    local len = math.sqrt(dx * dx + dy * dy)
    if len > RANGE then
        return 0
    end
    local rcp = 1 / len
    if not (rcp == rcp and rcp ~= math.huge and rcp > 0) then
        return cooldown
    end
    table.insert(inv.requests, {
        port = "projectile",
        value = {
            origin = at, dir = { dx * rcp, dy * rcp }, speed = BOLT_SPEED,
            damage = BOLT_DAMAGE, max_lifetime = BOLT_LIFETIME, half_extent = { 7, 7 },
        },
    })
    table.insert(inv.requests, { port = "sound", value = { cue = "world.rock.hit", at = at } })
    return FIRE_INTERVAL_S
end

-- Each tick of a living turret: count down, and fire when ready.
local function turret(inv: Inv)
    local c = inv.state.cadence
    if not c.armed then
        c.armed = true
        c.cooldown_s = ARM_DELAY_S
    end
    c.cooldown_s -= inv.dt
    if not (c.cooldown_s > 0) then
        c.cooldown_s = fire(inv, c.cooldown_s)
    end
end

return { turret = turret }
