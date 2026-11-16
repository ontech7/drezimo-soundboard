import pytest

from drezimo_case.board import load_board
from drezimo_case.measurements import Measurement, estimates
from drezimo_case.params import EstimateError, Params

NOMINAL = {
    "M1w": 124.3,
    "M1h": 190.35,
    "M2a": 10.5,
    "M2b": 6.5,
    "M2c": 18.1,
    "M3a": 15.6,
    "M4a": 5.5,
    "M5a": 6.45,
    "M6a": 4.5,
}


def derive_with(**overrides):
    m = {code: Measurement(value) for code, value in NOMINAL.items()}
    m.update({code: Measurement(value, measured=True) for code, value in overrides.items()})
    return Params.derive(load_board(), m)


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
