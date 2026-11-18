"""The shell: the deck with its walls down to the bottom, and what the rest of the board needs of it.

Modelled like the deck, in the case frame with z up from the PCB's top surface.
"""

import math

from build123d import Align, Box, Circle, Cylinder, Part, Pos, Rectangle, extrude

from .board import Rect
from .deck import box, build_deck, key_block, outline
from .inlay import build_labels
from .logo import build_logo

# The chamfer leaves 1.06 mm of wall at the cavity's top corner: filled at 45°, it is 2 mm.
FILL = 1.4
SCREW_LEAD_IN = 0.3
RIB = 1.2
# At most, between two ribs: a finger on the deck between them moves it by hundredths of a millimetre.
RIB_PITCH = 10.0
# A rib left shallower than this over a part holds nothing: a wall goes round the part instead.
SHALLOW = 2.0
# How far a rib goes into what it ends in.
JOIN = 0.5
ON_ITS_BASE = (Align.CENTER, Align.CENTER, Align.MIN)


def build_shell(board, p):
    """The black body and the white inlays that lie flush in its deck: the logo and the six labels."""
    inlay = Part() + build_labels(board, p) + build_logo(p)
    return build_body(board, p) - inlay, inlay


def support_areas(p):
    """The two wells, about the middle of the deck: the only places where supports may stand."""
    x, y = p.case_size[0] / 2, p.case_size[1] / 2
    return tuple(Rect(well.x0 - x, well.y0 - y, well.x1 - x, well.y1 - y) for well in (p.fn_well, p.pad_well))


def build_body(board, p):
    """The black body, before the inlays are cut into it."""
    bosses = [_boss(p, x, y) for x, y in board.holes]
    body = build_deck(board, p, bottom=p.wall_bottom, extra=[_corner_fill(p), *bosses, _ribs(p)])
    body -= [_bottom_seat(p), _display_window(p), _encoder(board, p), _usb_window(p)]
    return body - [_screw_hole(p, x, y) for x, y in board.holes]


def _corner_fill(p):
    underside = p.deck_top - p.deck_thickness
    cavity = Pos(0, 0, underside - FILL) * outline(p, p.wall)
    return extrude(cavity, amount=FILL) - extrude(cavity, amount=FILL, taper=45)


def _ribs(p):
    """Ribs under the deck between the back wall and the keys, both ways, so that it does not give under a finger.

    They stop short of the board and are lowered over the low parts of it. A part that leaves them
    no depth gets a wall round it, which the ribs it cuts end in.
    """
    underside = p.deck_top - p.deck_thickness
    width, _ = p.case_size
    front = key_block(p).y0

    ribs = [Rect(x - RIB / 2, 0, x + RIB / 2, front + JOIN) for x in _spread(p.wall, width - p.wall)]
    ribs += [Rect(0, y - RIB / 2, width, y + RIB / 2) for y in _spread(p.wall, front)]
    walled = [room.rect.grown(RIB) for room in p.rooms if underside - room.top < SHALLOW]
    ribs = Part() + [box(rect, p.rib_bottom, underside + JOIN) for rect in ribs + walled]

    cavity = extrude(Pos(0, 0, p.rib_bottom) * outline(p, p.wall - JOIN), amount=underside + JOIN - p.rib_bottom)
    ribs &= cavity & box(Rect(0, 0, width, front + JOIN), p.rib_bottom, underside + JOIN)
    return ribs - [box(room.rect, 0, room.top) for room in p.rooms]


def _spread(start, end):
    """Evenly between two ends, no further apart than the ribs' pitch."""
    count = math.ceil((end - start) / RIB_PITCH)
    return [start + (end - start) * i / count for i in range(1, count)]


def _bottom_seat(p):
    """Below the seat only a rim of the wall is left, round the edge of the bottom."""
    return extrude(Pos(0, 0, p.wall_bottom) * outline(p, p.rim), amount=p.seat - p.wall_bottom)


def _display_window(p):
    """Straight for a short land over the glass, then opening at 45° up to the deck."""
    underside = p.deck_top - p.deck_thickness
    bevel = p.deck_thickness - p.window_land
    x, y = p.window.centre
    mouth = Pos(x, y, p.deck_top) * Rectangle(p.window.width + 2 * bevel, p.window.height + 2 * bevel)
    return (
        box(p.window, underside - 1, p.deck_top)
        + extrude(mouth, amount=bevel, dir=(0, 0, -1), taper=45)
        + extrude(mouth, amount=1)
    )


def _encoder(board, p):
    """A pocket over the encoder's body and a hole for its shaft, which the knob covers."""
    x, y = board.encoder_shaft
    pocket = box(p.encoder_pocket, 0, p.encoder_pocket_top)
    return pocket + Pos(x, y, 0) * Cylinder(p.shaft_hole / 2, p.deck_top + 1, align=ON_ITS_BASE)


def _usb_window(p):
    """Through the back wall for the plug, and the wall under it down to the rim's thickness.

    The connector stands a little into the wall's thickness, and has to pass it when the board drops in.
    """
    window = p.usb_window
    through = Pos(window.centre[0], -1, window.y0) * Box(window.width, p.wall + 2, window.height, align=(Align.CENTER, Align.MIN, Align.MIN))
    way_in = Pos(window.centre[0], p.rim, p.seat - 1) * Box(window.width, p.wall, window.y1 - p.seat + 1, align=(Align.CENTER, Align.MIN, Align.MIN))
    return through + way_in


def _boss(p, x, y):
    return Pos(x, y, 0) * Cylinder(p.boss_radius, p.deck_top - p.deck_thickness, align=ON_ITS_BASE)


def _screw_hole(p, x, y):
    radius = p.screw_hole / 2
    hole = Pos(x, y, -1) * Cylinder(radius, p.screw_hole_top + 1, align=ON_ITS_BASE)
    return hole + extrude(Pos(x, y, 0) * Circle(radius + SCREW_LEAD_IN), amount=SCREW_LEAD_IN, taper=45)
