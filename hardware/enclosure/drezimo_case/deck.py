"""The shell's top: the deck with its chamfer, the key wells, the grid and the bar.

Modelled in the case frame, with z up from the PCB's top surface. That frame is left-handed
(y runs towards the user), so the solid here is the mirror image of the real part. export.py
flips z, which undoes the mirror and lays the deck face-down for printing in one move.
"""

from build123d import Align, Axis, Box, Pos, RectangleRounded, chamfer, extrude

from .params import Rect

SKIRT_BOTTOM = -1.0
CURB = 1.6
OPENING_RADIUS = 0.2
LEAD_IN = 0.5


def build_deck(board, p):
    deck = _shell(p) + _key_block(p)
    for well in (p.fn_well, p.pad_well):
        deck -= box(well, p.well_floor, p.deck_top)
    return deck - [_opening(p, *board.key_centre(n)) for n in board.keys]


def box(rect, z0, z1):
    return Pos(*rect.centre, z0) * Box(rect.width, rect.height, z1 - z0, align=(Align.CENTER, Align.CENTER, Align.MIN))


def _shell(p):
    width, length = p.case_size
    centre = Pos(width / 2, length / 2, SKIRT_BOTTOM)
    underside = p.deck_top - p.deck_thickness

    block = extrude(centre * RectangleRounded(width, length, p.corner_radius), amount=p.deck_top - SKIRT_BOTTOM)
    block = chamfer(block.edges().group_by(Axis.Z)[-1], length=p.chamfer)
    cavity = extrude(
        centre * RectangleRounded(width - 2 * p.wall, length - 2 * p.wall, p.corner_radius - p.wall),
        amount=underside - SKIRT_BOTTOM,
    )
    return block - cavity


def _key_block(p):
    area = Rect(p.fn_well.x0 - CURB, p.fn_well.y0 - CURB, p.pad_well.x1 + CURB, p.pad_well.y1 + CURB)
    return box(area, p.rib_bottom, p.deck_top - p.deck_thickness)


def _opening(p, x, y):
    through = extrude(
        RectangleRounded(p.opening, p.opening, OPENING_RADIUS),
        amount=p.well_floor - p.rib_bottom + 2,
    )
    flared = p.opening + 2 * LEAD_IN
    lead_in = extrude(RectangleRounded(flared, flared, OPENING_RADIUS + LEAD_IN), amount=LEAD_IN, taper=45)
    return Pos(x, y, p.rib_bottom - 1) * through + Pos(x, y, p.rib_bottom) * lead_in
