"""A picture to approve a part before printing: its top view and, for a part on the board, a section."""

from dataclasses import dataclass

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt  # noqa: E402
from build123d import Plane, section  # noqa: E402
from matplotlib.collections import PolyCollection  # noqa: E402
from matplotlib.patches import Polygon, Rectangle  # noqa: E402

SIZE_INCHES, DPI = (16, 12), 100
MESH_TOLERANCE = 0.05

PAPER, INK = "#17171a", "#d8d7d2"
INLAY = "#f4f3ef"
DECK, FLOOR, SLOPE = 0.20, 0.09, 0.30
PCB, SWITCH, PRESSED = "#1f6b3a", "#b58a2a", "#ff8a3d"

PCB_THICKNESS = 1.6
SWITCH_TOP = 11.6
KEYCAP_HEIGHT, KEYCAP_TOP_WIDTH = 8.5, 13.0


@dataclass(frozen=True)
class Keys:
    """What stands on the board along the section: key centres (y) and their measured sizes."""

    centres: tuple[float, ...]
    switch_width: float
    keycap_width: float
    skirt_at_rest: float
    skirt_pressed: float


def render_preview(path, body, inlay, section_x=None, keys=None, title="", captions=()):
    """Draws the part from above, with its captions, and cut along section_x when one is given."""
    box = body.bounding_box()
    figure = plt.figure(figsize=SIZE_INCHES, dpi=DPI, facecolor=PAPER)
    figure.suptitle(title, color=INK, fontsize=20)

    if section_x is None:
        above = figure.subplots()
        above.set_title("From above", color=INK, fontsize=13)
    else:
        above, cut = figure.subplots(1, 2, gridspec_kw={"width_ratios": (2, 3)})
        above.axvline(section_x, color=PRESSED, linewidth=1, linestyle=(0, (6, 4)))
        above.set_title("From above (dashed: where the section is cut)", color=INK, fontsize=13)
        _section_view(cut, body, inlay, section_x, box, keys)
        cut.set_title("Section, back to the left — dashed keycap: fully pressed", color=INK, fontsize=13)

    _top_view(above, body, inlay, box)
    for x, y, text in captions:
        above.text(x, y, text, color=PRESSED, ha="center", va="center", fontsize=12)

    figure.savefig(path, facecolor=PAPER)
    plt.close(figure)


def _top_view(ax, body, inlay, box):
    faces = []
    for shape, fixed_colour in ((body, None), (inlay, INLAY)):
        vertices, triangles = shape.tessellate(MESH_TOLERANCE)
        for triangle in triangles:
            a, b, c = (vertices[i] for i in triangle)
            normal = (b - a).cross(c - a)
            steepness = abs(normal.Z) / normal.length if normal.length else 0.0
            if steepness < 0.05:
                continue
            height = (a.Z + b.Z + c.Z) / 3
            colour = fixed_colour or _grey(SLOPE if steepness < 0.95 else _level(height, box.max.Z))
            faces.append((height, [(a.X, a.Y), (b.X, b.Y), (c.X, c.Y)], colour))

    faces.sort(key=lambda face: face[0])
    ax.add_collection(PolyCollection([f[1] for f in faces], facecolors=[f[2] for f in faces], edgecolors="none"))
    _frame(ax, (box.min.X, box.max.X), (box.max.Y, box.min.Y))


def _section_view(ax, body, inlay, x, box, keys):
    plane = Plane.YZ.offset(x)
    for shape, colour in ((body, _grey(DECK)), (inlay, INLAY)):
        triangles = []
        for face in section(shape, section_by=plane).faces():
            vertices, indices = face.tessellate(MESH_TOLERANCE)
            triangles += [[(vertices[i].Y, vertices[i].Z) for i in triangle] for triangle in indices]
        ax.add_collection(PolyCollection(triangles, facecolors=colour, edgecolors=colour, linewidths=0.3))

    ax.add_patch(Rectangle((box.min.Y, -PCB_THICKNESS), box.size.Y, PCB_THICKNESS, color=PCB))
    top = box.max.Z + 2
    if keys:
        for y in keys.centres:
            ax.add_patch(Rectangle((y - keys.switch_width / 2, 0), keys.switch_width, SWITCH_TOP, color=SWITCH, alpha=0.45))
            ax.add_patch(_keycap(y, keys, keys.skirt_at_rest, INK, "solid"))
            ax.add_patch(_keycap(y, keys, keys.skirt_pressed, PRESSED, (0, (5, 3))))
        top = keys.skirt_at_rest + KEYCAP_HEIGHT + 1
    _frame(ax, (box.min.Y - 1, box.max.Y + 1), (-PCB_THICKNESS - 1, top))


def _keycap(y, keys, skirt, colour, style):
    base, crown = keys.keycap_width / 2, KEYCAP_TOP_WIDTH / 2
    outline = [(y - base, skirt), (y - crown, skirt + KEYCAP_HEIGHT), (y + crown, skirt + KEYCAP_HEIGHT), (y + base, skirt)]
    return Polygon(outline, closed=True, fill=False, edgecolor=colour, linewidth=1.6, linestyle=style)


def _level(height, deck_top):
    return FLOOR + (DECK - FLOOR) * max(0.0, min(1.0, height / deck_top))


def _grey(value):
    return (value, value, value * 1.04)


def _frame(ax, x_limits, y_limits):
    ax.set_xlim(*x_limits)
    ax.set_ylim(*y_limits)
    ax.set_aspect("equal")
    ax.set_facecolor("#0c0c0e")
    ax.tick_params(colors=INK, labelsize=9)
    for spine in ax.spines.values():
        spine.set_color("#44444a")
