"""Generates the soundboard schematic (KiCad 10) from a Python description.

Every connection is made with a short wire stub and a net label on the symbol pin,
so the netlist is defined here, in one readable place, and checked by ERC.

Usage (any Python 3.10+):
    python build_schematic.py
Output: hardware/pcb/drezimo-soundboard.kicad_sch (+ .kicad_pro if missing)
"""

import json
import math
import uuid
from pathlib import Path

from design import CHAIN_ORDER, FN_KEYS, KEYS, LED_CHAIN_NETS, ZERO_PINS, key_cell
from sexpr import Sym, dump, find, first, parse

KICAD_SYMBOLS = Path("/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols")
OUT_DIR = Path(__file__).resolve().parent.parent / "pcb"
PROJECT_SYMBOLS = {"Drezimo": OUT_DIR.parent / "lib" / "drezimo.kicad_sym"}   # project symbols (RP2040-Zero)
PROJECT = "drezimo-soundboard"
ROOT_UUID = str(uuid.uuid5(uuid.NAMESPACE_URL, "drezimo-soundboard/root"))
GRID = 1.27


def uid(*parts):
    """Deterministic UUIDs: regenerating the schematic keeps the PCB links stable."""
    return str(uuid.uuid5(uuid.NAMESPACE_URL, "drezimo-soundboard/" + "/".join(map(str, parts))))


def snap(v):
    return round(round(v / GRID) * GRID, 4)


# ---------------------------------------------------------------- library symbols
_lib_cache = {}


def lib_root(lib):
    if lib not in _lib_cache:
        path = PROJECT_SYMBOLS.get(lib, KICAD_SYMBOLS / f"{lib}.kicad_sym")
        _lib_cache[lib] = parse(path.read_text(encoding="utf-8"))
    return _lib_cache[lib]


def raw_symbol(lib, name):
    for s in find(lib_root(lib), "symbol"):
        if s[1] == name:
            return s
    raise KeyError(f"{lib}:{name}")


def flat_symbol(lib, name):
    """Returns the symbol with `extends` resolved, named "lib:name" as schematics expect."""
    s = raw_symbol(lib, name)
    ext = first(s, "extends")
    if not ext:
        out = [Sym("symbol"), f"{lib}:{name}"] + [x for x in s[2:]]
        return out
    parent = flat_symbol(lib, ext[1])
    child_props = {p[1]: p for p in find(s, "property")}
    out = [Sym("symbol"), f"{lib}:{name}"]
    for x in parent[2:]:
        if isinstance(x, list) and x and x[0] == "property" and x[1] in child_props:
            out.append(child_props.pop(x[1]))
        elif isinstance(x, list) and x and x[0] == "symbol":
            sub = list(x)
            sub[1] = name + sub[1][len(ext[1]):]  # Parent_1_1 -> Child_1_1
            out.append(sub)
        else:
            out.append(x)
    out[2:2] = list(child_props.values())
    return out


def symbol_pins(sym, unit):
    """Pins of a unit (plus unit 0, common to all units): number -> (x, y, angle) in library frame."""
    pins = {}
    name = sym[1].split(":", 1)[1]
    for sub in find(sym, "symbol"):
        u = int(sub[1][len(name) + 1:].split("_")[0])
        if u not in (0, unit):
            continue
        for p in find(sub, "pin"):
            at, number = first(p, "at"), first(p, "number")
            assert at is not None and number is not None, p
            pins[str(number[1])] = (float(at[1]), float(at[2]), float(at[3]) if len(at) > 3 else 0.0)
    return pins


# ---------------------------------------------------------------- schematic model
class Schematic:
    def __init__(self):
        self.lib_symbols = {}
        self.items = []
        self.parts = {}  # reference -> {"value", "footprint", "pins": {num: net}}

    def text(self, s, x, y, size=1.8):
        self.items.append([Sym("text"), s, [Sym("exclude_from_sim"), Sym("no")], [Sym("at"), x, y, 0],
                           [Sym("effects"), [Sym("font"), [Sym("size"), size, size]], [Sym("justify"), Sym("left"), Sym("top")]],
                           [Sym("uuid"), uid("text", s[:40], x, y)]])

    def place(self, lib, name, ref, value, footprint, x, y, angle=0, unit=1, dnp=False, fields=None):
        sym_id = f"{lib}:{name}"
        if sym_id not in self.lib_symbols:
            self.lib_symbols[sym_id] = flat_symbol(lib, name)
        x, y = snap(x), snap(y)
        sym = self.lib_symbols[sym_id]
        pins = symbol_pins(sym, unit)
        inst_uuid = uid("sym", ref, unit)
        props = [("Reference", ref, False), ("Value", value, False), ("Footprint", footprint, True), ("Datasheet", "~", True)]
        props += [(k, v, True) for k, v in (fields or {}).items()]
        node = [Sym("symbol"), [Sym("lib_id"), sym_id], [Sym("at"), x, y, angle], [Sym("unit"), unit],
                [Sym("exclude_from_sim"), Sym("no")], [Sym("in_bom"), Sym("no" if lib in ("power", "Mechanical") else "yes")],
                [Sym("on_board"), Sym("no" if lib == "power" else "yes")], [Sym("dnp"), Sym("yes" if dnp else "no")],
                [Sym("uuid"), inst_uuid]]
        for i, (k, v, hidden) in enumerate(props):
            eff = [Sym("effects"), [Sym("font"), [Sym("size"), 1.27, 1.27]]]
            if hidden:
                eff.append([Sym("hide"), Sym("yes")])
            node.append([Sym("property"), k, v, [Sym("at"), x, y - 3 - 2 * i if not hidden else y, 0], eff])
        for num in pins:
            node.append([Sym("pin"), num, [Sym("uuid"), uid("pin", ref, unit, num)]])
        node.append([Sym("instances"), [Sym("project"), PROJECT,
                     [Sym("path"), "/" + ROOT_UUID, [Sym("reference"), ref], [Sym("unit"), unit]]]])
        self.items.append(node)
        part = self.parts.setdefault(ref, {"value": value, "footprint": footprint, "lib": sym_id, "pins": {}, "uuid": uid("sym", ref, 1)})
        return Placed(self, ref, x, y, angle, pins, part)

    def wire(self, a, b):
        self.items.append([Sym("wire"), [Sym("pts"), [Sym("xy"), a[0], a[1]], [Sym("xy"), b[0], b[1]]],
                           [Sym("stroke"), [Sym("width"), 0], [Sym("type"), Sym("default")]], [Sym("uuid"), uid("wire", a, b)]])

    def label(self, net, x, y, angle):
        justify = [Sym("justify"), Sym("left" if angle in (0, 90) else "right"), Sym("bottom")]
        self.items.append([Sym("label"), net, [Sym("at"), x, y, angle],
                           [Sym("effects"), [Sym("font"), [Sym("size"), 1.27, 1.27]], justify], [Sym("uuid"), uid("label", net, x, y)]])

    def no_connect(self, x, y):
        self.items.append([Sym("no_connect"), [Sym("at"), x, y], [Sym("uuid"), uid("nc", x, y)]])

    def save(self, path):
        root = [Sym("kicad_sch"), [Sym("version"), Sym("20250114")], [Sym("generator"), "eeschema"],
                [Sym("generator_version"), "10.0"], [Sym("uuid"), ROOT_UUID], [Sym("paper"), "A1"],
                [Sym("title_block"), [Sym("title"), "Drezimo soundboard - 6x6 Voicemod controller"],
                 [Sym("rev"), "A"], [Sym("company"), "drezimo-soundboard (open source)"],
                 [Sym("comment"), 1, "Generated by hardware/scripts/build_schematic.py - edit the script, not this file"]],
                [Sym("lib_symbols")] + list(self.lib_symbols.values())]
        root += self.items
        root += [[Sym("sheet_instances"), [Sym("path"), "/", [Sym("page"), "1"]]], [Sym("embedded_fonts"), Sym("no")]]
        path.write_text(dump(root) + "\n", encoding="utf-8")


class Placed:
    def __init__(self, sch, ref, x, y, angle, pins, part):
        self.sch, self.ref, self.x, self.y, self.angle, self.pins, self.part = sch, ref, x, y, angle, pins, part

    def pin_pos(self, num):
        px, py, pa = self.pins[str(num)]
        t = math.radians(self.angle)
        rx, ry = px * math.cos(t) - py * math.sin(t), px * math.sin(t) + py * math.cos(t)
        out = math.radians(pa + 180 + self.angle)  # outward from the symbol body
        return (round(self.x + rx, 4), round(self.y - ry, 4)), (round(math.cos(out)), -round(math.sin(out)))

    def net(self, num, net, stub=2.54):
        (x, y), (dx, dy) = self.pin_pos(num)
        end = (round(x + dx * stub, 4), round(y + dy * stub, 4))
        self.sch.wire((x, y), end)
        angle = {(1, 0): 0, (-1, 0): 180, (0, -1): 90, (0, 1): 270}[(dx, dy)]
        self.sch.label(net, end[0], end[1], angle)
        self.part["pins"][str(num)] = net

    def nc(self, num):
        (x, y), _ = self.pin_pos(num)
        self.sch.no_connect(x, y)
        self.part["pins"][str(num)] = None


# ---------------------------------------------------------------- the circuit
FP_MX = "Button_Switch_Keyboard:SW_Cherry_MX_1.00u_PCB"
FP_LED = "LED_SMD:LED_SK6812MINI-E_3.2x2.8mm_P1.5mm_ReverseMount"
FP_D = "Diode_SMD:D_SOD-123"
FP_C0603 = "Capacitor_SMD:C_0603_1608Metric"


def build():
    sch = Schematic()
    notes = [
        "DREZIMO SOUNDBOARD - rev A",
        "36 sound pads (6x6) + 6 function keys + EC11 encoder + 1.3in SH1106 I2C OLED, RP2040-Zero (RP2040, USB-C).",
        "Powered only by the PC USB port (VBUS). LED current capped in firmware at ~400 mA total.",
        "",
        "KEY MATRIX: 7 rows x 6 cols, one 1N4148W per key. Diode anode on the switch side, cathode on the ROW.",
        "  Scan: drive one ROW low at a time, read COLs with pull-ups. Rows 0-5 = pads, row 6 = function keys.",
        "  SW1..SW36 = pads row-major from top-left (pad index 0..35), SW37..SW42 = function keys left to right.",
        "  Function keys: PREV, NEXT, STOP, MUTE ME, VOICE, HEAR ME.",
        "LEDS: SK6812MINI-E reverse mount on the PCB bottom, under each switch window (south side, 4.47 mm from centre, measured).",
        "  LEDn sits under SWn. Data chain order: " + " ".join(f"{k}" for k in CHAIN_ORDER[:12]) + " ...",
        "  i.e. function row L->R, then pad rows serpentine (row 0 R->L, row 1 L->R, ...). Firmware maps chain index -> key.",
        "  GP0 (3.3 V) -> 74AHCT125 gate A (5 V out, VIH of the LED is 0.65*VDD) -> 330R -> LED chain. One 100 nF per LED.",
        "GPIO: COL0-5 = GP1-6, ROW0-6 = GP7-13, ENC A/B/SW = GP28/GP29/GP14, I2C1 SDA/SCL = GP26/GP27, LED data = GP0.",
        "OLED J1 pin order VCC-GND-SCL-SDA, as printed on the bought module (seen from the glass side). Other modules swap VCC/GND.",
        "Spare 74AHCT125 gates: OE tied high (disabled), inputs tied low. J2 = optional expansion header, spare GP15 (not fitted).",
    ]
    for i, line in enumerate(notes):
        sch.text(line, 20, 20 + i * 4.2)

    # --- RP2040-Zero (on 2.54 mm pin headers, under the board)
    u1 = sch.place("Drezimo", "RP2040-Zero", "U1", "RP2040-Zero", "Drezimo:RP2040-Zero", 80, 160)
    for pin, net in ZERO_PINS.items():
        if net is None:
            u1.nc(pin)
        else:
            u1.net(pin, net)

    # --- Power: polyfuse on the LED rail, bulk + flags
    f1 = sch.place("Device", "Polyfuse", "F1", "PTC 0.5A hold", "Fuse:Fuse_1206_3216Metric", 60, 270)
    f1.net(1, "VBUS")
    f1.net(2, "+5V_LED")
    c43 = sch.place("Device", "C", "C43", "47uF 10V X5R", "Capacitor_SMD:C_1210_3225Metric", 85, 270)
    c43.net(1, "+5V_LED")
    c43.net(2, "GND")
    # GND, VBUS and +3V3 are driven by the module's power pins; only the fused LED rail needs a flag.
    flag = sch.place("power", "PWR_FLAG", "#FLG01", "PWR_FLAG", "", 110, 265)
    flag.net(1, "+5V_LED")

    # --- LED level shifter: 74AHCT125, gate A used, gates B-D parked
    u2a = sch.place("74xx", "74AHCT125", "U2", "74AHCT125", "Package_DIP:DIP-14_W7.62mm", 200, 70, unit=1)
    u2a.net(1, "GND")            # 1OE active low -> enabled
    u2a.net(2, "LED_DIN_3V3")    # 1A from GP0
    u2a.net(3, "LED_DIN_5V")     # 1Y
    for unit, (oe, a, y), x in ((2, (4, 5, 6), 230), (3, (10, 9, 8), 260), (4, (13, 12, 11), 290)):
        g = sch.place("74xx", "74AHCT125", "U2", "74AHCT125", "Package_DIP:DIP-14_W7.62mm", x, 70, unit=unit)
        g.net(oe, "+5V_LED")     # disabled
        g.net(a, "GND")          # input not floating
        g.nc(y)
    u2e = sch.place("74xx", "74AHCT125", "U2", "74AHCT125", "Package_DIP:DIP-14_W7.62mm", 200, 110, unit=5)
    u2e.net(14, "+5V_LED")
    u2e.net(7, "GND")
    c44 = sch.place("Device", "C", "C44", "100nF", FP_C0603, 225, 110)
    c44.net(1, "+5V_LED")
    c44.net(2, "GND")
    r1 = sch.place("Device", "R", "R1", "330R", "Resistor_SMD:R_0603_1608Metric", 250, 110)
    r1.net(1, "LED_DIN_5V")
    r1.net(2, LED_CHAIN_NETS[0])

    # --- OLED, encoder, expansion header
    # The module's own straight header goes through these 4 holes and is soldered: no socket on the PCB
    j1 = sch.place("Connector_Generic", "Conn_01x04", "J1", "OLED 1.3in SH1106 I2C", "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical", 200, 165)
    for pin, net in ((1, "+3V3"), (2, "GND"), (3, "SCL"), (4, "SDA")):
        j1.net(pin, net)
    enc = sch.place("Device", "RotaryEncoder_Switch_MP", "ENC1", "EC11 encoder + switch", "Rotary_Encoder:RotaryEncoder_Alps_EC11E-Switch_Vertical_H20mm", 205, 205)
    for pin, net in (("A", "ENC_A"), ("B", "ENC_B"), ("C", "GND"), ("S1", "ENC_SW"), ("S2", "GND"), ("MP", "GND")):
        enc.net(pin, net)
    j2 = sch.place("Connector_Generic", "Conn_01x03", "J2", "Expansion (not fitted)", "Connector_PinHeader_2.54mm:PinHeader_1x03_P2.54mm_Vertical", 200, 245, dnp=True)
    for pin, net in ((1, "+3V3"), (2, "EXP_GP15"), (3, "GND")):
        j2.net(pin, net)

    # --- Key matrix
    for n, (row, col) in KEYS.items():
        cx, cy = 340 + col * 45, 60 + row * 28
        sw = sch.place("Switch", "SW_Push", f"SW{n}", FN_KEYS[n - 37] if n > 36 else f"PAD {n - 1}", FP_MX, cx, cy)
        sw.net(1, f"COL{col}")
        sw.net(2, f"K{n}")
        d = sch.place("Device", "D", f"D{n}", "1N4148W", FP_D, cx + 20, cy, angle=180)  # anode left, cathode right
        d.net(2, f"K{n}")
        d.net(1, f"ROW{row}")

    # --- LED chain (LEDn under SWn), one 100 nF each
    for n in KEYS:
        k = CHAIN_ORDER.index(n)
        gx, gy = key_cell(k)
        x, y = 340 + gx * 50, 330 + gy * 40
        led = sch.place("LED", "SK6812MINI-E", f"LED{n}", "SK6812MINI-E", FP_LED, x, y)
        led.net(2, LED_CHAIN_NETS[k])                       # DIN
        if k + 1 < len(CHAIN_ORDER):
            led.net(4, LED_CHAIN_NETS[k + 1])               # DOUT
        else:
            led.nc(4)                                       # end of chain
        led.net(3, "+5V_LED")
        led.net(1, "GND")
        c = sch.place("Device", "C", f"C{n}", "100nF", FP_C0603, x + 17.78, y)
        c.net(1, "+5V_LED")
        c.net(2, "GND")

    # --- Mounting holes (no electrical connection, plastic case)
    for i in range(6):
        sch.place("Mechanical", "MountingHole", f"H{i + 1}", "M3", "MountingHole:MountingHole_3.2mm_M3", 60 + i * 15, 300)

    return sch


def write_lib_tables(sch):
    """Project library tables for the stock KiCad libraries used, plus the project's own symbols and
    footprints (hardware/lib: RP2040-Zero, logo), so the project opens anywhere."""
    sym_libs = sorted({lib.split(":")[0] for lib in sch.lib_symbols})
    fp_libs = sorted({p["footprint"].split(":")[0] for p in sch.parts.values() if p["footprint"]} - {"Drezimo"})
    sym = ["(sym_lib_table", "\t(version 7)"] + [
        f'\t(lib (name "{n}")(type "KiCad")(uri "${{KICAD10_SYMBOL_DIR}}/{n}.kicad_sym")(options "")(descr ""))'
        for n in sym_libs if n not in PROJECT_SYMBOLS] + [
        '\t(lib (name "Drezimo")(type "KiCad")(uri "${KIPRJMOD}/../lib/drezimo.kicad_sym")(options "")(descr "Project symbols"))',
        ")"]
    fp = ["(fp_lib_table", "\t(version 7)"] + [
        f'\t(lib (name "{n}")(type "KiCad")(uri "${{KICAD10_FOOTPRINT_DIR}}/{n}.pretty")(options "")(descr ""))' for n in fp_libs] + [
        '\t(lib (name "Drezimo")(type "KiCad")(uri "${KIPRJMOD}/../lib/drezimo.pretty")(options "")(descr "Project footprints"))',
        ")"]
    (OUT_DIR / "sym-lib-table").write_text("\n".join(sym) + "\n", encoding="utf-8")
    (OUT_DIR / "fp-lib-table").write_text("\n".join(fp) + "\n", encoding="utf-8")


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    sch = build()
    sch.save(OUT_DIR / f"{PROJECT}.kicad_sch")
    write_lib_tables(sch)
    pro = OUT_DIR / f"{PROJECT}.kicad_pro"
    if not pro.exists():
        pro.write_text(json.dumps({"meta": {"filename": pro.name, "version": 3}}, indent=2), encoding="utf-8")
    # Netlist as the script intends it, used by the PCB builder and the checks.
    parts = {ref: {k: v for k, v in p.items()} for ref, p in sch.parts.items() if not ref.startswith("#")}
    (OUT_DIR / "intended_netlist.json").write_text(json.dumps(parts, indent=1, sort_keys=True), encoding="utf-8")
    print(f"wrote {OUT_DIR / (PROJECT + '.kicad_sch')} with {len(parts)} parts")


if __name__ == "__main__":
    main()
