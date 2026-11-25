"""Serve the inspector and save asset reviews on the local machine."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import threading
import webbrowser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

import yaml

from . import page, paths, reviews

#: What the page says when this server runs older code than the page it serves.
RESTART = "./scripts/asset_inspector.py --kill"

#: One byte range (RFC 9110): `bytes=first-last`, `bytes=first-` or the suffix `bytes=-length`.
_RANGE = re.compile(r"bytes=(\d*)-(\d*)")


class _Slice:
    """The part of an open file that a range request asked for: the server copies
    from it until it reads empty, as it copies a whole file."""

    def __init__(self, file, length: int):
        self.file, self.left = file, length

    def read(self, size: int = -1) -> bytes:
        size = self.left if size < 0 else min(size, self.left)
        data = self.file.read(size) if size else b""
        self.left -= len(data)
        return data

    def close(self):
        self.file.close()


def _code_stamp() -> tuple:
    """When each source file of this server last changed. A server keeps the code it
    started with, but a build (its own refresh, or another run) writes a new page:
    a server left running across an update answers the new page with old code."""
    return tuple(sorted((path.name, path.stat().st_mtime_ns) for path in Path(__file__).parent.glob("*.py")))


def make_handler(repo: Path, catalog, status=None, landing=None):
    """`landing` is the page's path relative to `repo`; `/` redirects to it."""
    lock = threading.Lock()
    started = _code_stamp()
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
            path = urlsplit(self.path).path
            if path == "/" and landing:
                # The repository root is not a page; the landing page is the inspector.
                self.send_response(302)
                self.send_header("Location", "/" + landing + "#overview")
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                return
            if path == "/api/build-status":
                return self.reply(200, dict(status() if status else {"state": "done"}, server_stale=_code_stamp() != started,
                                            restart=RESTART))
            if path != "/api/asset-reviews":
                return super().do_GET()
            try:
                self.reply(200, reviews.snapshot(repo, current_catalog()))
            except (ValueError, OSError, yaml.YAMLError) as error:
                self.reply(500, {"error": str(error)})

        def end_headers(self):
            if urlsplit(self.path).path.endswith("/catalog.js"):
                self.send_header("Cache-Control", "no-store")
            if self.command in ("GET", "HEAD"):
                self.send_header("Accept-Ranges", "bytes")
            super().end_headers()

        def send_head(self):
            """A file, or the byte range of it the request asks for. A browser seeks
            in audio by asking for the bytes at the new time: without a range
            answer it can only play from the start."""
            match = _RANGE.fullmatch(self.headers.get("Range", "").strip())
            path = self.translate_path(self.path)
            if not match or not any(match.groups()) or not os.path.isfile(path):
                return super().send_head()
            size = os.path.getsize(path)
            first, last = match.groups()
            start, end = (int(first), min(int(last), size - 1) if last else size - 1) if first else (max(0, size - int(last)), size - 1)
            if start >= size or start > end:
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{size}")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return None
            file = open(path, "rb")
            file.seek(start)
            self.send_response(206)
            self.send_header("Content-Type", self.guess_type(path))
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
            self.send_header("Content-Length", str(end - start + 1))
            self.send_header("Last-Modified", self.date_time_string(os.fstat(file.fileno()).st_mtime))
            self.end_headers()
            return _Slice(file, end - start + 1)

        def copyfile(self, source, outputfile):
            try:
                super().copyfile(source, outputfile)
            except (BrokenPipeError, ConnectionResetError):
                pass  # a player that seeks again drops the range it no longer wants

        def mutate(self):
            try:
                origin = self.headers.get("Origin")
                if origin and origin != "http://" + self.headers.get("Host", ""):
                    return self.reply(403, {"error": "Open the inspector from this local server."})
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= 65536 or self.headers.get_content_type() != "application/json":
                    return self.reply(400, {"error": "Send a JSON request of at most 64 KiB."})
                data = json.loads(self.rfile.read(length))
                path = urlsplit(self.path).path
                with lock:
                    if self.command == "POST" and path == "/api/asset-preflight":
                        from . import preflight
                        selection = {}
                        for key in ("worlds", "rooms", "characters", "effects"):
                            values = data.get(key, [])
                            if not isinstance(values, list) or any(not isinstance(value, str) for value in values):
                                raise ValueError(f"{key} must be a list of names")
                            selection[key] = values
                        if not any(selection.values()):
                            raise ValueError("Select a world, room, character, or effect")
                        selection["purpose"] = data.get("purpose", "play")
                        if selection["purpose"] not in {"play", "capture", "headless"}:
                            raise ValueError("Unknown preflight purpose")
                        result = preflight.collect(repo, **selection)
                        if data.get("generate_missing"):
                            generated = preflight.generate_missing(repo, result)
                            result = preflight.collect(repo, **selection)
                            result["generated"] = generated
                        return self.reply(200, result)
                    current = current_catalog()
                    if self.command != "POST" or path != "/api/asset-reviews":
                        return self.reply(404, {"error": f"This server has no {self.command} {path}. If the inspector "
                                                         f"was updated while it ran, restart it: {RESTART}"})
                    saved = reviews.save(repo, current, data)
                self.reply(200, dict(reviews.snapshot(repo, current), saved=saved))
            except reviews.ConflictError as error:
                self.reply(409, {"error": str(error)})
            except (ValueError, OSError, KeyError, TypeError, AttributeError, subprocess.CalledProcessError) as error:
                self.reply(400, {"error": str(error)})

        do_POST = mutate

    return Handler


#: Words in the command line of a server this tool may stop (`--kill`); `art_inspector` is the old name.
_OURS = ("asset_inspector", "asset_index")


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


def serve(repo: Path, out: Path, *, host="all", port=8767, open_browser=False, build=None,
          refresh_command=None, kill=False):
    """Serve the last index while a refresh runs in a separate process.

    Refresh on startup and when an open page sees a new art publication.
    If there is no index, build it before the server starts.
    """
    # A small alias map so the CLI reads like the web bundle tool's. The inspector
    # serves the POST review/preflight routes too, so a non-loopback bind is opt-in.
    bind = {"localhost": "127.0.0.1", "all": "0.0.0.0"}.get(host, host)
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
    publication = paths.publication_file(repo) if refresh_command else None

    def publication_stamp():
        if publication is not None:
            try:
                stat = publication.stat()
                return (stat.st_ino, stat.st_mtime_ns, stat.st_size)
            except FileNotFoundError:
                pass
        return None

    seen_publication = publication_stamp()
    first_build = build is not None and not (out / "index.html").is_file()
    if first_build:
        build(log)
    elif (out / "catalog.js").is_file():
        # The previous index shows until the refresh is done; its page files are
        # this code's, so the page asks this server for the routes it has.
        page.copy(out)
    child_box: list = []
    refresh_lock = threading.Lock()
    nice = shutil.which("nice")

    def refresh(has_publication):
        child = None
        try:
            generation = json.loads(publication.read_text())["generation"] if has_publication else None
            command = [nice, "-n", "10", *refresh_command] if nice else refresh_command
            child = subprocess.Popen(command, cwd=repo, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                     env=dict(os.environ, PYTHONUNBUFFERED="1"))
            child_box.append(child)
            lines = []
            for line in child.stdout:
                line = line.rstrip("\n")
                lines.append(line)
                log(line)
            if child.wait():
                raise RuntimeError("\n".join(lines[-8:]))
            status.update(state="done", generation=generation)
        except Exception as error:
            status.update(state="failed", step=f"{type(error).__name__}: {str(error).splitlines()[-1] if str(error) else ''}")
            print(f"Ambition Editor: the refresh failed; the page shows the previous index.\n{error}", flush=True)
        finally:
            if child is not None:
                child_box.remove(child)

    def refresh_status(force=False):
        nonlocal seen_publication
        with refresh_lock:
            if refresh_command and status["state"] != "building":
                current = publication_stamp()
                if force or current != seen_publication:
                    seen_publication = current
                    status.update(state="building", step="starting")
                    threading.Thread(target=refresh, args=(current is not None,), daemon=True).start()
            return dict(status)

    def stop(_signum, _frame):  # `--kill` from another run: take the refresh down too
        for child in child_box:
            child.terminate()
        raise SystemExit(0)
    import signal
    signal.signal(signal.SIGTERM, stop)
    if bind not in {"127.0.0.1", "::1"}:
        print(f"Ambition Editor: binding {bind} — the review and preflight POST routes are reachable from other machines.",
              flush=True)
    landing = (out / "index.html").relative_to(repo).as_posix()
    with ThreadingHTTPServer((bind, port), make_handler(repo, current_catalog, refresh_status, landing)) as server:
        url_host = "localhost" if bind in {"127.0.0.1", "0.0.0.0"} else bind
        url = f"http://{url_host}:{server.server_port}/"
        print(f"Ambition Editor: {url}", flush=True)
        print("Reviews: tools/ambition_editor/reviews/ (music: the music tool's reviews/music/)", flush=True)
        if open_browser:
            webbrowser.open(url)
        if refresh_command and not first_build:
            print("Showing the previous index while it refreshes.", flush=True)
            refresh_status(force=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            for child in child_box:
                if child.poll() is None:
                    child.terminate()
    return 0
