import pytest
from helpers import inside, nominal

from drezimo_case import __main__ as cli
from drezimo_case.gauge import build_label_gauge
from drezimo_case.inlay import CAP_HEIGHT, LABELS, label_outline

PLATE = (70.0, 63.0)
CUT_CORNER = 5.0
WIDTHS = (0.4, 0.5, 0.6, 0.7, 0.8, 1.0, 1.2)
ROWS = ((3.5, 20.5), (3.5, 27.0), (3.5, 33.5), (4.0, 40.0), (4.0, 47.0), (4.0, 54.0))
EDGE = 0.04


@pytest.fixture(scope="module")
def gauge(board, p):
    return build_label_gauge(board, p)


@pytest.fixture(scope="module")
def plate(gauge):
    return gauge[0]


@pytest.fixture(scope="module")
def white(gauge):
    return gauge[1]


def test_it_is_a_flat_plate_with_the_decks_thickness_and_inlay_depth(plate, white, p):
    box = plate.bounding_box()
    assert (box.size.X, box.size.Y) == pytest.approx(PLATE)
    assert (box.min.Z, box.max.Z) == pytest.approx((0.0, p.deck_thickness))
    assert len(plate.solids()) == 1

    inlay = white.bounding_box()
    assert (inlay.min.Z, inlay.max.Z) == pytest.approx((p.deck_thickness - p.inlay_depth, p.deck_thickness))
    assert (plate & white).volume == pytest.approx(0.0, abs=1e-6)
    uncut = (PLATE[0] * PLATE[1] - CUT_CORNER**2 / 2) * p.deck_thickness
    assert plate.volume + white.volume == pytest.approx(uncut, rel=1e-4)


def test_white_bars_step_from_0_4_to_1_2_mm(white, p):
    z = p.deck_thickness - 0.1
    for i, width in enumerate(WIDTHS):
        x, half = 6.0 + 5.0 * i, width / 2
        for y in (3.1, 8.9):
            assert inside(white, x - half + EDGE, y, z) and inside(white, x + half - EDGE, y, z)
            assert not inside(white, x - half - EDGE, y, z) and not inside(white, x + half + EDGE, y, z)
        assert not inside(white, x, 2.9, z) and not inside(white, x, 9.1, z)


def test_black_gaps_step_the_same_way_between_white_blocks(plate, white, p):
    z = p.deck_thickness - 0.1
    for i, width in enumerate(WIDTHS):
        x, half = 7.0 + 9.0 * i, width / 2
        assert inside(plate, x - half + EDGE, 14.5, z) and inside(plate, x + half - EDGE, 14.5, z)
        assert inside(white, x - half - EDGE, 14.5, z) and inside(white, x + half + EDGE, 14.5, z)
        assert inside(white, x - half - 2.0 + EDGE, 14.5, z) and not inside(white, x - half - 2.0 - EDGE, 14.5, z)
        assert inside(white, x + half + 2.0 - EDGE, 14.5, z) and not inside(white, x + half + 2.0 + EDGE, 14.5, z)


def test_the_six_labels_come_at_the_decks_capital_height_and_one_size_up(white):
    letters = [solid.bounding_box() for solid in white.solids()]
    pairs = list(zip(LABELS[::2], LABELS[1::2]))
    for (cap, top), pair in zip(ROWS, pairs + pairs):
        drawn = label_outline("H", cap).bounding_box().size.Y
        row = [box for box in letters if top - 0.5 < box.center().Y < top + drawn + 0.5]
        assert len(row) == sum(len(label_outline(text).faces()) for text in pair)
        assert min(box.min.X for box in row) == pytest.approx(3.0, abs=0.01)
        assert max(box.max.X for box in row) == pytest.approx(67.0, abs=0.01)
        assert max(box.size.Y for box in row) == pytest.approx(drawn, rel=0.05)
        assert min(box.min.Y for box in row) >= top - 0.03 * drawn
        assert max(box.max.Y for box in row) <= top + drawn + 0.03 * drawn
    assert ROWS[0][0] == CAP_HEIGHT
    assert len(letters) == len(WIDTHS) * 3 + 2 * sum(len(label_outline(text).faces()) for text in LABELS)


def test_the_corner_next_to_the_thinnest_samples_is_cut(plate):
    assert not inside(plate, 1.0, 1.0, 1.0)
    assert inside(plate, 4.0, 4.0, 1.0)
    assert inside(plate, 69.0, 1.0, 1.0) and inside(plate, 1.0, 62.0, 1.0) and inside(plate, 69.0, 62.0, 1.0)


def test_command_writes_the_gauge_and_says_nothing_about_estimates(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    cli.main(["label-gauge"])
    assert {path.name for path in tmp_path.iterdir()} == {"label-gauge.step", "label-gauge.3mf", "label-gauge.png"}
    assert "estimates" not in capsys.readouterr().out
