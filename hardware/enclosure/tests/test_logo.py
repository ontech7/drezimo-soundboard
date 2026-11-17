import pytest
from build123d import GeomType
from helpers import flat, pieces_and_counters
from shapely.geometry import Point
from shapely.ops import unary_union

from drezimo_case.inlay import MIN_OPENING, MIN_STROKE, PRINT_LOSS
from drezimo_case.logo import WIDTH, logo_outline, read_artwork

# The artwork is 60 × 14.5 units: the grid of pads, then DREZIMO over SOUNDBOARD.
UNIT = WIDTH / 60
GRID = ["111110", "100001", "100001", "100001", "100001", "111110"]
MARGIN = 0.01


def as_drawn(width=WIDTH):
    return logo_outline(width, loss=0.0, min_stroke=0.0)


def pads(outline, unit=UNIT):
    return [face for face in outline.faces() if face.bounding_box().max.X < 15 * unit]


def title(outline, unit=UNIT):
    return [face for face in outline.faces() if face.bounding_box().min.X > 15 * unit and face.bounding_box().max.Y < 8 * unit]


def small_line(outline, unit=UNIT):
    return [face for face in outline.faces() if face.bounding_box().min.X > 15 * unit and face.bounding_box().min.Y > 8 * unit]


def small_stroke(outline, unit=UNIT):
    # Round letters are traced with short straight sides, so they come a few microns short.
    heights = [letter.bounding_box().size.Y for letter in small_line(outline, unit)]
    assert max(heights) - min(heights) < 0.005
    return max(heights) - 3 * unit


def test_the_logo_is_42_mm_wide_with_the_artworks_proportions():
    assert WIDTH == 42.0
    box = as_drawn().bounding_box()
    assert (box.min.X, box.min.Y) == pytest.approx((0.0, 0.0), abs=0.01)
    assert (box.size.X, box.size.Y) == pytest.approx((42.0, 10.15), abs=0.01)


def test_it_has_the_grid_and_the_two_lines_of_the_wordmark():
    outline = as_drawn()
    assert len(outline.faces()) == 36 + 7 + 10
    assert len(pads(outline)) == 36

    drezimo, soundboard = title(outline), small_line(outline)
    assert (len(drezimo), sum(len(letter.inner_wires()) for letter in drezimo)) == (7, 3)
    assert (len(soundboard), sum(len(letter.inner_wires()) for letter in soundboard)) == (10, 8)
    assert [letter.bounding_box().size.Y for letter in drezimo] == pytest.approx([7 * UNIT] * 7, abs=0.01)
    assert [letter.bounding_box().size.Y for letter in soundboard] == pytest.approx([3.5 * UNIT] * 10, abs=0.01)
    assert max(letter.bounding_box().max.Y for letter in drezimo) < min(letter.bounding_box().min.Y for letter in soundboard)


def test_the_lit_pads_draw_a_d():
    drawn = [["?"] * 6 for _ in range(6)]
    for pad in pads(as_drawn()):
        box = pad.bounding_box()
        column, row = (round((centre / UNIT - 1) / 2.5) for centre in (box.center().X, box.center().Y))
        assert box.size.X == pytest.approx(box.size.Y, abs=0.01)
        drawn[row][column] = {1.4: "1", 0.7: "0"}[round(box.size.X, 2)]
    assert ["".join(row) for row in drawn] == GRID


def test_the_wordmark_reads_upright_and_left_to_right():
    white = unary_union([flat(face) for face in as_drawn().faces()])
    # The D's shoulder bulges away from its stem, and the Z's stroke falls from right to left.
    assert white.contains(Point(16.39, 1.11)) and not white.contains(Point(15.81, 1.69))
    assert white.contains(Point(29.4, 1.75)) and not white.contains(Point(28.0, 1.75))


def test_strokes_and_dots_are_drawn_bigger_by_what_printing_takes_off():
    outline = logo_outline()
    assert [letter.bounding_box().size.Y for letter in title(outline)] == pytest.approx([7 * UNIT + PRINT_LOSS] * 7, abs=0.01)
    sizes = sorted({round(pad.bounding_box().size.X, 2) for pad in pads(outline)})
    assert sizes == pytest.approx([UNIT + PRINT_LOSS, 2 * UNIT])


def test_the_lit_pads_stay_as_drawn_to_keep_the_black_between_them():
    top_row = sorted((pad.bounding_box() for pad in pads(logo_outline()) if pad.bounding_box().min.Y < 0.01), key=lambda box: box.min.X)
    assert len(top_row) == 5
    for left, right in zip(top_row, top_row[1:]):
        assert right.min.X - left.max.X == pytest.approx(0.5 * UNIT, abs=0.01)


def test_the_small_line_is_thickened_as_far_as_its_counters_allow():
    # Its bars are 1.05 mm apart, centre to centre: a 0.65 mm stroke leaves MIN_OPENING of black between them.
    assert small_stroke(logo_outline()) == pytest.approx(1.05 - MIN_OPENING, abs=0.01)
    assert small_stroke(logo_outline(min_stroke=0.0)) == pytest.approx(0.35 + PRINT_LOSS, abs=0.01)
    assert small_stroke(as_drawn()) == pytest.approx(0.35, abs=0.01)


def test_a_wider_logo_gets_the_thinnest_stroke_that_prints_and_no_more():
    assert small_stroke(logo_outline(48.0), unit=0.8) == pytest.approx(MIN_STROKE, abs=0.01)
    assert small_stroke(logo_outline(72.0), unit=1.2) == pytest.approx(0.6 + PRINT_LOSS, abs=0.01)


def test_no_opening_in_the_small_line_is_narrower_than_the_limit():
    for letter in small_line(logo_outline()):
        shape = flat(letter)
        assert pieces_and_counters(shape.buffer(MIN_OPENING / 2 - MARGIN)) == pieces_and_counters(shape)


def test_the_outline_is_plain_polygons_facing_up():
    # Curves do not survive a STEP export; straight segments do.
    outline = logo_outline()
    assert {edge.geom_type for edge in outline.edges()} == {GeomType.LINE}
    assert all(face.normal_at().Z > 0 for face in outline.faces())


def test_artwork_with_a_shape_it_cannot_trace_is_refused(tmp_path):
    drawing = tmp_path / "logo.svg"
    drawing.write_text('<svg xmlns="http://www.w3.org/2000/svg"><circle cx="1" cy="1" r="1"/></svg>')
    with pytest.raises(ValueError, match="circle"):
        read_artwork(drawing)

    drawing.write_text('<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0C1 1 2 2 3 3" stroke-width="1"/></svg>')
    with pytest.raises(ValueError, match="C"):
        read_artwork(drawing)
