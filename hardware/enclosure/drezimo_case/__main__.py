"""Generates the print files.

    uv run python -m drezimo_case coupon-a
    uv run python -m drezimo_case inlay-gauge
    uv run python -m drezimo_case shell

With --bambu after the name, also a Bambu Studio project with the print settings decided.
The shell is written only once every reading it uses is measured; until then, --draft
after its name draws it on the estimates, as a picture and nothing that can be printed.
"""

import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from . import bambu, gauge, sections
from .board import load_board
from .coupon import build_coupon_a
from .export import export_part
from .measurements import MEASUREMENTS
from .params import Params
from .preview import render_preview
from .shell import build_shell, support_areas

PRINT_DIR = Path(__file__).resolve().parents[1] / "print"
BAMBU, DRAFT = "--bambu", "--draft"


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
    # From the board, the dimensions and the readings, where its picture cuts it.
    sections: Callable = sections.along_the_keys
    # The diameter, in mm, of the nozzle its Bambu Studio project is for.
    nozzle: float = 0.4
    # From the dimensions, the only places where its supports may stand; anywhere when not given.
    supports_within: Callable | None = None
    # Pressed with a finger: its deck is printed full through.
    solid_deck: bool = False
    # Its outer walls are seen: it lies with its back wall to the back of the bed, where the seam goes, and gets no brim.
    walls_show: bool = False


PARTS = {
    "coupon-a": PrintedPart(build_coupon_a, "Coupon (a)", draft=True, supported=True, nozzle=0.2),
    "inlay-gauge": PrintedPart(
        gauge.build_inlay_gauge, "Inlay gauge", off_the_board=True, captions=gauge.captions(), nozzle=0.2
    ),
    "shell": PrintedPart(
        build_shell,
        "Shell",
        sections=sections.through_the_shell,
        supported=True,
        nozzle=0.2,
        supports_within=support_areas,
        solid_deck=True,
        walls_show=True,
    ),
}


def main(argv):
    names = [word for word in argv if word not in (BAMBU, DRAFT)]
    if len(names) != 1 or names[0] not in PARTS:
        sys.exit(_usage())
    name, part = names[0], PARTS[names[0]]
    final = not (part.draft or part.off_the_board)
    on_estimates = DRAFT in argv
    if on_estimates and not final:
        sys.exit(_usage())
    if on_estimates and BAMBU in argv:
        sys.exit(f"{DRAFT} draws a picture on estimates: no Bambu Studio project is made from it")
    if BAMBU in argv and not bambu.STUDIO.exists():
        sys.exit(f"{BAMBU} needs Bambu Studio, not found at {bambu.STUDIO}")

    board = load_board()
    p = Params.derive(board, MEASUREMENTS)
    if final and not on_estimates:
        p.require_measured()

    body, inlay = part.build(board, p)
    if part.off_the_board:
        preview = {"captions": part.captions}
    else:
        readings = {code: reading.value for code, reading in MEASUREMENTS.items()}
        preview = {"sections": part.sections(board, p, readings)}
        print(f"Deck top {p.deck_top} mm above the board, set by {p.deck_set_by}")
        if p.knob_lift:
            print(f"Set the knob {p.knob_lift:.1f} mm higher on its shaft than it was read (M5b): pressed, it would rub the deck")

    if on_estimates:
        picture = PRINT_DIR / f"{name}-draft.png"
        PRINT_DIR.mkdir(parents=True, exist_ok=True)
        render_preview(picture, body, inlay, title=f"{part.title}: a draft on estimates, not to be printed", **preview)
        written = [picture]
    else:
        scale = p.print_scale if final else 1.0
        written = export_part(name, body, inlay, PRINT_DIR, title=part.title, turned=part.walls_show, scale=scale, **preview)
        if scale > 1.0:
            print(f"Drawn {100 * (scale - 1):.2f} % bigger than the board asks, for what the printer shrinks (P1s, P1l)")

    for path in written:
        print(path)
    if BAMBU in argv:
        _write_bambu_project(name, part, written, p)
    if p.estimated and not part.off_the_board:
        print(f"Built on estimates, not for the final print: {', '.join(p.estimated)}")


def _usage():
    return f"usage: python -m drezimo_case {{{','.join(PARTS)}}} [{BAMBU}], or shell {DRAFT}"


def _write_bambu_project(name, part, written, p):
    neutral = next(path for path in written if path.suffix == ".3mf")
    project = PRINT_DIR / f"{name}-bambu.3mf"
    allowed = part.supports_within(p) if part.supports_within else None
    report = bambu.write_project(neutral, project, part.supported, part.nozzle, allowed, part.solid_deck, part.walls_show)
    black, white = report.grams
    print(project)
    print(
        f"Sliced by Bambu Studio {report.version} for the {part.nozzle} mm nozzle without a warning: {report.minutes:.0f} min, "
        f"{black:.1f} g black, {white:.1f} g white, black first{', on supports' if report.supports else ''}"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
