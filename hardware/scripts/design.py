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

# LED data chain: function row left->right, then pad rows serpentine
# (row 0 right->left, row 1 left->right, ...). Keeps every hop between neighbours.
CHAIN_ORDER = list(range(37, 43))
for r in range(ROWS):
    row = [r * COLS + c + 1 for c in range(COLS)]
    CHAIN_ORDER += row[::-1] if r % 2 == 0 else row
assert sorted(CHAIN_ORDER) == list(range(1, 43))

# Net feeding the DIN of the k-th LED in the chain (LED_D0 comes from R1).
LED_CHAIN_NETS = [f"LED_D{k}" for k in range(len(CHAIN_ORDER))]


def key_cell(chain_index):
    """Schematic grid position for the k-th LED (7 per row), purely cosmetic."""
    return chain_index % 7, chain_index // 7


# RP2040-Zero pad -> net (None = not connected). Pads are numbered counter-clockwise from 5V,
# looking at the module's USB-C face (see hardware/lib/drezimo.pretty/RP2040-Zero.kicad_mod).
# The left column (COL0..5, ROW0..1, LED data) and the 5-pin row (ROW2..6) face the key matrix;
# the right column (power, I2C, encoder) faces the OLED and the knob.
ZERO_PINS = {
    1: "VBUS",          # 5V = USB VBUS (measured 5.02 V: no diode on the module)
    2: "GND",
    3: "+3V3",          # on-board LDO output (OLED supply)
    4: "ENC_B",         # GP29
    5: "ENC_A",         # GP28
    6: "SCL",           # GP27 I2C1 SCL
    7: "SDA",           # GP26 I2C1 SDA
    8: "EXP_GP15",      # GP15 (expansion header)
    9: "ENC_SW",        # GP14
    10: "ROW6",         # GP13 function keys
    11: "ROW5",         # GP12
    12: "ROW4",         # GP11
    13: "ROW3",         # GP10
    14: "ROW2",         # GP9
    15: "ROW1",         # GP8
    16: "ROW0",         # GP7
    17: "COL5",         # GP6
    18: "COL4",         # GP5
    19: "COL3",         # GP4
    20: "COL2",         # GP3
    21: "COL1",         # GP2
    22: "COL0",         # GP1
    23: "LED_DIN_3V3",  # GP0  LED data, through the 74AHCT125
}

# Firmware-facing GPIO map (documentation and future firmware config)
GPIO = {
    "COLS": [1, 2, 3, 4, 5, 6],
    "ROWS": [7, 8, 9, 10, 11, 12, 13],
    "LED_DATA": 0,
    "I2C_SDA": 26,
    "I2C_SCL": 27,
    "ENC_A": 28,
    "ENC_B": 29,
    "ENC_SW": 14,
}
