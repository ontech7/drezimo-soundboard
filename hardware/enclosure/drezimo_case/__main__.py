"""Generates the print files.

    uv run python -m drezimo_case coupon-a
    uv run python -m drezimo_case label-gauge

With --bambu after the name, also a Bambu Studio project with the print settings decided.
"""

import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from . import bambu
from .board import load_board
from .coupon import build_coupon_a
from .export import export_part
from .gauge import build_label_gauge, captions
from .measurements import MEASUREMENTS
from .params import Params
from .preview import Keys

PRINT_DIR = Path(__file__).resolve().parents[1] / "print"
SECTION_KEYS = (37, 1)
BAMBU = "--bambu"


@dataclass(frozen=True)
class PrintedPart:
    build: Callable
    title: str
    # A draft exists to check the estimates, so it may be built on them. Anything else may not.
    draft: bool = False
    # A test piece that does not sit on the board: no measurement shapes it.
    off_the_board: bool = False
    # Part of it prints over nothing and needs supports.
    supported: bool = False
    captions: tuple = ()


PARTS = {
    "coupon-a": PrintedPart(build_coupon_a, "Coupon (a)", draft=True, supported=True),
    "label-gauge": PrintedPart(build_label_gauge, "Label gauge", off_the_board=True, captions=captions()),
}


def main(argv):
    names = [word for word in argv if word != BAMBU]
    if len(names) != 1 or names[0] not in PARTS:
        sys.exit(f"usage: python -m drezimo_case {{{','.join(PARTS)}}} [{BAMBU}]")
    name, part = names[0], PARTS[names[0]]
    if BAMBU in argv and not bambu.STUDIO.exists():
        sys.exit(f"{BAMBU} needs Bambu Studio, not found at {bambu.STUDIO}")

    board = load_board()
    p = Params.derive(board, MEASUREMENTS)
    if not (part.draft or part.off_the_board):
        p.require_measured()

    preview = {"captions": part.captions} if part.off_the_board else _section_along_the_keys(board)
    body, inlay = part.build(board, p)
    written = export_part(name, body, inlay, PRINT_DIR, title=part.title, **preview)

    for path in written:
        print(path)
    if BAMBU in argv:
        _write_bambu_project(name, part, written)
    if p.estimated and not part.off_the_board:
        print(f"Built on estimates, not for the final print: {', '.join(p.estimated)}")


def _write_bambu_project(name, part, written):
    neutral = next(path for path in written if path.suffix == ".3mf")
    project = PRINT_DIR / f"{name}-bambu.3mf"
    report = bambu.write_project(neutral, project, part.supported)
    black, white = report.grams
    print(project)
    print(
        f"Sliced by Bambu Studio {report.version} without a warning: {report.minutes:.0f} min, "
        f"{black:.1f} g black, {white:.1f} g white, black first{', on supports' if report.supports else ''}"
    )


def _section_along_the_keys(board):
    keys = Keys(
        centres=tuple(board.key_centre(n)[1] for n in SECTION_KEYS),
        switch_width=MEASUREMENTS["M3a"].value,
        keycap_width=MEASUREMENTS["M2c"].value,
        skirt_at_rest=MEASUREMENTS["M2a"].value,
        skirt_pressed=MEASUREMENTS["M2b"].value,
    )
    return {"section_x": board.key_centre(SECTION_KEYS[0])[0], "keys": keys}


if __name__ == "__main__":
    main(sys.argv[1:])
