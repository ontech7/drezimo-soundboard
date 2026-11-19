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
# The lightest grey of a part that is all ribs and columns, where the deck's would not tell them from the plate.
RELIEF = 0.46
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


@dataclass(frozen=True)
class Section:
    """A cut along the case's length at x, with what stands on the board there.

    board: where the PCB lies along the cut, as (from, to, thickness); under the whole part when not given.
    furniture: outlines of the parts the cut meets, each as (corners in (y, z), colour, filled).
    span: the stretch of the cut that is drawn, as (from, to); all of it when not given.
    """

    x: float
    title: str
    keys: Keys | None = None
    board: tuple[float, float, float] | None = None
    furniture: tuple = ()
    span: tuple[float, float] | None = None


def render_preview(path, body, inlay, sections=(), title="", captions=(), beside=(), underside=False):
    """Draws the part from above, with its captions, and cut along each section.

    beside: the other parts of the case the sections cut too, each as (shape, grey).
    underside: draw the part from below as well, as it is seen once turned over sideways.
    """
    box = body.bounding_box()
    figure = plt.figure(figsize=SIZE_INCHES, dpi=DPI, facecolor=PAPER)
    figure.suptitle(title, color=INK, fontsize=20)

    if not sections:
        above = figure.subplots()
        above.set_title("From above", color=INK, fontsize=13)
    else:
        plans = 2 if underside else 1
        grid = figure.add_gridspec(len(sections), plans + 1, width_ratios=(2,) * plans + (3,), hspace=0.5)
        above = figure.add_subplot(grid[:, 0])
        above.set_title("From above (dashed: where it is cut)", color=INK, fontsize=13)
        for row, cut in enumerate(sections):
            above.axvline(cut.x, color=PRESSED, linewidth=1, linestyle=(0, (6, 4)))
            view = figure.add_subplot(grid[row, plans])
            _section_view(view, body, inlay, cut, box, beside)
            view.set_title(cut.title, color=INK, fontsize=13)
        if underside:
            below = figure.add_subplot(grid[:, 1])
            below.set_title("From below, turned over sideways", color=INK, fontsize=13)
            _top_view(below, body, inlay, box, from_below=True)

    _top_view(above, body, inlay, box)
    for x, y, text in captions:
        above.text(x, y, text, color=PRESSED, ha="center", va="center", fontsize=12)

    figure.savefig(path, facecolor=PAPER)
    plt.close(figure)


def _top_view(ax, body, inlay, box, from_below=False):
    """Every face that is not upright, the nearest to the eye drawn last. From below, left and right change places."""
    faces = []
    for shape, fixed_colour in _coloured(body, inlay, None):
        vertices, triangles = shape.tessellate(MESH_TOLERANCE)
        for triangle in triangles:
            a, b, c = (vertices[i] for i in triangle)
            normal = (b - a).cross(c - a)
            steepness = abs(normal.Z) / normal.length if normal.length else 0.0
            if steepness < 0.05:
                continue
            height = (a.Z + b.Z + c.Z) / 3
            colour = fixed_colour or _grey(SLOPE if steepness < 0.95 else _level(height, box, from_below))
            faces.append((height, [(a.X, a.Y), (b.X, b.Y), (c.X, c.Y)], colour))

    faces.sort(key=lambda face: face[0], reverse=from_below)
    # From below one flat face hides all the rest: its triangles are edged, or the ribs show through their seams.
    seams = {"edgecolors": "face", "linewidths": 0.4} if from_below else {"edgecolors": "none"}
    ax.add_collection(PolyCollection([f[1] for f in faces], facecolors=[f[2] for f in faces], **seams))
    across = (box.max.X, box.min.X) if from_below else (box.min.X, box.max.X)
    _frame(ax, across, (box.max.Y, box.min.Y))


def _section_view(ax, body, inlay, cut, box, beside=()):
    plane = Plane.YZ.offset(cut.x)
    for shape, colour in [(shape, _grey(grey)) for shape, grey in beside] + _coloured(body, inlay, _grey(DECK)):
        triangles = []
        for face in section(shape, section_by=plane).faces():
            vertices, indices = face.tessellate(MESH_TOLERANCE)
            triangles += [[(vertices[i].Y, vertices[i].Z) for i in triangle] for triangle in indices]
        ax.add_collection(PolyCollection(triangles, facecolors=colour, edgecolors=colour, linewidths=0.3))

    start, end, thickness = cut.board or (box.min.Y, box.max.Y, PCB_THICKNESS)
    ax.add_patch(Rectangle((start, -thickness), end - start, thickness, color=PCB))
    top = max([box.max.Z, *(shape.bounding_box().max.Z for shape, _ in beside)]) + 2
    lowest = min(box.min.Z, -thickness) - 1
    for corners, colour, filled in cut.furniture:
        ax.add_patch(Polygon(corners, closed=True, fill=filled, facecolor=colour, edgecolor=colour, linewidth=1.4, alpha=0.75 if filled else 1.0))
        top = max(top, max(z for _, z in corners) + 1)
        lowest = min(lowest, min(z for _, z in corners) - 1)
    if cut.keys:
        keys = cut.keys
        for y in keys.centres:
            ax.add_patch(Rectangle((y - keys.switch_width / 2, 0), keys.switch_width, SWITCH_TOP, color=SWITCH, alpha=0.45))
            ax.add_patch(_keycap(y, keys, keys.skirt_at_rest, INK, "solid"))
            ax.add_patch(_keycap(y, keys, keys.skirt_pressed, PRESSED, (0, (5, 3))))
        top = max(top, keys.skirt_at_rest + KEYCAP_HEIGHT + 1)
    _frame(ax, cut.span or (box.min.Y - 1, box.max.Y + 1), (lowest, top))


def _keycap(y, keys, skirt, colour, style):
    base, crown = keys.keycap_width / 2, KEYCAP_TOP_WIDTH / 2
    outline = [(y - base, skirt), (y - crown, skirt + KEYCAP_HEIGHT), (y + crown, skirt + KEYCAP_HEIGHT), (y + base, skirt)]
    return Polygon(outline, closed=True, fill=False, edgecolor=colour, linewidth=1.6, linestyle=style)


def _coloured(body, inlay, body_colour):
    """The body and, for a part of two colours, its inlay."""
    return [(body, body_colour)] + ([] if inlay is None else [(inlay, INLAY)])


def _level(height, box, from_below=False):
    """Darker the further from the eye a face lies: from the board up, or for a part under the board from its own
    lowest face, which shows more of its relief."""
    under_the_board = box.max.Z <= 0
    lowest = box.min.Z if under_the_board else 0.0
    nearness = max(0.0, min(1.0, (height - lowest) / (box.max.Z - lowest)))
    if from_below:
        nearness = 1.0 - nearness
    return FLOOR + ((RELIEF if under_the_board else DECK) - FLOOR) * nearness


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
