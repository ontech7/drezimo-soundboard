"""Positions read from the KiCad board, in the case frame.

Case frame: mm, x to the right, y from the back edge (USB) to the front, origin at the case's
back-left corner. The case is the board plus INSET on every side.
"""

import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "hardware" / "scripts"))
from sexpr import find, first, parse  # noqa: E402

DEFAULT_PCB = REPO / "hardware" / "pcb" / "drezimo-soundboard.kicad_pcb"
INSET = 3.0
PITCH = 19.05

SWITCH_FOOTPRINT = "Drezimo:SW_MX_PCB"
HOLE_FOOTPRINT = "MountingHole:MountingHole_3.2mm_M3"
OUTLINE_SHAPES = ("gr_line", "gr_arc", "gr_rect")


@dataclass(frozen=True)
class Board:
    outline: tuple[float, float]
    keys: dict[int, tuple[float, float]]
    holes: list[tuple[float, float]]

    @property
    def case_size(self):
        return self.outline[0] + 2 * INSET, self.outline[1] + 2 * INSET

    def key_centre(self, n):
        return self.keys[n]


def load_board(path=DEFAULT_PCB):
    pcb = parse(Path(path).read_text())
    xs, ys = zip(*_outline_points(pcb))
    left, back = min(xs), min(ys)

    def to_case(fp):
        x, y = first(fp, "at")[1:3]
        return round(float(x) - left + INSET, 4), round(float(y) - back + INSET, 4)

    keys, holes = {}, []
    for fp in find(pcb, "footprint"):
        if fp[1] == SWITCH_FOOTPRINT:
            keys[int(_reference(fp).removeprefix("SW"))] = to_case(fp)
        elif fp[1] == HOLE_FOOTPRINT:
            holes.append(to_case(fp))
    return Board((round(max(xs) - left, 4), round(max(ys) - back, 4)), keys, sorted(holes))


def _outline_points(pcb):
    for shape in pcb:
        if not isinstance(shape, list) or shape[0] not in OUTLINE_SHAPES:
            continue
        if first(shape, "layer")[1] != "Edge.Cuts":
            continue
        for key in ("start", "mid", "end"):
            point = first(shape, key)
            if point:
                yield float(point[1]), float(point[2])


def _reference(fp):
    return next(p[2] for p in find(fp, "property") if p[1] == "Reference")
