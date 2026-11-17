"""Bambu Studio projects: a part's 3MF with its print settings already decided.

Bambu Studio imports the part and writes the project itself. This module hands it the
printer's complete presets, sets what the part needs and has the result sliced as a check,
so a project is only written if it slices without a warning.
"""

import json
import re
import shutil
import subprocess
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree

STUDIO = Path("/Applications/BambuStudio.app/Contents/MacOS/BambuStudio")
PROFILES = STUDIO.parents[1] / "Resources/profiles/BBL"

PRINTER = "Bambu Lab P1S 0.4 nozzle"
PROCESS = "0.20mm Standard @BBL X1C"
FILAMENT = "Overture Matte PLA @BBL X1C"

PROJECT = {
    "filament_colour": ["#1A1A1C", "#F5F2F0"],
    "curr_bed_type": "Textured PEI Plate",
    # mm3 purged going from black to white and from white to black: the white must come out clean.
    "flush_volumes_matrix": ["0", "800", "250", "0"],
}
# Strokes and gaps narrower than two lines are drawn at their width instead of being dropped.
INLAYS = {"wall_generator": "arachne"}
SUPPORTS = {
    "enable_support": "1",
    "support_type": "normal(auto)",
    "support_on_build_plate_only": "1",
    "support_interface_top_layers": "3",
    "support_interface_spacing": "0.2",
}
PARTS = (("body", 1), ("inlay", 2))
# The body goes down first and shapes the letters; the white then fills them.
FIRST_LAYER = {"first_layer_sequence_choice": "Customize", "first_layer_print_sequence": "1 2"}

SETTINGS_FILE = "Metadata/project_settings.config"
MODEL_SETTINGS_FILE = "Metadata/model_settings.config"
UNPRINTED = ("Custom", "Prime tower")
SUPPORT_FEATURE = "; FEATURE: Support"


class SliceError(Exception):
    """Bambu Studio refused the project or warned about it."""


@dataclass(frozen=True)
class SliceReport:
    version: str
    minutes: float
    grams: tuple
    first_layer_order: list
    supports: bool


def write_project(neutral, target, supported):
    """Write the project for a neutral 3MF and return what slicing it gave."""
    with tempfile.TemporaryDirectory() as scratch:
        work = Path(scratch)
        presets = {name: _write_preset(name, work) for name in (PRINTER, PROCESS, FILAMENT)}
        _studio(
            work,
            "--load-settings", f"{presets[PRINTER]};{presets[PROCESS]}",
            "--load-filaments", f"{presets[FILAMENT]};{presets[FILAMENT]}",
            "--export-3mf", "imported.3mf",
            Path(neutral).resolve(),
        )
        imported = _read_archive(work / "imported.3mf")
        _write_archive(work / "project.3mf", decide(imported, supported))
        _studio(work, "--slice", "0", "--export-3mf", "sliced.3mf", work / "project.3mf")

        report = _report(work, imported)
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


def decide(files, supported):
    """A project exported by Bambu Studio (archive name to bytes), with this part's settings."""
    process = INLAYS | (SUPPORTS if supported else {})
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
    layer, filament, feature, order = 0, None, "", []
    for line in gcode.splitlines():
        if line.startswith("; CHANGE_LAYER"):
            layer += 1
        elif line.startswith("; FEATURE:"):
            feature = line.split(":")[1].strip()
        elif (loaded := re.match(r"M620 S(\d+)A", line)) and int(loaded.group(1)) < 16:
            filament = int(loaded.group(1)) + 1
        elif layer == 1 and feature not in UNPRINTED and re.match(r"G[123] .*E\.?\d", line) and filament not in order:
            order.append(filament)
    return order


def _report(work, imported):
    result = json.loads((work / "result.json").read_text())
    plate = result["sliced_plates"][0]
    if plate["warning_message"]:
        raise SliceError(plate["warning_message"])
    gcode = (work / "plate_1.gcode").read_text()
    order = first_layer_order(gcode)
    if order != [filament for _, filament in PARTS]:
        raise SliceError(f"the first layer prints the filaments in the order {order}")
    return SliceReport(
        version=re.search(rb"BambuStudio-([\d.]+)", imported["3D/3dmodel.model"]).group(1).decode(),
        minutes=plate["total_predication"] / 60,
        grams=tuple(filament["total_used_g"] for filament in plate["filaments"]),
        first_layer_order=order,
        supports=SUPPORT_FEATURE in gcode,
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
