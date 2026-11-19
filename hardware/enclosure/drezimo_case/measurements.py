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
WINDOW_ON_THE_GLASS = "shapes nothing: it only keeps the window on the glass, which the glass's own size does by over a millimetre"
# The inlay gauge as drawn, by the codes of its two readings.
GAUGE_DRAWN = {"P1s": 70.0, "P1l": 101.0}

MEASUREMENTS = {
    "M1w": Measurement(124.25, measured=True),  # board width
    # Too long for the caliper: the sum of three readings along the second column of keys,
    # 64.30 (back edge to the function switch) + 8.47 (gap) + 117.60 (first pad to the front edge).
    "M1h": Measurement(190.37, measured=True),  # board length
    "M2a": Measurement(10.5, known="shapes nothing: only its order against M2b is checked"),  # keycap skirt, at rest
    # From here on, where nothing else is said: read on the assembled board on 2026-11-18.
    "M2b": Measurement(6.25, measured=True),  # keycap skirt, fully pressed: its top at 16.12 less its height, 9.87
    "M2c": Measurement(18.1, known=COUPON_FITS),  # keycap width at its base
    "M3a": Measurement(15.6, known=COUPON_FITS),  # switch's widest outline above the PCB
    "M4a": Measurement(5.83, measured=True),  # highest corner of the OLED glass, of 5.70, 5.81, 5.83 and 5.56
    # Read 5.24 on the board against 6.45 loose (docs/project.md, section 5). A body cannot sit lower
    # than it is tall and the glass sets the deck either way, so the higher one stands.
    "M5a": Measurement(6.45, measured=True),  # EC11 body top
    "M6a": Measurement(4.33, measured=True),  # U2 (DIP) top
    "M1t": Measurement(1.6, known="as ordered from the fab, whose 10 % either way is inside the clearances"),  # board thickness
    "M4b": Measurement(5.11, measured=True),  # highest solder point of the display's header
    # The lit area, like its glass of 34.44 x 22.97 mm, was measured loose (docs/project.md, section 4).
    "M4w": Measurement(29.71, measured=True),  # lit area, width
    "M4h": Measurement(14.90, measured=True),  # lit area, height
    # Where the light starts from the left and back edges of the display's own board, which hangs from its
    # header 56.93 and 5.81 mm inside the main board's edges: 2.50 and 7.87 mm by caliper and eye, 3.06 and
    # 7.38 off a photo of the lit display (2026-11-19). Neither is trusted alone: the mean of the two stands.
    "M4x": Measurement(74.565, measured=True),  # lit area's centre from the board's left edge: 56.93 + 2.78 + 29.71 / 2
    "M4y": Measurement(20.885, measured=True),  # lit area's centre from the board's back edge: 5.81 + 7.625 + 14.90 / 2
    "M4t": Measurement(1.9, known=WINDOW_ON_THE_GLASS),    # glass beyond the lit area, at the back: as the panel's makers draw it
    "M4s": Measurement(2.365, known=WINDOW_ON_THE_GLASS),  # glass beyond the lit area, at each side: half of 34.44 - 29.71
    "M4f": Measurement(6.17, known=WINDOW_ON_THE_GLASS),   # glass beyond the lit area, at the front: 22.97 - 14.90 - 1.9
    # The display module's own board, measured loose before it was soldered (docs/project.md, section 5).
    "M4p": Measurement(35.54, measured=True),  # its width
    "M4l": Measurement(33.59, measured=True),  # its length, from its header's edge
    "M4e": Measurement(1.69, measured=True),   # from the middle of its pin row to its edge behind
    "M5b": Measurement(11.2, measured=True),   # knob's lower edge with the knob pressed to its click: its top at 28.20 less its height, 17.0
    "M6b": Measurement(1.92, measured=True),   # tallest pin end of the USB module, on top
    "M7a": Measurement(6.57, measured=True),   # under the board: how far the USB connector stands off it
    "M7b": Measurement(6.57, measured=True),   # under the board: how far the tallest thing on that side stands off it, the connector itself
    "M8w": Measurement(12.12, measured=True),  # USB plug's grip, width
    "M8h": Measurement(6.60, measured=True),   # USB plug's grip, thickness
    # The printed inlay gauge between the jaws, across and along: the printer's own ruler.
    # Across it reads longer than drawn and along it shorter, so no correction comes of it.
    "P1s": Measurement(70.26, measured=True),
    "P1l": Measurement(100.70, measured=True),
    # For the bottom. The screws are M3, countersunk: the owner's 10 mm ones, since a 20 mm one would end in the
    # deck. Its length is the whole screw's, head included, if it was read like the 20 mm one's (20.6): asked.
    "M9l": Measurement(10.4),   # screw, from the flat face of its head to its tip
    "M9d": Measurement(5.85),   # its head, across: read on the 20 mm screw
    "M9k": Measurement(1.96),   # its head, height: read on the 20 mm screw
    "M10w": Measurement(20.0, measured=True),  # a rubber foot's side, as the owner gave it
    "M10h": Measurement(2.5, measured=True),   # and its height
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
    "M9l": (6.0, 25.0),
    "M9d": (4.5, 7.5),
    "M9k": (1.0, 3.0),
    "M10w": (8.0, 30.0),
    "M10h": (1.5, 6.0),
}


def estimates(m=MEASUREMENTS):
    return sorted(code for code, x in m.items() if not (x.measured or x.known))
