import pytest
from helpers import nominal

from drezimo_case.board import load_board
from drezimo_case.params import Params


@pytest.fixture(scope="session")
def board():
    return load_board()


@pytest.fixture(scope="session")
def p(board):
    return Params.derive(board, nominal())


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
