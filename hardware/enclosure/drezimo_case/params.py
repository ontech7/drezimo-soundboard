"""Every dimension of the enclosure that is derived from the board and the measurements.

Heights are in mm above the PCB's top surface; the rules are those of the enclosure spec
("Deck height against the keycaps").
"""

import math
from dataclasses import dataclass

from .board import CHIP, DISPLAY_HEADER, ENCODER, INSET, PITCH, USB_MODULE, Rect
from .measurements import GAUGE_DRAWN, MEASUREMENTS, PLAUSIBLE, estimates

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
# Round each switch: the most a keycap hides, for a shell that prints a little small over a field of 42.
OPENING_CLEARANCE = 0.25
ENCODER_GAP, ENCODER_SKIN = 0.25, 1.1
GLASS_GAP = 0.3
CHIP_GAP = 0.5
# What the deck must clear, each with the room it leaves over it and the deck it keeps there.
DECK_OVER = (
    ("M5a", ENCODER_GAP + ENCODER_SKIN),
    ("M4a", GLASS_GAP + DECK_THICKNESS),
    ("M4b", GLASS_GAP + DECK_THICKNESS),
    ("M6a", CHIP_GAP + DECK_THICKNESS),
    ("M6b", CHIP_GAP + DECK_THICKNESS),
)
# What must stay between the deck and the knob pressed to its click. A knob read lower is to be set higher on its shaft.
KNOB_GAP = 0.5
# The encoder's body, a square about its shaft, measured loose (docs/project.md, section 5).
# The footprint draws it 11.6 mm from back to front.
ENCODER_BODY = 12.0
ENCODER_CLEARANCE = 0.75
SHAFT_HOLE = 8.0

BOTTOM_THICKNESS = 2.0
RIM = 1.2
MODULE_CLEARANCE = 0.9
RECEPTACLE_HEIGHT = 3.03
PLUG_CLEARANCE = 0.5
# The board's holes keep copper and parts out of a 6.95 mm circle.
BOSS_RADIUS = 3.25
SCREW_HOLE = 2.7
SCREW_UNDER_FLOOR = 0.4

# The pixels lie this far under the top of the glass.
PIXEL_DEPTH = 0.7
WINDOW_SIDE, WINDOW_BACK, WINDOW_LAND = 1.0, 0.75, 0.3
SIGHT_SPARE = 0.2
GLASS_OVERLAP = 0.3

# The most a part is drawn bigger for the printer's shrink: PLA gives up to half a percent, and a gauge read wrong
# must not cost more than the 0.15 mm this is at the far keys.
MAX_PRINT_SCALE = 1.0025

# Kept clear round a part's body and legs, and round the copper of a single pin.
ROOM, PIN_ROOM = 1.5, 0.5
# The glass may read this far over the edge of its module's board: two readings disagreeing, not a slip.
GLASS_OVERHANG = 1.0

MIN_RIB_HEIGHT = 1.0
MIN_RIB_WIDTH = 1.2
MAX_OUTLINE_ERROR = 1.0

FIRST_FN_KEY, LAST_FN_KEY = 37, 42
FIRST_PAD, LAST_PAD = 1, 36


class EstimateError(RuntimeError):
    pass


@dataclass(frozen=True)
class Room:
    """What a part of the board needs of the shell: over `rect`, nothing under the deck comes lower than `top`."""

    rect: Rect
    top: float


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
    deck_set_by: str
    board_thickness: float
    under_board: float
    seat: float
    wall_bottom: float
    rim: float
    boss_radius: float
    screw_hole: float
    screw_hole_top: float
    lit: Rect
    glass: Rect
    window: Rect
    window_land: float
    encoder_pocket: Rect
    encoder_pocket_top: float
    shaft_hole: float
    # In the back wall's plane: x0 and x1 across, y0 and y1 the heights of its lower and upper edge.
    usb_window: Rect
    display_module: Rect
    # The chip's, the display's and the encoder's, then one for every other pin that comes up through the board.
    rooms: tuple[Room, ...]
    # How much higher on its shaft than it was read the knob must sit, to stay clear of the deck when pressed.
    knob_lift: float
    # How much bigger than drawn a part that goes on the board is printed, for what the printer shrinks.
    print_scale: float
    codes: tuple[str, ...]
    estimated: tuple[str, ...]

    @classmethod
    def derive(cls, board, m=MEASUREMENTS):
        value = {code: x.value for code, x in m.items()}
        _check_plausible(value, board)

        deck_set_by, over = max(DECK_OVER, key=lambda part: round(value[part[0]] + part[1], 6))
        deck_top = _round_up(value[deck_set_by] + over, LAYER)

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

        # The plug's grip passes the back wall round the connector, and the bottom closes the wall just under it.
        connector = value["M1t"] + value["M7a"] - RECEPTACLE_HEIGHT / 2
        plug = value["M8h"] / 2 + PLUG_CLEARANCE
        tallest = max(value["M7a"], value["M7b"])
        under_board = _round_up(max(tallest + MODULE_CLEARANCE, connector + plug - value["M1t"]), 0.1)
        seat = -(value["M1t"] + under_board)
        usb_x = board.usb_module.centre[0]
        half_width = value["M8w"] / 2 + PLUG_CLEARANCE
        usb_window = Rect(usb_x - half_width, -connector - plug, usb_x + half_width, -connector + plug)

        lit = _centred(value["M4x"] + INSET, value["M4y"] + INSET, value["M4w"], value["M4h"])
        glass = Rect(lit.x0 - value["M4s"], lit.y0 - value["M4t"], lit.x1 + value["M4s"], lit.y1 + value["M4f"])
        # Seen from the front at 45°, the lip must not hide the first lit row.
        lip = deck_top - DECK_THICKNESS + WINDOW_LAND
        front = lip - (value["M4a"] - PIXEL_DEPTH) + SIGHT_SPARE
        window = Rect(lit.x0 - WINDOW_SIDE, lit.y0 - WINDOW_BACK, lit.x1 + WINDOW_SIDE, lit.y1 + front)
        covered = min(window.x0 - glass.x0, window.y0 - glass.y0, glass.x1 - window.x1, glass.y1 - window.y1)
        if covered < GLASS_OVERLAP - 1e-9:
            raise ValueError(
                f"the window would come within {covered:.2f} mm of the edge of the glass "
                f"(minimum {GLASS_OVERLAP}): check M4t, M4s and M4f"
            )

        module = _display_module(board, value, glass)
        underside = deck_top - DECK_THICKNESS
        pocket = ENCODER_BODY + 2 * ENCODER_CLEARANCE

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
            deck_set_by=deck_set_by,
            board_thickness=value["M1t"],
            under_board=under_board,
            seat=seat,
            wall_bottom=seat - BOTTOM_THICKNESS,
            rim=RIM,
            boss_radius=BOSS_RADIUS,
            screw_hole=SCREW_HOLE,
            screw_hole_top=round(well_floor - SCREW_UNDER_FLOOR, 4),
            lit=lit,
            glass=glass,
            window=window,
            window_land=WINDOW_LAND,
            encoder_pocket=_centred(*board.encoder_shaft, pocket, pocket),
            encoder_pocket_top=value["M5a"] + ENCODER_GAP,
            shaft_hole=SHAFT_HOLE,
            usb_window=usb_window,
            display_module=module,
            rooms=_rooms(board, value, _around(module, glass), underside),
            knob_lift=_round_up(max(0.0, deck_top + KNOB_GAP - value["M5b"]), 0.1),
            print_scale=_print_scale(value),
            codes=tuple(sorted(m)),
            estimated=tuple(estimates(m)),
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


def _print_scale(value):
    """From the printed gauge, how much bigger to draw a part so that it comes out the size of the board.

    The smaller of what the gauge's two sides say, never under 1 and never over the cap. The gauge's
    edge is all first layers, which the plate spreads, and it bows between the jaws: read long or
    read short, it must not spoil a fit that holds without it.
    """
    read = min(drawn / value[code] for code, drawn in GAUGE_DRAWN.items())
    return min(max(read, 1.0), MAX_PRINT_SCALE)


def _display_module(board, value, glass):
    """The display's own board, which hangs from its header by the middle of its back edge."""
    pins = [pad for pad in board.pads if pad.part == DISPLAY_HEADER]
    x = sum(pin.x for pin in pins) / len(pins)
    back = pins[0].y - value["M4e"]
    module = Rect(x - value["M4p"] / 2, back, x + value["M4p"] / 2, back + value["M4l"])

    across = max(module.x0 - glass.x0, glass.x1 - module.x1)
    along = max(module.y0 - glass.y0, glass.y1 - module.y1)
    for overhang, codes in ((across, "M4p, M4x, M4w and M4s"), (along, "M4l, M4e, M4y, M4h, M4t and M4f")):
        if overhang > GLASS_OVERHANG:
            raise ValueError(f"the glass would overhang its module's board by {overhang:.2f} mm: check {codes}")
    return module


def _rooms(board, value, display, underside):
    def with_its_pins(part, body):
        return _around(body, *(pad.rect for pad in board.pads if pad.part == part)).grown(ROOM)

    parts = (
        Room(with_its_pins(CHIP, board.chip), value["M6a"] + CHIP_GAP),
        Room(display.grown(ROOM), underside),
        Room(with_its_pins(ENCODER, board.encoder_body), underside),
    )
    # A pin of a part nobody has measured gets all the height there is.
    tops = {USB_MODULE: value["M6b"] + CHIP_GAP}
    pins = tuple(
        Room(pad.rect.grown(PIN_ROOM), tops.get(pad.part, underside))
        for pad in board.pads
        if not any(room.rect.holds(pad.rect) for room in parts)
    )
    return parts + pins


def _around(*rects):
    return Rect(min(r.x0 for r in rects), min(r.y0 for r in rects), max(r.x1 for r in rects), max(r.y1 for r in rects))


def _round_up(height, step):
    return round(math.ceil(round(height / step, 6)) * step, 4)


def _centred(x, y, width, height):
    return Rect(x - width / 2, y - height / 2, x + width / 2, y + height / 2)


def _well(board, first_key, last_key, reach):
    x0, y0 = board.key_centre(first_key)
    x1, y1 = board.key_centre(last_key)
    return Rect(x0 - reach, y0 - reach, x1 + reach, y1 + reach)
