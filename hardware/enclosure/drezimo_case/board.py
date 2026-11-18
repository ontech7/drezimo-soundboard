"""Positions read from the KiCad board, in the case frame.

Case frame: mm, x to the right, y from the back edge (USB) to the front, origin at the case's
back-left corner. The case is the board plus INSET on every side.
"""

import math
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
BODY_SHAPES = ("fp_line", "fp_rect", "fp_circle", "fp_arc")
BODY_LAYERS = ("F.Fab", "B.Fab")
SOLDERED_THROUGH = "thru_hole"

DISPLAY_HEADER, ENCODER, CHIP, USB_MODULE = "J1", "ENC1", "U2", "U1"
# From the encoder footprint's origin, its pad A.
ENCODER_SHAFT = (7.5, 2.5)


@dataclass(frozen=True)
class Rect:
    x0: float
    y0: float
    x1: float
    y1: float

    @property
    def width(self):
        return self.x1 - self.x0

    @property
    def height(self):
        return self.y1 - self.y0

    @property
    def centre(self):
        return (self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2

    def grown(self, by):
        return Rect(self.x0 - by, self.y0 - by, self.x1 + by, self.y1 + by)

    def holds(self, other):
        return self.x0 <= other.x0 and self.y0 <= other.y0 and other.x1 <= self.x1 and other.y1 <= self.y1


@dataclass(frozen=True)
class Pad:
    """A pin soldered through the board: its part, its centre and how far its copper reaches from there."""

    part: str
    x: float
    y: float
    reach: float

    @property
    def rect(self):
        return Rect(self.x - self.reach, self.y - self.reach, self.x + self.reach, self.y + self.reach)


@dataclass(frozen=True)
class Board:
    """The USB module hangs under the board; its pins come up through it. The rest stands on top.

    pads: every pin soldered through the board, the switches' aside.
    """

    outline: tuple[float, float]
    keys: dict[int, tuple[float, float]]
    holes: list[tuple[float, float]]
    display_pin1: tuple[float, float]
    encoder_shaft: tuple[float, float]
    encoder_body: Rect
    chip: Rect
    usb_module: Rect
    usb_pins: tuple[tuple[float, float], ...]
    pads: tuple[Pad, ...]

    @property
    def case_size(self):
        return self.outline[0] + 2 * INSET, self.outline[1] + 2 * INSET

    def key_centre(self, n):
        return self.keys[n]


def load_board(path=DEFAULT_PCB):
    pcb = parse(Path(path).read_text())
    xs, ys = zip(*_outline_points(pcb))
    left, back = min(xs), min(ys)

    def to_case(fp, point=(0.0, 0.0)):
        """A point of the footprint's own frame. KiCad turns a footprint anticlockwise, with y down."""
        x, y, *turned = (float(value) for value in first(fp, "at")[1:])
        angle = math.radians(turned[0]) if turned else 0.0
        px, py = point
        return (
            round(x + px * math.cos(angle) + py * math.sin(angle) - left + INSET, 4),
            round(y - px * math.sin(angle) + py * math.cos(angle) - back + INSET, 4),
        )

    def body(fp):
        xs, ys = zip(*(to_case(fp, point) for point in _body_points(fp)))
        return Rect(min(xs), min(ys), max(xs), max(ys))

    keys, holes, parts = {}, [], {}
    for fp in find(pcb, "footprint"):
        if fp[1] == SWITCH_FOOTPRINT:
            keys[int(_reference(fp).removeprefix("SW"))] = to_case(fp)
        elif fp[1] == HOLE_FOOTPRINT:
            holes.append(to_case(fp))
        else:
            parts[_reference(fp)] = fp

    usb = parts[USB_MODULE]
    return Board(
        outline=(round(max(xs) - left, 4), round(max(ys) - back, 4)),
        keys=keys,
        holes=sorted(holes),
        display_pin1=to_case(parts[DISPLAY_HEADER]),
        encoder_shaft=to_case(parts[ENCODER], ENCODER_SHAFT),
        encoder_body=body(parts[ENCODER]),
        chip=body(parts[CHIP]),
        usb_module=body(usb),
        usb_pins=tuple(to_case(usb, _point(first(pad, "at"))) for pad in find(usb, "pad")),
        pads=tuple(
            Pad(reference, *to_case(fp, _point(first(pad, "at"))), max(_point(first(pad, "size"))) / 2)
            for reference, fp in parts.items()
            for pad in find(fp, "pad")
            if pad[2] == SOLDERED_THROUGH
        ),
    )


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


def _body_points(fp):
    """The corners of the part's drawn body, in the footprint's own frame."""
    for kind in BODY_SHAPES:
        for shape in find(fp, kind):
            if first(shape, "layer")[1] in BODY_LAYERS:
                yield from (_point(first(shape, key)) for key in ("start", "mid", "end") if first(shape, key))


def _point(item):
    return float(item[1]), float(item[2])


def _reference(fp):
    return next(p[2] for p in find(fp, "property") if p[1] == "Reference")
