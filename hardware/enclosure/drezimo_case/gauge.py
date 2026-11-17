"""A flat gauge for the white inlays.

It shows what the printer can draw in the deck's first layers: white bars and black gaps of
stepped widths, then the deck's six labels, at the deck's capital height and one size up.
It has the deck's thickness and inlay depth, lies flat and needs no supports. The corner
next to the thinnest samples is cut, so the plate cannot be read the wrong way round.
"""

from build123d import Part, Polygon, Pos, extrude

from .deck import box
from .inlay import CAP_HEIGHT, LABELS, label_outline
from .params import Rect

PLATE = (70.0, 63.0)
CUT_CORNER = 5.0
MARGIN = 3.0

WIDTHS = (0.4, 0.5, 0.6, 0.7, 0.8, 1.0, 1.2)
SAMPLE_LENGTH = 6.0
BARS_X, BARS_Y, BARS_PITCH = 6.0, 3.0, 5.0
GAPS_X, GAPS_Y, GAPS_PITCH = 7.0, 11.5, 9.0
BLOCK = 2.0

CAP_HEIGHTS = (CAP_HEIGHT, 4.0)
FIRST_ROW_Y, ROW_GAP = 20.5, 3.0

CAPTION_X = 26.0
TITLES_X = 53.0


def build_label_gauge(_board, p):
    top = p.deck_thickness
    floor = top - p.inlay_depth

    inlay = Part() + [box(rect, floor, top) for rect in _bars() + _blocks()]
    inlay += [extrude(Pos(x, y, floor) * outline, amount=p.inlay_depth, dir=(0, 0, 1)) for x, y, outline in _labels()]

    plate = Polygon((CUT_CORNER, 0), (PLATE[0], 0), PLATE, (0, PLATE[1]), (0, CUT_CORNER), align=None)
    return extrude(plate, amount=top) - inlay, inlay


def captions():
    """What each sample is, for the picture: (x, y, text) in the plate's top view."""
    notes = [(BARS_X + i * BARS_PITCH, BARS_Y - 1.2, f"{width:.1f}") for i, width in enumerate(WIDTHS)]
    notes += [(GAPS_X + i * GAPS_PITCH, GAPS_Y - 1.25, f"{width:.1f}") for i, width in enumerate(WIDTHS)]
    notes += [
        (TITLES_X, BARS_Y + 1.5, "white bars: width in mm"),
        (TITLES_X, BARS_Y + 4.5, "below: black gaps, width in mm"),
    ]
    return tuple(notes + [(CAPTION_X, top + cap / 2, f"capitals {cap:.1f} mm") for cap, top in list(_rows())[1::3]])


def _bars():
    return [_sample(BARS_X + i * BARS_PITCH - width / 2, BARS_Y, width) for i, width in enumerate(WIDTHS)]


def _blocks():
    blocks = []
    for i, gap in enumerate(WIDTHS):
        centre = GAPS_X + i * GAPS_PITCH
        blocks += [_sample(centre - gap / 2 - BLOCK, GAPS_Y, BLOCK), _sample(centre + gap / 2, GAPS_Y, BLOCK)]
    return blocks


def _sample(x, y, width):
    return Rect(x, y, x + width, y + SAMPLE_LENGTH)


def _labels():
    """The labels two to a row: one against the left margin, one against the right."""
    pairs = list(zip(LABELS[::2], LABELS[1::2]))
    for (cap, top), (left, right) in zip(_rows(), pairs * len(CAP_HEIGHTS)):
        for text, side in ((left, -1), (right, 1)):
            outline = label_outline(text, cap)
            reach = PLATE[0] / 2 - MARGIN - outline.bounding_box().size.X / 2
            yield PLATE[0] / 2 + side * reach, top + _drawn_height(cap) / 2, outline


def _rows():
    """The capital height and the top of each row: three rows of labels per height."""
    top = FIRST_ROW_Y
    for cap in CAP_HEIGHTS:
        for _ in range(len(LABELS) // 2):
            yield cap, top
            top += cap + ROW_GAP


def _drawn_height(cap):
    return label_outline("H", cap).bounding_box().size.Y
