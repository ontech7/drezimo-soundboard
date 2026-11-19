"""Where the picture of each part cuts it, and what stands on the board along the cut."""

from .board import INSET, PITCH
from .bottom import PEG_HEAD, PEG_HEAD_HEIGHT, PEG_NECK
from .params import COUNTERBORE_PLAY, FOOT_PLAY, PIXEL_DEPTH, SCREW
from .preview import INK, INLAY, PRESSED, Keys, Section

COUPON_KEYS = (37, 1)
GLASS, METAL, MODULE = "#33506b", "#8f9099", "#2c5d8c"
# Parts that no reading covers, as the drawings need them. All nominal: they shape nothing.
GLASS_THICKNESS = 1.4
GLASS_ON_ITS_BOARD = 3.02
KNOB_WIDTH, KNOB_HEIGHT = 14.4, 17.0
SHAFT_WIDTH, SHAFT_LENGTH = 6.0, 20.0
USB_SPACER, USB_BOARD = 2.5, 1.04
USB_CONNECTOR_LENGTH, USB_CONNECTOR_HEIGHT = 7.35, 3.03
# Drawn each side of a keyhole in its cut: the key over it and the ribs of its cell.
AROUND_A_KEYHOLE = 18.0
# A peg's neck is this much longer than the plate is thick, and this much of the stand under it is drawn.
PEG_NECK_SPARE, STAND_DRAWN = 0.4, 3.0


def along_the_keys(board, p, readings):
    """One cut, through a function key and the pad in front of it."""
    x = board.key_centre(COUPON_KEYS[0])[0]
    return (Section(x, "Section, back to the left — dashed keycap: fully pressed", keys=_keys(board, readings, COUPON_KEYS)),)


def through_the_shell(board, p, readings):
    """Cuts through the USB connector, the display, the knob and three of the screws."""
    pcb = (INSET, INSET + board.outline[1], readings["M1t"])
    # From the back wall to the first pads: the top strip, drawn large.
    strip = (-1.0, p.pad_well.y0 + PITCH)

    def cut(x, title, *furniture):
        column = sorted(n for n, (key_x, _) in board.keys.items() if abs(key_x - x) < PITCH / 2)
        return Section(x, title, _keys(board, readings, column), pcb, furniture, strip)

    usb = board.usb_module
    module_top = -(readings["M1t"] + USB_SPACER)
    connector_bottom = -(readings["M1t"] + readings["M7a"])

    glass, lit, module = p.glass, p.lit, p.display_module
    glass_top = readings["M4a"]
    pixels = glass_top - PIXEL_DEPTH
    over_the_deck = p.deck_top + 3
    sight_line = [(lit.y1, pixels), (lit.y1 + over_the_deck - pixels, over_the_deck)]

    shaft_x, shaft_y = board.encoder_shaft
    body, body_top = board.encoder_body, readings["M5a"]
    knob_bottom = readings["M5b"]
    knob_top = knob_bottom + KNOB_HEIGHT

    return (
        cut(
            usb.centre[0],
            "Through the USB connector — back of the case to the left",
            (_box(usb.y0 + 1.0, usb.y1, module_top - USB_BOARD, module_top), MODULE, True),
            (_box(usb.y0, usb.y0 + USB_CONNECTOR_LENGTH, connector_bottom, connector_bottom + USB_CONNECTOR_HEIGHT), METAL, True),
        ),
        cut(
            lit.centre[0],
            "Through the display — orange: a sight line at 45° from the front edge of the lit area",
            (_box(module.y0, module.y1, glass_top - GLASS_ON_ITS_BOARD, glass_top - GLASS_THICKNESS), MODULE, True),
            (_box(glass.y0, glass.y1, glass_top - GLASS_THICKNESS, glass_top), GLASS, True),
            (_box(lit.y0, lit.y1, pixels - 0.1, pixels + 0.1), INLAY, True),
            (sight_line, PRESSED, False),
        ),
        cut(
            shaft_x,
            "Through the knob",
            (_box(body.y0, body.y1, 0, body_top), METAL, True),
            (_box(shaft_y - SHAFT_WIDTH / 2, shaft_y + SHAFT_WIDTH / 2, body_top, min(body_top + SHAFT_LENGTH, knob_top)), METAL, True),
            (_box(shaft_y - KNOB_WIDTH / 2, shaft_y + KNOB_WIDTH / 2, knob_bottom, knob_top), INK, False),
        ),
        Section(board.holes[0][0], "Through three of the six screw holes", board=pcb),
    )


def through_the_closed_case(board, p, readings):
    """Three cuts with the shell and the board in place: through a screw, a keyhole and a foot's recess."""
    pcb = (INSET, INSET + board.outline[1], readings["M1t"])

    def cut(x, title, span, *furniture):
        column = sorted(n for n, (key_x, _) in board.keys.items() if abs(key_x - x) < PITCH / 2)
        return Section(x, title, _keys(board, readings, column) if column else None, pcb, furniture, span)

    screw_x, screw_y = board.holes[0]
    keyhole_x, locked = p.keyholes[0]
    foot = p.feet[0]
    foot_top = p.wall_bottom + p.foot_recess
    return (
        cut(screw_x, "Through the back left screw — back of the case to the left", (-1.0, 45.0), (_screw(p, screw_y), METAL, True)),
        cut(
            keyhole_x,
            "Through a keyhole — orange: a peg of the stand, locked",
            (locked - AROUND_A_KEYHOLE, locked + p.keyhole_slide + AROUND_A_KEYHOLE),
            (_peg(p, locked), PRESSED, False),
        ),
        cut(
            foot.centre[0],
            "Through a foot's recess — outlined: the rubber foot",
            (-1.0, 45.0),
            (_box(foot.y0 + FOOT_PLAY, foot.y1 - FOOT_PLAY, foot_top - p.foot_height, foot_top), INK, False),
        ),
    )


def _screw(p, y):
    """A countersunk screw seen from the side, from the flat face of its head to its tip."""
    head, shank = (p.counterbore - COUNTERBORE_PLAY) / 2, SCREW / 2
    rim = p.seat_wide + COUNTERBORE_PLAY / 2 - p.head_face
    cone = p.head_face + rim + head - shank
    return [
        (y - head, p.head_face), (y + head, p.head_face), (y + head, p.head_face + rim), (y + shank, cone),
        (y + shank, p.screw_tip), (y - shank, p.screw_tip), (y - shank, cone), (y - head, p.head_face + rim),
    ]  # fmt: skip


def _peg(p, y):
    """One of the stand's pegs in its keyhole: the neck through the plate, the head in the pocket behind it."""
    neck, head = PEG_NECK / 2, PEG_HEAD / 2
    stand, under_the_head = p.wall_bottom - STAND_DRAWN, p.plate_top + PEG_NECK_SPARE
    return [
        (y - neck, stand), (y + neck, stand), (y + neck, under_the_head), (y + head, under_the_head),
        (y + head, under_the_head + PEG_HEAD_HEIGHT), (y - head, under_the_head + PEG_HEAD_HEIGHT),
        (y - head, under_the_head), (y - neck, under_the_head),
    ]  # fmt: skip


def _keys(board, readings, numbers):
    return Keys(
        centres=tuple(board.key_centre(n)[1] for n in numbers),
        switch_width=readings["M3a"],
        keycap_width=readings["M2c"],
        skirt_at_rest=readings["M2a"],
        skirt_pressed=readings["M2b"],
    )


def _box(y0, y1, z0, z1):
    return [(y0, z0), (y1, z0), (y1, z1), (y0, z1)]
