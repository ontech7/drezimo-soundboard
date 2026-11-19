"""The bottom: the plate that closes the shell, and the standoffs that carry the board over it.

Modelled like the shell, in the case frame with z up from the PCB's top surface.
"""

from build123d import Align, Axis, Circle, Cylinder, Pos, chamfer, extrude

from .deck import outline

# The first layers print a little wide, and this edge has to enter the shell's rim.
EDGE_CHAMFER = 0.4
OFF_THE_WALLS = 0.3
# Over its last stretch under the board a column is as narrow as the board's hole allows.
NECK = 0.6
ON_ITS_BASE = (Align.CENTER, Align.CENTER, Align.MIN)


def build_bottom(board, p):
    p.check_bottom()
    plate = extrude(Pos(0, 0, p.wall_bottom) * outline(p, p.rim + p.bottom_fit), amount=p.plate_top - p.wall_bottom)
    plate = chamfer(plate.edges().group_by(Axis.Z)[0], length=EDGE_CHAMFER)
    columns = [_column(p, x, y) for x, y in board.holes]
    return plate + _inside_the_walls(p, columns) - [_screw_way(p, x, y) for x, y in board.holes]


def _inside_the_walls(p, solids):
    """What stands on the plate, kept off the shell's walls so that the bottom goes in."""
    cavity = extrude(Pos(0, 0, p.plate_top) * outline(p, p.wall + OFF_THE_WALLS), amount=p.standoff_top - p.plate_top)
    return [solid & cavity for solid in solids]


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
