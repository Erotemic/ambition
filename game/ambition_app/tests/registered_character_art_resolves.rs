//! **Every registered character's declared art actually resolves.**
//!
//! `CharacterDefinition::with_sheet` names a sheet TARGET, and preparation checks
//! it against the engine's baked manifest index. The check was already there and
//! already ran — it just could not fail anything. An unresolved reference is
//! logged and the registration publishes anyway, deliberately: a character that
//! draws a placeholder and says why beats a session that refuses to boot.
//!
//! The gap that left is that `checked_namespaces()` reports a resolver RAN, not
//! that it agreed. So four shipped characters — `sanic`, `super_sanic`, `mary_o`,
//! `mary_o_tall` — declared `<name>_spritesheet` (the sheet FILE stem) where the
//! registry is keyed by `<name>` (the sheet's `target:`), drew placeholders in the
//! running game, printed four ERROR lines listing all 400-odd available ids on
//! every boot, and no test anywhere went red.
//!
//! This is that test. It is not the same check as the resolver — the resolver
//! proves content agrees with content. This proves the SHIPPED composition
//! contains no such disagreement, which is a claim only the assembled app can
//! make.

use ambition::actors::character_runtime::PreparedCharacterRegistry;
use ambition_app::app::{build_visible_app, VisibleRenderMode};

#[test]
fn every_registered_character_resolves_the_art_it_declares() {
    // SHELL-hosted: the provider plugins that register characters (Sanic, Mary-O,
    // Pocket) join in that composition, and they are the ones this guard exists
    // for. The non-hosted build has no cast to check.
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let registry = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("the shipped composition registers characters through the one seam");
    assert!(
        !registry.is_empty(),
        "no character reached the prepared registry, so this guard would pass vacuously"
    );

    let mut complaints = Vec::new();
    for (id, prepared) in registry.iter() {
        for unresolved in prepared.unresolved_references() {
            // The resolver's own message already names the id, who declared it,
            // and the closest match. Reproducing it whole keeps the fix in the
            // failure output instead of one indirection away.
            complaints.push(format!("  {id}: {unresolved}"));
        }
    }
    assert!(
        complaints.is_empty(),
        "{} registered character reference(s) name something that does not exist. \
         Preparation logs these and publishes anyway, so the running game draws a \
         placeholder rather than crashing — which is why this has to be a test:\n{}",
        complaints.len(),
        complaints.join("\n"),
    );
}
