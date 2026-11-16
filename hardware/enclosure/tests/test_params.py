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
    with pytest.raises(KeyError, match="M7a"):
        derive_with().require_measured("M7a")


def test_without_codes_every_estimate_blocks():
    with pytest.raises(EstimateError, match="M2a"):
        derive_with(M5a=6.45).require_measured()
    derive_with(**NOMINAL).require_measured()


@pytest.mark.parametrize(
    "typo",
    [{"M1w": 24.3}, {"M1h": 19.035}, {"M2a": 105.0}, {"M2b": 65.0}, {"M2c": 24.0}, {"M3a": 0.0}, {"M4a": 55.0}, {"M5a": 64.5}, {"M6a": 45.0}],
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
