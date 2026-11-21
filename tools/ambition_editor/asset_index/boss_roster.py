"""Temporary boss entries for the character inspector."""
from pathlib import PurePosixPath


def entries(prepared: dict) -> list[dict]:
    # TODO(compat-remove): remove this bridge when bosses use prepared character
    # identities and the art export includes them in its character records.
    scope = prepared.get('scope') or {}
    images = scope.get('bosses') or {}
    names = {}
    for room in (scope.get('rooms') or {}).values():
        for spawn in room.get('spec', {}).get('boss_spawns', []):
            payload = spawn.get('payload') or {}
            if isinstance(payload, dict) and 'PhaseScript' in payload:
                key = payload['PhaseScript']['script_id']
                names.setdefault(key, spawn.get('name'))
    profiles = set()
    prefix = (prepared.get('provider') or 'unknown') + ':boss/'
    for reference in prepared.get('references', []):
        for value in (reference.get('from', ''), reference.get('to', '')):
            if value.startswith(prefix):
                profiles.add(value[len(prefix):])
    records = []
    for key in sorted(profiles | images.keys() | names.keys()):
        image = images.get(key)
        manifest = str(PurePosixPath('sprites') / PurePosixPath(image).with_suffix('.ron')) if image else ''
        display = key.removesuffix('_boss').replace('_', ' ').title()
        if names.get(key) and names[key].lower() != 'system boss':
            display = names[key]
        records.append({
            'id': 'boss/' + key, 'provider': prepared.get('provider') or 'unknown',
            'inspection_origin': 'boss', 'boss_id': key, 'boss_profile': key in profiles,
            'catalog': {'display_name': display, 'manifest': manifest, 'tags': ['boss'], 'rest_row': None},
            'sheet_target': PurePosixPath(image).stem.removesuffix('_spritesheet') if image else None,
            'required_actions': [], 'moves': [], 'capabilities_complete': False,
        })
    return records
