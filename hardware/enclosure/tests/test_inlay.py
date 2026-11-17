import math

import pytest
from build123d import Axis, CenterOf, GeomType
from shapely.geometry import Polygon

from drezimo_case.board import PITCH
from drezimo_case.inlay import ARROW_GAP, ARROW_WIDTH, CAP_HEIGHT, LABELS, MIN_GAP, MIN_STROKE, PRINT_LOSS, build_labels, label_outline

# Just under half the limit: a stroke or a gap at the limit survives, a thinner one does not.
MARGIN = 0.01
# As drawn. The narrowest gap inside a letter is the mouth of the G, 0.44 mm; the gauges printed
# a 0.4 mm slot twice and a 0.36 mm mouth.
MIN_OPENING = 0.4


def flat_glyphs():
    """Every drawn letter of every label as a flat polygon with its counters."""
    for text in LABELS:
        for glyph in label_outline(text).faces():
            letter = _ring(glyph.outer_wire())
            for counter in glyph.inner_wires():
                letter = letter.difference(_ring(counter))
            yield text, letter


def _ring(wire):
    sides = [(tuple(edge @ 0)[:2], tuple(edge @ 1)[:2]) for edge in wire.order_edges()]
    # The corner a side shares with the next one, whichever way each side runs.
    corners = [
        min(side, key=lambda end: min(math.dist(end, other) for other in following))
        for side, following in zip(sides, sides[1:] + sides[:1])
    ]
    return Polygon(corners)


def pieces_and_counters(shape):
    parts = [part for part in getattr(shape, "geoms", [shape]) if not part.is_empty]
    return len(parts), sum(len(part.interiors) for part in parts)


def test_the_labels_are_the_six_words_the_owner_chose():
    assert LABELS == ["◀ PAG", "PAG ▶", "STOP", "MUTO", "VOCE", "SENTI"]


def test_every_label_stays_inside_its_key_cell():
    for text in LABELS:
        assert label_outline(text).bounding_box().size.X <= PITCH - 0.4


def test_letters_are_drawn_bigger_than_they_print():
    # Black goes down first and takes PRINT_LOSS off every white stroke, so the letters are drawn
    # that much bigger: capitals drawn 4.2 mm tall print 4.0 mm tall.
    assert (CAP_HEIGHT, PRINT_LOSS, MIN_STROKE, MIN_GAP) == (4.0, 0.2, 0.7, 0.5)
    assert label_outline("H").bounding_box().size.Y == pytest.approx(CAP_HEIGHT + PRINT_LOSS, abs=0.01)
    assert label_outline("I").bounding_box().size.X == pytest.approx(0.92, abs=0.01)


def test_small_letters_are_thickened_until_their_thinnest_stroke_prints():
    bar_of_the_t = 0.45  # mm in the font, at capitals 3.0 mm tall
    grown = MIN_STROKE - bar_of_the_t
    assert label_outline("H", cap_height=3.0).bounding_box().size.Y == pytest.approx(3.0 + grown, abs=0.01)


def test_big_letters_are_only_thickened_by_what_printing_takes_off():
    assert label_outline("H", cap_height=6.0).bounding_box().size.Y == pytest.approx(6.0 + PRINT_LOSS, abs=0.01)


def test_no_stroke_is_thinner_than_the_limit():
    # Shrinking by half the limit erases a thinner stroke: the letter would fall apart or lose a counter.
    for text, letter in flat_glyphs():
        assert pieces_and_counters(letter.buffer(-(MIN_STROKE / 2 - MARGIN))) == pieces_and_counters(letter), text


def test_no_gap_inside_a_letter_is_narrower_than_the_limit():
    # Growing by half the limit closes a narrower gap: a counter would vanish or an opening would seal.
    for text, letter in flat_glyphs():
        assert pieces_and_counters(letter.buffer(MIN_OPENING / 2 - MARGIN)) == pieces_and_counters(letter), text


def test_letters_are_far_enough_apart_to_print():
    for text in LABELS:
        boxes = [glyph.bounding_box() for glyph in label_outline(text).faces().sort_by(Axis.X)]
        for left, right in zip(boxes, boxes[1:]):
            assert right.min.X - left.max.X >= MIN_GAP - 1e-6


def test_labels_sit_above_the_function_keys(board, p):
    boxes = [label.bounding_box() for label in build_labels(board, p)]
    assert len(boxes) == len(LABELS)
    for c, box in enumerate(boxes):
        assert box.center().X == pytest.approx(board.key_centre(37 + c)[0], abs=0.05)
        assert box.center().Y == pytest.approx(46.05, abs=0.2)
        assert (box.min.Z, box.max.Z) == pytest.approx((p.deck_top - p.inlay_depth, p.deck_top))
        assert box.max.Y <= p.fn_well.y0 - 1.4
        assert box.min.Y >= 42.40 + 1.4
    for left, right in zip(boxes, boxes[1:]):
        assert right.min.X - left.max.X >= 2.0


def test_letters_read_upright_in_the_top_view():
    # In the case's top view y grows towards the user, so the top of a letter is at the smaller y.
    bowl = label_outline("P").face()
    assert bowl.center(CenterOf.MASS).Y < bowl.bounding_box().center().Y
    foot = label_outline("L").face()
    assert foot.center(CenterOf.MASS).X < foot.bounding_box().center().X


def test_arrows_point_away_from_the_word():
    left_arrow = label_outline("◀ PAG").faces().sort_by(Axis.X)[0]
    assert left_arrow.center(CenterOf.MASS).X > left_arrow.bounding_box().center().X
    right_arrow = label_outline("PAG ▶").faces().sort_by(Axis.X)[-1]
    assert right_arrow.center(CenterOf.MASS).X < right_arrow.bounding_box().center().X


def test_arrows_are_drawn_bigger_like_the_letters():
    arrow, letter = label_outline("◀ PAG").faces().sort_by(Axis.X)[:2]
    box = arrow.bounding_box()
    assert box.size.Y == pytest.approx(CAP_HEIGHT + PRINT_LOSS, abs=0.01)
    assert box.size.X == pytest.approx(ARROW_WIDTH + PRINT_LOSS, abs=0.01)
    assert letter.bounding_box().min.X - box.max.X == pytest.approx(ARROW_GAP, abs=0.01)

    letter, arrow = label_outline("PAG ▶").faces().sort_by(Axis.X)[-2:]
    assert arrow.bounding_box().min.X - letter.bounding_box().max.X == pytest.approx(ARROW_GAP, abs=0.01)


def test_outlines_are_plain_polygons():
    # Offset curves do not survive a STEP export; straight segments do.
    for text in LABELS:
        assert {edge.geom_type for edge in label_outline(text).edges()} == {GeomType.LINE}


def test_tracing_keeps_the_shape_of_the_letters():
    outline = label_outline("MUTO")
    assert len(outline.faces()) == 4
    assert len(outline.faces().sort_by(Axis.X)[-1].inner_wires()) == 1
    assert all(face.normal_at().Z > 0 for face in outline.faces())


def test_a_label_can_be_drawn_at_another_capital_height():
    box = label_outline("PAG", cap_height=5.0).bounding_box()
    assert 5.2 <= box.size.Y <= 5.45
    assert label_outline("H", cap_height=3.5).bounding_box().size.Y == pytest.approx(3.7, abs=0.01)
