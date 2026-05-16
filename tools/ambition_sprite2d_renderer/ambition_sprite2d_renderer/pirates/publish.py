from __future__ import annotations

import sys

from .render import main as render_main


def main(argv: list[str] | None = None) -> int:
    return int(render_main(["--publish", *(argv if argv is not None else sys.argv[1:])]) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
