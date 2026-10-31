"""Serve the inspector and save review notes on the local machine."""

from __future__ import annotations

import json
import os
import subprocess
import threading
import webbrowser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from . import reviews


def make_handler(repo: Path, catalog, status=None):
    store = reviews.Store(repo)
    lock = threading.Lock()
    def current_catalog():
        return catalog() if callable(catalog) else catalog

    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(repo), **kwargs)

        def reply(self, status: int, data: dict):
            body = json.dumps(data).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if urlsplit(self.path).path == "/api/build-status":
                return self.reply(200, dict(status or {"state": "done"}))
            if urlsplit(self.path).path != "/api/art-reviews":
                return super().do_GET()
            try:
                self.reply(200, reviews.snapshot(repo, current_catalog()))
            except (ValueError, OSError) as error:
                self.reply(500, {"error": str(error)})

        def mutate(self):
            try:
                origin = self.headers.get("Origin")
                if origin and origin != "http://" + self.headers.get("Host", ""):
                    return self.reply(403, {"error": "Open the inspector from this local server."})
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= 65536 or self.headers.get_content_type() != "application/json":
                    return self.reply(400, {"error": "Send a JSON note of at most 64 KiB."})
                data = json.loads(self.rfile.read(length))
                path = urlsplit(self.path).path
                with lock:
                    current = current_catalog()
                    if self.command == "POST" and path == "/api/music-reviews":
                        from . import music_review
                        try:
                            return self.reply(200, music_review.save(repo, current, data))
                        except LookupError as error:
                            raise reviews.ConflictError(str(error)) from error
                    if self.command == "POST" and path == "/api/art-reviews":
                        subject = reviews.all_subjects(current).get(data.get("character"))
                        if not subject or data.get("generation_id") != subject["generation_id"]:
                            raise reviews.ConflictError("The page and server show different art. Reload the inspector.")
                        reviews.check_products(repo, subject)
                        store.add(subject, data.get("text", ""), priority=data.get("priority", "normal"), animation=data.get("animation", ""))
                    elif self.command == "PATCH" and path.startswith("/api/art-reviews/"):
                        store.update(path.rsplit("/", 1)[1], data["status"], data.get("resolution", ""), data.get("updated_at"))
                    else:
                        return self.reply(404, {"error": "Unknown review operation."})
                self.reply(200, reviews.snapshot(repo, current))
            except reviews.ConflictError as error:
                self.reply(409, {"error": str(error)})
            except (ValueError, OSError, KeyError, TypeError, AttributeError) as error:
                self.reply(400, {"error": str(error)})

        do_POST = mutate
        do_PATCH = mutate

    return Handler


def serve(repo: Path, out: Path, *, port=8767, open_browser=False, build=None, refresh_command=None):
    """Serve `out`. A page built before opens at once and `refresh_command` (a
    build command line) refreshes it behind, in its own low-priority process, so
    the server answers the page while it runs. The page polls `/api/build-status`
    and offers a reload. With no page yet, `build(log)` runs first."""
    cached = None
    stamp = None
    def current_catalog():
        nonlocal cached, stamp
        stat = (out / "catalog.js").stat()
        current_stamp = (stat.st_mtime_ns, stat.st_size)
        if stamp != current_stamp:
            cached = reviews.read_catalog(out)
            stamp = current_stamp
        return cached
    status = {"state": "done", "step": ""}
    def log(message: str) -> None:
        print(message, flush=True)
        status["step"] = message.split("\n", 1)[0]
    if build is not None and not (out / "index.html").is_file():
        build(log)
        refresh_command = None
    with ThreadingHTTPServer(("127.0.0.1", port), make_handler(repo, current_catalog, status)) as server:
        url = f"http://127.0.0.1:{server.server_port}/{(out / 'index.html').relative_to(repo).as_posix()}#overview"
        print(f"Asset inspector: {url}", flush=True)
        print("Review notes: tools/asset_inspector/reviews/notes/", flush=True)
        if open_browser:
            webbrowser.open(url)
        if refresh_command:
            def refresh():
                # A separate process at low priority: a refresh is pure Python work and a
                # possible cargo build, which in a thread here would starve page requests.
                status["state"] = "building"
                try:
                    child = subprocess.Popen(refresh_command, cwd=repo, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                             env=dict(os.environ, PYTHONUNBUFFERED="1"),
                                             preexec_fn=(lambda: os.nice(10)) if hasattr(os, "nice") else None)
                    lines = []
                    for line in child.stdout:
                        line = line.rstrip("\n")
                        lines.append(line)
                        log(line)
                    if child.wait():
                        raise RuntimeError("\n".join(lines[-8:]))
                    status["state"] = "done"
                except Exception as error:  # the open page stays usable; say why it did not refresh
                    status.update(state="failed", step=f"{type(error).__name__}: {str(error).splitlines()[-1] if str(error) else ''}")
                    print(f"Asset inspector: the refresh failed; the page shows the previous index.\n{error}", flush=True)
            print("Showing the previous index while it refreshes.", flush=True)
            threading.Thread(target=refresh, daemon=True).start()
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
