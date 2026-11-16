from build123d import Vector

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
}


def nominal(**measured):
    m = {code: Measurement(value) for code, value in NOMINAL.items()}
    m.update({code: Measurement(value, measured=True) for code, value in measured.items()})
    return m


def inside(shape, x, y, z):
    return any(solid.is_inside(Vector(x, y, z)) for solid in shape.solids())
