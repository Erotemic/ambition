//! An ACTIVATED session carries the mechanics it was prepared against.
//!
//! ⛔⛤ **THIS ARM EXISTS BECAUSE I SHIPPED THE PROMOTION AND THEN FOUND IT HAD NO
//! WITNESS.** `b3839b28f` made activation `insert_resource` the prepared
//! generation's `SessionMechanics` so a later room transition, death or reset
//! rebuilds from the SAME values the session was prepared against; `d7be0bb5e`
//! then made bypassing it a type error at the construction call. Neither proves
//! the resource is actually THERE in the shipped composition — a promotion that
//! silently stopped happening would leave every road falling back to the App's
//! registries, which is exactly the defect the packet closed, and every arm would
//! stay green because the fallback is legal for a composition with no generation.
//!
//! ⇒ The fallback is what makes this arm necessary rather than redundant: it is
//! designed to be indistinguishable from success, so somebody has to assert the
//! generation exists on the road where it must.
//!
//! ⚠ **IT ASSERTS PRESENCE AND CONTENT, NOT JUST PRESENCE.** A `SessionMechanics`
//! promoted as `Default` would be present and empty, and `GenerationMechanics`
//! treats presence as *"this generation's values win"* — so an empty one would
//! win with nothing. Presence alone would pass under that.

/// The shipped composition, driven to a live room session.
///
/// ⚠ **THE HARNESS IS THE ONE THAT ACTIVATES**, and the first version of this arm
/// used `build_visible_app` with 90 bare `update()`s — which publishes a cast
/// (the premise below passes) and never activates a gameplay session, so the arm
/// failed against its own fixture rather than against the promotion. A fixture
/// that never reaches its subject is the recorded trap; the premise assertions
/// below are what told the two apart.
#[test]
fn an_activated_session_carries_the_cast_it_was_prepared_against() {
    use ambition_platformer2d::actors::session::mechanics::SessionMechanics;
    use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

    let mut sim = crate::common::fixed_60hz_room_sim("proving_grounds");
    for _ in 0..30 {
        sim.step(crate::common::base());
    }
    let world = sim.world();

    // ⛔ THE PREMISE FIRST: this composition really did publish a cast. Without
    // it, "the generation's cast matches the App's" is two empties agreeing.
    let published: Vec<String> = world
        .get_resource::<PreparedCharacterRegistry>()
        .map(|registry| registry.ids().map(str::to_string).collect())
        .unwrap_or_default();
    assert!(
        published.len() >= 10,
        "the shipped composition published {} prepared character(s), so this arm \
         is comparing almost nothing",
        published.len()
    );

    let generation = world
        .get_resource::<SessionMechanics>()
        .unwrap_or_else(|| {
            panic!(
                "the activated session carries NO `SessionMechanics`, so every \
                 later rebuild road — a door, a death, a reset — falls back to \
                 whatever the App holds at the time. That fallback is LEGAL for a \
                 composition with no activated generation, which is why nothing \
                 else goes red when this stops happening."
            )
        });

    let frozen: Vec<String> = generation
        .prepared_cast()
        .map(|cast| cast.ids().map(str::to_string).collect())
        .unwrap_or_default();

    // ⛔ CONTENT, NOT PRESENCE. A `Default::default()` promotion would satisfy a
    // `contains_resource` assertion and then win every read with an empty cast.
    assert_eq!(
        frozen, published,
        "the session's frozen cast and the App's published cast name different \
         characters — the generation was promoted from something other than the \
         prepared record it was built from"
    );
}
