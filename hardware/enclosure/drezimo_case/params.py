"""Every dimension of the enclosure that is derived from the board and the measurements.

Heights are in mm above the PCB's top surface; the rules are those of the enclosure spec
("Deck height against the keycaps").
"""

import math
from dataclasses import dataclass

from .board import INSET, PITCH
from .measurements import MEASUREMENTS, PLAUSIBLE

LAYER = 0.2
DECK_THICKNESS = 2.0
RIB_BOTTOM = 0.5
WELL_MARGIN = 0.8
CHAMFER = 3.0
CORNER_RADIUS = 4.0
WALL = 2.5
INLAY_DEPTH = 1.0

PLATE_LEVEL = 5.0
SKIRT_CLEARANCE = 1.0
OPENING_CLEARANCE = 0.2
ENCODER_GAP, ENCODER_SKIN = 0.25, 1.1
GLASS_GAP = 0.3
CHIP_GAP = 0.5

MIN_RIB_HEIGHT = 1.0
MIN_RIB_WIDTH = 1.2
MAX_OUTLINE_ERROR = 1.0

FIRST_FN_KEY, LAST_FN_KEY = 37, 42
FIRST_PAD, LAST_PAD = 1, 36


class EstimateError(RuntimeError):
    pass


@dataclass(frozen=True)
class Rect:
    x0: float
    y0: float
    x1: float
    y1: float

    @property
    def width(self):
        return self.x1 - self.x0

    @property
    def height(self):
        return self.y1 - self.y0

    @property
    def centre(self):
        return (self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2


@dataclass(frozen=True)
class Params:
    layer: float
    deck_top: float
    deck_thickness: float
    well_floor: float
    rib_bottom: float
    opening: float
    rib_width: float
    well_margin: float
    chamfer: float
    corner_radius: float
    wall: float
    inlay_depth: float
    fn_well: Rect
    pad_well: Rect
    bar: Rect
    case_size: tuple[float, float]
    codes: tuple[str, ...]
    estimated: tuple[str, ...]

    @classmethod
    def derive(cls, board, m=MEASUREMENTS):
        value = {code: x.value for code, x in m.items()}
        _check_plausible(value, board)

        deck_top = _round_up_to_layer(max(
            value["M5a"] + ENCODER_GAP + ENCODER_SKIN,
            value["M4a"] + GLASS_GAP + DECK_THICKNESS,
            value["M6a"] + CHIP_GAP + DECK_THICKNESS,
        ))

        well_floor = min(PLATE_LEVEL, value["M2b"] - SKIRT_CLEARANCE)
        rib_height = well_floor - RIB_BOTTOM
        if rib_height < MIN_RIB_HEIGHT:
            raise ValueError(
                f"the grid's ribs would be {rib_height:.2f} mm tall (minimum {MIN_RIB_HEIGHT}): "
                f"the pressed keycap skirt (M2b = {value['M2b']}) is too low for a floor"
            )

        opening = value["M3a"] + 2 * OPENING_CLEARANCE
        rib_width = PITCH - opening
        if rib_width < MIN_RIB_WIDTH:
            raise ValueError(
                f"the grid's ribs would be {rib_width:.2f} mm wide (minimum {MIN_RIB_WIDTH}): "
                f"the switch outline (M3a = {value['M3a']}) leaves no room between the keys"
            )

        reach = value["M2c"] / 2 + WELL_MARGIN
        fn_well = _well(board, FIRST_FN_KEY, LAST_FN_KEY, reach)
        pad_well = _well(board, FIRST_PAD, LAST_PAD, reach)

        return cls(
            layer=LAYER,
            deck_top=deck_top,
            deck_thickness=DECK_THICKNESS,
            well_floor=well_floor,
            rib_bottom=RIB_BOTTOM,
            opening=opening,
            rib_width=rib_width,
            well_margin=WELL_MARGIN,
            chamfer=CHAMFER,
            corner_radius=CORNER_RADIUS,
            wall=WALL,
            inlay_depth=INLAY_DEPTH,
            fn_well=fn_well,
            pad_well=pad_well,
            bar=Rect(fn_well.x0, fn_well.y1, fn_well.x1, pad_well.y0),
            case_size=(value["M1w"] + 2 * INSET, value["M1h"] + 2 * INSET),
            codes=tuple(sorted(m)),
            estimated=tuple(sorted(code for code, x in m.items() if not x.measured)),
        )

    def require_measured(self, *codes):
        """Raises if any of these codes, or any code at all when none is given, is still an estimate."""
        unknown = [code for code in codes if code not in self.codes]
        if unknown:
            raise KeyError(f"unknown measurement codes: {', '.join(unknown)}")
        missing = [code for code in codes or self.codes if code in self.estimated]
        if missing:
            raise EstimateError(f"still estimates, measure them first: {', '.join(missing)}")


def _check_plausible(value, board):
    for code, (low, high) in PLAUSIBLE.items():
        if not low <= value[code] <= high:
            raise ValueError(f"{code} = {value[code]} mm is not plausible (expected {low} to {high}): check the reading")
    if value["M2b"] >= value["M2a"]:
        raise ValueError(f"M2a = {value['M2a']} (at rest) must be above M2b = {value['M2b']} (pressed): check the readings")
    for code, drawn in (("M1w", board.outline[0]), ("M1h", board.outline[1])):
        error = value[code] - drawn
        if abs(error) > MAX_OUTLINE_ERROR:
            raise ValueError(
                f"{code} = {value[code]} mm: the measured outline is {error:+.2f} mm off the drawing ({drawn}), "
                f"more than the {MAX_OUTLINE_ERROR} mm the case can follow"
            )


def _round_up_to_layer(height):
    return round(math.ceil(round(height / LAYER, 6)) * LAYER, 4)


def _well(board, first_key, last_key, reach):
    x0, y0 = board.key_centre(first_key)
    x1, y1 = board.key_centre(last_key)
    return Rect(x0 - reach, y0 - reach, x1 + reach, y1 + reach)
