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
/// `text` is what we render today. `i18n_key` is reserved for future
/// localization; the convention is `"<verb>.<variant>"` in snake_case so a
/// future locale pack maps `attack.d_air` -> "Air vers le bas" etc.
///
/// There is no `icon` hook: it existed as `fn icon(&self) -> Option<IconId>`
/// over an uninhabited `enum IconId {}`, so it could only ever return `None`,
/// and nothing in the workspace ever called it. A symbolic rendering pass will
/// bring its own icon vocabulary; an empty placeholder bought nothing.
pub trait VariantLabel {
    fn text(&self) -> &'static str;
    fn i18n_key(&self) -> &'static str;
}

/// What pressing Interact would do right now.
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

