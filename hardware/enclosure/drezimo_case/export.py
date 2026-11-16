"""Print files for a part: STEP and 3MF in the print orientation, and a PNG to approve."""

import re
import uuid
import zipfile
from pathlib import Path

from build123d import Color, Compound, Mesher, Plane, Pos, Rot, export_step, mirror

from .preview import render_preview

BODY_COLOUR = Color(0.1, 0.1, 0.11)
INLAY_COLOUR = Color(0.96, 0.95, 0.94)
# Keeps the day of the export out of the files.
STEP_TIMESTAMP = "2026-01-01T00:00:00"
ARCHIVE_DATE = (2026, 1, 1, 0, 0, 0)
MODEL_FILE = "3D/3dmodel.model"


def to_real(shape):
    """The real part, from a solid modelled in the case frame.

    The case frame is left-handed (y runs towards the user), so every modelled solid is the
    mirror image of the part. Reversing y undoes that, once, for any part and any orientation.
    """
    return mirror(shape, Plane.XZ)


def face_down(real, deck_top):
    """The shell's print orientation: turned over, with the deck on z = 0."""
    return Pos(0, 0, deck_top) * Rot(180, 0, 0) * real


def export_part(name, body, inlay, out_dir, **preview_options):
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    step, mesh, picture = (out_dir / f"{name}{suffix}" for suffix in (".step", ".3mf", ".png"))

    deck_top = body.bounding_box().max.Z
    parts = [
        _part(face_down(to_real(body), deck_top), "body", BODY_COLOUR),
        _part(face_down(to_real(inlay), deck_top), "inlay", INLAY_COLOUR),
    ]

    export_step(Compound(label=name, children=parts), step, write_pcurves=False, timestamp=STEP_TIMESTAMP)
    _write_3mf(mesh, name, parts)
    render_preview(picture, body, inlay, **preview_options)
    return [step, mesh, picture]


def _part(shape, label, colour):
    part = Compound(shape.solids())
    part.label = label
    part.color = colour
    return part


def _write_3mf(path, name, parts):
    mesher = Mesher()
    for part in parts:
        mesher.add_shape(part, uuid_value=_uuid(name, part.label))
    mesher.write(str(path))
    _join_as_one_object(path, name)


def _join_as_one_object(path, name):
    """Makes the parts the components of a single object.

    The writer gives each part its own build item, which a slicer loads as separate objects
    that it is free to move apart. One object with two parts keeps the inlay in its pockets.
    """
    with zipfile.ZipFile(path) as archive:
        files = {info.filename: archive.read(info) for info in archive.infolist()}

    model = files[MODEL_FILE].decode()
    part_ids = re.findall(r'<item objectid="(\d+)"', model)
    model = re.sub(r'<object id="\d+" type="model"[^>]*>\s*<components>.*?</components>\s*</object>\s*', "", model, flags=re.S)
    object_id = max(int(i) for i in re.findall(r'<(?:object|basematerials) id="(\d+)"', model)) + 1
    components = "".join(f'<component objectid="{i}" p:UUID="{_uuid(name, "component", i)}"/>' for i in part_ids)
    model = model.replace(
        "</resources>",
        f'<object id="{object_id}" name="{name}" type="model" p:UUID="{_uuid(name, "object")}">'
        f"<components>{components}</components></object></resources>",
    )
    model = re.sub(
        r"<build[^>]*>.*?</build>",
        f'<build p:UUID="{_uuid(name, "build")}"><item objectid="{object_id}" p:UUID="{_uuid(name, "item")}"/></build>',
        model,
        flags=re.S,
    )
    files[MODEL_FILE] = model.encode()

    with zipfile.ZipFile(path, "w") as archive:
        for filename, data in files.items():
            archive.writestr(zipfile.ZipInfo(filename, date_time=ARCHIVE_DATE), data, compress_type=zipfile.ZIP_DEFLATED)


def _uuid(*names):
    return uuid.uuid5(uuid.NAMESPACE_URL, "drezimo/" + "/".join(str(n) for n in names))
