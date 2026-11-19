import pytest
from build123d import Align, Box, Pos
from helpers import inside

MID = 98.0
DIRECTIONS = ((1, 0), (-1, 0), (0, 1), (0, -1))


def towards_the_middle(p, x, y):
    """Unit steps from a screw hole away from the walls it stands by."""
    width, length = p.case_size
    return (1 if x < width / 2 else -1), (1 if y < length / 2 else -1)


def by_a_side_wall(p, x):
    width, _ = p.case_size
    return x < 10 or x > width - 10


def test_the_bottom_is_one_piece_inside_the_shells_rim(bottom, p):
    assert len(bottom.solids()) == 1
    box = bottom.bounding_box()
    edge = p.rim + p.bottom_fit
    width, length = p.case_size
    assert (box.min.X, box.min.Y, box.max.X, box.max.Y) == pytest.approx((edge, edge, width - edge, length - edge), abs=0.01)
    assert (box.min.Z, box.max.Z) == pytest.approx((p.wall_bottom, p.standoff_top))


def test_the_plate_lies_flush_with_the_rims_end_and_short_of_the_seat(bottom, p, board):
    x, y = board.key_centre(15)
    x, y = x + 4, y - 4
    assert inside(bottom, x, y, p.wall_bottom + 0.1)
    assert inside(bottom, x, y, p.plate_top - 0.1)
    assert not inside(bottom, x, y, p.plate_top + 0.1)
    assert p.plate_top < p.seat


def test_the_bottom_goes_into_the_shell(bottom, shell_body, p):
    assert (bottom & shell_body).volume == pytest.approx(0.0, abs=0.01)
    assert not inside(bottom, p.rim + 0.1, MID, p.wall_bottom + 0.5)
    assert inside(bottom, p.rim + 0.3, MID, p.wall_bottom + 0.5)


def test_the_plates_outside_edge_is_chamfered(bottom, p):
    # The first layers print a little wide, and this edge has to enter the rim.
    edge = p.rim + p.bottom_fit
    assert not inside(bottom, edge + 0.15, MID, p.wall_bottom + 0.1)
    assert inside(bottom, edge + 0.45, MID, p.wall_bottom + 0.1)
    assert inside(bottom, edge + 0.1, MID, p.wall_bottom + 0.6)


def test_a_standoff_under_each_screw_hole_reaches_the_board(bottom, p, board):
    z = p.standoff_top - 0.2
    for x, y in board.holes:
        for dx, dy in DIRECTIONS:
            assert inside(bottom, x + 2.5 * dx, y + 2.5 * dy, z), (x, y, dx, dy)
            assert not inside(bottom, x + (p.standoff_radius + 0.2) * dx, y + (p.standoff_radius + 0.2) * dy, z), (x, y, dx, dy)
            assert not inside(bottom, x + 2.5 * dx, y + 2.5 * dy, p.standoff_top + 0.05)


def test_the_screw_passes_through_the_bottom_to_a_countersunk_seat(bottom, p, board):
    for x, y in board.holes:
        for z in (p.wall_bottom + 0.05, p.plate_top, p.seat_wide, p.seat_narrow + 0.5, p.standoff_top - 0.05):
            assert not inside(bottom, x, y, z), (x, y, z)
        dx, _ = towards_the_middle(p, x, y)

        under_the_board = p.standoff_top - 0.3
        assert not inside(bottom, x + (p.screw_clearance / 2 - 0.1) * dx, y, under_the_board)
        assert inside(bottom, x + (p.screw_clearance / 2 + 0.1) * dx, y, under_the_board)

        in_the_plate = p.wall_bottom + 1.0
        assert not inside(bottom, x + (p.counterbore / 2 - 0.1) * dx, y, in_the_plate)
        assert inside(bottom, x + (p.counterbore / 2 + 0.2) * dx, y, in_the_plate)

        halfway = (p.seat_wide + p.seat_narrow) / 2
        radius = (p.counterbore + p.screw_clearance) / 4
        assert not inside(bottom, x + (radius - 0.1) * dx, y, halfway)
        assert inside(bottom, x + (radius + 0.15) * dx, y, halfway)


def test_a_column_keeps_off_the_shells_wall_and_still_closes_round_its_counterbore(bottom, p, board):
    width, length = p.case_size
    z = p.seat + 1.0
    for x, y in board.holes:
        dx, dy = towards_the_middle(p, x, y)
        wall_x = p.wall if dx > 0 else width - p.wall
        assert not inside(bottom, wall_x + 0.2 * dx, y, z), (x, y)
        between = (wall_x + 0.3 * dx + x - p.counterbore / 2 * dx) / 2
        assert abs(wall_x + 0.3 * dx - (x - p.counterbore / 2 * dx)) >= 0.45
        assert inside(bottom, between, y, z), (x, y)
        if y < 10 or y > length - 10:
            wall_y = p.wall if dy > 0 else length - p.wall
            assert not inside(bottom, x, wall_y + 0.2 * dy, z), (x, y)
            assert inside(bottom, x, (wall_y + 0.3 * dy + y - p.counterbore / 2 * dy) / 2, z), (x, y)


def test_the_bottom_touches_the_board_with_its_six_standoffs_and_nothing_else(bottom, p):
    width, length = p.case_size
    at_the_board = Pos(0, 0, p.standoff_top - 0.1) * Box(width, length, 0.1, align=Align.MIN)
    assert len((bottom & at_the_board).solids()) == 6
