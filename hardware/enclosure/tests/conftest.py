import pytest
from helpers import nominal

from drezimo_case.board import load_board
from drezimo_case.measurements import MEASUREMENTS
from drezimo_case.params import Params

AS_MEASURED = "--as-measured"


def pytest_addoption(parser):
    parser.addoption(
        AS_MEASURED,
        action="store_true",
        help="build the parts on the board's own readings instead of the tests' nominal ones (for tests/test_shell.py)",
    )


@pytest.fixture(scope="session")
def board():
    return load_board()


@pytest.fixture(scope="session")
def readings(request):
    """What the session's parts are built on, by code."""
    return MEASUREMENTS if request.config.getoption(AS_MEASURED) else nominal()


@pytest.fixture(scope="session")
def p(board, readings):
    return Params.derive(board, readings)


@pytest.fixture(scope="session")
def deck(board, p):
    from drezimo_case.deck import build_deck

    return build_deck(board, p)


@pytest.fixture(scope="session")
def coupon(board, p):
    from drezimo_case.coupon import build_coupon_a

    return build_coupon_a(board, p)


@pytest.fixture(scope="session")
def body(coupon):
    return coupon[0]


@pytest.fixture(scope="session")
def inlay(coupon):
    return coupon[1]


@pytest.fixture(scope="session")
def shell_body(board, p):
    from drezimo_case.shell import build_body

    return build_body(board, p)


@pytest.fixture(scope="session")
def shell(board, p):
    from drezimo_case.shell import build_shell

    return build_shell(board, p)
