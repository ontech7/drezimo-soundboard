import pytest
from helpers import inside, nominal

from drezimo_case import __main__ as cli
from drezimo_case.logo_gauge import build_logo_gauge

PLATE = (60.0, 44.0)
CUT_CORNER = 5.0
LOGO_X, LOGO_SIZE = 15.0, (42.0, 10.15)
ROWS = (3.5, 17.0, 30.5)
# The small line's letters: 2.1 mm between the centres of their top and bottom strokes, plus a stroke.
SMALL_LETTERS = (2.1 + 0.35, 2.1 + 0.55, 2.1 + 0.65)
SHAPES_IN_A_LOGO = 53
MARK = 2.0


@pytest.fixture(scope="module")
def gauge(board, p):
    return build_logo_gauge(board, p)


@pytest.fixture(scope="module")
def plate(gauge):
    return gauge[0]


@pytest.fixture(scope="module")
def white(gauge):
    return gauge[1]


@pytest.fixture(scope="module")
def rows(white):
    """The boxes of the white shapes of each row: its marks, then its logo."""
    boxes = [solid.bounding_box() for solid in white.solids()]
    assert len(boxes) == sum(SHAPES_IN_A_LOGO + number for number in (1, 2, 3))
    split = []
    for top in ROWS:
        row = [box for box in boxes if top - 0.2 < box.center().Y < top + LOGO_SIZE[1] + 0.2]
        split.append(([box for box in row if box.max.X < LOGO_X - 1], [box for box in row if box.min.X > LOGO_X - 1]))
    return split


def test_it_is_a_flat_plate_with_the_decks_thickness_and_inlay_depth(plate, white, p):
    box = plate.bounding_box()
    assert (box.size.X, box.size.Y) == pytest.approx(PLATE)
    assert (box.min.Z, box.max.Z) == pytest.approx((0.0, p.deck_thickness))
    assert len(plate.solids()) == 1

    inlay = white.bounding_box()
    assert (inlay.min.Z, inlay.max.Z) == pytest.approx((p.deck_thickness - p.inlay_depth, p.deck_thickness))
    uncut = (PLATE[0] * PLATE[1] - CUT_CORNER**2 / 2) * p.deck_thickness
    assert plate.volume + white.volume == pytest.approx(uncut, rel=1e-4)


def test_it_carries_the_logo_three_times_at_the_decks_width(rows):
    for top, (_, logo) in zip(ROWS, rows):
        assert len(logo) == SHAPES_IN_A_LOGO
        assert min(box.min.X for box in logo) == pytest.approx(LOGO_X, abs=0.01)
        assert max(box.max.X for box in logo) == pytest.approx(LOGO_X + LOGO_SIZE[0], abs=0.11)
        assert min(box.min.Y for box in logo) == pytest.approx(top, abs=0.01)
        assert max(box.max.Y for box in logo) == pytest.approx(top + LOGO_SIZE[1], abs=0.01)


def test_the_rows_go_from_the_artwork_as_drawn_to_the_boldest_small_line(rows):
    for top, height, (_, logo) in zip(ROWS, SMALL_LETTERS, rows):
        small_line = [box for box in logo if box.min.X > LOGO_X + 12 and box.min.Y > top + 6]
        assert len(small_line) == 10
        assert [box.size.Y for box in small_line] == pytest.approx([height] * 10, abs=0.01)


def test_each_row_is_marked_with_as_many_squares_as_its_number(rows):
    for number, (top, (marks, _)) in enumerate(zip(ROWS, rows), start=1):
        assert len(marks) == number
        for i, mark in enumerate(sorted(marks, key=lambda box: box.min.X)):
            assert (mark.size.X, mark.size.Y) == pytest.approx((MARK, MARK))
            assert mark.min.X == pytest.approx(3.0 + 3.5 * i)
            assert mark.center().Y == pytest.approx(top + LOGO_SIZE[1] / 2)


def test_the_corner_next_to_the_first_row_is_cut(plate):
    assert not inside(plate, 1.0, 1.0, 1.0)
    assert inside(plate, 4.0, 4.0, 1.0)
    assert inside(plate, 59.0, 1.0, 1.0) and inside(plate, 1.0, 43.0, 1.0) and inside(plate, 59.0, 43.0, 1.0)


def test_command_writes_the_logo_gauge_and_says_nothing_about_estimates(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    cli.main(["logo-gauge"])
    assert {path.name for path in tmp_path.iterdir()} == {"logo-gauge.step", "logo-gauge.3mf", "logo-gauge.png"}
    assert "estimates" not in capsys.readouterr().out
