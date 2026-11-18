"""Bambu Studio projects: a part's 3MF with its print settings already decided.

Bambu Studio imports the part and writes the project itself. This module hands it the
printer's complete presets, sets what the part needs and has the result sliced as a check,
so a project is only written if it slices without a warning.
"""

import json
import math
import re
import shutil
import subprocess
import tempfile
import zipfile
from dataclasses import astuple, dataclass
from pathlib import Path
from xml.etree import ElementTree

STUDIO = Path("/Applications/BambuStudio.app/Contents/MacOS/BambuStudio")
PROFILES = STUDIO.parents[1] / "Resources/profiles/BBL"



@dataclass(frozen=True)
class Nozzle:
    """The system presets of the P1S with one nozzle: the process is the one Bambu Studio picks for it."""

    printer: str
    process: str
    filament: str


# By the nozzle's diameter in mm.
NOZZLES = {
    0.4: Nozzle("Bambu Lab P1S 0.4 nozzle", "0.20mm Standard @BBL X1C", "Overture Matte PLA @BBL X1C"),
    0.2: Nozzle("Bambu Lab P1S 0.2 nozzle", "0.10mm Standard @BBL X1C 0.2 nozzle", "Overture Matte PLA @BBL X1C 0.2 nozzle"),
}

PROJECT = {
    "filament_colour": ["#1A1A1C", "#F5F2F0"],
    "curr_bed_type": "Textured PEI Plate",
    # mm3 purged going from black to white and from white to black: the white must come out clean.
    "flush_volumes_matrix": ["0", "800", "250", "0"],
}
INLAYS = {
    # Strokes and gaps narrower than two lines are drawn at their width instead of being dropped.
    "wall_generator": "arachne",
    # mm/s, slower than either nozzle's process: the small black islands inside the letters must hold on the plate.
    "initial_layer_speed": ["30", "30"],
    "initial_layer_infill_speed": ["60", "60"],
}
SUPPORTS = {
    "enable_support": "1",
    "support_type": "normal(auto)",
    "support_on_build_plate_only": "1",
    "support_interface_top_layers": "3",
    "support_interface_spacing": "0.2",
    # The body's black. Left to Bambu Studio they start in white, 0.35 mm from the rim of the well on the face that shows.
    "support_filament": "1",
    "support_interface_filament": "1",
}
SOLID_DECK = {
    # Full plastic for this many layers, and at least this many mm, from each face: the two skins of a 2 mm deck
    # meet. Bambu Studio's own 0.5 and 0.8 mm leave the middle sparse.
    "bottom_shell_layers": "10",
    "bottom_shell_thickness": "1",
    "top_shell_layers": "10",
    "top_shell_thickness": "1",
}
PARTS = (("body", 1), ("inlay", 2))
# The body goes down first and shapes the letters; the white then fills them.
FIRST_LAYER = {"first_layer_sequence_choice": "Customize", "first_layer_print_sequence": "1 2"}

SETTINGS_FILE = "Metadata/project_settings.config"
MODEL_SETTINGS_FILE = "Metadata/model_settings.config"
UNPRINTED = ("Custom", "Prime tower")
SUPPORT = "Support"
# mm a support may stand outside the areas a part allows: the slicer spreads them a little.
SUPPORT_SPREAD = 0.5


class SliceError(Exception):
    """Bambu Studio refused the project or warned about it."""


@dataclass(frozen=True)
class SliceReport:
    version: str
    minutes: float
    grams: tuple
    first_layer_order: list
    supports: bool


def write_project(neutral, target, supported, nozzle, supports_within=None, solid_deck=False):
    """Write the project for a neutral 3MF, to print with this nozzle, and return what slicing it gave.

    supports_within: the only rectangles where supports may stand, about the middle of the part's first layer.
    solid_deck: print the deck full through, for a part that is pressed with a finger.
    """
    with tempfile.TemporaryDirectory() as scratch:
        work = Path(scratch)
        printer, process, filament = (_write_preset(name, work) for name in astuple(NOZZLES[nozzle]))
        _studio(
            work,
            "--load-settings", f"{printer};{process}",
            "--load-filaments", f"{filament};{filament}",
            "--export-3mf", "imported.3mf",
            Path(neutral).resolve(),
        )
        imported = _read_archive(work / "imported.3mf")
        _write_archive(work / "project.3mf", decide(imported, supported, solid_deck))
        _studio(work, "--slice", "0", "--export-3mf", "sliced.3mf", work / "project.3mf")

        report = _report(work, imported, supports_within)
        shutil.copyfile(work / "project.3mf", target)
        return report


def resolve_preset(name, profiles=PROFILES):
    """A system preset with everything it inherits, as the one flat file the command line wants."""
    preset = json.loads(_preset_file(name, profiles).read_text())
    values = resolve_preset(preset["inherits"], profiles) if preset.get("inherits") else {}
    for template in preset.get("include", []):
        values |= json.loads(_preset_file(template, profiles).read_text())
    values |= preset
    return {key: value for key, value in values.items() if key not in ("inherits", "include")}


def decide(files, supported, solid_deck=False):
    """A project exported by Bambu Studio (archive name to bytes), with this part's settings."""
    process = INLAYS | (SUPPORTS if supported else {}) | (SOLID_DECK if solid_deck else {})
    settings = json.loads(files[SETTINGS_FILE])
    settings |= PROJECT | process
    settings["different_settings_to_system"] = [";".join(sorted(process)), *settings["different_settings_to_system"][1:]]

    model = ElementTree.fromstring(files[MODEL_SETTINGS_FILE])
    parts = list(model.iter("part"))
    if len(parts) != len(PARTS):
        raise ValueError(f"expected {len(PARTS)} parts in the project, found {len(parts)}")
    for part, (name, filament) in zip(parts, PARTS):
        part.find("metadata[@key='name']").set("value", name)
        _add_metadata(part, "extruder", filament)
    for key, value in FIRST_LAYER.items():
        _add_metadata(model.find("plate"), key, value)

    return files | {
        SETTINGS_FILE: json.dumps(settings, indent=4).encode(),
        MODEL_SETTINGS_FILE: ElementTree.tostring(model, encoding="utf-8", xml_declaration=True),
    }


def first_layer_order(gcode):
    """The filaments in the order they print the part's first layer."""
    order = []
    for layer, filament, feature, _ in _extrusions(gcode):
        if layer == 1 and feature not in UNPRINTED and filament not in order:
            order.append(filament)
    return order


def support_filaments(gcode):
    """The filaments the supports are printed in."""
    return {filament for _, filament, feature, _ in _extrusions(gcode) if feature.startswith(SUPPORT)}


def support_points(gcode):
    """Where the supports' moves end, about the middle of the part's first layer."""
    moves = list(_extrusions(gcode))
    first_layer = [at for layer, _, feature, at in moves if layer == 1 and feature not in UNPRINTED and not feature.startswith(SUPPORT)]
    xs, ys = zip(*first_layer)
    middle = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    return [(x - middle[0], y - middle[1]) for _, _, feature, (x, y) in moves if feature.startswith(SUPPORT)]


def check_supports(gcode, allowed):
    """Raises if a support stands outside every rectangle of `allowed`; None allows them anywhere."""
    if allowed is None:
        return
    stray = [_distance_outside(point, allowed) for point in support_points(gcode)]
    stray = [distance for distance in stray if distance > SUPPORT_SPREAD]
    if stray:
        raise SliceError(f"{len(stray)} moves of the supports stand outside the places meant for them, up to {max(stray):.1f} mm out")


def _distance_outside(point, rectangles):
    x, y = point
    return min(math.hypot(max(r.x0 - x, 0, x - r.x1), max(r.y0 - y, 0, y - r.y1)) for r in rectangles)


def _extrusions(gcode):
    """The layer, the filament, the feature and the end point of every move that lays filament down.

    Filament pushed out with the head standing still is purged or primed, not laid.
    """
    layer, filament, feature, x, y = 0, None, "", 0.0, 0.0
    for line in gcode.splitlines():
        if line.startswith("; CHANGE_LAYER"):
            layer += 1
        elif line.startswith("; FEATURE:"):
            feature = line.split(":")[1].strip()
        elif (loaded := re.match(r"M620 S(\d+)A", line)) and int(loaded.group(1)) < 16:
            filament = int(loaded.group(1)) + 1
        elif re.match(r"G[0123] ", line):
            words = dict(re.findall(r"([XY])(-?\d*\.?\d+)", line.split(";")[0]))
            moved = (float(words.get("X", x)), float(words.get("Y", y)))
            if moved != (x, y) and re.match(r"G[123] .*E\.?\d", line):
                yield layer, filament, feature, moved
            x, y = moved


def _report(work, imported, supports_within):
    result = json.loads((work / "result.json").read_text())
    plate = result["sliced_plates"][0]
    if plate["warning_message"]:
        raise SliceError(plate["warning_message"])
    gcode = (work / "plate_1.gcode").read_text()
    order = first_layer_order(gcode)
    if order != [filament for _, filament in PARTS]:
        raise SliceError(f"the first layer prints the filaments in the order {order}")
    supports = support_filaments(gcode)
    if supports - {dict(PARTS)["body"]}:
        raise SliceError(f"the supports are printed in the filaments {sorted(supports)}")
    check_supports(gcode, supports_within)
    return SliceReport(
        version=re.search(rb"BambuStudio-([\d.]+)", imported["3D/3dmodel.model"]).group(1).decode(),
        minutes=plate["total_predication"] / 60,
        grams=tuple(filament["total_used_g"] for filament in plate["filaments"]),
        first_layer_order=order,
        supports=bool(supports),
    )


def _studio(work, *arguments):
    command = [STUDIO, "--datadir", work / "data", "--outputdir", work, *arguments]
    done = subprocess.run([str(word) for word in command], cwd=work, capture_output=True, text=True)
    if done.returncode:
        raise SliceError(" ".join(done.stdout.strip().splitlines()[-2:]))


def _write_preset(name, folder):
    path = folder / f"{name}.json"
    path.write_text(json.dumps(resolve_preset(name)))
    return path


def _preset_file(name, profiles):
    for kind in ("machine", "process", "filament"):
        path = Path(profiles) / kind / f"{name}.json"
        if path.exists():
            return path
    raise FileNotFoundError(f"no Bambu Studio preset called {name!r} in {profiles}")


def _add_metadata(element, key, value):
    element.insert(len(element.findall("metadata")), ElementTree.Element("metadata", key=key, value=str(value)))


def _read_archive(path):
    with zipfile.ZipFile(path) as archive:
        return {info.filename: archive.read(info.filename) for info in archive.infolist()}


def _write_archive(path, files):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in files.items():
            archive.writestr(name, data)
