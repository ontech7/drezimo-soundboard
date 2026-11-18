import re
import struct
import zipfile

import pytest
from build123d import import_step
from helpers import NOMINAL, inside, nominal

from drezimo_case import __main__ as cli
from drezimo_case.export import export_part, face_down, to_real
from drezimo_case.measurements import Measurement
from drezimo_case.params import MAX_PRINT_SCALE, EstimateError


@pytest.fixture(scope="module")
def exported(tmp_path_factory, body, inlay):
    out = tmp_path_factory.mktemp("print")
    return out, export_part("coupon-a", body, inlay, out)


@pytest.fixture(scope="module")
def step_parts(exported):
    out, _ = exported
    return {part.label: part for part in import_step(out / "coupon-a.step").children}


def all_measured():
    return {code: Measurement(value, measured=True) for code, value in NOMINAL.items()}


def test_real_part_is_the_model_with_y_reversed(body):
    real = to_real(body)
    assert real.volume == pytest.approx(body.volume)
    assert inside(real, 1.0, -60.0, 3.0)
    assert not inside(real, 1.0, 60.0, 3.0)


def test_face_down_lays_the_deck_on_the_plate(body, p):
    flat = face_down(to_real(body), p.deck_top)
    box = flat.bounding_box()
    assert (box.min.Z, box.max.Z) == pytest.approx((0.0, p.deck_top + 1.0), abs=1e-6)
    assert inside(flat, 4.9, 47.0, p.deck_top - 0.05)
    assert not inside(flat, 4.9, 47.0, p.deck_top + 0.05)


def test_step_holds_the_two_labelled_parts(exported, step_parts, body, inlay):
    _, paths = exported
    assert {path.suffix for path in paths} == {".step", ".3mf", ".png"}
    assert set(step_parts) == {"body", "inlay"}
    assert step_parts["body"].volume == pytest.approx(body.volume, rel=1e-3)
    assert step_parts["inlay"].volume == pytest.approx(inlay.volume, rel=1e-3)
    assert step_parts["body"].bounding_box().min.Z == pytest.approx(0.0, abs=1e-6)


def test_exported_part_is_not_a_mirror_image(step_parts):
    # Seen on the plate: x and y as in the case's top view, z up from the deck.
    printed = step_parts["body"]
    assert inside(printed, 27.0, 45.0, 0.5)        # solid deck between the two labels, at the back
    assert not inside(printed, 27.0, 88.0, 0.5)    # open well between the first two pads, towards the user
    assert inside(printed, 1.0, 60.0, 5.0)         # the wall, on the left
    assert not inside(printed, 45.0, 60.0, 8.0)


def test_3mf_is_one_object_made_of_the_two_parts(exported):
    out, _ = exported
    model = zipfile.ZipFile(out / "coupon-a.3mf").read("3D/3dmodel.model").decode()
    items = re.findall(r'<item objectid="(\d+)"', model)
    assert len(items) == 1
    assembly = re.search(rf'<object id="{items[0]}"[^>]*>\s*<components>(.*?)</components>', model, re.S)
    parts = re.findall(r'<component objectid="(\d+)"', assembly.group(1))
    assert {re.search(rf'<object id="{part}" name="(\w+)"', model).group(1) for part in parts} == {"body", "inlay"}
    assert len(re.findall(r"<object ", model)) == 3


def test_preview_is_a_1600_by_1200_picture(exported):
    out, _ = exported
    header = (out / "coupon-a.png").read_bytes()[:24]
    assert header[:8] == b"\x89PNG\r\n\x1a\n"
    assert struct.unpack(">II", header[16:24]) == (1600, 1200)


def test_command_names_exactly_the_estimates(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal(M1w=124.25, M5a=6.45))
    cli.main(["coupon-a"])
    assert {path.name for path in tmp_path.iterdir()} == {"coupon-a.step", "coupon-a.3mf", "coupon-a.png"}
    assert capsys.readouterr().out.splitlines()[-1].endswith("estimates, not for the final print: M1h, M1t, M2a, M2b, M2c, M3a, M4a, M4b, M4e, M4f, M4h, M4l, M4p, M4s, M4t, M4w, M4x, M4y, M5b, M6a, M6b, M7a, M7b, M8h, M8w, P1l, P1s")


def test_command_says_nothing_about_estimates_once_all_is_measured(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", all_measured())
    cli.main(["coupon-a"])
    assert "estimates" not in capsys.readouterr().out


def test_a_final_part_refuses_estimates(tmp_path, monkeypatch):
    final = cli.PrintedPart(cli.PARTS["coupon-a"].build, draft=False, title="Final")
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "PARTS", {"final": final})
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal(M5a=6.45))
    with pytest.raises(EstimateError, match="M2b"):
        cli.main(["final"])
    assert not list(tmp_path.iterdir())
    monkeypatch.setattr(cli, "MEASUREMENTS", all_measured())
    cli.main(["final"])
    assert (tmp_path / "final.step").exists()


def test_the_shell_is_refused_while_a_reading_is_an_estimate(tmp_path, monkeypatch):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal(M5a=6.45))
    with pytest.raises(EstimateError, match="M4x") as error:
        cli.main(["shell"])
    assert "M5a" not in str(error.value)
    assert not list(tmp_path.iterdir())


def test_a_draft_of_the_shell_is_a_picture_and_nothing_to_print(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal(M5a=6.45))
    cli.main(["shell", "--draft"])
    assert [path.name for path in tmp_path.iterdir()] == ["shell-draft.png"]
    header = (tmp_path / "shell-draft.png").read_bytes()[:24]
    assert header[:8] == b"\x89PNG\r\n\x1a\n"
    assert struct.unpack(">II", header[16:24]) == (1600, 1200)
    said = capsys.readouterr().out
    assert "Deck top 7.8 mm above the board, set by M5a" in said
    assert "estimates, not for the final print: M1h, M1t" in said

    with pytest.raises(SystemExit, match="picture"):
        cli.main(["shell", "--draft", "--bambu"])
    with pytest.raises(SystemExit, match="usage"):
        cli.main(["coupon-a", "--draft"])
    assert [path.name for path in tmp_path.iterdir()] == ["shell-draft.png"]


def test_the_measured_shell_is_written_to_print(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", all_measured())
    cli.main(["shell"])
    assert {path.name for path in tmp_path.iterdir()} == {"shell.step", "shell.3mf", "shell.png"}
    assert "estimates" not in capsys.readouterr().out


def test_a_part_whose_walls_show_lies_with_its_back_to_the_back_of_the_bed(body, p):
    # The slicer's "back" is the bed's far side: turned half a turn, the seam goes to the back wall.
    turned = face_down(to_real(body), p.deck_top, turned=True)
    assert turned.volume == pytest.approx(body.volume)
    assert inside(turned, -1.0, -60.0, 5.0)        # the left wall, now on the right of the plate
    assert inside(turned, -27.0, -45.0, 0.5)
    assert not inside(turned, 1.0, 60.0, 5.0)


def printed_size(path):
    """The size of what a 3MF holds, as a slicer loads it."""
    model = zipfile.ZipFile(path).read("3D/3dmodel.model").decode()
    points = [[float(v) for v in vertex] for vertex in re.findall(r'<vertex x="([^"]+)" y="([^"]+)" z="([^"]+)"', model)]
    return [max(axis) - min(axis) for axis in zip(*points)]


def test_a_part_is_printed_bigger_by_the_print_scale_and_drawn_as_modelled(tmp_path, body, inlay):
    plain = export_part("plain", body, inlay, tmp_path)
    bigger = export_part("bigger", body, inlay, tmp_path, scale=1.002)
    # Across the plate only: heights stay on the layers.
    assert [b / a for a, b in zip(printed_size(plain[1]), printed_size(bigger[1]))] == pytest.approx([1.002, 1.002, 1.0], abs=1e-5)
    assert tuple(import_step(bigger[0]).bounding_box().size) == pytest.approx(tuple(import_step(plain[0]).bounding_box().size))


def test_the_shell_can_be_written_as_big_as_the_gauge_may_ask(tmp_path, shell, p):
    # Made bigger as a solid, the shell did not mesh: its 3MF is made bigger point by point.
    written = export_part("shell", *shell, tmp_path, turned=True, scale=MAX_PRINT_SCALE)
    width, length = p.case_size
    assert printed_size(written[1])[:2] == pytest.approx((width * MAX_PRINT_SCALE, length * MAX_PRINT_SCALE), abs=0.01)


def test_the_shell_is_scaled_and_turned_and_the_coupon_is_not(tmp_path, monkeypatch):
    calls = []
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", {**all_measured(), "P1s": Measurement(69.86, measured=True), "P1l": Measurement(100.80, measured=True)})
    monkeypatch.setattr(cli, "export_part", lambda name, body, inlay, out, **options: calls.append((name, options)) or [])
    cli.main(["shell"])
    cli.main(["coupon-a"])
    (shell, shell_options), (coupon, coupon_options) = calls
    assert (shell, shell_options["turned"], shell_options["scale"]) == ("shell", True, pytest.approx(1.002, abs=1e-4))
    assert (coupon, coupon_options["turned"], coupon_options["scale"]) == ("coupon-a", False, 1.0)


def test_the_command_says_how_much_higher_the_knob_must_sit(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "export_part", lambda *_, **options: [])
    monkeypatch.setattr(cli, "MEASUREMENTS", all_measured())
    cli.main(["shell"])
    assert "knob" not in capsys.readouterr().out

    monkeypatch.setattr(cli, "MEASUREMENTS", {**all_measured(), "M5b": Measurement(8.0, measured=True)})
    cli.main(["shell"])
    assert "Set the knob 0.3 mm higher on its shaft" in capsys.readouterr().out
