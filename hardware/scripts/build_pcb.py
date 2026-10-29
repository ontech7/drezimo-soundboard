"""Builds the PCB placement (KiCad 10) from the schematic netlist and the case geometry.

Run with KiCad's bundled Python (it provides the pcbnew module):
    /Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3 build_pcb.py

Coordinates are millimetres in the case frame: origin at the case's top-left outer corner,
x to the right, y towards the user. Routing is not done here.
"""

import json
import subprocess
import sys
from pathlib import Path

import pcbnew

sys.path.insert(0, str(Path(__file__).resolve().parent))
from design import COLS, FN_KEYS, FN_ROW, KEYS, ROWS  # noqa: E402
from sexpr import find, first, parse  # noqa: E402

KICAD_CLI = "/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli"
FOOTPRINTS = Path("/Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints")
PCB_DIR = Path(__file__).resolve().parent.parent / "pcb"
BOARD_FILE = PCB_DIR / "drezimo-soundboard.kicad_pcb"
PROJECT_LIBS = {"Drezimo": PCB_DIR.parent / "lib" / "drezimo.pretty"}   # project footprints (logo)

BOARD_VERSION = "v0.1"            # silkscreen, next to the logo

# ---- Case geometry (must match simulator/dummy-board.html and the enclosure model)
PITCH = 19.05
MARGIN = 8.0
TOP_H = 38.0                      # top strip: Pico, OLED module, encoder
GAP = 4.0
LABEL_H = 5.0                     # engraved function-key labels
CASE_W = MARGIN * 2 + PITCH * COLS
Y_FN = MARGIN + TOP_H + GAP
Y_GRID = Y_FN + PITCH + LABEL_H
CASE_H = Y_GRID + PITCH * ROWS + MARGIN
WALL = 2.5                        # case wall thickness
PCB_INSET = WALL + 0.5            # PCB edge to case outer edge
CORNER_R = 2.0

# Akko V5 LED window: opposite the metal pins, ~5.08 mm south of the centre (MX standard).
# TO BE CONFIRMED with a caliper on the real switch before ordering.
LED_OFFSET_Y = 5.08


def key_center(n):
    row, col = KEYS[n]
    x = MARGIN + PITCH * col + PITCH / 2
    y = (Y_FN if row == FN_ROW else Y_GRID + PITCH * row) + PITCH / 2
    return x, y


NOT_FITTED = {"J2"}                # expansion header, DNP in the schematic

# Top strip layout
PICO_CENTER = (PCB_INSET + 1.8 + 25.5, MARGIN + 14.5)   # horizontal, USB facing the left wall
OLED_HEADER_CENTER = (77.7, MARGIN + 2.5)              # module header row (pins on the module's top edge)
ENC_SHAFT = (key_center(42)[0], MARGIN + TOP_H / 2)   # knob above the last column
U2_PIN1 = (PCB_INSET + 5.0, MARGIN + 37.0)             # DIP-14 lying horizontally under the Pico
MOUNT_INSET = 6.5                                       # hole centre to case outer edge
LOGO_CENTER = (OLED_HEADER_CENTER[0], MARGIN + 20.0)   # bottom side, behind the OLED module


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
    shape(board, L, pcbnew.SHAPE_T_CIRCLE, ENC_SHAFT, (ENC_SHAFT[0] + 7.5, ENC_SHAFT[1]), 0.1)  # ~15 mm knob
    # 1.3" SH1106 module, typical listing dimensions (TO BE MEASURED): PCB 35.4 x 33.5, header 1.5 mm
    # from the top edge, active area 29.42 x 14.70 centred horizontally on the glass.
    ox, oy = OLED_HEADER_CENTER[0], OLED_HEADER_CENTER[1] - 1.5
    shape(board, L, rect, (ox - 17.7, oy), (ox + 17.7, oy + 33.5), 0.15)
    shape(board, L, rect, (ox - 14.71, oy + 9.5), (ox + 14.71, oy + 9.5 + 14.70), 0.1)
    text(board, L, "OLED module (estimate)", ox, oy + 29.0, 1.5)
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
# custom rules cannot go below, so the edge floor is the LED cut-out case (0.2 mm, JLCPCB's
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
        fp.SetPath(pcbnew.KIID_PATH("/" + part["uuid"]))
        if ref in NOT_FITTED:
            fp.SetDNP(True)
            fp.SetExcludedFromPosFiles(True)
        board.Add(fp)
        for pad in fp.Pads():
            net = pin_net.get((ref, pad.GetNumber()))
            if net:
                pad.SetNet(nets[net])
        footprints[ref] = fp

    # --- Switches (top), LEDs + diodes + caps (bottom, assembled by the fab)
    for n in KEYS:
        cx, cy = key_center(n)
        place(footprints[f"SW{n}"], cx + 2.54, cy - 5.08)          # footprint origin is pin 1
        place(footprints[f"LED{n}"], cx, cy + LED_OFFSET_Y, bottom=True)
        place(footprints[f"D{n}"], cx + 6.9, cy + 3.6, angle=90, bottom=True)
        place(footprints[f"C{n}"], cx - 6.9, cy + 3.6, angle=90, bottom=True)

    # --- Top strip
    place(footprints["U1"], *PICO_CENTER, angle=90)                  # USB towards -x
    place(footprints["U2"], *U2_PIN1, angle=90)                      # pins 1-7 on the row nearer the keys
    place(footprints["J1"], OLED_HEADER_CENTER[0] - 3.81, OLED_HEADER_CENTER[1], angle=90)  # pin 1 (GND) on the left
    place(footprints["ENC1"], ENC_SHAFT[0] - 7.5, ENC_SHAFT[1] - 2.5)  # footprint origin is pin A
    place(footprints["J2"], U2_PIN1[0] + 21.0, U2_PIN1[1] - 3.81, angle=90)
    # Bottom side: fuse and bulk cap between the Pico's pin rows at the USB end (VBUS is pin 40),
    # U2 decoupling between its pin rows, R1 next to the 1Y output (pin 3)
    place(footprints["F1"], PCB_INSET + 11.0, PICO_CENTER[1] - 4.0, angle=0, bottom=True)
    place(footprints["C43"], PCB_INSET + 11.0, PICO_CENTER[1] + 0.5, angle=0, bottom=True)
    place(footprints["C44"], U2_PIN1[0] + 2.54, U2_PIN1[1] - 3.81, angle=0, bottom=True)
    place(footprints["R1"], U2_PIN1[0] + 5.08, U2_PIN1[1] + 3.6, angle=0, bottom=True)

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
    settings = board.GetDesignSettings()
    settings.SetAuxOrigin(vec(0, 0))
    settings.SetGridOrigin(vec(0, 0))
    board.Save(str(BOARD_FILE))
    # PAGE_INFO is not usable from the Python bindings: set A4 portrait in the file itself
    pcb_text = BOARD_FILE.read_text(encoding="utf-8")
    assert '(paper "A4")' in pcb_text
    BOARD_FILE.write_text(pcb_text.replace('(paper "A4")', '(paper "A4" portrait)', 1), encoding="utf-8")

    # Report what matters for the paper test
    usb = [p for p in footprints["U1"].Pads() if p.GetAttribute() == pcbnew.PAD_ATTRIB_NPTH]
    usb_x = min(pcbnew.ToMM(p.GetPosition().x) for p in usb) - ORIGIN[0]
    print(f"saved {BOARD_FILE}")
    print(f"case {CASE_W:.2f} x {CASE_H:.2f} mm, PCB {CASE_W - 2 * PCB_INSET:.2f} x {CASE_H - 2 * PCB_INSET:.2f} mm")
    print(f"Pico USB-side holes at x = {usb_x:.2f} mm (left PCB edge at {PCB_INSET} mm)")


if __name__ == "__main__":
    main()
