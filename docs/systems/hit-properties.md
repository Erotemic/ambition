# Hit properties

Each authored `HitVolume` can set `properties`. Missing fields give an ordinary
hit. Two volumes in one Active window can use different properties.

| Field | Default | Rule |
|---|---|---|
| `targets` | All three postures | Grounded, airborne, and prone are separate classes. Prone takes precedence over ground contact. A refused posture does not spend the strike contact or catch a parry. A body with no ground component is grounded. |
| `clash` | `Normal` | `Transcendent` passes through attack contests. It can still hit bodies and guards. If a normal sibling volume loses a contest, the whole move ends. |
| `hitlag_scale` | `1.0` | Scales the contact freeze, including a damage-only hit or a hit absorbed by motion armor. Zero gives no new freeze. |
| `sdi_scale` | `1.0` | Scales manual and automatic SDI. The hit with the longest remaining freeze owns this scale. A tie uses the later resolved hit. |
| `tech` | `Normal` | `Deny` prevents tech for this launch. It clears a buffered tech when the launch starts tumble. A later ordinary launch can be teched. |
| `weight` | `Victim` | `Fixed(w)` uses the positive weight `w` for launch growth. Percent, staling, rage, and DI still apply. `Fixed(1.0)` uses the reference body weight. Fixed weight is distinct from fixed knockback (`knockback_growth: Some(0.0)`). |
| `armor` | `Respect` | `Bypass` defeats motion armor. It does not bypass a guard, evasion, invulnerability, or health protection. |

The separate `guard` field uses `Blockable(extra_shield_damage: n)` or
`Unblockable`. The same type serves authored volumes and runtime contacts.

`ProjectileFlight` and `ProjectileSpawn` carry both `properties` and `guard`.
The shared projectile path keeps them on the live shot through reflection and
rollback. Direct contacts use the same body reaction as melee. A landing blast
is a separate attack and keeps its own policy.

Projectile spawn port version 2 exposes these values to scripts. Choice
values use lowercase text, such as `clash: "transcendent"`. Script `weight` is a
positive number, or unit for victim weight. Script `guard` is a shield damage
bonus, or unit for unblockable. The nested `properties` record is optional.

Smash enables clashes with a damage window of 9. Other games keep the engine
default of zero. Grounded melee can clash with grounded melee. Shots can clash
with shots and melee. Two aerial melee attacks do not clash. Resolution uses
stable attack identities before damage. A defeated move uses normal interruption;
a defeated shot uses normal expiry.

Projectile Polygon supplies the first customers. Down-air has an air spike and
a floor pop. The spike denies tech. Forward-air uses a fixed weight and passes
through clashes. Up-air reduces freeze and SDI. Forward smash bypasses motion
armor. The cannon adds shield damage and freeze. Grab in the air selects a long,
thin tether lash with ordinary hit volumes. Standing grabs still capture only
on the ground.
