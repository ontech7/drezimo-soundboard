import math
from dataclasses import astuple

import pytest
import shapely
from build123d import Align, Box, Pos
from helpers import cut, inside
from shapely.geometry import Point, Polygon
from shapely.geometry import box as flat_box
from shapely.ops import unary_union

from drezimo_case.board import PITCH, USB_MODULE
from drezimo_case.bottom import LAYER, POST_GAP

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


def test_a_post_under_every_point_between_four_keys_stops_just_short_of_the_board(bottom, p):
    assert len(p.nodes) == 30
    for x, y in p.nodes:
        top = p.standoff_top - POST_GAP
        for z in (p.plate_top + 0.1, (p.plate_top + p.standoff_top) / 2, top - 0.05):
            assert inside(bottom, x, y, z), (x, y, z)
        # The board is carried by the screws: a post only stops it giving under a key.
        assert not inside(bottom, x, y, top + 0.05), (x, y)
        assert inside(bottom, x + 1.8, y, top - 0.05) and not inside(bottom, x + 2.2, y + 2.2, top - 0.05)


def test_ribs_run_between_the_posts_both_ways(bottom, p):
    top = p.standoff_top - 1.0
    assert top - p.plate_top >= 6.0
    nodes = sorted(p.nodes)
    for (x0, y0), (x1, y1) in zip(nodes, nodes[1:]):
        if x0 != x1:
            continue
        x, y = x0, (y0 + y1) / 2
        assert inside(bottom, x, y, top - 0.1) and not inside(bottom, x, y, top + 0.1), (x, y)
        assert inside(bottom, x + 0.5, y, top - 1.0) and not inside(bottom, x + 0.7, y, top - 1.0), (x, y)
    across = sorted(p.nodes, key=lambda node: (node[1], node[0]))
    for (x0, y0), (x1, y1) in zip(across, across[1:]):
        if y0 != y1:
            continue
        x, y = (x0 + x1) / 2, y0
        assert inside(bottom, x, y, top - 0.1) and not inside(bottom, x, y, top + 0.1), (x, y)
        assert inside(bottom, x, y + 0.5, top - 1.0) and not inside(bottom, x, y + 0.7, top - 1.0), (x, y)


def test_no_stretch_of_plain_plate_is_wider_than_a_key(bottom, p, board):
    # Pressed in the middle, a 1.8 mm plate between ribs a key apart gives hundredths of a millimetre.
    z = p.plate_top + 3.0
    for x, y in board.keys.values():
        left = max(line for line in p.rib_lines_x if line < x)
        right = min(line for line in p.rib_lines_x if line > x)
        back = max(line for line in p.rib_lines_y if line < y)
        front = min(line for line in p.rib_lines_y if line > y)
        assert right - left <= PITCH + 0.01 and front - back <= PITCH + 5.01
        for px, py in ((left, y), (right, y), (x, back), (x, front)):
            assert inside(bottom, px, py, z), (x, y, px, py)


def test_nowhere_on_the_plate_does_a_circle_of_plain_plate_wider_than_a_key_fit(bottom, p):
    # No rib can stand under the USB module: 27 mm of plain plate there gave ten times what a key's cell gives.
    plate = unary_union(cut(bottom, p.plate_top - 0.05))
    standing = unary_union(cut(bottom, p.plate_top + 0.05))
    stretches = [stretch for stretch in plate.difference(standing).geoms if stretch.area > 1.0]
    widest = max(shapely.maximum_inscribed_circle(stretch, tolerance=0.05).length for stretch in stretches)
    assert widest <= PITCH / 2


def test_under_the_usb_module_the_plate_is_thicker_and_keeps_out_of_the_plugs_way(bottom, p, board):
    module = board.usb_module
    x, y = module.centre
    top = p.wall_bottom + math.floor((p.standoff_top - p.under_the_module - p.wall_bottom) / LAYER + 1e-6) * LAYER
    assert p.standoff_top - p.under_the_module - LAYER < top <= p.standoff_top - p.under_the_module + 1e-9
    for px, py in ((x, y), (module.x0, module.y1), (module.x1, module.y1), (module.x0, module.y0 + 4.0), (module.x1, module.y0 + 4.0)):
        assert inside(bottom, px, py, top - 0.05) and not inside(bottom, px, py, top + 0.05), (px, py)
    # The plug's grip comes through the back wall as far as the connector's mouth: 3 mm behind it stay plain plate.
    window = p.usb_window
    assert module.x0 < window.x0 and window.x1 < module.x1
    for px in (window.x0, x, window.x1):
        assert not inside(bottom, px, module.y0 + 2.4, p.plate_top + 0.05), px
        assert inside(bottom, px, module.y0 + 3.0, p.plate_top + 0.05), px


def test_the_ribs_on_the_middle_screws_line_end_in_their_columns(bottom, p, board):
    z = p.plate_top + 1.0
    for x, y in board.holes:
        if y not in p.rib_lines_y:
            continue
        dx, _ = towards_the_middle(p, x, y)
        assert inside(bottom, x + 4.0 * dx, y, z)
        assert inside(bottom, x + 6.5 * dx, y, z)
    assert sum(1 for _, y in board.holes if y in p.rib_lines_y) == 2


def test_nothing_of_the_bottom_stands_under_a_part_of_the_board(bottom, p):
    # Half a millimetre clear sideways of all that hangs under the board, or 0.3 mm under how far it can hang;
    # under the USB module, the space the shell itself keeps under it.
    levels = [p.plate_top + 0.05 + 0.25 * step for step in range(int((p.standoff_top - p.plate_top) / 0.25) + 1)]
    cuts = {z: cut(bottom, z) for z in levels if z < p.standoff_top - 0.01}
    assert len(p.under) > 380
    for entry in p.under:
        lowest = p.standoff_top - (p.under_the_module if entry.part == USB_MODULE else entry.drop + 0.3)
        room = flat_box(entry.rect.x0 - 0.5, entry.rect.y0 - 0.5, entry.rect.x1 + 0.5, entry.rect.y1 + 0.5)
        for z, flat in cuts.items():
            if z > lowest:
                assert flat.intersection(room).area == pytest.approx(0.0, abs=1e-4), (entry, z)


def test_a_keyhole_takes_a_peg_in_and_lets_only_its_neck_slide(bottom, p):
    z = p.wall_bottom + 0.1
    for x, y in p.keyholes:
        entry = y + p.keyhole_slide
        for along in (y, (y + entry) / 2, entry):
            assert not inside(bottom, x, along, z), (x, along)
        # The neck's slot, 4 mm and play.
        assert not inside(bottom, x + 2.3, y + 1.0, z) and inside(bottom, x + 2.6, y + 1.0, z), (x, y)
        assert not inside(bottom, x, y - 2.3, z) and inside(bottom, x, y - 2.6, z), (x, y)
        # The head's way in, 7 mm and play.
        assert not inside(bottom, x + 3.8, entry, z) and inside(bottom, x + 4.1, entry, z), (x, y)
        assert not inside(bottom, x, entry + 3.8, z) and inside(bottom, x, entry + 4.1, z), (x, y)


def test_behind_each_keyhole_a_closed_pocket_for_the_pegs_head(bottom, p):
    # The roof is a bridge of fine strands: a head 2 mm high on a neck 0.4 mm longer than the plate keeps 0.6 mm under it.
    room = p.plate_top + 3.0
    walls = cut(bottom, p.plate_top + 1.3)
    pieces = list(getattr(walls, "geoms", [walls]))
    for x, y in p.keyholes:
        entry = y + p.keyhole_slide
        for along in (y, (y + entry) / 2, entry):
            assert not inside(bottom, x, along, p.plate_top + 0.1) and not inside(bottom, x, along, room - 0.1), (x, along)
            assert not inside(bottom, x + 3.8, along, p.plate_top + 1.0), (x, along)
            assert inside(bottom, x + 4.2, along, p.plate_top + 1.0), (x, along)
            # A roof over it: the case stays closed.
            assert inside(bottom, x, along, room + 0.1) and inside(bottom, x, along, room + 0.9), (x, along)
            assert not inside(bottom, x, along, room + 1.1), (x, along)
        # And walls all round: the pocket is a hole in one piece of the cut.
        assert any(
            Polygon(hole).contains(Point(x, y)) and Polygon(hole).contains(Point(x, entry))
            for piece in pieces
            for hole in piece.interiors
        ), (x, y)


def test_a_pockets_roof_stays_well_under_all_that_hangs_over_it(bottom, p):
    roof = p.plate_top + 4.0
    for x, y in p.keyholes:
        pocket = flat_box(x - 5.1, y - 5.1, x + 5.1, y + p.keyhole_slide + 5.1)
        over = [entry for entry in p.under if pocket.intersects(flat_box(entry.rect.x0, entry.rect.y0, entry.rect.x1, entry.rect.y1))]
        assert over, (x, y)
        assert all(p.standoff_top - entry.drop - roof >= 1.5 for entry in over), (x, y)


def test_each_foot_is_glued_flat_on_the_plate_inside_a_shallow_frame(bottom, p):
    # A recess would print as a bridge 20.8 mm long, and the foot would be glued to the underside of its strands.
    assert len(p.feet) == 4
    for foot in p.feet:
        for x, y in ((foot.x0 + 0.2, foot.y0 + 0.2), (foot.x1 - 0.2, foot.y1 - 0.2), foot.centre):
            assert inside(bottom, x, y, p.wall_bottom + 0.05), foot
        groove, beyond = foot.grown(p.foot_mark / 2), foot.grown(p.foot_mark + 0.2)
        cx, cy = foot.centre
        for x, y in ((groove.x0, cy), (groove.x1, cy), (cx, groove.y0), (cx, groove.y1), (groove.x0, groove.y0)):
            assert not inside(bottom, x, y, p.wall_bottom + p.foot_mark_depth - 0.05), foot
            assert inside(bottom, x, y, p.wall_bottom + p.foot_mark_depth + 0.05), foot
        assert inside(bottom, beyond.x0, cy, p.wall_bottom + 0.05) and inside(bottom, cx, beyond.y1, p.wall_bottom + 0.05), foot
    assert (p.foot_mark, p.foot_mark_depth) == (0.6, 0.2)


def test_feet_keyholes_and_screws_keep_out_of_each_others_way(p, board):
    feet = [flat_box(*astuple(foot.grown(p.foot_mark))) for foot in p.feet]
    screws = [Point(x, y).buffer(p.counterbore / 2) for x, y in board.holes]
    keyholes = [flat_box(x - 3.9, y - 3.9, x + 3.9, y + p.keyhole_slide + 3.9) for x, y in p.keyholes]
    openings = feet + screws + keyholes
    for i, one in enumerate(openings):
        for other in openings[i + 1:]:
            assert one.distance(other) >= 1.0
    width, length = p.case_size
    inside_the_plate = flat_box(p.rim + p.bottom_fit + 1.0, p.rim + p.bottom_fit + 1.0, width - p.rim - p.bottom_fit - 1.0, length - p.rim - p.bottom_fit - 1.0)
    assert all(inside_the_plate.contains(opening) for opening in feet + keyholes)


def test_with_its_keyholes_and_recesses_the_bottom_is_still_one_piece(bottom):
    assert len(bottom.solids()) == 1


def test_the_heights_that_carry_the_board_are_whole_layers(p):
    # Read in the g-code at 0.2 mm layers: the standoffs, 10.9 mm high, ended 0.1 mm short, and the posts, 10.75 mm
    # high, level with them.
    for top in (p.plate_top, p.standoff_top, p.standoff_top - POST_GAP):
        layers = (top - p.wall_bottom) / LAYER
        assert layers == pytest.approx(round(layers), abs=1e-6)
    assert POST_GAP >= LAYER
