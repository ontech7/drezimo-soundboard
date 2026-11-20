"""The logo as a white inlay, traced from the artwork that every version of it shares.

The outline is in the case's top view: x to the right, y towards the user, with the top left
corner of the artwork, as drawn, on the origin.
"""

import math
import re
from dataclasses import dataclass
from functools import cache
from itertools import pairwise
from xml.etree import ElementTree

from build123d import Compound, Face, Pos, Sketch, Wire, extrude
from shapely.affinity import translate
from shapely.geometry import MultiLineString, box
from shapely.ops import unary_union

from .board import REPO
from .inlay import MIN_GAP, MIN_OPENING, MIN_STROKE, PRINT_LOSS

ARTWORK = REPO / "assets" / "logo" / "drezimo-logo-mono.svg"
WIDTH = 42.0

# Between the bars of a letter, centre to centre, in strokes: both lines of the wordmark are drawn alike.
BAR_PITCH = 3.0
ARC_STEP = math.radians(10)
SEGMENTS_PER_QUARTER = 4
SVG = "{http://www.w3.org/2000/svg}"


@dataclass(frozen=True)
class Artwork:
    """The drawing in its own units.

    squares: the pads, each (x, y, side, corner radius).
    lines: the lines of the wordmark, each (stroke width, the centre lines its strokes follow).
    """

    squares: tuple
    lines: tuple


def logo_outline(width=WIDTH, loss=PRINT_LOSS, min_stroke=MIN_STROKE):
    """The logo as it is drawn to print, `width` wide before anything is thickened.

    Strokes and dots are drawn `loss` bigger, which printing takes back. A line of letters whose
    strokes are still thinner than min_stroke is thickened towards it, as far as its counters allow.
    """
    return _outline(width, loss, min_stroke)


def build_logo(p):
    """The logo in the deck: its left edge on the first keycap's, its middle level with the display's lit area."""
    _, _, width, height = _frame()
    x = p.fn_well.x0 + p.well_margin
    y = p.lit.centre[1] - height * WIDTH / width / 2
    return extrude(Pos(x, y, p.deck_top - p.inlay_depth) * logo_outline(), amount=p.inlay_depth, dir=(0, 0, 1))


def stroke_widths(width=WIDTH, loss=PRINT_LOSS, min_stroke=MIN_STROKE):
    """How thick each line of the wordmark is drawn, from the first line down."""
    unit = width / _frame()[2]
    return tuple(_stroke(stroke * unit, loss, min_stroke) for stroke, _ in _artwork().lines)


def read_artwork(path=ARTWORK):
    squares, lines = [], []
    for element in ElementTree.parse(path).getroot():
        kind = element.tag.removeprefix(SVG)
        if kind == "rect":
            x, y, side, height, radius = (float(element.get(key)) for key in ("x", "y", "width", "height", "rx"))
            if side != height:
                raise ValueError("the logo's artwork has a rectangle that is not square, which is not traced")
            squares.append((x, y, side, radius))
        elif kind == "path":
            lines.append((float(element.get("stroke-width")), tuple(_centre_lines(element.get("d")))))
        else:
            raise ValueError(f"the logo's artwork has a <{kind}>, which is not traced")
    return Artwork(tuple(squares), tuple(lines))


@cache
def _artwork():
    return read_artwork()


@cache
def _outline(width, loss, min_stroke):
    left, top, width_in_units, _ = _frame()
    unit = width / width_in_units
    shapes = translate(_drawn(unit, loss, min_stroke), -left * unit, -top * unit)
    return Sketch(Compound([_face(shape) for shape in shapes.geoms]).wrapped)


@cache
def _frame():
    """The artwork's top left corner, its width and its height, as drawn and in its own units."""
    left, top, right, bottom = _drawn(1.0, 0.0, 0.0).bounds
    return left, top, right - left, bottom - top


def _drawn(unit, loss, min_stroke):
    """Every shape of the logo, in mm from the artwork's own origin."""
    artwork = _artwork()
    pitch = min(b - a for a, b in pairwise(sorted({x + side / 2 for x, _, side, _ in artwork.squares}))) * unit

    shapes = []
    for x, y, side, radius in artwork.squares:
        grow = _square_growth(side * unit, pitch, loss)
        core = box((x + radius) * unit, (y + radius) * unit, (x + side - radius) * unit, (y + side - radius) * unit)
        shapes.append(core.buffer(radius * unit + grow, quad_segs=SEGMENTS_PER_QUARTER))
    for stroke, centre_lines in artwork.lines:
        letters = MultiLineString([[(x * unit, y * unit) for x, y in line] for line in centre_lines])
        shapes.append(letters.buffer(_stroke(stroke * unit, loss, min_stroke) / 2, quad_segs=SEGMENTS_PER_QUARTER))
    return unary_union(shapes)


def _square_growth(side, pitch, loss):
    """Half the print loss on every side, where that leaves MIN_GAP of black between two squares of this size.

    The lit pads are too close for it, so they stay as drawn and printing widens the black between them.
    """
    return loss / 2 if pitch - side - loss >= MIN_GAP else 0.0


def _stroke(drawn, loss, min_stroke):
    """As drawn plus the print loss, then towards min_stroke while MIN_OPENING of black is left between two bars."""
    room = BAR_PITCH * drawn - MIN_OPENING
    return max(drawn + loss, min(min_stroke, room))


def _centre_lines(path):
    """Each stroke of an SVG path as the points along its centre: a segment, or an arc of a circle."""
    words = iter(re.findall(r"[A-Za-z]|-?\d*\.?\d+", path))

    def numbers(count):
        return [float(next(words)) for _ in range(count)]

    here = None
    for command in words:
        if command == "M":
            here = tuple(numbers(2))
        elif command == "L":
            there = tuple(numbers(2))
            yield here, there
            here = there
        elif command == "A":
            radius, other_radius, _, large, sweep, *there = numbers(7)
            if radius != other_radius:
                raise ValueError("the logo's artwork has an arc of an ellipse, which is not traced")
            yield _arc(here, tuple(there), radius, bool(large), bool(sweep))
            here = tuple(there)
        else:
            raise ValueError(f"the logo's artwork has a path command {command}, which is not traced")


def _arc(start, end, radius, large, sweep):
    """Points along an arc of a circle given the SVG way. With y down, a sweep turns towards growing angles."""
    (x0, y0), (x1, y1) = start, end
    half_x, half_y = (x0 - x1) / 2, (y0 - y1) / 2
    off = math.sqrt(max(0.0, radius**2 / (half_x**2 + half_y**2) - 1))
    if large == sweep:
        off = -off
    cx, cy = (x0 + x1) / 2 + off * half_y, (y0 + y1) / 2 - off * half_x

    first = math.atan2(y0 - cy, x0 - cx)
    turn = (math.atan2(y1 - cy, x1 - cx) - first) % math.tau
    if not sweep:
        turn -= math.tau
    steps = max(1, math.ceil(abs(turn) / ARC_STEP))
    angles = (first + turn * i / steps for i in range(steps + 1))
    return tuple((cx + radius * math.cos(angle), cy + radius * math.sin(angle)) for angle in angles)


def _face(shape):
    outer, *counters = (Wire.make_polygon(ring.coords[:-1], close=True) for ring in (shape.exterior, *shape.interiors))
    face = Face(outer, counters)
    return face if face.normal_at().Z > 0 else -face
