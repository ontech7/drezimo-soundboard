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
    "M1w": Measurement(124.3),   # board width
    "M1h": Measurement(190.35),  # board length
    "M2a": Measurement(10.5),    # keycap skirt, at rest
    "M2b": Measurement(6.5),     # keycap skirt, fully pressed
    "M2c": Measurement(18.1),    # keycap width at its base
    "M3a": Measurement(15.6),    # switch's widest outline above the PCB
    "M4a": Measurement(5.5),     # highest corner of the OLED glass
    "M5a": Measurement(6.45),    # EC11 body top
    "M6a": Measurement(4.5),     # U2 (DIP) top
}


def estimates(m=MEASUREMENTS):
    return sorted(code for code, x in m.items() if not x.measured)
