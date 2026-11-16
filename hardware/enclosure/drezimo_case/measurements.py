"""Caliper measurements of the assembled board, by the M-codes of the enclosure spec.

Heights are in mm above the PCB's top surface. A value is an estimate until the owner has
measured it; final parts refuse to build on estimates.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class Measurement:
    value: float
    measured: bool = False


MEASUREMENTS = {
    "M1w": Measurement(124.25, measured=True),  # board width
    # Too long for the caliper: the sum of three readings along the second column of keys,
    # 64.30 (back edge to the function switch) + 8.47 (gap) + 117.60 (first pad to the front edge).
    "M1h": Measurement(190.37, measured=True),  # board length
    "M2a": Measurement(10.5),    # keycap skirt, at rest
    "M2b": Measurement(6.5),     # keycap skirt, fully pressed
    "M2c": Measurement(18.1),    # keycap width at its base
    "M3a": Measurement(15.6),    # switch's widest outline above the PCB
    "M4a": Measurement(5.5),     # highest corner of the OLED glass
    "M5a": Measurement(6.45),    # EC11 body top
    "M6a": Measurement(4.5),     # U2 (DIP) top
}

# Outside these a value is a slip of the hand, not a measurement. The outline (M1w, M1h) is
# checked against the drawing instead.
PLAUSIBLE = {
    "M2a": (8.0, 14.0),
    "M2b": (2.0, 10.0),
    "M2c": (17.0, 19.0),
    "M3a": (14.0, 18.0),
    "M4a": (3.0, 8.0),
    "M5a": (5.0, 8.0),
    "M6a": (2.0, 8.0),
}


def estimates(m=MEASUREMENTS):
    return sorted(code for code, x in m.items() if not x.measured)
