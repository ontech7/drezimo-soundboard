import pytest
from build123d import Axis, CenterOf, GeomType, offset

from drezimo_case.board import PITCH
from drezimo_case.inlay import LABELS, build_labels, label_outline


def test_widest_label_stays_inside_its_key_cell():
    box = label_outline("ASCOLTO").bounding_box()
    assert box.size.X <= PITCH - 0.4
    assert box.size.Y == pytest.approx(2.6, abs=0.3)


def test_no_stroke_under_the_hard_limit():
    # Shrinking by 0.22 per side erases any stroke thinner than 0.44 mm, the closest the offset gets to 0.45.
    for text in LABELS:
        for glyph in label_outline(text).faces():
            assert len(offset(glyph, amount=-0.22).faces()) == 1


def test_letters_are_far_enough_apart_to_print():
    for text in LABELS:
        boxes = [glyph.bounding_box() for glyph in label_outline(text).faces().sort_by(Axis.X)]
        for left, right in zip(boxes, boxes[1:]):
            assert right.min.X - left.max.X >= 0.45 - 1e-6


def test_labels_sit_above_the_function_keys(board, p):
    boxes = [label.bounding_box() for label in build_labels(board, p)]
    assert len(boxes) == len(LABELS)
    for c, box in enumerate(boxes):
        assert box.center().X == pytest.approx(board.key_centre(37 + c)[0], abs=0.05)
        assert box.center().Y == pytest.approx(45.2, abs=0.2)
        assert (box.min.Z, box.max.Z) == pytest.approx((p.deck_top - p.inlay_depth, p.deck_top))
        assert box.max.Y <= p.fn_well.y0 - 0.6
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


def test_outlines_are_plain_polygons():
    # Offset curves do not survive a STEP export; straight segments do.
    for text in LABELS:
        assert {edge.geom_type for edge in label_outline(text).edges()} == {GeomType.LINE}


def test_tracing_keeps_the_shape_of_the_letters():
    outline = label_outline("ASCOLTO")
    assert len(outline.faces()) == 7
    assert len(outline.faces().sort_by(Axis.X)[0].inner_wires()) == 1
    assert all(face.normal_at().Z > 0 for face in outline.faces())
    assert outline.area == pytest.approx(21.9, abs=0.3)
