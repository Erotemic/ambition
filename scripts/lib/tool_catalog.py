"""Read command descriptions from their owners. This catalog grants no authority."""
from __future__ import annotations

import json
import subprocess
import tomllib
from pathlib import Path


def commands(repo: Path) -> list[dict]:
    rows = []
    descriptors = [repo / 'scripts/commands.json', *sorted((repo / 'scripts').glob('*/commands.json')),
                   *sorted((repo / 'tools').glob('*/commands.json')),
                   repo / 'game/ambition_app_tools/commands.json']
    for descriptor in descriptors:
        if not descriptor.is_file():
            continue
        for entry in json.loads(descriptor.read_text())['commands']:
            row = dict(entry, descriptor=descriptor.relative_to(repo).as_posix())
            missing = [path for path in row['requires'] if not (repo / path).is_file()]
            row['availability'] = 'missing source: ' + ', '.join(missing) if missing else 'source present; runtime prerequisites not checked'
            if row.get('binary'):
                row['existing_binaries'] = [f'target/{profile}/{row["binary"]}' for profile in ('debug', 'release')
                                            if (repo / 'target' / profile / row['binary']).is_file()]
                if row['existing_binaries']:
                    binary = max(row['existing_binaries'], key=lambda path: (repo / path).stat().st_mtime_ns)
                    for key in ('command', 'help'):
                        row[key] = row[key].replace(f'target/debug/{row["binary"]}', binary)
                    row['availability'] += '; existing binary may be stale'
                else:
                    row['availability'] += '; no existing binary; build explicitly'
            rows.append(row)
    return rows


def inventory(repo: Path) -> dict:
    """List command candidates and path consumers before a physical move."""
    supported = commands(repo)
    scripts = []
    tracked = subprocess.run(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '--', 'scripts/'],
                             cwd=repo, text=True, capture_output=True)
    paths = {repo / path for path in tracked.stdout.splitlines()} if tracked.returncode == 0 else set((repo / 'scripts').rglob('*'))
    for path in sorted(paths):
        if not path.is_file() or path.suffix not in {'.py', '.sh', '.cjs'} or any(part in {'tests', 'lib', '__pycache__'} for part in path.relative_to(repo / 'scripts').parts):
            continue
        relative = path.relative_to(repo).as_posix()
        text = path.read_text(errors='replace')
        scripts.append({'path': relative, 'status': 'active' if any(relative in row['requires'] for row in supported) else ('experimental' if 'experimental' in path.relative_to(repo / 'scripts').parts else 'internal'),
                        'path_risks': [token for token in ('__file__', 'sys.path', 'scripts/lib', 'Path.cwd()', 'os.getcwd') if token in text]})
    manifest = repo / 'game/ambition_app_tools/Cargo.toml'
    config = tomllib.loads(manifest.read_text()) if manifest.is_file() else {}
    bins = list(config.get('bin', []))
    if config.get('package', {}).get('autobins', True):
        named = {row['name'] for row in bins}
        bin_dir = manifest.parent / 'src/bin'
        for source in sorted([*bin_dir.glob('*.rs'), *bin_dir.glob('*/main.rs')]):
            name = source.parent.name if source.name == 'main.rs' else source.stem
            if name not in named:
                bins.append({'name': name, 'path': source.relative_to(manifest.parent).as_posix()})

    candidates = sorted({row['path'] for row in scripts} | {path for row in supported for path in row['requires']})
    result = subprocess.run(['git', 'grep', '-n', '-F', '-f', '-',
                             '--', '.', ':!docs/brainstorms', ':!dev/journals'], input='\n'.join(candidates),
                            cwd=repo, text=True, capture_output=True)
    return {'commands': supported, 'scripts': scripts,
            'cargo_bins': [dict(row, status='active' if any(command.get('binary') == row['name'] for command in supported) else 'unknown') for row in bins],
            'path_consumers': result.stdout.splitlines(), 'consumer_scan_available': result.returncode in (0, 1)}
