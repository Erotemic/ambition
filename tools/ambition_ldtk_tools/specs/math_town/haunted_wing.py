"""The haunted wing of Hylbert's Hotel. Use the Math-town generator to apply it."""

LANDING = "math_hotel_spectral_landing"
GALLERY = "math_hotel_eigen_gallery"
GAP = "math_hotel_spectral_gap"
SUITES = "math_hotel_shifted_suites"
SERVICE = "math_hotel_service_landing"
ARENA = "math_hotel_gram_reaper"
VAULT = "math_hotel_spectral_treasury"
SHORTCUT = "math_hotel_service_lift"
BOSS = "hylbert_hotel.gram_reaper"

# The side exits meet at world y 5104. Doors connect the hotel interiors.
ROOMS_AT = {
    LANDING: (8500, 1280, 640),
    GALLERY: (9780, 1920, 768),
    GAP: (11700, 1600, 960),
    SUITES: (13300, 1920, 768),
    SERVICE: (15220, 1120, 640),
    ARENA: (16340, 1600, 768),
    VAULT: (17940, 800, 640),
}
FLOORS = {rid: (576 if height == 640 else 704) for rid, (_, _, height) in ROOMS_AT.items()}
ROOMS_Y = {rid: 5104 - FLOORS[rid] for rid in ROOMS_AT}


def builders(kit):
    """Use the hotel family's room, doorway, and collision helpers."""

    def prop(name, kind, x, y, w, h):
        return (f'( type: "Prop", px: [{x}, {y}], size: [{w}, {h}], '
                f'fields: ( name: "{name}", kind: "{kind}" ) ),')

    def marble(name, x, y, w, *, one_way=False):
        # Each native tile is a still platform. Its art and its contact face agree.
        return [(f'( type: "MovingPlatform", px: [{px}, {y}], size: [{min(128, x + w - px)}, 32], '
                 f'fields: ( id: "{name}_{i}", name: "{name}_{i}", speed: 0.0, '
                 f'visual: "hylbert_floor_strip", one_way: {str(one_way).lower()} ) ),')
                for i, px in enumerate(range(x, x + w, 128))]

    def platform(name, x, y, w):
        return marble(name, x, y, w, one_way=True)

    def painted_door(zone, name, x, floor, to_room, to_zone):
        return [prop(f"{zone}_art", "hylbert_room_door", x - 12, floor - 126, 72, 126),
                kit.door(zone, name, x, floor, to_room, to_zone)]

    def enemy(rid, n, character, x, feet, w, h):
        return (f'( type: "EnemySpawn", px: [{x}, {feet - h}], size: [{w}, {h}], '
                f'fields: ( name: "{rid}_{n}", character_id: "{character}", '
                'brain: "combatant", respawn: "OnRoomReenter" ) ),')

    def chest(name, x, floor, amount):
        return (f'( type: "ChestSpawn", px: [{x}, {floor - 58}], size: [76, 58], '
                f'fields: ( name: "{name}", reward: "currency:{amount}" ) ),')

    def health(name, x, floor):
        return (f'( type: "PickupSpawn", px: [{x}, {floor - 28}], size: [24, 24], '
                f'fields: ( name: "{name}", kind: "health:2", respawn: "Never" ) ),')

    def shell(rid, spans=None):
        _, w, h = ROOMS_AT[rid]
        floor = FLOORS[rid]
        wall_height = floor - 32
        panel_width = wall_height * 3 // 4
        e = [prop(f"{rid}_wall_{i}", "hylbert_wallpaper_panel", x, 32,
                  min(panel_width, w - x), wall_height)
             for i, x in enumerate(range(0, w, panel_width))]
        e += [kit.solid("ceiling", 0, 0, w, 32),
              kit.solid("west_wall", 0, 32, 16, floor - 128),
              kit.solid("east_wall", w - 16, 32, 16, floor - 128)]
        for i, (left, right) in enumerate(spans if spans is not None else [(0, w)]):
            e += [kit.solid(f"floor_{i}", left, floor + 32, right - left, h - floor - 32),
                  *marble(f"{rid}_floor_{i}", left, floor, right - left)]
        for i, x in enumerate(range(320, w - 120, 320)):
            e.append(prop(f"{rid}_sconce_{i}", "hylbert_wall_sconce", x, floor - 248, 36, 54))
        e += [prop(f"{rid}_chandelier", "hylbert_chandelier", w // 2 - 80, 32, 160, 128),
              f'( type: "PlayerStart", px: [80, {floor - 48}], size: [28, 48], fields: ( name: "{rid}_spawn" ) ),']
        return e

    def join(rid, direction, other):
        _, w, _ = ROOMS_AT[rid]
        opposite = "west" if direction == "east" else "east"
        return kit.edge(f"{rid}_{direction}", "through the haunted wing",
                        w - 16 if direction == "east" else 0, FLOORS[rid] - 96, 96,
                        other, f"{other}_{opposite}")

    def finish(rid, e, title):
        return rid, kit.room(rid, e, title, theme="math_interior",
                             bg="#20142B", music="math_town_night")

    def landing():
        rid, floor = LANDING, FLOORS[LANDING]
        e = shell(rid)
        e += painted_door(f"{rid}_entry", "back to the occupied rooms", 80, floor,
                          kit.CORRIDOR, "math_hotel_corridor_spectral")
        e += painted_door(f"{rid}_service", "the service lift", 360, floor - 160,
                          SERVICE, f"{SERVICE}_lift")
        e += platform("service_lift_step", 224, floor - 80, 112)
        e += platform("service_lift_landing", 340, floor - 160, 112)
        e += [kit.solid("landing_west_wall", 0, floor - 96, 16, 96),
              kit.lock_wall(f"{rid}_lift_lock", "the service lift doors", 348, floor - 286, 72, 126,
                            f"world.switch_on {SHORTCUT}", "hylbert_room_door"),
              prop("abandoned_luggage", "hylbert_luggage_cart", 560, floor - 120, 128, 120),
              enemy(rid, 1, "eigenghost", 900, floor - 48, 48, 60),
              join(rid, "east", GALLERY),
              kit.label("wing_notice", 80, 240, "THE SPECTRAL WING / Service lift: locked from upstairs", 680)]
        return finish(rid, e, "The haunted floors. The guests have not checked out.")

    def gallery():
        rid, floor = GALLERY, FLOORS[GALLERY]
        e = shell(rid)
        e += [join(rid, "west", LANDING), join(rid, "east", GAP)]
        for i, (x, y, w) in enumerate([(280, 608, 160), (500, 512, 160), (720, 416, 160), (960, 384, 192)]):
            e += platform(f"gallery_ledge_{i}", x, y, w)
        e += [enemy(rid, 1, "decomposing_specter", 560, floor, 36, 70),
              enemy(rid, 2, "eigenghost", 900, 560, 48, 60),
              enemy(rid, 3, "decomposing_specter", 1230, floor, 36, 70),
              enemy(rid, 4, "spectral_perturbation", 1560, 520, 64, 62),
              chest("the guest's lost deposit", 1020, 384, 45),
              health("gallery_first_aid", 1760, floor)]
        return finish(rid, e, "The Eigen Gallery. The bellhops still offer room service.")

    def spectral_gap():
        rid, floor = GAP, FLOORS[GAP]
        e = shell(rid, [(0, 608), (992, 1600)])
        e += [join(rid, "west", GALLERY), join(rid, "east", SUITES),
              prop("gap_back_west", "hylbert_wallpaper_panel", 608, floor, 192, 256),
              prop("gap_back_east", "hylbert_wallpaper_panel", 800, floor, 192, 256),
              prop("the_spectral_gap", "hylbert_spectral_gap", 608, floor, 384, 216),
              *marble("gap_bottom", 608, 928, 384)]
        # The upper path crosses the gap. The lower steps let a fallen body return.
        for i, (x, y, w) in enumerate([(648, 640, 112), (824, 592, 112),
                                       (624, 848, 112), (752, 768, 112), (872, 688, 112)]):
            e += platform(f"gap_step_{i}", x, y, w)
        e += [enemy(rid, 1, "eigenghost", 610, 536, 48, 60),
              enemy(rid, 2, "spectral_perturbation", 1020, 500, 64, 62),
              enemy(rid, 3, "decomposing_specter", 1260, floor, 36, 70),
              chest("the missing eigenvalue", 760, 928, 70),
              kit.label("gap_notice", 80, 320, "THE SPECTRAL GAP / The hole keeps this floor stable.", 700)]
        return finish(rid, e, "The Spectral Gap. Mind the gap; it holds the space together.")

    def suites():
        rid, floor = SUITES, FLOORS[SUITES]
        e = shell(rid)
        e += [join(rid, "west", GAP), join(rid, "east", SERVICE)]
        for i, x in enumerate((360, 680, 1000, 1320, 1640)):
            e.append(prop(f"shifted_door_{i}", "hylbert_room_door", x, floor - 126, 72, 126))
        for i, (x, y, w) in enumerate([(320, 608, 160), (560, 512, 192), (880, 464, 192), (1200, 544, 192)]):
            e += platform(f"suite_balcony_{i}", x, y, w)
        e += [enemy(rid, 1, "decomposing_specter", 480, floor, 36, 70),
              enemy(rid, 2, "shift_operator", 760, 500, 48, 78),
              enemy(rid, 3, "shift_operator", 1180, 440, 48, 78),
              enemy(rid, 4, "decomposing_specter", 1480, floor, 36, 70),
              enemy(rid, 5, "eigenghost", 1700, 560, 48, 60),
              prop("suite_luggage", "hylbert_luggage_cart", 960, floor - 120, 128, 120),
              chest("the room n plus one gratuity", 1240, 544, 55)]
        return finish(rid, e, "The Shifted Suites. Housekeeping moves every guest one room along.")

    def service():
        rid, floor = SERVICE, FLOORS[SERVICE]
        e = shell(rid)
        e += [join(rid, "west", SUITES), join(rid, "east", ARENA),
              (f'( type: "Switch", px: [320, {floor - 32}], size: [24, 32], '
               f'fields: ( id: "{SHORTCUT}", name: "service lift breaker", prompt: "Restore the service lift", '
               'action: "ResetEncounter", target_encounter: "" ) ),'),
              health("service_first_aid", 560, floor),
              kit.label("reaper_notice", 480, 240, "GRAM REAPER / Final checkout", 540)]
        e += painted_door(f"{rid}_lift", "the service lift to the wing entrance", 160, floor,
                          LANDING, f"{LANDING}_service")
        return finish(rid, e, "The service landing. Restore the lift before the last guest.")

    def arena():
        rid, floor = ARENA, FLOORS[ARENA]
        e = shell(rid)
        e += [join(rid, "west", SERVICE), join(rid, "east", VAULT),
              kit.solid("treasury_lintel", 1408, 32, 72, floor - 158),
              kit.lock_wall("hylbert_reaper_treasury_seal", "the treasury seal", 1408, floor - 126, 72, 126,
                            f"boss.cleared {BOSS}", "hylbert_room_door"),
              (f'( type: "BossSpawn", px: [900, {floor - 184}], size: [110, 170], '
               f'fields: ( name: "Gram Reaper", brain: "PhaseScript:gram_reaper_boss", encounter_id: "{BOSS}" ) ),')]
        for i, (x, y, w) in enumerate([(224, 608, 160), (1184, 608, 160)]):
            e += platform(f"reaper_ledge_{i}", x, y, w)
        return finish(rid, e, "The Gram Reaper. The treasury opens after the final checkout.")

    def vault():
        rid, floor = VAULT, FLOORS[VAULT]
        e = shell(rid)
        e += [join(rid, "west", ARENA), kit.solid("vault_east_wall", 784, floor - 96, 16, 96),
              chest("the spectral wing's unclaimed deposits", 420, floor, 250),
              health("treasury_first_aid", 580, floor)]
        e += painted_door(f"{rid}_return", "the lift back to the wing entrance", 160, floor,
                          LANDING, f"{LANDING}_entry")
        # The return lift has no entrance at its destination.
        e[-1] = e[-1].replace("bidirectional: true", "bidirectional: false")
        return finish(rid, e, "The unclaimed deposits. The hotel can always take one more.")

    return landing, gallery, spectral_gap, suites, service, arena, vault
