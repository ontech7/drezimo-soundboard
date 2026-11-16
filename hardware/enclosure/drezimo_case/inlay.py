"""White inlays in the deck: the function-key labels."""

import math
from functools import cache

from build123d import Align, Axis, Face, GeomType, Polygon, Pos, Rectangle, Sketch, Text, Wire, extrude, offset

from .board import PITCH, REPO

FONT = REPO / "app" / "src-tauri" / "src" / "overlay" / "Inter-SemiBold.ttf"
LABELS = ["◀ PAG", "PAG ▶", "STOP", "MUTO", "VOCE", "ASCOLTO"]
LABEL_Y = 45.2

CAP_HEIGHTS = (2.6, 2.5, 2.4)
MAX_WIDTH = PITCH - 0.4
STROKE = 0.5
MIN_GAP = 0.45
TRACE_STEP = 0.1
JOIN_TOLERANCE = 1e-4
ARROW_WIDTH = 2.2
ARROW_GAP = 1.0
LEFT_ARROW, RIGHT_ARROW = "◀", "▶"
FIRST_FN_KEY = 37


def label_outline(text, cap_height=None):
    """The label as it reads in the case's top view, centred on the origin.

    Its capitals are as tall as the deck's labels, unless another height is given.
    """
    return _outline(text, cap_height or _cap_height())


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
def _cap_height():
    for height in CAP_HEIGHTS:
        if all(_outline(text, height).bounding_box().size.X <= MAX_WIDTH for text in LABELS):
            return height
    raise ValueError(f"the labels do not fit in {MAX_WIDTH} mm, even with capitals {CAP_HEIGHTS[-1]} mm tall")


@cache
def _outline(text, cap_height):
    label = _letters(text.strip(LEFT_ARROW + RIGHT_ARROW + " "), cap_height / _glyph_size("H").Y)

    box = label.bounding_box()
    if text.startswith(LEFT_ARROW):
        label += _arrow(box.min.X - ARROW_GAP, -ARROW_WIDTH, cap_height)
    if text.endswith(RIGHT_ARROW):
        label += _arrow(box.max.X + ARROW_GAP, ARROW_WIDTH, cap_height)

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


def _letters(word, size):
    """The word with every stroke thickened to STROKE and every pair of letters MIN_GAP apart."""
    grow = max(0.0, (STROKE - size * _thinnest_stroke()) / 2)
    typeset = Text(word, size, font_path=str(FONT), align=(Align.MIN, Align.MIN))

    letters, shift, previous_right = Sketch(), 0.0, None
    for glyph in typeset.faces().sort_by(Axis.X):
        if grow:
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
