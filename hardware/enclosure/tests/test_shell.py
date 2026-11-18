import math

import pytest
from build123d import Align, Box, Pos, RectangleRounded, extrude
from helpers import inside

from drezimo_case.board import INSET

MID = 98.0
# The board's corners are drawn with this radius.
BOARD_CORNER = 5.0


def wells(p):
    return (p.fn_well, p.pad_well)


def in_a_well(p, x, y):
    return any(well.x0 < x < well.x1 and well.y0 < y < well.y1 for well in wells(p))


def test_the_shell_is_one_piece_from_the_bottom_to_the_deck(shell_body, p):
    assert len(shell_body.solids()) == 1
    box = shell_body.bounding_box()
    assert (box.min.X, box.min.Y, box.max.X, box.max.Y) == pytest.approx((0.0, 0.0, *p.case_size), abs=0.01)
    assert (box.min.Z, box.max.Z) == pytest.approx((p.wall_bottom, p.deck_top))


def test_the_wall_thins_to_a_rim_where_the_bottom_seats(shell_body, p):
    assert inside(shell_body, 1.0, MID, p.seat - 1.0)
    assert not inside(shell_body, 1.8, MID, p.seat - 1.0)
    assert inside(shell_body, 1.8, MID, p.seat + 1.0)
    assert not inside(shell_body, 3.0, MID, p.seat + 1.0)
    # All round, the back wall included.
    assert inside(shell_body, 80.0, 1.0, p.seat - 1.0) and not inside(shell_body, 80.0, 1.8, p.seat - 1.0)


def test_the_corner_under_the_deck_is_filled_at_45_degrees(shell_body, p):
    # Without it the chamfer leaves 1.06 mm of wall there.
    z = p.deck_top - p.deck_thickness - 0.6
    assert inside(shell_body, 3.2, MID, z)
    assert not inside(shell_body, 4.2, MID, z)
    assert not inside(shell_body, 3.2, MID, z - 1.0)


def test_a_boss_stands_on_the_board_round_each_screw_hole(shell_body, p, board):
    assert len(board.holes) == 6
    for x, y in board.holes:
        for dx, dy in ((2.5, 0), (-2.5, 0), (0, 2.5), (0, -2.5)):
            assert inside(shell_body, x + dx, y + dy, 0.3), (x, y, dx, dy)
            assert not inside(shell_body, x + dx, y + dy, -0.1)
        assert not inside(shell_body, x, y, 0.05)
        assert not inside(shell_body, x, y, p.screw_hole_top - 0.05)
        assert not inside(shell_body, x + p.screw_hole / 2 - 0.1, y, 2.0)
        assert inside(shell_body, x + p.screw_hole / 2 + 0.2, y, 2.0)
        assert inside(shell_body, x, y, p.screw_hole_top + 0.1)
        assert inside(shell_body, x, y, p.deck_top - 0.1)


def test_no_boss_and_no_screw_hole_shows_in_a_well(shell_body, p, board):
    for x, y in board.holes:
        for step in range(24):
            angle = math.tau * step / 24
            for radius in (p.screw_hole / 2 - 0.05, p.boss_radius - 0.2):
                px, py = x + radius * math.cos(angle), y + radius * math.sin(angle)
                if in_a_well(p, px, py):
                    assert not inside(shell_body, px, py, p.well_floor + 0.3), (x, y, step)
                    # The floor is whole over the screw hole.
                    assert inside(shell_body, px, py, p.well_floor - 0.1), (x, y, step)


def test_only_the_bosses_touch_the_board_and_the_board_drops_in(shell_body, p, board):
    width, length = board.outline
    inner = (width - 0.8, length - 0.8)
    at_the_board = Pos(INSET + 0.4, INSET + 0.4, 0.05) * Box(*inner, 0.1, align=Align.MIN)
    assert len((shell_body & at_the_board).solids()) == 6

    grown = Pos(INSET + width / 2, INSET + length / 2, p.wall_bottom - 1) * RectangleRounded(width + 0.8, length + 0.8, BOARD_CORNER + 0.4)
    way_in = extrude(grown, amount=-p.wall_bottom + 1 - 0.01)
    assert (shell_body & way_in).volume == pytest.approx(0.0, abs=1e-6)
