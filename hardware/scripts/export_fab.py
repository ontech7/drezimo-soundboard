"""Exports the JLCPCB manufacturing package into hardware/fab/.

- drezimo-soundboard-gerbers.zip   Gerbers + Excellon drill: upload this as the PCB
- drezimo-soundboard-bom.csv       assembly BOM: Comment, Designator, Footprint, LCSC Part #
- drezimo-soundboard-cpl.csv       placement: Designator, Mid X, Mid Y, Layer, Rotation

The fab assembles every footprint that has an LCSC field (set in build_schematic.py), all of them
on the PCB bottom. The switches, the RP2040-Zero, the SN74AHCT125N, the OLED and the encoder are
through-hole, hand-soldered, and stay out of the BOM and the placement file.

Run with KiCad's bundled Python (it provides the pcbnew module), after build_pcb.py:
    /Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3 export_fab.py
"""

import csv
import re
import shutil
import subprocess
import zipfile
from collections import defaultdict
from pathlib import Path

import pcbnew

KICAD_CLI = "/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli"
HW = Path(__file__).resolve().parent.parent
PCB = HW / "pcb" / "drezimo-soundboard.kicad_pcb"
OUT = HW / "fab"
NAME = "drezimo-soundboard"

LAYERS = "F.Cu,B.Cu,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,F.Mask,B.Mask,Edge.Cuts"


def cli(*args):
    subprocess.run([KICAD_CLI, *map(str, args)], check=True, capture_output=True)


def natural(ref):
    m = re.match(r"([A-Z]+)(\d+)$", ref)
    return (m.group(1), int(m.group(2))) if m else (ref, 0)


def gerbers():
    """Gerbers and drill files share the drill/place origin (the case corner), like the CPL."""
    tmp = OUT / "gerbers"
    shutil.rmtree(tmp, ignore_errors=True)
    tmp.mkdir(parents=True)
    cli("pcb", "export", "gerbers", "--layers", LAYERS, "--no-x2", "--no-netlist", "--subtract-soldermask",
        "--use-drill-file-origin", "-o", tmp, PCB)
    cli("pcb", "export", "drill", "--format", "excellon", "--drill-origin", "plot", "--excellon-units", "mm",
        "--excellon-zeros-format", "decimal", "--excellon-oval-format", "alternate", "--excellon-separate-th",
        "-o", tmp, PCB)
    archive = OUT / f"{NAME}-gerbers.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for f in sorted(tmp.iterdir()):
            z.write(f, f.name)
    files = sorted(f.name for f in tmp.iterdir())
    shutil.rmtree(tmp)
    return archive, files


def placement(fp, origin):
    """JLC wants the part centre, which need not be the footprint origin: use the centre of the
    soldered pads' bounding box, relative to the drill/place origin, y pointing up.

    Bottom rotation: JLC reads bottom parts as seen from below, so rotation = 180 - KiCad orientation
    (KiCad stores a bottom footprint as a y-mirror of its library drawing, then rotated); the same
    rule as kicad-jlcpcb-tools. Checked on 2026-11-01 against each part's JLC (EasyEDA) footprint:
    SK6812MINI-E (same pin numbers: 1 GND, 2 DIN, 3 VDD, 4 DOUT), SOD-123 (pad 1 = cathode on the
    left) and the 0603/1206 parts match KiCad's pad layout, so no extra correction."""
    pads = [p for p in fp.Pads() if p.GetAttribute() != pcbnew.PAD_ATTRIB_NPTH]
    box = pads[0].GetBoundingBox()
    for p in pads[1:]:
        box.Merge(p.GetBoundingBox())
    c = box.GetCenter()
    x, y = pcbnew.ToMM(c.x - origin.x), -pcbnew.ToMM(c.y - origin.y)
    rotation = (180 - fp.GetOrientationDegrees()) % 360
    return x, y, rotation


def main():
    OUT.mkdir(exist_ok=True)
    archive, files = gerbers()

    board = pcbnew.LoadBoard(str(PCB))
    origin = board.GetDesignSettings().GetAuxOrigin()
    groups = defaultdict(list)       # (LCSC, footprint) -> [footprint, ...]
    hand = []
    for fp in board.Footprints():
        ref = fp.GetReference()
        lcsc = fp.GetFieldText("LCSC") if fp.HasField("LCSC") else ""
        if lcsc:
            if fp.GetLayer() != pcbnew.B_Cu:
                raise RuntimeError(f"{ref}: assembled parts must be on the bottom (single-side assembly)")
            groups[(lcsc, str(fp.GetFPID().GetLibItemName()))].append(fp)
        elif fp.GetAttributes() & pcbnew.FP_SMD:
            raise RuntimeError(f"{ref}: SMD footprint without an LCSC field")
        elif any(p.GetAttribute() == pcbnew.PAD_ATTRIB_PTH for p in fp.Pads()):
            hand.append(ref)

    bom = OUT / f"{NAME}-bom.csv"
    cpl = OUT / f"{NAME}-cpl.csv"
    rows = []
    with bom.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["Comment", "Designator", "Footprint", "LCSC Part #"])
        for (lcsc, footprint), fps in sorted(groups.items(), key=lambda g: natural(g[1][0].GetReference())):
            fps.sort(key=lambda fp: natural(fp.GetReference()))
            values = {fp.GetValue() for fp in fps}
            comment = values.pop() if len(values) == 1 else footprint
            w.writerow([comment, ",".join(fp.GetReference() for fp in fps), footprint, lcsc])
            rows += fps
    with cpl.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["Designator", "Mid X", "Mid Y", "Layer", "Rotation"])
        for fp in sorted(rows, key=lambda fp: natural(fp.GetReference())):
            x, y, rot = placement(fp, origin)
            w.writerow([fp.GetReference(), f"{x:.4f}mm", f"{y:.4f}mm", "Bottom", f"{rot:g}"])

    print("gerbers:", archive.name, "->", ", ".join(files))
    for (lcsc, footprint), fps in sorted(groups.items(), key=lambda g: natural(g[1][0].GetReference())):
        print(f"  {lcsc:9} x{len(fps):<3} {footprint}")
    print(f"assembled: {len(rows)} parts on the bottom, {len(groups)} BOM lines")
    switches = [ref for ref in hand if ref.startswith("SW")]
    others = sorted(set(hand) - set(switches), key=natural)
    print(f"hand-soldered (not in BOM/CPL): {len(switches)} switches, {', '.join(others)}")
    print("wrote", bom.name, cpl.name)


if __name__ == "__main__":
    main()
