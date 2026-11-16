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
