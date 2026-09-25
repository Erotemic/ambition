//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of all animation rows a character sheet can
//! define. The boss has its own row set (`boss_encounter::sprites::BossAnim`).
//! A sheet need not define every row: `CharacterSheetSpec::resolve_anim` falls
//! back for rows the sheet does not have.

/// Animation ids that a character sheet may define.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterAnim {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Jump = 3,
    Fall = 4,
    Slash = 5,
    Hit = 6,
    Death = 7,
    BlinkOut = 8,
    BlinkIn = 9,
    Dash = 10,
    /// Free-flight pose (jets or hover). Generator row `hover`.
    Fly = 11,
    /// Idle gesture for hostile NPCs (generator row `taunt`). No picker selects
    /// it yet. The variant keeps atlas indexing aligned with the PNG.
    Taunt = 12,
    /// Hang on a ledge. Selected by `pick_player_anim` while
    /// `Player::ledge_grab` is `Some` and not climbing.
    LedgeGrab = 13,
    /// Slow climb loop against an overhead grip. `pick_player_anim` does not
    /// select it; mantle pop-ups use `LedgeGetup`.
    LedgeClimb = 14,
    /// Mantle pop-up from grip to standing. Selected by `pick_player_anim`
    /// when `Player::ledge_grab.climbing == true`.
    LedgeGetup = 15,
    /// Wall cling with no slide or climb. Different from `wall_slide`, the
    /// downward-scrape state.
    WallGrab = 16,
    /// Held-jump glide pose. Selected by `player.gliding`. Different from
    /// `Fly` (jets) and `Fall`.
    FloatGlide = 17,
    /// Heavy landing. Selected by `pick_player_anim` while
    /// `BodyAnimFacts::land_anim_timer` is positive and `land_anim_hard` is set.
    LandHard = 18,
    /// Recovery after a landing. Plays while `land_anim_timer` is positive and
    /// `land_anim_hard` is false, or in the tail of a hard landing.
    LandRecovery = 19,
    /// Brief animation-only dash pre-roll.
    DashStartup = 20,
    /// Grounded side swing. Used for the `Forward`, `Neutral`, `Back`,
    /// `DashForward`, and `WallOut` attack intents. The sprite flips with facing.
    AttackSide = 21,
    /// Grounded up-tilt — overhead arc.
    AttackUp = 22,
    /// Grounded down-tilt — sweep down to the floor.
    AttackDown = 23,
    /// Aerial neutral spin-slash. No `AttackIntent::AirNeutral` exists yet, so
    /// no picker selects this row.
    AirNeutral = 24,
    /// Aerial forward swing.
    AirForward = 25,
    /// Aerial backward swing (no engine intent yet — placeholder row).
    AirBack = 26,
    /// Aerial down-thrust (spike).
    AirDown = 27,
    /// Aerial up-thrust.
    AirUp = 28,
    /// Ledge roll with invulnerability frames. Selected by `pick_player_anim`
    /// when `ledge_grab.climbing && getup_kind == Roll`.
    LedgeRoll = 29,
    /// Ledge getup attack. Selected by `pick_player_anim` when
    /// `ledge_grab.climbing && getup_kind == Attack`. The slash op fires at the
    /// start of the transition, so the sprite should peak the swing mid-clip.
    LedgeGetupAttack = 30,
    /// Crouch pose. Selected while `body_mode.body_mode == BodyMode::Crouching`
    /// and the body is not walking. Generator row `crouch`.
    Crouch = 31,
    /// Hands-and-knees crawl. Selected while
    /// `body_mode.body_mode == BodyMode::Crawling`. Falls back to `crouch_walk`.
    Crawl = 32,
    /// Ground slide. Selected while `body_mode.body_mode == BodyMode::Sliding`.
    /// Generator row `slide`.
    Slide = 33,
    /// Ladder or vine climb. Selected while
    /// `body_mode.body_mode == BodyMode::Climbing`. Generator row `climb`.
    LadderClimb = 34,
    /// Swim stroke. Selected while `env_contact.water.is_some()` and the `swim`
    /// ability is on. Generator row `swim`.
    Swim = 35,
    /// Projectile release pose. Generator row `shoot`. Not selected yet; needs
    /// a `shoot_anim_timer` on `BodyAnimFacts`.
    Shoot = 36,
    /// Projectile charge or aim pose. Generator row `aim`. Not selected yet;
    /// needs the charge state as a presentation flag.
    Aim = 37,
    /// Melee wind-up pose. Generator row `charge`. No engine intent maps to it.
    Charge = 38,
    /// Shield-up pose. Generator row `block`. Needs the shield-held state on
    /// `BodyAnimFacts` (the input is `ControlFrame.shield_held` +
    /// `AbilitySet::shield`).
    Block = 39,
    /// Ground dodge roll with invulnerability frames. Generator row `roll`.
    /// Needs the dodge-roll timer on `BodyAnimFacts`. `LedgeRoll` is the ledge
    /// variant.
    DodgeRoll = 40,
    /// Wall-jump push-off. Generator row `wall_jump`. Not selected yet; needs a
    /// `wall_jump_anim_timer` set by the wall-jump op.
    WallJump = 41,
    /// Interaction gesture. Generator row `interact`. Not selected yet; needs
    /// an `interact_anim_timer` set when the interact buffer fires.
    Interact = 42,
    /// Heavy melee (generator row `punch`). The quick `jab` aliases to `Slash`.
    /// Not selected yet: `BodyMelee`/`AttackSpec` have no heavy tag, so the
    /// picker uses the directional swing.
    Punch = 43,
    /// Charge-and-release special (generator row `special`). Used by the glider
    /// zoning verb. Not selected yet: no actor cluster has a "special active"
    /// flag. The `special` field on [`BodyAnimView`] is already in the ladder.
    Special = 44,
    /// Looping ball locomotion (spin dash, morph ball). Generator row `ball`.
    /// It loops; [`Self::DodgeRoll`] is one-shot. Selected while
    /// `BodyAnimFacts::rolling` is set.
    Roll = 45,
    /// Braking against travel. Generator row `skid`. Selected while
    /// `BodyMotionFacts::skidding` is set.
    Skid = 46,
    /// Shelled enemy pulls into its shell (generator row `retreat`). One-shot
    /// into [`Self::ShellIdle`]. A content state machine forces it with an
    /// animation override; the shared picker does not.
    Retreat = 47,
    /// At rest inside the shell (generator row `boxed_idle`). The kickable pose.
    ShellIdle = 48,
    /// Wary look out of the shell (generator row `peek`). One-shot into
    /// [`Self::Emerge`].
    Peek = 49,
    /// Climb out of the shell (generator row `emerge`). One-shot.
    Emerge = 50,
    /// An aggravated hiss/telegraph (generator row `hiss`).
    Hiss = 51,
    // --- Form transitions -------------------------------------------------
    //
    // Clips for a body that changes form (power tier, stage). All are one-shot
    // and hold the arrived form. Each clip is on the sheet of the arrived form,
    // so the body plays it after the identity swap and nothing defers the swap.
    // `transform_beat` holds the pose while it runs.
    //
    // These are not aliases of [`Self::Hit`]. The beat picks the transition and
    // the locomotion picker reads hitstun; an alias made the hit read replay
    // the form-change flicker.
    /// Arrive at a larger form (generator row `grow`).
    Grow = 52,
    /// Arrive at a smaller form, one tier down (generator row `shrink`).
    /// Falls back to [`Self::Hit`].
    Shrink = 53,
    /// Arrive two tiers down at once (generator row `big_shrink`).
    BigShrink = 54,
    /// Arrive at a different form of the same size (generator row
    /// `transform`), usually a palette flash.
    Transform = 55,
    /// Crouch-walk (generator row `crouch_walk`).
    ///
    /// Not an alias of [`Self::Crawl`]: crawling is a different body mode with
    /// a different box. `Crawl` falls back to this row.
    CrouchWalk = 56,
    /// Crouching and airborne (generator row `crouch_jump`).
    CrouchJump = 57,
    Bark = 58,
    SitDown = 59,
    SitIdle = 60,
    StandUp = 61,
}

impl CharacterAnim {
    /// Map a generator row name (`rows[*].animation` in `*_spritesheet.ron`)
    /// to its variant. Returns `None` for unknown names; the sheet spec drops
    /// that row.
    ///
    /// Aliases:
    /// - `hurt` -> `Hit`.
    /// - `hover` -> `Fly`.
    /// - `opening` -> `Idle`, `stable` / `spin` -> `Walk`, `closing` -> `Run`.
    ///   Gate portal and ring sheets use these slots for their phase rows; see
    ///   `GATE_PORTAL_SHEET` / `GATE_RING_SHEET` in `sheets.rs`.
    pub fn from_name(name: &str) -> Option<Self> {
        // Exact matches against the generator strings.
        Some(match name {
            // `rest` (boss sheets) and `front_idle` / `side_idle` (facing-split
            // sheet) alias to Idle so the catalog can load every character.
            "idle" | "opening" | "rest" | "front_idle" | "side_idle" => Self::Idle,
            "walk" | "stable" | "spin" | "side_walk" => Self::Walk,
            "run" | "closing" => Self::Run,
            "jump" => Self::Jump,
            "fall" => Self::Fall,
            // `jab` is the quick poke; it shares the generic `Slash` read.
            // `punch` is the committal heavy with its own row.
            "slash" | "jab" => Self::Slash,
            "punch" => Self::Punch,
            // Full charge-and-release beat; `Charge` is only the wind-up.
            "special" => Self::Special,
            "hit" | "hurt" => Self::Hit,
            "grow" => Self::Grow,
            "shrink" => Self::Shrink,
            "big_shrink" => Self::BigShrink,
            "transform" => Self::Transform,
            "death" => Self::Death,
            "bark" => Self::Bark,
            "sit_down" => Self::SitDown,
            "sit_idle" => Self::SitIdle,
            "stand_up" => Self::StandUp,
            "blink_out" => Self::BlinkOut,
            "blink_in" => Self::BlinkIn,
            "dash" => Self::Dash,
            "fly" | "hover" => Self::Fly,
            "taunt" => Self::Taunt,
            "ledge_grab" => Self::LedgeGrab,
            "ledge_climb" => Self::LedgeClimb,
            "ledge_getup" => Self::LedgeGetup,
            "wall_grab" => Self::WallGrab,
            "float_glide" => Self::FloatGlide,
            "land_hard" => Self::LandHard,
            "land_recovery" => Self::LandRecovery,
            "dash_startup" => Self::DashStartup,
            "attack_side" => Self::AttackSide,
            "attack_up" => Self::AttackUp,
            "attack_down" => Self::AttackDown,
            "air_neutral" => Self::AirNeutral,
            "air_forward" => Self::AirForward,
            "air_back" => Self::AirBack,
            "air_down" => Self::AirDown,
            "air_up" => Self::AirUp,
            "ledge_roll" => Self::LedgeRoll,
            "ledge_getup_attack" => Self::LedgeGetupAttack,
            "crouch" => Self::Crouch,
            "crouch_walk" => Self::CrouchWalk,
            "crouch_jump" => Self::CrouchJump,
            "crawl" => Self::Crawl,
            "slide" => Self::Slide,
            "climb" | "ladder_climb" => Self::LadderClimb,
            "swim" => Self::Swim,
            "shoot" => Self::Shoot,
            "aim" => Self::Aim,
            "charge" => Self::Charge,
            "block" | "shield" => Self::Block,
            "roll" | "dodge_roll" => Self::DodgeRoll,
            // `ball` is the looping curl; `roll` stays the one-shot dodge.
            "ball" => Self::Roll,
            "skid" => Self::Skid,
            // Shell cycle. A content state machine drives it with an override.
            "retreat" => Self::Retreat,
            "boxed_idle" => Self::ShellIdle,
            "peek" => Self::Peek,
            "emerge" => Self::Emerge,
            "hiss" => Self::Hiss,
            "wall_jump" => Self::WallJump,
            "interact" => Self::Interact,
            _ => return None,
        })
    }

    /// The next less specific pose, or `None` at `Idle`.
    ///
    /// This is a taxonomy, not a list of which poses an actor has. The actor's
    /// set is the rows in its manifest RON ([`CharacterSheetSpec::maps`]).
    /// [`CharacterSheetSpec::resolve_anim`] walks this chain to the most
    /// specific pose the sheet has. A sheet with only `slash` shows `slash` for
    /// an up-tilt; add an `attack_up` row and up-tilts use it with no code
    /// change.
    pub fn base_pose(self) -> Option<Self> {
        use CharacterAnim::*;
        Some(match self {
            Idle => return None,
            // Ground locomotion.
            Walk => Idle,
            Run => Walk,
            Crouch => Idle,
            CrouchWalk => Crouch,
            // Through `crouch_walk`: sheets that author only one author this one.
            Crawl => CrouchWalk,
            Slide => Run,
            // Air.
            Jump => Fall,
            Fall => Idle,
            FloatGlide => Fall,
            Fly => Idle,
            WallJump => Jump,
            WallGrab => Idle,
            // Dash.
            DashStartup => Dash,
            Dash => Run,
            // Directional and aerial swings refine the side swing, then slash.
            AttackUp => AttackSide,
            AttackDown => AttackSide,
            AttackSide => Slash,
            AirNeutral => Slash,
            AirForward => AttackSide,
            AirBack => AttackSide,
            AirUp => AttackUp,
            AirDown => AttackDown,
            Punch => Slash,
            Special => Slash,
            Slash => Idle,
            LedgeGetupAttack => LedgeGetup,
            // Ranged / charge.
            Shoot => Idle,
            Charge => Aim,
            Aim => Idle,
            // Defensive / utility.
            Block => Idle,
            // Without a ball row, the held last tumble frame is better than a run.
            Roll => DodgeRoll,
            Skid => Run,
            DodgeRoll => Idle,
            Interact => Idle,
            Swim => Idle,
            LadderClimb => Idle,
            // Ledge.
            LedgeClimb => LedgeGrab,
            LedgeGetup => LedgeGrab,
            LedgeRoll => DodgeRoll,
            LedgeGrab => Idle,
            // Blink.
            BlinkIn => Idle,
            BlinkOut => Idle,
            // Reactions.
            Death => Hit,
            Hit => Idle,
            // Form transitions: same-size goes to grow, grow to stand. A power
            // loss goes to the hurt reaction.
            Transform => Grow,
            CrouchJump => Jump,
            Grow => Idle,
            BigShrink => Shrink,
            Shrink => Hit,
            LandHard => LandRecovery,
            LandRecovery => Idle,
            // Idle-variant gesture.
            Taunt => Idle,
            // Shell cycle falls to the boxed pose, then Idle.
            ShellIdle => Idle,
            Retreat => ShellIdle,
            Peek => ShellIdle,
            Emerge => ShellIdle,
            Hiss => Idle,
            Bark | SitDown | SitIdle | StandUp => Idle,
        })
    }
}

pub fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash
            | CharacterAnim::Hit
            | CharacterAnim::Death
            | CharacterAnim::LedgeClimb
            | CharacterAnim::LedgeGetup
            | CharacterAnim::LedgeRoll
            | CharacterAnim::LedgeGetupAttack
            | CharacterAnim::LandHard
            | CharacterAnim::LandRecovery
            | CharacterAnim::DashStartup
            | CharacterAnim::AttackSide
            | CharacterAnim::AttackUp
            | CharacterAnim::AttackDown
            | CharacterAnim::AirNeutral
            | CharacterAnim::AirForward
            | CharacterAnim::AirBack
            | CharacterAnim::AirDown
            | CharacterAnim::AirUp
            // One-shot action poses hold the last frame.
            | CharacterAnim::Shoot
            | CharacterAnim::DodgeRoll
            | CharacterAnim::WallJump
            | CharacterAnim::Interact
            | CharacterAnim::Punch
            | CharacterAnim::Special
            // Shell transitions are one-shot; only `ShellIdle` and `Hiss` loop.
            | CharacterAnim::Retreat
            | CharacterAnim::Peek
            | CharacterAnim::Emerge
            // Form transitions hold the arrived form; a loop would flicker.
            | CharacterAnim::Grow
            | CharacterAnim::Shrink
            | CharacterAnim::BigShrink
            | CharacterAnim::Transform
            | CharacterAnim::Bark
            | CharacterAnim::SitDown
            | CharacterAnim::StandUp
    )
}

/// Content-driven animation pin for a body.
///
/// The locomotion picker knows nothing about content states. A content state
/// machine that needs a pose the picker cannot infer (for example a shell
/// withdraw) inserts this component. The anim-index rebuild uses it over the
/// picked pose. Remove it to return to normal picking.
///
/// The content system derives it from rollback state each tick, so it is not
/// snapshot state: a resim recomputes it before the anim index reads it.
///
/// It lives here, not in the actor crate, because it only wraps this crate's
/// vocabulary and all its readers sit above the actor crate.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorAnimOverride(pub CharacterAnim);

/// A short pose that follows an actor's authored ambient bark.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct ActorBarkGesture(pub f32);

#[cfg(test)]
mod shell_anim_tests {
    use super::{non_looping, CharacterAnim};

    /// The shell rows map to their own variants, so the sheet spec keeps them.
    #[test]
    fn the_shell_rows_map_to_their_own_variants() {
        assert_eq!(
            CharacterAnim::from_name("retreat"),
            Some(CharacterAnim::Retreat)
        );
        assert_eq!(
            CharacterAnim::from_name("boxed_idle"),
            Some(CharacterAnim::ShellIdle)
        );
        assert_eq!(CharacterAnim::from_name("peek"), Some(CharacterAnim::Peek));
        assert_eq!(
            CharacterAnim::from_name("emerge"),
            Some(CharacterAnim::Emerge)
        );
        assert_eq!(CharacterAnim::from_name("hiss"), Some(CharacterAnim::Hiss));
    }

    /// The withdraw/re-emerge transitions hold their final frame; only the at-rest
    /// shell (and the hiss telegraph) loop.
    #[test]
    fn the_shell_transitions_are_one_shot_but_the_boxed_pose_loops() {
        assert!(non_looping(CharacterAnim::Retreat));
        assert!(non_looping(CharacterAnim::Peek));
        assert!(non_looping(CharacterAnim::Emerge));
        assert!(!non_looping(CharacterAnim::ShellIdle));
        assert!(!non_looping(CharacterAnim::Hiss));
    }

    /// A missing shell row falls to the boxed pose, then Idle.
    #[test]
    fn missing_shell_rows_degrade_toward_idle() {
        assert_eq!(
            CharacterAnim::Retreat.base_pose(),
            Some(CharacterAnim::ShellIdle)
        );
        assert_eq!(
            CharacterAnim::ShellIdle.base_pose(),
            Some(CharacterAnim::Idle)
        );
    }
}
