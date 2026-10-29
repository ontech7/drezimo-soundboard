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


# Raspberry Pi Pico physical pin -> net (None = not connected).
# GPIO numbers: matrix GP6..GP18, LED data GP2, I2C0 GP4/GP5, encoder GP26..GP28.
PICO_PINS = {
    1: "EXP_GP0",       # GP0  (expansion header)
    2: "EXP_GP1",       # GP1  (expansion header)
    3: "GND",
    4: "LED_DIN_3V3",   # GP2  LED data, through the 74AHCT125
    5: None,            # GP3
    6: "SDA",           # GP4  I2C0 SDA
    7: "SCL",           # GP5  I2C0 SCL
    8: "GND",
    9: "COL0",          # GP6
    10: "COL1",         # GP7
    11: "COL2",         # GP8
    12: "COL3",         # GP9
    13: "GND",
    14: "COL4",         # GP10
    15: "COL5",         # GP11
    16: "ROW0",         # GP12
    17: "ROW1",         # GP13
    18: "GND",
    19: "ROW2",         # GP14
    20: "ROW3",         # GP15
    21: "ROW4",         # GP16
    22: "ROW5",         # GP17
    23: "GND",
    24: "ROW6",         # GP18 function keys
    25: None,           # GP19
    26: None,           # GP20
    27: None,           # GP21
    28: "GND",
    29: None,           # GP22
    30: None,           # RUN
    31: "ENC_A",        # GP26
    32: "ENC_B",        # GP27
    33: None,           # AGND (ADC not used; already a power output in the symbol)
    34: "ENC_SW",       # GP28
    35: None,           # ADC_VREF
    36: "+3V3",         # 3V3 out (OLED supply)
    37: None,           # 3V3_EN
    38: "GND",
    39: None,           # VSYS (Pico is powered from VBUS internally)
    40: "VBUS",         # USB 5 V
}

# Firmware-facing GPIO map (documentation and future firmware config)
GPIO = {
    "COLS": [6, 7, 8, 9, 10, 11],
    "ROWS": [12, 13, 14, 15, 16, 17, 18],
    "LED_DATA": 2,
    "I2C_SDA": 4,
    "I2C_SCL": 5,
    "ENC_A": 26,
    "ENC_B": 27,
    "ENC_SW": 28,
}
