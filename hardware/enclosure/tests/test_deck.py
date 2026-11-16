import pytest
from helpers import inside

from drezimo_case.board import PITCH


def test_rim_and_chamfer(deck, p):
    assert inside(deck, 4.0, 120.0, p.deck_top - 0.1)
    assert not inside(deck, 1.0, 120.0, p.deck_top - 0.1)
    assert inside(deck, 1.0, 120.0, p.deck_top - 3.5)


def test_well_is_open_above_the_floor(deck, p, board):
    x, y = board.key_centre(1)
    rib = x + PITCH / 2
    assert not inside(deck, rib, y, p.well_floor + 0.5)
    assert inside(deck, rib, y, p.well_floor - 0.1)
    assert not inside(deck, rib, y, p.rib_bottom - 0.1)


def test_openings_are_through(deck, p, board):
    for n in range(1, 43):
        x, y = board.key_centre(n)
        assert not inside(deck, x, y, p.well_floor - 0.1)
        assert not inside(deck, x + p.opening / 2 - 0.1, y, p.well_floor - 1.5)
        assert inside(deck, x + p.opening / 2 + 0.2, y, p.well_floor - 1.5)


def test_openings_widen_towards_the_board(deck, p, board):
    x, y = board.key_centre(1)
    assert not inside(deck, x + p.opening / 2 + 0.3, y, p.rib_bottom + 0.1)
    assert inside(deck, x + p.opening / 2 + 0.3, y, p.rib_bottom + 0.6)


def test_curb_joins_the_grid_to_the_deck(deck, p):
    x = p.pad_well.x0 - 0.8
    assert inside(deck, x, 120.0, p.rib_bottom + 0.1)
    assert inside(deck, x, 120.0, p.deck_top - 0.1)
    assert not inside(deck, p.pad_well.x0 - 2.0, 120.0, p.rib_bottom + 0.1)


def test_bar_reaches_the_deck(deck, p):
    assert inside(deck, 65.0, (p.bar.y0 + p.bar.y1) / 2, p.deck_top - 0.1)


def test_nothing_proud_of_the_deck(deck, p):
    assert deck.bounding_box().max.Z == pytest.approx(p.deck_top)


def test_deck_is_one_piece(deck):
    assert len(deck.solids()) == 1
