"""A flat gauge for the white inlays, to see what the 0.2 mm nozzle draws in the deck's first layers.

From the cut corner down: white bars and black gaps of stepped widths, the deck's six labels
at three capital heights, and the logo three times at the deck's width, each time drawn
bigger by another guess of what printing takes off. Small squares to the left number the
logos. The white is as deep as on the deck, lies flat and needs no supports. The corner next
to the thinnest samples is cut, so the plate cannot be read the wrong way round.
"""

from build123d import Compound, Part, Polygon, Pos, Rectangle, Sketch, extrude

from .inlay import LABELS, label_outline
from .logo import WIDTH, logo_outline, stroke_widths
from .params import Rect

PLATE = (70.0, 101.0)
CUT_CORNER = 5.0
MARGIN = 3.0
# Black behind the white, enough to hold it. The deck has more, which would show nowhere and take this nozzle long.
BACKING = 0.4

WIDTHS = (0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0)
SAMPLE_LENGTH = 6.0
BARS_X, BARS_Y, BARS_PITCH = 6.0, 3.0, 5.0
GAPS_X, GAPS_Y, GAPS_PITCH = 7.0, 11.5, 9.0
BLOCK = 2.0
TITLES_X = 53.0

# The guess this gauge was printed to check: the 0.2 mm nozzle takes half of what the 0.4 mm nozzle
# does off a stroke, draws the font's own strokes and keeps a gap of one line open.
LOSS, MIN_GAP = 0.1, 0.3
CAP_HEIGHTS = (3.0, 3.5, 4.0)
LABEL_ROWS = (LABELS[:3], LABELS[3:])
FIRST_ROW_Y, ROW_GAP = 20.5, 3.0

LOGO_LOSSES = (0.0, 0.1, 0.2)
LOGO_X = 15.0
FIRST_LOGO_Y, LOGO_PITCH = 60.5, 13.5
MARK, MARK_PITCH = 2.0, 3.5
CAPTION_ABOVE = 1.6


def build_inlay_gauge(_board, p):
    width, length = PLATE
    plate = Polygon((CUT_CORNER, 0), (width, 0), PLATE, (0, length), (0, CUT_CORNER), align=None)
    white = [Pos(*rect.centre) * Rectangle(rect.width, rect.height) for rect in _bars() + _blocks() + _marks()]
    white += [Pos(x, y) * outline for x, y, outline in _labels() + _logos()]

    def first_layers(sketch):
        return extrude(Pos(0, 0, BACKING) * sketch, amount=p.inlay_depth, dir=(0, 0, 1))

    # The black is cut flat and then made solid: scooping every letter out of a solid plate takes a minute.
    black = plate - Sketch(Compound([face for shape in white for face in shape.faces()]).wrapped)
    return extrude(plate, amount=BACKING) + first_layers(black), Part() + [first_layers(shape) for shape in white]


def captions():
    """What each sample is, for the picture: (x, y, text) in the plate's top view."""
    notes = [(BARS_X + i * BARS_PITCH, BARS_Y - 1.2, f"{width:.1f}") for i, width in enumerate(WIDTHS)]
    notes += [(GAPS_X + i * GAPS_PITCH, GAPS_Y - 1.25, f"{width:.1f}") for i, width in enumerate(WIDTHS)]
    notes += [
        (TITLES_X, BARS_Y + 1.5, "white bars: width in mm"),
        (TITLES_X, BARS_Y + 4.5, "below: black gaps, width in mm"),
    ]
    notes += [
        (PLATE[0] / 2, top - ROW_GAP / 2, f"capitals {cap:.1f} mm, drawn {LOSS:.1f} mm bigger")
        for cap, top in _rows()[:: len(LABEL_ROWS)]
    ]
    for number, (loss, y) in enumerate(zip(LOGO_LOSSES, _logo_rows()), start=1):
        _, small_line = stroke_widths(WIDTH, loss=loss, min_stroke=0.0)
        drawn = f"strokes and dots drawn {loss:.1f} mm bigger" if loss else "the artwork as drawn"
        notes.append((LOGO_X + WIDTH / 2, y - CAPTION_ABOVE, f"{number}: {drawn} (small line {small_line:.2f} mm)"))
    return tuple(notes)


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
    """The labels three to a row, each in the middle of its third of the plate."""
    third = PLATE[0] / len(LABEL_ROWS[0])
    return [
        (third * (column + 0.5), top + (cap + LOSS) / 2, label_outline(text, cap, loss=LOSS, min_stroke=0.0, min_gap=MIN_GAP))
        for (cap, top), row in zip(_rows(), LABEL_ROWS * len(CAP_HEIGHTS))
        for column, text in enumerate(row)
    ]


def _rows():
    """The capital height and the top of each row of labels: two rows per height."""
    rows, top = [], FIRST_ROW_Y
    for cap in CAP_HEIGHTS:
        for _ in LABEL_ROWS:
            rows.append((cap, top))
            top += cap + ROW_GAP
    return rows


def _logos():
    return [(LOGO_X, y, logo_outline(WIDTH, loss=loss, min_stroke=0.0)) for loss, y in zip(LOGO_LOSSES, _logo_rows())]


def _logo_rows():
    return [FIRST_LOGO_Y + LOGO_PITCH * row for row in range(len(LOGO_LOSSES))]


def _marks():
    """As many squares as the logo's number, level with its middle."""
    height = logo_outline(WIDTH, loss=0.0, min_stroke=0.0).bounding_box().size.Y
    marks = []
    for count, y in enumerate(_logo_rows(), start=1):
        middle = y + height / 2
        for i in range(count):
            x = MARGIN + MARK_PITCH * i
            marks.append(Rect(x, middle - MARK / 2, x + MARK, middle + MARK / 2))
    return marks
