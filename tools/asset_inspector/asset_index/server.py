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


#: Words in the command line of a server this tool may stop (`--kill`); `art_inspector` is the old name.
_OURS = ("asset_inspector", "art_inspector", "asset_index", "art_index")


def _listener(port: int) -> int | None:
    """The process listening on 127.0.0.1:`port`, from /proc (Linux), or None."""
    inodes = set()
    for table in ("/proc/net/tcp", "/proc/net/tcp6"):
        try:
            rows = Path(table).read_text().splitlines()[1:]
        except OSError:
            continue
        for row in rows:
            fields = row.split()
            if len(fields) > 9 and fields[3] == "0A" and int(fields[1].rsplit(":", 1)[1], 16) == port:
                inodes.add(fields[9])
    if not inodes:
        return None
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            for fd in (proc / "fd").iterdir():
                link = os.readlink(fd)
                if link.startswith("socket:[") and link[8:-1] in inodes:
                    return int(proc.name)
        except OSError:
            continue
    return None


def _command(pid: int) -> str:
    try:
        return Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace").strip()
    except OSError:
        return ""


def _children(pid: int) -> list[int]:
    found = []
    for proc in Path("/proc").iterdir():
        try:
            if proc.name.isdigit() and int((proc / "stat").read_text().rsplit(")", 1)[1].split()[1]) == pid:
                found.append(int(proc.name))
        except (OSError, ValueError, IndexError):
            continue
    return found


def kill_server(port: int, log=print) -> bool:
    """Stop the asset inspector serving `port` and its refresh. False when another
    program holds the port: that is not ours to stop."""
    import signal
    import time
    pid = _listener(port)
    if pid is None or pid == os.getpid():
        return True
    command = _command(pid)
    if not any(word in command for word in _OURS):
        log(f"Port {port} is held by another program (pid {pid}: {command[:120]}); --kill stops only an asset inspector.")
        return False
    family = [pid, *_children(pid)]
    family += [grandchild for child in family[1:] for grandchild in _children(child)]
    log(f"Stopping the asset inspector on port {port} (pid {pid}).")
    for target in reversed(family):
        try:
            os.kill(target, signal.SIGTERM)
        except OSError:
            pass
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline and _listener(port) == pid:
        time.sleep(0.1)
    if _listener(port) == pid:
        os.kill(pid, signal.SIGKILL)
        time.sleep(0.3)
    return True


def serve(repo: Path, out: Path, *, port=8767, open_browser=False, build=None, refresh_command=None, kill=False):
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
    if kill and not kill_server(port):
        return 1
    holder = _listener(port)
    if holder is not None:
        ours = any(word in _command(holder) for word in _OURS)
        print(f"Port {port} is in use by {'another asset inspector' if ours else 'another program'} (pid {holder}). "
              + ("Rerun with --kill to replace it, or use --port." if ours else "Use --port to choose another."), flush=True)
        return 1
    if build is not None and not (out / "index.html").is_file():
        build(log)
        refresh_command = None
    child_box: list = []
    def stop(_signum, _frame):  # `--kill` from another run: take the refresh down too
        for child in child_box:
            child.terminate()
        raise SystemExit(0)
    import signal
    signal.signal(signal.SIGTERM, stop)
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
                    child_box.append(child)
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
        finally:
            for child in child_box:
                if child.poll() is None:
                    child.terminate()
    return 0
