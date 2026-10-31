#!/usr/bin/env python3
"""Prepare compressed browser code and serve it with HTTP content encoding."""

from __future__ import annotations

import argparse
from functools import partial
import gzip
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import tempfile


def write_bytes(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if (path.is_file() and not path.is_symlink()
            and path.stat().st_mode & 0o777 == 0o644 and path.read_bytes() == data):
        return
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as output:
        output.write(data)
        temporary = Path(output.name)
    temporary.chmod(0o644)
    temporary.replace(path)


def prepare(root: Path, code_dir: Path) -> dict:
    root = root.resolve()
    code_dir = code_dir.resolve()
    code = []
    for path in sorted(code_dir.rglob("*")):
        if not path.is_file() or path.suffix not in {".js", ".wasm"}:
            continue
        data = path.read_bytes()
        compressed = gzip.compress(data, compresslevel=9, mtime=0)
        write_bytes(path.with_name(path.name + ".gz"), compressed)
        code.append({"path": os.path.relpath(path, root),
                     "raw_bytes": len(data), "gzip_bytes": len(compressed)})
    if not code:
        raise ValueError(f"No browser code in {code_dir}")
    assets = [p for p in (root / "assets").rglob("*") if p.is_file()]
    encoded_assets = []
    for path in assets:
        if path.suffix != ".bank":
            continue
        data = path.read_bytes()
        compressed = gzip.compress(data, compresslevel=9, mtime=0)
        relative = path.relative_to(root)
        output = root / ".encoded" / relative.with_name(relative.name + ".gz")
        write_bytes(output, compressed)
        encoded_assets.append({"path": relative.as_posix(), "raw_bytes": len(data), "gzip_bytes": len(compressed)})
    report = {
        "code": code,
        "code_raw_bytes": sum(row["raw_bytes"] for row in code),
        "code_gzip_bytes": sum(row["gzip_bytes"] for row in code),
        "published_assets": {"files": len(assets), "raw_bytes": sum(p.stat().st_size for p in assets)},
        "compressed_assets": encoded_assets,
        "note": "Published assets are available files, not an initial-download measurement.",
    }
    write_bytes(root / "bundle-report.json", (json.dumps(report, indent=2) + "\n").encode())
    print(f"Browser code: {report['code_raw_bytes'] / 1048576:.2f} MiB raw, "
          f"{report['code_gzip_bytes'] / 1048576:.2f} MiB gzip")
    print(f"Available assets: {len(assets)} files, {report['published_assets']['raw_bytes'] / 1048576:.2f} MiB")
    print(f"Report: {root / 'bundle-report.json'}")
    return report


class Handler(SimpleHTTPRequestHandler):
    """Keep the original MIME type when a compressed representation is served."""

    def send_head(self):
        self.encoding = None
        self.original_type = None
        original = self.path
        path = Path(self.translate_path(original))
        accepted = {}
        for entry in self.headers.get("Accept-Encoding", "").split(","):
            name, *params = entry.strip().split(";")
            quality = 1.0
            for param in params:
                if param.strip().startswith("q="):
                    try:
                        quality = float(param.strip()[2:])
                    except ValueError:
                        quality = 0.0
            accepted[name] = quality
        if path.is_file():
            for encoding, suffix in (("br", ".br"), ("gzip", ".gz")):
                sidecar = path.with_name(path.name + suffix)
                if not sidecar.is_file():
                    relative = path.relative_to(Path(self.directory))
                    sidecar = Path(self.directory) / ".encoded" / relative.with_name(relative.name + suffix)
                if accepted.get(encoding, accepted.get("*", 0)) > 0 and sidecar.is_file():
                    self.encoding = encoding
                    self.original_type = self.guess_type(str(path))
                    self.path = "/" + sidecar.relative_to(Path(self.directory)).as_posix()
                    break
        try:
            return super().send_head()
        finally:
            self.path = original

    def guess_type(self, path):
        return self.original_type or ("application/wasm" if path.endswith(".wasm") else super().guess_type(path))

    def end_headers(self):
        self.send_header("Vary", "Accept-Encoding")
        self.send_header("Cache-Control", "no-cache")
        if getattr(self, "encoding", None):
            self.send_header("Content-Encoding", self.encoding)
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("prepare")
    build.add_argument("--root", type=Path, required=True)
    build.add_argument("--code-dir", type=Path, required=True)
    serve = commands.add_parser("serve")
    serve.add_argument("--root", type=Path, required=True)
    serve.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.root, args.code_dir)
    else:
        server = ThreadingHTTPServer(("127.0.0.1", args.port), partial(Handler, directory=str(args.root.resolve())))
        print(f"Serving {args.root} at http://localhost:{args.port}/", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            server.server_close()


if __name__ == "__main__":
    main()
