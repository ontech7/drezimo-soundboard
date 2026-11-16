import pytest
from build123d import Align, Box, Pos
from helpers import NOMINAL, inside

FEET = ((4.9, 47.0), (4.9, 86.0), (44.6, 71.55), (45.35, 83.575))
COUPON_KEYS = (37, 38, 1, 2)


def test_size(body, p):
    box = body.bounding_box()
    assert (box.min.X, box.min.Y, box.max.X, box.max.Y) == pytest.approx((0.0, 40.0, 46.1, 93.1), abs=0.01)
    assert (box.min.Z, box.max.Z) == pytest.approx((-1.0, p.deck_top))


def test_coupon_is_one_piece(body):
    assert len(body.solids()) == 1


def test_pressed_keycaps_have_a_millimetre_of_air(body, p, board):
    side = NOMINAL["M2c"] + 1.2
    for n in COUPON_KEYS:
        x, y = board.key_centre(n)
        swept = Pos(x, y, p.well_floor + 0.01) * Box(side, side, 20, align=(Align.CENTER, Align.CENTER, Align.MIN))
        assert (body & swept).volume == pytest.approx(0.0, abs=1e-6)


def test_feet_stand_on_the_pcb(body):
    for x, y in FEET:
        assert inside(body, x, y, 0.05)
        assert not inside(body, x, y, -0.05)


def test_feet_stay_below_the_keycaps(body, p):
    x, y = FEET[3]
    assert not inside(body, x, y, p.well_floor + 0.1)


def test_body_and_inlay_do_not_overlap(body, inlay, p):
    assert (body & inlay).volume == pytest.approx(0.0, abs=1e-6)
    assert inlay.bounding_box().max.Z == pytest.approx(p.deck_top)


def test_inlay_is_the_first_two_labels(inlay, board):
    box = inlay.bounding_box()
    assert box.min.X > board.key_centre(37)[0] - 9.0
    assert box.max.X < board.key_centre(38)[0] + 9.0
    assert inlay.volume > 5.0


def test_no_gap_wide_enough_to_show_the_pcb(p):
    assert (p.opening - NOMINAL["M3a"]) / 2 <= 0.25
