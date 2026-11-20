"""Coupon (a): the back-left corner of the key well, to drop over four real switches.

It is the real deck clipped to two function keys, two pads, the bar between them and the first
two labels, plus four feet that seat it on the PCB at the height the shell will have.
"""

from build123d import Align, Box, Part, Pos

from .deck import LEAD_IN, SKIRT_BOTTOM, box, build_deck
from .inlay import build_labels
from .params import Rect

COUPON_A = Rect(0.0, 40.0, 46.1, 93.1)

RIM_FEET = (((4.9, 47.0), (3.0, 3.0)), ((4.9, 86.0), (3.0, 3.0)))
GRID_FEET = (((44.6, 71.55), (3.0, 3.0)), ((45.35, 83.575), (1.5, 4.0)))


def build_coupon_a(board, p):
    clip = box(COUPON_A, SKIRT_BOTTOM, p.deck_top)
    inlay = (Part() + build_labels(board, p)) & clip
    body = (build_deck(board, p) & clip) + _feet(p) - inlay
    return body, inlay


def _feet(p):
    under_the_rim = p.deck_top - p.deck_thickness
    into_the_grid = p.rib_bottom + LEAD_IN
    return [_foot(at, size, under_the_rim) for at, size in RIM_FEET] + [
        _foot(at, size, into_the_grid) for at, size in GRID_FEET
    ]


def _foot(at, size, top):
    return Pos(*at, 0) * Box(*size, top, align=(Align.CENTER, Align.CENTER, Align.MIN))
