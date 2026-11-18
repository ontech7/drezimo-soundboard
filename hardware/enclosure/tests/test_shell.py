import math

import pytest
from build123d import Align, Box, Pos, RectangleRounded, extrude
from helpers import NOMINAL, inside

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


def sides(rect, by):
    """A point `by` inside each side of a rectangle (outside it when negative), at the middle of that side."""
    x, y = rect.centre
    return ((rect.x0 + by, y), (rect.x1 - by, y), (x, rect.y0 + by), (x, rect.y1 - by))


def test_the_window_opens_on_the_lit_area_with_a_land_and_a_bevel(shell_body, p):
    underside = p.deck_top - p.deck_thickness
    x, y = p.lit.centre
    for z in (underside + 0.05, p.deck_top - 0.05):
        assert not inside(shell_body, x, y, z)

    lip = underside + p.window_land / 2
    assert all(not inside(shell_body, px, py, lip) for px, py in sides(p.window, 0.1))
    assert all(inside(shell_body, px, py, lip) for px, py in sides(p.window, -0.1))

    bevel = p.deck_thickness - p.window_land
    top = p.deck_top - 0.02
    assert all(not inside(shell_body, px, py, top) for px, py in sides(p.window, -bevel + 0.12))
    assert all(inside(shell_body, px, py, top) for px, py in sides(p.window, -bevel - 0.1))


def test_no_wall_of_the_window_hides_a_lit_pixel_seen_from_the_front_at_45_degrees(shell_body, p):
    pixels = NOMINAL["M4a"] - 0.7
    underside = p.deck_top - p.deck_thickness
    for x in (p.lit.x0 + 1, p.lit.centre[0], p.lit.x1 - 1):
        for z in (underside + 0.02, underside + p.window_land, underside + 1.0, p.deck_top - 0.02):
            assert not inside(shell_body, x, p.lit.y1 + (z - pixels), z), (x, z)


def test_the_lip_covers_the_edges_of_the_glass(shell_body, p):
    lip = p.deck_top - p.deck_thickness + 0.1
    assert all(inside(shell_body, px, py, lip) for px, py in sides(p.glass, 0.15))


def test_the_encoder_has_a_pocket_and_a_hole_for_its_shaft(shell_body, p, board):
    underside = p.deck_top - p.deck_thickness
    x, y = board.encoder_shaft
    for z in (underside + 0.1, p.encoder_pocket_top + 0.1, p.deck_top - 0.1):
        assert not inside(shell_body, x, y, z)
        assert not inside(shell_body, x + p.shaft_hole / 2 - 0.1, y, z)
    assert inside(shell_body, x + p.shaft_hole / 2 + 0.2, y, p.deck_top - 0.1)

    pocket = p.encoder_pocket
    for px, py in ((pocket.x0 + 0.2, pocket.y0 + 0.2), (pocket.x1 - 0.2, pocket.y1 - 0.2)):
        assert not inside(shell_body, px, py, p.encoder_pocket_top - 0.1)
        assert inside(shell_body, px, py, p.encoder_pocket_top + 0.1)
    assert all(inside(shell_body, px, py, underside + 0.1) for px, py in sides(pocket, -0.2))


def test_the_usb_plug_passes_the_back_wall_and_the_connector_passes_the_rim(shell_body, p):
    window = p.usb_window
    x, z = window.centre
    assert not inside(shell_body, x, 0.5, z) and not inside(shell_body, x, 2.0, z)
    for px, pz in sides(window, -0.2):
        assert inside(shell_body, px, 0.5, pz)
    for px, pz in sides(window, 0.2):
        assert not inside(shell_body, px, 0.5, pz)

    # Under the window the wall is no thicker than the rim, so the connector goes down past it with the board.
    between = (p.seat + window.y0) / 2
    assert not inside(shell_body, x, p.rim + 0.3, between)
    assert inside(shell_body, x, p.rim - 0.3, p.seat - 0.5)
    assert inside(shell_body, window.x0 - 0.5, p.rim + 0.3, between)
