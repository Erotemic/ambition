//! Optional movement/combat capabilities.
//!
//! Ambition is expected to have many upgrades, and the endgame sandbox should
//! usually run with everything enabled. The engine still needs the opposite:
//! small, explicit capability sets that can be tested in isolation. This file
//! is the vocabulary for that.
//!
//! The important rule is that an ability flag should answer "may this verb be
//! used at all?" Tuning values such as speed, duration, and charge counts live
//! in `MovementTuning`, while this module decides which groups of verbs exist.

use serde::{Deserialize, Serialize};

/// A set of optional player capabilities.
///
/// This is intentionally a plain data struct. Later we can load it from RON,
/// JSON, a save file, an AI-generated spec, or an in-game upgrade graph without
/// changing the movement simulation API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilitySet {
    /// Horizontal ground/air steering. Disabling this is mostly useful for
    /// tests or scripted story moments.
    pub move_horizontal: bool,
    /// Basic jump from ground/coyote time.
    pub jump: bool,
    /// Early jump release shortens the active jump arc according to the selected jump law.
    pub variable_jump: bool,
    /// One extra air jump in the current tuning pass.
    pub double_jump: bool,
    /// Double-tap down while airborne starts fast-fall. Holding down alone does
    /// not fast-fall, so down+attack/pogo can remain a natural input.
    pub fast_fall: bool,
    /// Jumping from a wall contact.
    pub wall_jump: bool,
    /// Slow or stop wall sliding while pressing into a wall.
    pub wall_cling: bool,
    /// Climb upward/downward while clinging to a wall.
    pub wall_climb: bool,
    /// Aerial/ground dash.
    pub dash: bool,
    /// Upgrade that gives two dash charges before refresh.
    pub double_dash: bool,
    /// Free-flight movement capability. When the body is in flight mode,
    /// movement input applies acceleration toward a terminal velocity instead
    /// of normal ground/air platformer steering.
    pub fly: bool,
    /// Whether the controlled body may toggle flight mode at runtime.
    ///
    /// Floating bodies such as TwinTrack's spacecraft can author permanent
    /// free flight (`fly = true`, `fly_toggle = false`) so the action scheme
    /// does not advertise a meaningless mode switch. Existing authored data
    /// defaults this to true to preserve the historical fly-button behavior.
    #[serde(default = "default_fly_toggle")]
    pub fly_toggle: bool,
    /// Short-range teleport. Quick release blinks immediately along input/facing.
    pub blink: bool,
    /// Upgrade for blink: holding the blink button enters aim/bullet-time mode
    /// and releases to blink to a more deliberate destination.
    pub precision_blink: bool,
    /// Allow blink pathing through soft blink gates. The destination must still
    /// be open space; this only permits crossing selected wall volumes.
    pub blink_through_soft_walls: bool,
    /// Allow blink pathing through hard blink gates. This is intentionally a
    /// separate future upgrade so some walls can remain meaningful blockers.
    pub blink_through_hard_walls: bool,
    /// Generic slash/attack verb.
    pub attack: bool,
    /// Downward attack/pogo refresh verb.
    pub pogo: bool,
    /// Direction + primary attack can eventually produce distinct attacks.
    /// The first implementation still shares the same hitbox helper.
    pub directional_primary: bool,
    /// Direction + special/secondary can eventually produce distinct specials.
    /// Blink is the first concrete special in this category.
    pub directional_special: bool,
    /// Allow special world surfaces to apply an impulse.
    pub rebound: bool,
    /// Debug/sandbox reset. In the final game this may become a menu/system
    /// action rather than a player ability.
    pub reset: bool,
    /// Snap onto ledges while wall-sliding and pull-up to the platform
    /// above. Gated as a separate ability so the early game can ship
    /// without it and a mid-game upgrade or piece of gear can light it
    /// up. Movement integration reads `Player::abilities.ledge_grab`
    /// before running the snap probe, so disabling this turns the
    /// mechanic off entirely.
    #[serde(default)]
    pub ledge_grab: bool,
    /// Active swim controls inside any `WaterRegion`: jump becomes a
    /// swim impulse, the player can rise with repeated presses, and
    /// surface exit is allowed. Without this flag the player drowns
    /// on water contact (movement triggers a respawn). Source of the
    /// region — IntGrid `Water` cells or entity `WaterVolume` — is
    /// abstracted by `World::water_at`.
    #[serde(default)]
    pub swim: bool,
    /// Glide / cape / slow-fall: holding the jump button while
    /// airborne and falling caps the fall speed at
    /// `MovementTuning::glide_fall_speed` instead of `max_fall_speed`.
    /// Cancels on ground / dash / blink / jump release. Cheap held
    /// ability that pairs well with `wall_jump` and `double_jump` for
    /// long-distance platforming. No resource cost in the v1 — that
    /// can land later as a `hover_fuel` `ResourceMeter` tap.
    #[serde(default)]
    pub glide: bool,
    /// Ground dodge roll: pressing dash while grounded triggers a short
    /// lateral roll with invulnerability frames. Uses its own cooldown
    /// separate from air-dash charges so it does not compete with aerial
    /// movement options. When enabled, ground-dash input is consumed by
    /// the dodge roll first; air dashes still consume charges as normal.
    #[serde(default)]
    pub dodge: bool,
    /// Bubble shield: holding the shield button deploys a protective
    /// bubble. The first [`crate::PARRY_WINDOW_TIME`] seconds grant
    /// full invulnerability (parry window). After that the shield
    /// remains visible but grants no damage reduction in the current
    /// implementation — the hook exists for future parry mechanics
    /// (projectile reflection, counter stun).
    #[serde(default)]
    pub shield: bool,
    /// **World interaction: talk, open, read.** Resolved against nearby
    /// interactables at press time, so this flag is only "does this body have the
    /// verb at all" — the world half was always handled downstream.
    ///
    /// ⛔ **it used to be UNCONDITIONAL.** `derive_action_scheme` upserted an
    /// Interact action for every controllable body, with the comment *"Interact
    /// is available to every controllable subject"* — an assumption about the
    /// GAME rather than about the body, so a game with nothing to interact with
    /// put a button on screen that never did anything. Jon, from a phone: *"maryo
    /// has more than 2 on screen buttons … that shouldn't be the case for her."*
    ///
    /// ⚠ **absent from [`Self::NONE`] on purpose, which is what makes a
    /// restricted kit restrictive.** `compose` folds grants from `NONE`, so a
    /// character authoring a grant list gets exactly what it asked for — and
    /// [`AbilityGrant::RunJump`], "the minimal kit a platformer protagonist
    /// needs", carries no talk verb, faithfully to the game it is named after.
    /// `basic` / `sane_subset` / `sandbox_all` all keep it.
    ///
    /// ⚠ **`serde` default is TRUE, not `bool::default()`.** Authored data that
    /// predates this field (the shipped `platformer_defaults.ron`, a save, an
    /// authored spec) must keep the verb it had — a missing field meaning "no
    /// interact" would silently take talking away from every body loaded from
    /// data, which is the opposite of the conservative reading and would not
    /// fail loudly anywhere.
    #[serde(default = "yes")]
    pub interact: bool,
}

/// **Can this body STAY WHERE IT IS, without being carried off?**
///
/// The authority Jon's dialogue-continuity design needs
/// (`docs/planning/engine/dialogue-continuity.md`): a conversation asks its
/// participants to hold a conversational stance, they comply if they can, and
/// the ones that cannot are carried away by ordinary physics — at which point
/// the conversation breaks.
///
/// ⭐ **derived, not a new flag.** Holding station is not a capability anybody
/// authors; it is what being grounded OR being able to fly already means. A body
/// standing on a floor holds station by standing still, and a body that can fly
/// holds station by hovering. Adding a `can_hover` bool beside `fly` would be a
/// second authority for one fact, and content would eventually set them
/// disagreeing.
///
/// ⚠ **symmetric on purpose.** Jon: *"if both character are capable of flying
/// and hoverying and you stop to talk, then both characters should hover so they
/// can have the dialog."* This takes a body's own facts and knows nothing about
/// players — the flying parrot and the flying player answer it the same way, and
/// a caller that asks it of only one of them has reintroduced the
/// player-centrism the design is written against.
pub fn can_hold_station(abilities: &AbilitySet, grounded: bool) -> bool {
    grounded || abilities.fly
}

impl AbilitySet {
    /// Minimal movement for a first-room player.
    pub const fn basic() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: false,
            fast_fall: false,
            wall_jump: false,
            wall_cling: false,
            wall_climb: false,
            dash: false,
            double_dash: false,
            fly: false,
            fly_toggle: false,
            blink: false,
            precision_blink: false,
            blink_through_soft_walls: false,
            blink_through_hard_walls: false,
            attack: false,
            pogo: false,
            directional_primary: false,
            directional_special: false,
            rebound: false,
            reset: true,
            ledge_grab: false,
            swim: false,
            glide: false,
            dodge: false,
            shield: false,
            interact: true,
        }
    }

    /// Endgame sandbox defaults: every currently implemented verb is enabled.
    pub const fn sandbox_all() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            wall_jump: true,
            wall_cling: true,
            wall_climb: true,
            dash: true,
            double_dash: true,
            fly: true,
            fly_toggle: true,
            blink: true,
            precision_blink: true,
            blink_through_soft_walls: true,
            blink_through_hard_walls: true,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
            ledge_grab: true,
            swim: true,
            glide: true,
            dodge: true,
            shield: true,
            interact: true,
        }
    }

    /// A deliberately sane initial endgame subset.
    ///
    /// This is a smaller list than "all platformer abilities ever", but it is
    /// broad enough to exercise movement, wall routing, combat, and one special
    /// teleport verb.  The sandbox currently uses `sandbox_all`; tests and later
    /// story states can use this as a balanced default.
    pub const fn sane_subset() -> Self {
        Self {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            wall_jump: true,
            wall_cling: true,
            wall_climb: true,
            dash: true,
            double_dash: true,
            fly: true,
            fly_toggle: true,
            blink: true,
            precision_blink: true,
            blink_through_soft_walls: true,
            blink_through_hard_walls: false,
            attack: true,
            pogo: true,
            directional_primary: true,
            directional_special: true,
            rebound: true,
            reset: true,
            // ledge grab + swim + glide + dodge + shield are mid-game upgrades;
            // not part of the "sane subset" early-game baseline.
            ledge_grab: false,
            swim: false,
            glide: false,
            dodge: false,
            shield: false,
            interact: true,
        }
    }

    /// The empty set: every verb denied. Identity element for [`union`].
    ///
    /// Composition of a character's grant bundles is a fold of `union` starting
    /// here, so a character with no grants can do nothing until something is
    /// composed onto it.
    ///
    /// [`union`]: AbilitySet::union
    pub const NONE: Self = Self {
        move_horizontal: false,
        jump: false,
        variable_jump: false,
        double_jump: false,
        fast_fall: false,
        wall_jump: false,
        wall_cling: false,
        wall_climb: false,
        dash: false,
        double_dash: false,
        fly: false,
        fly_toggle: false,
        blink: false,
        precision_blink: false,
        blink_through_soft_walls: false,
        blink_through_hard_walls: false,
        attack: false,
        pogo: false,
        directional_primary: false,
        directional_special: false,
        rebound: false,
        reset: false,
        ledge_grab: false,
        swim: false,
        glide: false,
        dodge: false,
        shield: false,
        interact: false,
    };

    /// Field-wise OR: a verb is granted if *either* set grants it.
    ///
    /// This is the composition operator. A character is not a frozen preset; it
    /// is the `union` of the grant bundles it carries (base kit ∪ gear ∪
    /// upgrades). `union` is commutative, associative, and idempotent, with
    /// [`NONE`] as its identity, so grant order never matters.
    ///
    /// [`NONE`]: AbilitySet::NONE
    pub const fn union(self, other: Self) -> Self {
        Self {
            move_horizontal: self.move_horizontal || other.move_horizontal,
            jump: self.jump || other.jump,
            variable_jump: self.variable_jump || other.variable_jump,
            double_jump: self.double_jump || other.double_jump,
            fast_fall: self.fast_fall || other.fast_fall,
            wall_jump: self.wall_jump || other.wall_jump,
            wall_cling: self.wall_cling || other.wall_cling,
            wall_climb: self.wall_climb || other.wall_climb,
            dash: self.dash || other.dash,
            double_dash: self.double_dash || other.double_dash,
            fly: self.fly || other.fly,
            fly_toggle: self.fly_toggle || other.fly_toggle,
            blink: self.blink || other.blink,
            precision_blink: self.precision_blink || other.precision_blink,
            blink_through_soft_walls: self.blink_through_soft_walls
                || other.blink_through_soft_walls,
            blink_through_hard_walls: self.blink_through_hard_walls
                || other.blink_through_hard_walls,
            attack: self.attack || other.attack,
            pogo: self.pogo || other.pogo,
            directional_primary: self.directional_primary || other.directional_primary,
            directional_special: self.directional_special || other.directional_special,
            rebound: self.rebound || other.rebound,
            reset: self.reset || other.reset,
            ledge_grab: self.ledge_grab || other.ledge_grab,
            swim: self.swim || other.swim,
            glide: self.glide || other.glide,
            dodge: self.dodge || other.dodge,
            shield: self.shield || other.shield,
            interact: self.interact || other.interact,
        }
    }

    /// Field-wise AND with a mask: a verb survives only if *both* grant it.
    ///
    /// A mask can only ever REMOVE grants, never add them, so it is the safe
    /// operator for a session-level capability restriction (a story moment that
    /// forbids a verb, a dev toggle that gates one off) layered over whatever a
    /// character composed. `intersect` with a mask of [`sandbox_all`] is the
    /// identity.
    ///
    /// [`sandbox_all`]: AbilitySet::sandbox_all
    pub const fn intersect(self, mask: Self) -> Self {
        Self {
            move_horizontal: self.move_horizontal && mask.move_horizontal,
            jump: self.jump && mask.jump,
            variable_jump: self.variable_jump && mask.variable_jump,
            double_jump: self.double_jump && mask.double_jump,
            fast_fall: self.fast_fall && mask.fast_fall,
            wall_jump: self.wall_jump && mask.wall_jump,
            wall_cling: self.wall_cling && mask.wall_cling,
            wall_climb: self.wall_climb && mask.wall_climb,
            dash: self.dash && mask.dash,
            double_dash: self.double_dash && mask.double_dash,
            fly: self.fly && mask.fly,
            fly_toggle: self.fly_toggle && mask.fly_toggle,
            blink: self.blink && mask.blink,
            precision_blink: self.precision_blink && mask.precision_blink,
            blink_through_soft_walls: self.blink_through_soft_walls
                && mask.blink_through_soft_walls,
            blink_through_hard_walls: self.blink_through_hard_walls
                && mask.blink_through_hard_walls,
            attack: self.attack && mask.attack,
            pogo: self.pogo && mask.pogo,
            directional_primary: self.directional_primary && mask.directional_primary,
            directional_special: self.directional_special && mask.directional_special,
            rebound: self.rebound && mask.rebound,
            reset: self.reset && mask.reset,
            ledge_grab: self.ledge_grab && mask.ledge_grab,
            swim: self.swim && mask.swim,
            glide: self.glide && mask.glide,
            dodge: self.dodge && mask.dodge,
            shield: self.shield && mask.shield,
            interact: self.interact && mask.interact,
        }
    }

    /// Compose a slice of grant bundles into one effective set.
    ///
    /// This is how a character is defined: not by picking a preset, but by the
    /// `union` of the bundles it lists. An empty slice composes to [`NONE`].
    pub fn compose(grants: &[AbilityGrant]) -> Self {
        grants
            .iter()
            .fold(Self::NONE, |acc, grant| acc.union(grant.to_set()))
    }

    /// Number of air jumps granted by the active ability set.
    pub const fn air_jump_count(self, tuning_air_jumps: u8) -> u8 {
        if self.double_jump {
            tuning_air_jumps
        } else {
            0
        }
    }

    /// Number of dash charges granted by the active ability set.
    pub const fn dash_charge_count(self) -> u8 {
        if !self.dash {
            0
        } else if self.double_dash {
            2
        } else {
            1
        }
    }

    /// Human-readable compatibility warnings.
    ///
    /// These are warnings, not hard errors. Some story/gameplay moments may
    /// intentionally enable a dependent ability without its normal prerequisite.
    pub fn compatibility_warnings(self) -> Vec<&'static str> {
        let mut warnings = Vec::new();
        if self.double_jump && !self.jump {
            warnings.push("double_jump is enabled but jump is disabled");
        }
        if self.wall_jump && !self.jump {
            warnings.push("wall_jump is enabled but jump is disabled");
        }
        if self.wall_climb && !self.wall_cling {
            warnings.push("wall_climb is enabled but wall_cling is disabled");
        }
        if self.double_dash && !self.dash {
            warnings.push("double_dash is enabled but dash is disabled");
        }
        if self.fly && !self.move_horizontal {
            warnings.push("fly is enabled but move_horizontal is disabled");
        }
        if self.fly_toggle && !self.fly {
            warnings.push("fly_toggle is enabled but fly is disabled");
        }
        if self.precision_blink && !self.blink {
            warnings.push("precision_blink is enabled but blink is disabled");
        }
        if self.blink_through_soft_walls && !self.blink {
            warnings.push("blink_through_soft_walls is enabled but blink is disabled");
        }
        if self.blink_through_hard_walls && !self.blink_through_soft_walls {
            warnings.push("blink_through_hard_walls is enabled without blink_through_soft_walls");
        }
        if self.directional_special && !self.blink {
            warnings
                .push("directional_special currently has no concrete verb unless blink is enabled");
        }
        if self.pogo && !self.attack {
            warnings.push("pogo is enabled but attack is disabled");
        }
        if self.glide && !self.jump {
            warnings.push("glide is enabled but jump is disabled (the trigger is hold-jump)");
        }
        warnings
    }
}

/// `serde` default for [`AbilitySet::fly_toggle`]. Historical authored bodies
/// that granted flight also exposed the toggle; the action scheme additionally
/// requires `fly`, so old non-flying bodies do not gain a phantom button.
fn default_fly_toggle() -> bool {
    true
}

/// `serde` default for [`AbilitySet::interact`]: data that does not mention the
/// field describes a body from before it existed, and that body could interact.
fn yes() -> bool {
    true
}

impl Default for AbilitySet {
    fn default() -> Self {
        Self::basic()
    }
}

/// A named, composable bundle of grants.
///
/// A character is not defined by picking ONE preset; it is defined by the set
/// of grant bundles it carries, composed with [`AbilitySet::union`]. This is the
/// authoring vocabulary: a catalog row lists its grants, and
/// [`AbilitySet::compose`] folds them into the character's base capability set.
///
/// The vocabulary starts at exactly the bundles that have a consumer today.
/// Finer-grained single-verb grants (a `WallMovement`, an `AirJumps`, a `Dash`)
/// are added HERE as the systems behind them land — a character then composes
/// them into its list instead of a preset gaining a new variant. That is the
/// whole point of composition over presets: new verbs never fork the roster.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AbilityGrant {
    /// The classic run-and-jump floor: horizontal steering, a ground jump, and
    /// variable jump height. The minimal kit a platformer protagonist needs.
    RunJump,
    /// An extra jump in the air. On its own this is a *double* jump (the
    /// [`air_jumps`](AxisSweptParams) tuning default is 1); a character that
    /// authors a higher `air_jumps` count turns the same grant into a triple
    /// jump without a new verb — the count is feel (tuning), the *capability*
    /// is this flag.
    AirJump,
    /// Wall mobility: cling to (and slide down) a wall, and kick off it. The
    /// pair a "run up a chimney" platformer move needs; deliberately excludes
    /// `wall_climb` (climbing a wall is a different, ladder-like verb).
    WallMobility,
    /// Fast fall: press down in the air to dive faster. The gesture that a
    /// ground-pound-style down-slam is built on.
    FastFall,
    /// The curated mid-game subset ([`AbilitySet::sane_subset`]).
    SaneSubset,
    /// Permanent gravity-free 2D steering with interaction, but no jump or
    /// flight-toggle button. Intended for spacecraft, swimmers in a dedicated
    /// free-movement game, and other bodies whose base locomotion is flight.
    FreeFlight,
    /// Every implemented verb ([`AbilitySet::sandbox_all`]).
    SandboxAll,
}

impl AbilityGrant {
    /// The concrete verbs this bundle grants.
    pub fn to_set(self) -> AbilitySet {
        match self {
            Self::RunJump => AbilitySet {
                move_horizontal: true,
                jump: true,
                variable_jump: true,
                ..AbilitySet::NONE
            },
            Self::AirJump => AbilitySet {
                double_jump: true,
                ..AbilitySet::NONE
            },
            Self::WallMobility => AbilitySet {
                wall_jump: true,
                wall_cling: true,
                ..AbilitySet::NONE
            },
            Self::FastFall => AbilitySet {
                fast_fall: true,
                ..AbilitySet::NONE
            },
            Self::FreeFlight => AbilitySet {
                move_horizontal: true,
                fly: true,
                fly_toggle: false,
                interact: true,
                ..AbilitySet::NONE
            },
            Self::SaneSubset => AbilitySet::sane_subset(),
            Self::SandboxAll => AbilitySet::sandbox_all(),
        }
    }
}

/// **WHAT A MATCH SAYS ITS FIGHTERS MAY DO**, as two statements rather than one.
///
/// A mode has two different things to say about a body's verbs, and the day they
/// were one field only one of them could be true at a time:
///
/// ```text
///   granted    every fighter HAS these, whatever its character authored
///   permitted  and no fighter has anything OUTSIDE these
/// ```
///
/// ⇒ `effective = (authored ∪ granted) ∩ permitted`, which is
/// [`Self::apply`] and is the whole rule.
///
/// ⛔⛔ **A MASK ALONE COULD NOT GUARANTEE A FLOOR, and that is the defect this
/// type exists for** (Jon, 2026-08-16: *"in smash all characters should be sure
/// they are granted the basic smash abilities"*). While a match declared one set
/// and INTERSECTED it, a character that authored its own kit could only ever
/// have FEWER verbs than the mode named — so the Perfect Cellular Automaton,
/// whose kit was written for a duel arena on [`AbilitySet::basic`], arrived on a
/// platform-fighter stage with no double jump, no fast fall, no dodge and no
/// ledge grab, and the stage had no way to say otherwise. Every character that
/// gains an authored kit is one more chance at that, and the count of those is
/// meant to GROW.
///
/// ⛔ **and a grant alone is not the answer either.** A mode that simply stamped
/// its set over every body manufactures capabilities the body never had — the
/// Puppy Slug jumping and dashing like a humanoid, which is why the mask
/// replaced the grant in the first place. It also hands back verbs a character
/// deliberately REFUSED: the robot lineage states *"`reset` stays out … authoring
/// it would hand every game that seats the robot a way to teleport home"*, and a
/// mode whose set happened to include `reset` would undo that.
///
/// Both statements are needed because they answer different questions. A stage
/// says which it means with [`Self::levelled`] or [`Self::at_most`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchAbilities {
    /// **Every fighter has these**, whatever its character authored. The floor a
    /// mode guarantees — for a platform fighter, the verbs without which a body
    /// is not playable on the stage at all.
    pub granted: AbilitySet,
    /// **And nothing outside these.** The ceiling a mode permits — for a
    /// platform fighter, the reason an exploration protagonist's flight and
    /// blink do not come to the fight.
    pub permitted: AbilitySet,
}

impl MatchAbilities {
    /// **One kit, for everybody.** `granted == permitted`, so every fighter in
    /// the match has precisely this set and a character's own kit changes
    /// nothing — which is what a levelled versus stage means and says.
    ///
    /// ⚠ the day a stage wants a fighter's own flavour to survive (a wall jump
    /// on the characters that have one), it widens `permitted` past `granted`
    /// rather than reaching for a third operator.
    pub const fn levelled(kit: AbilitySet) -> Self {
        Self {
            granted: kit,
            permitted: kit,
        }
    }

    /// **A ceiling only** — grant nothing, permit `kit`. The behaviour a lone
    /// mask had: a character keeps what it authored, minus anything this mode
    /// forbids.
    pub const fn at_most(kit: AbilitySet) -> Self {
        Self {
            granted: AbilitySet::NONE,
            permitted: kit,
        }
    }

    /// The verbs a body seated under these rules actually has.
    ///
    /// ⚠ **`None` means the character stated nothing, and it takes the mode's
    /// ceiling** — the migration bridge, expressed as the DEFAULT rather than as
    /// a branch. Almost every character in the repo authors no verbs, so
    /// removing this would strip a crossover cast down to whatever construction
    /// happened to build. It disappears one character at a time, and the day it
    /// is unreachable this line is `unwrap_or(AbilitySet::NONE)`.
    pub fn apply(self, authored: Option<AbilitySet>) -> AbilitySet {
        authored
            .unwrap_or(self.permitted)
            .union(self.granted)
            .intersect(self.permitted)
    }

    /// **Is the declaration coherent — is everything GRANTED also PERMITTED?**
    ///
    /// A mode that guarantees a verb it also forbids is a contradiction nothing
    /// downstream can act on: [`Self::apply`] intersects last, so the verb is
    /// silently dropped and the stage seats bodies that cannot do the thing it
    /// promised. One equality rather than a named list of offenders, because the
    /// answer a caller needs is *"is this declaration sound"* and a stage's own
    /// test is where the reading happens.
    pub fn is_coherent(self) -> bool {
        self.granted.union(self.permitted) == self.permitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A kit with a hole in it, and a kit with something extra: the two shapes
    /// a character can disagree with a mode about.
    fn crawler() -> AbilitySet {
        AbilitySet {
            move_horizontal: true,
            attack: true,
            ..AbilitySet::NONE
        }
    }

    fn fighter_kit() -> AbilitySet {
        AbilitySet {
            move_horizontal: true,
            jump: true,
            double_jump: true,
            attack: true,
            ..AbilitySet::NONE
        }
    }

    /// **A LEVELLING GRANTS WHAT A CHARACTER LACKS AND FORBIDS WHAT IT ADDS.**
    /// Both halves in one assertion, because a `granted` that worked while
    /// `permitted` did nothing would pass half of this.
    #[test]
    fn a_levelling_grants_the_missing_and_removes_the_extra() {
        let rules = MatchAbilities::levelled(fighter_kit());

        // The character is short a jump. The mode guarantees one.
        let short = rules.apply(Some(crawler()));
        assert!(
            short.jump && short.double_jump,
            "a levelling match left a fighter without the jump it declares"
        );

        // The character brings flight. The mode does not permit it.
        let extra = rules.apply(Some(AbilitySet {
            fly: true,
            ..fighter_kit()
        }));
        assert!(
            !extra.fly,
            "a levelling match let a character smuggle in a verb outside its kit"
        );
        assert_eq!(
            extra,
            fighter_kit(),
            "every fighter under a levelling has the SAME kit, whatever it authored"
        );
    }

    /// **A CEILING CAN ONLY EVER TAKE AWAY** — the behaviour a lone mask had,
    /// and the one the versus stage still wants.
    #[test]
    fn a_ceiling_only_rule_never_manufactures_a_verb() {
        let rules = MatchAbilities::at_most(fighter_kit());
        let seated = rules.apply(Some(crawler()));
        assert!(
            !seated.jump && !seated.double_jump,
            "a ceiling handed a crawler a jump it never authored"
        );
        assert!(
            seated.attack && seated.move_horizontal,
            "the seat received nothing at all, so the line above proves nothing"
        );
    }

    /// **AN UNAUTHORED CHARACTER TAKES THE CEILING** — the migration bridge,
    /// and the reason `apply` defaults rather than branches. Almost every
    /// character in the repo authors no verbs.
    #[test]
    fn a_character_that_authors_nothing_takes_what_the_mode_permits() {
        assert_eq!(
            MatchAbilities::at_most(fighter_kit()).apply(None),
            fighter_kit()
        );
        assert_eq!(
            MatchAbilities::levelled(fighter_kit()).apply(None),
            fighter_kit()
        );
    }

    /// **A VERB GRANTED BUT NOT PERMITTED IS A CONTRADICTION**, and `apply`
    /// resolves it silently in favour of the ceiling — so the declaration has to
    /// be checkable before anybody seats a body under it.
    #[test]
    fn a_grant_outside_the_ceiling_is_reported_as_incoherent() {
        assert!(MatchAbilities::levelled(fighter_kit()).is_coherent());
        assert!(MatchAbilities::at_most(fighter_kit()).is_coherent());
        let contradictory = MatchAbilities {
            granted: fighter_kit(),
            permitted: crawler(),
        };
        assert!(!contradictory.is_coherent());
        assert!(
            !contradictory.apply(Some(fighter_kit())).jump,
            "the contradiction resolves toward the ceiling, which is why it has \
             to be reported rather than reasoned about at the seat"
        );
    }

    #[test]
    fn sandbox_all_has_no_compatibility_warnings() {
        assert!(AbilitySet::sandbox_all()
            .compatibility_warnings()
            .is_empty());
    }

    #[test]
    fn dependent_abilities_report_warnings() {
        let mut abilities = AbilitySet::basic();
        abilities.double_dash = true;
        abilities.wall_climb = true;
        abilities.precision_blink = true;
        abilities.blink_through_soft_walls = true;
        let warnings = abilities.compatibility_warnings();
        assert!(warnings.iter().any(|w| w.contains("double_dash")));
        assert!(warnings.iter().any(|w| w.contains("wall_climb")));
        assert!(warnings.iter().any(|w| w.contains("precision_blink")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("blink_through_soft_walls")));
    }

    #[test]
    fn glide_without_jump_warns() {
        let mut abilities = AbilitySet::basic();
        abilities.glide = true;
        abilities.jump = false;
        let warnings = abilities.compatibility_warnings();
        assert!(warnings.iter().any(|w| w.contains("glide")));
    }

    #[test]
    fn dash_charge_count_respects_double_dash() {
        let mut abilities = AbilitySet::basic();
        abilities.dash = true;
        assert_eq!(abilities.dash_charge_count(), 1);
        abilities.double_dash = true;
        assert_eq!(abilities.dash_charge_count(), 2);
        abilities.dash = false;
        assert_eq!(abilities.dash_charge_count(), 0);
    }

    #[test]
    fn air_jump_count_zero_without_double_jump() {
        let mut abilities = AbilitySet::basic();
        assert_eq!(abilities.air_jump_count(2), 0);
        abilities.double_jump = true;
        assert_eq!(abilities.air_jump_count(2), 2);
    }

    #[test]
    fn sane_subset_passes_compatibility() {
        // Same contract as sandbox_all: no warnings on a curated set.
        assert!(AbilitySet::sane_subset()
            .compatibility_warnings()
            .is_empty());
    }

    #[test]
    fn none_is_the_union_identity() {
        // Composing NONE with anything leaves it untouched, in both orders.
        let s = AbilitySet::sane_subset();
        assert_eq!(s.union(AbilitySet::NONE), s);
        assert_eq!(AbilitySet::NONE.union(s), s);
        // And NONE truly grants nothing.
        assert_eq!(AbilitySet::NONE, AbilitySet::compose(&[]));
    }

    #[test]
    fn union_adds_verbs_and_never_removes() {
        // run_jump has no dash; unioning a dash-bearing set must ADD dash while
        // keeping run_jump's own verbs — a grant can only turn things ON.
        let run_jump = AbilityGrant::RunJump.to_set();
        assert!(!run_jump.dash);
        let composed = run_jump.union(AbilitySet::sandbox_all());
        assert!(
            composed.dash,
            "union must add the verb the other set grants"
        );
        assert!(composed.jump, "union must keep run_jump's own jump");
        // Poison: union with NONE must not silently drop a verb.
        assert!(run_jump.union(AbilitySet::NONE).jump);
        // sandbox_all ∪ anything is still sandbox_all (idempotent at the top).
        assert_eq!(
            AbilitySet::sandbox_all().union(run_jump),
            AbilitySet::sandbox_all()
        );
    }

    #[test]
    fn intersect_only_removes_never_adds() {
        // A mask can gate a verb OFF but can never grant one the base lacks.
        let base = AbilityGrant::RunJump.to_set();
        // Masking by sandbox_all (the permissive default) is the identity.
        assert_eq!(base.intersect(AbilitySet::sandbox_all()), base);
        // A restrictive mask removes jump.
        let no_jump_mask = AbilitySet {
            jump: false,
            ..AbilitySet::sandbox_all()
        };
        assert!(!base.intersect(no_jump_mask).jump);
        // Poison: a mask that grants blink must NOT add blink to a base without it.
        let base_has_no_blink = AbilityGrant::RunJump.to_set();
        assert!(!base_has_no_blink.blink);
        let blink_mask = AbilitySet {
            blink: true,
            ..AbilitySet::NONE
        };
        assert!(
            !base_has_no_blink.intersect(blink_mask).blink,
            "intersect must never ADD a verb — masks only remove"
        );
    }

    #[test]
    fn compose_folds_a_grant_list_by_union() {
        // A character listing multiple grants gets their union; order-independent.
        let a = AbilitySet::compose(&[AbilityGrant::RunJump, AbilityGrant::SandboxAll]);
        let b = AbilitySet::compose(&[AbilityGrant::SandboxAll, AbilityGrant::RunJump]);
        assert_eq!(a, b, "compose must be order-independent");
        assert_eq!(a, AbilitySet::sandbox_all());
        // A single run-jump grant composes to exactly move + jump + variable_jump.
        let run_jump = AbilitySet::compose(&[AbilityGrant::RunJump]);
        assert!(run_jump.move_horizontal && run_jump.jump && run_jump.variable_jump);
        assert!(
            !run_jump.dash && !run_jump.blink && !run_jump.attack && !run_jump.wall_jump,
            "run-jump must NOT grant sandbox verbs"
        );
    }

    #[test]
    fn free_flight_is_permanent_two_dimensional_movement_without_a_jump_button() {
        let kit = AbilityGrant::FreeFlight.to_set();
        assert!(kit.move_horizontal);
        assert!(kit.fly);
        assert!(kit.interact);
        assert!(!kit.jump);
        assert!(!kit.variable_jump);
        assert!(!kit.fly_toggle);
        assert!(kit.compatibility_warnings().is_empty());
    }

    /// **Granting `fly` through a `..NONE` spread grants PERMANENT flight**, and
    /// the trap is that it reads as "this body can fly".
    ///
    /// ⛔ two shipped kits did exactly that when `fly_toggle` was introduced —
    /// `enemies::movement_kit` and the boss kit in `spawn_actors` — and the cost
    /// was worse than a wrong default. Permanent flight is latched into
    /// `BodyFlightState` when the cluster is BUILT (`fly && !fly_toggle`), so a
    /// body whose brain expects to toggle flight on later never flies at all.
    /// The duel PCA pressed the toggle 128 times over 30 seconds with
    /// `fly_frames = 0`.
    ///
    /// ⚠ this test does not forbid the permanent kind — TwinTrack's spacecraft
    /// is exactly that, and says so. It pins that the two spellings are
    /// DIFFERENT, so a caller reaching for the ordinary one has to say the word.
    #[test]
    fn granting_flight_without_the_toggle_is_a_different_kit() {
        // `move_horizontal` because flight without it warns for an unrelated
        // reason; the subject here is the toggle alone.
        let permanent = AbilitySet {
            fly: true,
            move_horizontal: true,
            ..AbilitySet::NONE
        };
        assert!(permanent.fly);
        assert!(
            !permanent.fly_toggle,
            "a `..NONE` spread leaves the toggle off, which is PERMANENT flight"
        );

        let toggled = AbilitySet {
            fly: true,
            fly_toggle: true,
            move_horizontal: true,
            ..AbilitySet::NONE
        };
        assert_ne!(
            permanent, toggled,
            "the two kits must not be interchangeable — a body that flies on \
             command and a body that is always flying are different bodies"
        );
        // Neither spelling is malformed; the warning list is about `fly_toggle`
        // WITHOUT `fly`, which is the one that means nothing.
        assert!(permanent.compatibility_warnings().is_empty());
        assert!(toggled.compatibility_warnings().is_empty());
    }

    #[test]
    fn the_platformer_mobility_grants_compose_a_richer_kit() {
        // The exact kit a "proper" classic platformer protagonist composes: the
        // run-jump floor plus an air jump, wall mobility, and a fast fall — each
        // a single-verb grant appended to the list, none forking a preset.
        let kit = AbilitySet::compose(&[
            AbilityGrant::RunJump,
            AbilityGrant::AirJump,
            AbilityGrant::WallMobility,
            AbilityGrant::FastFall,
        ]);
        // The floor survives.
        assert!(kit.move_horizontal && kit.jump && kit.variable_jump);
        // Each grant lit its own verb.
        assert!(kit.double_jump, "AirJump grants the air jump flag");
        assert!(
            kit.wall_jump && kit.wall_cling,
            "WallMobility grants both wall verbs"
        );
        assert!(kit.fast_fall, "FastFall grants the dive");
        // WallMobility deliberately stops short of wall_climb, and nothing here
        // conjured the sandbox-only verbs.
        assert!(
            !kit.wall_climb && !kit.blink && !kit.dash && !kit.fly && !kit.attack,
            "the platformer kit is mobility only — no blink/dash/fly/climb/attack"
        );
        // A kit composed from these grants must not warn (each dependent verb has
        // its prerequisite: air/wall jumps ride the RunJump ground jump).
        assert!(
            kit.compatibility_warnings().is_empty(),
            "the composed platformer kit is internally consistent"
        );
    }

    /// **A grounded body and a flying body both hold station; a falling one
    /// does not.**
    ///
    /// The three cases Jon's parrot example names: talk to it standing on the
    /// ground and the conversation holds; talk to it while you are BOTH able to
    /// hover and it holds; talk to it while you are falling past and it breaks.
    #[test]
    fn holding_station_is_being_grounded_or_being_able_to_fly() {
        let grounded_walker = AbilitySet::basic();
        assert!(!grounded_walker.fly, "precondition: this body cannot fly");
        assert!(
            can_hold_station(&grounded_walker, true),
            "standing on a floor is how a body with no wings holds still"
        );
        assert!(
            !can_hold_station(&grounded_walker, false),
            "and falling past somebody is not a conversation either of them can hold"
        );

        let flier = AbilitySet {
            fly: true,
            ..AbilitySet::basic()
        };
        assert!(
            can_hold_station(&flier, false),
            "a body that can fly holds station in the air — the parrot, and \
             equally the player who can also fly"
        );
    }
}
