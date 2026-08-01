//! Typed per-verb variant enums for player affordances.
//!
//! Each "verb" button (Jump, Attack, Shield, Dash, Interact, Special)
//! resolves to one variant per frame describing the action that would
//! actually fire if the player pressed it RIGHT NOW. The label of the
//! variant (via [`VariantLabel`]) is what the HUD shows; the variant
//! itself is what gameplay should consume so the HUD and the simulation
//! never disagree about what a button does.
//!
//! Adding a new contextual rule = adding (or branching on) one variant
//! here and one resolver branch in [`super::resolvers`]. The HUD
//! updates automatically because it just displays the variant.

use std::borrow::Cow;

/// Renderable label hooks for a variant. Implemented by every
/// `*Variant` enum so the HUD (and future tutorial overlays, AI hint
/// systems, accessibility prompts) can pick the rendering style
/// independently of the resolver logic.
///
/// `text` is what we render today. `icon` is reserved for the future
/// symbolic rendering pass (e.g. a sword pointing down for `D-Air`).
/// `i18n_key` is reserved for future localization; the convention is
/// `"<verb>.<variant>"` in snake_case so a future locale pack maps
/// `attack.d_air` -> "Air vers le bas" etc.
pub trait VariantLabel {
    fn text(&self) -> &'static str;
    fn icon(&self) -> Option<IconId> {
        None
    }
    fn i18n_key(&self) -> &'static str;
}

/// Placeholder for the future icon registry. No icon catalog exists
/// yet; this enum is empty on purpose so `VariantLabel::icon` can
/// return `Option<IconId>` today without committing to a particular
/// icon vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconId {}

/// What pressing **Attack** would do right now. Smash-style vocabulary:
/// grounded reads (`Jab` / tilts / smashes) versus aerial reads
/// (`*Air`) versus context-specific bounces (`Pogo` once the world
/// query lands in Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AttackVariant {
    /// Grounded, no directional aim: light neutral attack.
    #[default]
    Jab,
    /// Grounded + aim down: low poke.
    DTilt,
    /// Grounded + aim up: anti-air poke.
    UTilt,
    /// Reserved for the held-attack-with-direction read. Resolvers
    /// don't fire these yet; the variants exist so the HUD can adopt
    /// them in lockstep with the smash mechanic when it lands.
    FSmash,
    DSmash,
    USmash,
    /// Aerial, no directional aim: neutral air.
    NAir,
    /// Aerial, aim opposite facing: back air.
    BAir,
    /// Aerial, aim up: up air.
    UAir,
    /// Aerial, aim down: down air. Becomes [`AttackVariant::Pogo`] once
    /// a pogo target is in range below the player (Phase 3).
    DAir,
    /// Aerial, aim down, and a pogo-able target is below the player.
    /// Distinct from `DAir` because the gameplay outcome is different
    /// (bounce + dash refresh vs free-air swing).
    Pogo,
}

impl VariantLabel for AttackVariant {
    fn text(&self) -> &'static str {
        match self {
            AttackVariant::Jab => "Jab",
            AttackVariant::DTilt => "Down Tilt",
            AttackVariant::UTilt => "Up Tilt",
            AttackVariant::FSmash => "F-Smash",
            AttackVariant::DSmash => "D-Smash",
            AttackVariant::USmash => "U-Smash",
            AttackVariant::NAir => "N-Air",
            AttackVariant::BAir => "B-Air",
            AttackVariant::UAir => "U-Air",
            AttackVariant::DAir => "D-Air",
            AttackVariant::Pogo => "Pogo",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            AttackVariant::Jab => "attack.jab",
            AttackVariant::DTilt => "attack.d_tilt",
            AttackVariant::UTilt => "attack.u_tilt",
            AttackVariant::FSmash => "attack.f_smash",
            AttackVariant::DSmash => "attack.d_smash",
            AttackVariant::USmash => "attack.u_smash",
            AttackVariant::NAir => "attack.n_air",
            AttackVariant::BAir => "attack.b_air",
            AttackVariant::UAir => "attack.u_air",
            AttackVariant::DAir => "attack.d_air",
            AttackVariant::Pogo => "attack.pogo",
        }
    }
}

/// What pressing **Jump** would do right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JumpVariant {
    #[default]
    Jump,
    /// On a ledge: climb up.
    Climb,
    /// In morph-ball / crouched body mode: exit body mode.
    Unmorph,
    /// In water: swim stroke.
    Stroke,
}

impl VariantLabel for JumpVariant {
    fn text(&self) -> &'static str {
        match self {
            JumpVariant::Jump => "Jump",
            JumpVariant::Climb => "Climb",
            JumpVariant::Unmorph => "Unmorph",
            JumpVariant::Stroke => "Stroke",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            JumpVariant::Jump => "jump.jump",
            JumpVariant::Climb => "jump.climb",
            JumpVariant::Unmorph => "jump.unmorph",
            JumpVariant::Stroke => "jump.stroke",
        }
    }
}

/// What pressing **Shield** would do right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShieldVariant {
    #[default]
    Shield,
    /// On a ledge: smash-style roll-back-onto-platform.
    Roll,
}

impl VariantLabel for ShieldVariant {
    fn text(&self) -> &'static str {
        match self {
            ShieldVariant::Shield => "Shield",
            ShieldVariant::Roll => "Roll",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            ShieldVariant::Shield => "shield.shield",
            ShieldVariant::Roll => "shield.roll",
        }
    }
}

/// What pressing **Dash** would do right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DashVariant {
    #[default]
    Dash,
    /// Aerial: i-frame dodge.
    Dodge,
}

impl VariantLabel for DashVariant {
    fn text(&self) -> &'static str {
        match self {
            DashVariant::Dash => "Dash",
            DashVariant::Dodge => "Dodge",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            DashVariant::Dash => "dash.dash",
            DashVariant::Dodge => "dash.dodge",
        }
    }
}

/// What pressing **Interact** would do right now.
///
/// Carries an optional authored prompt string so a chest with an
/// authored prompt of "Loot Cache" can override the generic "Open"
/// without forcing a new variant per author-defined phrase.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InteractVariant {
    /// Nothing within range. The button still renders.
    #[default]
    None,
    /// NPC in range — opens dialogue.
    Talk,
    /// Chest in range — opens it.
    Open,
    /// Door in range — uses it. (Doors today share the buffered
    /// interact path with chests/NPCs; this variant lets the HUD
    /// distinguish them when authoring prompts.)
    Use,
    /// Switch in range — flips it.
    Activate,
    /// No interactable in range, but the player holds an item that
    /// rebinds Interact to a context action (the portal gun: toggle
    /// blue/orange mode). Takes precedence over `None` but yields to a
    /// genuine interactable.
    ModeSwitch,
    /// Authored prompt override: the interactable's own prompt
    /// string. Use sparingly — prefer the typed variants above so the
    /// HUD can swap icons / locales coherently.
    Custom(Cow<'static, str>),
}

impl VariantLabel for InteractVariant {
    fn text(&self) -> &'static str {
        match self {
            // Nothing to interact with: label it "Context" (not the
            // misleading "Interact") since a press would do nothing.
            InteractVariant::None => "Context",
            InteractVariant::Talk => "Talk",
            InteractVariant::Open => "Open",
            InteractVariant::Use => "Use",
            InteractVariant::Activate => "Activate",
            InteractVariant::ModeSwitch => "Mode Switch",
            // `&'static str` return forces typed-variant text. Custom
            // prompts are rendered via [`InteractVariant::display`] so
            // the HUD path can borrow either source uniformly.
            InteractVariant::Custom(_) => "Interact",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            InteractVariant::None => "interact.none",
            InteractVariant::Talk => "interact.talk",
            InteractVariant::Open => "interact.open",
            InteractVariant::Use => "interact.use",
            InteractVariant::Activate => "interact.activate",
            InteractVariant::ModeSwitch => "interact.mode_switch",
            InteractVariant::Custom(_) => "interact.custom",
        }
    }
}

impl InteractVariant {
    /// HUD-facing display: typed variants return their canonical
    /// `text()`; `Custom` returns the authored prompt itself. Returns
    /// `Cow` so the HUD never needs to allocate for the common typed
    /// case.
    pub fn display(&self) -> Cow<'_, str> {
        match self {
            InteractVariant::Custom(prompt) => Cow::Borrowed(prompt.as_ref()),
            other => Cow::Borrowed(other.text()),
        }
    }
}

/// What pressing **Special** would do right now. Smash-style four-way
/// vocabulary (neutral / side / up / down) plus the motion-input
/// special readings the player can trigger with quarter-circle
/// inputs.
///
/// **Today** every variant *fires the same gameplay action* (the
/// default charged projectile / fireball — Jon's "wire those in and
/// just set them to fireball and change what they do later").
/// They're distinct here so the HUD can label them differently
/// today, and so adding the real gameplay branches later is a
/// one-arm change in the consumer instead of a refactor across the
/// affordances surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpecialVariant {
    /// Cold-start fallback before the resolver runs. Pure
    /// inhabitant; never produced by `resolve_special`.
    #[default]
    Special,
    /// Neutral special — no directional hold on the stick.
    /// Today: charged fireball projectile.
    NeutralSpecial,
    /// Side special — stick held forward or back along the player's
    /// facing axis. Today: fireball.
    SideSpecial,
    /// Up special — stick held up (with or without lateral).
    /// Today: fireball.
    UpSpecial,
    /// Down special — stick held down. Today: fireball.
    DownSpecial,
    /// Motion-input special: the canonical quarter-circle-forward
    /// fireball read. Distinct from `NeutralSpecial` for HUD
    /// purposes — once the input-buffer detects QCF the label
    /// switches and the player gets feedback that the motion read.
    /// Today: same gameplay outcome as `NeutralSpecial`; in future
    /// this'll diverge.
    Hadouken,
    /// Quick blink / dash-warp. Distinct from the others because
    /// the touch HUD has a separate Blink button that always reads
    /// as this.
    Blink,
}

impl VariantLabel for SpecialVariant {
    fn text(&self) -> &'static str {
        match self {
            SpecialVariant::Special => "Special",
            SpecialVariant::NeutralSpecial => "N-Special",
            SpecialVariant::SideSpecial => "S-Special",
            SpecialVariant::UpSpecial => "U-Special",
            SpecialVariant::DownSpecial => "D-Special",
            SpecialVariant::Hadouken => "Hadouken",
            SpecialVariant::Blink => "Blink",
        }
    }

    fn i18n_key(&self) -> &'static str {
        match self {
            SpecialVariant::Special => "special.fallback",
            SpecialVariant::NeutralSpecial => "special.neutral",
            SpecialVariant::SideSpecial => "special.side",
            SpecialVariant::UpSpecial => "special.up",
            SpecialVariant::DownSpecial => "special.down",
            SpecialVariant::Hadouken => "special.hadouken",
            SpecialVariant::Blink => "special.blink",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_variant_labels_match_smash_vocabulary() {
        assert_eq!(AttackVariant::Jab.text(), "Jab");
        assert_eq!(AttackVariant::DTilt.text(), "Down Tilt");
        assert_eq!(AttackVariant::NAir.text(), "N-Air");
        assert_eq!(AttackVariant::DAir.text(), "D-Air");
        assert_eq!(AttackVariant::BAir.text(), "B-Air");
        assert_eq!(AttackVariant::UAir.text(), "U-Air");
        assert_eq!(AttackVariant::Pogo.text(), "Pogo");
    }

    #[test]
    fn interact_custom_renders_authored_prompt() {
        let v = InteractVariant::Custom(Cow::Borrowed("Loot Cache"));
        assert_eq!(v.display(), "Loot Cache");
        // Typed variants keep their canonical label.
        assert_eq!(InteractVariant::Talk.display(), "Talk");
        // No interactable → "Context", not "Interact".
        assert_eq!(InteractVariant::None.display(), "Context");
        assert_eq!(InteractVariant::ModeSwitch.display(), "Mode Switch");
    }

    #[test]
    fn i18n_keys_are_snake_cased_and_prefixed() {
        assert!(AttackVariant::DAir.i18n_key().starts_with("attack."));
        assert!(JumpVariant::Climb.i18n_key().starts_with("jump."));
        assert!(ShieldVariant::Roll.i18n_key().starts_with("shield."));
        assert!(DashVariant::Dodge.i18n_key().starts_with("dash."));
        assert!(InteractVariant::Talk.i18n_key().starts_with("interact."));
        assert!(SpecialVariant::Hadouken.i18n_key().starts_with("special."));
        assert!(SpecialVariant::NeutralSpecial
            .i18n_key()
            .starts_with("special."));
    }
}
