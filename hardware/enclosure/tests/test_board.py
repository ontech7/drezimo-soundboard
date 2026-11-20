from collections import Counter

import pytest

from drezimo_case.board import SMD_DROP, SWITCH_DROP, load_board


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


def under(part):
    return [entry for entry in load_board().under if entry.part == part]


def test_what_is_soldered_under_the_board():
    parts = Counter(entry.part.rstrip("0123456789") for entry in load_board().under if entry.drop == SMD_DROP)
    # A capacitor under each key, one by the USB module and the big one on the supply.
    assert parts == {"LED": 42, "D": 42, "C": 44, "F": 1, "R": 1}
    assert len(under("C43")) == 1


def test_a_keys_led_and_diode_hang_where_the_board_file_puts_them():
    b = load_board()
    x, y = b.key_centre(8)
    (led,) = under("LED8")
    assert (led.rect.x0, led.rect.y0, led.rect.x1, led.rect.y1) == pytest.approx(
        (x - 3.4, y + 3.31, x + 3.4, y + 5.63), abs=0.3
    )
    (diode,) = under("D8")
    assert diode.rect.centre == pytest.approx((x - 7.3, y + 1.4), abs=0.05)
    # Turned a quarter: its two pads lie one behind the other.
    assert 3.7 <= diode.rect.height <= 4.3
    assert 1.2 <= diode.rect.width <= 1.8 + 1e-6


def test_the_usb_module_hangs_as_far_as_its_connector_which_a_reading_gives():
    b = load_board()
    (module,) = [entry for entry in under("U1") if entry.rect == b.usb_module]
    assert module.drop is None


def test_a_switch_hangs_two_pins_and_three_pegs():
    b = load_board()
    switches = [entry for entry in b.under if entry.part.startswith("SW")]
    assert len(switches) == 42 * 5
    assert {entry.drop for entry in switches} == {SWITCH_DROP}
    x, y = b.key_centre(8)
    assert any(entry.rect.centre == pytest.approx((x + 2.54, y - 5.08), abs=0.01) for entry in under("SW8"))
    widths = sorted(round(entry.rect.width, 2) for entry in under("SW8"))
    assert widths == [1.75, 1.75, 2.2, 2.2, 4.0]


def test_the_tip_of_a_pin_no_reading_covers_hangs_no_one_knows_how_far():
    b = load_board()
    tips = [entry for entry in b.under if entry.drop is None and entry.rect != b.usb_module]
    assert Counter((entry.part, entry.rect) for entry in tips) == Counter((pad.part, pad.rect) for pad in b.pads)
    assert len(tips) == 48


def test_everything_under_the_board_is_under_the_board():
    b = load_board()
    width, length = b.outline
    for entry in b.under:
        assert 3.0 <= entry.rect.x0 < entry.rect.x1 <= 3.0 + width, entry
        # The USB module's connector end stands out over the board's back edge.
        back = b.usb_module.y0 if entry.rect == b.usb_module else 3.0
        assert back <= entry.rect.y0 < entry.rect.y1 <= 3.0 + length, entry
