"""White inlays in the deck: the function-key labels."""

import math
from functools import cache

from build123d import Align, Axis, Face, GeomType, Polygon, Pos, Rectangle, Sketch, Text, Wire, extrude, offset

from .board import REPO

FONT = REPO / "app" / "src-tauri" / "src" / "overlay" / "Inter-SemiBold.ttf"
LABELS = ["◀ PAG", "PAG ▶", "STOP", "MUTO", "VOCE", "SENTI"]
# Midway between the OLED module (y 42.40) and the key well (y 49.7).
LABEL_Y = 46.05

# The capitals' height as printed.
CAP_HEIGHT = 4.0
# Black goes down first on the first layer and takes this much off the width of every white stroke,
# so letters and arrows are drawn that much bigger.
PRINT_LOSS = 0.2
# As drawn. A thinner stroke prints as a hairline; a narrower gap, between two letters or inside
# one, is smaller than any the label gauge has shown to print.
MIN_STROKE = 0.7
MIN_GAP = 0.5
TRACE_STEP = 0.1
JOIN_TOLERANCE = 1e-4
ARROW_WIDTH = 2.2
ARROW_GAP = 1.0
LEFT_ARROW, RIGHT_ARROW = "◀", "▶"
FIRST_FN_KEY = 37


def label_outline(text, cap_height=CAP_HEIGHT):
    """The label as it reads in the case's top view, centred on the origin."""
    return _outline(text, cap_height)


def build_labels(board, p):
    floor = p.deck_top - p.inlay_depth
    return [
        extrude(
            Pos(board.key_centre(FIRST_FN_KEY + column)[0], LABEL_Y, floor) * label_outline(text),
            amount=p.inlay_depth,
            dir=(0, 0, 1),
        )
        for column, text in enumerate(LABELS)
    ]


@cache
def _outline(text, cap_height):
    size = cap_height / _glyph_size("H").Y
    grow = _grow(size)
    label = _letters(text.strip(LEFT_ARROW + RIGHT_ARROW + " "), size, grow)

    box = label.bounding_box()
    if text.startswith(LEFT_ARROW):
        label += offset(_arrow(box.min.X - ARROW_GAP - grow, -ARROW_WIDTH, cap_height), amount=grow)
    if text.endswith(RIGHT_ARROW):
        label += offset(_arrow(box.max.X + ARROW_GAP + grow, ARROW_WIDTH, cap_height), amount=grow)

    return _traced(label)


def _traced(label):
    """The label as plain polygons, centred, and flipped to read in the case's top view.

    Letters are drawn with y up, while in the case frame the top of the deck's text is at the
    smaller y. Polygons also replace the offset curves, which a STEP file cannot carry.
    """
    centre = label.bounding_box().center()

    def polygon(wire):
        path = []
        for edge in wire.order_edges():
            steps = 1 if edge.geom_type == GeomType.LINE else max(2, math.ceil(edge.length / TRACE_STEP))
            piece = [edge @ (i / steps) for i in range(steps + 1)]
            if path:
                if min((piece[0] - path[-1]).length, (piece[-1] - path[-1]).length) > JOIN_TOLERANCE:
                    path.reverse()
                if (piece[0] - path[-1]).length > JOIN_TOLERANCE:
                    piece.reverse()
                piece = piece[1:]
            path += piece
        return Wire.make_polygon([(pt.X - centre.X, centre.Y - pt.Y) for pt in path[:-1]], close=True)

    traced = Sketch()
    for glyph in label.faces():
        face = Face(polygon(glyph.outer_wire()), [polygon(hole) for hole in glyph.inner_wires()])
        traced += face if face.normal_at().Z > 0 else -face
    return traced


def _grow(size):
    """How far every outline moves out: by what printing takes off, and further until the thinnest stroke is MIN_STROKE."""
    return max(PRINT_LOSS, MIN_STROKE - size * _thinnest_stroke()) / 2


def _letters(word, size, grow):
    """The word as it is drawn to print: strokes thickened, every pair of letters MIN_GAP apart."""
    typeset = Text(word, size, font_path=str(FONT), align=(Align.MIN, Align.MIN))

    letters, shift, previous_right = Sketch(), 0.0, None
    for glyph in typeset.faces().sort_by(Axis.X):
        glyph = offset(glyph, amount=grow).face()
        box = glyph.bounding_box()
        if previous_right is not None:
            shift += max(0.0, MIN_GAP - (box.min.X + shift - previous_right))
        letters += Pos(shift, 0) * glyph
        previous_right = box.max.X + shift
    return letters


def _arrow(base_x, reach, height):
    return Polygon((base_x, 0), (base_x, height), (base_x + reach, height / 2), align=None)


@cache
def _glyph_size(glyph):
    return Text(glyph, 1.0, font_path=str(FONT)).bounding_box().size


@cache
def _thinnest_stroke():
    """Per unit of font size: the vertical stem of the I or the horizontal bar of the T."""
    bar = Text("T", 1.0, font_path=str(FONT), align=(Align.MIN, Align.MIN))
    slice_of_bar = bar & (Pos(0.1 * bar.bounding_box().size.X, 0.5) * Rectangle(0.001, 2.0))
    return min(_glyph_size("I").X, slice_of_bar.bounding_box().size.Y)
