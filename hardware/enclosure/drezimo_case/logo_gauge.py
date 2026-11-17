"""A flat gauge for the logo's white inlay.

It carries the logo three times at the deck's width, each drawn another way, to see on a print
which one the printer draws best: the artwork as it is, then with strokes and dots drawn bigger
by what printing takes off, then with the small line thickened too. Small squares to the left
number the rows, and the corner next to the first row is cut. It has the deck's thickness and
inlay depth, lies flat and needs no supports.
"""

from build123d import Part, Pos, extrude

from .deck import box
from .gauge import MARGIN, plate
from .logo import WIDTH, logo_outline, stroke_widths
from .params import Rect

PLATE = (60.0, 44.0)
LOGO_X = 15.0
FIRST_ROW_Y, ROW_PITCH = 3.5, 13.5
MARK, MARK_PITCH = 2.0, 3.5
CAPTION_ABOVE = 1.6

VARIANTS = (
    ("the artwork as drawn", {"loss": 0.0, "min_stroke": 0.0}),
    ("strokes and dots drawn bigger by the print loss", {"min_stroke": 0.0}),
    ("the same, and the small line thickened", {}),
)


def build_logo_gauge(_board, p):
    top = p.deck_thickness
    floor = top - p.inlay_depth

    inlay = Part() + [box(mark, floor, top) for mark in _marks()]
    inlay += [
        extrude(Pos(LOGO_X, y, floor) * logo_outline(WIDTH, **drawing), amount=p.inlay_depth, dir=(0, 0, 1))
        for y, (_, drawing) in zip(_rows(), VARIANTS)
    ]
    return extrude(plate(PLATE), amount=top) - inlay, inlay


def captions():
    """What each row is, for the picture: (x, y, text) in the plate's top view."""
    notes = []
    for number, (y, (text, drawing)) in enumerate(zip(_rows(), VARIANTS), start=1):
        _, small_line = stroke_widths(WIDTH, **drawing)
        notes.append((LOGO_X + WIDTH / 2, y - CAPTION_ABOVE, f"{number}: {text} (small line {small_line:.2f} mm)"))
    return tuple(notes)


def _rows():
    return [FIRST_ROW_Y + ROW_PITCH * row for row in range(len(VARIANTS))]


def _marks():
    """As many squares as the row's number, level with the middle of its logo."""
    height = logo_outline(WIDTH, loss=0.0, min_stroke=0.0).bounding_box().size.Y
    for count, y in enumerate(_rows(), start=1):
        middle = y + height / 2
        for i in range(count):
            x = MARGIN + MARK_PITCH * i
            yield Rect(x, middle - MARK / 2, x + MARK, middle + MARK / 2)
