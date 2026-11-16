"""Print files for a part: STEP and 3MF in the print orientation, and a PNG to approve."""

import uuid
from pathlib import Path

from build123d import Color, Compound, Mesher, Plane, Pos, export_step, mirror

from .preview import render_preview

BODY_COLOUR = Color(0.1, 0.1, 0.11)
INLAY_COLOUR = Color(0.96, 0.95, 0.94)
# A fixed header date keeps a regenerated file identical when the geometry has not changed.
STEP_TIMESTAMP = "2026-01-01T00:00:00"


def to_print(shape, deck_top):
    """The deck on z = 0, facing down.

    The solids are modelled in the left-handed case frame, so flipping z is also what turns
    them into the real part (see deck.py).
    """
    return Pos(0, 0, deck_top) * mirror(shape, Plane.XY)


def export_part(name, body, inlay, out_dir, **preview_options):
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    step, mesh, picture = (out_dir / f"{name}{suffix}" for suffix in (".step", ".3mf", ".png"))

    deck_top = body.bounding_box().max.Z
    parts = [
        _part(to_print(body, deck_top), "body", BODY_COLOUR),
        _part(to_print(inlay, deck_top), "inlay", INLAY_COLOUR),
    ]

    export_step(Compound(label=name, children=parts), step, write_pcurves=False, timestamp=STEP_TIMESTAMP)

    mesher = Mesher()
    for part in parts:
        mesher.add_shape(part, uuid_value=uuid.uuid5(uuid.NAMESPACE_URL, f"drezimo/{name}/{part.label}"))
    mesher.write(str(mesh))

    render_preview(picture, body, inlay, **preview_options)
    return [step, mesh, picture]


def _part(shape, label, colour):
    part = Compound(shape.solids())
    part.label = label
    part.color = colour
    return part
