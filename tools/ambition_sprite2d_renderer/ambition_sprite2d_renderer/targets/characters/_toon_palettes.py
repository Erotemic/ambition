"""Palette definitions for the toon adapter (see `toon_side.py`).

Extracted from `toon_side.py` to keep that file under 2500 lines —
adding a new palette is a one-block edit here without scrolling
through 800 lines of rig code. See GOALS.md goal #1 for the
larger direction (shared shape-primitive rig with declarative
per-character files).
"""

from __future__ import annotations

from typing import Tuple


Color = Tuple[int, int, int, int]


def rgba(value: str, alpha: int = 255) -> Color:
    # Re-defined here to keep this module standalone — toon_side.py
    # has the canonical definition but we mirror it so importing
    # this module doesn't require a circular dep on toon_side.
    from PIL import ImageColor

    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, alpha)


PALETTES = {
    "hero": {
        "skin": rgba("#F1C7A4"),
        "skin_shadow": rgba("#D09C77"),
        "hair": rgba("#423137"),
        "hair_shine": rgba("#614850"),
        "outfit": rgba("#3C6FF4"),
        "outfit_dark": rgba("#2448A5"),
        "accent": rgba("#FFBA49"),
        "accent_dark": rgba("#D07C1F"),
        "shoe": rgba("#2D2738"),
        "outline": rgba("#1B1B22"),
        "shadow": rgba("#000000", 36),
        "white": rgba("#FFF6E8"),
    },
    "guide": {
        "skin": rgba("#D8C6B1"),
        "skin_shadow": rgba("#BAA189"),
        "hair": rgba("#32404B"),
        "hair_shine": rgba("#556774"),
        "outfit": rgba("#5D9BA3"),
        "outfit_dark": rgba("#396771"),
        "accent": rgba("#EAD27A"),
        "accent_dark": rgba("#C8AB44"),
        "shoe": rgba("#2F353A"),
        "outline": rgba("#1C1F22"),
        "shadow": rgba("#000000", 38),
        "white": rgba("#FFF6E8"),
    },
    "merchant": {
        "skin": rgba("#E2C09B"),
        "skin_shadow": rgba("#BB9471"),
        "hair": rgba("#5B3F2E"),
        "hair_shine": rgba("#7D5A45"),
        "outfit": rgba("#8A5D3C"),
        "outfit_dark": rgba("#5D3A23"),
        "accent": rgba("#D8B464"),
        "accent_dark": rgba("#A57A2D"),
        "shoe": rgba("#342A27"),
        "outline": rgba("#1E1B1A"),
        "shadow": rgba("#000000", 40),
        "white": rgba("#FFF6E8"),
    },
    "architect": {
        "skin": rgba("#CFBDB4"),
        "skin_shadow": rgba("#A48E85"),
        "hair": rgba("#24262B"),
        "hair_shine": rgba("#4A4F57"),
        "outfit": rgba("#6B4FD7"),
        "outfit_dark": rgba("#43318D"),
        "accent": rgba("#79D5E8"),
        "accent_dark": rgba("#3C98B0"),
        "shoe": rgba("#2A2732"),
        "outline": rgba("#17161F"),
        "shadow": rgba("#000000", 40),
        "white": rgba("#FFF6E8"),
    },
    "keeper": {
        "skin": rgba("#DDB69A"),
        "skin_shadow": rgba("#B88E71"),
        "hair": rgba("#F2E8D8"),
        "hair_shine": rgba("#FFF9EE"),
        "outfit": rgba("#6F303B"),
        "outfit_dark": rgba("#4A1E24"),
        "accent": rgba("#E1C66F"),
        "accent_dark": rgba("#BA9E45"),
        "shoe": rgba("#322730"),
        "outline": rgba("#1C171C"),
        "shadow": rgba("#000000", 42),
        "white": rgba("#FFF6E8"),
    },
    "absurd_general": {
        "skin": rgba("#E0A27F"),
        "skin_shadow": rgba("#B8725F"),
        "hair": rgba("#3A2E23"),
        "hair_shine": rgba("#67513C"),
        "outfit": rgba("#355F32"),
        "outfit_dark": rgba("#1D3421"),
        "accent": rgba("#FFD34F"),
        "accent_dark": rgba("#B77814"),
        "shoe": rgba("#201D17"),
        "outline": rgba("#181512"),
        "shadow": rgba("#000000", 46),
        "white": rgba("#FFF0DB"),
    },
    "raid_enforcer": {
        "skin": rgba("#D9C1AF"),
        "skin_shadow": rgba("#B99683"),
        "hair": rgba("#D6D0C2"),
        "hair_shine": rgba("#F1EBDD"),
        "outfit": rgba("#383B42"),
        "outfit_dark": rgba("#17191E"),
        "accent": rgba("#C02632"),
        "accent_dark": rgba("#78141D"),
        "shoe": rgba("#101114"),
        "outline": rgba("#0A0B0E"),
        "shadow": rgba("#000000", 52),
        "white": rgba("#F1ECE2"),
    },
    # Oiler — gate mechanic with an 18th-century-mathematician streak.
    # The name is a pun on "Euler" (pronounced "oiler") so the
    # silhouette leans powdered-wig + workshop apron: pale cream
    # wig hair with shine, grimy olive coveralls under a leather
    # apron, rust-orange accent for the tied wig ribbon and tool
    # trim. Reads as "person who fixes pipes for a living and
    # also keeps a notebook of theorems in the satchel."
    "oiler": {
        "skin": rgba("#E6C4A4"),
        "skin_shadow": rgba("#B8957A"),
        # Hair is mostly hidden by the savant_cap; the values still
        # need to be set in case the head-back branch ever runs.
        "hair": rgba("#9E9784"),
        "hair_shine": rgba("#C9C2AE"),
        # Silk banyan: warm rust-brocade body with a darker
        # rust-brown lapel/sash and a brighter ochre paisley dot.
        # Skin is warm and contrasts the silk so the silhouette
        # reads "robed scholar holding a wrench."
        "outfit": rgba("#7C3D24"),
        "outfit_dark": rgba("#46210F"),
        "accent": rgba("#E3A24B"),
        "accent_dark": rgba("#9A5A1B"),
        # Soft cloth cap (Handmann turban) and its darker hem band.
        "cap": rgba("#C9B98F"),
        "cap_band": rgba("#7F5E2B"),
        "shoe": rgba("#211E1A"),
        "outline": rgba("#14110E"),
        "shadow": rgba("#000000", 46),
        "white": rgba("#FBEAC9"),
    },
    # Erdish — wandering graph-theory eccentric. Dusty travel coat,
    # ink-stained satchel, ash-violet trim. Hair is gray and
    # combed back, slightly receding, slightly wild — "open mind,
    # possibly also open door."
    "erdish": {
        "skin": rgba("#D6C2B0"),
        "skin_shadow": rgba("#A78F7B"),
        "hair": rgba("#8A8694"),
        "hair_shine": rgba("#CFCAD6"),
        "outfit": rgba("#3F394A"),
        "outfit_dark": rgba("#1F1C28"),
        "accent": rgba("#C9B6FF"),
        "accent_dark": rgba("#7E6BBF"),
        "shoe": rgba("#221E27"),
        "outline": rgba("#12111A"),
        "shadow": rgba("#000000", 40),
        "white": rgba("#F5EFE2"),
    },
    # Cryptography crew — Bob/Alice/Eve/Mallory/Trent/Judy. The
    # palettes lock the silhouette read at first glance even when
    # two characters share a body plan or hair primitive: e.g.
    # Trent and Judy are both `broad` robed figures, but Trent's
    # forest-and-gold reads "council" while Judy's black-and-
    # crimson reads "courtroom."
    # Bob — practical key engineer. Warm tan workshop vest, slate
    # blue undershirt, safety-yellow trim so the silhouette pops
    # against any background.
    "bob": {
        "skin": rgba("#D8AF8F"),
        "skin_shadow": rgba("#A88567"),
        "hair": rgba("#3D2B22"),
        "hair_shine": rgba("#6A4B3A"),
        "outfit": rgba("#9D7548"),
        "outfit_dark": rgba("#6A4C2A"),
        "accent": rgba("#F2C752"),
        "accent_dark": rgba("#A47616"),
        "shoe": rgba("#231C16"),
        "outline": rgba("#1A130E"),
        "shadow": rgba("#000000", 42),
        "white": rgba("#FBF0DC"),
    },
    # Alice — cryptographer. Deep teal tabard with a cream cipher-
    # checker placket and black-white check accents (one-time pad).
    "alice": {
        "skin": rgba("#E5C5A6"),
        "skin_shadow": rgba("#B3936F"),
        "hair": rgba("#1C1C24"),
        "hair_shine": rgba("#3A3A4A"),
        "outfit": rgba("#1B5E6E"),
        "outfit_dark": rgba("#0F3A47"),
        "accent": rgba("#F0EAD2"),
        "accent_dark": rgba("#A89C7A"),
        "shoe": rgba("#1B1F22"),
        "outline": rgba("#10141A"),
        "shadow": rgba("#000000", 40),
        "white": rgba("#F8F2E0"),
    },
    # Eve — eavesdropper. Deep aubergine cloak + slate undertones;
    # warm-cream listening horn brass color so the prop pops.
    "eve": {
        "skin": rgba("#D6BBA6"),
        "skin_shadow": rgba("#A88E78"),
        "hair": rgba("#2A1F2F"),
        "hair_shine": rgba("#54405E"),
        "outfit": rgba("#3D2A4A"),
        "outfit_dark": rgba("#22162D"),
        "accent": rgba("#D9B26A"),
        "accent_dark": rgba("#8E6F2D"),
        "shoe": rgba("#171019"),
        "outline": rgba("#0E0913"),
        "shadow": rgba("#000000", 46),
        "white": rgba("#EFE6D5"),
    },
    # Mallory — malicious attacker. Oxblood + black + chrome. NOT
    # cartoon-evil; reads more like "competent threat-actor in
    # tactical streetwear" than mustache-twirler.
    "mallory": {
        "skin": rgba("#D6A38B"),
        "skin_shadow": rgba("#A07560"),
        "hair": rgba("#B22531"),
        "hair_shine": rgba("#E04956"),
        "outfit": rgba("#1F1A1E"),
        "outfit_dark": rgba("#0B0A0C"),
        "accent": rgba("#8B1A23"),
        "accent_dark": rgba("#4F0A10"),
        "shoe": rgba("#080709"),
        "outline": rgba("#070608"),
        "shadow": rgba("#000000", 54),
        "white": rgba("#E8E1D6"),
    },
    # Trent — trusted arbitrator. Forest green formal robe with
    # brushed gold trim — secular council energy, not wizardly.
    "trent": {
        "skin": rgba("#C9A78B"),
        "skin_shadow": rgba("#9B7C63"),
        "hair": rgba("#E4DCC8"),
        "hair_shine": rgba("#F4EFDC"),
        "outfit": rgba("#264E32"),
        "outfit_dark": rgba("#142A1B"),
        "accent": rgba("#CBA653"),
        "accent_dark": rgba("#866820"),
        "shoe": rgba("#181612"),
        "outline": rgba("#0C0F0C"),
        "shadow": rgba("#000000", 44),
        "white": rgba("#F5EEDA"),
    },
    # Judy — the judge. Black judicial robe + crimson trim + white
    # jabot collar + powdered barrister wig. The black-on-white
    # contrast is what locks the read at small render scales.
    "judy": {
        "skin": rgba("#D6B89E"),
        "skin_shadow": rgba("#A88B73"),
        # Wig is intentionally a different cream than Newton's so
        # they don't read as the same character at a glance.
        "hair": rgba("#EFE6D2"),
        "hair_shine": rgba("#FAF4E2"),
        "outfit": rgba("#16141A"),
        "outfit_dark": rgba("#080608"),
        "accent": rgba("#9D1F2A"),
        "accent_dark": rgba("#5F0E15"),
        "shoe": rgba("#0A090C"),
        "outline": rgba("#050306"),
        "shadow": rgba("#000000", 56),
        "white": rgba("#F8F2E2"),
    },
    # ─── Crypto crew batch 2 ─────────────────────────────────────
    # Phenotype variation is deliberate: the previously-landed
    # crew (Bob/Alice/Eve/Mallory/Trent/Judy) clusters around
    # warm-tan skin tones. Batch 2 deliberately spreads across
    # the human skin-tone range from very pale to very dark,
    # with distinct hair colors + textures, so the cast
    # actually looks like a crew rather than seven shades of
    # the same person.
    # Trudy (intruder) — warm-tan East Asian phenotype, jet
    # black short hair, slate field jacket.
    "trudy": {
        "skin": rgba("#D9B190"),
        "skin_shadow": rgba("#A88260"),
        "hair": rgba("#0E0B12"),
        "hair_shine": rgba("#36303A"),
        "outfit": rgba("#2E3A2A"),
        "outfit_dark": rgba("#19211B"),
        "accent": rgba("#C49A4A"),
        "accent_dark": rgba("#7A5B22"),
        "shoe": rgba("#171814"),
        "outline": rgba("#0A0C0A"),
        "shadow": rgba("#000000", 44),
        "white": rgba("#F2EBDA"),
    },
    # Craig (cracker/safe-cracker) — pale freckled European
    # phenotype, weathered older man, faded auburn hair,
    # denim + wool palette.
    "craig": {
        "skin": rgba("#ECCBB0"),
        "skin_shadow": rgba("#C09980"),
        "hair": rgba("#7A4630"),
        "hair_shine": rgba("#A56B4A"),
        "outfit": rgba("#3F5C7A"),
        "outfit_dark": rgba("#22364B"),
        "accent": rgba("#C8A878"),
        "accent_dark": rgba("#8A7048"),
        "shoe": rgba("#221A14"),
        "outline": rgba("#161513"),
        "shadow": rgba("#000000", 40),
        "white": rgba("#F8EFDC"),
    },
    # Sybil (pseudonymous attacker) — rich deep brown skin
    # with cool undertone, hair worn in many small black
    # braids, layered patchwork colors.
    "sybil": {
        "skin": rgba("#6B4530"),
        "skin_shadow": rgba("#3F271A"),
        "hair": rgba("#0A0A10"),
        "hair_shine": rgba("#2D2A36"),
        "outfit": rgba("#5C2E7A"),  # plum base
        "outfit_dark": rgba("#321648"),
        "accent": rgba("#E6A14E"),  # contrasting marigold
        "accent_dark": rgba("#A36A1F"),
        "shoe": rgba("#1A1418"),
        "outline": rgba("#0A0710"),
        "shadow": rgba("#000000", 48),
        "white": rgba("#F4ECDA"),
    },
    # Victor (verifier) — olive Mediterranean phenotype, sharp
    # angular features, black square-fringe hair, cool slate
    # blazer palette with chrome accents.
    "victor": {
        "skin": rgba("#B58B6E"),
        "skin_shadow": rgba("#7E5C44"),
        "hair": rgba("#1A1820"),
        "hair_shine": rgba("#3E3A48"),
        "outfit": rgba("#3F4A5C"),  # slate blue blazer
        "outfit_dark": rgba("#22293A"),
        "accent": rgba("#B8C0CA"),  # brushed chrome
        "accent_dark": rgba("#727B86"),
        "shoe": rgba("#16181E"),
        "outline": rgba("#0C0F14"),
        "shadow": rgba("#000000", 42),
        "white": rgba("#F1F3F6"),
    },
    # Peggy (prover) — rich brown South-Asian/Latin phenotype,
    # black ponytail, athletic energy. Bright orange-and-cream
    # palette signals "moving + demonstrating things."
    "peggy": {
        "skin": rgba("#97694A"),
        "skin_shadow": rgba("#5C3F2A"),
        "hair": rgba("#13100E"),
        "hair_shine": rgba("#3A302A"),
        "outfit": rgba("#D8662A"),  # warm orange
        "outfit_dark": rgba("#8A3B15"),
        "accent": rgba("#F2E5C2"),  # cream
        "accent_dark": rgba("#B8A87E"),
        "shoe": rgba("#1F1612"),
        "outline": rgba("#100A08"),
        "shadow": rgba("#000000", 42),
        "white": rgba("#FBF1D7"),
    },
    # Walter (warden) — medium-cool European tan with silver
    # hair (older). Deep navy long coat + brass accents.
    "walter": {
        "skin": rgba("#C2A48B"),
        "skin_shadow": rgba("#8C7158"),
        "hair": rgba("#B4B0B0"),  # silver
        "hair_shine": rgba("#E2DEDE"),
        "outfit": rgba("#1A2438"),  # deep navy
        "outfit_dark": rgba("#0B1020"),
        "accent": rgba("#C49A56"),  # brass
        "accent_dark": rgba("#7E5E24"),
        "shoe": rgba("#0E0E14"),
        "outline": rgba("#06080E"),
        "shadow": rgba("#000000", 48),
        "white": rgba("#EFE8D2"),
    },
    # Olivia (oracle) — very pale Northern European phenotype,
    # nearly-white platinum hair, lavender + silver layered
    # robe. Ethereal, quiet.
    "olivia": {
        "skin": rgba("#F2DDC6"),
        "skin_shadow": rgba("#C8AE96"),
        "hair": rgba("#F4EEDC"),  # platinum/white
        "hair_shine": rgba("#FFFAEC"),
        "outfit": rgba("#9A8FB0"),  # lavender
        "outfit_dark": rgba("#5E5470"),
        "accent": rgba("#D6D2E4"),  # silver-lavender
        "accent_dark": rgba("#888098"),
        "shoe": rgba("#28242E"),
        "outline": rgba("#1B1822"),
        "shadow": rgba("#000000", 36),
        "white": rgba("#FAF6E6"),
    },
}
