import pytest
from helpers import NOMINAL, nominal

from drezimo_case.board import Rect, load_board
from drezimo_case.measurements import Measurement, estimates
from drezimo_case.params import BOTTOM_ONLY, EstimateError, Params


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


def test_a_knob_that_would_rub_the_deck_is_to_sit_higher_on_its_shaft():
    # M5b is read with the knob pressed to its click: no travel is assumed. Half a millimetre must stay
    # over the deck. A knob can be set higher on its shaft, so it stops no shell.
    assert derive_with(M5b=8.3).knob_lift == pytest.approx(0.0)
    assert derive_with(M5b=8.2).knob_lift == pytest.approx(0.1)
    # A display that sits a quarter of a millimetre high raises the deck under the knob too.
    assert derive_with(M4a=5.75).knob_lift == pytest.approx(0.2)


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

    assert {code for code, reading in MEASUREMENTS.items() if reading.known} == {"M1t", "M2a", "M2c", "M3a", "M4f", "M4s", "M4t"}
    assert not any(reading.known and reading.measured for reading in MEASUREMENTS.values())


def test_the_board_is_read_and_the_shell_may_be_built():
    from drezimo_case.measurements import MEASUREMENTS

    Params.derive(load_board(), MEASUREMENTS).require_measured(but=BOTTOM_ONLY)


def test_the_bottom_may_be_built_on_the_owners_screws():
    from drezimo_case.measurements import MEASUREMENTS

    p = Params.derive(load_board(), MEASUREMENTS)
    p.require_measured()
    p.check_bottom()
    # The owner's M3 x 10 run from 10.0 to 10.4 mm: the longest is drawn, the shortest keeps a diameter of thread and more.
    assert p.screw_length == 10.4
    assert p.screw_tip - (10.4 - 10.0) >= 3.5


def test_where_the_lit_area_lies_in_its_glass_shapes_nothing():
    # The glass was measured loose, not how the light sits in it: the window follows the light alone.
    drawn, moved = derive_with(), derive_with(M4t=1.2, M4s=2.0, M4f=4.0)
    assert moved.glass != drawn.glass
    assert (moved.window, moved.rooms, moved.display_module) == (drawn.window, drawn.rooms, drawn.display_module)


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


def test_the_bottoms_plate_and_the_screws_seat():
    p = derive_with()
    # The standoffs must reach the board before the plate reaches the shell's seat.
    assert p.plate_top == pytest.approx(p.seat - 0.2) == pytest.approx(-10.7)
    assert p.standoff_top == pytest.approx(-1.6)
    assert p.screw_tip == pytest.approx(4.0)
    assert p.head_face == pytest.approx(-6.4)
    assert p.counterbore == pytest.approx(6.35)
    # Of the head's 1.96 mm its cone is 1.425: the rest is a rim that stands out of the seat.
    assert p.seat_wide == pytest.approx(-6.115)
    assert p.seat_narrow == pytest.approx(-4.64)
    assert p.column_radius == pytest.approx(4.975)
    p.check_bottom()


@pytest.mark.parametrize("length", [9.0, 10.0, 10.4, 12.0])
def test_the_screws_tip_stays_short_of_the_end_of_its_hole_whatever_its_length(length):
    p = derive_with(M9l=length)
    assert p.screw_tip == pytest.approx(p.screw_hole_top - 0.6)
    assert p.head_face == pytest.approx(p.screw_tip - length)
    p.check_bottom()


def test_a_screw_too_long_would_stand_out_of_the_bottom():
    with pytest.raises(ValueError, match=r"M9l.*4\.4 mm"):
        derive_with(M9l=20.6).check_bottom()


def test_a_screw_too_short_leaves_no_standoff_over_its_head():
    with pytest.raises(ValueError, match="M9l"):
        derive_with(M9l=6.0).check_bottom()


def test_a_wrong_screw_stops_the_bottom_and_not_the_shell():
    assert derive_with(M9l=20.6).deck_top == derive_with().deck_top


def test_a_foot_must_stand_out_of_its_recess():
    with pytest.raises(ValueError, match="M10h"):
        derive_with(M10h=1.5).check_bottom()


def test_four_feet_near_the_corners_beside_the_screws():
    p = derive_with()
    width, length = p.case_size
    back_left = Rect(11.1, 3.5, 31.9, 24.3)
    expected = [
        back_left,
        Rect(width - back_left.x1, back_left.y0, width - back_left.x0, back_left.y1),
        Rect(back_left.x0, length - back_left.y1, back_left.x1, length - back_left.y0),
        Rect(width - back_left.x1, length - back_left.y1, width - back_left.x0, length - back_left.y0),
    ]
    assert len(p.feet) == 4
    for foot, wanted in zip(p.feet, expected):
        assert (foot.x0, foot.y0, foot.x1, foot.y1) == pytest.approx((wanted.x0, wanted.y0, wanted.x1, wanted.y1))


def test_the_shell_does_not_wait_for_the_screw_and_the_bottom_does():
    p = derive_with(**{code: value for code, value in NOMINAL.items() if code not in BOTTOM_ONLY})
    p.require_measured(but=BOTTOM_ONLY)
    with pytest.raises(EstimateError, match="M9l"):
        p.require_measured()


def test_everything_under_the_board_has_a_depth():
    board = load_board()
    p = derive_with()
    assert len(p.under) == len(board.under)
    assert all(entry.drop is not None for entry in p.under)
    # Under the USB module the bottom is plain plate, and a pin's tip that no reading covers may hang as far.
    (module,) = [entry for entry in p.under if entry.rect == board.usb_module]
    header = [entry for entry in p.under if entry.part == "J1"]
    assert len(header) == 4 and {entry.drop for entry in (module, *header)} == {p.under_board}


def test_the_points_between_four_keys_and_the_lines_through_them():
    p = derive_with()
    xs = [27.05, 46.1, 65.15, 84.2, 103.25]
    ys = [71.55, 93.1, 112.15, 131.2, 150.25, 169.3]
    assert sorted(p.nodes) == [pytest.approx(node) for node in sorted((x, y) for x in xs for y in ys)]
    # One more line half a key pitch outside the keys on every side.
    assert p.rib_lines_x == pytest.approx([8.0, *xs, 122.3])
    assert p.rib_lines_y == pytest.approx([50.0, *ys, 188.35])


def test_four_keyholes_under_keys_wide_apart_each_in_the_middle_of_its_cell():
    board = load_board()
    p = derive_with()
    assert p.keyhole_slide == 8.0
    # Where a peg ends when locked; it goes in 8 mm nearer the front.
    assert list(p.keyholes) == [pytest.approx((x, y - 4.0)) for x, y in (board.key_centre(n) for n in (7, 12, 25, 30))]
