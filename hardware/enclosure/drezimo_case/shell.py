"""The shell: the deck with its walls down to the bottom, and what the rest of the board needs of it.

Modelled like the deck, in the case frame with z up from the PCB's top surface.
"""

from build123d import Align, Circle, Cylinder, Pos, extrude

from .deck import build_deck, outline

# The chamfer leaves 1.06 mm of wall at the cavity's top corner: filled at 45°, it is 2 mm.
FILL = 1.4
SCREW_LEAD_IN = 0.3
ON_ITS_BASE = (Align.CENTER, Align.CENTER, Align.MIN)


def build_body(board, p):
    """The black body, before the inlays are cut into it."""
    bosses = [_boss(p, x, y) for x, y in board.holes]
    body = build_deck(board, p, bottom=p.wall_bottom, extra=[_corner_fill(p), *bosses])
    body -= _bottom_seat(p)
    return body - [_screw_hole(p, x, y) for x, y in board.holes]


def _corner_fill(p):
    underside = p.deck_top - p.deck_thickness
    cavity = Pos(0, 0, underside - FILL) * outline(p, p.wall)
    return extrude(cavity, amount=FILL) - extrude(cavity, amount=FILL, taper=45)


def _bottom_seat(p):
    """Below the seat only a rim of the wall is left, round the edge of the bottom."""
    return extrude(Pos(0, 0, p.wall_bottom) * outline(p, p.rim), amount=p.seat - p.wall_bottom)


def _boss(p, x, y):
    return Pos(x, y, 0) * Cylinder(p.boss_radius, p.deck_top - p.deck_thickness, align=ON_ITS_BASE)


def _screw_hole(p, x, y):
    radius = p.screw_hole / 2
    hole = Pos(x, y, -1) * Cylinder(radius, p.screw_hole_top + 1, align=ON_ITS_BASE)
    return hole + extrude(Pos(x, y, 0) * Circle(radius + SCREW_LEAD_IN), amount=SCREW_LEAD_IN, taper=45)
