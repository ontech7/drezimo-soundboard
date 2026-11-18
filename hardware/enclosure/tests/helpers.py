import math

from build123d import Vector
from shapely.geometry import Polygon

from drezimo_case.measurements import Measurement

NOMINAL = {
    "M1w": 124.3,
    "M1h": 190.35,
    "M2a": 10.5,
    "M2b": 6.5,
    "M2c": 18.1,
    "M3a": 15.6,
    "M4a": 5.5,
    "M5a": 6.45,
    "M6a": 4.5,
    "M1t": 1.6,
    "M4b": 5.0,
    "M4w": 29.7,
    "M4h": 14.9,
    "M4x": 74.7,
    "M4y": 21.4,
    "M4t": 1.9,
    "M4s": 2.4,
    "M4f": 6.0,
    "M4p": 35.54,
    "M4l": 33.59,
    "M4e": 1.69,
    "M5b": 9.0,
    "M6b": 2.0,
    "M7a": 6.6,
    "M8w": 12.0,
    "M8h": 6.5,
}


def nominal(**measured):
    m = {code: Measurement(value) for code, value in NOMINAL.items()}
    m.update({code: Measurement(value, measured=True) for code, value in measured.items()})
    return m


def inside(shape, x, y, z):
    return any(solid.is_inside(Vector(x, y, z)) for solid in shape.solids())


def flat(face):
    """A drawn face as a flat polygon with its counters."""
    shape = _ring(face.outer_wire())
    for counter in face.inner_wires():
        shape = shape.difference(_ring(counter))
    return shape


def pieces_and_counters(shape):
    parts = [part for part in getattr(shape, "geoms", [shape]) if not part.is_empty]
    return len(parts), sum(len(part.interiors) for part in parts)


def _ring(wire):
    sides = [(tuple(edge @ 0)[:2], tuple(edge @ 1)[:2]) for edge in wire.order_edges()]
    # The corner a side shares with the next one, whichever way each side runs.
    corners = [
        min(side, key=lambda end: min(math.dist(end, other) for other in following))
        for side, following in zip(sides, sides[1:] + sides[:1])
    ]
    return Polygon(corners)
