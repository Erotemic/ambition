"""Per-archetype toon preset definitions.

Each ``<name>.py`` here defines one ``PRESET`` dict for one toon
archetype. The ``PRESETS`` dict below collects them so the rig
can look up by archetype name. Adding a new archetype is a
single-file drop:

    1. Create `_toon_presets/<archetype>.py` with `PRESET = {...}`.
    2. Import + add to the `PRESETS` dict here.

Step 2 will become automatic once we collapse this dict into a
module-walk discovery (matching how `target_registry.py` walks
the tack-on targets dir). For now we keep the explicit dict so
the rig sees a stable ordering.
"""
from __future__ import annotations

from .general_hero import PRESET as _GENERAL_HERO
from .absurd_general import PRESET as _ABSURD_GENERAL
from .fascist_enforcer import PRESET as _FASCIST_ENFORCER
from .kernel_guide import PRESET as _KERNEL_GUIDE
from .merchant_prototype import PRESET as _MERCHANT_PROTOTYPE
from .vault_keeper import PRESET as _VAULT_KEEPER
from .architect import PRESET as _ARCHITECT
from .oiler import PRESET as _OILER
from .erdish import PRESET as _ERDISH
from .bob import PRESET as _BOB
from .alice import PRESET as _ALICE
from .eve import PRESET as _EVE
from .mallory import PRESET as _MALLORY
from .trent import PRESET as _TRENT
from .judy import PRESET as _JUDY
from .trudy import PRESET as _TRUDY
from .craig import PRESET as _CRAIG
from .sybil import PRESET as _SYBIL
from .victor import PRESET as _VICTOR
from .peggy import PRESET as _PEGGY
from .walter import PRESET as _WALTER
from .olivia import PRESET as _OLIVIA

PRESETS = {
    "general_hero": _GENERAL_HERO,
    "absurd_general": _ABSURD_GENERAL,
    "fascist_enforcer": _FASCIST_ENFORCER,
    "kernel_guide": _KERNEL_GUIDE,
    "merchant_prototype": _MERCHANT_PROTOTYPE,
    "vault_keeper": _VAULT_KEEPER,
    "architect": _ARCHITECT,
    "oiler": _OILER,
    "erdish": _ERDISH,
    "bob": _BOB,
    "alice": _ALICE,
    "eve": _EVE,
    "mallory": _MALLORY,
    "trent": _TRENT,
    "judy": _JUDY,
    "trudy": _TRUDY,
    "craig": _CRAIG,
    "sybil": _SYBIL,
    "victor": _VICTOR,
    "peggy": _PEGGY,
    "walter": _WALTER,
    "olivia": _OLIVIA,
}
