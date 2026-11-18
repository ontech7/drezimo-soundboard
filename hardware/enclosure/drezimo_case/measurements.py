"""Caliper measurements of the assembled board, by the M-codes of the enclosure spec.

Heights are in mm above the PCB's top surface. A value is an estimate until the owner has
measured it or it is known from elsewhere; final parts refuse to build on estimates.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class Measurement:
    value: float
    measured: bool = False
    # Where the value comes from when no caliper reading stands behind it. Such a value is no estimate.
    known: str = ""


COUPON_FITS = "the coupon printed on it went over the board's keys, and their keycaps to the bottom of its wells"
# The inlay gauge as drawn, by the codes of its two readings.
GAUGE_DRAWN = {"P1s": 70.0, "P1l": 101.0}

MEASUREMENTS = {
    "M1w": Measurement(124.25, measured=True),  # board width
    # Too long for the caliper: the sum of three readings along the second column of keys,
    # 64.30 (back edge to the function switch) + 8.47 (gap) + 117.60 (first pad to the front edge).
    "M1h": Measurement(190.37, measured=True),  # board length
    "M2a": Measurement(10.5, known="shapes nothing: only its order against M2b is checked"),  # keycap skirt, at rest
    "M2b": Measurement(6.5),     # keycap skirt, fully pressed
    "M2c": Measurement(18.1, known=COUPON_FITS),  # keycap width at its base
    "M3a": Measurement(15.6, known=COUPON_FITS),  # switch's widest outline above the PCB
    "M4a": Measurement(5.5),     # highest corner of the OLED glass
    "M5a": Measurement(6.45),    # EC11 body top
    "M6a": Measurement(4.5),     # U2 (DIP) top
    "M1t": Measurement(1.6, known="as ordered from the fab, whose 10 % either way is inside the clearances"),  # board thickness
    "M4b": Measurement(5.0),     # highest solder point of the display's header
    "M4w": Measurement(29.7),    # lit area, width
    "M4h": Measurement(14.9),    # lit area, height
    "M4x": Measurement(74.7),    # lit area's centre from the board's left edge
    "M4y": Measurement(21.4),    # lit area's centre from the board's back edge
    "M4t": Measurement(1.9),     # glass beyond the lit area, at the back
    "M4s": Measurement(2.4),     # glass beyond the lit area, at each side
    "M4f": Measurement(6.0),     # glass beyond the lit area, at the front
    # The display module's own board, measured loose before it was soldered (docs/project.md, section 5).
    "M4p": Measurement(35.54, measured=True),  # its width
    "M4l": Measurement(33.59, measured=True),  # its length, from its header's edge
    "M4e": Measurement(1.69, measured=True),   # from the middle of its pin row to its edge behind
    "M5b": Measurement(8.5),     # knob's lower edge with the knob pressed to its click
    "M6b": Measurement(2.0),     # tallest pin end of the USB module, on top
    "M7a": Measurement(6.6),     # under the board: how far the USB connector stands off it
    "M7b": Measurement(6.6),     # under the board: how far the tallest thing on that side stands off it
    "M8w": Measurement(12.0),    # USB plug's grip, width
    "M8h": Measurement(6.5),     # USB plug's grip, thickness
    # The printed inlay gauge between the jaws, across and along: the printer's own ruler.
    "P1s": Measurement(GAUGE_DRAWN["P1s"]),
    "P1l": Measurement(GAUGE_DRAWN["P1l"]),
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
    "M1t": (1.4, 1.8),
    "M4b": (2.0, 8.0),
    "M4w": (28.0, 31.0),
    "M4h": (13.5, 16.0),
    "M4x": (71.7, 77.7),
    "M4y": (18.4, 24.4),
    "M4t": (0.5, 4.0),
    "M4s": (0.5, 4.0),
    "M4f": (1.0, 10.0),
    "M4p": (33.0, 38.0),
    "M4l": (30.0, 37.0),
    "M4e": (1.0, 4.0),
    "M5b": (6.0, 14.0),
    "M6b": (0.0, 8.0),
    "M7a": (5.0, 9.0),
    "M7b": (1.5, 12.0),
    "M8w": (8.0, 16.0),
    "M8h": (4.0, 9.0),
    "P1s": (69.3, 70.7),
    "P1l": (100.0, 102.0),
}


def estimates(m=MEASUREMENTS):
    return sorted(code for code, x in m.items() if not (x.measured or x.known))
