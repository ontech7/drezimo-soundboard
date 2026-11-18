import pytest
from helpers import NOMINAL, nominal

from drezimo_case.board import load_board
from drezimo_case.measurements import Measurement, estimates
from drezimo_case.params import EstimateError, Params


def derive_with(**measured):
    return Params.derive(load_board(), nominal(**measured))


def test_nominal():
    p = derive_with()
    assert p.deck_top == pytest.approx(7.8)
    assert p.well_floor == pytest.approx(5.0)
    # A quarter of a millimetre round each switch: what a keycap still hides, and room for a shell printed a little small.
    assert p.opening == pytest.approx(16.1)
    assert p.rib_width == pytest.approx(2.95)
    assert (p.fn_well.x0, p.fn_well.y0) == pytest.approx((7.675, 49.675))
    assert (p.pad_well.x1, p.pad_well.y1) == pytest.approx((122.625, 188.675))
    assert (p.bar.y0, p.bar.y1) == pytest.approx((69.375, 73.725))
    assert round(p.deck_top / p.layer, 6).is_integer()


def test_low_skirt_lowers_the_floor():
    assert derive_with(M2b=5.6).well_floor == pytest.approx(4.6)


def test_floor_too_thin_is_an_error():
    with pytest.raises(ValueError, match="rib"):
        derive_with(M2b=2.4)


def test_wide_switch_narrows_the_rib_until_it_fails():
    assert derive_with(M3a=15.9).rib_width == pytest.approx(2.65)
    with pytest.raises(ValueError, match="rib"):
        derive_with(M3a=17.6)


def test_case_grows_with_the_board_up_to_1mm():
    assert derive_with(M1w=124.9).case_size[0] == pytest.approx(130.9)
    with pytest.raises(ValueError, match="outline"):
        derive_with(M1w=125.6)


def test_final_parts_refuse_estimates():
    p = derive_with(M5a=6.45)
    with pytest.raises(EstimateError, match="M2b") as error:
        p.require_measured("M2b", "M5a")
    assert "M5a" not in str(error.value)
    p.require_measured("M5a")


def test_estimates_lists_what_is_not_measured():
    m = {"M2a": Measurement(10.5), "M2b": Measurement(6.4, measured=True)}
    assert estimates(m) == ["M2a"]


def test_unknown_codes_are_an_error():
    with pytest.raises(KeyError, match="M99"):
        derive_with().require_measured("M99")


def test_without_codes_every_estimate_blocks():
    with pytest.raises(EstimateError, match="M2a"):
        derive_with(M5a=6.45).require_measured()
    derive_with(**NOMINAL).require_measured()


@pytest.mark.parametrize(
    "typo",
    [
        {"M1w": 24.3}, {"M1h": 19.035}, {"M2a": 105.0}, {"M2b": 65.0}, {"M2c": 24.0}, {"M3a": 0.0}, {"M4a": 55.0}, {"M5a": 64.5}, {"M6a": 45.0},
        {"M1t": 16.0}, {"M4b": 50.0}, {"M4w": 2.97}, {"M4h": 149.0}, {"M4x": 7.47}, {"M4y": 214.0}, {"M4t": 19.0}, {"M4s": 24.0}, {"M4f": 60.0},
        {"M5b": 90.0}, {"M6b": 20.0}, {"M7a": 66.0}, {"M7b": 66.0}, {"M8w": 120.0}, {"M8h": 65.0}, {"P1s": 7.0}, {"P1l": 110.0},
    ],
)
def test_implausible_values_are_an_error(typo):
    with pytest.raises(ValueError, match=next(iter(typo))):
        derive_with(**typo)


def test_a_pressed_skirt_above_the_resting_one_is_an_error():
    with pytest.raises(ValueError, match="M2a"):
        derive_with(M2a=8.5, M2b=9.0)


def test_the_length_may_grow_by_1mm_too():
    assert derive_with(M1h=190.95).case_size[1] == pytest.approx(196.95)
    with pytest.raises(ValueError, match="M1h"):
        derive_with(M1h=191.65)


def as_tuple(rect):
    return rect.x0, rect.y0, rect.x1, rect.y1


def test_the_deck_says_which_part_sets_its_height():
    assert derive_with().deck_set_by == "M5a"
    raised = derive_with(M4b=6.0, M5b=10.0)
    assert (raised.deck_top, raised.deck_set_by) == (pytest.approx(8.4), "M4b")
    assert derive_with(M6b=6.0, M5b=10.0).deck_top == pytest.approx(8.6)


def test_a_knob_that_would_rub_the_deck_is_an_error():
    # M5b is read with the knob pressed to the click: no travel is assumed. Half a millimetre must stay.
    derive_with(M5b=8.3)
    with pytest.raises(ValueError, match="M5b.*7.8"):
        derive_with(M5b=8.2)


def test_the_walls_go_down_to_a_bottom_that_clears_the_usb_plug():
    p = derive_with()
    assert (p.board_thickness, p.under_board, p.seat, p.wall_bottom, p.rim) == pytest.approx((1.6, 8.9, -10.5, -12.5, 1.2))
    assert as_tuple(p.usb_window) == pytest.approx((23.5, -10.435, 36.5, -2.935), abs=0.001)
    assert p.usb_window.y0 >= p.seat
    # A thin plug still decides; the module's drop and its clearance alone would give 7.5.
    assert derive_with(M8h=4.0).under_board == pytest.approx(7.6)
    assert derive_with(M1t=1.8).wall_bottom == pytest.approx(-12.7)
    # Pin ends under the module that hang lower than its connector push the bottom down.
    assert derive_with(M7b=9.0).under_board == pytest.approx(9.9)


def test_the_screw_holes_stop_under_the_well_floor():
    # Two of the six stand a sixth of a millimetre into a well's wall: a deeper hole would open into it.
    p = derive_with()
    assert (p.boss_radius, p.screw_hole, p.screw_hole_top) == pytest.approx((3.25, 2.7, 4.6))
    assert derive_with(M2b=5.0).screw_hole_top == pytest.approx(3.6)


def test_the_window_follows_the_lit_area_and_a_sight_line_from_the_front():
    p = derive_with()
    assert as_tuple(p.lit) == pytest.approx((62.85, 16.95, 92.55, 31.85))
    assert as_tuple(p.glass) == pytest.approx((60.45, 15.05, 94.95, 37.85))
    assert as_tuple(p.window) == pytest.approx((61.85, 16.2, 93.55, 33.35))
    assert p.window_land == pytest.approx(0.3)

    moved = derive_with(M4x=75.7, M4y=20.4, M4w=30.0)
    assert as_tuple(moved.window) == pytest.approx((62.7, 15.2, 94.7, 32.35))
    # A higher deck stands further over the pixels, so the front edge moves away.
    assert derive_with(M6b=6.0, M5b=10.0).window.y1 == pytest.approx(31.85 + 2.3)


def test_a_window_that_would_show_the_edge_of_the_glass_is_an_error():
    with pytest.raises(ValueError, match="glass"):
        derive_with(M4t=0.9)


def test_the_encoder_gets_a_pocket_and_a_hole_for_its_shaft():
    p = derive_with()
    # Round the 12 mm square the body measures, not the 11.6 mm the footprint draws from back to front.
    assert as_tuple(p.encoder_pocket) == pytest.approx((106.025, 20.25, 119.525, 33.75))
    assert (p.encoder_pocket_top, p.shaft_hole) == pytest.approx((6.7, 8.0))
    assert p.deck_top - p.encoder_pocket_top >= 1.1 - 1e-9


def test_the_display_module_hangs_centred_on_its_header():
    module = derive_with().display_module
    assert (module.x0, module.y0, module.x1, module.y1) == pytest.approx((59.93, 8.81, 95.47, 42.4), abs=0.01)

    wider = derive_with(M4p=36.0, M4l=34.0, M4e=2.5).display_module
    assert (wider.x0, wider.y0, wider.x1, wider.y1) == pytest.approx((59.7, 8.0, 95.7, 42.0), abs=0.01)


def test_a_glass_that_would_overhang_its_module_is_an_error():
    # Under a millimetre is two readings disagreeing, and the display's room follows the glass; more is a slip.
    assert derive_with(M4p=34.0).rooms[1].rect.x0 == pytest.approx(60.45 - 1.5, abs=0.01)
    with pytest.raises(ValueError, match="M4p"):
        derive_with(M4p=33.0, M4x=75.7)
    with pytest.raises(ValueError, match="M4l"):
        derive_with(M4l=30.0, M4e=4.0)


def test_every_part_of_the_top_strip_keeps_its_room_under_the_deck():
    p = derive_with()
    underside = p.deck_top - p.deck_thickness
    chip, display, encoder, *pins = p.rooms

    # The chip with its legs, 1.5 mm round it, up to half a millimetre over its top.
    assert (chip.rect.x0, chip.rect.y0, chip.rect.x1, chip.rect.y1) == pytest.approx((7.7, 26.23, 19.92, 47.01), abs=0.01)
    assert chip.top == pytest.approx(5.0)
    # The display's module and the encoder with its terminals, all the way up to the deck.
    assert (display.rect.x0, display.rect.y0, display.rect.x1, display.rect.y1) == pytest.approx((58.43, 7.31, 96.97, 43.9), abs=0.01)
    assert (encoder.rect.x0, encoder.rect.y0, encoder.rect.x1, encoder.rect.y1) == pytest.approx((102.775, 18.3, 122.275, 35.7), abs=0.01)
    assert display.top == encoder.top == pytest.approx(underside)
    # Each pin end of the USB module, which comes up through the board.
    assert len(pins) == 23
    assert {round(pin.rect.width, 2) for pin in pins} == {2.7}
    assert {round(pin.top, 6) for pin in pins} == {2.5}

    assert {round(room.top, 6) for room in derive_with(M6b=4.0).rooms[3:]} == {4.5}
    tall = derive_with(M6b=7.5, M5b=14.0)
    assert {round(room.top, 6) for room in tall.rooms[3:]} == {round(tall.deck_top - tall.deck_thickness, 6)}


def test_a_value_known_without_a_caliper_is_no_estimate():
    known = nominal(M5a=6.45)
    known["M1t"] = Measurement(1.6, known="as ordered from the fab")
    p = Params.derive(load_board(), known)
    assert "M1t" not in p.estimated and "M5a" not in p.estimated and "M2a" in p.estimated
    assert "M1t" not in estimates(known)


def test_the_readings_file_says_where_each_unmeasured_value_comes_from():
    from drezimo_case.measurements import MEASUREMENTS

    assert {code for code, reading in MEASUREMENTS.items() if reading.known} == {"M1t", "M2a", "M2c", "M3a"}
    assert not any(reading.known and reading.measured for reading in MEASUREMENTS.values())


def test_the_shell_is_drawn_bigger_by_what_the_printer_shrinks():
    # The printed inlay gauge, drawn 70 x 101 mm, is the printer's own ruler.
    assert derive_with().print_scale == pytest.approx(1.0)
    assert derive_with(P1s=69.86, P1l=100.80).print_scale == pytest.approx(1.002, abs=1e-4)


def test_the_gauge_never_makes_the_shell_smaller_nor_much_bigger():
    # Its edge is all first layers, which the plate spreads: a gauge that reads long says nothing.
    assert derive_with(P1s=70.07, P1l=101.10).print_scale == 1.0
    # Squeezed between the jaws it bows and reads short.
    assert derive_with(P1s=69.50, P1l=100.40).print_scale == pytest.approx(1.0025)


def test_gauge_sides_that_disagree_give_the_smaller_correction():
    # 0.2 % of shrink under a lip of 0.1 mm: the short side hides more of it.
    assert derive_with(P1s=69.96, P1l=100.90).print_scale == pytest.approx(70 / 69.96)
