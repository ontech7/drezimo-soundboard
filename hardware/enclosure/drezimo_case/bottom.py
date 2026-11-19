"""The bottom: the plate that closes the shell, with what stands on it.

Six standoffs take the screws and clamp the board against the shell's bosses. Ribs between the keys and a post under
every point between four of them keep plate and board from giving under a finger; under the USB module, where
neither can stand, the plate is thicker. Four keyhole sockets take the stand's pegs, and four shallow frames show
where the rubber feet are glued.

Modelled like the shell, in the case frame with z up from the PCB's top surface.
"""

import math

from build123d import Align, Axis, Circle, Cylinder, Part, Pos, Rectangle, chamfer, extrude

from .board import Rect
from .deck import box, outline

# The first layers print a little wide, and this edge has to enter the shell's rim.
EDGE_CHAMFER = 0.4
OFF_THE_WALLS = 0.3
# Over its last stretch under the board a column is as narrow as the board's hole allows.
NECK = 0.6
RIB = 1.2
# The height of the layers it is printed in. Its heights are given to a tenth of a millimetre: at 0.2 mm the
# standoffs ended a tenth short of the board, and the posts level with them.
LAYER = 0.1
# A rib stops this far under the board. A post goes nearly up to it, two layers short: the screws carry the board,
# the posts only keep it from giving under a key.
RIB_UNDER_BOARD, POST_GAP = 1.0, 2 * LAYER
POST_RADIUS = 2.0
# Kept clear of all that hangs under the board: sideways, or under how far it can hang.
CLEAR, UNDER = 0.5, 0.3
# The stand's peg: a 4 mm neck under a head 7 mm across and 2 mm high.
PEG_NECK, PEG_HEAD, PEG_HEAD_HEIGHT = 4.0, 7.0, 2.0
KEYHOLE_PLAY = 0.4
# The roof is a bridge of fine strands that may hang a little: the head keeps 0.6 mm under it on the stand's neck,
# which is 0.4 mm longer than the plate is thick.
POCKET_HEADROOM, POCKET_ROOF, POCKET_WALL = 1.0, 1.0, 1.2
# Left as plain plate behind the USB connector's mouth, where the plug's grip comes in through the back wall.
BEHIND_THE_PLUG = 3.0
ON_ITS_BASE = (Align.CENTER, Align.CENTER, Align.MIN)


def build_bottom(board, p):
    p.check_bottom()
    plate = extrude(Pos(0, 0, p.wall_bottom) * outline(p, p.rim + p.bottom_fit), amount=p.plate_top - p.wall_bottom)
    plate = chamfer(plate.edges().group_by(Axis.Z)[0], length=EDGE_CHAMFER)
    columns = [_column(p, x, y) for x, y in board.holes]
    pockets = [_pocket(p, x, y) for x, y in p.keyholes]
    body = plate + _inside_the_walls(p, [*columns, _ribs_and_posts(p), _under_the_module(board, p), *pockets])
    body -= [_screw_way(p, x, y) for x, y in board.holes]
    body -= [_keyhole(p, x, y) for x, y in p.keyholes]
    return body - [_foot_mark(p, foot) for foot in p.feet]


def _inside_the_walls(p, solids):
    """What stands on the plate, kept off the shell's walls so that the bottom goes in."""
    cavity = extrude(Pos(0, 0, p.plate_top) * outline(p, p.wall + OFF_THE_WALLS), amount=p.standoff_top - p.plate_top)
    return [solid & cavity for solid in solids]


def _ribs_and_posts(p):
    """A rib along every line between the keys, both ways, and a post where they cross between four keys.

    The plate does not give between ribs a key apart, and the board does not give between the posts.
    """
    width, length = p.case_size
    ribs = [Rect(x - RIB / 2, 0, x + RIB / 2, length) for x in p.rib_lines_x]
    ribs += [Rect(0, y - RIB / 2, width, y + RIB / 2) for y in p.rib_lines_y]
    posts = [Rect(x - POST_RADIUS, y - POST_RADIUS, x + POST_RADIUS, y + POST_RADIUS) for x, y in p.nodes]

    grid = Part() + [box(rib, p.plate_top, p.standoff_top - RIB_UNDER_BOARD) for rib in ribs]
    grid += [
        Pos(x, y, p.plate_top) * Cylinder(POST_RADIUS, p.standoff_top - POST_GAP - p.plate_top, align=ON_ITS_BASE)
        for x, y in p.nodes
    ]
    rooms = [(entry.rect.grown(CLEAR), p.standoff_top - entry.drop - UNDER) for entry in p.under]
    return grid - [
        box(room, lowest, p.standoff_top) for room, lowest in rooms if any(room.overlaps(part) for part in ribs + posts)
    ]


def _under_the_module(board, p):
    """Where no rib can stand, under the USB module, the plate is thicker instead, up to a whole layer."""
    module = board.usb_module.grown(CLEAR)
    layers = math.floor((p.standoff_top - p.under_the_module - p.wall_bottom) / LAYER + 1e-6)
    thicker = Rect(module.x0, board.usb_module.y0 + BEHIND_THE_PLUG, module.x1, module.y1)
    return box(thicker, p.plate_top, p.wall_bottom + layers * LAYER)


def _foot_mark(p, foot):
    """The groove round a foot's place, in the plate's outside face."""
    depth = p.wall_bottom - 1, p.wall_bottom + p.foot_mark_depth
    return box(foot.grown(p.foot_mark), *depth) - box(foot, *depth)


def _slot(x, y, slide, width):
    """In the top view, the way of something `width` across from the locked point (x, y) to where it goes in."""
    return Pos(x, y + slide / 2) * Rectangle(width, slide) + [Pos(x, end) * Circle(width / 2) for end in (y, y + slide)]


def _pocket(p, x, y):
    """A closed box on the plate, behind a keyhole: the peg's head moves in it, and the case stays shut."""
    height = PEG_HEAD_HEIGHT + POCKET_HEADROOM + POCKET_ROOF
    around = _slot(x, y, p.keyhole_slide, PEG_HEAD + 2 * (KEYHOLE_PLAY + POCKET_WALL))
    return extrude(Pos(0, 0, p.plate_top) * around, amount=height)


def _keyhole(p, x, y):
    """Through the plate, a hole for the peg's head and a slot for its neck; behind it, the room its head slides in."""
    head, neck = PEG_HEAD + 2 * KEYHOLE_PLAY, PEG_NECK + 2 * KEYHOLE_PLAY
    way_in = Pos(x, y + p.keyhole_slide) * Circle(head / 2) + _slot(x, y, p.keyhole_slide, neck)
    through = extrude(Pos(0, 0, p.wall_bottom - 1) * way_in, amount=p.plate_top - p.wall_bottom + 1)
    room = extrude(Pos(0, 0, p.plate_top) * _slot(x, y, p.keyhole_slide, head), amount=PEG_HEAD_HEIGHT + POCKET_HEADROOM)
    return through + room


def _column(p, x, y):
    """Wide round the screw's head, then narrowing at 45° to what the board's hole leaves room for."""
    taper = p.column_radius - p.standoff_radius
    shoulder = p.standoff_top - NECK - taper
    return (
        Pos(x, y, p.plate_top) * Cylinder(p.column_radius, shoulder - p.plate_top, align=ON_ITS_BASE)
        + extrude(Pos(x, y, shoulder) * Circle(p.column_radius), amount=taper, taper=45)
        + Pos(x, y, shoulder + taper) * Cylinder(p.standoff_radius, NECK, align=ON_ITS_BASE)
    )


def _screw_way(p, x, y):
    """From outside: a counterbore for the head, a 45° seat, and the hole for the thread up to the board."""
    seat = p.seat_narrow - p.seat_wide
    return (
        Pos(x, y, p.wall_bottom - 1) * Cylinder(p.counterbore / 2, p.seat_wide - p.wall_bottom + 1, align=ON_ITS_BASE)
        + extrude(Pos(x, y, p.seat_wide) * Circle(p.counterbore / 2), amount=seat, taper=45)
        + Pos(x, y, p.seat_wide) * Cylinder(p.screw_clearance / 2, p.standoff_top - p.seat_wide + 1, align=ON_ITS_BASE)
    )
