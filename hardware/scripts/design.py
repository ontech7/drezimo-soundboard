"""Single source of truth for the electrical design: key matrix, GPIO map and LED chain.

Shared by build_schematic.py, build_pcb.py and check_design.py.
"""

COLS, ROWS = 6, 6                    # sound pads
FN_ROW = ROWS                        # matrix row of the function keys (row 6)
FN_KEYS = ["PREV", "NEXT", "STOP", "MUTE ME", "VOICE", "HEAR ME"]

# Key number n (= SWn, Dn, LEDn) -> (matrix row, matrix column).
# Pads 1..36 row-major from the top-left, function keys 37..42 left to right.
KEYS = {r * COLS + c + 1: (r, c) for r in range(ROWS) for c in range(COLS)}
KEYS.update({ROWS * COLS + c + 1: (FN_ROW, c) for c in range(COLS)})

# LED data chain: every row left->right (function row, then pad rows 0..5). All LEDs share one
# orientation and one routing pattern; the end of each row returns to the start of the next one.
CHAIN_ORDER = list(range(37, 43)) + list(range(1, ROWS * COLS + 1))
assert sorted(CHAIN_ORDER) == list(range(1, 43))

# Net feeding the DIN of the k-th LED in the chain (LED_D0 comes from R1).
LED_CHAIN_NETS = [f"LED_D{k}" for k in range(len(CHAIN_ORDER))]


def key_cell(chain_index):
    """Schematic grid position for the k-th LED (7 per row), purely cosmetic."""
    return chain_index % 7, chain_index // 7


# RP2040-Zero pad -> net (None = not connected). Pads are numbered counter-clockwise from 5V,
# looking at the module's USB-C face (see hardware/lib/drezimo.pretty/RP2040-Zero.kicad_mod).
# Chosen for routing, seen from above: the left column takes the ROW bus coming up the left edge
# (outermost bus line on the top pin), the 5-pin row takes COL0..5 coming up from the keys, the
# right column (power, I2C, encoder) faces the OLED and the knob.
ZERO_PINS = {
    1: "VBUS",          # 5V = USB VBUS (measured 5.02 V: no diode on the module)
    2: "GND",
    3: "+3V3",          # on-board LDO output (OLED supply)
    4: "SCL",           # GP29 I2C0 SCL
    5: "SDA",           # GP28 I2C0 SDA
    6: "ENC_SW",        # GP27
    7: "ENC_A",         # GP26
    8: "ENC_B",         # GP15
    9: "COL5",          # GP14
    10: "COL4",         # GP13
    11: "COL3",         # GP12
    12: "COL2",         # GP11
    13: "COL1",         # GP10
    14: "COL0",         # GP9
    15: "LED_DIN_3V3",  # GP8  LED data, through the 74AHCT125
    16: None,           # GP7  spare
    17: "ROW6",         # GP6  function keys
    18: "ROW0",         # GP5
    19: "ROW1",         # GP4
    20: "ROW2",         # GP3
    21: "ROW3",         # GP2
    22: "ROW4",         # GP1
    23: "ROW5",         # GP0
}

# Firmware-facing GPIO map (documentation and future firmware config)
GPIO = {
    "COLS": [9, 10, 11, 12, 13, 14],
    "ROWS": [5, 4, 3, 2, 1, 0, 6],
    "LED_DATA": 8,
    "I2C_SDA": 28,
    "I2C_SCL": 29,
    "ENC_A": 26,
    "ENC_B": 15,
    "ENC_SW": 27,
}
