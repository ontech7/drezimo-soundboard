"""Independent checks of the netlist KiCad actually produces from the schematic.

ERC only proves the schematic is self-consistent; this proves it implements the design:
key matrix, diode direction, LED chain, power rails, level shifter, RP2040-Zero pin map.

Usage: python check_design.py   (exits non-zero on any failure)
"""

import subprocess
import sys
from pathlib import Path

from design import CHAIN_ORDER, KEYS, LED_CHAIN_NETS, ZERO_PINS
from sexpr import find, first, parse

KICAD_CLI = "/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli"
PCB_DIR = Path(__file__).resolve().parent.parent / "pcb"
SCH = PCB_DIR / "drezimo-soundboard.kicad_sch"

failures = []


def val(node, key):
    item = first(node, key)
    assert item is not None, (key, node[:2])
    return item[1]


def unconnected(net):
    return net is None or net.startswith("unconnected")


def check(cond, msg):
    if not cond:
        failures.append(msg)


def load_netlist():
    out = PCB_DIR / "netlist.net"
    subprocess.run([KICAD_CLI, "sch", "export", "netlist", "--format", "kicadsexpr", "-o", str(out), str(SCH)],
                   check=True, capture_output=True)
    root = parse(out.read_text(encoding="utf-8"))
    nets = {}
    nets_node, comps_node = first(root, "nets"), first(root, "components")
    assert nets_node is not None and comps_node is not None
    for n in find(nets_node, "net"):
        name = val(n, "name")
        nodes = {(val(x, "ref"), str(val(x, "pin"))) for x in find(n, "node")}
        nets[name.lstrip("/")] = nodes
    comps = {val(c, "ref"): val(c, "footprint") if first(c, "footprint") else "" for c in find(comps_node, "comp")}
    return nets, comps


def main():
    nets, comps = load_netlist()
    pin_net = {}
    for name, nodes in nets.items():
        for node in nodes:
            pin_net[node] = name
    net_of = lambda ref, pin: pin_net.get((ref, str(pin)))  # noqa: E731

    # 1. Key matrix: COLc - SWn.1 ; SWn.2 - Dn.A(2) ; Dn.K(1) - ROWr
    for n, (row, col) in KEYS.items():
        check(net_of(f"SW{n}", 1) == f"COL{col}", f"SW{n} pin 1 should be COL{col}, is {net_of(f'SW{n}', 1)}")
        k = net_of(f"SW{n}", 2)
        check(k is not None and k == net_of(f"D{n}", 2), f"SW{n} pin 2 must meet D{n} anode")
        check(len(nets.get(k, ())) == 2, f"net {k} must have exactly SW{n}.2 and D{n}.A")
        check(net_of(f"D{n}", 1) == f"ROW{row}", f"D{n} cathode should be ROW{row}, is {net_of(f'D{n}', 1)}")
        check(comps.get(f"D{n}") == "Diode_SMD:D_SOD-123", f"D{n} footprint")
        check(comps.get(f"SW{n}") == "Drezimo:SW_MX_PCB", f"SW{n} footprint (soldered MX switch)")
    for c in range(6):
        check(len(nets[f"COL{c}"]) == 7 + 1, f"COL{c} must reach 7 switches + the MCU")
    for r in range(7):
        check(len(nets[f"ROW{r}"]) == 6 + 1, f"ROW{r} must reach 6 diodes + the MCU")

    # 2. LED chain. SK6812MINI-E pins (datasheet 2025, verified against the footprint): 1 GND, 2 DIN, 3 VDD, 4 DOUT
    check(net_of("R1", 2) == LED_CHAIN_NETS[0], "R1 must feed the first LED")
    for k, n in enumerate(CHAIN_ORDER):
        ref = f"LED{n}"
        check(net_of(ref, 2) == LED_CHAIN_NETS[k], f"{ref} DIN should be {LED_CHAIN_NETS[k]}")
        if k + 1 < len(CHAIN_ORDER):
            nxt = f"LED{CHAIN_ORDER[k + 1]}"
            check(net_of(ref, 4) == net_of(nxt, 2), f"{ref} DOUT must drive {nxt} DIN")
            check(len(nets[LED_CHAIN_NETS[k + 1]]) == 2, f"{LED_CHAIN_NETS[k + 1]} must be a point-to-point link")
        else:
            check(unconnected(net_of(ref, 4)), f"{ref} DOUT is the end of the chain")
        check(net_of(ref, 3) == "+5V_LED", f"{ref} VDD on +5V_LED")
        check(net_of(ref, 1) == "GND", f"{ref} GND")
        check(net_of(f"C{n}", 1) == "+5V_LED" and net_of(f"C{n}", 2) == "GND", f"C{n} decoupling across +5V_LED/GND")
        check(comps.get(ref) == "LED_SMD:LED_SK6812MINI-E_3.2x2.8mm_P1.5mm_ReverseMount", f"{ref} footprint")

    # 3. Level shifter 74AHCT125 (pinout: 7 GND, 14 VCC; OE/A/Y = 1/2/3, 4/5/6, 10/9/8, 13/12/11); gate 3 used
    check(net_of("U2", 10) == "GND", "U2 3OE low (gate enabled)")
    check(net_of("U2", 9) == "LED_DIN_3V3", "U2 3A from GP8")
    check(net_of("U2", 8) == "LED_DIN_5V" == net_of("R1", 1), "U2 3Y to R1")
    check(net_of("U2", 14) == "+5V_LED" and net_of("U2", 7) == "GND", "U2 powered from the 5 V LED rail")
    for oe, a in ((1, 2), (4, 5), (13, 12)):
        check(net_of("U2", oe) == "+5V_LED", f"U2 pin {oe} (OE) high: spare gate disabled")
        check(net_of("U2", a) == "GND", f"U2 pin {a} (A) tied low")
    check(comps.get("U2") == "Package_DIP:DIP-14_W7.62mm", "U2 is the owned DIP-14 part")

    # 4. RP2040-Zero pin map, power, peripherals
    for pin, net in ZERO_PINS.items():
        actual = net_of("U1", pin)
        if net is None:
            check(unconnected(actual), f"U1 pin {pin} should be unconnected, is {actual}")
        else:
            check(actual == net, f"U1 pin {pin} should be {net}, is {actual}")
    check(net_of("F1", 1) == "VBUS" and net_of("F1", 2) == "+5V_LED", "polyfuse between VBUS and the LED rail")
    check(net_of("C43", 1) == "+5V_LED" and net_of("C43", 2) == "GND", "bulk capacitor on the LED rail")
    check(net_of("C44", 1) == "+5V_LED" and net_of("C44", 2) == "GND", "U2 decoupling")
    check([net_of("J1", p) for p in (1, 2, 3, 4)] == ["+3V3", "GND", "SCL", "SDA"], "OLED header VCC-GND-SCL-SDA, VCC = 3.3 V")
    check(comps.get("U1") == "Drezimo:RP2040-Zero", "U1 is the RP2040-Zero footprint")
    check({p: net_of("ENC1", p) for p in ("A", "B", "C", "S1", "S2", "MP")} ==
          {"A": "ENC_A", "B": "ENC_B", "C": "GND", "S1": "ENC_SW", "S2": "GND", "MP": "GND"}, "encoder wiring")
    check(not any(net_of(f"LED{n}", 3) == "VBUS" for n in KEYS), "LEDs only on the fused rail")
    check(len(nets["+3V3"]) == 2, "+3V3 only feeds the OLED")

    # 5. No accidental single-node nets (typos in labels)
    for name, nodes in nets.items():
        if not name.startswith("unconnected") and len(nodes) < 2:
            failures.append(f"net {name} has a single node {nodes}")

    total = len(nets)
    if failures:
        print(f"FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  -", f)
        sys.exit(1)
    print(f"OK: {len(comps)} components, {total} nets; matrix, diodes, LED chain, power and pin map all match the design")


if __name__ == "__main__":
    main()
