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
    assert p.opening == pytest.approx(16.0)
    assert p.rib_width == pytest.approx(3.05)
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
    assert derive_with(M3a=15.9).rib_width == pytest.approx(2.75)
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
        {"M5b": 90.0}, {"M6b": 20.0}, {"M7a": 66.0}, {"M8w": 120.0}, {"M8h": 65.0},
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
    with pytest.raises(ValueError, match="M5b.*7.8"):
        derive_with(M5b=8.0)


def test_the_walls_go_down_to_a_bottom_that_clears_the_usb_plug():
    p = derive_with()
    assert (p.board_thickness, p.under_board, p.seat, p.wall_bottom, p.rim) == pytest.approx((1.6, 8.9, -10.5, -12.5, 1.2))
    assert as_tuple(p.usb_window) == pytest.approx((23.5, -10.435, 36.5, -2.935), abs=0.001)
    assert p.usb_window.y0 >= p.seat
    # A thin plug still decides; the module's drop and its clearance alone would give 7.5.
    assert derive_with(M8h=4.0).under_board == pytest.approx(7.6)
    assert derive_with(M1t=1.8).wall_bottom == pytest.approx(-12.7)


def test_the_bosses_leave_deck_over_their_screw_holes():
    p = derive_with()
    assert (p.boss_radius, p.screw_hole, p.screw_hole_top) == pytest.approx((3.25, 2.7, 6.6))
    assert derive_with(M4a=4.0, M4b=4.0, M5a=5.0, M6a=3.0, M5b=8.0).screw_hole_top == pytest.approx(6.4 - 1.2)


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
    assert as_tuple(p.encoder_pocket) == pytest.approx((106.025, 20.45, 119.525, 33.55))
    assert (p.encoder_pocket_top, p.shaft_hole) == pytest.approx((6.7, 8.0))
    assert p.deck_top - p.encoder_pocket_top >= 1.1 - 1e-9
