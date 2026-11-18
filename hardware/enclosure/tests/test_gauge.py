import pytest
from helpers import inside, nominal

from drezimo_case import __main__ as cli
from drezimo_case.gauge import build_inlay_gauge
from drezimo_case.inlay import LABELS, label_outline

PLATE = (70.0, 101.0)
CUT_CORNER = 5.0
BACKING = 0.4
WIDTHS = (0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0)
EDGE = 0.04

LOSS, MIN_GAP = 0.1, 0.3
LABEL_ROWS = ((3.0, 20.5), (3.0, 26.5), (3.5, 32.5), (3.5, 39.0), (4.0, 45.5), (4.0, 52.5))
THIRD = PLATE[0] / 3

LOGO_X, LOGO_SIZE = 15.0, (42.0, 10.15)
# What each logo's strokes and dots are drawn bigger by, and its top.
LOGOS = ((0.0, 60.5), (0.1, 74.0), (0.2, 87.5))
SHAPES_IN_A_LOGO = 53
MARK = 2.0


@pytest.fixture(scope="module")
def gauge(board, p):
    return build_inlay_gauge(board, p)


@pytest.fixture(scope="module")
def plate(gauge):
    return gauge[0]


@pytest.fixture(scope="module")
def white(gauge):
    return gauge[1]


@pytest.fixture(scope="module")
def boxes(white):
    return [solid.bounding_box() for solid in white.solids()]


@pytest.fixture(scope="module")
def letters(boxes):
    return [box for box in boxes if 18.0 < box.center().Y < 58.0]


@pytest.fixture(scope="module")
def logos(boxes):
    """The boxes of the white shapes of each logo's row: its marks, then its logo."""
    rows = []
    for _, top in LOGOS:
        row = [box for box in boxes if top - 0.2 < box.center().Y < top + LOGO_SIZE[1] + 0.2]
        rows.append(([box for box in row if box.max.X < LOGO_X - 1], [box for box in row if box.min.X > LOGO_X - 1]))
    return rows


def test_it_is_a_flat_plate_with_the_decks_inlay_depth_and_little_black_behind(plate, white, p):
    top = p.inlay_depth + BACKING
    box = plate.bounding_box()
    assert (box.size.X, box.size.Y) == pytest.approx(PLATE)
    assert (box.min.Z, box.max.Z) == pytest.approx((0.0, top))
    assert len(plate.solids()) == 1

    inlay = white.bounding_box()
    assert (inlay.min.Z, inlay.max.Z) == pytest.approx((BACKING, top))
    uncut = (PLATE[0] * PLATE[1] - CUT_CORNER**2 / 2) * top
    assert plate.volume + white.volume == pytest.approx(uncut, rel=1e-4)


def test_white_bars_step_from_0_2_to_1_0_mm(white, p):
    z = p.inlay_depth + BACKING - 0.1
    for i, width in enumerate(WIDTHS):
        x, half = 6.0 + 5.0 * i, width / 2
        for y in (3.1, 8.9):
            assert inside(white, x - half + EDGE, y, z) and inside(white, x + half - EDGE, y, z)
            assert not inside(white, x - half - EDGE, y, z) and not inside(white, x + half + EDGE, y, z)
        assert not inside(white, x, 2.9, z) and not inside(white, x, 9.1, z)


def test_black_gaps_step_the_same_way_between_white_blocks(plate, white, p):
    z = p.inlay_depth + BACKING - 0.1
    for i, width in enumerate(WIDTHS):
        x, half = 7.0 + 9.0 * i, width / 2
        assert inside(plate, x - half + EDGE, 14.5, z) and inside(plate, x + half - EDGE, 14.5, z)
        assert inside(white, x - half - EDGE, 14.5, z) and inside(white, x + half + EDGE, 14.5, z)
        assert inside(white, x - half - 2.0 + EDGE, 14.5, z) and not inside(white, x - half - 2.0 - EDGE, 14.5, z)
        assert inside(white, x + half + 2.0 - EDGE, 14.5, z) and not inside(white, x + half + 2.0 + EDGE, 14.5, z)


def test_the_six_labels_come_at_three_capital_heights_three_to_a_row(letters):
    for (cap, top), texts in zip(LABEL_ROWS, (LABELS[:3], LABELS[3:]) * 3):
        drawn = cap + LOSS
        row = [box for box in letters if top - 0.5 < box.center().Y < top + drawn + 0.5]
        for column, text in enumerate(texts):
            label = [box for box in row if THIRD * column < box.center().X < THIRD * (column + 1)]
            assert len(label) == len(label_outline(text).faces())
            middle = (min(box.min.X for box in label) + max(box.max.X for box in label)) / 2
            assert middle == pytest.approx(THIRD * (column + 0.5), abs=0.01)
        # Flat letters are as tall as the capitals are drawn; round ones overshoot a little.
        assert min(box.size.Y for box in row) == pytest.approx(drawn, abs=0.02)
        assert min(box.min.Y for box in row) >= top - 0.03 * drawn
        assert max(box.max.Y for box in row) <= top + drawn + 0.03 * drawn
    assert len(letters) == 3 * sum(len(label_outline(text).faces()) for text in LABELS)


def test_the_labels_are_drawn_on_the_guess_the_gauge_was_printed_to_check(letters):
    # Half the loss the deck's labels are drawn for, and no thickening.
    stop = sorted((box for box in letters if 20.0 < box.center().Y < 24.5 and box.center().X > 2 * THIRD), key=lambda box: box.min.X)
    _, t, o, _ = stop
    assert t.size.Y == pytest.approx(3.0 + LOSS, abs=0.01)
    assert o.min.X - t.max.X == pytest.approx(MIN_GAP, abs=0.01)


def test_the_logo_comes_three_times_at_the_decks_width(logos):
    for (_, top), (_, logo) in zip(LOGOS, logos):
        assert len(logo) == SHAPES_IN_A_LOGO
        assert min(box.min.X for box in logo) == pytest.approx(LOGO_X, abs=0.01)
        assert max(box.max.X for box in logo) == pytest.approx(LOGO_X + LOGO_SIZE[0], abs=0.11)
        assert min(box.min.Y for box in logo) == pytest.approx(top, abs=0.01)
        assert max(box.max.Y for box in logo) == pytest.approx(top + LOGO_SIZE[1], abs=0.01)


def test_each_logo_is_drawn_bigger_by_another_guess_of_the_print_loss(logos):
    # The small line's strokes are 0.35 mm as drawn, and 2.1 mm apart from the top of a letter to its foot.
    for (loss, top), (_, logo) in zip(LOGOS, logos):
        small_line = [box for box in logo if box.min.X > LOGO_X + 12 and box.min.Y > top + 6]
        assert len(small_line) == 10
        assert [box.size.Y for box in small_line] == pytest.approx([2.1 + 0.35 + loss] * 10, abs=0.01)


def test_each_logo_is_marked_with_as_many_squares_as_its_number(logos):
    for number, ((_, top), (marks, _)) in enumerate(zip(LOGOS, logos), start=1):
        assert len(marks) == number
        for i, mark in enumerate(sorted(marks, key=lambda box: box.min.X)):
            assert (mark.size.X, mark.size.Y) == pytest.approx((MARK, MARK))
            assert mark.min.X == pytest.approx(3.0 + 3.5 * i)
            assert mark.center().Y == pytest.approx(top + LOGO_SIZE[1] / 2)


def test_nothing_else_is_white(boxes, letters, logos):
    samples = len(WIDTHS) * 3
    assert len(boxes) == samples + len(letters) + sum(len(marks) + len(logo) for marks, logo in logos)


def test_the_corner_next_to_the_thinnest_samples_is_cut(plate):
    assert not inside(plate, 1.0, 1.0, 1.0)
    assert inside(plate, 4.0, 4.0, 1.0)
    assert inside(plate, 69.0, 1.0, 1.0) and inside(plate, 1.0, 100.0, 1.0) and inside(plate, 69.0, 100.0, 1.0)


def test_command_writes_the_gauge_and_says_nothing_about_estimates(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    cli.main(["inlay-gauge"])
    assert {path.name for path in tmp_path.iterdir()} == {"inlay-gauge.step", "inlay-gauge.3mf", "inlay-gauge.png"}
    assert "estimates" not in capsys.readouterr().out
