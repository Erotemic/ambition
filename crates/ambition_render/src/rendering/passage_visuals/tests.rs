use super::*;
use ambition_platformer2d_core as ae;
use ambition_sim_view::{ActorAnimFrame, ActorAnimIndex, FeatureView, FeatureViewIndex, PassageLookView};

const LOOK: PassageLookView = PassageLookView { scale: 0.5, alpha: 0.4, shade: 0.5, transit: 0.0 };

fn actor_view(height: f32) -> FeatureView {
    FeatureView {
        pos: ae::Vec2::ZERO,
        size: ae::Vec2::new(20.0, height),
        kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        visible: true,
        submerged: false,
        wire_anchor: None,
        grab_reach: None,
        line_anchor: None,
        limb_host: None,
        depth_plane: ae::DepthPlane::PLAYABLE,
        flash: false,
        breakable_state: None,
        chest_opened: false,
        fighting: false,
        switch_on: false,
        rotation_rad: 0.0,
        alive: true,
        hit_flash_secs: 0.0,
        parry_flash_secs: 0.0,
        hp_current: 1,
        hp_max: 1,
        training_dummy: false,
        hit_strength: 0.0,
        unhittable: false,
        defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
        sprite_offset: None,
        room: None,
    }
}

fn frame(passage_look: Option<PassageLookView>) -> ActorAnimFrame {
    ActorAnimFrame {
        anim: ambition_sprite_sheet::character::CharacterAnim::Idle,
        conversation_held: false,
        pos: ae::Vec2::ZERO,
        facing: 1.0,
        gravity_dir: ae::DEFAULT_GRAVITY_DIR,
        clip: None,
        move_charge: None,
        passage_look,
        stop: Default::default(),
        turn: Default::default(),
    }
}

fn drawn(app: &App, body: Entity) -> (Vec3, f32, Srgba) {
    let transform = app.world().get::<Transform>(body).expect("a transform");
    let sprite = app.world().get::<Sprite>(body).expect("a sprite");
    (transform.scale, transform.translation.y, sprite.color.to_srgba())
}

/// Review finding #3 (2026-10-19). A body drawn on the actor road (a
/// possessed body, an NPC) in a passage is drawn at its passage's size,
/// opacity and shade, about its feet; when the passage is over it is drawn at
/// its own size again. Measured before: the look was applied to player bodies
/// only, so a possessed body walked through a door at full size. Poison:
/// filter the actor query out.
#[test]
fn a_body_drawn_as_a_feature_takes_its_passage_look() {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::test_support::shared_sprites());
    app.add_systems(Update, apply_passage_looks);
    app.insert_resource(ActorAnimIndex::from_rows([("npc".to_string(), frame(Some(LOOK)))]));
    app.insert_resource(FeatureViewIndex::from_rows([("npc".to_string(), actor_view(40.0))]));
    let body = app
        .world_mut()
        .spawn((FeatureVisual { id: "npc".to_string() }, Transform::from_xyz(0.0, 100.0, 0.0), Sprite::default()))
        .id();
    app.update();
    let (scale, y, color) = drawn(&app, body);
    assert_eq!(scale, Vec3::new(0.5, 0.5, 1.0), "the body is drawn at its passage's size");
    assert!((y - 90.0).abs() < 1e-4, "about its feet: half of the 40 px body, halved, is 10 px down; y is {y}");
    assert!((color.alpha - 0.4).abs() < 1e-4 && (color.red - 0.5).abs() < 1e-4, "opacity and shade: {color:?}");

    // The passage is over. `sync_visuals` places the body again each frame.
    app.insert_resource(ActorAnimIndex::from_rows([("npc".to_string(), frame(None))]));
    app.world_mut().get_mut::<Transform>(body).expect("a transform").translation.y = 100.0;
    app.update();
    let (scale, y, _) = drawn(&app, body);
    assert_eq!((scale, y), (Vec3::ONE, 100.0), "after the passage the body is drawn at its own size");
    assert!(app.world().get::<PassageScaled>(body).is_none());
}

/// Control: a player body still takes the look beside its pose.
#[test]
fn a_player_body_takes_its_passage_look() {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::test_support::shared_sprites());
    app.add_systems(Update, apply_passage_looks);
    let pose = ambition_sim_view::BodyPoseView { size: ae::Vec2::new(20.0, 40.0), ..Default::default() };
    let body = app
        .world_mut()
        .spawn((PlayerVisual, pose, LOOK, Transform::from_xyz(0.0, 100.0, 0.0), Sprite::default()))
        .id();
    app.update();
    let (scale, y, color) = drawn(&app, body);
    assert_eq!(scale, Vec3::new(0.5, 0.5, 1.0));
    assert!((y - 90.0).abs() < 1e-4 && (color.alpha - 0.4).abs() < 1e-4, "y {y}, {color:?}");
}

/// Review finding #7, the actor road: a body drawn as a feature that its
/// passage fades is declared as read as one image; one at full opacity is
/// not. Poison: declare player bodies only.
#[test]
fn a_feature_body_its_passage_fades_is_declared_as_one_image() {
    use ambition_sprite_sheet::character::rigged::ComposedBodyDemand;
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::test_support::shared_sprites());
    app.init_resource::<ComposedBodyDemand>();
    app.add_systems(Update, declare_passage_fade_demand);
    let opaque = PassageLookView { alpha: 1.0, ..LOOK };
    app.insert_resource(ActorAnimIndex::from_rows([
        ("fading".to_string(), frame(Some(LOOK))),
        ("opaque".to_string(), frame(Some(opaque))),
        ("outside".to_string(), frame(None)),
    ]));
    let body = |app: &mut App, id: &str| app.world_mut().spawn(FeatureVisual { id: id.to_string() }).id();
    let (fading, opaque, outside) = (body(&mut app, "fading"), body(&mut app, "opaque"), body(&mut app, "outside"));
    app.update();
    let demand = app.world().resource::<ComposedBodyDemand>();
    assert_eq!(
        [fading, opaque, outside].map(|body| demand.is_declared(body)),
        [true, false, false],
        "(fading, opaque, outside a passage)"
    );
}

/// A new room may request its door animation on the same update on which
/// the room look inserts `EntityArt`. Capture only after that art is bound.
#[test]
fn a_new_hub_door_opens_with_its_room_art_and_restores_it() {
    use ambition_sprite_sheet::game_assets::{EntitySprite, GameAssets};
    use super::super::world::{apply_entity_art, BoundEntitySprite};

    #[derive(Resource)]
    struct WantedArt(EntitySprite);
    #[derive(Resource, Default)]
    struct NewDoor(Option<Entity>);

    fn spawn_new_door(
        mut commands: Commands,
        mut born: ResMut<NewDoor>,
        assets: Res<GameAssets>,
    ) {
        if born.0.is_some() {
            return;
        }
        let ordinary = assets.entities.get(EntitySprite::DoorZone).unwrap().clone();
        let door = commands.spawn((
            LoadingZoneVisual { id: "hub_entry".into() },
            Sprite::from_image(ordinary),
            BoundEntitySprite { key: EntitySprite::DoorZone },
        )).id();
        born.0 = Some(door);
    }

    fn dress_new_door(
        mut commands: Commands,
        wanted: Res<WantedArt>,
        doors: Query<Entity, (With<LoadingZoneVisual>, Without<EntityArt>)>,
    ) {
        for door in &doors {
            commands.entity(door).try_insert(EntityArt(wanted.0));
        }
    }

    for kind in [EntitySprite::DoorStone, EntitySprite::DoorVoxel] {
        let mut app = door_test_app();
        add_door_art_catalog(&mut app);
        app.insert_resource(WantedArt(kind));
        app.init_resource::<NewDoor>();
        // `chain` flushes deferred commands between the four stages ordered
        // by the shipped composition. The door is born on this update.
        app.add_systems(Update, (spawn_new_door, dress_new_door, apply_entity_art, sync_door_openings).chain());

        let zone = "hub_entry";
        let intended = app.world().resource::<GameAssets>().entities.get(kind).unwrap().clone();
        set_open_door(&mut app, zone);
        app.update();
        let door = app.world().resource::<NewDoor>().0.expect("a door spawned in Update");

        let opening = app.world().get::<DoorOpening>(door).expect("an opening on the first entry update");
        assert_eq!(opening.closed, intended, "the first opening captures {kind:?}");
        let strip = opening.strip.clone();
        assert!(
            image_path(&app, &strip).ends_with(match kind {
                EntitySprite::DoorStone => "door_stone_opening.png",
                _ => "door_voxel_opening.png",
            }),
            "the strip must match the look art",
        );
        put_strip_image(&mut app, &strip);
        app.update();
        let sprite = app.world().get::<Sprite>(door).unwrap();
        assert_eq!(sprite.image, strip, "the matching strip plays");
        assert_eq!(sprite.texture_atlas.as_ref().unwrap().index, 5, "fully open is the last frame");

        app.insert_resource(ambition_sim_view::PassageViews::default());
        app.update();
        assert_eq!(app.world().get::<Sprite>(door).unwrap().image, intended, "closing restores the current look");
        assert!(app.world().get::<DoorOpening>(door).is_none());
        assert!(app.world().get::<Sprite>(door).unwrap().texture_atlas.is_none());
    }
}

/// A front moving across an open door must change the active strip and the
/// image restored on close. The old snapshot must not survive the change.
#[test]
fn changing_a_door_from_pure_to_corrupt_while_open_updates_its_strip() {
    use ambition_sprite_sheet::game_assets::{EntitySprite, GameAssets};
    use super::super::world::{apply_entity_art, BoundEntitySprite};

    let mut app = door_test_app();
    add_door_art_catalog(&mut app);
    app.add_systems(Update, (apply_entity_art, sync_door_openings).chain());
    let zone = "hub_entry";
    let pure = app.world().resource::<GameAssets>().entities.get(EntitySprite::DoorStone).unwrap().clone();
    let corrupt = app.world().resource::<GameAssets>().entities.get(EntitySprite::DoorVoxel).unwrap().clone();
    let door = app.world_mut().spawn((
        LoadingZoneVisual { id: zone.into() },
        Sprite::from_image(pure.clone()),
        BoundEntitySprite { key: EntitySprite::DoorZone },
        EntityArt(EntitySprite::DoorStone),
    )).id();
    set_open_door(&mut app, zone);
    app.update();
    let first_strip = app.world().get::<DoorOpening>(door).unwrap().strip.clone();
    put_strip_image(&mut app, &first_strip);
    app.update();
    assert_eq!(app.world().get::<Sprite>(door).unwrap().image, first_strip);

    // The room's advancing front changes art while the old strip plays.
    app.world_mut().entity_mut(door).insert(EntityArt(EntitySprite::DoorVoxel));
    app.update();
    let opening = app.world().get::<DoorOpening>(door).unwrap();
    assert_eq!(opening.closed, corrupt);
    assert!(image_path(&app, &opening.strip).ends_with("door_voxel_opening.png"));
    let next_strip = opening.strip.clone();
    put_strip_image(&mut app, &next_strip);
    app.update();
    assert_eq!(app.world().get::<Sprite>(door).unwrap().image, next_strip);

    app.insert_resource(ambition_sim_view::PassageViews::default());
    app.update();
    assert_eq!(app.world().get::<Sprite>(door).unwrap().image, corrupt);
    assert!(app.world().get::<DoorOpening>(door).is_none());
}

/// A themed door has no `EntityArt` or entity-sheet binding once dressed.
/// The door animation must still follow its theme's current closed image.
#[test]
fn a_themed_door_restores_the_current_theme_image() {
    let mut app = door_test_app();
    app.add_systems(Update, sync_door_openings);
    let assets = app.world().resource::<AssetServer>().clone();
    let theme = assets.load::<Image>("sprites/dressing/forest/door.png");
    let replacement = assets.load::<Image>("sprites/dressing/cave/door.png");
    let zone = "theme_entry";
    let door = app.world_mut().spawn((LoadingZoneVisual { id: zone.into() }, Sprite::from_image(theme.clone()))).id();
    set_open_door(&mut app, zone);
    app.update();
    assert_eq!(app.world().get::<DoorOpening>(door).unwrap().closed, theme);
    let strip = app.world().get::<DoorOpening>(door).unwrap().strip.clone();
    put_strip_image(&mut app, &strip);
    app.update();
    assert_eq!(app.world().get::<Sprite>(door).unwrap().image, strip);

    app.world_mut().get_mut::<Sprite>(door).unwrap().image = replacement.clone();
    app.insert_resource(ambition_sim_view::PassageViews::default());
    app.update();
    assert_eq!(app.world().get::<Sprite>(door).unwrap().image, replacement);
    assert!(app.world().get::<DoorOpening>(door).is_none());
}

fn door_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::test_support::shared_sprites());
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
    app.insert_resource(ambition_sim_view::PassageViews::default());
    app
}

fn set_open_door(app: &mut App, zone: &str) {
    app.insert_resource(ambition_sim_view::PassageViews {
        doors: vec![(None, zone.to_string(), 1.0)],
        ..Default::default()
    });
}

fn image_path(app: &App, image: &Handle<Image>) -> String {
    app.world()
        .resource::<AssetServer>()
        .get_path(image)
        .expect("the image has a source path")
        .path()
        .to_string_lossy()
        .into_owned()
}

fn put_strip_image(app: &mut App, strip: &Handle<Image>) {
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d { width: 60, height: 10, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        &[255, 255, 255, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    app.world_mut().resource_mut::<Assets<Image>>().insert(strip, image).expect("the opening strip is writable");
}

fn add_door_art_catalog(app: &mut App) {
    app.init_resource::<ambition_sprite_sheet::SpriteContent>();
    use ambition_asset_manager::{platformer_assets::Platformer2dAssetCatalog, AmbitionAssetCatalog, AssetProfile};
    use ambition_sprite_sheet::game_assets::{load_entity_sprites, GameAssets};

    let catalog = Platformer2dAssetCatalog::new(
        AmbitionAssetCatalog::new(ambition_sprite_sheet::test_support::shared_libraries().asset_manifest().unwrap()),
        AssetProfile::AndroidBundle,
    ).with_libraries(&ambition_sprite_sheet::test_support::shared_libraries()).unwrap();
    let assets = app.world().resource::<AssetServer>().clone();
    let entities = load_entity_sprites(&catalog, &assets, None);
    app.insert_resource(GameAssets { entities, ..Default::default() });
}
