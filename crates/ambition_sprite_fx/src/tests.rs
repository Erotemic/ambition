use super::*;

/// The free path is free, and only the free path is.
///
/// An effect the sprite pipeline can express must not cost a material. One it
/// cannot express must not be approximated: a `HueShift` treated as free would
/// write its white colour argument into `Sprite.color` and change nothing.
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

/// A shader effect takes the draw over, and gives it back.
///
/// A one-way takeover looks correct in screenshots but makes the effect
/// impossible to cancel: the sprite would stay a mesh after removal.
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

/// The plugin must survive a composition that has an asset plugin and no
/// render stack, as every demo test binary does.
///
/// The `EmbeddedAssetRegistry` check finds an AssetPlugin, but there is no
/// `Assets<Mesh>`. In Bevy 0.19 a missing system parameter fails the whole
/// `App`, so `draw_sprite_effects` must stand down on its own guard.
///
/// The other two systems must still run. Disabling the whole plugin would
/// also avoid the panic but remove the free tint path, so this asserts the
/// tint was applied on the same frame the mesh path stood down.
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

/// A world with the effect systems and the assets they need, without a render
/// stack — the shape every test below wants.
fn fx_app() -> App {
    let mut app = App::new();
    app.init_resource::<Assets<SpriteFxMaterial>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<Assets<Image>>();
    // The plugin's own ordering, spelled out: free path first (so a tint is off
    // the sprite before the mesh path clones it), then the two restores, then
    // the draw.
    app.add_systems(
        Update,
        (
            apply_free_sprite_effects,
            restore_tinted_sprites_without_effects,
            restore_sprites_without_effects,
            draw_sprite_effects,
        )
            .chain(),
    );
    app
}

fn a_32x16_sprite(app: &mut App) -> Sprite {
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    Sprite {
        image,
        custom_size: Some(Vec2::new(32.0, 16.0)),
        ..default()
    }
}

/// The entity's own scale survives the effect.
///
/// The mesh is a unit quad, so the draw writes `frame_size * scale` into
/// `Transform`. Restoring only the `Sprite` would leave the entity magnified by
/// its frame size, and everything that reads the transform would be wrong.
///
/// The initial scale is not unit, so "restore" and "leave it alone" give
/// different results.
#[test]
fn cancelling_a_shader_effect_gives_back_the_entitys_own_scale() {
    let mut app = fx_app();
    let sprite = a_32x16_sprite(&mut app);
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::HueShift { degrees: 137.5 },
            sprite,
            Transform::from_scale(Vec3::new(2.0, 3.0, 1.0)),
        ))
        .id();

    app.update();
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale.truncate(),
        Vec2::new(64.0, 48.0),
        "premise: the quad draws at frame size times the entity's own scale"
    );

    app.world_mut().entity_mut(entity).remove::<SpriteEffect>();
    app.update();
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale,
        Vec3::new(2.0, 3.0, 1.0),
        "the effect was removed; the entity keeps the scale it arrived with"
    );
}

/// The scale does not compound.
///
/// Without a restore, each on/off cycle multiplies the frame size again
/// (32 -> 1024 -> 32768), so a flickering effect sends the sprite off screen.
#[test]
fn re_applying_a_shader_effect_does_not_compound_the_scale() {
    let mut app = fx_app();
    let sprite = a_32x16_sprite(&mut app);
    let entity = app
        .world_mut()
        .spawn((SpriteEffect::HueShift { degrees: 90.0 }, sprite, Transform::default()))
        .id();

    for _ in 0..3 {
        app.update();
        app.world_mut().entity_mut(entity).remove::<SpriteEffect>();
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .insert(SpriteEffect::HueShift { degrees: 90.0 });
    }
    app.update();

    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale.truncate(),
        Vec2::new(32.0, 16.0),
        "three add/remove cycles must leave the quad at ONE frame size, not at \
         32^4 of it"
    );
}

/// A changed shader effect rebuilds from the original, not from the last draw.
///
/// The restore-then-redraw path inside `draw_sprite_effects` is the one that
/// runs while the entity keeps its effect, so it is the compounding case that
/// never passes through `restore_sprites_without_effects` at all.
#[test]
fn changing_one_shader_effect_for_another_does_not_compound_the_scale() {
    let mut app = fx_app();
    let sprite = a_32x16_sprite(&mut app);
    let entity = app
        .world_mut()
        .spawn((SpriteEffect::HueShift { degrees: 10.0 }, sprite, Transform::default()))
        .id();
    app.update();

    app.world_mut()
        .entity_mut(entity)
        .insert(SpriteEffect::HueShift { degrees: 200.0 });
    // One frame to restore, one for the pass above to redraw.
    app.update();
    app.update();

    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale.truncate(),
        Vec2::new(32.0, 16.0),
        "swapping effect A for effect B redrew from a transform effect A had \
         already written"
    );
}

/// A tint comes back off.
///
/// The free path writes `Sprite.color` in place. Without a record of the
/// previous colour, removing the effect would leave the sprite tinted.
#[test]
fn cancelling_a_tint_gives_back_the_sprites_own_colour() {
    let mut app = fx_app();
    let mut sprite = a_32x16_sprite(&mut app);
    sprite.color = Color::srgb(0.2, 0.4, 0.6);
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::Tint(Color::srgb(1.0, 0.0, 0.0)),
            sprite,
            Transform::default(),
        ))
        .id();

    app.update();
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0),
        "premise: the tint applied"
    );

    app.world_mut().entity_mut(entity).remove::<SpriteEffect>();
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().color,
        Color::srgb(0.2, 0.4, 0.6),
        "the sprite's authored colour, not the tint's"
    );
    assert!(
        app.world().get::<SpriteFxTinted>(entity).is_none(),
        "and the record goes with it, or the NEXT tint records this one as the \
         original"
    );
}

/// Crossing from the free path to the mesh path must not bake the tint in.
///
/// `draw_sprite_effects` stores `original: sprite.clone()`. If the tint is
/// still on the sprite then, every later restore gives back the tinted
/// sprite. So the free path removes its colour on the frame the effect stops
/// being a tint, before the mesh path runs.
#[test]
fn a_tint_replaced_by_a_shader_effect_is_not_baked_into_the_stored_original() {
    let mut app = fx_app();
    let mut sprite = a_32x16_sprite(&mut app);
    sprite.color = Color::srgb(0.2, 0.4, 0.6);
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::Tint(Color::srgb(1.0, 0.0, 0.0)),
            sprite,
            Transform::default(),
        ))
        .id();
    app.update();

    app.world_mut()
        .entity_mut(entity)
        .insert(SpriteEffect::HueShift { degrees: 90.0 });
    app.update();
    assert_eq!(
        app.world()
            .get::<SpriteFxDrawn>(entity)
            .expect("the hue shift took the sprite over")
            .original
            .color,
        Color::srgb(0.2, 0.4, 0.6),
        "the mesh path stored the TINTED sprite as the original"
    );

    app.world_mut().entity_mut(entity).remove::<SpriteEffect>();
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().color,
        Color::srgb(0.2, 0.4, 0.6),
        "and so the entity came back wearing a tint nothing asked for"
    );
}

/// The other crossing: shader to tint.
///
/// The tint path queries `&mut Sprite`, and the mesh path removes `Sprite`
/// while it owns the entity. So on the frame a `HueShift` becomes a `Tint`,
/// only `draw_sprite_effects`' changed-effect arm can give the sprite back.
///
/// `restore_sprites_without_effects` cannot help: it requires
/// `Without<SpriteEffect>`, and here the component is present with a different
/// variant. A sprite stranded this way stays invisible.
#[test]
fn a_shader_effect_replaced_by_a_tint_gives_the_sprite_back() {
    let mut app = fx_app();
    let mut sprite = a_32x16_sprite(&mut app);
    sprite.color = Color::srgb(0.2, 0.4, 0.6);
    let entity = app
        .world_mut()
        .spawn((
            SpriteEffect::HueShift { degrees: 90.0 },
            sprite,
            Transform::from_scale(Vec3::splat(2.0)),
        ))
        .id();
    app.update();
    assert!(
        app.world().get::<Sprite>(entity).is_none(),
        "the mesh path should have taken the Sprite over, or this test is not \
         exercising the crossing it claims to"
    );

    app.world_mut()
        .entity_mut(entity)
        .insert(SpriteEffect::Tint(Color::srgb(1.0, 0.0, 0.0)));
    // Two updates: one for the mesh path to hand the sprite back, one for the
    // free path to see it. ⚠ The one-frame gap is a real property of this
    // crossing and is asserted rather than papered over — a caller flickering
    // between the two paths draws one untinted frame.
    app.update();
    let handed_back = app.world().get::<Sprite>(entity).is_some();
    app.update();

    assert!(
        handed_back,
        "the mesh path did not give the Sprite back when the effect changed to a \
         free one, so the entity is stranded: invisible, still carrying a mesh, \
         and never matched by `restore_sprites_without_effects` because it still \
         has a `SpriteEffect`"
    );
    assert!(
        app.world().get::<SpriteFxDrawn>(entity).is_none(),
        "the mesh path's bookkeeping outlived the effect that created it"
    );
    assert_eq!(
        app.world().get::<Transform>(entity).unwrap().scale,
        Vec3::splat(2.0),
        "the entity's own scale did not come back across the crossing"
    );
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().color,
        Color::srgb(1.0, 0.0, 0.0),
        "the tint never landed after the crossing"
    );
}
