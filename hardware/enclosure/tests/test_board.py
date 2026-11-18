from collections import Counter

import pytest

from drezimo_case.board import load_board


def test_outline_and_case():
    b = load_board()
    assert b.outline == pytest.approx((124.3, 190.35), abs=0.01)
    assert b.case_size == pytest.approx((130.3, 196.35), abs=0.01)


def test_key_centres():
    b = load_board()
    for c in range(6):
        assert b.key_centre(37 + c) == pytest.approx((17.525 + 19.05 * c, 59.525), abs=0.01)
        for r in range(6):
            assert b.key_centre(r * 6 + c + 1) == pytest.approx((17.525 + 19.05 * c, 83.575 + 19.05 * r), abs=0.01)


def test_holes():
    expected = {(x, y) for x in (6.5, 123.8) for y in (6.5, 131.2, 189.85)}
    assert {(round(x, 2), round(y, 2)) for x, y in load_board().holes} == expected


def test_the_display_hangs_from_its_header():
    assert load_board().display_pin1 == pytest.approx((73.89, 10.5), abs=0.01)


def test_the_encoder_and_its_body():
    b = load_board()
    assert b.encoder_shaft == pytest.approx((112.775, 27.0), abs=0.01)
    body = b.encoder_body
    assert (body.x0, body.y0, body.x1, body.y1) == pytest.approx((106.775, 21.2, 118.775, 32.8), abs=0.01)
    assert body.centre == pytest.approx(b.encoder_shaft, abs=0.01)


def test_the_chip_left_of_the_display():
    chip = load_board().chip
    assert (chip.x0, chip.y0, chip.x1, chip.y1) == pytest.approx((10.64, 27.73, 16.99, 45.51), abs=0.01)


def test_the_usb_module_under_the_board_and_its_pins_on_top():
    b = load_board()
    module = b.usb_module
    assert (module.x0, module.x1) == pytest.approx((21.05, 38.95), abs=0.01)
    assert module.centre[0] == pytest.approx(30.0, abs=0.01)
    # Its USB end is at the back edge.
    assert (module.y0, module.y1) == pytest.approx((2.27, 26.8), abs=0.01)

    assert len(b.usb_pins) == 23
    for row in (22.38, 37.62):
        assert sum(1 for x, _ in b.usb_pins if x == pytest.approx(row, abs=0.01)) == 9
    assert all(module.x0 < x < module.x1 and module.y0 < y < module.y1 for x, y in b.usb_pins)


def test_every_pin_soldered_through_the_board_outside_the_keys():
    pads = load_board().pads
    assert Counter(pad.part for pad in pads) == {"J1": 4, "ENC1": 7, "U2": 14, "U1": 23}

    # The encoder's terminals stand outside its body, three behind it and two in front; its two lugs are wider.
    encoder = sorted((pad.x, pad.y, pad.reach) for pad in pads if pad.part == "ENC1")
    assert encoder == [
        pytest.approx(pin, abs=0.01)
        for pin in (
            (105.275, 24.5, 1.0), (105.275, 27.0, 1.0), (105.275, 29.5, 1.0),
            (112.775, 21.4, 1.6), (112.775, 32.6, 1.6),
            (119.775, 24.5, 1.0), (119.775, 29.5, 1.0),
        )
    ]
    header = sorted(pad.x for pad in pads if pad.part == "J1")
    assert header == pytest.approx([73.89, 76.43, 78.97, 81.51], abs=0.01)
    pin = next(pad for pad in pads if pad.part == "U2")
    assert (pin.rect.x0, pin.rect.y0, pin.rect.x1, pin.rect.y1) == pytest.approx((pin.x - 0.8, pin.y - 0.8, pin.x + 0.8, pin.y + 0.8))
