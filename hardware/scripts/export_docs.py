"""Exports review documents for the current schematic and PCB into hardware/docs/.

- schematic.pdf                 the A1 schematic
- pcb-fitcheck-top.pdf          1:1 on A4: case, keycaps, footprints seen from above (print at 100 %)
- pcb-fitcheck-bottom.pdf       1:1 on A4, mirrored: the underside as you see it when you flip the board
- pcb-top.png / pcb-bottom.png  3D renders
- drc.json                      design rule check, with schematic parity

Usage: python export_docs.py   (after build_schematic.py and build_pcb.py)
"""

import json
import subprocess
from collections import Counter
from pathlib import Path

KICAD_CLI = "/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli"
HW = Path(__file__).resolve().parent.parent
PCB = HW / "pcb" / "drezimo-soundboard.kicad_pcb"
SCH = HW / "pcb" / "drezimo-soundboard.kicad_sch"
OUT = HW / "docs"

FITCHECK_COMMON = ["--mode-single", "--scale", "1", "--sketch-pads-on-fab-layers", "--drill-shape-opt", "2",
                   "--black-and-white", "--sdnp"]


def cli(*args):
    subprocess.run([KICAD_CLI, *map(str, args)], check=True, capture_output=True)


def main():
    OUT.mkdir(exist_ok=True)
    cli("sch", "export", "pdf", "-o", OUT / "schematic.pdf", SCH)
    cli("pcb", "export", "pdf", *FITCHECK_COMMON, "--layers", "Edge.Cuts,F.Fab,F.Courtyard,Dwgs.User",
        "-o", OUT / "pcb-fitcheck-top.pdf", PCB)
    cli("pcb", "export", "pdf", *FITCHECK_COMMON, "--mirror", "--ev", "--layers", "Edge.Cuts,B.Fab,B.Courtyard,Dwgs.User",
        "-o", OUT / "pcb-fitcheck-bottom.pdf", PCB)
    for side in ("top", "bottom"):
        cli("pcb", "render", "--side", side, "--quality", "high", "--width", "1600", "--height", "2200",
            "--background", "opaque", "-o", OUT / f"pcb-{side}.png", PCB)
    cli("pcb", "render", "--quality", "high", "--rotate", "-40,0,25", "--perspective", "--zoom", "0.9",
        "--width", "2000", "--height", "1600", "--background", "opaque", "-o", OUT / "pcb-iso.png", PCB)

    drc = OUT / "drc.json"
    subprocess.run([KICAD_CLI, "pcb", "drc", "--format", "json", "--severity-all", "--schematic-parity",
                    "-o", str(drc), str(PCB)], check=True, capture_output=True)
    report = json.loads(drc.read_text(encoding="utf-8"))
    print("DRC violations:", dict(Counter(v["type"] for v in report["violations"])) or "none")
    print("schematic parity issues:", len(report["schematic_parity"]))
    print("unrouted connections:", len(report["unconnected_items"]), "(expected until routing)")
    print("wrote", ", ".join(sorted(p.name for p in OUT.iterdir())))


if __name__ == "__main__":
    main()
