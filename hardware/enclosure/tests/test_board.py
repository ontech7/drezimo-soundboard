import pytest

from drezimo_case.board import load_board


def test_outline_and_case():
    b = load_board()
    assert b.outline == pytest.approx((124.3, 190.35), abs=0.01)
    assert b.case_size == pytest.approx((130.3, 196.35), abs=0.01)


def test_key_centres():
    b = load_board()
    for c in range(6):
        assert b.key_centre(37 + c) == pytest.approx((17.525 + 19.05 * c, 59.525), abs=0.01)
        for r in range(6):
            assert b.key_centre(r * 6 + c + 1) == pytest.approx((17.525 + 19.05 * c, 83.575 + 19.05 * r), abs=0.01)


def test_holes():
    expected = {(x, y) for x in (6.5, 123.8) for y in (6.5, 131.2, 189.85)}
    assert {(round(x, 2), round(y, 2)) for x, y in load_board().holes} == expected
