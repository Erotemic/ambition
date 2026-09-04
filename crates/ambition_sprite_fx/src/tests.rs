use super::*;

/// THE FREE PATH IS FREE, AND ONLY THE FREE PATH IS.
///
/// The split this crate exists to make: an effect the built-in sprite pipeline
/// can express must not cost a material, and one it cannot must not be quietly
/// approximated by writing a colour that means something else. A `HueShift`
/// treated as free would write its WHITE colour argument into `Sprite.color`
/// and render the sprite unchanged — a silent no-op, the worst outcome.
#[test]
fn only_the_multiply_is_free() {
    assert!(!SpriteEffect::Tint(Color::WHITE).needs_material());
    assert!(SpriteEffect::HueShift { degrees: 90.0 }.needs_material());
    assert!(SpriteEffect::Saturate { factor: 0.0 }.needs_material());
    assert!(SpriteEffect::Silhouette(Color::WHITE).needs_material());
}

/// The free system applies a tint and REFUSES the rest.
#[test]
fn the_free_system_applies_a_tint_and_leaves_shader_effects_alone() {
    let mut app = App::new();
    app.add_systems(Update, apply_free_sprite_effects);

    let red = Color::srgb(1.0, 0.0, 0.0);
    let tinted = app
        .world_mut()
        .spawn((SpriteEffect::Tint(red), Sprite::default()))
        .id();
    // A hue shift's colour argument is white; if this system treated it as a
    // tint it would write white here and the sprite would look untouched.
    let hued = app
        .world_mut()
        .spawn((
            SpriteEffect::HueShift { degrees: 180.0 },
            Sprite {
                color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            },
        ))
        .id();
    app.update();

    assert_eq!(app.world().get::<Sprite>(tinted).unwrap().color, red);
    assert_eq!(
        app.world().get::<Sprite>(hued).unwrap().color,
        Color::srgb(0.0, 1.0, 0.0),
        "the free system wrote a shader effect's colour argument into the sprite"
    );
}

/// Each effect reaches the shader as its OWN opcode.
///
/// The uniform is the whole contract between this enum and the WGSL; a
/// duplicated or shifted opcode renders the wrong operation with no error
/// anywhere, on a machine that may not be the author's.
#[test]
fn every_effect_has_its_own_distinct_opcode() {
    let basis = SpriteFrameBasis {
        uv_rect: Vec4::new(0.0, 0.0, 1.0, 1.0),
        size: Vec2::splat(16.0),
    };
    let ops: Vec<f32> = [
        SpriteEffect::Tint(Color::WHITE),
        SpriteEffect::HueShift { degrees: 30.0 },
        SpriteEffect::Saturate { factor: 0.5 },
        SpriteEffect::Silhouette(Color::BLACK),
    ]
    .into_iter()
    .map(|effect| {
        SpriteFxMaterial::for_effect(effect, basis, Handle::default(), false)
            .control
            .x
    })
    .collect();

    assert_eq!(ops, vec![0.0, 1.0, 2.0, 3.0]);
    // And the scalar argument travels in the slot the shader reads it from.
    let hue = SpriteFxMaterial::for_effect(
        SpriteEffect::HueShift { degrees: 137.5 },
        basis,
        Handle::default(),
        true,
    );
    assert_eq!(hue.control.z, 137.5, "the hue angle did not reach the shader");
    assert_eq!(hue.control.y, 1.0, "flip_x did not reach the shader");
}

/// A whole-image sprite's frame is the whole texture, at its custom size.
#[test]
fn a_whole_image_sprites_basis_is_the_whole_texture() {
    let mut images = Assets::<Image>::default();
    let layouts = Assets::<TextureAtlasLayout>::default();
    let image = images.add(Image::default());
    let sprite = Sprite {
        image,
        custom_size: Some(Vec2::new(40.0, 24.0)),
        ..default()
    };

    let basis = sprite_frame_basis(&sprite, &layouts, &images).expect("the image is loaded");
    assert_eq!(basis.uv_rect, Vec4::new(0.0, 0.0, 1.0, 1.0));
    assert_eq!(
        basis.size,
        Vec2::new(40.0, 24.0),
        "custom_size is the drawn size; the native pixel size is only the fallback"
    );

    // An unloaded texture answers None rather than inventing a frame — the
    // caller draws the plain sprite that frame instead of a quad sampling
    // nothing.
    let missing = Sprite {
        image: Handle::default(),
        ..default()
    };
    assert!(sprite_frame_basis(&missing, &layouts, &images).is_none());
}

/// A SHADER EFFECT TAKES THE DRAW OVER, AND GIVES IT BACK.
///
/// ⛔ The giving-back half is the one worth a test. A one-way takeover looks
/// completely correct in every screenshot and makes the effect impossible to
/// CANCEL — the sprite would stay a mesh after the effect was removed, which is
/// a worse failure than the effect never applying, and it is invisible until
/// something removes an effect.
#[test]
fn a_shader_effect_replaces_the_sprite_draw_and_restores_it() {
    let mut app = App::new();
    app.init_resource::<Assets<SpriteFxMaterial>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<Assets<Image>>();
    app.add_systems(
        Update,
        (restore_sprites_without_effects, draw_sprite_effects).chain(),
    );

    let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::HueShift { degrees: 137.5 },
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::new(32.0, 16.0)),
                ..default()
            },
            Transform::default(),
        ))
        .id();

    app.update();
    assert!(
        app.world().get::<Sprite>(entity).is_none(),
        "the sprite still draws alongside the mesh, so the gun renders twice"
    );
    assert!(app.world().get::<Mesh2d>(entity).is_some());
    let state = app
        .world()
        .get::<SpriteFxDrawn>(entity)
        .expect("the original sprite was not kept");
    assert_eq!(state.original.custom_size, Some(Vec2::new(32.0, 16.0)));
    // The quad is scaled to the sprite's drawn size, not left unit-sized.
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale.truncate(),
        Vec2::new(32.0, 16.0)
    );

    // Cancel the effect: the entity must come back exactly as it went in.
    app.world_mut().entity_mut(entity).remove::<SpriteEffect>();
    app.update();
    let restored = app
        .world()
        .get::<Sprite>(entity)
        .expect("removing the effect did not give the sprite back");
    assert_eq!(restored.custom_size, Some(Vec2::new(32.0, 16.0)));
    assert_eq!(restored.image, image);
    assert!(app.world().get::<Mesh2d>(entity).is_none());
    assert!(app.world().get::<SpriteFxDrawn>(entity).is_none());
}

/// A free effect is left on the sprite path, never turned into a mesh.
#[test]
fn a_tint_never_becomes_a_mesh() {
    let mut app = App::new();
    app.init_resource::<Assets<SpriteFxMaterial>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<Assets<Image>>();
    app.add_systems(Update, (apply_free_sprite_effects, draw_sprite_effects).chain());

    let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::Tint(Color::srgb(1.0, 0.0, 0.0)),
            Sprite {
                image,
                ..default()
            },
            Transform::default(),
        ))
        .id();
    app.update();

    assert!(
        app.world().get::<Mesh2d>(entity).is_none(),
        "a multiply was routed through a material the sprite pipeline does for free"
    );
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0)
    );
}

/// ⛔⛔ THE PLUGIN MUST SURVIVE A COMPOSITION THAT HAS AN ASSET PLUGIN AND NO
/// RENDER STACK, because that is what every demo test binary is.
///
/// The plugin already skips its mesh path when there is no
/// `EmbeddedAssetRegistry`. That check answers *"is there an AssetPlugin"*, and
/// the demos HAVE one — what they do not have is `Assets<Mesh>`. In Bevy 0.19 a
/// missing system parameter is a HARD FAILURE that takes the whole `App` down,
/// so `draw_sprite_effects` did not skip: it panicked, and with it every test in
/// the binary.
///
/// ⭐ Measured on the workspace feature union 2026-09-04, before the guard:
/// **7,072 passed, 40 failed, and 39 of the 40 were this one system**, every one
/// reading *"Parameter `ResMut<Assets<Mesh>>` failed validation: Resource does
/// not exist"*. The 40th named it too.
///
/// ⚠ THE OTHER TWO SYSTEMS MUST STILL RUN, which is the half a bare "does not
/// panic" test would miss. Guarding by disabling the whole plugin would also
/// pass, and would delete the free tint path from every demo — so this asserts
/// the tint was applied on the same frame the mesh path stood down.
#[test]
fn the_plugin_steps_in_a_composition_with_no_render_stack_and_still_tints() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    // No `Assets<Mesh>`, no `Assets<TextureAtlasLayout>`, no `Assets<Image>`:
    // the state a headless demo App is actually in.
    app.add_plugins(SpriteFxPlugin);

    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::Tint(Color::srgb(0.25, 0.5, 0.75)),
            Sprite::default(),
            Transform::default(),
        ))
        .id();
    // A second entity asking for the MESH path, so the guarded system has work
    // waiting for it and a missing guard cannot be hidden by an empty query.
    app.world_mut().spawn((
        SpriteEffect::HueShift { degrees: 90.0 },
        Sprite::default(),
        Transform::default(),
    ));

    app.update();

    assert_eq!(
        app.world().entity(entity).get::<Sprite>().expect("sprite").color,
        Color::srgb(0.25, 0.5, 0.75),
        "the FREE path must keep running when the mesh path stands down — a guard \
         that disabled the whole plugin would pass a bare no-panic assertion and \
         delete every tint in the demo",
    );
}
