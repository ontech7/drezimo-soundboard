"""Builds the PCB placement (KiCad 10) from the schematic netlist and the case geometry.

Run with KiCad's bundled Python (it provides the pcbnew module):
    /Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3 build_pcb.py

Coordinates are millimetres in the case frame: origin at the case's top-left outer corner,
x to the right, y towards the user. Placement, routing and the GND pours are all generated here.
"""

import json
import subprocess
import sys
from pathlib import Path

import pcbnew

sys.path.insert(0, str(Path(__file__).resolve().parent))
from design import CHAIN_ORDER, COLS, FN_KEYS, FN_ROW, KEYS, LED_CHAIN_NETS, ROWS, ZERO_PINS  # noqa: E402
from sexpr import find, first, parse  # noqa: E402

KICAD_CLI = "/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli"
FOOTPRINTS = Path("/Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints")
PCB_DIR = Path(__file__).resolve().parent.parent / "pcb"
BOARD_FILE = PCB_DIR / "drezimo-soundboard.kicad_pcb"
PROJECT_LIBS = {"Drezimo": PCB_DIR.parent / "lib" / "drezimo.pretty"}   # project footprints (RP2040-Zero, logo)

BOARD_VERSION = "v0.2"            # silkscreen, next to the logo

# ---- Case geometry (must match simulator/dummy-board.html and the enclosure model)
PITCH = 19.05
MARGIN = 8.0
TOP_H = 38.0                      # top strip: RP2040-Zero (underneath), OLED module, encoder
GAP = 4.0
LABEL_H = 5.0                     # engraved function-key labels
CASE_W = MARGIN * 2 + PITCH * COLS
Y_FN = MARGIN + TOP_H + GAP
Y_GRID = Y_FN + PITCH + LABEL_H
CASE_H = Y_GRID + PITCH * ROWS + MARGIN
WALL = 2.5                        # case wall thickness
PCB_INSET = WALL + 0.5            # PCB edge to case outer edge
CORNER_R = 5.0                    # lets the case use up to ~8 mm outer corners

# Akko V5 LED window: opposite the metal pins, 6.00 x 2.04 mm, its centre 4.47 mm south of the
# switch centre (caliper: post 3.81 mm, post-to-window gap 1.54 mm). Not the 5.08 mm MX default.
LED_OFFSET_Y = 4.47


def key_center(n):
    row, col = KEYS[n]
    x = MARGIN + PITCH * col + PITCH / 2
    y = (Y_FN if row == FN_ROW else Y_GRID + PITCH * row) + PITCH / 2
    return x, y


# Top strip layout
# RP2040-Zero: on the bottom side on 2.54 mm headers (2.5 mm spacer), USB-C face down, chip side up.
# Its USB edge sits 0.5 mm inside the PCB edge so the silkscreen stays on the board; the receptacle
# overhangs the module by 1.23 mm, so its face ends up inside the case wall, where the plug opening is.
ZERO_W, ZERO_H, ZERO_USB_OVERHANG = 17.90, 23.30, 1.23
ZERO_CENTER = (30.0, PCB_INSET + 0.5 + ZERO_H / 2)
OLED_HEADER_CENTER = (77.7, MARGIN + 2.5)              # module header row (pins on the module's top edge)
ENC_SHAFT = (key_center(42)[0], MARGIN + TOP_H / 2)   # knob above the last column
U2_PIN1 = (10.0, 29.0)                                 # DIP-14 upright below-left of the module, right of the ROW bus
MOUNT_INSET = 6.5                                       # hole centre to case outer edge
LOGO_CENTER = (OLED_HEADER_CENTER[0], MARGIN + 20.0)   # bottom side, behind the OLED module
TRUNK_X = 39.8                                          # VBUS / +5V_LED trunk (bottom), right of the module


# Per-key parts around the switch centre (bottom side). The switch pins sit north of the centre,
# the LED window south: diode west of pin 2, cap east next to the LED's VDD pad.
def DIODE_AT(cx, cy):
    return cx - 7.3, cy + 1.4


def CAP_AT(cx, cy):
    return cx + 5.3, cy + 5.6


def mm(v):
    return pcbnew.FromMM(v)


# The case frame sits centred on an A4 portrait sheet so the 1:1 printout has no clipped edges.
# KiCad's grid and drill/place origin are set to the case corner, so coordinates read in the case frame.
PAGE_W, PAGE_H = 210.0, 297.0
ORIGIN = (round((PAGE_W - CASE_W) / 2, 2), round((PAGE_H - CASE_H) / 2, 2))


def vec(x, y):
    return pcbnew.VECTOR2I(mm(ORIGIN[0] + x), mm(ORIGIN[1] + y))


def load_fp(fpid):
    lib, name = fpid.split(":")
    fp = pcbnew.FootprintLoad(str(PROJECT_LIBS.get(lib, FOOTPRINTS / f"{lib}.pretty")), name)
    if fp is None:
        raise RuntimeError(f"footprint not found: {fpid}")
    fp.SetFPID(pcbnew.LIB_ID(lib, name))
    return fp


def place(fp, x, y, angle=0.0, bottom=False):
    fp.SetPosition(vec(x, y))
    if bottom:
        fp.Flip(fp.GetPosition(), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)
    fp.SetOrientationDegrees(angle)


def edge_outline(board):
    x0, y0 = PCB_INSET, PCB_INSET
    x1, y1 = CASE_W - PCB_INSET, CASE_H - PCB_INSET
    r = CORNER_R
    segs = [((x0 + r, y0), (x1 - r, y0)), ((x1, y0 + r), (x1, y1 - r)),
            ((x1 - r, y1), (x0 + r, y1)), ((x0, y1 - r), (x0, y0 + r))]
    for a, b in segs:
        s = pcbnew.PCB_SHAPE(board)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetStart(vec(*a))
        s.SetEnd(vec(*b))
        s.SetLayer(pcbnew.Edge_Cuts)
        s.SetWidth(mm(0.1))
        board.Add(s)
    for cx, cy, start in ((x0 + r, y0 + r, (x0, y0 + r)), (x1 - r, y0 + r, (x1 - r, y0)),
                          (x1 - r, y1 - r, (x1, y1 - r)), (x0 + r, y1 - r, (x0 + r, y1))):
        a = pcbnew.PCB_SHAPE(board)
        a.SetShape(pcbnew.SHAPE_T_ARC)
        a.SetCenter(vec(cx, cy))
        a.SetStart(vec(*start))
        a.SetArcAngleAndEnd(pcbnew.EDA_ANGLE(90, pcbnew.DEGREES_T))
        a.SetLayer(pcbnew.Edge_Cuts)
        a.SetWidth(mm(0.1))
        board.Add(a)


def shape(board, layer, kind, a, b, width=0.15):
    s = pcbnew.PCB_SHAPE(board)
    s.SetShape(kind)
    s.SetStart(vec(*a))
    s.SetEnd(vec(*b))
    s.SetLayer(layer)
    s.SetWidth(mm(width))
    board.Add(s)


def text(board, layer, msg, x, y, size=2.0, mirrored=False):
    t = pcbnew.PCB_TEXT(board)
    t.SetText(msg)
    t.SetLayer(layer)
    t.SetPosition(vec(x, y))
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(size * 0.15))
    t.SetMirrored(mirrored)
    board.Add(t)


def reference_drawings(board):
    """User.Drawings: case outline, keycaps and a scale bar, for the 1:1 paper fit check."""
    L = pcbnew.Dwgs_User
    rect = pcbnew.SHAPE_T_RECTANGLE
    shape(board, L, rect, (0, 0), (CASE_W, CASE_H), 0.3)
    shape(board, L, rect, (WALL, WALL), (CASE_W - WALL, CASE_H - WALL), 0.1)
    for n in KEYS:
        cx, cy = key_center(n)
        shape(board, L, rect, (cx - 9, cy - 9), (cx + 9, cy + 9), 0.1)              # 18 mm keycap
    for i, label in enumerate(FN_KEYS):
        text(board, L, label, key_center(37 + i)[0], Y_GRID - LABEL_H / 2, 1.8)        # engraved labels
    shape(board, L, pcbnew.SHAPE_T_CIRCLE, ENC_SHAFT, (ENC_SHAFT[0] + 7.2, ENC_SHAFT[1]), 0.1)  # 14.4 mm knob
    # 1.3" SH1106 module as measured: PCB 35.54 x 33.59, pin 1 centre 13.92 mm from the left edge and
    # 1.69 mm from the top; glass 34.44 x 22.97 at 0.50 / 6.24 mm from the left / top edges; holes 3.2 mm,
    # top pair 2.64 / 3.03 mm from the side / top edges, bottom pair ~2.4 / ~2.0 mm (photo estimate);
    # pixel area 29.71 x 14.90, ~1.9 mm below the top of the glass (photo estimate), centred on it.
    ox, oy = OLED_HEADER_CENTER[0] - 3.81 - 13.92, OLED_HEADER_CENTER[1] - 1.69
    ow, oh = 35.54, 33.59
    shape(board, L, rect, (ox, oy), (ox + ow, oy + oh), 0.15)
    shape(board, L, rect, (ox + 0.50, oy + 6.24), (ox + 0.50 + 34.44, oy + 6.24 + 22.97), 0.1)
    gx = ox + 0.50 + 34.44 / 2
    shape(board, L, rect, (gx - 29.71 / 2, oy + 6.24 + 1.9), (gx + 29.71 / 2, oy + 6.24 + 1.9 + 14.90), 0.1)
    for hx, hy in ((2.64, 3.03), (ow - 2.64, 3.03), (2.4, oh - 2.0), (ow - 2.4, oh - 2.0)):
        shape(board, L, pcbnew.SHAPE_T_CIRCLE, (ox + hx, oy + hy), (ox + hx + 1.6, oy + hy), 0.1)
    text(board, L, "OLED module", gx, oy + 31.0, 1.5)
    # RP2040-Zero hangs under the board: its outline and the USB-C receptacle, seen from above
    zx, zy = ZERO_CENTER
    shape(board, L, rect, (zx - ZERO_W / 2, zy - ZERO_H / 2), (zx + ZERO_W / 2, zy + ZERO_H / 2), 0.1)
    shape(board, L, rect, (zx - 4.45, zy - ZERO_H / 2 - ZERO_USB_OVERHANG), (zx + 4.45, zy - ZERO_H / 2 + 5.2), 0.15)
    text(board, L, "RP2040-Zero (underneath)", zx, zy + 2.0, 1.2)
    # 100 mm scale bar under the case: measure it on the printout before trusting anything else
    y = CASE_H + 12
    shape(board, L, pcbnew.SHAPE_T_SEGMENT, (0, y), (100, y), 0.3)
    for k in range(0, 101, 10):
        shape(board, L, pcbnew.SHAPE_T_SEGMENT, (k, y - (2.5 if k % 50 == 0 else 1.2)), (k, y), 0.2)
    text(board, L, "100 mm - print at 100 % / actual size", 50, y + 4, 2.0)


def mounting_holes():
    """Corners and middle of the long sides, in the margin outside the switch cut-outs."""
    m = MOUNT_INSET
    mid = Y_GRID + PITCH * ROWS / 2              # between pad rows 3 and 4
    return [(m, m), (CASE_W - m, m), (m, CASE_H - m), (CASE_W - m, CASE_H - m), (m, mid), (CASE_W - m, mid)]


# JLCPCB standard 2-layer process, with margin. Board-level values are the floor that
# custom rules cannot go below, so the floor is the exception (LED cut-out edge 0.2 mm, JLCPCB's
# routed-edge minimum) and the .kicad_dru restores 0.5 mm for everything else.
DESIGN_RULES = {
    "min_clearance": 0.15,
    "min_track_width": 0.15,
    "min_via_diameter": 0.6,
    "min_through_hole_diameter": 0.3,
    "min_via_annular_width": 0.15,
    "min_hole_to_hole": 0.5,
    "min_hole_clearance": 0.25,
    "min_copper_edge_clearance": 0.2,
}
CUSTOM_RULES = """(version 1)

# The stock SK6812MINI-E reverse-mount footprint puts its pads 0.25-0.35 mm from the LED's
# own window cut-out (by design: the leads sit right at the body). 0.5 mm everywhere else.
(rule "Board edge"
  (constraint edge_clearance (min 0.5mm))
  (condition "!(A.Parent.Reference == 'LED*' || B.Parent.Reference == 'LED*')"))
"""


def schematic_netlist():
    """(ref, pin) -> net name, from the netlist kicad-cli exports for the current schematic."""
    out = PCB_DIR / "netlist.net"
    subprocess.run([KICAD_CLI, "sch", "export", "netlist", "--format", "kicadsexpr", "-o", str(out),
                    str(PCB_DIR / "drezimo-soundboard.kicad_sch")], check=True, capture_output=True)
    nets_node = first(parse(out.read_text(encoding="utf-8")), "nets")
    assert nets_node is not None
    pin_net = {}
    for n in find(nets_node, "net"):
        name = first(n, "name")[1]
        for node in find(n, "node"):
            pin_net[(first(node, "ref")[1], str(first(node, "pin")[1]))] = name
    return pin_net


def write_rules():
    pro_file = PCB_DIR / "drezimo-soundboard.kicad_pro"
    pro = json.loads(pro_file.read_text(encoding="utf-8"))
    pro.setdefault("board", {}).setdefault("design_settings", {}).setdefault("rules", {}).update(DESIGN_RULES)
    pro_file.write_text(json.dumps(pro, indent=2), encoding="utf-8")
    (PCB_DIR / "drezimo-soundboard.kicad_dru").write_text(CUSTOM_RULES, encoding="utf-8")


# ---------------------------------------------------------------- routing
# Two layers. F.Cu: the COL and +5V_LED columns, the top-strip signals and a GND pour.
# B.Cu: every part, the ROW trunks and bus, the LED data chain and a second GND pour, stitched to
# the first by the two GND vias of every key. Each key repeats the same pattern (offsets below).
F_CU, B_CU = pcbnew.F_Cu, pcbnew.B_Cu
W_SIG, W_STUB, W_PWR, W_BUS, W_ROWBUS = 0.25, 0.4, 0.5, 0.8, 0.2
VIA_D, VIA_DRILL = 0.6, 0.3

COL_X = 7.9           # COL column (F.Cu), its via right of switch pin 1
V5_X, V5_Y = 8.9, 6.9  # +5V_LED column (F.Cu, clears hole H6) and the per-key via under the LED
ROW_Y = 8.0           # ROW trunk (B.Cu), below the LEDs
RET_Y = 10.5          # LED chain going back from the end of a row (B.Cu), below the ROW trunk
HOP_Y = 3.72          # LED chain hops over the next key's diode on F.Cu, at the level of DOUT
HOP_OUT, HOP_IN = 10.3, -5.05   # hop vias: right of a key / left of the next key's LED
RET_X = 9.4           # the chain comes down the left side on F.Cu to start the next row
V5_BUS_Y = 50.0       # +5V_LED bus (B.Cu) above the function row, feeding the columns
U2_FEED_X = 19.5      # +5V_LED up to U2, right of its pins
ROW_BUS = {5: 3.7, 4: 4.1, 3: 4.5, 2: 4.9, 1: 5.3, 0: 5.7, 6: 6.1}   # left-edge bus, deepest row outermost
# Where each ROW line leaves the bus (y) and turns towards its module pin (x): no crossings
ROW_FAN = {5: (10.5, 12.0), 4: (11.0, 12.6), 3: (11.5, 13.2), 2: (12.0, 13.8), 1: (12.5, 13.2), 0: (13.0, 12.6), 6: (13.5, 12.0)}
COL_TURN = {1: 39.0, 2: 38.0, 3: 37.0, 4: 36.0, 5: 35.0}   # COL1..5 run west under the OLED, COL5 highest
ZERO_PAD = {net: pad for pad, net in ZERO_PINS.items() if net}


class Router:
    def __init__(self, board, nets, footprints):
        self.board, self.nets, self.fps = board, nets, footprints

    def net(self, name):
        return self.nets.get("/" + name) or self.nets[name]

    def pad(self, ref, num):
        p = self.fps[ref].FindPadByNumber(str(num)).GetPosition()
        return pcbnew.ToMM(p.x) - ORIGIN[0], pcbnew.ToMM(p.y) - ORIGIN[1]

    def track(self, name, pts, layer=B_CU, width=W_SIG):
        for a, b in zip(pts, pts[1:]):
            t = pcbnew.PCB_TRACK(self.board)
            t.SetStart(vec(*a))
            t.SetEnd(vec(*b))
            t.SetWidth(mm(width))
            t.SetLayer(layer)
            t.SetNet(self.net(name))
            self.board.Add(t)

    def via(self, name, x, y):
        v = pcbnew.PCB_VIA(self.board)
        v.SetViaType(pcbnew.VIATYPE_THROUGH)
        v.SetLayerPair(F_CU, B_CU)
        v.SetPosition(vec(x, y))
        v.SetWidth(mm(VIA_D))
        v.SetDrill(mm(VIA_DRILL))
        v.SetNet(self.net(name))
        self.board.Add(v)


def row_center_y(row):
    return (Y_FN if row == FN_ROW else Y_GRID + PITCH * row) + PITCH / 2


def col_center_x(col):
    return MARGIN + PITCH * col + PITCH / 2


def route_keys(r):
    """Per key: switch -> COL via, switch -> diode -> ROW trunk, LED and cap to the GND vias and +5V via."""
    for n, (row, col) in KEYS.items():
        cx, cy = key_center(n)
        r.track(f"COL{col}", [r.pad(f"SW{n}", 1), (cx + COL_X, cy - 5.08)])
        r.via(f"COL{col}", cx + COL_X, cy - 5.08)
        ax, ay = r.pad(f"D{n}", 2)
        kx, ky = r.pad(f"D{n}", 1)
        r.track(f"K{n}", [r.pad(f"SW{n}", 2), (ax, cy - 2.54), (ax, ay)])   # pin 2 west, then down to the anode
        r.track(f"ROW{row}", [(kx, ky), (kx, cy + ROW_Y)])
        gx, gy = r.pad(f"LED{n}", 1)
        r.track("GND", [(gx, gy), (gx, cy + 2.35)])
        r.via("GND", gx, cy + 2.35)
        vx, vy = r.pad(f"LED{n}", 3)
        c1, c2 = r.pad(f"C{n}", 1), r.pad(f"C{n}", 2)
        r.track("+5V_LED", [(vx, vy), (vx + 0.9, c1[1]), c1], width=W_STUB)
        r.track("+5V_LED", [(vx, vy), (vx, cy + V5_Y), (cx + V5_X, cy + V5_Y)], width=W_STUB)
        r.via("+5V_LED", cx + V5_X, cy + V5_Y)
        r.track("GND", [c2, (cx + 7.0, c2[1])])
        r.via("GND", cx + 7.0, c2[1])
    rows = [FN_ROW] + list(range(ROWS))
    for col in range(COLS):
        x = col_center_x(col)
        r.track(f"COL{col}", [(x + COL_X, row_center_y(rw) - 5.08) for rw in rows], layer=F_CU)
        r.via("+5V_LED", x + V5_X, V5_BUS_Y)
        r.track("+5V_LED", [(x + V5_X, V5_BUS_Y), (x + V5_X, row_center_y(ROWS - 1) + V5_Y)], layer=F_CU, width=W_PWR)


def route_chain(r):
    """LED data, every row left to right: DOUT -> F.Cu hop over the next diode -> DIN; at the end of
    a row: down on F.Cu, back west on B.Cu below the ROW trunk, down the left side, into the next row."""
    for i, n in enumerate(CHAIN_ORDER):
        row, _ = KEYS[n]
        cx, cy = key_center(n)
        din = LED_CHAIN_NETS[i]
        dx, dy = r.pad(f"LED{n}", 2)
        r.via(din, cx + HOP_IN, cy + HOP_Y)
        r.track(din, [(cx + HOP_IN, cy + HOP_Y), (cx + HOP_IN, dy), (dx, dy)])
        if i + 1 == len(CHAIN_ORDER):
            break                                                       # last DOUT is not connected
        nxt = CHAIN_ORDER[i + 1]
        ncx, ncy = key_center(nxt)
        dout = LED_CHAIN_NETS[i + 1]
        ox, oy = r.pad(f"LED{n}", 4)
        if KEYS[nxt][0] == row:
            r.track(dout, [(ox, oy), (cx + HOP_OUT, oy)])
            r.via(dout, cx + HOP_OUT, oy)
            r.track(dout, [(cx + HOP_OUT, oy), (ncx + HOP_IN, ncy + HOP_Y)], layer=F_CU)
        else:
            x1 = cx + 4.2
            r.track(dout, [(ox, oy), (x1, oy)])
            r.via(dout, x1, oy)
            r.track(dout, [(x1, oy), (x1, cy + RET_Y)], layer=F_CU)
            r.via(dout, x1, cy + RET_Y)
            r.track(dout, [(x1, cy + RET_Y), (RET_X, cy + RET_Y)])
            r.via(dout, RET_X, cy + RET_Y)
            r.track(dout, [(RET_X, cy + RET_Y), (RET_X, ncy + HOP_Y), (ncx + HOP_IN, ncy + HOP_Y)], layer=F_CU)


def route_rows(r):
    """ROW trunks along each row, up the left edge (B.Cu), fanned into the module's left column."""
    hole_y = mounting_holes()[4][1]
    for row in range(ROWS + 1):
        name, y = f"ROW{row}", row_center_y(row) + ROW_Y
        x_end = col_center_x(COLS - 1) - 7.3                           # last diode's cathode stub
        xb, (level, fx) = ROW_BUS[row], ROW_FAN[row]
        px, py = r.pad("U1", ZERO_PAD[name])
        trunk = [(x_end, y), (xb, y)]
        if abs(y - hole_y) < 4.0:                                       # dodge the middle mounting hole
            trunk = [(x_end, y), (9.0, y), (9.0, hole_y - 3.8), (xb, hole_y - 3.8)]
        r.track(name, trunk + [(xb, level), (fx, level), (fx, py), (px, py)], width=W_ROWBUS)


def route_top(r):
    """Module, U2, power entry, OLED, encoder and the COL fan-in under the OLED."""
    # COL0..5 come up from the function row and run west to the module's 5-pin row
    for col in range(COLS):
        name, x, y0 = f"COL{col}", col_center_x(col) + COL_X, row_center_y(FN_ROW) - 5.08
        px, py = r.pad("U1", ZERO_PAD[name])
        pts = [(x, y0), (x, py + 1.3), (px, py)] if col == 0 else [(x, y0), (x, COL_TURN[col]), (px, COL_TURN[col]), (px, py)]
        r.track(name, pts, layer=F_CU)
    # Power: 5V pin -> trunk down the right of the module (F1, C43) -> bus above the function row
    p5 = r.pad("U1", ZERO_PAD["VBUS"])
    f1a, f1b = r.pad("F1", 1), r.pad("F1", 2)
    r.track("VBUS", [p5, (TRUNK_X, p5[1]), f1a], width=W_BUS)
    r.track("+5V_LED", [f1b, (TRUNK_X, V5_BUS_Y)], width=W_BUS)
    r.track("+5V_LED", [(U2_FEED_X, V5_BUS_Y), (col_center_x(COLS - 1) + V5_X, V5_BUS_Y)], width=W_BUS)
    # U2: +5V to VCC (14) and the spare OE pins (1, 4, 13), decoupling, gate 3 in and out
    p1, p4, p13, p14 = (r.pad("U2", k) for k in (1, 4, 13, 14))
    feed_y = p14[1] - 1.4
    r.track("+5V_LED", [(U2_FEED_X, V5_BUS_Y), (U2_FEED_X, feed_y), (p1[0], feed_y), p1], width=W_STUB)
    r.track("+5V_LED", [(U2_FEED_X, p14[1]), p14], width=W_STUB)
    r.track("+5V_LED", [(U2_FEED_X, p13[1]), p13], width=W_STUB)
    c44 = r.pad("C44", 1)
    r.track("+5V_LED", [(c44[0], feed_y), c44], width=W_STUB)
    r.track("+5V_LED", [p1, (p1[0] - 1.4, p1[1] + 1.0), (p1[0] - 1.4, p4[1] - 0.6), p4], width=W_STUB)
    g8, a9 = r.pad("U1", ZERO_PAD["LED_DIN_3V3"]), r.pad("U2", 9)
    r.track("LED_DIN_3V3", [g8, (20.5, g8[1] + 1.88), (20.5, a9[1]), a9], layer=F_CU)
    r.track("LED_DIN_5V", [r.pad("U2", 8), r.pad("R1", 1)])
    # Start of the LED chain: R1 -> via -> F.Cu down the left of the first function key -> its DIN hop via
    d0, (rx, ry) = LED_CHAIN_NETS[0], r.pad("R1", 2)
    cx0, cy0 = key_center(CHAIN_ORDER[0])
    r.track(d0, [(rx, ry), (rx, ry + 1.0)])
    r.via(d0, rx, ry + 1.0)
    r.track(d0, [(rx, ry + 1.0), (10.5, ry + 1.0), (10.5, cy0 + HOP_Y), (cx0 + HOP_IN, cy0 + HOP_Y)], layer=F_CU)
    # OLED (VCC-GND-SCL-SDA) and encoder, straight out of the module's right column on F.Cu
    j1 = {k: r.pad("J1", k) for k in (1, 3, 4)}
    p3, scl, sda = r.pad("U1", ZERO_PAD["+3V3"]), r.pad("U1", ZERO_PAD["SCL"]), r.pad("U1", ZERO_PAD["SDA"])
    r.track("+3V3", [p3, (j1[1][0] - 0.5, p3[1]), j1[1]], layer=F_CU, width=W_STUB)
    r.track("SCL", [scl, (j1[3][0] - 1.0, scl[1]), (j1[3][0], scl[1] - 1.0), j1[3]], layer=F_CU)
    r.track("SDA", [sda, (j1[4][0], sda[1]), j1[4]], layer=F_CU)
    enc = {k: r.pad("ENC1", k) for k in ("A", "B", "S1")}
    sw, ea, eb = (r.pad("U1", ZERO_PAD[k]) for k in ("ENC_SW", "ENC_A", "ENC_B"))
    sx = enc["S1"][0] + 2.5                                             # round the right of the encoder
    r.track("ENC_SW", [sw, (sx, sw[1]), (sx, enc["S1"][1]), enc["S1"]], layer=F_CU)
    r.track("ENC_A", [ea, (enc["A"][0] - 1.8, ea[1]), (enc["A"][0] - 1.8, enc["A"][1]), enc["A"]], layer=F_CU)
    r.track("ENC_B", [eb, (enc["B"][0] - 2.8, eb[1]), (enc["B"][0] - 2.8, enc["B"][1]), enc["B"]], layer=F_CU)


def stitching_vias():
    """Extra GND vias where the pours on the two layers would otherwise meet rarely."""
    pts = [(x, 6.0) for x in (50, 60, 90, 100, 112)]                         # top edge, right of the module
    pts += [(x, 34.0) for x in (45, 50, 55, 60, 65, 70, 85, 90, 95, 100)]   # under the OLED, above COL5
    pts += [(x, 44.0) for x in (30, 50, 70, 90, 110)]                       # between the COL runs and the bus
    pts += [(col_center_x(c) - 2.0, V5_BUS_Y + 1.6) for c in range(COLS)]   # between the bus and the keys
    pts += [(x, CASE_H - PCB_INSET - 2.8) for x in (20, 40, 60, 80, 100)]  # bottom edge
    pts += [(CASE_W - PCB_INSET - 2.0, y) for y in (60, 80, 100, 120, 145, 160, 180)]   # right edge
    return pts


def add_gnd_pours(r):
    for layer in (F_CU, B_CU):
        z = pcbnew.ZONE(r.board)
        z.SetLayer(layer)
        z.SetNet(r.net("GND"))
        outline = z.Outline()
        outline.NewOutline()
        for x, y in ((0, 0), (CASE_W, 0), (CASE_W, CASE_H), (0, CASE_H)):   # clipped to the board edge
            p = vec(x, y)
            outline.Append(p.x, p.y)
        z.SetLocalClearance(mm(0.25))
        z.SetMinThickness(mm(0.25))
        z.SetThermalReliefGap(mm(0.4))
        z.SetThermalReliefSpokeWidth(mm(0.4))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
        z.SetIslandRemovalMode(pcbnew.ISLAND_REMOVAL_MODE_ALWAYS)
        r.board.Add(z)
    for x, y in stitching_vias():
        r.via("GND", x, y)
    r.board.BuildConnectivity()                     # island removal needs to know what each fill touches
    pcbnew.ZONE_FILLER(r.board).Fill(r.board.Zones())


def route(board, nets, footprints):
    r = Router(board, nets, footprints)
    route_keys(r)
    route_chain(r)
    route_rows(r)
    route_top(r)
    add_gnd_pours(r)


def main():
    write_rules()
    parts = json.loads((PCB_DIR / "intended_netlist.json").read_text(encoding="utf-8"))
    board = pcbnew.BOARD()
    board.SetCopperLayerCount(2)

    # Net names exactly as KiCad derives them from the schematic ("/GND", "unconnected-(...)"),
    # so DRC's schematic parity check compares like with like.
    pin_net = schematic_netlist()
    for ref, part in parts.items():
        for pin, net in part["pins"].items():
            actual = pin_net.get((ref, pin))
            if net and (actual or "").lstrip("/") != net:
                raise RuntimeError(f"{ref}.{pin}: schematic has {actual}, design says {net}")
    nets = {}
    for net in sorted(set(pin_net.values())):
        nets[net] = pcbnew.NETINFO_ITEM(board, net)
        board.Add(nets[net])

    footprints = {}
    for ref, part in sorted(parts.items()):
        fp = load_fp(part["footprint"])
        fp.SetReference(ref)
        fp.SetValue(part["value"])
        for name, value in part["fields"].items():          # e.g. LCSC, read by export_fab.py
            fp.SetField(name, value)
            field = fp.GetField(name)
            field.SetVisible(False)                          # new fields default to visible silkscreen
            field.SetLayer(pcbnew.F_Fab)
        fp.SetPath(pcbnew.KIID_PATH("/" + part["uuid"]))
        board.Add(fp)
        for pad in fp.Pads():
            net = pin_net.get((ref, pad.GetNumber()))
            if net:
                pad.SetNet(nets[net])
        footprints[ref] = fp

    # --- Switches (top, hand-soldered); LEDs, diodes and caps (bottom, assembled by the fab).
    # On the bottom, 90 deg puts pad 2 of a 2-pad part on top and 270 deg puts pad 1 on top.
    for n in KEYS:
        cx, cy = key_center(n)
        place(footprints[f"SW{n}"], cx, cy)                          # origin = switch centre
        place(footprints[f"LED{n}"], cx, cy + LED_OFFSET_Y, bottom=True)
        label = footprints[f"LED{n}"].Reference()                   # unlocked: it would read mirrored from below
        label.SetTextAngleDegrees(0)
        label.SetPosition(vec(cx, cy + LED_OFFSET_Y + 2.8))         # clear of the LED outline
        place(footprints[f"D{n}"], *DIODE_AT(cx, cy), angle=90, bottom=True)     # anode up, level with pin 2
        place(footprints[f"C{n}"], *CAP_AT(cx, cy), bottom=True)                 # +5V pad next to the LED's VDD
        for ref in (f"D{n}", f"C{n}"):                               # no room for their labels between the holes
            footprints[ref].Reference().SetVisible(False)

    # --- Top strip
    place(footprints["U1"], *ZERO_CENTER, angle=180, bottom=True)    # USB towards -y (back edge); 0 deg on the bottom points it at the keys
    place(footprints["U2"], *U2_PIN1)                                # gate 3 (pins 8-10) at the bottom right
    place(footprints["J1"], OLED_HEADER_CENTER[0] - 3.81, OLED_HEADER_CENTER[1], angle=90)  # pin 1 (VCC) on the left
    place(footprints["ENC1"], ENC_SHAFT[0] - 7.5, ENC_SHAFT[1] - 2.5)  # footprint origin is pin A
    # Bottom side: fuse and bulk cap on the 5 V trunk that runs down the right of the module,
    # U2 decoupling next to its VCC pin (14), R1 right under the 3Y output (pin 8)
    place(footprints["F1"], TRUNK_X, 30.0, angle=270, bottom=True)            # VBUS on top
    place(footprints["C43"], TRUNK_X + 1.475, 42.0, bottom=True)              # + pad on the trunk
    place(footprints["C44"], U2_PIN1[0] + 5.3, U2_PIN1[1] + 0.8, angle=270, bottom=True)
    place(footprints["R1"], U2_PIN1[0] + 7.62, U2_PIN1[1] + 17.75, angle=270, bottom=True)

    for i, (x, y) in enumerate(mounting_holes()):
        place(footprints[f"H{i + 1}"], x, y)

    # --- Logo and hardware version on the bottom silkscreen (board-only, not in the schematic)
    logo = load_fp("Drezimo:Drezimo_Logo")
    logo.SetReference("LOGO1")
    board.Add(logo)
    place(logo, *LOGO_CENTER, angle=180, bottom=True)   # 0 deg on the bottom reads upside down
    text(board, pcbnew.B_SilkS, BOARD_VERSION, LOGO_CENTER[0], LOGO_CENTER[1] + 8.0, size=1.5, mirrored=True)

    edge_outline(board)
    reference_drawings(board)
    route(board, nets, footprints)
    settings = board.GetDesignSettings()
    settings.SetAuxOrigin(vec(0, 0))
    settings.SetGridOrigin(vec(0, 0))
    board.Save(str(BOARD_FILE))
    # PAGE_INFO is not usable from the Python bindings: set A4 portrait in the file itself
    pcb_text = BOARD_FILE.read_text(encoding="utf-8")
    assert '(paper "A4")' in pcb_text
    BOARD_FILE.write_text(pcb_text.replace('(paper "A4")', '(paper "A4" portrait)', 1), encoding="utf-8")

    # Report what matters for the paper test and the enclosure
    pads = {p.GetNumber(): p.GetPosition() for p in footprints["U1"].Pads()}
    p1 = pads["1"]
    usb_face = ZERO_CENTER[1] - ZERO_H / 2 - ZERO_USB_OVERHANG
    print(f"saved {BOARD_FILE}")
    print(f"case {CASE_W:.2f} x {CASE_H:.2f} mm, PCB {CASE_W - 2 * PCB_INSET:.2f} x {CASE_H - 2 * PCB_INSET:.2f} mm")
    print(f"RP2040-Zero 5V pad at ({pcbnew.ToMM(p1.x) - ORIGIN[0]:.2f}, {pcbnew.ToMM(p1.y) - ORIGIN[1]:.2f}) mm, "
          f"USB-C centre x = {ZERO_CENTER[0]:.2f} mm, receptacle face y = {usb_face:.2f} mm (case outer edge y = 0)")


if __name__ == "__main__":
    main()
