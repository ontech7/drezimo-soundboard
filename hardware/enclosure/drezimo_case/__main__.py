"""Generates the print files.

    uv run python -m drezimo_case coupon-a
"""

import sys
from pathlib import Path

from .board import load_board
from .coupon import build_coupon_a
from .export import export_part
from .inlay import LABEL_POSITIONS, LABEL_Y
from .measurements import MEASUREMENTS, estimates
from .params import Params
from .preview import Keys, render_preview

PRINT_DIR = Path(__file__).resolve().parents[1] / "print"
SECTION_KEYS = (37, 1)


def main(argv):
    if argv != ["coupon-a"]:
        sys.exit("usage: python -m drezimo_case coupon-a")

    board = load_board()
    p = Params.derive(board)
    section_x = board.key_centre(SECTION_KEYS[0])[0]
    keys = Keys(
        centres=tuple(board.key_centre(n)[1] for n in SECTION_KEYS),
        switch_width=MEASUREMENTS["M3a"].value,
        keycap_width=MEASUREMENTS["M2c"].value,
        skirt_at_rest=MEASUREMENTS["M2a"].value,
        skirt_pressed=MEASUREMENTS["M2b"].value,
    )

    body, inlay = build_coupon_a(board, p)
    written = export_part(
        "coupon-a", body, inlay, PRINT_DIR, section_x=section_x, keys=keys, title="Coupon (a): labels above the function row"
    )

    # The label position is still the owner's to confirm: a second picture shows the other one.
    for name, y in LABEL_POSITIONS.items():
        if y != LABEL_Y:
            picture = PRINT_DIR / f"coupon-a-labels-on-the-{name}.png"
            render_preview(
                picture, *build_coupon_a(board, p, y), section_x=section_x, keys=keys, title=f"Coupon (a): labels on the {name}"
            )
            written.append(picture)

    for path in written:
        print(path)
    if p.estimated:
        print(f"Built on estimates, not for the final print: {', '.join(estimates())}")


if __name__ == "__main__":
    main(sys.argv[1:])
